use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const CORE_API_ENV: &str = "CODEX_TERMUX_CORE_API";
const CORE_ENTRYPOINT_ENV: &str = "CODEX_TERMUX_CORE_ENTRYPOINT";
const CORE_API: &str = "codex-manager-core-v1";
const CALLER_SENTINEL_ENV: &str = "MGR_CALLER_SENTINEL";

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-manager-integration-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn manager_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_codex-manager"))
}

#[test]
fn manager_artifact_probe_is_exact_and_state_free() {
    let denied = Command::new(manager_binary())
        .env_clear()
        .arg("--artifact-probe")
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(2));
    assert!(!denied
        .stdout
        .windows(b"codex-manager-artifact-v1".len())
        .any(|window| window == b"codex-manager-artifact-v1"));

    let output = Command::new(manager_binary())
        .env_clear()
        .env("CODEX_MANAGER_ARTIFACT_PROBE", "1")
        .arg("--artifact-probe")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        b"codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n"
    );
    assert!(output.stderr.is_empty());
}

fn base_manager_command(home: &Path, core: &Path) -> Command {
    let mut command = Command::new(manager_binary());
    command
        .env_clear()
        .env("HOME", home)
        .env(CORE_API_ENV, CORE_API)
        .env(CORE_ENTRYPOINT_ENV, core)
        .env(CALLER_SENTINEL_ENV, "keep");
    command
}

fn run_manager(
    home: &Path,
    core: &Path,
    args: &[&str],
    inherited_codex_home: Option<&Path>,
) -> Output {
    let mut command = base_manager_command(home, core);
    command.args(args);
    if let Some(value) = inherited_codex_home {
        command.env("CODEX_HOME", value);
    }
    command.output().unwrap()
}

fn run_manager_with_input(
    home: &Path,
    core: &Path,
    args: &[&str],
    provider_dir: &Path,
    provider_log: &Path,
    input: &[u8],
) -> Output {
    let mut command = base_manager_command(home, core);
    command
        .args(args)
        .env("PATH", provider_dir)
        .env("PROVIDER_LOG", provider_log)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn write_core_probe(root: &Path) -> PathBuf {
    let shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            std::env::var_os("PATH").and_then(|path| {
                std::env::split_paths(&path)
                    .map(|directory| directory.join("sh"))
                    .find(|path| path.is_file())
            })
        })
        .expect("test shell must be available");
    let path = root.join("core-probe");
    let body = format!(
        "#!{}\n\
if [ \"$1\" = \"signal\" ]; then\n\
  printf '%s\\n' \"$$\" > \"$MGR_SIGNAL_PID\"\n\
  trap 'exit 143' TERM\n\
  while :; do :; done\n\
fi\n\
if [ \"$1\" = \"tty\" ]; then\n\
  [ -t 0 ] && [ -t 1 ] && [ -t 2 ] && exit 0\n\
  exit 88\n\
fi\n\
printf 'API_SET=%s\\n' \"${{CODEX_TERMUX_CORE_API+x}}\"\n\
printf 'ENTRY_SET=%s\\n' \"${{CODEX_TERMUX_CORE_ENTRYPOINT+x}}\"\n\
printf 'HOME_SET=%s\\n' \"${{CODEX_HOME+x}}\"\n\
printf 'HOME_VALUE=%s\\n' \"${{CODEX_HOME-}}\"\n\
printf 'SQLITE_HOME_SET=%s\\n' \"${{CODEX_SQLITE_HOME+x}}\"\n\
printf 'SQLITE_HOME_VALUE=%s\\n' \"${{CODEX_SQLITE_HOME-}}\"\n\
printf 'CALLER_SENTINEL=%s\\n' \"${{MGR_CALLER_SENTINEL-}}\"\n\
printf 'ARGC=%s\\n' \"$#\"\n\
for argument in \"$@\"; do printf 'ARG=<%s>\\n' \"$argument\"; done\n\
printf 'PROBE_STDERR\\n' >&2\n\
exit 37\n",
        shell.display()
    );
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn profile_lifecycle_and_isolated_exec_are_publicly_wired() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));
    assert_eq!(created.stdout, b"created: work\n");
    assert!(created.stderr.is_empty());

    let shared = root.0.join(".codex");
    let work_home = root.0.join(".local/share/codex/manager/profiles/work/home");
    assert!(
        !shared.exists(),
        "Manager must not prepare the conversation store"
    );
    assert_eq!(fs::read_dir(&work_home).unwrap().count(), 0);

    let dotted = run_manager(&root.0, &core, &["profile", "create", "account.a"], None);
    assert_eq!(dotted.status.code(), Some(0));
    assert!(!shared.exists());
    let listed = run_manager(&root.0, &core, &["profile", "list"], None);
    assert_eq!(listed.status.code(), Some(0));
    assert_eq!(listed.stdout, b"default\naccount.a\nwork\n");

    let current = run_manager(&root.0, &core, &["profile", "current"], None);
    assert_eq!(current.status.code(), Some(0));
    assert_eq!(current.stdout, b"current: default\nsource: default\n");

    let custom = run_manager(
        &root.0,
        &core,
        &["profile", "use", "work", "--", "--model", "gpt-5"],
        Some(Path::new("/caller/environment")),
    );
    assert_eq!(custom.status.code(), Some(37));
    assert!(custom
        .stdout
        .windows(b"API_SET=\n".len())
        .any(|window| window == b"API_SET=\n"));
    assert!(custom
        .stdout
        .windows(b"ENTRY_SET=\n".len())
        .any(|window| window == b"ENTRY_SET=\n"));
    assert!(custom
        .stdout
        .windows(b"HOME_SET=x\n".len())
        .any(|window| window == b"HOME_SET=x\n"));
    assert!(custom
        .stdout
        .windows(b"CALLER_SENTINEL=keep\n".len())
        .any(|window| window == b"CALLER_SENTINEL=keep\n"));
    let expected_home = root.0.join(".local/share/codex/manager/profiles/work/home");
    let expected_home_line = format!("HOME_VALUE={}\n", expected_home.display());
    assert!(custom
        .stdout
        .windows(expected_home_line.len())
        .any(|window| window == expected_home_line.as_bytes()));
    assert!(custom
        .stdout
        .windows(b"ARG=<--model>\n".len())
        .any(|window| window == b"ARG=<--model>\n"));
    assert!(custom
        .stdout
        .windows(b"ARG=<gpt-5>\n".len())
        .any(|window| window == b"ARG=<gpt-5>\n"));
    assert!(custom
        .stderr
        .windows(b"PROBE_STDERR\n".len())
        .any(|window| window == b"PROBE_STDERR\n"));

    let default = run_manager(
        &root.0,
        &core,
        &["profile", "use", "default", "--", "--version"],
        Some(&expected_home),
    );
    assert_eq!(default.status.code(), Some(37));
    assert!(default
        .stdout
        .windows(b"HOME_SET=\n".len())
        .any(|window| window == b"HOME_SET=\n"));
    assert!(default
        .stdout
        .windows(b"CALLER_SENTINEL=keep\n".len())
        .any(|window| window == b"CALLER_SENTINEL=keep\n"));
    assert!(default
        .stdout
        .windows(b"ARG=<--version>\n".len())
        .any(|window| window == b"ARG=<--version>\n"));
    assert!(!root.0.join(".local/share/codex/manager/state-v1").exists());

    let raw = OsString::from_vec(vec![0xff, 0x80, b'x']);
    let mut raw_command = base_manager_command(&root.0, &core);
    raw_command.args([
        OsString::from("profile"),
        OsString::from("use"),
        OsString::from("work"),
        OsString::from("--"),
        raw,
    ]);
    let raw_output = raw_command.output().unwrap();
    assert_eq!(raw_output.status.code(), Some(37));
    assert!(raw_output
        .stdout
        .windows(b"ARG=<\xff\x80x>\n".len())
        .any(|window| window == b"ARG=<\xff\x80x>\n"));

    assert!(!root.0.join(".local/share/codex/manager/state-v1").exists());
}

