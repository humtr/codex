use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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

fn write_session(home: &Path, relative_path: &str, body: &[u8], mtime: u64) {
    let path = home.join("sessions").join(relative_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    let times = fs::FileTimes::new()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime));
    file.set_times(times).unwrap();
}

fn session_rows(output: &[u8]) -> Vec<Vec<String>> {
    std::str::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect()
}

#[test]
fn profile_lifecycle_and_isolated_exec_are_publicly_wired() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));
    assert_eq!(created.stdout, b"created: work\n");
    assert!(created.stderr.is_empty());

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
fn session_list_is_empty_or_exact_tsv_and_respects_profile_selection() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);

    let empty = run_manager(&root.0, &core, &["session", "list"], None);
    assert_eq!(empty.status.code(), Some(0));
    assert!(empty.stdout.is_empty());
    assert!(!root.0.join(".local/share/codex/manager").exists());

    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));
    let work_home = root.0.join(".local/share/codex/manager/profiles/work/home");
    let default_home = root.0.join(".codex");
    write_session(
        &default_home,
        "2026/09/default-ref.jsonl",
        b"default-body-secret-sentinel",
        100,
    );
    write_session(&work_home, "nested/work-ref.jsonl", b"work-body", 200);

    let default = run_manager(&root.0, &core, &["session", "list"], None);
    assert_eq!(default.status.code(), Some(0));
    assert_eq!(
        session_rows(&default.stdout),
        vec![vec![
            "default".to_owned(),
            "default-ref".to_owned(),
            "100".to_owned()
        ]]
    );
    assert!(!default
        .stdout
        .windows(root.0.to_string_lossy().len())
        .any(|window| { window == root.0.to_string_lossy().as_bytes() }));
    assert!(!default
        .stdout
        .windows(b"default-body-secret-sentinel".len())
        .any(|window| { window == b"default-body-secret-sentinel" }));

    let selected = run_manager(
        &root.0,
        &core,
        &["session", "list", "--profile", "work"],
        None,
    );
    assert_eq!(selected.status.code(), Some(0));
    assert_eq!(
        session_rows(&selected.stdout),
        vec![vec![
            "work".to_owned(),
            "work-ref".to_owned(),
            "200".to_owned()
        ]]
    );
    assert!(!root.0.join(".local/share/codex/manager/state-v1").exists());
}

#[test]
fn session_list_all_has_bytewise_profile_tie_order_without_an_index() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    for profile in ["work", "Alpha"] {
        let created = run_manager(&root.0, &core, &["profile", "create", profile], None);
        assert_eq!(created.status.code(), Some(0));
    }
    write_session(&root.0.join(".codex"), "default-ref.jsonl", b"default", 300);
    write_session(
        &root.0.join(".local/share/codex/manager/profiles/work/home"),
        "work-ref.jsonl",
        b"work",
        300,
    );
    write_session(
        &root
            .0
            .join(".local/share/codex/manager/profiles/Alpha/home"),
        "alpha-ref.jsonl",
        b"alpha",
        300,
    );

    let all = run_manager(&root.0, &core, &["session", "list", "--all"], None);
    assert_eq!(all.status.code(), Some(0));
    assert_eq!(
        all.stdout,
        b"Alpha\talpha-ref\t300\ndefault\tdefault-ref\t300\nwork\twork-ref\t300\n"
    );
    assert!(!root.0.join(".local/share/codex/manager/state-v1").exists());
}

