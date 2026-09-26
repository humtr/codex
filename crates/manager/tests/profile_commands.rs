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
const CORE_REPAIR_REQUEST: &str = "codex-manager-repair-v1";
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
        .stdin(Stdio::piped());
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
printf 'REQUEST=%s\\n' \"${{CODEX_TERMUX_CORE_REQUEST-}}\"\n\
printf 'OPERATION=%s\\n' \"${{CODEX_TERMUX_CORE_OPERATION-}}\"\n\
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
    for name in [
        "sessions",
        "archived_sessions",
        "thread-writer-locks",
        "rollout-migrations",
        "memories",
        "memories_v2",
        "tui-thread-reference-capabilities",
        "session_index.jsonl",
        "history.jsonl",
        "installation_id",
    ] {
        assert_eq!(
            fs::read_link(work_home.join(name)).unwrap(),
            shared.join(name)
        );
    }

    let listed = run_manager(&root.0, &core, &["profile", "list"], None);
    assert_eq!(listed.status.code(), Some(0));
    assert_eq!(listed.stdout, b"default\nwork\n");

    let current = run_manager(&root.0, &core, &["profile", "current"], None);
    assert_eq!(current.status.code(), Some(0));
    assert_eq!(
        current.stdout,
        b"current: default\nsource: last-selection\n"
    );

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
    let selected = fs::read(root.0.join(".local/share/codex/manager/state-v1")).unwrap();
    assert_eq!(selected, b"codex-manager-state-v1\nlast_profile\tdefault\n");

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

    let selected = fs::read(root.0.join(".local/share/codex/manager/state-v1")).unwrap();
    assert_eq!(selected, b"codex-manager-state-v1\nlast_profile\twork\n");
}

#[test]
fn upstream_resume_is_forwarded_through_the_selected_execution_profile() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));

    let resumed = run_manager(
        &root.0,
        &core,
        &["profile", "use", "work", "--", "resume", "--all"],
        Some(Path::new("/caller/environment")),
    );
    assert_eq!(resumed.status.code(), Some(37));
    for expected in [
        b"ARGC=2\n".as_slice(),
        b"ARG=<resume>\n".as_slice(),
        b"ARG=<--all>\n".as_slice(),
        b"HOME_SET=x\n".as_slice(),
        b"SQLITE_HOME_SET=x\n".as_slice(),
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
    let expected_sqlite = format!("SQLITE_HOME_VALUE={}\n", root.0.join(".codex").display());
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
fn repair_requests_use_only_the_fixed_core_boundary() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let plan = run_manager(
        &root.0,
        &core,
        &["repair", "plan"],
        Some(Path::new("/caller/profile")),
    );
    assert_eq!(plan.status.code(), Some(37));
    assert!(plan
        .stderr
        .windows(b"PROBE_STDERR\n".len())
        .any(|window| { window == b"PROBE_STDERR\n" }));
    for expected in [
        format!("REQUEST={CORE_REPAIR_REQUEST}\n"),
        "OPERATION=plan\n".to_owned(),
        "API_SET=\n".to_owned(),
        "ENTRY_SET=\n".to_owned(),
        "HOME_SET=\n".to_owned(),
        "ARG=<doctor>\n".to_owned(),
        "ARG=<--json>\n".to_owned(),
        "CALLER_SENTINEL=keep\n".to_owned(),
    ] {
        assert!(
            plan.stdout
                .windows(expected.len())
                .any(|window| window == expected.as_bytes()),
            "missing {expected:?} in {:?}",
            plan.stdout
        );
    }

    let apply = run_manager(
        &root.0,
        &core,
        &["repair", "apply"],
        Some(Path::new("/caller/profile")),
    );
    assert_eq!(apply.status.code(), Some(37));
    for expected in [
        format!("REQUEST={CORE_REPAIR_REQUEST}\n"),
        "OPERATION=apply\n".to_owned(),
        "API_SET=\n".to_owned(),
        "ENTRY_SET=\n".to_owned(),
        "HOME_SET=\n".to_owned(),
        "ARG=<update>\n".to_owned(),
    ] {
        assert!(
            apply
                .stdout
                .windows(expected.len())
                .any(|window| window == expected.as_bytes()),
            "missing {expected:?} in {:?}",
            apply.stdout
        );
    }

    let invalid = run_manager(&root.0, &core, &["repair", "apply", "--rollback"], None);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert!(!invalid.stderr.is_empty());
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
        b"channel=both\nhooks=Stop\ncontent-chars=8\npreserve-newlines=0\ntoast-gravity=bottom\ntoast-short=1\ntoast-background=#a0b1c2\ntoast-color=#d3e4f5\ngroup=ops.v1\n"
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

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}