#[test]
fn upstream_resume_is_forwarded_through_the_selected_execution_profile() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));

    let resumed = base_manager_command(&root.0, &core)
        .args(["profile", "use", "work", "--", "resume", "--all"])
        .env("CODEX_HOME", "/caller/environment")
        .env("CODEX_SQLITE_HOME", "/caller/sqlite")
        .output()
        .unwrap();
    assert_eq!(resumed.status.code(), Some(37));
    for expected in [
        b"ARGC=2\n".as_slice(),
        b"ARG=<resume>\n".as_slice(),
        b"ARG=<--all>\n".as_slice(),
        b"HOME_SET=x\n".as_slice(),
        b"SQLITE_HOME_SET=\n".as_slice(),
    ] {
        assert!(
            resumed
                .stdout
                .windows(expected.len())
                .any(|window| window == expected),
            "missing {:?} in {:?}",
            String::from_utf8_lossy(expected),
            resumed.stdout
        );
    }
    let work_home = root.0.join(".local/share/codex/manager/profiles/work/home");
    let expected_home = format!("HOME_VALUE={}\n", work_home.display());
    assert!(resumed
        .stdout
        .windows(expected_home.len())
        .any(|window| window == expected_home.as_bytes()));
    let expected_sqlite = "SQLITE_HOME_VALUE=\n";
    assert!(resumed
        .stdout
        .windows(expected_sqlite.len())
        .any(|window| window == expected_sqlite.as_bytes()));
}

#[test]
fn manager_session_family_is_retired_and_non_mutating() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let result = run_manager(&root.0, &core, &["session", "list"], None);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(result
        .stderr
        .windows(b"codex termux: invalid command\n".len())
        .any(|window| window == b"codex termux: invalid command\n"));
    assert!(!root.0.join(".local/share/codex/manager").exists());
}

#[test]
fn invalid_handoff_and_reserved_route_are_non_mutating() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let invalid = Command::new(manager_binary())
        .env_clear()
        .env("HOME", &root.0)
        .env(CORE_API_ENV, "wrong-api")
        .env(CORE_ENTRYPOINT_ENV, &core)
        .args(["profile", "create", "work"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(1));
    assert!(!root.0.join(".local/share/codex/manager").exists());

    let reserved = run_manager(&root.0, &core, &["profile", "use", "work", "update"], None);
    assert_eq!(reserved.status.code(), Some(2));
    assert!(!root.0.join(".local/share/codex/manager").exists());
}

#[test]
fn retired_repair_routes_never_handoff_or_mutate_manager_state() {
    for existing_state in [false, true] {
        let root = TestRoot::new();
        let core = write_core_probe(&root.0);
        let mut protected = vec![core.clone()];
        if existing_state {
            let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
            assert_eq!(created.status.code(), Some(0));
            let saved = run_manager(
                &root.0,
                &core,
                &["notify", "set", "--hooks", "UserInputRequest,Stop"],
                None,
            );
            assert_eq!(saved.status.code(), Some(0));
            let manager = root.0.join(".local/share/codex/manager");
            protected.push(manager.join("notifications/config-v1"));
            protected.push(manager.join("profiles/work/profile.meta"));
            let history = manager.join("state-v1");
            fs::write(&history, b"preserve retired history").unwrap();
            protected.push(history);
        }
        let snapshots: Vec<_> = protected
            .iter()
            .map(|path| fs::read(path).unwrap())
            .collect();
        for args in [
            vec!["repair"],
            vec!["repair", "plan"],
            vec!["repair", "apply"],
            vec!["repair", "apply", "--rollback"],
        ] {
            let result = run_manager(&root.0, &core, &args, Some(Path::new("/caller/profile")));
            assert_eq!(result.status.code(), Some(2));
            assert!(result.stdout.is_empty());
            assert!(!result.stderr.is_empty());
            assert!(!result
                .stderr
                .windows(b"PROBE_STDERR".len())
                .any(|w| w == b"PROBE_STDERR"));
            assert_eq!(root.0.join(".local").exists(), existing_state);
            for (path, before) in protected.iter().zip(&snapshots) {
                assert_eq!(&fs::read(path).unwrap(), before);
            }
        }
        let help = run_manager(&root.0, &core, &["help"], None);
        assert_eq!(help.status.code(), Some(0));
        assert!(!String::from_utf8_lossy(&help.stdout).contains("repair"));
    }
}

#[test]
fn inherited_codex_home_is_reported_without_revealing_paths() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let external = root.0.join("external-home");
    fs::create_dir(&external).unwrap();
    fs::set_permissions(&external, fs::Permissions::from_mode(0o700)).unwrap();

    let current = run_manager(&root.0, &core, &["profile", "current"], Some(&external));
    assert_eq!(current.status.code(), Some(0));
    assert_eq!(current.stdout, b"current: external\nsource: inherited\n");
    assert!(!current
        .stdout
        .windows(root.0.to_string_lossy().len())
        .any(|window| { window == root.0.to_string_lossy().as_bytes() }));
}

