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

#[cfg(unix)]
pub const RESOLVER_FD: std::os::raw::c_int = 33;
#[cfg(unix)]
pub const CONFIG_DIR_FD: std::os::raw::c_int = 34;
#[cfg(unix)]
const SAFE_MIN_FD: std::os::raw::c_int = 35;

#[cfg(unix)]
const F_DUPFD_CLOEXEC: std::os::raw::c_int = 1030;
#[cfg(unix)]
const F_GETFD: std::os::raw::c_int = 1;
#[cfg(unix)]
const F_SETFD: std::os::raw::c_int = 2;
#[cfg(unix)]
const EBADF: std::os::raw::c_int = 9;

#[cfg(unix)]
extern "C" {
    fn dup2(oldfd: std::os::raw::c_int, newfd: std::os::raw::c_int) -> std::os::raw::c_int;
    fn fcntl(fd: std::os::raw::c_int, cmd: std::os::raw::c_int, ...) -> std::os::raw::c_int;
    fn close(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(unix)]
#[derive(Debug)]
enum PriorFdState {
    Absent,
    Present {
        backup_fd: std::os::raw::c_int,
        flags: std::os::raw::c_int,
    },
}

#[cfg(unix)]
impl PriorFdState {
    /// Captures the current state of `target_fd`.
    ///
    /// Safety Invariants:
    /// - `fcntl(target_fd, F_GETFD)` safely probes `target_fd`. If negative and errno is `EBADF`,
    ///   the descriptor is treated as `Absent`. Any other probe error is returned as `Err`.
    /// - If open, `fcntl(target_fd, F_DUPFD_CLOEXEC, SAFE_MIN_FD)` duplicates it to a safe
    ///   descriptor >= SAFE_MIN_FD (35) with `FD_CLOEXEC` set. This guarantees:
    ///   1. The backup descriptor is strictly above target descriptors 33 and 34.
    ///   2. The backup descriptor is automatically closed by the kernel on successful `execve`.
    unsafe fn capture(target_fd: std::os::raw::c_int) -> std::io::Result<Self> {
        let flags = fcntl(target_fd, F_GETFD);
        if flags < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(EBADF) {
                Ok(PriorFdState::Absent)
            } else {
                Err(err)
            }
        } else {
            let backup = fcntl(target_fd, F_DUPFD_CLOEXEC, SAFE_MIN_FD);
            if backup < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(PriorFdState::Present {
                backup_fd: backup,
                flags,
            })
        }
    }

    /// Restores `target_fd` to its previously captured state and closes the backup descriptor.
    ///
    /// Safety Invariants:
    /// - If originally `Absent`, `close(target_fd)` is invoked to ensure `target_fd` is absent.
    /// - If originally `Present`, `dup2(backup_fd, target_fd)` restores the original open file
    ///   description, `fcntl(target_fd, F_SETFD, flags)` restores descriptor flags, and `close(backup_fd)`
    ///   releases the safe backup descriptor.
    unsafe fn restore_and_cleanup(&mut self, target_fd: std::os::raw::c_int) {
        match *self {
            PriorFdState::Absent => {
                let _ = close(target_fd);
            }
            PriorFdState::Present {
                ref mut backup_fd,
                flags,
            } => {
                if *backup_fd >= 0 {
                    let _ = dup2(*backup_fd, target_fd);
                    let _ = fcntl(target_fd, F_SETFD, flags);
                    let _ = close(*backup_fd);
                    *backup_fd = -1;
                }
            }
        }
    }
}

#[cfg(unix)]
struct FdRestorationGuard {
    target_fd: std::os::raw::c_int,
    state: PriorFdState,
    armed: bool,
}

#[cfg(unix)]
impl FdRestorationGuard {
    unsafe fn capture(target_fd: std::os::raw::c_int) -> std::io::Result<Self> {
        let state = PriorFdState::capture(target_fd)?;
        Ok(Self {
            target_fd,
            state,
            armed: true,
        })
    }

