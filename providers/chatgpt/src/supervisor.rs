//! Process and profile owner for one AppHost generation.
use crate::config::{ensure_private_dir, random_hex};
use crate::error::{ProviderError, Result};
use crate::host::{duplicate_fd, prepare_auth_browser, termux_prefix};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::net::Shutdown;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const WAIT_TICK: Duration = Duration::from_millis(30);

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Ready {
    pub display: u16,
    pub directory: PathBuf,
    pub rootfs: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StartError {
    pub code: String,
    pub detail: String,
}

type StartReply = std::result::Result<Ready, StartError>;

pub(crate) fn run(args: &[std::ffi::OsString]) -> Result<()> {
    let [root, profile, control_fd, cdp_fd, mode] = args else {
        return Err(ProviderError::bad_request(
            "host_supervisor_args",
            "invalid supervisor invocation",
        ));
    };
    if mode != "serve" && mode != "probe" {
        return Err(ProviderError::bad_request(
            "host_supervisor_args",
            "invalid supervisor mode",
        ));
    }
    let parse_fd = |arg: &std::ffi::OsStr| {
        arg.to_str()
            .and_then(|s| s.parse::<RawFd>().ok())
            .filter(|&fd| fd >= 3)
            .ok_or_else(|| {
                ProviderError::bad_request("host_supervisor_args", "invalid supervisor descriptor")
            })
    };
    let (ctrl, cdp) = (parse_fd(control_fd)?, parse_fd(cdp_fd)?);
    if ctrl == cdp {
        return Err(ProviderError::bad_request(
            "host_supervisor_args",
            "duplicate descriptor",
        ));
    }
    for fd in [ctrl, cdp] {
        let mut kind = 0i32;
        let mut len = std::mem::size_of_val(&kind) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_TYPE,
                (&mut kind as *mut i32).cast(),
                &mut len,
            )
        } != 0
            || kind != libc::SOCK_STREAM
            || unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0
        {
            return Err(host_io("host_supervisor_fd", io::Error::last_os_error()));
        }
    }
    // Both descriptors were transferred by the caller and are distinct.
    let mut control = unsafe { UnixStream::from_raw_fd(ctrl) };
    let cdp = unsafe { OwnedFd::from_raw_fd(cdp) };
    let signals = match initialize() {
        Ok(signals) => signals,
        Err(error) => {
            announce_error(&mut control, &error);
            return Err(error);
        }
    };
    let mut host = HostProcesses::default();
    let started = host.start(
        &control,
        &signals,
        Path::new(root),
        Path::new(profile),
        cdp,
        mode == "probe",
    );
    let result = match started {
        Ok(ready) => match publish(&mut control, &StartReply::Ok(ready)) {
            Ok(()) => {
                host.serve(&control, &signals);
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            Err(error) => Err(host_io("host_control_failed", error)),
        },
        Err(error) => {
            announce_error(&mut control, &error);
            Err(error)
        }
    };
    let _ = control.shutdown(Shutdown::Write);
    result.and(host.stop(&signals))
}

fn announce_error(control: &mut UnixStream, error: &ProviderError) {
    let _ = publish(
        control,
        &StartReply::Err(StartError {
            code: error.code.to_owned(),
            detail: error.detail.clone(),
        }),
    );
}

fn publish(control: &mut UnixStream, reply: &StartReply) -> io::Result<()> {
    serde_json::to_writer(&mut *control, reply)?;
    control.write_all(b"\n")
}

fn host_io(code: &'static str, error: io::Error) -> ProviderError {
    ProviderError::unavailable(code, error.to_string())
}

fn initialize() -> Result<OwnedFd> {
    // This mode enters before the Provider creates its Tokio runtime.
    unsafe {
        let mut mask = std::mem::zeroed();
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGCHLD);
        if libc::signal(libc::SIGCHLD, libc::SIG_DFL) == libc::SIG_ERR
            || libc::prctl(
                libc::PR_SET_CHILD_SUBREAPER,
                1 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
            ) < 0
            || libc::sigprocmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut()) < 0
            || (libc::getsid(0) != libc::getpid() && libc::setsid() < 0)
        {
            return Err(host_io("host_supervisor_init", io::Error::last_os_error()));
        }
        let fd = libc::signalfd(-1, &mask, libc::SFD_CLOEXEC | libc::SFD_NONBLOCK);
        if fd < 0 {
            return Err(host_io("host_supervisor_init", io::Error::last_os_error()));
        }
        Ok(OwnedFd::from_raw_fd(fd))
    }
}