#[test]
fn current_identity_ignores_retired_history_and_matches_fresh_launch() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    assert_eq!(
        run_manager(&root.0, &core, &["profile", "create", "account.a"], None)
            .status
            .code(),
        Some(0)
    );
    let state = root.0.join(".local/share/codex/manager/state-v1");
    fs::write(&state, b"codex-manager-state-v1\nlast_profile\taccount.a\n").unwrap();
    let state_before = fs::read(&state).unwrap();
    let selected = run_manager(
        &root.0,
        &core,
        &["profile", "use", "account.a", "--", "--version"],
        None,
    );
    assert_eq!(selected.status.code(), Some(37));
    let current = run_manager(&root.0, &core, &["profile", "current"], None);
    assert_eq!(current.status.code(), Some(0));
    assert_eq!(current.stdout, b"current: default\nsource: default\n");
    let profile = root
        .0
        .join(".local/share/codex/manager/profiles/account.a/home");
    let inherited = run_manager(&root.0, &core, &["profile", "current"], Some(&profile));
    assert_eq!(inherited.stdout, b"current: account.a\nsource: inherited\n");
    let default = run_manager(
        &root.0,
        &core,
        &["profile", "current"],
        Some(&root.0.join(".codex")),
    );
    assert_eq!(default.stdout, b"current: default\nsource: inherited\n");
    for selected in [OsString::new(), OsString::from_vec(vec![0xff, 0x80])] {
        let current = base_manager_command(&root.0, &core)
            .args(["profile", "current"])
            .env("CODEX_HOME", selected)
            .output()
            .unwrap();
        assert_eq!(current.status.code(), Some(0));
        assert_eq!(current.stdout, b"current: default\nsource: default\n");
    }
    fs::write(&state, b"invalid legacy history\n").unwrap();
    assert_eq!(
        run_manager(
            &root.0,
            &core,
            &["profile", "use", "default", "--", "--version"],
            None
        )
        .status
        .code(),
        Some(37)
    );
    assert_eq!(fs::read(&state).unwrap(), b"invalid legacy history\n");
    fs::write(&state, &state_before).unwrap();
    let outside = root.0.join("retired-history-target");
    fs::write(&outside, b"do-not-touch").unwrap();
    fs::remove_file(&state).unwrap();
    std::os::unix::fs::symlink(&outside, &state).unwrap();
    assert_eq!(
        run_manager(
            &root.0,
            &core,
            &["profile", "use", "account.a", "--", "--version"],
            None
        )
        .status
        .code(),
        Some(37)
    );
    assert_eq!(fs::read(&outside).unwrap(), b"do-not-touch");
    assert!(fs::symlink_metadata(&state)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn final_exec_preserves_pty_and_signal_delivery() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));

    let script = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|directory| directory.join("script"))
                .find(|path| path.is_file())
        })
        .expect("script must be available for the PTY proof");
    let invocation = format!("{} profile use work -- tty", shell_quote(manager_binary()));
    let tty = Command::new(script)
        .env_clear()
        .env("HOME", &root.0)
        .env(CORE_API_ENV, CORE_API)
        .env(CORE_ENTRYPOINT_ENV, &core)
        .env(CALLER_SENTINEL_ENV, "keep")
        .args([
            "--quiet",
            "--return",
            "--flush",
            "--command",
            &invocation,
            "/dev/null",
        ])
        .output()
        .unwrap();
    assert_eq!(tty.status.code(), Some(0));

    let signal_pid_path = root.0.join("signal.pid");
    let mut child = base_manager_command(&root.0, &core);
    child
        .env("MGR_SIGNAL_PID", &signal_pid_path)
        .args(["profile", "use", "work", "--", "signal"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut child = child.spawn().unwrap();
    let mut pid = None;
    for _ in 0..200 {
        match fs::read_to_string(&signal_pid_path) {
            Ok(value) => {
                pid = value.trim().parse::<i32>().ok();
                if pid.is_some() {
                    break;
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!("signal pid probe failed: {error}"),
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let pid = pid.expect("Core signal probe did not publish a pid");
    // SAFETY: the pid was published by the child that this test just spawned.
    assert_eq!(unsafe { kill(pid, 15) }, 0);
    assert_eq!(child.wait().unwrap().code(), Some(143));
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\"'\"'"))
}

fn write_notification_provider(root: &Path, name: &str) -> PathBuf {
    let shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            std::env::var_os("PATH").and_then(|path| {
                std::env::split_paths(&path)
                    .map(|directory| directory.join("sh"))
                    .find(|path| path.is_file())
            })
        })
        .expect("test shell must be available");
    let path = root.join(name);
    let body = format!(
        "#!{}\nprintf 'provider=%s\\n' \"$0\" >> \"$PROVIDER_LOG\"\nfor argument in \"$@\"; do printf 'arg=%s\\n' \"$argument\" >> \"$PROVIDER_LOG\"; done\nprintf 'provider stderr must be hidden\\n' >&2\nexit 23\n",
        shell.display()
    );
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn notification_test_reports_delivery_without_settings_or_hook_input() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    for name in ["termux-notification", "termux-toast"] {
        let p = write_notification_provider(&providers, name);
        fs::write(
            &p,
            fs::read_to_string(&p).unwrap().replace("exit 23", "exit 0"),
        )
        .unwrap();
    }
    let log = root.0.join("providers.log");
    let default =
        run_manager_with_input(&root.0, &core, &["notify", "test"], &providers, &log, b"");
    assert_eq!(default.status.code(), Some(0));
    assert_eq!(default.stdout, b"notification=ok\n");
    assert!(default.stderr.is_empty());
    assert!(!root.0.join(".local/share/codex/manager").exists());
    let set = run_manager(
        &root.0,
        &core,
        &["notify", "set", "--channel", "both", "--hooks", "none"],
        None,
    );
    assert_eq!(set.status.code(), Some(0));
    let config = root
        .0
        .join(".local/share/codex/manager/notifications/config-v1");
    let before = fs::read(&config).unwrap();
    let both = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "test"],
        &providers,
        &log,
        b"secret input must be ignored",
    );
    assert_eq!(both.status.code(), Some(0));
    assert_eq!(both.stdout, b"notification=ok\ntoast=ok\n");
    assert!(both.stderr.is_empty());
    assert_eq!(fs::read(&config).unwrap(), before);
    let calls = fs::read_to_string(&log).unwrap();
    assert!(calls.contains("arg=Codex notification test\n"));
    assert_eq!(calls.matches("arg=--action\n").count(), 2);
    assert_eq!(
        calls
            .matches("--activity-reorder-to-front --activity-single-top")
            .count(),
        2
    );
    assert!(!calls.contains("startservice") && !calls.contains("resume"));
    assert!(!calls.contains("secret input"));
    assert!(calls.contains("termux-toast\n"));
}

#[test]
fn notification_test_reports_missing_and_failed_providers_and_keeps_both_independent() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    let toast = write_notification_provider(&providers, "termux-toast");
    fs::write(
        &toast,
        fs::read_to_string(&toast)
            .unwrap()
            .replace("exit 23", "exit 0"),
    )
    .unwrap();
    let log = root.0.join("providers.log");
    let set = run_manager(
        &root.0,
        &core,
        &["notify", "set", "--channel", "both"],
        None,
    );
    assert_eq!(set.status.code(), Some(0));
    let missing =
        run_manager_with_input(&root.0, &core, &["notify", "test"], &providers, &log, b"");
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(missing.stdout, b"notification=unavailable\ntoast=ok\n");
    assert_eq!(missing.stderr, b"codex termux: notification test failed\n");
    let p = write_notification_provider(&providers, "termux-notification");
    for executable in [true, false] {
        if !executable {
            fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let failed =
            run_manager_with_input(&root.0, &core, &["notify", "test"], &providers, &log, b"");
        assert_eq!(failed.status.code(), Some(1));
        assert_eq!(failed.stdout, b"notification=failed\ntoast=ok\n");
        assert_eq!(failed.stderr, missing.stderr);
    }
    let invalid = run_manager(&root.0, &core, &["notify", "test", "unexpected"], None);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    let config = root
        .0
        .join(".local/share/codex/manager/notifications/config-v1");
    fs::write(&config, b"malformed\n").unwrap();
    let bad = run_manager_with_input(&root.0, &core, &["notify", "test"], &providers, &log, b"");
    assert_eq!(bad.status.code(), Some(1));
    assert!(bad.stdout.is_empty());
    assert_eq!(fs::read(config).unwrap(), b"malformed\n");
}

