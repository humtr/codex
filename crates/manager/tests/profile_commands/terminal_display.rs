//! Owned PTY/Native-owner admission proof. Android is an explicit AM fixture;
//! these tests never claim physical Activity visibility or real model inference.
use super::*;
use std::io::{Seek, SeekFrom};
use std::os::fd::FromRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixListener;
use std::time::{Duration, Instant};

fn private_dir(path: &Path) {
    // Every newly created ancestor must meet the same private-directory contract.
    // chmod on only the leaf concealed an invalid manager root under umask 077.
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn core_capability(f: &TaskFixture, reply: &str, status: i32) {
    let source = fs::read_to_string(&f.core).unwrap();
    let (shell, body) = source.split_once('\n').unwrap();
    fs::write(&f.core, format!(
        "{shell}\nif [ \"$#\" -eq 5 ] && [ \"$1\" = termux ] && [ \"$2\" = __terminal-capabilities-v1 ] && [ \"$3\" = -- ] && [ \"$4\" = resume ] && [ \"$5\" = '{TASK_ID}' ]; then printf '%s' '{reply}'; exit {status}; fi\n{body}"
    )).unwrap();
}

#[test]
fn foreground_fixture() {
    let Some(root) = std::env::var_os("DISPLAY_FIXTURE_ROOT").map(PathBuf::from) else {
        return;
    };
    // This helper runs alone in its own process. Exercise the host-CI umask even
    // when its parent is Termux, without changing the parent or operating session.
    unsafe { libc::umask(0o022) };
    let token = TASK_ID.replace('-', "");
    let pid = std::process::id();
    let start: u64 = codex_runtime_binding::native::process_start(pid)
        .unwrap()
        .parse()
        .unwrap();
    let tty = fs::read_link("/proc/self/fd/0").unwrap();
    assert!(tty.starts_with("/dev/pts"));
    let directory = root.join(".local/share/codex/manager/terminals");
    private_dir(&directory);
    let route = serde_json::json!({"kind":"stock","thread":TASK_ID,
        "name":format!("codex-{token}"),"pid":pid,"start":start,"tty":tty});
    let route_path = directory.join(format!("route-{token}"));
    fs::write(&route_path, route.to_string()).unwrap();
    fs::set_permissions(&route_path, fs::Permissions::from_mode(0o600)).unwrap();
    let slot_path = root.join(format!(".codex-terminal-thread-v1-{pid}-display-proof"));
    let mut slot = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&slot_path)
        .unwrap();
    fs::remove_file(&slot_path).unwrap();
    slot.write_all(TASK_ID.as_bytes()).unwrap();
    use std::os::fd::AsRawFd;
    let bound = Command::new(manager_binary())
        .args(["__terminal-bind-v1", &slot.as_raw_fd().to_string()])
        .env("CODEX_TERMUX_WINDOW_TOKEN", &token)
        .output()
        .unwrap();
    assert!(bound.status.success(), "{bound:?}");
    let mut current = TASK_ID.to_owned();
    fs::write(root.join("display-current"), &current).unwrap();
    fs::write(root.join("display-ready"), pid.to_string()).unwrap();
    loop {
        let requested = fs::read_to_string(root.join("display-thread")).unwrap();
        if requested == "exit" {
            return;
        }
        assert!(codex_runtime_binding::native::canonical_session_id(
            &requested
        ));
        // Match the real frontend Slot::update: an unchanged conversation is a
        // stable publication, not a stream of artificial unbound transitions.
        if requested != current {
            slot.seek(SeekFrom::Start(0)).unwrap();
            slot.set_len(0).unwrap();
            slot.write_all(requested.as_bytes()).unwrap();
            current = requested;
            fs::write(root.join("display-current"), &current).unwrap();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct Foreground {
    child: std::process::Child,
    _master: fs::File,
    _am: UnixListener,
    prefix: PathBuf,
    log: PathBuf,
    root: PathBuf,
}
impl Drop for Foreground {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Foreground {
    fn new(f: &TaskFixture, execution_home: &Path) -> Self {
        core_capability(f, "codex-terminal-capabilities-v1\nidentity=native\n", 0);
        let prefix = f.root.0.join("usr");
        private_dir(&prefix.join("bin"));
        let socket = f.root.0.join("apps/com.termux/termux-am/am.sock");
        private_dir(socket.parent().unwrap());
        use std::os::unix::ffi::OsStrExt;
        assert!(socket.as_os_str().as_bytes().len() <= 107);
        let am = UnixListener::bind(&socket).unwrap();
        let log = f.root.0.join("display-am.log");
        let client = prefix.join("bin/termux-am-socket");
        fs::write(&client, format!("#!{}\nprintf '%s\\n' \"$1\" >> \"$DISPLAY_AM_LOG\"\nprintf 'Starting service: owned fixture\\n'\n", test_shell().display())).unwrap();
        fs::set_permissions(client, fs::Permissions::from_mode(0o700)).unwrap();
        let executable = f
            .root
            .0
            .join(".local/lib/codex/core/generations/fixture-display/helpers/2");
        private_dir(executable.parent().unwrap());
        fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let (mut master, mut slave) = (-1, -1);
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            },
            0
        );
        let master = unsafe { fs::File::from_raw_fd(master) };
        let slave = unsafe { fs::File::from_raw_fd(slave) };
        fs::write(f.root.0.join("display-thread"), TASK_ID).unwrap();
        let child = Command::new(&executable)
            .args([
                "--exact",
                "terminal_display::foreground_fixture",
                "--nocapture",
            ])
            .env_clear()
            .env("HOME", &f.root.0)
            .env("CODEX_HOME", execution_home)
            .env(CORE_API_ENV, CORE_API)
            .env(CORE_ENTRYPOINT_ENV, &f.core)
            .env("DISPLAY_FIXTURE_ROOT", &f.root.0)
            .env("DISPLAY_AM_LOG", &log)
            .env("PREFIX", &prefix)
            .stdin(Stdio::from(slave))
            .stdout(Stdio::null())
            .stderr(fs::File::create(f.root.0.join("display-child.err")).unwrap())
            .spawn()
            .unwrap();
        let mut fixture = Self {
            child,
            _master: master,
            _am: am,
            prefix,
            log,
            root: f.root.0.clone(),
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while !fixture.root.join("display-ready").is_file() {
            assert!(
                fixture.child.try_wait().unwrap().is_none(),
                "{}",
                fs::read_to_string(fixture.root.join("display-child.err")).unwrap()
            );
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        fixture
    }
    fn command(&self, f: &TaskFixture, home: &Path, automatic: bool) -> Command {
        let m = fs::metadata(home).unwrap();
        let mut command = base_manager_command(&f.root.0, &f.core);
        command
            .env("PREFIX", &self.prefix)
            .env("DISPLAY_AM_LOG", &self.log)
            .arg(if automatic {
                "__terminal-auto-show-v1"
            } else {
                "__terminal-show-v1"
            })
            .arg(TASK_ID)
            .arg(home)
            .arg(m.dev().to_string())
            .arg(m.ino().to_string());
        command
    }
    fn calls(&self) -> usize {
        fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .count()
    }
    fn switch_foreground(&mut self, id: &str) {
        let pending = self.root.join("display-thread-next");
        fs::write(&pending, id).unwrap();
        fs::rename(pending, self.root.join("display-thread")).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while fs::read_to_string(self.root.join("display-current"))
            .ok()
            .as_deref()
            != Some(id)
        {
            assert!(self.child.try_wait().unwrap().is_none());
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn repeated_click_and_auto_on_focus_exact_owner_without_launch_or_writer_transfer() {
    let f = TaskFixture::new();
    let foreground = Foreground::new(&f, &f.account);
    let owner_before = fs::read(f.binding.join("owner")).unwrap();
    let writer = f
        .account
        .join(format!("thread-writer-locks/{TASK_ID}.lock"));
    let writer_before = fs::read(&writer).unwrap();
    // OFF returns before even trying the deliberately broken Owner query.
    let pid = f.binding.join("pid");
    let pid_before = fs::read(&pid).unwrap();
    fs::write(&pid, b"1\n").unwrap();
    assert!(foreground
        .command(&f, &f.account, true)
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(foreground.calls(), 0);
    fs::write(&pid, &pid_before).unwrap();
    // A verified running foreground remains focusable even if a new qualified
    // frontend can no longer be constructed from the active Core generation.
    core_capability(&f, "codex-terminal-capabilities-v1\nidentity=title\n", 0);
    for _ in 0..2 {
        let output = foreground.command(&f, &f.account, false).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8(output.stdout)
            .unwrap()
            .contains("visibility is not confirmed"));
    }
    assert!(run_manager(
        &f.root.0,
        &f.core,
        &["notify", "set", "--auto-display", "on"],
        None
    )
    .status
    .success());
    assert!(foreground
        .command(&f, &f.account, true)
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(foreground.calls(), 3);
    let calls = fs::read_to_string(&foreground.log).unwrap();
    assert!(calls.lines().all(|line| {
        line.contains("/bin/true")
            && !line.contains("__terminal-start-v1")
            && line.contains("--es com.termux.RUN_COMMAND_SESSION_ACTION 0")
            && !line.contains("--ei com.termux.RUN_COMMAND_SESSION_ACTION")
    }));
    assert_eq!(fs::read(f.binding.join("owner")).unwrap(), owner_before);
    assert_eq!(fs::read(&writer).unwrap(), writer_before);
    assert!(fs::File::open(&writer).unwrap().try_lock().is_err());
    assert_eq!(fs::read(f.binding.join("pid")).unwrap(), pid_before);
}

#[test]
fn existing_terminal_never_bypasses_owner_or_execution_home_admission() {
    let f = TaskFixture::new();
    let foreign = f.root.0.join("external-b");
    private_dir(&foreign);
    let foreground = Foreground::new(&f, &foreign);
    let rejected = foreground.command(&f, &f.account, false).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(foreground.calls(), 0);
    // Supplying the foreign home cannot override Native's authoritative owner.
    let rejected = foreground.command(&f, &foreign, false).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(foreground.calls(), 0);
    fs::write(f.binding.join("pid"), b"1\n").unwrap();
    let rejected = foreground.command(&f, &f.account, false).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(foreground.calls(), 0);
}

#[test]
fn old_route_cannot_focus_a_different_foreground_thread_or_unbound_surface() {
    let f = TaskFixture::new();
    let mut foreground = Foreground::new(&f, &f.account);
    foreground.switch_foreground(OTHER_TASK_ID);
    let rejected = foreground.command(&f, &f.account, false).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8(rejected.stderr)
        .unwrap()
        .contains("display is pending"));
    assert_eq!(foreground.calls(), 0);
    foreground.switch_foreground(TASK_ID);
    let directory = f.root.0.join(".local/share/codex/manager/terminals");
    for entry in fs::read_dir(&directory).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name().to_string_lossy().starts_with("binding-") {
            fs::remove_file(entry.path()).unwrap();
        }
    }
    for _ in 0..2 {
        let rejected = foreground.command(&f, &f.account, false).output().unwrap();
        assert_eq!(rejected.status.code(), Some(1));
        assert!(String::from_utf8(rejected.stderr)
            .unwrap()
            .contains("display is pending"));
    }
    assert_eq!(foreground.calls(), 0);
    assert!(!directory
        .join(format!("launch-{}", TASK_ID.replace('-', "")))
        .exists());
}

#[test]
fn display_resume_preserves_exact_live_owner_and_refuses_foreign_home() {
    let f = TaskFixture::new();
    core_capability(&f, "codex-terminal-capabilities-v1\nidentity=native\n", 0);
    let owner_before = fs::read(f.binding.join("owner")).unwrap();
    let writer = f
        .account
        .join(format!("thread-writer-locks/{TASK_ID}.lock"));
    let writer_before = fs::read(&writer).unwrap();
    let m = fs::metadata(&f.account).unwrap();
    let output = base_manager_command(&f.root.0, &f.core)
        .arg("__terminal-display-v1")
        .arg(TASK_ID)
        .arg(&f.account)
        .arg(m.dev().to_string())
        .arg(m.ino().to_string())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(37));
    let output = String::from_utf8(output.stdout).unwrap();
    let args: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with("ARG=<"))
        .collect();
    assert_eq!(args, ["ARG=<resume>", &format!("ARG=<{TASK_ID}>")]);
    assert!(!output.contains("ARG=<--remote>"));
    assert!(output.contains(&format!("HOME_VALUE={}", f.account.display())));
    assert!(!output.contains("ARG=<exec>") && !output.contains("ARG=<--model>"));
    let foreign = f.root.0.join("foreign");
    private_dir(&foreign);
    let m = fs::metadata(&foreign).unwrap();
    let refused = base_manager_command(&f.root.0, &f.core)
        .arg("__terminal-display-v1")
        .arg(TASK_ID)
        .arg(&foreign)
        .arg(m.dev().to_string())
        .arg(m.ino().to_string())
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty());
    assert_eq!(fs::read(f.binding.join("owner")).unwrap(), owner_before);
    assert_eq!(fs::read(&writer).unwrap(), writer_before);
    assert!(fs::File::open(&writer).unwrap().try_lock().is_err());
}

#[test]
fn missing_or_unqualified_foreground_capability_never_constructs_or_reenters() {
    for (reply, status) in [
        ("codex-terminal-capabilities-v1\nidentity=title\n", 0),
        (
            "codex-terminal-capabilities-v1\nidentity=native\nextra\n",
            0,
        ),
        ("codex-terminal-capabilities-v1\nidentity=native\n", 1),
        ("", 0),
    ] {
        let f = TaskFixture::new();
        core_capability(&f, reply, status);
        let owner_before = fs::read(f.binding.join("owner")).unwrap();
        let writer = f
            .account
            .join(format!("thread-writer-locks/{TASK_ID}.lock"));
        let writer_before = fs::read(&writer).unwrap();
        let m = fs::metadata(&f.account).unwrap();
        for action in ["__terminal-show-v1", "__terminal-display-v1"] {
            let rejected = base_manager_command(&f.root.0, &f.core)
                .args([action, TASK_ID])
                .arg(&f.account)
                .arg(m.dev().to_string())
                .arg(m.ino().to_string())
                .output()
                .unwrap();
            assert_eq!(rejected.status.code(), Some(1));
            assert!(String::from_utf8(rejected.stderr)
                .unwrap()
                .contains("requires a qualified foreground frontend"));
            assert!(rejected.stdout.is_empty());
        }
        assert!(!f
            .root
            .0
            .join(".local/share/codex/manager/terminals")
            .exists());
        assert_eq!(fs::read(f.binding.join("owner")).unwrap(), owner_before);
        assert_eq!(fs::read(&writer).unwrap(), writer_before);
        assert!(fs::File::open(&writer).unwrap().try_lock().is_err());
    }
}

#[test]
fn concurrent_headless_display_observes_original_intent_before_android_reply() {
    let f = TaskFixture::new();
    let mut foreground = Foreground::new(&f, &f.account);
    foreground.child.kill().unwrap();
    foreground.child.wait().unwrap();
    assert!(run_manager(
        &f.root.0,
        &f.core,
        &["notify", "set", "--auto-display", "on"],
        None
    )
    .status
    .success());
    let release = f.root.0.join("display-am-release");
    let client = foreground.prefix.join("bin/termux-am-socket");
    fs::write(&client, format!(
        "#!{}\nprintf '%s\\n' \"$1\" >> \"$DISPLAY_AM_LOG\"\nwhile [ ! -f \"$DISPLAY_AM_RELEASE\" ]; do sleep 0.01; done\nprintf 'Starting service: owned delayed fixture\\n'\n",
        test_shell().display()
    )).unwrap();
    let owner_before = fs::read(f.binding.join("owner")).unwrap();
    let writer = f
        .account
        .join(format!("thread-writer-locks/{TASK_ID}.lock"));
    let writer_before = fs::read(&writer).unwrap();
    struct Pending {
        child: Option<std::process::Child>,
        release: PathBuf,
    }
    impl Drop for Pending {
        fn drop(&mut self) {
            let _ = fs::write(&self.release, b"release");
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let mut first = Pending {
        child: Some(
            foreground
                .command(&f, &f.account, false)
                .env("DISPLAY_AM_RELEASE", &release)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        ),
        release: release.clone(),
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while foreground.calls() == 0 {
        assert!(first.child.as_mut().unwrap().try_wait().unwrap().is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let directory = f.root.0.join(".local/share/codex/manager/terminals");
    let launch = directory.join(format!("launch-{}", TASK_ID.replace('-', "")));
    let launch_before = fs::read(&launch).unwrap();
    // Android is deliberately still waiting. The launch's filesystem lock
    // cannot exclude an independent display admission during that wait.
    let lock = fs::File::open(&directory).unwrap();
    let short_critical_section = lock.try_lock().is_ok();
    drop(lock);
    let second = foreground
        .command(&f, &f.account, true)
        .env("DISPLAY_AM_RELEASE", &release)
        .output()
        .unwrap();
    let calls_before_release = foreground.calls();
    let preserved = fs::read(&launch).unwrap() == launch_before;
    fs::write(&release, b"release").unwrap();
    let first = first.child.take().unwrap().wait_with_output().unwrap();
    assert!(
        short_critical_section,
        "Android dispatch must not hold the launch directory"
    );
    for result in [first, second] {
        assert_eq!(result.status.code(), Some(1), "{result:?}");
        assert!(String::from_utf8(result.stderr)
            .unwrap()
            .contains("display is pending"));
    }
    assert_eq!(
        calls_before_release, 1,
        "click and auto display must share the original intent"
    );
    assert_eq!(foreground.calls(), 1);
    assert!(preserved);
    assert_eq!(fs::read(f.binding.join("owner")).unwrap(), owner_before);
    assert_eq!(fs::read(&writer).unwrap(), writer_before);
    assert!(fs::File::open(&writer).unwrap().try_lock().is_err());
}
