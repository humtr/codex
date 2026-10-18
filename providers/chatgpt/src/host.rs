//! Provider-owned official-app process and private pipe, with no Bun/Python host.
//! PRoot is compatibility, not a filesystem sandbox or a native tool executor.
use crate::{
    cdp::Client,
    config::HostConfig,
    journal::{private_directory, random_id},
    protocol::Error,
};
use serde_json::json;
use std::{
    fs::{self, File},
    io::Write,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{
            fs::{MetadataExt, OpenOptionsExt},
            net::UnixStream,
        },
    },
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::{Child, Command};

struct ChildGroup {
    child: Child,
    pid: u32,
}
impl ChildGroup {
    fn spawn(command: &mut Command) -> Result<Self, Error> {
        command.kill_on_drop(true).process_group(0);
        let parent = unsafe { libc::getpid() };
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
                    || libc::getppid() != parent
                {
                    return Err(std::io::Error::other("parent unavailable"));
                }
                Ok(())
            });
        }
        let child = command
            .spawn()
            .map_err(|_| Error("host_process_start_failed"))?;
        let pid = child.id().ok_or(Error("host_process_start_failed"))?;
        Ok(Self { child, pid })
    }
    async fn stop(&mut self) {
        // Keep the leader unreaped until signalling its private group: its PID cannot
        // be recycled into an unrelated group while this child handle owns it.
        unsafe {
            libc::kill(-(self.pid as i32), libc::SIGTERM);
        }
        if tokio::time::timeout(Duration::from_secs(3), self.child.wait())
            .await
            .is_err()
        {
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
            let _ = self.child.wait().await;
        }
        self.pid = 0;
    }
}
impl Drop for ChildGroup {
    fn drop(&mut self) {
        if self.pid != 0 {
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
        }
    }
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct DisplayArtifacts(Vec<(PathBuf, u64, u64)>);
impl DisplayArtifacts {
    fn capture(prefix: &Path, display: u16, pid: u32) -> Result<Self, Error> {
        let lock = prefix.join(format!("tmp/.X{display}-lock"));
        if fs::read_to_string(&lock)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            != Some(pid)
        {
            return Err(Error("host_display_owner_unconfirmed"));
        }
        let paths = [lock, prefix.join(format!("tmp/.X11-unix/X{display}"))];
        let mut records = Vec::new();
        for path in paths {
            let m =
                fs::symlink_metadata(&path).map_err(|_| Error("host_display_owner_unconfirmed"))?;
            if m.uid() != unsafe { libc::geteuid() } {
                return Err(Error("host_display_owner_unconfirmed"));
            }
            records.push((path, m.dev(), m.ino()));
        }
        Ok(Self(records))
    }
}
impl Drop for DisplayArtifacts {
    fn drop(&mut self) {
        // Group guards have already sent SIGKILL on abnormal exit. Compare the
        // exact objects we claimed; never unlink a later display owner's socket.
        for (path, dev, ino) in &self.0 {
            if fs::symlink_metadata(path).is_ok_and(|m| m.dev() == *dev && m.ino() == *ino) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

struct Display {
    child: ChildGroup,
    _artifacts: DisplayArtifacts,
}
impl Display {
    async fn stop(&mut self) {
        self.child.stop().await;
    }
}

pub struct OwnedHost {
    pub client: Client,
    app: ChildGroup,
    display: Display,
    _lease: File,
    _temporary: Temporary,
}
fn plain_directory(path: &Path) -> Result<(), Error> {
    let meta = fs::symlink_metadata(path).map_err(|_| Error("host_directory_unavailable"))?;
    if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(Error("owned_host_directory_required"));
    }
    Ok(())
}
fn authority(path: &Path, display: u16) -> Result<(), Error> {
    let raw = random_id()?;
    let cookie = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
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
    File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|_| Error("host_authority_unavailable"))
}
impl OwnedHost {
    pub async fn start(config: &HostConfig, state: &Path) -> Result<Self, Error> {
        let rootfs = config
            .rootfs
            .as_ref()
            .ok_or(Error("host_rootfs_required"))?;
        plain_directory(rootfs)?;
        if !rootfs.join("usr/lib/chatgpt/resources/app.asar").is_file()
            || !rootfs.join("usr/bin/chatgpt").is_file()
        {
            return Err(Error("official_app_rootfs_required"));
        }
        let prefix =
            PathBuf::from(std::env::var_os("PREFIX").ok_or(Error("termux_prefix_required"))?);
        if !prefix.is_absolute() {
            return Err(Error("termux_prefix_required"));
        }
        let desktop = state.join("desktop");
        private_directory(&desktop)?;
        let home = desktop.join("home");
        private_directory(&home)?;
        let lease = File::open(&home).map_err(|_| Error("host_profile_unavailable"))?;
        if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(Error("host_profile_busy"));
        }
        let path = std::env::temp_dir().join(format!("ch-{}", &random_id()?[..8]));
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| Error("host_temporary_unavailable"))?;
        let temporary = Temporary(path);
        let path = &temporary.0;
        for name in ["tmp", "shm", "links"] {
            private_directory(&path.join(name))?;
        }
        let socket_root = prefix.join("tmp/.X11-unix");
        let display = (100..2000_u16)
            .find(|id| {
                !socket_root.join(format!("X{id}")).exists()
                    && !prefix.join(format!("tmp/.X{id}-lock")).exists()
            })
            .ok_or(Error("host_display_unavailable"))?;
        let auth = path.join("Xauthority");
        authority(&auth, display)?;
        let mut xserver = Command::new(prefix.join("bin/Xvnc"));
        xserver
            .env_clear()
            .env("HOME", &home)
            .env("PATH", prefix.join("bin"))
            .env("PREFIX", &prefix)
            .env("TMPDIR", path)
            .args([
                format!(":{display}"),
                "-geometry".into(),
                "1024x768".into(),
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
        let mut display_child = ChildGroup::spawn(&mut xserver)?;
        let until = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if tokio::net::UnixStream::connect(socket_root.join(format!("X{display}")))
                .await
                .is_ok()
            {
                break;
            }
            if tokio::time::Instant::now() >= until {
                display_child.stop().await;
                return Err(Error("host_display_start_timeout"));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let artifacts = DisplayArtifacts::capture(&prefix, display, display_child.pid)?;
        let display_guard = Display {
            child: display_child,
            _artifacts: artifacts,
        };
        let (parent, child) = UnixStream::pair().map_err(|_| Error("host_pipe_unavailable"))?;
        parent
            .set_nonblocking(true)
            .map_err(|_| Error("host_pipe_unavailable"))?;
        let fd = unsafe { libc::fcntl(child.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
        if fd < 0 {
            return Err(Error("host_pipe_unavailable"));
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let source = fd.as_raw_fd();
        let root = File::open(rootfs).map_err(|_| Error("host_directory_unavailable"))?;
        let root_fd = root.as_raw_fd();
        let mut command = Command::new(prefix.join("bin/proot"));
        command
            .env_clear()
            .env("PATH", prefix.join("bin"))
            .env("HOME", &home)
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
            .arg(format!("--bind={}:/root", home.display()))
            .arg(format!("--bind={}:/tmp", path.join("tmp").display()))
            .arg(format!("--bind={}:/dev/shm", path.join("shm").display()))
            .arg(format!("--bind={}:/tmp/.X11-unix", socket_root.display()))
            .arg(format!("--bind={}:/root/.Xauthority", auth.display()))
            .args([
                "/usr/bin/env",
                "-i",
                "HOME=/root",
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "LANG=C.UTF-8",
                "XAUTHORITY=/root/.Xauthority",
            ])
            .arg(format!("DISPLAY=:{display}"))
            .args([
                "/usr/bin/dbus-run-session",
                "--",
                "/usr/bin/chatgpt",
                "--no-sandbox",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--user-data-dir=/root/profile",
                "--remote-debugging-pipe",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        unsafe {
            command.pre_exec(move || {
                if libc::fchdir(root_fd) != 0
                    || libc::dup2(source, 3) < 0
                    || libc::dup2(source, 4) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let app = ChildGroup::spawn(&mut command)?;
        drop(child);
        drop(fd);
        let parent =
            tokio::net::UnixStream::from_std(parent).map_err(|_| Error("host_pipe_unavailable"))?;
        let (read, write) = parent.into_split();
        let client = Client::pipe(read, write, random_id()?);
        let mut owned = Self {
            client,
            app,
            display: display_guard,
            _lease: lease,
            _temporary: temporary,
        };
        if owned
            .client
            .request("Browser.getVersion", json!({}), None)
            .await
            .is_err()
        {
            owned.shutdown().await;
            return Err(Error("official_app_start_failed"));
        }
        Ok(owned)
    }
    pub async fn shutdown(&mut self) {
        let _ = tokio::time::timeout(
            Duration::from_secs(3),
            self.client.request("Browser.close", json!({}), None),
        )
        .await;
        self.app.stop().await;
        self.display.stop().await;
    }
}