#[test]
fn notification_test_timeout_terminates_api_helper_descendants() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    let p = write_notification_provider(&providers, "termux-notification");
    let shell = fs::read_to_string(&p)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let sleep = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("sleep"))
        .find(|p| p.is_file())
        .unwrap();
    let child_pid = root.0.join("child.pid");
    fs::write(
        &p,
        format!(
            "{shell}\n{} 30 &\nprintf '%s\\n' \"$!\" > \"$PROVIDER_CHILD\"\nwait\n",
            sleep.display()
        ),
    )
    .unwrap();
    let mut command = base_manager_command(&root.0, &core);
    command
        .args(["notify", "test"])
        .env("PATH", &providers)
        .env("PROVIDER_CHILD", &child_pid);
    let start = std::time::Instant::now();
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"notification=timeout\n");
    assert_eq!(output.stderr, b"codex termux: notification test failed\n");
    assert!(start.elapsed() >= std::time::Duration::from_secs(5));
    assert!(start.elapsed() < std::time::Duration::from_secs(8));
    let pid = fs::read_to_string(child_pid).unwrap();
    let proc = PathBuf::from("/proc").join(pid.trim());
    if let Ok(stat) = fs::read_to_string(proc.join("stat")) {
        assert!(matches!(
            stat.rsplit_once(") ").unwrap().1.chars().next(),
            Some('Z' | 'X')
        ));
    }
}

#[test]
fn notification_configuration_and_emit_are_best_effort_at_manager_boundary() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let provider_dir = root.0.join("providers");
    fs::create_dir(&provider_dir).unwrap();
    fs::set_permissions(&provider_dir, fs::Permissions::from_mode(0o700)).unwrap();
    write_notification_provider(&provider_dir, "termux-notification");
    write_notification_provider(&provider_dir, "termux-toast");
    let provider_log = root.0.join("provider.log");

    let invalid = run_manager(
        &root.0,
        &core,
        &["notify", "set", "--hooks", "Stop,Stop"],
        None,
    );
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert!(!invalid.stderr.is_empty());
    assert!(!root.0.join(".local/share/codex/manager").exists());

    let set = run_manager(
        &root.0,
        &core,
        &[
            "notify",
            "set",
            "--channel",
            "both",
            "--hooks",
            "Stop",
            "--content-chars",
            "8",
            "--preserve-newlines",
            "0",
            "--toast-gravity",
            "bottom",
            "--toast-short",
            "1",
            "--toast-background",
            "#A0b1C2",
            "--toast-color",
            "#D3e4F5",
            "--group",
            "ops.v1",
        ],
        None,
    );
    assert_eq!(set.status.code(), Some(0));
    assert_eq!(set.stdout, b"saved\n");
    assert!(set.stderr.is_empty());

    let shown = run_manager(&root.0, &core, &["notify", "show"], None);
    assert_eq!(shown.status.code(), Some(0));
    assert_eq!(
        shown.stdout,
        b"channel=both\nhooks=Stop\ncontent-chars=8\npreserve-newlines=0\ntoast-gravity=bottom\ntoast-short=1\ntoast-background=#a0b1c2\ntoast-color=#d3e4f5\ngroup=ops.v1\nfocus=termux\n"
    );
    assert!(shown.stderr.is_empty());

    let emitted = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop"],
        &provider_dir,
        &provider_log,
        br#"{"title":"Turn title","content":"line one\r\nsecret body","message":"fallback","secret":"must-not-forward"}"#,
    );
    assert_eq!(emitted.status.code(), Some(0));
    assert!(emitted.stdout.is_empty());
    assert!(emitted.stderr.is_empty());
    let calls = fs::read_to_string(&provider_log).unwrap();
    assert!(calls.contains("arg=--title\narg=Turn title\n"));
    assert!(calls.contains("arg=--content\narg=line one\n"));
    assert!(calls.contains("arg=--group\narg=ops.v1\n"));
    assert!(calls.contains("arg=-g\narg=bottom\n"));
    assert!(calls.contains("arg=-s\n"));
    assert!(calls.contains("arg=-b\narg=#a0b1c2\n"));
    assert!(calls.contains("arg=-c\narg=#d3e4f5\n"));
    assert!(!calls.contains("must-not-forward"));
    assert!(!calls.contains("secret body"));

    let before_malformed = calls.len();
    let malformed = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop"],
        &provider_dir,
        &provider_log,
        b"not-json",
    );
    assert_eq!(malformed.status.code(), Some(0));
    assert!(malformed.stdout.is_empty());
    assert!(malformed.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&provider_log).unwrap().len(),
        before_malformed
    );

    let disabled = run_manager(&root.0, &core, &["notify", "set", "--hooks", "none"], None);
    assert_eq!(disabled.status.code(), Some(0));
    assert_eq!(disabled.stdout, b"saved\n");
    let disabled_emit = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop"],
        &provider_dir,
        &provider_log,
        br#"{"title":"disabled","content":"disabled body"}"#,
    );
    assert_eq!(disabled_emit.status.code(), Some(0));
    assert!(disabled_emit.stdout.is_empty());
    assert!(disabled_emit.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&provider_log).unwrap().len(),
        before_malformed
    );
}

#[test]
fn native_completion_argv_delivers_once_ignores_stdin_and_respects_disabled_events() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    write_notification_provider(&providers, "termux-notification");
    let log = root.0.join("provider.log");
    let configured = run_manager(&root.0, &core, &["notify", "set", "--focus", "tmux"], None);
    assert!(configured.status.success());
    let settings = run_manager(&root.0, &core, &["notify", "show"], None).stdout;
    let payload = r#"{"type":"agent-turn-complete","thread-id":"01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3","last-assistant-message":"\n completed\n normally\t","input-messages":["ignored-private-prompt"],"cwd":"/ignored-private-path"}"#;
    let output = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop", payload],
        &providers,
        &log,
        br#"{"content":"ignored-stdin"}"#,
    );
    assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
    let calls = fs::read_to_string(&log).unwrap();
    assert_eq!(calls.matches("provider=").count(), 1);
    assert!(calls.contains("arg=--content\narg=completed normally\n"));
    assert!(
        calls.contains("__tmux_focus") && calls.contains("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3")
    );
    assert!(!calls.contains("ignored-"));
    let oversized = format!(
        r#"{{"type":"agent-turn-complete","content":"{}"}}"#,
        "x".repeat(65536)
    );
    for payload in [
        "{",
        r#"{"content":"untyped"}"#,
        r#"{"type":"other"}"#,
        &oversized,
    ] {
        let output = run_manager_with_input(
            &root.0,
            &core,
            &["notify", "emit", "Stop", payload],
            &providers,
            &log,
            br#"{"content":"must-not-fallback"}"#,
        );
        assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
        assert_eq!(fs::read_to_string(&log).unwrap(), calls);
    }
    assert_eq!(
        run_manager(&root.0, &core, &["notify", "show"], None).stdout,
        settings
    );
    assert!(
        run_manager(&root.0, &core, &["notify", "set", "--hooks", "none"], None)
            .status
            .success()
    );
    let disabled = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop", payload],
        &providers,
        &log,
        b"",
    );
    assert!(disabled.status.success() && disabled.stdout.is_empty() && disabled.stderr.is_empty());
    assert_eq!(fs::read_to_string(&log).unwrap(), calls);
}