/// The control peer sends no commands. Readability means EOF or an invalid message.
fn wait_event(
    control: Option<&UnixStream>,
    signals: &OwnedFd,
    duration: Duration,
) -> io::Result<bool> {
    let mut fds = [
        libc::pollfd {
            fd: control.map_or(-1, AsRawFd::as_raw_fd),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: signals.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    let millis = duration.as_millis().min(i32::MAX as u128) as i32;
    let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, millis) };
    if result < 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::Interrupted {
            Ok(false)
        } else {
            Err(error)
        };
    }
    if fds[1].revents & libc::POLLIN != 0 {
        let mut signal = std::mem::MaybeUninit::<libc::signalfd_siginfo>::uninit();
        unsafe {
            libc::read(
                signals.as_raw_fd(),
                signal.as_mut_ptr().cast(),
                std::mem::size_of_val(&signal),
            );
        }
    }
    Ok(fds[0].revents != 0)
}

fn still_connected(control: &UnixStream, signals: &OwnedFd, wait: Duration) -> Result<()> {
    match wait_event(Some(control), signals, wait) {
        Ok(false) => Ok(()),
        Ok(true) => Err(ProviderError::unavailable(
            "host_start_cancelled",
            "AppHost caller disconnected",
        )),
        Err(error) => Err(host_io("host_control_failed", error)),
    }
}

