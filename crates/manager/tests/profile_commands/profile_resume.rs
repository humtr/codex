use super::profile_snapshot::inventory;
use super::*;

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