#[test]
fn notification_is_single_line_while_toast_preserves_configured_newlines() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    write_notification_provider(&providers, "termux-notification");
    write_notification_provider(&providers, "termux-toast");
    let log = root.0.join("provider.log");
    let set = run_manager(
        &root.0,
        &core,
        &[
            "notify",
            "set",
            "--channel",
            "both",
            "--hooks",
            "Stop",
            "--preserve-newlines",
            "1",
        ],
        None,
    );
    assert_eq!(set.status.code(), Some(0));
    let before = run_manager(&root.0, &core, &["notify", "show"], None).stdout;
    let emitted = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop"],
        &providers,
        &log,
        br#"{"title":"\n Codex\t done \r\n","content":"\r\n first\nsecond\tthird\u2028fourth \n"}"#,
    );
    assert_eq!(emitted.status.code(), Some(0));
    assert!(emitted.stdout.is_empty() && emitted.stderr.is_empty());
    let calls = fs::read_to_string(&log).unwrap();
    let (notification, toast) = calls
        .split_once(&format!(
            "provider={}\n",
            providers.join("termux-toast").display()
        ))
        .unwrap();
    assert!(notification.contains("arg=--title\narg=Codex done\n"));
    assert!(notification.contains("arg=--content\narg=first second third fourth\n"));
    assert!(toast.contains("arg=\n first\nsecond\tthird\u{2028}fourth \n\n"));
    assert_eq!(
        run_manager(&root.0, &core, &["notify", "show"], None).stdout,
        before
    );
}

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

#[test]
fn notification_click_reuses_activity_repeatedly_with_quoted_absolute_provider_and_private_output()
{
    let root = TestRoot::new();
    let bin = root.0.join("bin space ' $(touch injected)");
    fs::create_dir(&bin).unwrap();
    let core = write_core_probe(&bin);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    write_notification_provider(&providers, "termux-notification");
    let log = root.0.join("provider.log");
    let output = run_manager_with_input(
        &root.0,
        &core,
        &["notify", "emit", "Stop"],
        &providers,
        &log,
        br#"{"content":"own synthetic click test","session_id":"01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3","command":"touch injected","cwd":"/untrusted","CODEX_HOME":"/untrusted"}"#,
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let calls = fs::read_to_string(&log).unwrap();
    let action = calls
        .split("arg=--action\narg=")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap();
    assert!(action.contains("start --activity-reorder-to-front --activity-single-top -n com.termux/com.termux.app.TermuxActivity"));
    assert!(action.contains(" >/dev/null 2>&1"));
    assert!(!action.contains("/untrusted") && !action.contains("own synthetic click test"));
    assert!(
        !action.contains("01a0fc82")
            && !action.contains("resume")
            && !action.contains("startservice")
    );
    let am = bin.join("am");
    let shell = std::env::var_os("SHELL").unwrap();
    fs::write(&am, format!("#!{}\nprintf '%s\\n' \"$@\" >> \"$CLICK_LOG\"\nprintf private-output\nprintf private-error >&2\n", Path::new(&shell).display())).unwrap();
    fs::set_permissions(&am, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&log, "").unwrap();
    for _ in 0..3 {
        let clicked = Command::new(&shell)
            .args(["-c", action])
            .env("CLICK_LOG", &log)
            .current_dir(&root.0)
            .output()
            .unwrap();
        assert!(clicked.status.success());
        assert!(clicked.stdout.is_empty() && clicked.stderr.is_empty());
    }
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        "start\n--activity-reorder-to-front\n--activity-single-top\n-n\ncom.termux/com.termux.app.TermuxActivity\n".repeat(3)
    );
    assert!(!root.0.join("injected").exists());
}

#[test]
fn notification_tmux_click_executes_only_existing_focus_and_activity_with_safe_fallback() {
    let root = TestRoot::new();
    let home = root.0.join("home ' $(touch injected)");
    fs::create_dir(&home).unwrap();
    let bin = home.join("bin");
    fs::create_dir(&bin).unwrap();
    let core = write_core_probe(&bin);
    let providers = root.0.join("providers");
    fs::create_dir(&providers).unwrap();
    write_notification_provider(&providers, "termux-notification");
    let log = root.0.join("provider.log");
    let configured = run_manager(&home, &core, &["notify", "set", "--focus", "tmux"], None);
    assert!(configured.status.success());
    let shown = run_manager(&home, &core, &["notify", "show"], None);
    assert!(String::from_utf8(shown.stdout)
        .unwrap()
        .ends_with("focus=tmux\n"));
    let output = run_manager_with_input(&home, &core, &["notify", "emit", "Stop"], &providers, &log,
        br#"{"session_id":"01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3","content":"test","command":"touch injected"}"#);
    assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
    let calls = fs::read_to_string(&log).unwrap();
    let action = calls
        .split("arg=--action\narg=")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap();
    assert!(action.contains("__tmux_focus") && !action.contains("resume"));
    let shell = std::env::var_os("SHELL").unwrap();
    for (name, code) in [("ai", 1), ("am", 0)] {
        let path = bin.join(name);
        fs::write(
            &path,
            format!(
                "#!{}\nprintf '%s\n' '{}' \"$@\" >> \"$CLICK_LOG\"\nexit {}\n",
                Path::new(&shell).display(),
                name,
                code
            ),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let clicked_log = root.0.join("click.log");
    for _ in 0..3 {
        let clicked = Command::new(&shell)
            .args(["-c", action])
            .env("CLICK_LOG", &clicked_log)
            .current_dir(&root.0)
            .output()
            .unwrap();
        assert!(clicked.status.success() && clicked.stdout.is_empty() && clicked.stderr.is_empty());
    }
    assert_eq!(fs::read_to_string(&clicked_log).unwrap(), "ai\n__tmux_focus\n01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3\nam\nstart\n--activity-reorder-to-front\n--activity-single-top\n-n\ncom.termux/com.termux.app.TermuxActivity\n".repeat(3));
    assert!(!root.0.join("injected").exists());
}

#[test]
fn active_task_status_and_usage_are_publicly_wired_and_state_free() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let result = run_manager(&root.0, &core, &["task", "status"], None);
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"No current task writers found.\n");
    assert!(result.stderr.is_empty());
    for args in [
        vec!["task", "stop"],
        vec!["task", "status", "bad"],
        vec![
            "task",
            "takeover",
            "12345678-1234-1234-1234-123456789abc",
            "--force-server",
            "1:1",
        ],
    ] {
        assert_eq!(
            run_manager(&root.0, &core, &args, None).status.code(),
            Some(2)
        );
    }
    assert!(!root.0.join(".local").exists());
    let help = run_manager(&root.0, &core, &["help"], None);
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("codex termux task"));
}

const TASK_ID: &str = "12345678-1234-1234-1234-123456789abc";
const CHILD_TASK_ID: &str = "12345678-1234-1234-1234-123456789abe";
const OTHER_TASK_ID: &str = "12345678-1234-1234-1234-123456789abd";