#[test]
fn session_resume_selects_once_and_preserves_core_boundary() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));
    let work_home = root.0.join(".local/share/codex/manager/profiles/work/home");
    write_session(
        &work_home,
        "2026/09/resume-ref.jsonl",
        b"resume-body-secret-sentinel",
        400,
    );

    let resumed = run_manager(
        &root.0,
        &core,
        &[
            "session",
            "resume",
            "resume-ref",
            "--profile",
            "work",
            "--",
            "--model",
            "gpt-5",
        ],
        Some(Path::new("/caller/environment")),
    );
    assert_eq!(resumed.status.code(), Some(37));
    assert!(resumed
        .stdout
        .windows(b"ARGC=4\n".len())
        .any(|window| { window == b"ARGC=4\n" }));
    assert!(resumed
        .stdout
        .windows(b"ARG=<resume>\n".len())
        .any(|window| { window == b"ARG=<resume>\n" }));
    assert!(resumed
        .stdout
        .windows(b"ARG=<resume-ref>\n".len())
        .any(|window| window == b"ARG=<resume-ref>\n"));
    assert!(resumed
        .stdout
        .windows(b"ARG=<--model>\n".len())
        .any(|window| { window == b"ARG=<--model>\n" }));
    assert!(resumed
        .stdout
        .windows(b"ARG=<gpt-5>\n".len())
        .any(|window| { window == b"ARG=<gpt-5>\n" }));
    assert!(resumed
        .stdout
        .windows(b"HOME_SET=x\n".len())
        .any(|window| window == b"HOME_SET=x\n"));
    let expected_home_line = format!("HOME_VALUE={}\n", work_home.display());
    assert!(resumed
        .stdout
        .windows(expected_home_line.len())
        .any(|window| window == expected_home_line.as_bytes()));
    assert!(!resumed
        .stdout
        .windows(b"resume-body-secret-sentinel".len())
        .any(|window| window == b"resume-body-secret-sentinel"));
    assert!(resumed
        .stdout
        .windows(b"API_SET=\n".len())
        .any(|window| window == b"API_SET=\n"));
    assert!(resumed
        .stdout
        .windows(b"ENTRY_SET=\n".len())
        .any(|window| window == b"ENTRY_SET=\n"));
    assert_eq!(
        fs::read(root.0.join(".local/share/codex/manager/state-v1")).unwrap(),
        b"codex-manager-state-v1\nlast_profile\twork\n"
    );

    let resumed_from_selection = run_manager(
        &root.0,
        &core,
        &["session", "resume", "resume-ref", "--", "--version"],
        Some(Path::new("/caller/environment")),
    );
    assert_eq!(resumed_from_selection.status.code(), Some(37));
    assert!(resumed_from_selection
        .stdout
        .windows(b"ARG=<resume>\n".len())
        .any(|window| window == b"ARG=<resume>\n"));
    assert!(resumed_from_selection
        .stdout
        .windows(b"ARG=<--version>\n".len())
        .any(|window| window == b"ARG=<--version>\n"));
}

#[test]
fn session_resume_never_launches_or_selects_on_missing_or_ambiguous_reference() {
    let root = TestRoot::new();
    let core = write_core_probe(&root.0);
    let missing = run_manager(&root.0, &core, &["session", "resume", "missing"], None);
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    assert!(missing
        .stderr
        .windows(b"codex termux: session is unavailable\n".len())
        .any(|window| window == b"codex termux: session is unavailable\n"));
    assert!(!root.0.join(".local/share/codex/manager/state-v1").exists());

    let created = run_manager(&root.0, &core, &["profile", "create", "work"], None);
    assert_eq!(created.status.code(), Some(0));
    let work_home = root.0.join(".local/share/codex/manager/profiles/work/home");
    write_session(&work_home, "one/ambiguous-ref.jsonl", b"one", 500);
    write_session(&work_home, "two/ambiguous-ref.jsonl", b"two", 500);
    let ambiguous = run_manager(
        &root.0,
        &core,
        &["session", "resume", "ambiguous-ref", "--profile", "work"],
        None,
    );
    assert_eq!(ambiguous.status.code(), Some(1));
    assert!(ambiguous.stdout.is_empty());
    assert!(ambiguous
        .stderr
        .windows(b"codex termux: session is unavailable\n".len())
        .any(|window| window == b"codex termux: session is unavailable\n"));
    assert!(!root.0.join(".local/share/codex/manager/state-v1").exists());
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

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}