fn spawn(command: &mut Command) -> Result<i32> {
    unsafe {
        command.pre_exec(|| {
            let mut mask = std::mem::zeroed();
            libc::sigemptyset(&mut mask);
            libc::sigaddset(&mut mask, libc::SIGCHLD);
            if libc::sigprocmask(libc::SIG_UNBLOCK, &mask, std::ptr::null_mut()) < 0
                || libc::setsid() < 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    // This process alone waits for children, including those adopted after launch.
    command
        .spawn()
        .map(|child| child.id() as i32)
        .map_err(|error| host_io("host_process_start_failed", error))
}

fn profile_lease(profile: &Path, control: &UnixStream, signals: &OwnedFd) -> Result<File> {
    let mut lock_path = profile.as_os_str().to_owned();
    lock_path.push(".lock");
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(lock_path)
        .map_err(|error| host_io("host_profile_unavailable", error))?;
    let metadata = lease
        .metadata()
        .map_err(|error| host_io("host_profile_unavailable", error))?;
    if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(ProviderError::unavailable(
            "host_profile_unavailable",
            "profile lease is not a user-owned regular file",
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        still_connected(control, signals, Duration::ZERO)?;
        if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(lease);
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::WouldBlock {
            return Err(host_io("host_profile_unavailable", error));
        }
        if Instant::now() >= deadline {
            return Err(ProviderError::unavailable(
                "host_profile_busy",
                format!("profile is locked: {}", profile.display()),
            ));
        }
        still_connected(control, signals, WAIT_TICK)?;
    }
}

fn pid_looks_like_chatgpt_host(pid: i32) -> Option<bool> {
    let raw = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let lower = String::from_utf8_lossy(&raw).to_ascii_lowercase();
    Some(lower.contains("chatgpt") || lower.contains("electron") || lower.contains("proot"))
}

fn check_singleton(chromium: &Path) -> Result<()> {
    let target = match fs::read_link(chromium.join("SingletonLock")) {
        Ok(target) => target,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(host_io("host_profile_busy_stale_electron", error)),
    };
    let pid = target
        .to_str()
        .and_then(|s| s.rsplit_once('-'))
        .and_then(|(_, s)| s.parse::<i32>().ok())
        .filter(|&pid| pid > 1);
    if let Some(pid) = pid {
        if unsafe { libc::kill(pid, 0) } < 0 {
            let errno = io::Error::last_os_error().raw_os_error();
            // ESRCH: dead PID. EPERM: PID recycled to another UID (cannot be our Electron).
            if matches!(errno, Some(libc::ESRCH | libc::EPERM)) {
                return Ok(());
            }
        } else if pid_looks_like_chatgpt_host(pid) == Some(false) {
            // PID recycled to an unrelated same-UID process (e.g. Termux shell/daemon).
            return Ok(());
        }
    }
    Err(ProviderError::unavailable(
        "host_profile_busy_stale_electron",
        "Chromium singleton belongs to a live or unverified process",
    ))
}

#[derive(Default)]
struct HostProcesses {
    proot: Option<i32>,
    display_pid: Option<i32>,
    openbox: Option<i32>,
    wmctrl: Option<i32>,
    x11: Option<(PathBuf, u16, i32)>,
    artifacts: Option<DisplayArtifacts>,
    directory: Option<TemporaryDir>,
    lease: Option<File>,
}

impl HostProcesses {
    fn start(
        &mut self,
        control: &UnixStream,
        signals: &OwnedFd,
        root: &Path,
        profile: &Path,
        cdp: OwnedFd,
        probe: bool,
    ) -> Result<Ready> {
        let canonical_rootfs =
            fs::canonicalize(root).map_err(|error| host_io("host_rootfs_not_found", error))?;
        if !canonical_rootfs.join("usr/bin/chatgpt").is_file()
            || !canonical_rootfs
                .join("usr/lib/chatgpt/resources/app.asar")
                .is_file()
        {
            return Err(ProviderError::unavailable(
                "official_app_rootfs_required",
                "rootfs missing chatgpt binary or app.asar",
            ));
        }
        let prefix = termux_prefix();
        ensure_private_dir(profile)?;
        let profile = fs::canonicalize(profile)
            .map_err(|error| host_io("host_profile_unavailable", error))?;
        let profile = profile.as_path();
        self.lease = Some(profile_lease(profile, control, signals)?);
        let chromium = profile.join("chromium");
        ensure_private_dir(&chromium)?;
        check_singleton(&chromium)?;
        ensure_private_dir(&profile.join(".codex"))?;
        let path = std::env::temp_dir().join(format!("cgp-{}", random_hex(4)?));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|error| host_io("host_temp_failed", error))?;
        self.directory = Some(TemporaryDir(path.clone()));
        let path = path.as_path();
        for subdirectory in ["tmp", "shm", "links"] {
            ensure_private_dir(&path.join(subdirectory))?;
        }
        if !probe {
            prepare_auth_browser(profile, path)?;
        }
        let socket_root = prefix.join("tmp/.X11-unix");
        let display = (130..1950u16)
            .find(|id| {
                fs::symlink_metadata(prefix.join(format!("tmp/.X{id}-lock")))
                    .is_err_and(|e| e.kind() == io::ErrorKind::NotFound)
                    && fs::symlink_metadata(socket_root.join(format!("X{id}")))
                        .is_err_and(|e| e.kind() == io::ErrorKind::NotFound)
            })
            .ok_or_else(|| {
                ProviderError::unavailable("host_display_unavailable", "no free X11 display")
            })?;
        let auth = path.join("Xauthority");
        write_xauthority(&auth, display)?;
        let mut xserver = Command::new(prefix.join("bin/Xvnc"));
        xserver
            .env_clear()
            .env("HOME", path)
            .env("PATH", prefix.join("bin"))
            .env("PREFIX", &prefix)
            .env("TMPDIR", path)
            .args([
                format!(":{display}"),
                "-geometry".into(),
                "480x785".into(),
                "-depth".into(),
                "24".into(),
                "-nolisten".into(),
                "tcp".into(),
                "-auth".into(),
                auth.to_string_lossy().into_owned(),
                "-rfbport".into(),
                "-1".into(),
                "-rfbunixpath".into(),
                path.join("viewer").to_string_lossy().into_owned(),
                "-rfbunixmode".into(),
                "0600".into(),
                "-SecurityTypes".into(),
                "None".into(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        still_connected(control, signals, Duration::ZERO)?;
        let display_pid = spawn(&mut xserver)?;
        self.display_pid = Some(display_pid);
        self.x11 = Some((prefix.clone(), display, display_pid));
        let deadline = Instant::now() + Duration::from_secs(10);
        let socket = socket_root.join(format!("X{display}"));
        loop {
            self.capture_artifacts();
            if self
                .artifacts
                .as_ref()
                .is_some_and(|artifacts| artifacts.0.len() == 2)
                && UnixStream::connect(&socket).is_ok()
            {
                break;
            }
            self.reap();
            if self.display_pid.is_none() || Instant::now() >= deadline {
                return Err(ProviderError::unavailable(
                    "host_display_start_timeout",
                    "Xvnc did not open its display",
                ));
            }
            still_connected(control, signals, WAIT_TICK)?;
        }
        let openbox = prefix.join("bin/openbox");
        if openbox.is_file() {
            self.openbox = spawn(&mut ui_command(&prefix, path, display, &auth, &openbox)).ok();
        }
        still_connected(control, signals, Duration::ZERO)?;
        let source = duplicate_fd(cdp.as_raw_fd())
            .map_err(|error| host_io("host_pipe_unavailable", error))?;
        let source_fd = source.as_raw_fd();
        let root_file = File::open(&canonical_rootfs)
            .map_err(|e| ProviderError::unavailable("host_rootfs_open_failed", e.to_string()))?;
        let root_fd = root_file.as_raw_fd();

        let host_log_path = profile.join("host.log");
        let app_log_out = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&host_log_path)
            .ok();
        let app_log_err = app_log_out.as_ref().and_then(|f| f.try_clone().ok());

        let mut cmd = Command::new(prefix.join("bin/proot"));
        cmd.env_clear()
            .env("PATH", prefix.join("bin"))
            .env("HOME", profile)
            .env("TMPDIR", path)
            .env("PROOT_TMP_DIR", path)
            .env("PROOT_L2S_DIR", path.join("links"))
            .args([
                "--kill-on-exit",
                "--link2symlink",
                "--sysvipc",
                "-L",
                "--change-id=0:0",
                "--rootfs=.",
                "--cwd=/root",
                "--bind=/dev",
                "--bind=/proc",
                "--bind=/sys",
                "--bind=/dev/urandom:/dev/random",
            ])
            .arg(format!("--bind={}:/root", profile.display()))
            .arg(format!("--bind={}:/tmp", path.join("tmp").display()))
            .arg(format!("--bind={}:/dev/shm", path.join("shm").display()))
            .arg(format!("--bind={}:/tmp/.X11-unix", socket_root.display()))
            .arg(format!("--bind={}:/ui", path.display()))
            .args([
                "/usr/bin/env",
                "-i",
                "HOME=/root",
                "CODEX_HOME=/root/.codex",
                "PATH=/ui/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "BROWSER=/ui/bin/xdg-open",
                "LANG=C.UTF-8",
                "XAUTHORITY=/ui/Xauthority",
            ])
            .arg(format!("DISPLAY=:{display}"))
            .args([
                "/usr/bin/dbus-run-session",
                "--",
                "/usr/bin/chatgpt",
                "--no-sandbox",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--user-data-dir=/root/chromium",
                "--remote-debugging-pipe",
            ])
            .stdin(Stdio::null())
            .stdout(app_log_out.map_or_else(Stdio::null, Stdio::from))
            .stderr(app_log_err.map_or_else(Stdio::null, Stdio::from));
        unsafe {
            cmd.pre_exec(move || {
                if libc::fchdir(root_fd) != 0
                    || libc::dup2(source_fd, 3) < 0
                    || libc::dup2(source_fd, 4) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        self.proot = Some(spawn(&mut cmd)?);
        Ok(Ready {
            display,
            directory: path.to_path_buf(),
            rootfs: canonical_rootfs,
        })
    }

    fn capture_artifacts(&mut self) {
        if let Some((prefix, display, pid)) = &self.x11 {
            if self.display_pid != Some(*pid) {
                return;
            }
            if self
                .artifacts
                .as_ref()
                .is_none_or(|artifacts| artifacts.0.len() != 2)
            {
                if let Some(artifacts) = DisplayArtifacts::capture(prefix, *display, *pid) {
                    // Further work may observe the socket after the lock has already appeared.
                    self.artifacts = Some(artifacts);
                }
            }
        }
    }

    fn serve(&mut self, control: &UnixStream, signals: &OwnedFd) {
        let (prefix, display, _) = self.x11.as_ref().expect("started display").clone();
        let wmctrl_bin = prefix.join("bin/wmctrl");
        let directory = self
            .directory
            .as_ref()
            .expect("started directory")
            .0
            .clone();
        let mut adjust = wmctrl_bin.is_file();
        let adjust_deadline = Instant::now() + Duration::from_secs(45);
        loop {
            let reaped = self.reap();
            if self.proot.is_none() || self.display_pid.is_none() {
                break;
            }
            let wait = if adjust && Instant::now() < adjust_deadline {
                if reaped.wmctrl_succeeded {
                    adjust = false;
                } else if self.wmctrl.is_none() {
                    self.wmctrl = spawn(
                        ui_command(
                            &prefix,
                            &directory,
                            display,
                            &directory.join("Xauthority"),
                            &wmctrl_bin,
                        )
                        .args([
                            "-r",
                            "ChatGPT",
                            "-b",
                            "add,maximized_vert,maximized_horz",
                        ]),
                    )
                    .ok();
                }
                Duration::from_millis(250)
            } else {
                // poll wakes for disconnection or SIGCHLD even with an idle host.
                Duration::from_secs(3600)
            };
            if wait_event(Some(control), signals, wait).unwrap_or(true) {
                break;
            }
        }
    }

    fn reap(&mut self) -> Reaped {
        let mut result = Reaped::default();
        loop {
            let mut status = 0;
            let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG | libc::__WALL) };
            if pid > 0 {
                if self.wmctrl == Some(pid) {
                    result.wmctrl_succeeded =
                        libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;
                }
                for tracked in [
                    &mut self.proot,
                    &mut self.display_pid,
                    &mut self.openbox,
                    &mut self.wmctrl,
                ] {
                    if *tracked == Some(pid) {
                        *tracked = None;
                    }
                }
            } else if pid == 0 {
                return result;
            } else {
                match io::Error::last_os_error().raw_os_error() {
                    Some(libc::EINTR) => continue,
                    Some(libc::ECHILD) => result.empty = true,
                    _ => {}
                }
                return result;
            }
        }
    }

    fn stop(&mut self, signals: &OwnedFd) -> Result<()> {
        self.capture_artifacts();
        self.reap();
        // Browser.close can acknowledge before Chromium has flushed its
        // profile. Let a closing guest exit before interrupting its tracer.
        if self.proot.is_some_and(catches_quit) {
            let deadline = Instant::now() + Duration::from_millis(350);
            while self.proot.is_some() && Instant::now() < deadline {
                let _ = wait_event(None, signals, WAIT_TICK);
                self.reap();
            }
        }
        // Only send QUIT once PRoot advertises a handler. An early default-action
        // QUIT would kill the tracer first. The descendant sweep covers either path.
        if let Some(pid) = self.proot.filter(|&pid| catches_quit(pid)) {
            signal_child(pid, libc::SIGQUIT);
            let deadline = Instant::now() + Duration::from_millis(800);
            while self.proot.is_some() && Instant::now() < deadline {
                let _ = wait_event(None, signals, WAIT_TICK);
                self.reap();
            }
        }
        self.capture_artifacts();
        for pid in [self.openbox, self.display_pid, self.wmctrl]
            .into_iter()
            .flatten()
        {
            signal_child(pid, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_millis(200);
        while self.proot.is_none() && !self.reap().empty && Instant::now() < deadline {
            let _ = wait_event(None, signals, WAIT_TICK);
        }
        let report_at = Instant::now() + Duration::from_secs(4);
        let mut warned = false;
        while !self.reap().empty {
            if let Err(error) = kill_descendants(self.proot) {
                if !warned {
                    eprintln!("chatgpt-provider: AppHost cleanup pending: {error}");
                    warned = true;
                }
            } else if !warned && Instant::now() >= report_at {
                eprintln!("chatgpt-provider: AppHost cleanup pending; profile remains locked");
                warned = true;
            }
            let _ = wait_event(
                None,
                signals,
                if warned {
                    Duration::from_millis(200)
                } else {
                    WAIT_TICK
                },
            );
        }
        let display = self
            .artifacts
            .take()
            .map_or(Ok(()), DisplayArtifacts::cleanup);
        let directory = self.directory.take().map_or(Ok(()), TemporaryDir::cleanup);
        self.lease.take();
        display
            .and(directory)
            .map_err(|error| host_io("host_artifact_cleanup_failed", error))
    }
}

#[derive(Default)]
struct Reaped {
    empty: bool,
    wmctrl_succeeded: bool,
}

fn ui_command(prefix: &Path, path: &Path, display: u16, auth: &Path, executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .env_clear()
        .env("HOME", path)
        .env("PATH", prefix.join("bin"))
        .env("PREFIX", prefix)
        .env("DISPLAY", format!(":{display}"))
        .env("XAUTHORITY", auth)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn signal_child(pid: i32, signal: libc::c_int) {
    // Direct child identity remains reserved until this sole waiter reaps it.
    unsafe {
        libc::kill(pid, signal);
    }
}

fn catches_quit(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/status")).is_ok_and(|status| {
        status
            .lines()
            .find_map(|line| line.strip_prefix("SigCgt:"))
            .and_then(|mask| u64::from_str_radix(mask.trim(), 16).ok())
            .is_some_and(|mask| mask & (1 << (libc::SIGQUIT - 1)) != 0)
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Process {
    parent: i32,
    started: u64,
    zombie: bool,
}

fn snapshot() -> io::Result<std::collections::HashMap<i32, Process>> {
    let mut processes = std::collections::HashMap::new();
    let uid = unsafe { libc::geteuid() };
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<i32>().ok())
        else {
            continue;
        };
        if !entry.metadata().is_ok_and(|metadata| metadata.uid() == uid) {
            continue;
        }
        if let Ok(stat) = fs::read_to_string(entry.path().join("stat")) {
            if let Some((_, fields)) = stat.rsplit_once(')') {
                let mut fields = fields.split_whitespace();
                if let (Some(state), Some(parent), Some(started)) = (
                    fields.next(),
                    fields.next().and_then(|p| p.parse().ok()),
                    fields.nth(17).and_then(|s| s.parse().ok()),
                ) {
                    processes.insert(
                        pid,
                        Process {
                            parent,
                            started,
                            zombie: state == "Z",
                        },
                    );
                }
            }
        }
    }
    Ok(processes)
}

fn is_descendant(processes: &std::collections::HashMap<i32, Process>, pid: i32, root: i32) -> bool {
    let mut current = pid;
    for _ in 0..processes.len() {
        let Some(process) = processes.get(&current) else {
            break;
        };
        if process.parent == root {
            return true;
        }
        current = process.parent;
    }
    false
}

fn kill_descendants(proot: Option<i32>) -> io::Result<()> {
    let root = std::process::id() as i32;
    let before = snapshot()?;
    if let Err(error) = kill_with_pidfds(&before, root, proot) {
        if !matches!(
            error.raw_os_error(),
            Some(libc::ENOSYS | libc::EINVAL | libc::EPERM)
        ) {
            return Err(error);
        }
        // With SIGCHLD at its default and no concurrent waiter, the kernel
        // reserves direct child PIDs until reaped. Orphans become direct
        // children of this subreaper for the next pass.
        let owned = snapshot()?;
        for (&pid, process) in &owned {
            if process.parent == root && !process.zombie && Some(pid) != proot {
                signal_child(pid, libc::SIGKILL);
            }
        }
        if let Some(pid) = proot {
            signal_child(pid, libc::SIGKILL);
        }
    }
    Ok(())
}

fn kill_with_pidfds(
    before: &std::collections::HashMap<i32, Process>,
    root: i32,
    proot: Option<i32>,
) -> io::Result<()> {
    let mut handles = Vec::new();
    for (&pid, &process) in before {
        if process.zombie || !is_descendant(before, pid, root) {
            continue;
        }
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) } as i32;
        if fd < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                continue;
            }
            return Err(error);
        }
        handles.push((pid, process.started, unsafe { OwnedFd::from_raw_fd(fd) }));
    }
    let after = snapshot()?;
    // Stop producers before killing children, and leave the tracer running
    // until every observed guest has been signaled.
    handles.retain(|(pid, started, _)| {
        after
            .get(pid)
            .is_some_and(|process| process.started == *started && !process.zombie)
            && is_descendant(&after, *pid, root)
    });
    for (_, _, fd) in handles.iter().filter(|(pid, _, _)| Some(*pid) != proot) {
        pidfd_signal(fd, libc::SIGSTOP)?;
    }
    handles.sort_by_key(|(pid, _, _)| Some(*pid) == proot);
    for (_, _, fd) in &handles {
        pidfd_signal(fd, libc::SIGKILL)?;
    }
    Ok(())
}

fn pidfd_signal(fd: &OwnedFd, signal: libc::c_int) -> io::Result<()> {
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            signal,
            std::ptr::null::<libc::siginfo_t>(),
            0u32,
        )
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error);
        }
    }
    Ok(())
}
struct TemporaryDir(PathBuf);
impl TemporaryDir {
    fn cleanup(self) -> io::Result<()> {
        // PRoot creates this empty, mode-000 bind placeholder. rmdir only
        // needs access to its parent. remove_dir_all tries to open it first.
        let _ = fs::remove_dir(self.0.join("tmp/.X11-unix"));
        fs::remove_dir_all(self.0)
    }
}