// Owned executable server fixture: no production injection or live account state.
#[test]
fn active_task_native_fixture() {
    use serde_json::{json, Value};
    use std::os::unix::net::UnixListener;
    use tungstenite::Message;
    let Some(path) = std::env::var_os("TASK_FIXTURE_SOCKET") else {
        return;
    };
    let listener = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    if std::env::var_os("TASK_FIXTURE_IGNORE_TERM").is_some() {
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_IGN);
        }
    }
    let mut active = std::collections::BTreeSet::from([
        TASK_ID.to_owned(),
        OTHER_TASK_ID.to_owned(),
        CHILD_TASK_ID.to_owned(),
    ]);
    let mut goal_active = true;
    let mut loaded_calls = 0;
    let mut writer = std::env::var_os("TASK_FIXTURE_WRITER").map(|p| {
        let f = std::fs::File::open(p).unwrap();
        f.try_lock().unwrap();
        f
    });
    let mut other_writers = std::collections::BTreeMap::new();
    if let Some(directory) = std::env::var_os("TASK_FIXTURE_LOCKS") {
        for id in [OTHER_TASK_ID, CHILD_TASK_ID] {
            if id == CHILD_TASK_ID && std::env::var_os("TASK_FIXTURE_CHILD_VIEW_ONLY").is_some() {
                continue;
            }
            let file =
                std::fs::File::open(PathBuf::from(&directory).join(format!("{id}.lock"))).unwrap();
            file.try_lock().unwrap();
            other_writers.insert(id, file);
        }
    }
    if std::env::var_os("TASK_FIXTURE_UNRESPONSIVE").is_some() {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    for stream in listener.incoming() {
        let Ok(mut ws) = tungstenite::accept(stream.unwrap()) else {
            continue;
        };
        while let Ok(message) = ws.read() {
            let Ok(text) = message.to_text() else {
                continue;
            };
            let value: Value = serde_json::from_str(text).unwrap();
            let method = value["method"].as_str().unwrap();
            if method == "initialized" {
                continue;
            }
            let result = match method {
                "initialize" => json!({}),
                "thread/loaded/list" => {
                    loaded_calls += 1;
                    if let Some(path) = std::env::var_os("TASK_FIXTURE_WRITER") {
                        let marker = PathBuf::from(path)
                            .parent()
                            .unwrap()
                            .join(".release-during-scope");
                        if fs::read_to_string(marker)
                            .ok()
                            .and_then(|s| s.trim().parse::<usize>().ok())
                            == Some(loaded_calls)
                        {
                            writer.take();
                        }
                    }
                    json!({"data":[TASK_ID,OTHER_TASK_ID,CHILD_TASK_ID],"nextCursor":null})
                }
                "thread/read" => {
                    json!({"thread":{"id":value["params"]["threadId"],"status":{"type":if active.contains(value["params"]["threadId"].as_str().unwrap()) {"active"} else {"idle"}},"preview":"NEVER_PRINT_PRIVATE"}})
                }
                "thread/list" => {
                    assert_eq!(value["params"]["ancestorThreadId"], TASK_ID);
                    json!({"data":[{"id":CHILD_TASK_ID}],"nextCursor":null})
                }
                "thread/goal/get" => {
                    if let Some(path) = std::env::var_os("TASK_FIXTURE_WRITER") {
                        if PathBuf::from(path)
                            .parent()
                            .unwrap()
                            .join(".release-before-goal-change")
                            .exists()
                        {
                            writer.take();
                        }
                    }
                    json!({"goal":{"status":if goal_active {"active"} else {"paused"},"objective":"NEVER_PRINT_PRIVATE"}})
                }
                "thread/goal/set" => {
                    assert_eq!(value["params"]["status"], "paused");
                    goal_active = false;
                    json!({})
                }
                "thread/turns/list" => {
                    assert_eq!(value["params"]["itemsView"], "notLoaded");
                    json!({"data":[{"id":"turn-owned","status":"inProgress"}]})
                }
                "turn/interrupt" => {
                    assert!(!goal_active);
                    let id = value["params"]["threadId"].as_str().unwrap();
                    assert!(id == TASK_ID || id == CHILD_TASK_ID);
                    assert!(
                        id != CHILD_TASK_ID
                            || std::env::var_os("TASK_FIXTURE_CHILD_VIEW_ONLY").is_none()
                    );
                    active.remove(id);
                    if std::env::var_os("TASK_FIXTURE_RETAIN").is_none() {
                        if id == TASK_ID {
                            writer.take();
                        } else {
                            other_writers.remove(id);
                        }
                    }
                    json!({})
                }
                "thread/backgroundTerminals/clean" => json!({}),
                _ => panic!("unexpected protocol method"),
            };
            if ws
                .send(Message::Text(
                    json!({"id":value["id"],"result":result}).to_string().into(),
                ))
                .is_err()
            {
                break;
            }
        }
    }
}