    unsafe fn restore(&mut self) {
        if self.armed {
            self.state.restore_and_cleanup(self.target_fd);
            self.armed = false;
        }
    }
}

#[cfg(unix)]
impl Drop for FdRestorationGuard {
    fn drop(&mut self) {
        if self.armed {
            unsafe {
                self.state.restore_and_cleanup(self.target_fd);
            }
            self.armed = false;
        }
    }
}

#[cfg(unix)]
struct SafeFd(std::os::raw::c_int);

#[cfg(unix)]
impl Drop for SafeFd {
    fn drop(&mut self) {
        if self.0 >= 0 {
            unsafe {
                close(self.0);
            }
            self.0 = -1;
        }
    }
}

/// Executes the given upstream program with runtime file descriptors mapped, replacing the current process.
///
/// Maps `resolver_path` (opened read-only) to file descriptor 33 and `config_dir` (opened read-only)
/// to file descriptor 34, with `FD_CLOEXEC` cleared so both descriptors survive final exec.
/// All existing argv, stream, exit status, and environment fence semantics from `exec_upstream` are preserved.
///
/// If setup or exec fails, prior caller descriptors at FD 33 and FD 34 are restored exactly to their
/// prior state, and the resulting `std::io::Error` is returned.
#[cfg(unix)]
pub fn exec_upstream_with_runtime_fds<P, I, S, R, C>(
    program: P,
    args: I,
    resolver_path: R,
    config_dir: C,
) -> std::io::Error
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let res = (|| -> std::io::Result<()> {
        // Step 1: Capture prior FD 33/34 states with restoration guards BEFORE opening or modifying any FDs.
        let mut guard_33 = unsafe { FdRestorationGuard::capture(RESOLVER_FD)? };
        let mut guard_34 = unsafe { FdRestorationGuard::capture(CONFIG_DIR_FD)? };

        // Step 2: Open resolver file read-only via standard library. Reject directories.
        let resolver_file = std::fs::File::open(resolver_path.as_ref())?;
        let res_meta = resolver_file.metadata()?;
        if res_meta.is_dir() {
            return Err(std::io::Error::from_raw_os_error(21 /* EISDIR */));
        }

        // Step 3: Open managed configuration directory read-only via standard library. Reject non-directories.
        let config_file = std::fs::File::open(config_dir.as_ref())?;
        let cfg_meta = config_file.metadata()?;
        if !cfg_meta.is_dir() {
            return Err(std::io::Error::from_raw_os_error(20 /* ENOTDIR */));
        }

        // Step 4: Duplicate source descriptors to safe descriptors >= SAFE_MIN_FD (35) with FD_CLOEXEC.
        // Safety Invariants:
        // Duplicating opened source descriptors to >= 35 before mapping guarantees that even if
        // `File::open` allocated FD 33 or 34, mapping 33 and 34 will not overwrite or corrupt source descriptors.
        use std::os::unix::io::AsRawFd;
        let safe_res_fd = unsafe { fcntl(resolver_file.as_raw_fd(), F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
        if safe_res_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        drop(resolver_file);
        let mut safe_res = SafeFd(safe_res_fd);

        let safe_cfg_fd = unsafe { fcntl(config_file.as_raw_fd(), F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
        if safe_cfg_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        drop(config_file);
        let mut safe_cfg = SafeFd(safe_cfg_fd);

        // Step 5: Map safe_res to FD 33 and clear FD_CLOEXEC.
        // Safety Invariants:
        // `dup2` maps `safe_res.0` onto descriptor 33. `fcntl(33, F_SETFD, 0)` explicitly clears
        // `FD_CLOEXEC`, guaranteeing that descriptor 33 survives the final execve.
        if unsafe { dup2(safe_res.0, RESOLVER_FD) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if unsafe { fcntl(RESOLVER_FD, F_SETFD, 0 as std::os::raw::c_int) } < 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Step 6: Map safe_cfg to FD 34 and clear FD_CLOEXEC.
        // Safety Invariants:
        // `dup2` maps `safe_cfg.0` onto descriptor 34. `fcntl(34, F_SETFD, 0)` explicitly clears
        // `FD_CLOEXEC`, guaranteeing that descriptor 34 survives the final execve.
        if unsafe { dup2(safe_cfg.0, CONFIG_DIR_FD) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if unsafe { fcntl(CONFIG_DIR_FD, F_SETFD, 0 as std::os::raw::c_int) } < 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Step 7: Close safe temporary duplicates so they do not leak into the exec target.
        // Safety Invariants:
        // Closing temporary safe duplicates leaves only 33 and 34 plus ordinary caller descriptors.
        unsafe {
            close(safe_res.0);
            safe_res.0 = -1;
            close(safe_cfg.0);
            safe_cfg.0 = -1;
        }

        // Step 8: Build and execute command with exact five environment removals.
        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new(program.as_ref());
        cmd.args(args)
            .env_remove("CODEX_MANAGED_BY_NPM")
            .env_remove("CODEX_MANAGED_BY_BUN")
            .env_remove("CODEX_MANAGED_PACKAGE_ROOT")
            .env_remove("LD_PRELOAD")
            .env_remove("LD_LIBRARY_PATH");

        let err = cmd.exec();

        // Step 9: If cmd.exec() returns, exec failed. Explicitly restore prior FD 33/34 state.
        unsafe {
            guard_34.restore();
            guard_33.restore();
        }
        Err(err)
    })();

    match res {
        Err(err) => err,
        Ok(()) => unreachable!("exec never returns on success"),
    }
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
    const PROBE_RESOLVER_PATH_ENV: &str = "CODEX_TEST_EXEC_RESOLVER_PATH";
    #[cfg(unix)]
    const PROBE_CONFIG_DIR_PATH_ENV: &str = "CODEX_TEST_EXEC_CONFIG_DIR_PATH";

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
    fn run_exec_probe_with_env(
        scenario: &str,
        extra_env: &[(&str, &std::ffi::OsStr)],
    ) -> ProbeResult {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);

        let current_exe = std::env::current_exe().expect("failed to get current_exe");
        let shell = resolve_test_shell();

        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let stdout_file = temp_dir.join(format!("codex-test-stdout-{pid}-{count}-{scenario}.tmp"));
        let stderr_file = temp_dir.join(format!("codex-test-stderr-{pid}-{count}-{scenario}.tmp"));

        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("tests::exec_probe_subprocess_entry")
            .arg("--exact")
            .stdout(std::process::Stdio::null())
            .env(PROBE_ROLE_ENV, PROBE_ROLE_LAUNCHER)
            .env(PROBE_SHELL_ENV, shell)
            .env(PROBE_SCENARIO_ENV, scenario)
            .env(PROBE_STDOUT_FILE_ENV, &stdout_file)
            .env(PROBE_STDERR_FILE_ENV, &stderr_file);

        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        let status = cmd.status().expect("failed to execute probe subprocess");

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
    fn run_exec_probe(scenario: &str) -> ProbeResult {
        run_exec_probe_with_env(scenario, &[])
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
            drop(out_file);
        }

        if let Some(stderr_path) = std::env::var_os(PROBE_STDERR_FILE_ENV) {
            let err_file =
                std::fs::File::create(stderr_path).expect("failed to create stderr probe file");
            unsafe {
                dup2(err_file.as_raw_fd(), 2);
            }
            drop(err_file);
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
            "runtime_fds_probe_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                std::env::set_var(PROBE_SCENARIO_ENV, "runtime_fds_probe_target");
                let current_exe = std::env::current_exe().expect("current_exe");
                let err = exec_upstream_with_runtime_fds(
                    current_exe,
                    &[
                        OsStr::new("tests::exec_probe_subprocess_entry"),
                        OsStr::new("--exact"),
                    ],
                    resolver_path,
                    config_dir_path,
                );
                panic!("exec_upstream_with_runtime_fds failed to replace process: {err}");
            }
            "runtime_fds_probe_target" => {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::io::FromRawFd;

                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let expected_res_meta =
                    std::fs::metadata(&resolver_path).expect("failed to stat resolver");
                let expected_cfg_meta =
                    std::fs::metadata(&config_dir_path).expect("failed to stat config_dir");

                // 1. Verify /proc/self/fd/33
                let fd33_link = std::fs::read_link("/proc/self/fd/33")
                    .expect("failed to readlink /proc/self/fd/33");
                let canonical_res = std::path::Path::new(&resolver_path)
                    .canonicalize()
                    .expect("canonicalize resolver");
                assert_eq!(
                    fd33_link.canonicalize().unwrap_or(fd33_link.clone()),
                    canonical_res
                );
                let fd33_meta =
                    std::fs::metadata("/proc/self/fd/33").expect("stat /proc/self/fd/33");
                assert_eq!(fd33_meta.dev(), expected_res_meta.dev());
                assert_eq!(fd33_meta.ino(), expected_res_meta.ino());

                // Read all bytes from FD 33
                let mut content = Vec::new();
                unsafe {
                    let mut file = std::fs::File::from_raw_fd(33);
                    use std::io::Read;
                    file.read_to_end(&mut content).expect("read fd 33");
                    std::mem::forget(file); // Do not close fd 33
                }
                let expected_bytes = std::fs::read(&resolver_path).expect("read resolver file");
                assert_eq!(content, expected_bytes);

                // Prove write to FD 33 fails because it was opened read-only
                let write_res = unsafe {
                    let mut file = std::fs::File::from_raw_fd(33);
                    use std::io::Write;
                    let r = file.write_all(b"MUTATION_ATTEMPT");
                    std::mem::forget(file);
                    r
                };
                assert!(
                    write_res.is_err(),
                    "write to FD 33 must fail (opened read-only)"
                );

                // 2. Verify /proc/self/fd/34
                let fd34_link = std::fs::read_link("/proc/self/fd/34")
                    .expect("failed to readlink /proc/self/fd/34");
                let canonical_cfg = std::path::Path::new(&config_dir_path)
                    .canonicalize()
                    .expect("canonicalize config_dir");
                assert_eq!(
                    fd34_link.canonicalize().unwrap_or(fd34_link.clone()),
                    canonical_cfg
                );
                let fd34_meta =
                    std::fs::metadata("/proc/self/fd/34").expect("stat /proc/self/fd/34");
                assert!(fd34_meta.is_dir(), "FD 34 must be a directory");
                assert_eq!(fd34_meta.dev(), expected_cfg_meta.dev());
                assert_eq!(fd34_meta.ino(), expected_cfg_meta.ino());

                // 3. Verify no leaked backup descriptors in range 35..64
                for fd in 35..64 {
                    assert!(
                        unsafe { fcntl(fd, F_GETFD) < 0 },
                        "expected FD {fd} to be closed, but it was open"
                    );
                }

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"RUNTIME_FDS_VERIFIED\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "runtime_fds_shell_probe_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let script = r#"
res_content=$(cat /proc/self/fd/33)
if [ "$res_content" != "nameserver 8.8.8.8" ]; then
    printf "RESOLVER_CONTENT_MISMATCH:%s\n" "$res_content" >&2
    exit 20
fi

if [ ! -d /proc/self/fd/34 ]; then
    printf "FD34_NOT_DIR\n" >&2
    exit 21
fi

printf "SHELL_RUNTIME_FDS_SUCCESS\n"
exit 0
"#;
                let args: Vec<&OsStr> = vec![
                    OsStr::new("-c"),
                    OsStr::new(script),
                    OsStr::new("upstream-probe"),
                ];

                let err =
                    exec_upstream_with_runtime_fds(shell, args, resolver_path, config_dir_path);
                panic!("exec_upstream_with_runtime_fds failed to replace process: {err}");
            }
            "failed_exec_restoration_absent" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                // Ensure 33 and 34 are absent initially
                unsafe {
                    close(33);
                    close(34);
                }
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b4-nonexistent-bin"),
                    &[OsStr::new("--version")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

                // Verify 33 and 34 are absent again
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"RESTORED_ABSENT_SUCCESS\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "failed_exec_restoration_sentinels" => {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::io::AsRawFd;
                use std::os::unix::io::FromRawFd;

                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let temp_dir = std::env::temp_dir();
                let pid = std::process::id();
                let sentinel33_path = temp_dir.join(format!("codex-sentinel-33-{pid}.tmp"));
                let sentinel34_path = temp_dir.join(format!("codex-sentinel-34-{pid}.tmp"));

                let s33_bytes = b"ORIGINAL_SENTINEL_33_DATA_EXACT";
                let s34_bytes = b"ORIGINAL_SENTINEL_34_DATA_EXACT";
                std::fs::write(&sentinel33_path, s33_bytes).expect("write sentinel 33");
                std::fs::write(&sentinel34_path, s34_bytes).expect("write sentinel 34");

                let f33 = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&sentinel33_path)
                    .expect("open sentinel 33");
                let f34 = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&sentinel34_path)
                    .expect("open sentinel 34");

                let meta33 = f33.metadata().expect("meta sentinel 33");
                let meta34 = f34.metadata().expect("meta sentinel 34");

                unsafe {
                    dup2(f33.as_raw_fd(), 33);
                    dup2(f34.as_raw_fd(), 34);
                }
                drop(f33);
                drop(f34);

                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b4-nonexistent-bin"),
                    &[OsStr::new("--version")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

                // Verify FD 33 is restored to sentinel33
                let restored_meta33 = std::fs::metadata("/proc/self/fd/33").expect("stat fd 33");
                assert_eq!(restored_meta33.dev(), meta33.dev());
                assert_eq!(restored_meta33.ino(), meta33.ino());

                // Verify reading from FD 33 gives sentinel bytes (seeking back to 0)
                let mut buf33 = Vec::new();
                unsafe {
                    use std::io::Read;
                    use std::io::Seek;
                    let mut file = std::fs::File::from_raw_fd(33);
                    let _ = file.rewind();
                    file.read_to_end(&mut buf33).expect("read restored fd 33");
                    // Verify write to FD 33 succeeds because sentinel is read-write
                    use std::io::Write;
                    file.write_all(b"_APPENDED")
                        .expect("write to restored fd 33");
                    std::mem::forget(file);
                }
                assert_eq!(&buf33[..s33_bytes.len()], s33_bytes);

                // Verify FD 34 is restored to sentinel34
                let restored_meta34 = std::fs::metadata("/proc/self/fd/34").expect("stat fd 34");
                assert_eq!(restored_meta34.dev(), meta34.dev());
                assert_eq!(restored_meta34.ino(), meta34.ino());

                let _ = std::fs::remove_file(&sentinel33_path);
                let _ = std::fs::remove_file(&sentinel34_path);

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"RESTORED_SENTINELS_SUCCESS\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "runtime_fds_env_fence_evidence" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

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

                let err =
                    exec_upstream_with_runtime_fds(shell, args, resolver_path, config_dir_path);
                panic!("exec_upstream_with_runtime_fds failed to replace process: {err}");
            }
            "runtime_fds_collision_probe_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                // Keep dummy files open so descriptors 3..33 are occupied with FD_CLOEXEC
                let mut dummy_files = Vec::new();
                for _ in 3..33 {
                    dummy_files.push(std::fs::File::open("/dev/null").expect("open /dev/null"));
                }
                unsafe {
                    close(33);
                    close(34);
                }

                std::env::set_var(PROBE_SCENARIO_ENV, "runtime_fds_probe_target");
                let current_exe = std::env::current_exe().expect("current_exe");
                let err = exec_upstream_with_runtime_fds(
                    current_exe,
                    &[
                        OsStr::new("tests::exec_probe_subprocess_entry"),
                        OsStr::new("--exact"),
                    ],
                    resolver_path,
                    config_dir_path,
                );
                panic!("exec_upstream_with_runtime_fds failed to replace process: {err}");
            }
            "failed_exec_restoration_collision" => {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::io::AsRawFd;
                use std::os::unix::io::FromRawFd;

                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let temp_dir = std::env::temp_dir();
                let pid = std::process::id();
                let sentinel33_path =
                    temp_dir.join(format!("codex-sentinel-collision-33-{pid}.tmp"));
                let s33_bytes = b"COLLISION_SENTINEL_33_DATA";
                std::fs::write(&sentinel33_path, s33_bytes).expect("write sentinel 33");

                let f33 = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&sentinel33_path)
                    .expect("open sentinel 33");
                let meta33 = f33.metadata().expect("meta sentinel 33");

                // Keep dummy files open so descriptors 3..33 are occupied with FD_CLOEXEC
                let mut dummy_files = Vec::new();
                for _ in 3..33 {
                    dummy_files.push(std::fs::File::open("/dev/null").expect("open /dev/null"));
                }

                // Place sentinel at 33, close 34
                unsafe {
                    dup2(f33.as_raw_fd(), 33);
                    close(34);
                }
                drop(f33);

                assert!(unsafe { fcntl(33, F_GETFD) >= 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b4-nonexistent-bin"),
                    &[OsStr::new("--version")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

                // Verify 33 is restored to sentinel and 34 remains absent
                assert!(unsafe { fcntl(33, F_GETFD) >= 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                let restored_meta33 = std::fs::metadata("/proc/self/fd/33").expect("stat fd 33");
                assert_eq!(restored_meta33.dev(), meta33.dev());
                assert_eq!(restored_meta33.ino(), meta33.ino());

                let mut buf33 = Vec::new();
                unsafe {
                    use std::io::Read;
                    use std::io::Seek;
                    let mut file = std::fs::File::from_raw_fd(33);
                    let _ = file.rewind();
                    file.read_to_end(&mut buf33).expect("read restored fd 33");
                    std::mem::forget(file);
                }
                assert_eq!(&buf33[..s33_bytes.len()], s33_bytes);

                let _ = std::fs::remove_file(&sentinel33_path);

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"RESTORED_COLLISION_SUCCESS\n");
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

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_evidence_and_resolver_immutability() {
        use std::os::unix::fs::MetadataExt;

        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-resolver-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");

        let resolver_bytes =
            b"# M1-B4 test synthetic resolver source\nnameserver 192.0.2.53\noptions timeout:2\n";
        std::fs::write(&resolver_path, resolver_bytes)
            .expect("failed to write synthetic resolv.conf");

        let meta_before =
            std::fs::metadata(&resolver_path).expect("failed to get resolver metadata before");

        let result = run_exec_probe_with_env(
            "runtime_fds_probe_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"RUNTIME_FDS_VERIFIED\n");
        assert_eq!(result.stderr, b"");

        // Capture after evidence and assert complete immutability
        let meta_after =
            std::fs::metadata(&resolver_path).expect("failed to get resolver metadata after");
        let bytes_after = std::fs::read(&resolver_path).expect("failed to read resolver after");

        assert_eq!(bytes_after, resolver_bytes);
        assert_eq!(meta_after.mode(), meta_before.mode());
        assert_eq!(meta_after.dev(), meta_before.dev());
        assert_eq!(meta_after.ino(), meta_before.ino());
        assert_eq!(meta_after.len(), meta_before.len());
        assert_eq!(meta_after.mtime(), meta_before.mtime());
        assert_eq!(meta_after.mtime_nsec(), meta_before.mtime_nsec());

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_shell_probe() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-shell-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");

        let resolver_bytes = b"nameserver 8.8.8.8\n";
        std::fs::write(&resolver_path, resolver_bytes)
            .expect("failed to write synthetic resolv.conf");

        let result = run_exec_probe_with_env(
            "runtime_fds_shell_probe_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"SHELL_RUNTIME_FDS_SUCCESS\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_failed_exec_restores_absent() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-absent-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result = run_exec_probe_with_env(
            "failed_exec_restoration_absent",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"RESTORED_ABSENT_SUCCESS\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_failed_exec_restores_sentinels() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-sentinels-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result = run_exec_probe_with_env(
            "failed_exec_restoration_sentinels",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"RESTORED_SENTINELS_SUCCESS\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_sanitizes_contamination_vars() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-env-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result = run_exec_probe_with_env(
            "runtime_fds_env_fence_evidence",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

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

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_nonexistent_resolver_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-no-res-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let nonexistent_resolver = test_root.join("nonexistent-resolv.conf");
        let config_dir = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir).expect("create config dir");

        unsafe {
            close(33);
            close(34);
        }
        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let err = exec_upstream_with_runtime_fds(
            OsStr::new("sh"),
            &[OsStr::new("-c"), OsStr::new("exit 0")],
            &nonexistent_resolver,
            &config_dir,
        );
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_nonexistent_config_dir_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-no-cfg-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver = test_root.join("resolv.conf");
        std::fs::write(&resolver, b"nameserver 127.0.0.1\n").expect("write resolver");
        let nonexistent_config = test_root.join("nonexistent-config-dir");

        unsafe {
            close(33);
            close(34);
        }
        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let err = exec_upstream_with_runtime_fds(
            OsStr::new("sh"),
            &[OsStr::new("-c"), OsStr::new("exit 0")],
            &resolver,
            &nonexistent_config,
        );
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);

        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_config_dir_is_file_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-cfg-file-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver = test_root.join("resolv.conf");
        std::fs::write(&resolver, b"nameserver 127.0.0.1\n").expect("write resolver");
        let file_as_config = test_root.join("config-file.txt");
        std::fs::write(&file_as_config, b"not a directory").expect("write file");

        unsafe {
            close(33);
            close(34);
        }
        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let err = exec_upstream_with_runtime_fds(
            OsStr::new("sh"),
            &[OsStr::new("-c"), OsStr::new("exit 0")],
            &resolver,
            &file_as_config,
        );
        assert_eq!(err.raw_os_error(), Some(20 /* ENOTDIR */));

        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_resolver_is_dir_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-res-dir-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let dir_as_resolver = test_root.join("resolv-dir");
        std::fs::create_dir_all(&dir_as_resolver).expect("create dir");
        let config_dir = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir).expect("create config dir");

        unsafe {
            close(33);
            close(34);
        }
        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let err = exec_upstream_with_runtime_fds(
            OsStr::new("sh"),
            &[OsStr::new("-c"), OsStr::new("exit 0")],
            &dir_as_resolver,
            &config_dir,
        );
        assert_eq!(err.raw_os_error(), Some(21 /* EISDIR */));

        assert!(unsafe { fcntl(33, F_GETFD) < 0 });
        assert!(unsafe { fcntl(34, F_GETFD) < 0 });

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_handles_fd_collision_on_source_open() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-collision-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");

        let resolver_bytes = b"nameserver 10.10.10.10\n";
        std::fs::write(&resolver_path, resolver_bytes)
            .expect("failed to write synthetic resolv.conf");

        let result = run_exec_probe_with_env(
            "runtime_fds_collision_probe_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"RUNTIME_FDS_VERIFIED\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_failed_exec_restores_collision() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b4-fail-collision-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result = run_exec_probe_with_env(
            "failed_exec_restoration_collision",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"RESTORED_COLLISION_SUCCESS\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }
}