struct DisplayArtifacts(Vec<(PathBuf, u64, u64)>);

impl DisplayArtifacts {
    fn capture(prefix: &Path, display: u16, pid: i32) -> Option<Self> {
        let lock = prefix.join(format!("tmp/.X{display}-lock"));
        let lock_meta = fs::symlink_metadata(&lock).ok()?;
        let uid = unsafe { libc::geteuid() };
        if !lock_meta.is_file()
            || lock_meta.uid() != uid
            || fs::read_to_string(&lock).ok()?.trim().parse::<i32>().ok()? != pid
        {
            return None;
        }
        let mut records = vec![(lock, lock_meta.dev(), lock_meta.ino())];
        let socket = prefix.join(format!("tmp/.X11-unix/X{display}"));
        if let Ok(meta) = fs::symlink_metadata(&socket) {
            if meta.file_type().is_socket() && meta.uid() == uid {
                records.insert(0, (socket, meta.dev(), meta.ino()));
            }
        }
        Some(Self(records))
    }
}

impl DisplayArtifacts {
    fn cleanup(self) -> io::Result<()> {
        let mut result = Ok(());
        for (path, dev, ino) in self.0 {
            if fs::symlink_metadata(&path).is_ok_and(|m| m.dev() == dev && m.ino() == ino) {
                if let Err(error) = fs::remove_file(path) {
                    if error.kind() != io::ErrorKind::NotFound {
                        result = Err(error);
                    }
                }
            }
        }
        result
    }
}