struct TaskFixture {
    root: TestRoot,
    core: PathBuf,
    child: std::process::Child,
    account: PathBuf,
    binding: PathBuf,
}
impl TaskFixture {
    fn new() -> Self {
        Self::configured(false)
    }
    fn configured(retain: bool) -> Self {
        Self::configured_mode(retain, false, false, false)
    }
    fn configured_mode(
        retain: bool,
        ignore_term: bool,
        unresponsive: bool,
        child_view_only: bool,
    ) -> Self {
        use std::os::unix::ffi::OsStrExt;
        let root = TestRoot::new();
        let core = write_core_probe(&root.0);
        let account = root.0.join("external-a");
        fs::create_dir(&account).unwrap();
        fs::set_permissions(&account, fs::Permissions::from_mode(0o700)).unwrap();
        let runtime = root
            .0
            .join(".local/lib/codex/core/generations/fixture-old/runtime");
        fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        fs::copy(std::env::current_exe().unwrap(), &runtime).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).unwrap();
        let mut hash = 0xcbf29ce484222325u64;
        for b in account
            .as_os_str()
            .as_bytes()
            .iter()
            .chain([0].iter())
            .chain(runtime.as_os_str().as_bytes())
        {
            hash = (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
        let binding = root
            .0
            .join(format!(".local/share/codex/core/servers/{hash:016x}"));
        fs::create_dir_all(&binding).unwrap();
        for p in [binding.parent().unwrap(), binding.as_path()] {
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let socket = root.0.join("s");
        let locks = root.0.join(".codex/thread-writer-locks");
        fs::create_dir_all(&locks).unwrap();
        for p in [locks.parent().unwrap(), locks.as_path()] {
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let writer = locks.join(format!("{TASK_ID}.lock"));
        fs::write(&writer, b"native-writer").unwrap();
        fs::set_permissions(&writer, fs::Permissions::from_mode(0o600)).unwrap();
        for id in [OTHER_TASK_ID, CHILD_TASK_ID] {
            let path = locks.join(format!("{id}.lock"));
            fs::write(&path, b"native-writer").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        fs::write(locks.join(".coordination.lock"), b"").unwrap();
        let mut process = Command::new(&runtime);
        process
            .args(["--exact", "active_task_native_fixture", "--nocapture"])
            .env("TASK_FIXTURE_SOCKET", &socket)
            .env("TASK_FIXTURE_WRITER", &writer)
            .env("TASK_FIXTURE_LOCKS", &locks)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if retain {
            process.env("TASK_FIXTURE_RETAIN", "1");
        }
        if ignore_term {
            process.env("TASK_FIXTURE_IGNORE_TERM", "1");
        }
        if unresponsive {
            process.env("TASK_FIXTURE_UNRESPONSIVE", "1");
        }
        if child_view_only {
            process.env("TASK_FIXTURE_CHILD_VIEW_ONLY", "1");
        }
        let child = process.spawn().unwrap();
        let mut owner = account.as_os_str().as_bytes().to_vec();
        owner.push(0);
        owner.extend_from_slice(runtime.as_os_str().as_bytes());
        for (name, bytes) in [
            ("owner", owner),
            ("pid", format!("{}\n", child.id()).into_bytes()),
        ] {
            let p = binding.join(name);
            fs::write(&p, bytes).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
        }
        std::os::unix::fs::symlink(&socket, binding.join("s")).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !socket.exists() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Self {
            root,
            core,
            child,
            account,
            binding,
        }
    }
}
impl Drop for TaskFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn active_task_owner_follows_current_kernel_writer_not_original_loaded_server() {
    use std::os::unix::ffi::OsStrExt;
    struct OwnedProcess(std::process::Child);
    impl Drop for OwnedProcess {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = TaskFixture::new();
    let stopped = run_manager(&f.root.0, &f.core, &["task", "stop", TASK_ID], None);
    assert_eq!(stopped.status.code(), Some(0));
    // A remains alive and loaded, but has released its writer: it is no owner.
    assert!(f.child.id() > 1);
    let released = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    assert_eq!(released.stdout, b"No current task writers found.\n");
    let account = f.root.0.join("external-b");
    fs::create_dir(&account).unwrap();
    fs::set_permissions(&account, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = f
        .root
        .0
        .join(".local/lib/codex/core/generations/fixture-old/runtime");
    let mut owner = account.as_os_str().as_bytes().to_vec();
    owner.push(0);
    owner.extend_from_slice(runtime.as_os_str().as_bytes());
    let mut hash = 0xcbf29ce484222325u64;
    for b in &owner {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3);
    }
    let binding = f.binding.parent().unwrap().join(format!("{hash:016x}"));
    fs::create_dir(&binding).unwrap();
    fs::set_permissions(&binding, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = f.root.0.join("s2");
    let mut child = OwnedProcess(
        Command::new(&runtime)
            .args(["--exact", "active_task_native_fixture", "--nocapture"])
            .env("TASK_FIXTURE_SOCKET", &socket)
            .env(
                "TASK_FIXTURE_WRITER",
                f.root
                    .0
                    .join(format!(".codex/thread-writer-locks/{TASK_ID}.lock")),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    for (name, bytes) in [
        ("owner", owner),
        ("pid", format!("{}\n", child.0.id()).into_bytes()),
    ] {
        let path = binding.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    std::os::unix::fs::symlink(&socket, binding.join("s")).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !socket.exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let status = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    assert_eq!(
        status.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let text = String::from_utf8(status.stdout).unwrap();
    assert!(text.contains("external-b") && !text.contains("external-a"));
    let reconnect = run_manager(&f.root.0, &f.core, &["task", "reconnect", TASK_ID], None);
    assert_eq!(reconnect.status.code(), Some(37));
    assert!(String::from_utf8(reconnect.stdout)
        .unwrap()
        .contains(&format!("HOME_VALUE={}", account.display())));
    assert!(child.0.try_wait().unwrap().is_none());
}

#[test]
fn active_task_stop_preserves_loaded_descendant_without_writer_ownership() {
    use serde_json::{json, Value};
    let f = TaskFixture::configured_mode(false, false, false, true);
    let stopped = run_manager(&f.root.0, &f.core, &["task", "stop", TASK_ID], None);
    assert_eq!(
        stopped.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    let stream = std::os::unix::net::UnixStream::connect(f.root.0.join("s")).unwrap();
    let (mut ws, _) = tungstenite::client("ws://localhost/", stream).unwrap();
    ws.send(tungstenite::Message::Text(
        json!({"id":1,"method":"thread/read","params":{"threadId":CHILD_TASK_ID}})
            .to_string()
            .into(),
    ))
    .unwrap();
    let result: Value = serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(result["result"]["thread"]["status"]["type"], "active");
    drop(ws);
    let status = run_manager(&f.root.0, &f.core, &["task", "status", CHILD_TASK_ID], None);
    assert_eq!(status.stdout, b"No current task writers found.\n");
}

#[test]
fn active_task_released_owner_resumes_current_account_without_contacting_original_author() {
    let f = TaskFixture::new();
    assert_eq!(
        run_manager(&f.root.0, &f.core, &["task", "stop", TASK_ID], None)
            .status
            .code(),
        Some(0)
    );
    let current = f.root.0.join("external-b");
    fs::create_dir(&current).unwrap();
    fs::set_permissions(&current, fs::Permissions::from_mode(0o700)).unwrap();
    let resumed = run_manager(
        &f.root.0,
        &f.core,
        &["task", "takeover", TASK_ID],
        Some(&current),
    );
    assert_eq!(resumed.status.code(), Some(37));
    let text = String::from_utf8(resumed.stdout).unwrap();
    assert!(text.contains(&format!("HOME_VALUE={}", current.display())));
    assert!(!text.contains("ARG=<--remote>"));
    let denied = run_manager(
        &f.root.0,
        &f.core,
        &["task", "takeover", TASK_ID, "--force-server", "10:123"],
        Some(&current),
    );
    assert_eq!(denied.status.code(), Some(1));
    // A holder absent from the trusted server records remains unknown, never free.
    let lock = std::fs::File::open(
        f.root
            .0
            .join(format!(".codex/thread-writer-locks/{TASK_ID}.lock")),
    )
    .unwrap();
    lock.try_lock().unwrap();
    let unknown = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    assert_eq!(unknown.status.code(), Some(1));
    let blocked = run_manager(
        &f.root.0,
        &f.core,
        &["task", "takeover", TASK_ID],
        Some(&current),
    );
    assert_eq!(blocked.status.code(), Some(1));
    assert!(blocked.stdout.is_empty());
}

#[test]
fn active_task_changed_owner_during_metadata_never_authorizes_old_goal_change_or_signal() {
    for force in [false, true] {
        let mut f = TaskFixture::configured(true);
        let status = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
        let text = String::from_utf8(status.stdout).unwrap();
        let token = text.split("server=").nth(1).unwrap().trim();
        let locks = f.root.0.join(".codex/thread-writer-locks");
        let output = if force {
            fs::write(locks.join(".release-during-scope"), b"3").unwrap();
            run_manager(
                &f.root.0,
                &f.core,
                &["task", "takeover", TASK_ID, "--force-server", token],
                None,
            )
        } else {
            fs::write(locks.join(".release-before-goal-change"), b"").unwrap();
            run_manager(&f.root.0, &f.core, &["task", "stop", TASK_ID], None)
        };
        assert_eq!(output.status.code(), Some(1));
        assert!(f.child.try_wait().unwrap().is_none());
        assert!(output.stdout.is_empty());
        let status = run_manager(&f.root.0, &f.core, &["task", "status", OTHER_TASK_ID], None);
        assert_eq!(status.status.code(), Some(0));
        assert!(String::from_utf8(status.stdout)
            .unwrap()
            .contains("state=active"));
    }
}

#[test]
fn active_task_discovery_identifies_external_old_runtime_and_rejects_binding_faults() {
    let f = TaskFixture::new();
    let output = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(TASK_ID));
    assert!(text.contains("external-a"));
    assert!(text.contains("state=active"));
    assert!(!text.contains(OTHER_TASK_ID));
    assert!(!text.contains("NEVER_PRINT_PRIVATE"));
    assert!(f.account.is_dir());
    let pid = f.binding.join("pid");
    let before = fs::read(&pid).unwrap();
    fs::write(&pid, b"1\n").unwrap();
    assert_eq!(
        run_manager(&f.root.0, &f.core, &["task", "status"], None)
            .status
            .code(),
        Some(1)
    );
    fs::write(&pid, &before).unwrap();
    let owner = f.binding.join("owner");
    let before = fs::read(&owner).unwrap();
    fs::write(&owner, b"substituted\0runtime").unwrap();
    assert_eq!(
        run_manager(&f.root.0, &f.core, &["task", "status"], None)
            .status
            .code(),
        Some(1)
    );
    fs::write(&owner, before).unwrap();
    fs::set_permissions(&pid, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        run_manager(&f.root.0, &f.core, &["task", "status"], None)
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn active_task_reconnect_execs_owner_and_stop_waits_but_reports_retained_writer() {
    for retain in [false, true] {
        let f = TaskFixture::configured(retain);
        let reconnect = run_manager(&f.root.0, &f.core, &["task", "reconnect", TASK_ID], None);
        assert_eq!(reconnect.status.code(), Some(37));
        let text = String::from_utf8(reconnect.stdout).unwrap();
        assert!(text.contains(&format!("HOME_VALUE={}\n", f.account.display())));
        assert!(text.contains("ARG=<--remote>"));
        assert!(text.contains(&format!("ARG=<{}>", TASK_ID)));
        assert!(text.contains("API_SET=\n"));
        let stopped = run_manager(&f.root.0, &f.core, &["task", "stop", TASK_ID], None);
        assert_eq!(
            stopped.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&stopped.stderr)
        );
        assert_eq!(
            String::from_utf8(stopped.stdout)
                .unwrap()
                .contains("still owned"),
            retain
        );
        let status = run_manager(&f.root.0, &f.core, &["task", "status"], None);
        let text = String::from_utf8(status.stdout).unwrap();
        assert_eq!(
            text.lines()
                .any(|l| l.contains(TASK_ID) && l.contains("state=idle")),
            retain
        );
        assert!(text
            .lines()
            .any(|l| l.contains(OTHER_TASK_ID) && l.contains("state=active")));
        assert_eq!(
            text.lines()
                .any(|l| l.contains(CHILD_TASK_ID) && l.contains("state=idle")),
            retain
        );
        assert_eq!(
            fs::read(
                f.root
                    .0
                    .join(format!(".codex/thread-writer-locks/{TASK_ID}.lock"))
            )
            .unwrap(),
            b"native-writer"
        );
    }
}

#[test]
fn active_task_takeover_uses_chosen_account_and_never_starts_a_second_writer() {
    for retain in [false, true] {
        let f = TaskFixture::configured(retain);
        assert_eq!(
            run_manager(
                &f.root.0,
                &f.core,
                &["profile", "create", "account-b"],
                None
            )
            .status
            .code(),
            Some(0)
        );
        let target = f
            .root
            .0
            .join(".local/share/codex/manager/profiles/account-b/home");
        let output = run_manager(
            &f.root.0,
            &f.core,
            &["task", "takeover", TASK_ID, "--profile", "account-b"],
            None,
        );
        if retain {
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8(output.stderr)
                .unwrap()
                .contains("writer is still owned"));
            assert!(output.stdout.is_empty());
        } else {
            assert_eq!(output.status.code(), Some(37));
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains(&format!("HOME_VALUE={}\n", target.display())));
            assert!(!text.contains("ARG=<--remote>"));
            assert!(text.contains(&format!("ARG=<{}>", TASK_ID)));
        }
    }
}
#[test]
fn active_task_force_is_pid_stable_server_scoped_and_rejects_stale_confirmation() {
    let mut f = TaskFixture::configured(true);
    let mut unrelated = TaskFixture::new();
    let status = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    let status = String::from_utf8(status.stdout).unwrap();
    let token = status.split("server=").nth(1).unwrap().trim();
    let stale = format!("{}:0", f.child.id());
    let denied = run_manager(
        &f.root.0,
        &f.core,
        &["task", "takeover", TASK_ID, "--force-server", &stale],
        None,
    );
    assert_eq!(denied.status.code(), Some(1));
    assert!(f.child.try_wait().unwrap().is_none());
    let takeover = run_manager(
        &f.root.0,
        &f.core,
        &["task", "takeover", TASK_ID, "--force-server", token],
        None,
    );
    assert_eq!(
        takeover.status.code(),
        Some(37),
        "{}",
        String::from_utf8_lossy(&takeover.stderr)
    );
    let scope = String::from_utf8(takeover.stderr).unwrap();
    assert!(scope.contains("Whole-server"));
    assert!(scope.contains(TASK_ID));
    assert!(scope.contains(OTHER_TASK_ID));
    assert!(f.child.wait().unwrap().code() != Some(0));
    assert!(unrelated.child.try_wait().unwrap().is_none());
    assert_eq!(
        fs::read(
            f.root
                .0
                .join(format!(".codex/thread-writer-locks/{TASK_ID}.lock"))
        )
        .unwrap(),
        b"native-writer"
    );
}
#[test]
fn active_task_destination_failure_and_noninteractive_menu_never_cancel_owner() {
    let f = TaskFixture::new();
    assert_eq!(
        run_manager(&f.root.0, &f.core, &["task"], None)
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        run_manager(
            &f.root.0,
            &f.core,
            &["task", "takeover", TASK_ID, "--profile", "absent"],
            None
        )
        .status
        .code(),
        Some(1)
    );
    let output = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("state=active"));
}

#[test]
fn active_task_unresponsive_owner_and_term_ignoring_force_preserve_unrelated_work() {
    let mut f = TaskFixture::configured_mode(true, true, true, false);
    let status = run_manager(&f.root.0, &f.core, &["task", "status", TASK_ID], None);
    assert_eq!(
        status.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let text = String::from_utf8(status.stdout).unwrap();
    assert!(text.contains("unresponsive"));
    let token = text.split("server=").nth(1).unwrap().trim();
    let output = run_manager(
        &f.root.0,
        &f.core,
        &["task", "takeover", TASK_ID, "--force-server", token],
        None,
    );
    assert_eq!(
        output.status.code(),
        Some(37),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("unknown"));
    let exit = f.child.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(exit.signal(), Some(libc::SIGKILL));
    // Core's old rendezvous may remain until native retirement; it cannot poison discovery.
    let status = run_manager(&f.root.0, &f.core, &["task", "status"], None);
    assert_eq!(status.status.code(), Some(0));
    assert_eq!(status.stdout, b"No current task writers found.\n");
}
#[test]
fn active_task_interactive_menu_allows_selection_cancel_and_current_account_takeover() {
    let script = std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join("script"))
                .find(|p| p.is_file())
        })
        .unwrap();
    for (input, expected) in [
        ("\n", 130),
        ("1\n\n", 130),
        ("1\n4\n\n", 130),
        ("1\n3\n", 37),
    ] {
        let f = TaskFixture::new();
        let current = f.root.0.join("external-b");
        fs::create_dir(&current).unwrap();
        fs::set_permissions(&current, fs::Permissions::from_mode(0o700)).unwrap();
        let invocation = format!("{} task", shell_quote(manager_binary()));
        let mut process = Command::new(&script)
            .env_clear()
            .env("HOME", &f.root.0)
            .env(CORE_API_ENV, CORE_API)
            .env(CORE_ENTRYPOINT_ENV, &f.core)
            .env("CODEX_HOME", &current)
            .args([
                "--quiet",
                "--return",
                "--flush",
                "--command",
                &invocation,
                "/dev/null",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = process.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Reconnect to owner") || text.contains("Task number"));
        if expected == 37 {
            assert!(text.contains(&format!("HOME_VALUE={}", current.display())));
        }
    }
}
