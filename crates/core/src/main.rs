use std::ffi::OsStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandClass {
    Update,
    Doctor,
    Termux,
    Passthrough,
}

pub fn classify_first_arg(arg: Option<&OsStr>) -> CommandClass {
    match arg.and_then(|a| a.to_str()) {
        Some("update") => CommandClass::Update,
        Some("doctor") => CommandClass::Doctor,
        Some("termux") => CommandClass::Termux,
        _ => CommandClass::Passthrough,
    }
}

/// Executes the given upstream program with the supplied arguments, replacing the current process.
///
/// On Unix/Android, this performs a final process replacement via `execvp`.
/// If process replacement succeeds, this function never returns because the current
/// process image is replaced.
/// If process replacement fails, it returns the resulting `std::io::Error`.
#[cfg(unix)]
pub fn exec_upstream<P, I, S>(program: P, args: I) -> std::io::Error
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    use std::os::unix::process::CommandExt;
    std::process::Command::new(program.as_ref())
        .args(args)
        .exec()
}

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let _ = classify_first_arg(args.next().as_deref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_argv() {
        assert_eq!(classify_first_arg(None), CommandClass::Passthrough);
        assert_eq!(
            classify_first_arg(Some(OsStr::new(""))),
            CommandClass::Passthrough
        );
    }

    #[test]
    fn test_exact_core_commands() {
        assert_eq!(
            classify_first_arg(Some(OsStr::new("update"))),
            CommandClass::Update
        );
        assert_eq!(
            classify_first_arg(Some(OsStr::new("doctor"))),
            CommandClass::Doctor
        );
        assert_eq!(
            classify_first_arg(Some(OsStr::new("termux"))),
            CommandClass::Termux
        );
    }

    #[test]
    fn test_near_miss_spellings_and_affixes() {
        let near_misses = [
            "Update",
            "UPDATE",
            "uPdate",
            "Doctor",
            "DOCTOR",
            "Termux",
            "TERMUX",
            "update ",
            " update",
            "update\n",
            "updates",
            "updated",
            "updater",
            "doctors",
            "doctored",
            "termuxes",
            "termux-cli",
            "updat",
            "docto",
            "termu",
            "up",
            "doc",
            "ter",
            "update-all",
            "doctor-check",
        ];
        for arg in near_misses {
            assert_eq!(
                classify_first_arg(Some(OsStr::new(arg))),
                CommandClass::Passthrough,
                "expected '{}' to classify as Passthrough",
                arg
            );
        }
    }

    #[test]
    fn test_version_flags() {
        assert_eq!(
            classify_first_arg(Some(OsStr::new("--version"))),
            CommandClass::Passthrough
        );
        assert_eq!(
            classify_first_arg(Some(OsStr::new("-V"))),
            CommandClass::Passthrough
        );
        assert_eq!(
            classify_first_arg(Some(OsStr::new("-v"))),
            CommandClass::Passthrough
        );
        assert_eq!(
            classify_first_arg(Some(OsStr::new("version"))),
            CommandClass::Passthrough
        );
    }

    #[test]
    fn test_arbitrary_passthrough_arguments() {
        let passthrough_args = [
            "exec",
            "run",
            "auth",
            "login",
            "logout",
            "--help",
            "-h",
            "--update",
            "--doctor",
            "--termux",
            "subcommand",
            "app:start",
            "123",
            "-",
            "--",
        ];
        for arg in passthrough_args {
            assert_eq!(
                classify_first_arg(Some(OsStr::new(arg))),
                CommandClass::Passthrough,
                "expected '{}' to classify as Passthrough",
                arg
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_non_utf8_first_argument_on_unix() {
        use std::os::unix::ffi::OsStrExt;

        let invalid_utf8_samples: &[&[u8]] = &[
            &[0xff],
            &[0x80],
            &[0xfe, 0xfd],
            &[0xc0, 0xaf],
            b"update\xff",
            b"\xffupdate",
            b"doctor\xfe",
            b"\xfedoctor",
            b"termux\x80",
            b"\x80termux",
        ];

        for sample in invalid_utf8_samples {
            let os_str = OsStr::from_bytes(sample);
            assert_eq!(
                classify_first_arg(Some(os_str)),
                CommandClass::Passthrough,
                "expected non-UTF-8 sample {:?} to classify as Passthrough",
                sample
            );
        }
    }

    #[cfg(unix)]
    const PROBE_ROLE_ENV: &str = "CODEX_TEST_EXEC_PROBE_ROLE";
    #[cfg(unix)]
    const PROBE_ROLE_LAUNCHER: &str = "launcher";
    #[cfg(unix)]
    const PROBE_SHELL_ENV: &str = "CODEX_TEST_EXEC_SHELL";
    #[cfg(unix)]
    const PROBE_SCENARIO_ENV: &str = "CODEX_TEST_EXEC_SCENARIO";
    #[cfg(unix)]
    const PROBE_STDOUT_FILE_ENV: &str = "CODEX_TEST_EXEC_STDOUT_FILE";
    #[cfg(unix)]
    const PROBE_STDERR_FILE_ENV: &str = "CODEX_TEST_EXEC_STDERR_FILE";

    #[cfg(unix)]
    extern "C" {
        fn dup2(oldfd: std::os::raw::c_int, newfd: std::os::raw::c_int) -> std::os::raw::c_int;
    }

    #[cfg(unix)]
    fn resolve_test_shell() -> std::ffi::OsString {
        if let Some(path_var) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join("sh");
                if candidate.is_file() {
                    return candidate.into_os_string();
                }
            }
        }
        for fallback in ["/bin/sh", "/system/bin/sh", "/usr/bin/sh"] {
            let p = std::path::Path::new(fallback);
            if p.is_file() {
                return p.into();
            }
        }
        std::ffi::OsString::from("sh")
    }

    #[cfg(unix)]
    struct ProbeResult {
        status: std::process::ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    }

    #[cfg(unix)]
    fn run_exec_probe(scenario: &str) -> ProbeResult {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);

        let current_exe = std::env::current_exe().expect("failed to get current_exe");
        let shell = resolve_test_shell();

        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let stdout_file = temp_dir.join(format!("codex-test-stdout-{pid}-{count}-{scenario}.tmp"));
        let stderr_file = temp_dir.join(format!("codex-test-stderr-{pid}-{count}-{scenario}.tmp"));

        let status = std::process::Command::new(current_exe)
            .arg("tests::exec_probe_subprocess_entry")
            .arg("--exact")
            .stdout(std::process::Stdio::null())
            .env(PROBE_ROLE_ENV, PROBE_ROLE_LAUNCHER)
            .env(PROBE_SHELL_ENV, shell)
            .env(PROBE_SCENARIO_ENV, scenario)
            .env(PROBE_STDOUT_FILE_ENV, &stdout_file)
            .env(PROBE_STDERR_FILE_ENV, &stderr_file)
            .status()
            .expect("failed to execute probe subprocess");

        let stdout = std::fs::read(&stdout_file).unwrap_or_default();
        let stderr = std::fs::read(&stderr_file).unwrap_or_default();

        let _ = std::fs::remove_file(&stdout_file);
        let _ = std::fs::remove_file(&stderr_file);

        ProbeResult {
            status,
            stdout,
            stderr,
        }
    }

    #[cfg(unix)]
    #[test]
    fn exec_probe_subprocess_entry() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::io::AsRawFd;

        let role = match std::env::var(PROBE_ROLE_ENV) {
            Ok(r) => r,
            Err(_) => return, // Normal test invocation, nothing to execute.
        };

        if role != PROBE_ROLE_LAUNCHER {
            return;
        }

        let shell = std::env::var_os(PROBE_SHELL_ENV)
            .expect("PROBE_SHELL_ENV must be set in launcher probe");
        let scenario = std::env::var(PROBE_SCENARIO_ENV)
            .expect("PROBE_SCENARIO_ENV must be set in launcher probe");

        if let Some(stdout_path) = std::env::var_os(PROBE_STDOUT_FILE_ENV) {
            let out_file =
                std::fs::File::create(stdout_path).expect("failed to create stdout probe file");
            unsafe {
                dup2(out_file.as_raw_fd(), 1);
            }
        }

        if let Some(stderr_path) = std::env::var_os(PROBE_STDERR_FILE_ENV) {
            let err_file =
                std::fs::File::create(stderr_path).expect("failed to create stderr probe file");
            unsafe {
                dup2(err_file.as_raw_fd(), 2);
            }
        }

        match scenario.as_str() {
            "all_evidence" => {
                let script = r#"
printf "STDOUT_START\n"
for a in "$@"; do
    printf "ARG:%s\n" "$a"
done
printf "STDERR_EXACT_BYTES\n" >&2
exit 42
"#;
                let non_utf8_arg = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
                let ordinary_arg = OsStr::new("ordinary arg with spaces and =");

                let args: Vec<&OsStr> = vec![
                    OsStr::new("-c"),
                    OsStr::new(script),
                    OsStr::new("upstream-probe"),
                    OsStr::new("--version"),
                    OsStr::new("-V"),
                    ordinary_arg,
                    non_utf8_arg,
                ];

                let err = exec_upstream(shell, args);
                panic!("exec_upstream failed to replace process: {err}");
            }
            "exit_status_and_custom_streams" => {
                let script = r#"
printf "STATUS_TEST_STDOUT\n"
printf "STATUS_TEST_STDERR\n" >&2
exit 99
"#;
                let args: Vec<&OsStr> = vec![
                    OsStr::new("-c"),
                    OsStr::new(script),
                    OsStr::new("upstream-probe"),
                ];

                let err = exec_upstream(shell, args);
                panic!("exec_upstream failed to replace process: {err}");
            }
            "raw_binary_stream_bytes" => {
                let script = r#"
printf "\001\002\003\377\376\n"
printf "\004\005\006\200\201\n" >&2
exit 7
"#;
                let args: Vec<&OsStr> = vec![
                    OsStr::new("-c"),
                    OsStr::new(script),
                    OsStr::new("upstream-probe"),
                ];

                let err = exec_upstream(shell, args);
                panic!("exec_upstream failed to replace process: {err}");
            }
            _ => panic!("unknown probe scenario: {scenario}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_passthrough_evidence() {
        use std::os::unix::ffi::OsStrExt;

        let result = run_exec_probe("all_evidence");

        assert_eq!(result.status.code(), Some(42));
        assert_eq!(result.stderr, b"STDERR_EXACT_BYTES\n");

        let mut expected_stdout = Vec::new();
        expected_stdout.extend_from_slice(b"STDOUT_START\n");
        expected_stdout.extend_from_slice(b"ARG:--version\n");
        expected_stdout.extend_from_slice(b"ARG:-V\n");
        expected_stdout.extend_from_slice(b"ARG:ordinary arg with spaces and =\n");
        expected_stdout.extend_from_slice(b"ARG:");
        expected_stdout.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).as_bytes());
        expected_stdout.extend_from_slice(b"\n");

        assert_eq!(result.stdout, expected_stdout);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_exit_status_and_custom_streams() {
        let result = run_exec_probe("exit_status_and_custom_streams");

        assert_eq!(result.status.code(), Some(99));
        assert_eq!(result.stdout, b"STATUS_TEST_STDOUT\n");
        assert_eq!(result.stderr, b"STATUS_TEST_STDERR\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_raw_binary_stream_bytes() {
        let result = run_exec_probe("raw_binary_stream_bytes");

        assert_eq!(result.status.code(), Some(7));
        assert_eq!(result.stdout, b"\x01\x02\x03\xff\xfe\n");
        assert_eq!(result.stderr, b"\x04\x05\x06\x80\x81\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_nonexistent_binary_returns_error() {
        let err = exec_upstream(
            OsStr::new("/path/that/does/not/exist/codex-nonexistent-bin"),
            &[OsStr::new("--version")],
        );
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
