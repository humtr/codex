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
    let mut cmd = std::process::Command::new(program.as_ref());
    cmd.args(args)
        .env_remove("CODEX_MANAGED_BY_NPM")
        .env_remove("CODEX_MANAGED_BY_BUN")
        .env_remove("CODEX_MANAGED_PACKAGE_ROOT")
        .env_remove("LD_PRELOAD")
        .env_remove("LD_LIBRARY_PATH");
    cmd.exec()
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
            "env_fence_evidence" => {
                std::env::set_var("CODEX_MANAGED_BY_NPM", "npm-synthetic-val-1");
                std::env::set_var("CODEX_MANAGED_BY_BUN", "bun-synthetic-val-2");
                std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/synthetic/pkg/root");
                std::env::set_var("LD_PRELOAD", "/synthetic/lib/libtest.so");
                std::env::set_var("LD_LIBRARY_PATH", "/synthetic/lib:/synthetic/lib64");
                std::env::set_var("CODEX_TEST_UNRELATED_ALPHA", "alpha-exact-surviving-value");
                std::env::set_var(
                    "CODEX_TEST_UNRELATED_BETA",
                    "beta-value with spaces & = symbols",
                );

                let script = r#"
if [ -z "${CODEX_MANAGED_BY_NPM+x}" ]; then
    printf "NPM:ABSENT\n"
else
    printf "NPM:PRESENT=%s\n" "$CODEX_MANAGED_BY_NPM"
fi

if [ -z "${CODEX_MANAGED_BY_BUN+x}" ]; then
    printf "BUN:ABSENT\n"
else
    printf "BUN:PRESENT=%s\n" "$CODEX_MANAGED_BY_BUN"
fi

if [ -z "${CODEX_MANAGED_PACKAGE_ROOT+x}" ]; then
    printf "PACKAGE_ROOT:ABSENT\n"
else
    printf "PACKAGE_ROOT:PRESENT=%s\n" "$CODEX_MANAGED_PACKAGE_ROOT"
fi

if [ -z "${LD_PRELOAD+x}" ]; then
    printf "LD_PRELOAD:ABSENT\n"
else
    printf "LD_PRELOAD:PRESENT=%s\n" "$LD_PRELOAD"
fi

if [ -z "${LD_LIBRARY_PATH+x}" ]; then
    printf "LD_LIBRARY_PATH:ABSENT\n"
else
    printf "LD_LIBRARY_PATH:PRESENT=%s\n" "$LD_LIBRARY_PATH"
fi

if [ -n "${CODEX_TEST_UNRELATED_ALPHA+x}" ]; then
    printf "UNRELATED_ALPHA:PRESENT=%s\n" "$CODEX_TEST_UNRELATED_ALPHA"
else
    printf "UNRELATED_ALPHA:ABSENT\n"
fi

if [ -n "${CODEX_TEST_UNRELATED_BETA+x}" ]; then
    printf "UNRELATED_BETA:PRESENT=%s\n" "$CODEX_TEST_UNRELATED_BETA"
else
    printf "UNRELATED_BETA:ABSENT\n"
fi
exit 0
"#;
                let args: Vec<&OsStr> = vec![
                    OsStr::new("-c"),
                    OsStr::new(script),
                    OsStr::new("upstream-probe"),
                ];

                let err = exec_upstream(shell, args);
                panic!("exec_upstream failed to replace process: {err}");
            }
            "env_fence_failure_preserves_env" => {
                std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-npm-failure-test");
                std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-bun-failure-test");
                std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/failure/pkg/root");
                std::env::set_var("LD_PRELOAD", "/probe/fake/preload.so");
                std::env::set_var("LD_LIBRARY_PATH", "/probe/fake/lib");
                std::env::set_var("CODEX_TEST_UNRELATED_FAIL_VAR", "unrelated-value-999");

                let err = exec_upstream(
                    OsStr::new("/path/that/does/not/exist/codex-nonexistent-failure-probe"),
                    &[OsStr::new("--version")],
                );
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

                assert_eq!(
                    std::env::var("CODEX_MANAGED_BY_NPM").as_deref(),
                    Ok("probe-npm-failure-test")
                );
                assert_eq!(
                    std::env::var("CODEX_MANAGED_BY_BUN").as_deref(),
                    Ok("probe-bun-failure-test")
                );
                assert_eq!(
                    std::env::var("CODEX_MANAGED_PACKAGE_ROOT").as_deref(),
                    Ok("/probe/failure/pkg/root")
                );
                assert_eq!(
                    std::env::var("LD_PRELOAD").as_deref(),
                    Ok("/probe/fake/preload.so")
                );
                assert_eq!(
                    std::env::var("LD_LIBRARY_PATH").as_deref(),
                    Ok("/probe/fake/lib")
                );
                assert_eq!(
                    std::env::var("CODEX_TEST_UNRELATED_FAIL_VAR").as_deref(),
                    Ok("unrelated-value-999")
                );

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"EXEC_FAILURE_ENV_PRESERVED\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
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

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_sanitizes_contamination_vars_and_preserves_unrelated() {
        let result = run_exec_probe("env_fence_evidence");

        assert_eq!(result.status.code(), Some(0));
        let stdout_str = String::from_utf8(result.stdout).expect("valid utf-8 output from probe");
        let expected = "\
NPM:ABSENT\n\
BUN:ABSENT\n\
PACKAGE_ROOT:ABSENT\n\
LD_PRELOAD:ABSENT\n\
LD_LIBRARY_PATH:ABSENT\n\
UNRELATED_ALPHA:PRESENT=alpha-exact-surviving-value\n\
UNRELATED_BETA:PRESENT=beta-value with spaces & = symbols\n";
        assert_eq!(stdout_str, expected);
        assert_eq!(result.stderr, b"");
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_failed_exec_preserves_caller_process_environment() {
        let result = run_exec_probe("env_fence_failure_preserves_env");

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"EXEC_FAILURE_ENV_PRESERVED\n");
        assert_eq!(result.stderr, b"");

        // Verify parent test runner environment is not contaminated
        assert_ne!(
            std::env::var("CODEX_MANAGED_BY_NPM").as_deref(),
            Ok("probe-npm-failure-test")
        );
        assert_ne!(
            std::env::var("CODEX_TEST_UNRELATED_FAIL_VAR").as_deref(),
            Ok("unrelated-value-999")
        );
    }
}
