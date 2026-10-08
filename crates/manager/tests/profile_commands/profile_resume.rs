use super::profile_snapshot::inventory;
use super::*;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

const ID: &str = "12345678-1234-1234-1234-123456789abc";

fn setup() -> (TestRoot, PathBuf, PathBuf) {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let home = root.0.join("home");
    fs::create_dir(&home).unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        run_manager(&home, &core, &["profile", "create", "work"], None)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(
        run_manager(&home, &core, &["profile", "default", "work"], None)
            .status
            .code(),
        Some(0)
    );
    (root, core, home)
}

#[test]
fn profile_resume_validates_exact_grammar_and_destination_without_writes() {
    let (_root, core, home) = setup();
    for args in [
        vec!["__profile-resume-v1"],
        vec!["__profile-resume-v1", "work"],
        vec!["__profile-resume-v1", "work", ID, "extra"],
        vec!["__profile-resume-v1", "../work", ID],
        vec!["__profile-resume-v1", "home", ID],
        vec!["__profile-resume-v1", "work", "bad-thread"],
        vec![
            "__profile-resume-v1",
            "work",
            "12345678-1234-1234-1234-123456789ABC",
        ],
    ] {
        let before = inventory(&home);
        let output = run_manager(&home, &core, &args, None);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(inventory(&home), before);
    }
    let before = inventory(&home);
    let output = run_manager(&home, &core, &["__profile-resume-v1", "missing", ID], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(inventory(&home), before);
}

#[test]
fn profile_resume_reenters_exact_conversation_and_home_preserving_state_and_exit() {
    let (_root, core, home) = setup();
    let body = fs::read_to_string(&core).unwrap();
    fs::write(
        &core,
        body.replace("exit 37", "printf 'CWD=%s\\n' \"$PWD\"\nexit 37"),
    )
    .unwrap();
    let account = home.join(".local/share/codex/manager/profiles/work/home");
    fs::write(account.join("config.toml"), b"owned-config-sentinel").unwrap();
    let default = home.join(".codex");
    fs::create_dir(&default).unwrap();
    fs::set_permissions(&default, fs::Permissions::from_mode(0o700)).unwrap();
    for (target, expected_home) in [("work", &account), ("default", &default)] {
        let before = inventory(&home);
        let output = base_manager_command(&home, &core)
            .current_dir(&home)
            .env("CODEX_HOME", &default)
            .env("CODEX_SQLITE_HOME", "/unrelated")
            .args(["__profile-resume-v1", target, ID])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        assert_eq!(output.stderr, b"PROBE_STDERR\n");
        assert_eq!(String::from_utf8(output.stdout).unwrap(), format!(
            "API_SET=\nENTRY_SET=\nHOME_SET=x\nHOME_VALUE={}\nSQLITE_HOME_SET=\nSQLITE_HOME_VALUE=\nCALLER_SENTINEL=keep\nARGC=2\nARG=<resume>\nARG=<{ID}>\nCWD={}\n", expected_home.display(), home.display()));
        assert_eq!(inventory(&home), before);
    }
}

#[test]
fn profile_resume_refuses_held_coordination_or_writer_without_stopping_or_unlinking() {
    let (_root, core, home) = setup();
    let dir = home.join(".codex/thread-writer-locks");
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    let coordination = dir.join(".coordination.lock");
    let writer = dir.join(format!("{ID}.lock"));
    for path in [&coordination, &writer] {
        fs::write(path, b"owned-native-lock-sentinel").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    for path in [&coordination, &writer] {
        let held = fs::File::open(path).unwrap();
        held.try_lock().unwrap();
        let before = inventory(&home);
        let started = std::time::Instant::now();
        let output = run_manager(&home, &core, &["__profile-resume-v1", "work", ID], None);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("writer is still owned"));
        assert_eq!(inventory(&home), before);
        // Still held by this original descriptor after the failed switch.
        let probe = fs::File::open(path).unwrap();
        assert!(matches!(
            probe.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        drop(held);
    }
    let held = fs::File::open(&writer).unwrap();
    held.try_lock().unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        drop(held);
    });
    let before = inventory(&home);
    let output = run_manager(&home, &core, &["__profile-resume-v1", "work", ID], None);
    release.join().unwrap();
    assert_eq!(output.status.code(), Some(37));
    assert_eq!(inventory(&home), before);
}

#[test]
fn profile_resume_keeps_selected_registry_stable_during_writer_release() {
    let (_root, core, home) = setup();
    let dir = home.join(".codex/thread-writer-locks");
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    for name in [".coordination.lock".to_string(), format!("{ID}.lock")] {
        let path = dir.join(name);
        fs::write(&path, b"").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let held = fs::File::open(dir.join(format!("{ID}.lock"))).unwrap();
    held.try_lock().unwrap();
    let registry = fs::File::open(home.join(".local/share/codex/manager/profiles")).unwrap();
    let mut child = base_manager_command(&home, &core)
        .args(["__profile-resume-v1", "work", ID])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match registry.try_lock() {
            Err(std::fs::TryLockError::WouldBlock) => break,
            Ok(()) => registry.unlock().unwrap(),
            Err(error) => panic!("owned registry probe failed: {error}"),
        }
        assert!(std::time::Instant::now() < deadline);
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let before = inventory(&home);
    let rename = run_manager(
        &home,
        &core,
        &["profile", "rename", "work", "renamed"],
        None,
    );
    assert_eq!(rename.status.code(), Some(1));
    assert!(String::from_utf8(rename.stderr)
        .unwrap()
        .contains("profile is in use"));
    assert_eq!(inventory(&home), before);
    drop(held);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(37));
    assert_eq!(inventory(&home), before);
}

struct Terminal {
    child: Child,
    master: fs::File,
    output: Vec<u8>,
}

impl Terminal {
    fn spawn(command: &mut Command, redirected: Option<i32>) -> Self {
        let (mut master, mut slave) = (-1, -1);
        // SAFETY: openpty fills two owned descriptors; no borrowed pointers.
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
        assert_eq!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
            0
        );
        command
            .stdin(slave.try_clone().unwrap())
            .stdout(slave.try_clone().unwrap())
            .stderr(slave);
        match redirected {
            Some(0) => {
                command.stdin(Stdio::null());
            }
            Some(1) => {
                command.stdout(Stdio::null());
            }
            Some(2) => {
                command.stderr(Stdio::null());
            }
            None => {}
            _ => panic!("invalid owned stream"),
        }
        Self {
            child: command.spawn().unwrap(),
            master,
            output: Vec::new(),
        }
    }

    fn drain(&mut self) {
        let mut bytes = [0; 4096];
        loop {
            match self.master.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => self.output.extend_from_slice(&bytes[..n]),
                Err(error)
                    if error.kind() == ErrorKind::WouldBlock
                        || error.raw_os_error() == Some(libc::EIO) =>
                {
                    break
                }
                Err(error) => panic!("owned PTY read: {error}"),
            }
        }
    }

    fn prompt(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.drain();
            if String::from_utf8_lossy(&self.output).contains("Enter to return") {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "{}",
                String::from_utf8_lossy(&self.output)
            );
            assert!(Instant::now() < deadline, "source return did not prompt");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn finish(&mut self, input: &[u8]) -> (ExitStatus, String) {
        if !input.is_empty() {
            self.master.write_all(input).unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.drain();
            if let Some(status) = self.child.try_wait().unwrap() {
                self.drain();
                return (status, String::from_utf8_lossy(&self.output).into_owned());
            }
            assert!(
                Instant::now() < deadline,
                "owned source return did not finish"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            self.child.kill().unwrap();
            self.child.wait().unwrap();
        }
    }
}

fn source_home(root: &TestRoot) -> PathBuf {
    let path = root.0.join("original-account");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
fn profile_resume_tty_refusal_returns_only_explicit_original_home_same_pid_cwd_uuid() {
    let (root, core, home) = setup();
    let source = source_home(&root);
    let cwd = root.0.join("current-workspace");
    fs::create_dir(&cwd).unwrap();
    let body = fs::read_to_string(&core).unwrap();
    fs::write(&core, body.replace("exit 37", "[ -t 0 ] && [ -t 1 ] && [ -t 2 ] || exit 88\nprintf 'PID=%s\\nCWD=%s\\n' \"$$\" \"$PWD\"\nexit 37")).unwrap();
    let locks = home.join(".codex/thread-writer-locks");
    fs::create_dir_all(&locks).unwrap();
    fs::set_permissions(&locks, fs::Permissions::from_mode(0o700)).unwrap();
    let writer = locks.join(format!("{ID}.lock"));
    fs::write(&writer, b"owned-writer-sentinel").unwrap();
    fs::set_permissions(&writer, fs::Permissions::from_mode(0o600)).unwrap();
    let held = fs::File::open(&writer).unwrap();
    held.try_lock().unwrap();
    let before = inventory(&home);
    for target in ["work", "missing"] {
        let mut command = base_manager_command(&home, &core);
        command.env("CODEX_HOME", &source).current_dir(&cwd).args([
            "__profile-resume-v1",
            target,
            ID,
        ]);
        let mut terminal = Terminal::spawn(&mut command, None);
        let pid = terminal.child.id();
        terminal.prompt();
        assert_eq!(inventory(&home), before);
        let (status, output) = terminal.finish(b"\n");
        assert_eq!(status.code(), Some(37), "{output}");
        for expected in [
            format!("HOME_VALUE={}", source.display()),
            format!("PID={pid}"),
            format!("CWD={}", cwd.display()),
            format!("ARG=<{ID}>"),
            "ARG=<resume>".into(),
            "API_SET=\r\n".into(),
            "ENTRY_SET=\r\n".into(),
        ] {
            assert!(output.contains(&expected), "missing {expected}: {output}");
        }
        assert_eq!(inventory(&home), before);
        assert!(matches!(
            fs::File::open(&writer).unwrap().try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
    }
}

#[test]
fn profile_resume_tty_cancel_eof_and_bounded_input_never_confirm_return() {
    let (root, core, home) = setup();
    let source = source_home(&root);
    let before = inventory(&home);
    for input in [b"q\n".as_slice(), b"other\n", b"\x04", &[b'x'; 128]] {
        let mut command = base_manager_command(&home, &core);
        command
            .env("CODEX_HOME", &source)
            .args(["__profile-resume-v1", "missing", ID]);
        let mut terminal = Terminal::spawn(&mut command, None);
        terminal.prompt();
        let mut input = input.to_vec();
        if input.len() == 128 {
            input.push(b'\n');
        }
        let (status, output) = terminal.finish(&input);
        assert_eq!(status.code(), Some(130), "{output}");
        assert!(!output.contains("HOME_VALUE="), "{output}");
        assert_eq!(inventory(&home), before);
    }
}

#[test]
fn profile_resume_tty_missing_unsafe_source_or_any_redirected_stream_does_not_prompt() {
    let (root, core, home) = setup();
    let source = source_home(&root);
    let symlink = root.0.join("source-link");
    std::os::unix::fs::symlink(&source, &symlink).unwrap();
    for inherited in [
        None,
        Some(symlink.as_path()),
        Some(root.0.join("missing").as_path()),
        Some(Path::new("")),
    ] {
        let mut command = base_manager_command(&home, &core);
        if let Some(path) = inherited {
            command.env("CODEX_HOME", path);
        }
        command.args(["__profile-resume-v1", "missing", ID]);
        let (status, output) = Terminal::spawn(&mut command, None).finish(b"");
        assert_eq!(status.code(), Some(1));
        assert!(!output.contains("Enter to return"));
    }
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
    let mut command = base_manager_command(&home, &core);
    command
        .env("CODEX_HOME", &source)
        .args(["__profile-resume-v1", "missing", ID]);
    let (status, output) = Terminal::spawn(&mut command, None).finish(b"");
    assert_eq!(status.code(), Some(1));
    assert!(!output.contains("Enter to return"));
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
    for stream in 0..3 {
        let mut command = base_manager_command(&home, &core);
        command
            .env("CODEX_HOME", &source)
            .args(["__profile-resume-v1", "missing", ID]);
        let (status, output) = Terminal::spawn(&mut command, Some(stream)).finish(b"");
        assert_eq!(status.code(), Some(1));
        assert!(!output.contains("Enter to return"));
    }
    let mut command = base_manager_command(&home, &core);
    command
        .env("CODEX_HOME", &source)
        .args(["__profile-resume-v1", "missing", "invalid"]);
    let (status, output) = Terminal::spawn(&mut command, None).finish(b"");
    assert_eq!(status.code(), Some(2));
    assert!(!output.contains("Enter to return"));
}

#[test]
fn profile_resume_tty_revalidates_original_physical_identity_and_private_mode() {
    let (root, core, home) = setup();
    let source = source_home(&root);
    for replace in [false, true] {
        let mut command = base_manager_command(&home, &core);
        command
            .env("CODEX_HOME", &source)
            .args(["__profile-resume-v1", "missing", ID]);
        let mut terminal = Terminal::spawn(&mut command, None);
        terminal.prompt();
        if replace {
            fs::rename(&source, root.0.join("retired-original-account")).unwrap();
            fs::create_dir(&source).unwrap();
            fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        } else {
            fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let (status, output) = terminal.finish(b"\n");
        assert_eq!(status.code(), Some(1), "{output}");
        assert!(!output.contains("HOME_VALUE="));
        if replace {
            assert!(output.contains("original profile changed"));
        }
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

#[test]
fn profile_resume_tty_success_executes_destination_without_return_prompt() {
    let (root, core, home) = setup();
    let source = source_home(&root);
    let before = inventory(&home);
    let mut command = base_manager_command(&home, &core);
    command
        .env("CODEX_HOME", &source)
        .args(["__profile-resume-v1", "work", ID]);
    let (status, output) = Terminal::spawn(&mut command, None).finish(b"");
    assert_eq!(status.code(), Some(37));
    assert!(!output.contains("Enter to return"));
    assert!(output.contains(&format!(
        "HOME_VALUE={}",
        home.join(".local/share/codex/manager/profiles/work/home")
            .display()
    )));
    assert_eq!(inventory(&home), before);
}