fn write_xauthority(path: &Path, display: u16) -> Result<()> {
    let raw = random_hex(16)?;
    let cookie: Vec<u8> = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&65535_u16.to_be_bytes());
    for field in [
        b"".as_slice(),
        display.to_string().as_bytes(),
        b"MIT-MAGIC-COOKIE-1".as_slice(),
        &cookie,
    ] {
        bytes.extend_from_slice(&(field.len() as u16).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|e| ProviderError::internal("host_authority_failed", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn check_singleton_allows_missing_dead_and_non_chatgpt_recycled_pids() {
        let dir = std::env::temp_dir().join(format!(
            "cgp-singleton-test-{}",
            random_hex(6).expect("random hex")
        ));
        fs::create_dir_all(&dir).unwrap();
        // Missing SingletonLock is fine.
        assert!(check_singleton(&dir).is_ok());

        // Dead PID is treated as stale.
        let lock = dir.join("SingletonLock");
        symlink("localhost-9999999", &lock).unwrap();
        assert!(check_singleton(&dir).is_ok());
        let _ = fs::remove_file(&lock);

        // Live non-ChatGPT PID (this test binary's own cmdline contains `chatgpt_provider`,
        // so test with a short-lived `sleep` child whose cmdline is clearly not ChatGPT).
        let mut child = Command::new("sleep").arg("10").spawn().unwrap();
        symlink(format!("localhost-{}", child.id()), &lock).unwrap();
        let res = check_singleton(&dir);
        let _ = child.kill();
        let _ = child.wait();
        assert!(
            res.is_ok(),
            "recycled non-ChatGPT PID should be stale: {res:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
