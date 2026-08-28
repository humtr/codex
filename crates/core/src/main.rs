use std::ffi::{OsStr, OsString};

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum PassthroughError {
    UnsupportedSandboxMode(String),
    UnsupportedSandboxSubcommand,
}

impl std::fmt::Display for PassthroughError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassthroughError::UnsupportedSandboxMode(mode) => {
                write!(
                    f,
                    "Termux does not support Linux sandbox mode '{mode}': Linux namespace and bwrap sandboxing cannot be enforced"
                )
            }
            PassthroughError::UnsupportedSandboxSubcommand => {
                write!(
                    f,
                    "Termux does not support 'sandbox linux' subcommand: Linux namespace and bwrap sandboxing cannot be enforced"
                )
            }
        }
    }
}

impl std::error::Error for PassthroughError {}

fn normalize_sandbox_value(raw: &str) -> &str {
    let trimmed = raw.trim();
    let unquoted = if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    unquoted.trim()
}

fn check_unsupported_sandbox_flag_val(raw: &str) -> Option<String> {
    let val = normalize_sandbox_value(raw);
    match val {
        "read-only" | "workspace-write" => Some(val.to_owned()),
        _ => None,
    }
}

fn check_unsupported_config_token(token: &str) -> Option<String> {
    let token = token.strip_prefix('=').unwrap_or(token);
    let (key, raw_val) = token.split_once('=')?;
    if normalize_sandbox_value(key) != "sandbox_mode" {
        return None;
    }
    let val = normalize_sandbox_value(raw_val);
    if val.is_empty() || val == "danger-full-access" {
        None
    } else {
        Some(val.to_owned())
    }
}

/// Plans upstream passthrough arguments for Termux execution.
///
/// Validates that explicit Linux sandbox requests that Termux cannot enforce
/// (such as `read-only`, `workspace-write`, and `sandbox linux`) fail clearly.
/// On accepted arguments, prepends exactly `-c` and `sandbox_mode="danger-full-access"`
/// before all original user arguments unchanged.
#[allow(dead_code)]
fn plan_passthrough_args<I, S>(args: I) -> Result<Vec<OsString>, PassthroughError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let original: Vec<OsString> = args.into_iter().map(Into::into).collect();

    // 1. Leading argv check: exactly "sandbox", "linux" as argv[0], argv[1].
    if original.len() >= 2
        && original[0].to_str() == Some("sandbox")
        && original[1].to_str() == Some("linux")
    {
        return Err(PassthroughError::UnsupportedSandboxSubcommand);
    }

    // 2. Scan before the first exact "--" separator.
    let mut i = 0;
    while i < original.len() {
        let s = match original[i].to_str() {
            Some(s) => s,
            None => {
                i += 1;
                continue;
            }
        };

        if s == "--" {
            break;
        }

        if s == "--sandbox" || s == "-s" {
            if i + 1 < original.len() {
                i += 1;
                if let Some(next_str) = original[i].to_str() {
                    if let Some(unsupported) = check_unsupported_sandbox_flag_val(next_str) {
                        return Err(PassthroughError::UnsupportedSandboxMode(unsupported));
                    }
                }
            }
        } else if let Some(val) = s.strip_prefix("--sandbox=") {
            if let Some(unsupported) = check_unsupported_sandbox_flag_val(val) {
                return Err(PassthroughError::UnsupportedSandboxMode(unsupported));
            }
        } else if let Some(val) = s.strip_prefix("-s") {
            if !val.is_empty() {
                let val = val.strip_prefix('=').unwrap_or(val);
                if let Some(unsupported) = check_unsupported_sandbox_flag_val(val) {
                    return Err(PassthroughError::UnsupportedSandboxMode(unsupported));
                }
            }
        } else if s == "--config" || s == "-c" {
            if i + 1 < original.len() {
                i += 1;
                if let Some(next_str) = original[i].to_str() {
                    if let Some(unsupported) = check_unsupported_config_token(next_str) {
                        return Err(PassthroughError::UnsupportedSandboxMode(unsupported));
                    }
                }
            }
        } else if let Some(token) = s.strip_prefix("--config=") {
            if let Some(unsupported) = check_unsupported_config_token(token) {
                return Err(PassthroughError::UnsupportedSandboxMode(unsupported));
            }
        } else if let Some(token) = s.strip_prefix("-c") {
            if !token.is_empty() {
                if let Some(unsupported) = check_unsupported_config_token(token) {
                    return Err(PassthroughError::UnsupportedSandboxMode(unsupported));
                }
            }
        }

        i += 1;
    }

    let mut planned = Vec::with_capacity(original.len() + 2);
    planned.push(OsString::from("-c"));
    planned.push(OsString::from("sandbox_mode=\"danger-full-access\""));
    planned.extend(original);

    Ok(planned)
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
    unsafe fn restore_and_cleanup(
        &mut self,
        target_fd: std::os::raw::c_int,
    ) -> std::io::Result<()> {
        match *self {
            PriorFdState::Absent => {
                if close(target_fd) == 0 {
                    return Ok(());
                }
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(EBADF) {
                    Ok(())
                } else {
                    Err(err)
                }
            }
            PriorFdState::Present {
                ref mut backup_fd,
                flags,
            } => {
                if *backup_fd < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "runtime FD restoration backup is no longer available",
                    ));
                }

                let mut first_error = None;
                if dup2(*backup_fd, target_fd) < 0 {
                    first_error = Some(std::io::Error::last_os_error());
                } else if fcntl(target_fd, F_SETFD, flags) < 0 {
                    first_error = Some(std::io::Error::last_os_error());
                }

                if close(*backup_fd) < 0 && first_error.is_none() {
                    first_error = Some(std::io::Error::last_os_error());
                }
                // close(2) may have closed the descriptor even when it reports EINTR;
                // never retry or let Drop close a possibly-reused descriptor number.
                *backup_fd = -1;

                match first_error {
                    Some(err) => Err(err),
                    None => Ok(()),
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

    unsafe fn restore(&mut self) -> std::io::Result<()> {
        if !self.armed {
            return Ok(());
        }
        let result = self.state.restore_and_cleanup(self.target_fd);
        self.armed = false;
        result
    }
}

#[cfg(unix)]
impl Drop for FdRestorationGuard {
    fn drop(&mut self) {
        if self.armed {
            unsafe {
                let _ = self.state.restore_and_cleanup(self.target_fd);
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
    exec_upstream_with_runtime_fds_and_env(program, args, resolver_path, config_dir, None)
}

#[cfg(unix)]
fn exec_upstream_with_runtime_fds_and_env<P, I, S, R, C>(
    program: P,
    args: I,
    resolver_path: R,
    config_dir: C,
    env_plan: Option<&TermuxBaseEnvPlan>,
) -> std::io::Error
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let res = (|| -> std::io::Result<()> {
        // Capture FD 33 first. If capture of FD 34 fails, restore FD 33 explicitly
        // so a restoration failure is observable rather than hidden by Drop.
        let mut guard_33 = unsafe { FdRestorationGuard::capture(RESOLVER_FD)? };
        let mut guard_34 = match unsafe { FdRestorationGuard::capture(CONFIG_DIR_FD) } {
            Ok(guard) => guard,
            Err(capture_err) => {
                return match unsafe { guard_33.restore() } {
                    Ok(()) => Err(capture_err),
                    Err(restore_err) => Err(restore_err),
                };
            }
        };

        // Once both prior states are captured, keep the operation error separate
        // from restoration. Every returned setup/exec failure restores 34 then 33
        // and a restoration failure takes precedence over the original failure.
        let operation_err = match (|| -> std::io::Result<()> {
            let resolver_file = std::fs::File::open(resolver_path.as_ref())?;
            let res_meta = resolver_file.metadata()?;
            if res_meta.is_dir() {
                return Err(std::io::Error::from_raw_os_error(21 /* EISDIR */));
            }

            let config_file = std::fs::File::open(config_dir.as_ref())?;
            let cfg_meta = config_file.metadata()?;
            if !cfg_meta.is_dir() {
                return Err(std::io::Error::from_raw_os_error(20 /* ENOTDIR */));
            }

            use std::os::unix::io::AsRawFd;
            let safe_res_fd =
                unsafe { fcntl(resolver_file.as_raw_fd(), F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
            if safe_res_fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            drop(resolver_file);
            let mut safe_res = SafeFd(safe_res_fd);

            let safe_cfg_fd =
                unsafe { fcntl(config_file.as_raw_fd(), F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
            if safe_cfg_fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            drop(config_file);
            let mut safe_cfg = SafeFd(safe_cfg_fd);

            if unsafe { dup2(safe_res.0, RESOLVER_FD) } < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if unsafe { fcntl(RESOLVER_FD, F_SETFD, 0 as std::os::raw::c_int) } < 0 {
                return Err(std::io::Error::last_os_error());
            }

            if unsafe { dup2(safe_cfg.0, CONFIG_DIR_FD) } < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if unsafe { fcntl(CONFIG_DIR_FD, F_SETFD, 0 as std::os::raw::c_int) } < 0 {
                return Err(std::io::Error::last_os_error());
            }

            // Close temporary duplicates before exec. These are test-owned/process-local
            // descriptors; close errors are still surfaced before attempting exec.
            let safe_res_fd = safe_res.0;
            safe_res.0 = -1;
            if unsafe { close(safe_res_fd) } < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let safe_cfg_fd = safe_cfg.0;
            safe_cfg.0 = -1;
            if unsafe { close(safe_cfg_fd) } < 0 {
                return Err(std::io::Error::last_os_error());
            }

            use std::os::unix::process::CommandExt;
            let mut cmd = std::process::Command::new(program.as_ref());
            cmd.args(args);
            if let Some(plan) = env_plan {
                for (k, v) in plan.assignments() {
                    cmd.env(k, v);
                }
            }
            cmd.env_remove("CODEX_MANAGED_BY_NPM")
                .env_remove("CODEX_MANAGED_BY_BUN")
                .env_remove("CODEX_MANAGED_PACKAGE_ROOT")
                .env_remove("LD_PRELOAD")
                .env_remove("LD_LIBRARY_PATH");

            Err(cmd.exec())
        })() {
            Err(err) => err,
            Ok(()) => unreachable!("exec never returns on success"),
        };

        let restore_34 = unsafe { guard_34.restore() };
        let restore_33 = unsafe { guard_33.restore() };
        if let Err(err) = restore_34 {
            return Err(err);
        }
        if let Err(err) = restore_33 {
            return Err(err);
        }
        Err(operation_err)
    })();

    match res {
        Err(err) => err,
        Ok(()) => unreachable!("exec never returns on success"),
    }
}

#[derive(Debug)]
enum LaunchError {
    Policy(PassthroughError),
    Exec(std::io::Error),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::Policy(err) => write!(f, "{err}"),
            LaunchError::Exec(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for LaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LaunchError::Policy(err) => Some(err),
            LaunchError::Exec(err) => Some(err),
        }
    }
}

impl From<PassthroughError> for LaunchError {
    fn from(err: PassthroughError) -> Self {
        LaunchError::Policy(err)
    }
}

impl From<std::io::Error> for LaunchError {
    fn from(err: std::io::Error) -> Self {
        LaunchError::Exec(err)
    }
}

#[cfg(unix)]
fn launch_upstream_impl<P, R, C, I, S>(
    program: P,
    resolver_path: R,
    config_dir: C,
    args: I,
    env_plan: Option<&TermuxBaseEnvPlan>,
) -> LaunchError
where
    P: AsRef<OsStr>,
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let planned_args = match plan_passthrough_args(args) {
        Ok(args) => args,
        Err(policy_err) => return LaunchError::Policy(policy_err),
    };

    let exec_err = exec_upstream_with_runtime_fds_and_env(
        program,
        &planned_args,
        resolver_path,
        config_dir,
        env_plan,
    );
    LaunchError::Exec(exec_err)
}

/// Launches the upstream program with planned sandbox-policy arguments and runtime file descriptors.
///
/// Inputs are explicit: the selected upstream program, resolver path, managed-config directory,
/// and original raw user argv.
///
/// Sandbox-policy planning is performed first. If the user arguments request an unsupported Linux
/// sandbox mode or subcommand, a `LaunchError::Policy` error is returned immediately before any
/// runtime file descriptor manipulation or program execution is attempted.
///
/// Once policy planning succeeds, delegates execution to `exec_upstream_with_runtime_fds`, which
/// maps the resolver file to FD 33 and the configuration directory to FD 34, fences contamination
/// environment variables, and replaces the current process image.
///
/// If process replacement succeeds, this function never returns.
/// If planning or execution fails, it returns the corresponding `LaunchError`.
#[cfg(unix)]
#[allow(dead_code)]
fn launch_upstream<P, R, C, I, S>(
    program: P,
    resolver_path: R,
    config_dir: C,
    args: I,
) -> LaunchError
where
    P: AsRef<OsStr>,
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    launch_upstream_impl(program, resolver_path, config_dir, args, None)
}

/// Launches the upstream program with planned sandbox-policy arguments, runtime file descriptors,
/// and an explicit base-environment plan.
///
/// Inputs are explicit: the selected upstream program, resolver path, managed-config directory,
/// original raw user argv, and pre-built environment plan.
///
/// Sandbox-policy planning is performed first. If the user arguments request an unsupported Linux
/// sandbox mode or subcommand, a `LaunchError::Policy` error is returned immediately before any
/// runtime file descriptor manipulation or program execution is attempted.
///
/// Once policy planning succeeds, applies the planned environment variables to the child Command,
/// maps the resolver file to FD 33 and the configuration directory to FD 34, fences contamination
/// environment variables, and replaces the current process image.
///
/// If process replacement succeeds, this function never returns.
/// If planning or execution fails, it returns the corresponding `LaunchError`.
#[cfg(unix)]
#[allow(dead_code)]
fn launch_upstream_with_env<P, R, C, I, S>(
    program: P,
    resolver_path: R,
    config_dir: C,
    args: I,
    env_plan: &TermuxBaseEnvPlan,
) -> LaunchError
where
    P: AsRef<OsStr>,
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    launch_upstream_impl(program, resolver_path, config_dir, args, Some(env_plan))
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum TermuxBaseEnvError {
    EmptyPathComponent(&'static str),
    ColonInPathComponent(&'static str),
    NulInPathComponent(&'static str),
}

#[cfg(unix)]
impl std::fmt::Display for TermuxBaseEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermuxBaseEnvError::EmptyPathComponent(name) => {
                write!(f, "explicit PATH component '{name}' must not be empty")
            }
            TermuxBaseEnvError::ColonInPathComponent(name) => {
                write!(f, "explicit PATH component '{name}' must not contain ':'")
            }
            TermuxBaseEnvError::NulInPathComponent(name) => {
                write!(
                    f,
                    "explicit PATH component '{name}' must not contain NUL byte"
                )
            }
        }
    }
}

#[cfg(unix)]
impl std::error::Error for TermuxBaseEnvError {}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxProcessEnvSnapshot {
    prefix: Option<OsString>,
    tmpdir: Option<OsString>,
    inherited_path: Option<OsString>,
    inherited_ssl_cert_file: Option<OsString>,
    inherited_ssl_cert_dir: Option<OsString>,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum TermuxProcessEnvError {
    MissingRequired(&'static str),
    EmptyRequired(&'static str),
    BaseEnv(TermuxBaseEnvError),
}

#[cfg(unix)]
impl std::fmt::Display for TermuxProcessEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermuxProcessEnvError::MissingRequired(name) => {
                write!(
                    f,
                    "required process environment variable '{name}' is missing"
                )
            }
            TermuxProcessEnvError::EmptyRequired(name) => {
                write!(f, "required process environment variable '{name}' is empty")
            }
            TermuxProcessEnvError::BaseEnv(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for TermuxProcessEnvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TermuxProcessEnvError::BaseEnv(err) => Some(err),
            TermuxProcessEnvError::MissingRequired(_) | TermuxProcessEnvError::EmptyRequired(_) => {
                None
            }
        }
    }
}

/// Captures only the ambient process values needed by the Termux base-environment planner.
#[cfg(unix)]
#[allow(dead_code)]
fn capture_termux_process_env() -> TermuxProcessEnvSnapshot {
    TermuxProcessEnvSnapshot {
        prefix: std::env::var_os("PREFIX"),
        tmpdir: std::env::var_os("TMPDIR"),
        inherited_path: std::env::var_os("PATH"),
        inherited_ssl_cert_file: std::env::var_os("SSL_CERT_FILE"),
        inherited_ssl_cert_dir: std::env::var_os("SSL_CERT_DIR"),
    }
}

#[cfg(unix)]
fn required_process_env<'a>(
    value: &'a Option<OsString>,
    name: &'static str,
) -> Result<&'a OsStr, TermuxProcessEnvError> {
    match value.as_deref() {
        None => Err(TermuxProcessEnvError::MissingRequired(name)),
        Some(value) if value.is_empty() => Err(TermuxProcessEnvError::EmptyRequired(name)),
        Some(value) => Ok(value),
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxBaseEnvInputs<'a> {
    compat_dir: &'a OsStr,
    prefix_bin_dir: &'a OsStr,
    temp_dir: &'a OsStr,
    cert_file: &'a OsStr,
    cert_dir: Option<&'a OsStr>,
    inherited_path: Option<&'a OsStr>,
    inherited_ssl_cert_file: Option<&'a OsStr>,
    inherited_ssl_cert_dir: Option<&'a OsStr>,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxBaseEnvPlan {
    assignments: Vec<(OsString, OsString)>,
}

#[cfg(unix)]
impl TermuxBaseEnvPlan {
    #[allow(dead_code)]
    fn assignments(&self) -> &[(OsString, OsString)] {
        &self.assignments
    }

    #[allow(dead_code)]
    fn get<K: AsRef<OsStr>>(&self, key: K) -> Option<&OsStr> {
        let key = key.as_ref();
        self.assignments
            .iter()
            .find(|(k, _)| k.as_os_str() == key)
            .map(|(_, v)| v.as_os_str())
    }

    #[allow(dead_code)]
    fn contains_key<K: AsRef<OsStr>>(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    #[allow(dead_code)]
    fn into_assignments(self) -> Vec<(OsString, OsString)> {
        self.assignments
    }
}

#[cfg(unix)]
fn validate_path_component(
    name: &'static str,
    component: &OsStr,
) -> Result<(), TermuxBaseEnvError> {
    use std::os::unix::ffi::OsStrExt;
    let bytes = component.as_bytes();
    if bytes.is_empty() {
        return Err(TermuxBaseEnvError::EmptyPathComponent(name));
    }
    if bytes.contains(&b':') {
        return Err(TermuxBaseEnvError::ColonInPathComponent(name));
    }
    if bytes.contains(&b'\0') {
        return Err(TermuxBaseEnvError::NulInPathComponent(name));
    }
    Ok(())
}

#[cfg(unix)]
fn build_planned_path(
    compat_dir: &OsStr,
    prefix_bin_dir: &OsStr,
    inherited_path: Option<&OsStr>,
) -> Result<OsString, TermuxBaseEnvError> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    validate_path_component("compat_dir", compat_dir)?;
    validate_path_component("prefix_bin_dir", prefix_bin_dir)?;

    let compat_bytes = compat_dir.as_bytes();
    let prefix_bytes = prefix_bin_dir.as_bytes();
    let inherited_bytes = inherited_path.map(|p| p.as_bytes()).unwrap_or(b"");

    let total_len = if inherited_bytes.is_empty() {
        compat_bytes.len() + 1 + prefix_bytes.len()
    } else {
        compat_bytes.len() + 1 + prefix_bytes.len() + 1 + inherited_bytes.len()
    };

    let mut path_bytes = Vec::with_capacity(total_len);
    path_bytes.extend_from_slice(compat_bytes);
    path_bytes.push(b':');
    path_bytes.extend_from_slice(prefix_bytes);

    if !inherited_bytes.is_empty() {
        path_bytes.push(b':');
        path_bytes.extend_from_slice(inherited_bytes);
    }

    Ok(OsString::from_vec(path_bytes))
}

/// Plans child environment variable assignments for Termux execution.
///
/// Receives every input explicitly and returns deterministic child-environment assignments.
/// Validates that explicit PATH components are non-empty and free of ':' and NUL delimiters.
/// Preserves raw inherited PATH byte-for-byte without re-normalization or lossy decoding.
#[cfg(unix)]
#[allow(dead_code)]
fn plan_termux_base_env(
    inputs: &TermuxBaseEnvInputs<'_>,
) -> Result<TermuxBaseEnvPlan, TermuxBaseEnvError> {
    let planned_path = build_planned_path(
        inputs.compat_dir,
        inputs.prefix_bin_dir,
        inputs.inherited_path,
    )?;

    let mut assignments = Vec::with_capacity(7);

    // 1. Temp directory assignments
    assignments.push((OsString::from("TMPDIR"), inputs.temp_dir.to_os_string()));
    assignments.push((OsString::from("TMP"), inputs.temp_dir.to_os_string()));
    assignments.push((OsString::from("TEMP"), inputs.temp_dir.to_os_string()));
    assignments.push((
        OsString::from("SQLITE_TMPDIR"),
        inputs.temp_dir.to_os_string(),
    ));

    // 2. SSL_CERT_FILE assignment: inherited non-empty wins; otherwise selected cert file.
    let ssl_cert_file = match inputs.inherited_ssl_cert_file {
        Some(inherited) if !inherited.is_empty() => inherited.to_os_string(),
        _ => inputs.cert_file.to_os_string(),
    };
    assignments.push((OsString::from("SSL_CERT_FILE"), ssl_cert_file));

    // 3. SSL_CERT_DIR assignment: inherited non-empty wins; otherwise selected cert dir if present.
    // If neither exists, omitted entirely.
    let ssl_cert_dir = match inputs.inherited_ssl_cert_dir {
        Some(inherited) if !inherited.is_empty() => Some(inherited.to_os_string()),
        _ => inputs
            .cert_dir
            .filter(|d| !d.is_empty())
            .map(|d| d.to_os_string()),
    };
    if let Some(dir_val) = ssl_cert_dir {
        assignments.push((OsString::from("SSL_CERT_DIR"), dir_val));
    }

    // 4. PATH assignment: compat_dir, prefix_bin_dir, then inherited non-empty PATH.
    assignments.push((OsString::from("PATH"), planned_path));

    Ok(TermuxBaseEnvPlan { assignments })
}

/// Converts one captured Termux process-environment snapshot into the pure B8 base-env plan.
///
/// The selected compatibility directory and certificate fallbacks remain explicit caller inputs.
/// This function derives only the prefix `bin` directory and performs no filesystem or runtime I/O.
#[cfg(unix)]
#[allow(dead_code)]
fn plan_termux_base_env_from_snapshot(
    snapshot: &TermuxProcessEnvSnapshot,
    compat_dir: &OsStr,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
) -> Result<TermuxBaseEnvPlan, TermuxProcessEnvError> {
    let prefix = required_process_env(&snapshot.prefix, "PREFIX")?;
    let temp_dir = required_process_env(&snapshot.tmpdir, "TMPDIR")?;
    let prefix_bin_dir = std::path::PathBuf::from(prefix).join("bin");

    let inputs = TermuxBaseEnvInputs {
        compat_dir,
        prefix_bin_dir: prefix_bin_dir.as_os_str(),
        temp_dir,
        cert_file,
        cert_dir,
        inherited_path: snapshot.inherited_path.as_deref(),
        inherited_ssl_cert_file: snapshot.inherited_ssl_cert_file.as_deref(),
        inherited_ssl_cert_dir: snapshot.inherited_ssl_cert_dir.as_deref(),
    };

    plan_termux_base_env(&inputs).map_err(TermuxProcessEnvError::BaseEnv)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerationQualification {
    Qualified,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenerationHelperDigest {
    identity: String,
    digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenerationManifest {
    upstream_package_identity: String,
    upstream_package_version: String,
    source_artifact_digest: String,
    expected_platform: String,
    expected_architecture: String,
    patch_policy_id: String,
    patch_report: String,
    runtime_digest: String,
    helper_digests: Vec<GenerationHelperDigest>,
    core_artifact_digest: String,
    manager_artifact_digest: Option<String>,
    core_api_identity: String,
    persistent_schema_identity: String,
    qualification: GenerationQualification,
    creation_metadata: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GenerationManifestRequirements<'a> {
    platform: &'a str,
    architecture: &'a str,
    core_api_identity: &'a str,
    persistent_schema_identity: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GenerationManifestError {
    EmptyRequirement(&'static str),
    EmptyRequired(&'static str),
    PlatformMismatch,
    ArchitectureMismatch,
    CoreApiMismatch,
    PersistentSchemaMismatch,
    RejectedQualification,
    EmptyHelperIdentity(usize),
    EmptyHelperDigest(usize),
    DuplicateHelperIdentity { first: usize, duplicate: usize },
    EmptyManagerDigest,
}

impl std::fmt::Display for GenerationManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerationManifestError::EmptyRequirement(name) => {
                write!(
                    f,
                    "generation manifest validation requirement '{name}' is empty"
                )
            }
            GenerationManifestError::EmptyRequired(name) => {
                write!(f, "required generation manifest binding '{name}' is empty")
            }
            GenerationManifestError::PlatformMismatch => {
                write!(
                    f,
                    "generation manifest platform does not match Core requirements"
                )
            }
            GenerationManifestError::ArchitectureMismatch => {
                write!(
                    f,
                    "generation manifest architecture does not match Core requirements"
                )
            }
            GenerationManifestError::CoreApiMismatch => {
                write!(f, "generation manifest Core API identity is incompatible")
            }
            GenerationManifestError::PersistentSchemaMismatch => {
                write!(
                    f,
                    "generation manifest persistent schema identity is incompatible"
                )
            }
            GenerationManifestError::RejectedQualification => {
                write!(
                    f,
                    "generation manifest qualification result is not successful"
                )
            }
            GenerationManifestError::EmptyHelperIdentity(index) => {
                write!(
                    f,
                    "generation manifest helper binding {index} has an empty identity"
                )
            }
            GenerationManifestError::EmptyHelperDigest(index) => {
                write!(
                    f,
                    "generation manifest helper binding {index} has an empty digest"
                )
            }
            GenerationManifestError::DuplicateHelperIdentity { first, duplicate } => write!(
                f,
                "generation manifest helper binding {duplicate} duplicates helper binding {first}"
            ),
            GenerationManifestError::EmptyManagerDigest => {
                write!(
                    f,
                    "generation manifest Manager digest is explicitly present but empty"
                )
            }
        }
    }
}

impl std::error::Error for GenerationManifestError {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct QualifiedGenerationManifest<'a> {
    manifest: &'a GenerationManifest,
}

#[allow(dead_code)]
impl<'a> QualifiedGenerationManifest<'a> {
    fn manifest(self) -> &'a GenerationManifest {
        self.manifest
    }
}

fn validate_non_empty_manifest_binding(
    value: &str,
    name: &'static str,
) -> Result<(), GenerationManifestError> {
    if value.is_empty() {
        Err(GenerationManifestError::EmptyRequired(name))
    } else {
        Ok(())
    }
}

fn validate_non_empty_manifest_requirement(
    value: &str,
    name: &'static str,
) -> Result<(), GenerationManifestError> {
    if value.is_empty() {
        Err(GenerationManifestError::EmptyRequirement(name))
    } else {
        Ok(())
    }
}

#[allow(dead_code)]
fn qualify_generation_manifest<'a>(
    manifest: &'a GenerationManifest,
    requirements: &GenerationManifestRequirements<'_>,
) -> Result<QualifiedGenerationManifest<'a>, GenerationManifestError> {
    validate_non_empty_manifest_requirement(requirements.platform, "platform")?;
    validate_non_empty_manifest_requirement(requirements.architecture, "architecture")?;
    validate_non_empty_manifest_requirement(requirements.core_api_identity, "core_api_identity")?;
    validate_non_empty_manifest_requirement(
        requirements.persistent_schema_identity,
        "persistent_schema_identity",
    )?;

    validate_non_empty_manifest_binding(
        &manifest.upstream_package_identity,
        "upstream_package_identity",
    )?;
    validate_non_empty_manifest_binding(
        &manifest.upstream_package_version,
        "upstream_package_version",
    )?;
    validate_non_empty_manifest_binding(
        &manifest.source_artifact_digest,
        "source_artifact_digest",
    )?;
    validate_non_empty_manifest_binding(&manifest.expected_platform, "expected_platform")?;
    validate_non_empty_manifest_binding(&manifest.expected_architecture, "expected_architecture")?;
    validate_non_empty_manifest_binding(&manifest.patch_policy_id, "patch_policy_id")?;
    validate_non_empty_manifest_binding(&manifest.patch_report, "patch_report")?;
    validate_non_empty_manifest_binding(&manifest.runtime_digest, "runtime_digest")?;
    validate_non_empty_manifest_binding(&manifest.core_artifact_digest, "core_artifact_digest")?;
    validate_non_empty_manifest_binding(&manifest.core_api_identity, "core_api_identity")?;
    validate_non_empty_manifest_binding(
        &manifest.persistent_schema_identity,
        "persistent_schema_identity",
    )?;
    validate_non_empty_manifest_binding(&manifest.creation_metadata, "creation_metadata")?;

    if manifest.expected_platform != requirements.platform {
        return Err(GenerationManifestError::PlatformMismatch);
    }
    if manifest.expected_architecture != requirements.architecture {
        return Err(GenerationManifestError::ArchitectureMismatch);
    }
    if manifest.core_api_identity != requirements.core_api_identity {
        return Err(GenerationManifestError::CoreApiMismatch);
    }
    if manifest.persistent_schema_identity != requirements.persistent_schema_identity {
        return Err(GenerationManifestError::PersistentSchemaMismatch);
    }
    if manifest.qualification != GenerationQualification::Qualified {
        return Err(GenerationManifestError::RejectedQualification);
    }

    if matches!(manifest.manager_artifact_digest.as_deref(), Some("")) {
        return Err(GenerationManifestError::EmptyManagerDigest);
    }

    for (index, helper) in manifest.helper_digests.iter().enumerate() {
        if helper.identity.is_empty() {
            return Err(GenerationManifestError::EmptyHelperIdentity(index));
        }
        if helper.digest.is_empty() {
            return Err(GenerationManifestError::EmptyHelperDigest(index));
        }
        for (first, previous) in manifest.helper_digests[..index].iter().enumerate() {
            if previous.identity == helper.identity {
                return Err(GenerationManifestError::DuplicateHelperIdentity {
                    first,
                    duplicate: index,
                });
            }
        }
    }

    Ok(QualifiedGenerationManifest { manifest })
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeAssetBinding<'a> {
    program_path: &'a OsStr,
    observed_digest: &'a str,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HelperAssetBinding<'a> {
    identity: &'a str,
    asset_path: &'a OsStr,
    observed_digest: &'a str,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeAssetSelection<'a> {
    runtime: RuntimeAssetBinding<'a>,
    compatibility_dir: &'a OsStr,
    helpers: &'a [HelperAssetBinding<'a>],
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeAssetError {
    EmptyPath(&'static str),
    RelativePath(&'static str),
    NulPath(&'static str),
    CompatibilityPath(TermuxBaseEnvError),
    EmptyRuntimeDigest,
    RuntimeDigestMismatch,
    EmptyHelperIdentity(usize),
    EmptyHelperDigest(usize),
    DuplicateHelperIdentity { first: usize, duplicate: usize },
    ExtraHelperIdentity(usize),
    MissingHelperIdentity(usize),
    HelperDigestMismatch(usize),
}

#[cfg(unix)]
impl std::fmt::Display for RuntimeAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeAssetError::EmptyPath(name) => write!(f, "runtime asset path '{name}' is empty"),
            RuntimeAssetError::RelativePath(name) => {
                write!(f, "runtime asset path '{name}' must be absolute")
            }
            RuntimeAssetError::NulPath(name) => {
                write!(f, "runtime asset path '{name}' must not contain NUL")
            }
            RuntimeAssetError::CompatibilityPath(err) => err.fmt(f),
            RuntimeAssetError::EmptyRuntimeDigest => {
                write!(f, "runtime asset observed digest is empty")
            }
            RuntimeAssetError::RuntimeDigestMismatch => {
                write!(
                    f,
                    "runtime asset digest does not match qualified generation"
                )
            }
            RuntimeAssetError::EmptyHelperIdentity(index) => {
                write!(f, "selected helper asset {index} has an empty identity")
            }
            RuntimeAssetError::EmptyHelperDigest(index) => {
                write!(
                    f,
                    "selected helper asset {index} has an empty observed digest"
                )
            }
            RuntimeAssetError::DuplicateHelperIdentity { first, duplicate } => write!(
                f,
                "selected helper asset {duplicate} duplicates selected helper asset {first}"
            ),
            RuntimeAssetError::ExtraHelperIdentity(index) => {
                write!(
                    f,
                    "selected helper asset {index} is not declared by the generation"
                )
            }
            RuntimeAssetError::MissingHelperIdentity(index) => {
                write!(f, "generation helper binding {index} has no selected asset")
            }
            RuntimeAssetError::HelperDigestMismatch(index) => {
                write!(
                    f,
                    "selected helper asset {index} digest does not match generation"
                )
            }
        }
    }
}

#[cfg(unix)]
impl std::error::Error for RuntimeAssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuntimeAssetError::CompatibilityPath(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct QualifiedRuntimeAssets<'selection, 'asset, 'generation> {
    generation: QualifiedGenerationManifest<'generation>,
    selection: &'selection RuntimeAssetSelection<'asset>,
}

#[cfg(unix)]
#[allow(dead_code)]
impl<'selection, 'asset, 'generation> QualifiedRuntimeAssets<'selection, 'asset, 'generation> {
    fn generation(self) -> QualifiedGenerationManifest<'generation> {
        self.generation
    }

    fn selection(self) -> &'selection RuntimeAssetSelection<'asset> {
        self.selection
    }
}

#[cfg(unix)]
fn validate_absolute_runtime_asset_path(
    path: &OsStr,
    name: &'static str,
) -> Result<(), RuntimeAssetError> {
    use std::os::unix::ffi::OsStrExt;

    if path.is_empty() {
        return Err(RuntimeAssetError::EmptyPath(name));
    }
    if path.as_bytes().contains(&0) {
        return Err(RuntimeAssetError::NulPath(name));
    }
    if !std::path::Path::new(path).is_absolute() {
        return Err(RuntimeAssetError::RelativePath(name));
    }
    Ok(())
}

#[cfg(unix)]
#[allow(dead_code)]
fn qualify_runtime_assets<'selection, 'asset, 'generation>(
    generation: QualifiedGenerationManifest<'generation>,
    selection: &'selection RuntimeAssetSelection<'asset>,
) -> Result<QualifiedRuntimeAssets<'selection, 'asset, 'generation>, RuntimeAssetError> {
    validate_absolute_runtime_asset_path(selection.runtime.program_path, "runtime_program")?;
    validate_absolute_runtime_asset_path(selection.compatibility_dir, "compatibility_dir")?;
    validate_path_component("compat_dir", selection.compatibility_dir)
        .map_err(RuntimeAssetError::CompatibilityPath)?;

    if selection.runtime.observed_digest.is_empty() {
        return Err(RuntimeAssetError::EmptyRuntimeDigest);
    }
    if selection.runtime.observed_digest != generation.manifest().runtime_digest {
        return Err(RuntimeAssetError::RuntimeDigestMismatch);
    }

    for (index, helper) in selection.helpers.iter().enumerate() {
        if helper.identity.is_empty() {
            return Err(RuntimeAssetError::EmptyHelperIdentity(index));
        }
        validate_absolute_runtime_asset_path(helper.asset_path, "helper_asset")?;
        if helper.observed_digest.is_empty() {
            return Err(RuntimeAssetError::EmptyHelperDigest(index));
        }
        for (first, previous) in selection.helpers[..index].iter().enumerate() {
            if previous.identity == helper.identity {
                return Err(RuntimeAssetError::DuplicateHelperIdentity {
                    first,
                    duplicate: index,
                });
            }
        }

        let Some(manifest_helper) = generation
            .manifest()
            .helper_digests
            .iter()
            .find(|declared| declared.identity == helper.identity)
        else {
            return Err(RuntimeAssetError::ExtraHelperIdentity(index));
        };
        if manifest_helper.digest != helper.observed_digest {
            return Err(RuntimeAssetError::HelperDigestMismatch(index));
        }
    }

    for (manifest_index, declared) in generation.manifest().helper_digests.iter().enumerate() {
        if !selection
            .helpers
            .iter()
            .any(|selected| selected.identity == declared.identity)
        {
            return Err(RuntimeAssetError::MissingHelperIdentity(manifest_index));
        }
    }

    Ok(QualifiedRuntimeAssets {
        generation,
        selection,
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateEvidenceVerdict {
    Satisfied,
    Rejected,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateArtifactSource<'a> {
    Remote { immutable_locator: &'a str },
    LocalArtifact { path: &'a OsStr },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdaterResolverDependency<'a> {
    Independent,
    SharedRuntimeResolver { qualification_identity: &'a str },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UpdateAdmissionEvidence<'a> {
    signed_release_manifest_identity: &'a str,
    expected_source_artifact_digest: &'a str,
    release_signature: UpdateEvidenceVerdict,
    architecture_policy: UpdateEvidenceVerdict,
    core_api_policy: UpdateEvidenceVerdict,
    channel_policy: UpdateEvidenceVerdict,
    anti_rollback_policy: UpdateEvidenceVerdict,
    resolver_dependency: UpdaterResolverDependency<'a>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UpdateRequest<'a> {
    source: UpdateArtifactSource<'a>,
    evidence: UpdateAdmissionEvidence<'a>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StagedArtifactEvidence {
    artifact_digest: UpdateEvidenceVerdict,
    archive_safety: UpdateEvidenceVerdict,
    compatibility_metadata: UpdateEvidenceVerdict,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CandidateReadinessEvidence {
    candidate_probe: UpdateEvidenceVerdict,
    rollback_state_ready: UpdateEvidenceVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UpdateInterfaceError {
    EmptySignedReleaseManifestIdentity,
    EmptyExpectedSourceArtifactDigest,
    EmptyRemoteLocator,
    EmptyLocalArtifactPath,
    ReleaseSignatureRejected,
    ArchitecturePolicyRejected,
    CoreApiPolicyRejected,
    ChannelPolicyRejected,
    AntiRollbackPolicyRejected,
    SharedResolverMissingQualification,
    ArtifactDigestRejected,
    ArchiveSafetyRejected,
    CompatibilityMetadataRejected,
    SourceArtifactDigestMismatch,
    CandidateProbeRejected,
    RollbackStateNotReady,
}

impl std::fmt::Display for UpdateInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            UpdateInterfaceError::EmptySignedReleaseManifestIdentity => {
                "signed release manifest identity is empty"
            }
            UpdateInterfaceError::EmptyExpectedSourceArtifactDigest => {
                "expected immutable source artifact digest is empty"
            }
            UpdateInterfaceError::EmptyRemoteLocator => {
                "immutable remote artifact locator is empty"
            }
            UpdateInterfaceError::EmptyLocalArtifactPath => "explicit local artifact path is empty",
            UpdateInterfaceError::ReleaseSignatureRejected => {
                "release signature evidence was rejected"
            }
            UpdateInterfaceError::ArchitecturePolicyRejected => {
                "architecture policy evidence was rejected"
            }
            UpdateInterfaceError::CoreApiPolicyRejected => "Core API policy evidence was rejected",
            UpdateInterfaceError::ChannelPolicyRejected => "channel policy evidence was rejected",
            UpdateInterfaceError::AntiRollbackPolicyRejected => {
                "anti-rollback policy evidence was rejected"
            }
            UpdateInterfaceError::SharedResolverMissingQualification => {
                "shared updater/runtime resolver lacks explicit qualification"
            }
            UpdateInterfaceError::ArtifactDigestRejected => "artifact digest evidence was rejected",
            UpdateInterfaceError::ArchiveSafetyRejected => "archive safety evidence was rejected",
            UpdateInterfaceError::CompatibilityMetadataRejected => {
                "compatibility metadata evidence was rejected"
            }
            UpdateInterfaceError::SourceArtifactDigestMismatch => {
                "qualified generation source digest does not match admitted release"
            }
            UpdateInterfaceError::CandidateProbeRejected => {
                "candidate generation probe was rejected"
            }
            UpdateInterfaceError::RollbackStateNotReady => "verified rollback state is not ready",
        };
        f.write_str(message)
    }
}

impl std::error::Error for UpdateInterfaceError {}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct AdmittedUpdateRequest<'request, 'value> {
    request: &'request UpdateRequest<'value>,
}

#[allow(dead_code)]
impl<'request, 'value> AdmittedUpdateRequest<'request, 'value> {
    fn request(self) -> &'request UpdateRequest<'value> {
        self.request
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct ActivationReadyUpdate<'request, 'value, 'generation> {
    admitted: AdmittedUpdateRequest<'request, 'value>,
    generation: QualifiedGenerationManifest<'generation>,
}

#[allow(dead_code)]
impl<'request, 'value, 'generation> ActivationReadyUpdate<'request, 'value, 'generation> {
    fn admitted(self) -> AdmittedUpdateRequest<'request, 'value> {
        self.admitted
    }

    fn generation(self) -> QualifiedGenerationManifest<'generation> {
        self.generation
    }
}

fn require_update_evidence(
    verdict: UpdateEvidenceVerdict,
    error: UpdateInterfaceError,
) -> Result<(), UpdateInterfaceError> {
    match verdict {
        UpdateEvidenceVerdict::Satisfied => Ok(()),
        UpdateEvidenceVerdict::Rejected => Err(error),
    }
}

#[allow(dead_code)]
fn admit_update_request<'request, 'value>(
    request: &'request UpdateRequest<'value>,
) -> Result<AdmittedUpdateRequest<'request, 'value>, UpdateInterfaceError> {
    if request.evidence.signed_release_manifest_identity.is_empty() {
        return Err(UpdateInterfaceError::EmptySignedReleaseManifestIdentity);
    }
    if request.evidence.expected_source_artifact_digest.is_empty() {
        return Err(UpdateInterfaceError::EmptyExpectedSourceArtifactDigest);
    }
    match request.source {
        UpdateArtifactSource::Remote { immutable_locator } if immutable_locator.is_empty() => {
            return Err(UpdateInterfaceError::EmptyRemoteLocator);
        }
        UpdateArtifactSource::LocalArtifact { path } if path.is_empty() => {
            return Err(UpdateInterfaceError::EmptyLocalArtifactPath);
        }
        _ => {}
    }

    require_update_evidence(
        request.evidence.release_signature,
        UpdateInterfaceError::ReleaseSignatureRejected,
    )?;
    require_update_evidence(
        request.evidence.architecture_policy,
        UpdateInterfaceError::ArchitecturePolicyRejected,
    )?;
    require_update_evidence(
        request.evidence.core_api_policy,
        UpdateInterfaceError::CoreApiPolicyRejected,
    )?;
    require_update_evidence(
        request.evidence.channel_policy,
        UpdateInterfaceError::ChannelPolicyRejected,
    )?;
    require_update_evidence(
        request.evidence.anti_rollback_policy,
        UpdateInterfaceError::AntiRollbackPolicyRejected,
    )?;

    if let UpdaterResolverDependency::SharedRuntimeResolver {
        qualification_identity,
    } = request.evidence.resolver_dependency
    {
        if qualification_identity.is_empty() {
            return Err(UpdateInterfaceError::SharedResolverMissingQualification);
        }
    }

    Ok(AdmittedUpdateRequest { request })
}

#[allow(dead_code)]
fn qualify_update_candidate<'request, 'value, 'generation>(
    admitted: AdmittedUpdateRequest<'request, 'value>,
    staged: &StagedArtifactEvidence,
    generation: QualifiedGenerationManifest<'generation>,
    readiness: &CandidateReadinessEvidence,
) -> Result<ActivationReadyUpdate<'request, 'value, 'generation>, UpdateInterfaceError> {
    require_update_evidence(
        staged.artifact_digest,
        UpdateInterfaceError::ArtifactDigestRejected,
    )?;
    require_update_evidence(
        staged.archive_safety,
        UpdateInterfaceError::ArchiveSafetyRejected,
    )?;
    require_update_evidence(
        staged.compatibility_metadata,
        UpdateInterfaceError::CompatibilityMetadataRejected,
    )?;

    if generation.manifest().source_artifact_digest
        != admitted.request().evidence.expected_source_artifact_digest
    {
        return Err(UpdateInterfaceError::SourceArtifactDigestMismatch);
    }

    require_update_evidence(
        readiness.candidate_probe,
        UpdateInterfaceError::CandidateProbeRejected,
    )?;
    require_update_evidence(
        readiness.rollback_state_ready,
        UpdateInterfaceError::RollbackStateNotReady,
    )?;

    Ok(ActivationReadyUpdate {
        admitted,
        generation,
    })
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
    const PROBE_FAKE_UPSTREAM_PATH_ENV: &str = "CODEX_TEST_EXEC_FAKE_UPSTREAM_PATH";

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
            "m1_r1_nonexistent_resolver" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                unsafe {
                    close(33);
                    close(34);
                }
                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("sh"),
                    &[OsStr::new("-c"), OsStr::new("exit 0")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });
                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_R1_NONEXISTENT_RESOLVER_OK\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "m1_r1_nonexistent_config" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                unsafe {
                    close(33);
                    close(34);
                }
                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("sh"),
                    &[OsStr::new("-c"), OsStr::new("exit 0")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });
                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_R1_NONEXISTENT_CONFIG_OK\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "m1_r1_config_is_file" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                unsafe {
                    close(33);
                    close(34);
                }
                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("sh"),
                    &[OsStr::new("-c"), OsStr::new("exit 0")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.raw_os_error(), Some(20));
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });
                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_R1_CONFIG_IS_FILE_OK\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "m1_r1_resolver_is_dir" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                unsafe {
                    close(33);
                    close(34);
                }
                let err = exec_upstream_with_runtime_fds(
                    OsStr::new("sh"),
                    &[OsStr::new("-c"), OsStr::new("exit 0")],
                    resolver_path,
                    config_dir_path,
                );
                assert_eq!(err.raw_os_error(), Some(21));
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });
                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_R1_RESOLVER_IS_DIR_OK\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
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
            "tty_evidence" => {
                let script = r#"
if test -t 0; then
    printf "UPSTREAM_TTY_STDIN:1\n"
else
    printf "UPSTREAM_TTY_STDIN:0\n"
fi
if test -t 1; then
    printf "UPSTREAM_TTY_STDOUT:1\n"
else
    printf "UPSTREAM_TTY_STDOUT:0\n"
fi
if test -t 2; then
    printf "UPSTREAM_TTY_STDERR:1\n"
else
    printf "UPSTREAM_TTY_STDERR:0\n"
fi
printf "UPSTREAM_TTY_SUCCESS\n"
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
            "external_sigterm_evidence" => {
                let script = r#"
trap 'exit 73' TERM
printf "READY:PID:%d\n" "$$"
while true; do
    sleep 2 &
    wait $!
done
"#;
                let args: Vec<&OsStr> = vec![
                    OsStr::new("-c"),
                    OsStr::new(script),
                    OsStr::new("upstream-probe"),
                ];

                let err = exec_upstream(shell, args);
                panic!("exec_upstream failed to replace process: {err}");
            }
            "m1_b7_fake_upstream_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                let fake_upstream_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");

                std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-npm-contam-val");
                std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-bun-contam-val");
                std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/fake/pkg/root");
                std::env::set_var("LD_PRELOAD", "/probe/fake/preload.so");
                std::env::set_var("LD_LIBRARY_PATH", "/probe/fake/lib");
                std::env::set_var(
                    "CODEX_TEST_UNRELATED_M1_B7_SURVIVING_VAR",
                    "m1_b7_surviving_exact_value_84920",
                );

                use std::os::unix::ffi::OsStrExt;
                let non_utf8_arg = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
                let user_args: Vec<OsString> = vec![
                    OsString::from("exec"),
                    OsString::from("custom_task"),
                    OsString::from("--custom-flag=val1"),
                    OsString::from("ordinary arg with spaces and ="),
                    non_utf8_arg.to_os_string(),
                ];

                let err = launch_upstream(
                    fake_upstream_path,
                    resolver_path,
                    config_dir_path,
                    user_args,
                );
                panic!("launch_upstream failed to replace process: {err}");
            }
            "m1_b7_failed_exec_restoration_absent" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                unsafe {
                    close(33);
                    close(34);
                }
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                let err = launch_upstream(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b7-nonexistent-bin"),
                    resolver_path,
                    config_dir_path,
                    &[OsStr::new("--version")],
                );
                match err {
                    LaunchError::Exec(io_err) => {
                        assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
                    }
                    LaunchError::Policy(p_err) => {
                        panic!("unexpected policy error on valid args: {p_err}");
                    }
                }

                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_B7_RESTORED_ABSENT_SUCCESS\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "m1_b7_failed_exec_restoration_sentinels" => {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::io::AsRawFd;
                use std::os::unix::io::FromRawFd;

                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let temp_dir = std::env::temp_dir();
                let pid = std::process::id();
                let sentinel33_path = temp_dir.join(format!("codex-m1-b7-sentinel-33-{pid}.tmp"));
                let sentinel34_path = temp_dir.join(format!("codex-m1-b7-sentinel-34-{pid}.tmp"));

                let s33_bytes = b"ORIGINAL_M1_B7_SENTINEL_33_DATA";
                let s34_bytes = b"ORIGINAL_M1_B7_SENTINEL_34_DATA";
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

                let err = launch_upstream(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b7-nonexistent-bin"),
                    resolver_path,
                    config_dir_path,
                    &[OsStr::new("--version")],
                );
                match err {
                    LaunchError::Exec(io_err) => {
                        assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
                    }
                    LaunchError::Policy(p_err) => {
                        panic!("unexpected policy error on valid args: {p_err}");
                    }
                }

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
                    file.write_all(b"_APPENDED")
                        .expect("write to restored fd 33");
                    std::mem::forget(file);
                }
                assert_eq!(&buf33[..s33_bytes.len()], s33_bytes);

                let restored_meta34 = std::fs::metadata("/proc/self/fd/34").expect("stat fd 34");
                assert_eq!(restored_meta34.dev(), meta34.dev());
                assert_eq!(restored_meta34.ino(), meta34.ino());

                let _ = std::fs::remove_file(&sentinel33_path);
                let _ = std::fs::remove_file(&sentinel34_path);

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_B7_RESTORED_SENTINELS_SUCCESS\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "m1_b9_fake_upstream_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                let fake_upstream_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");

                std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-npm-contam-val");
                std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-bun-contam-val");
                std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/fake/pkg/root");
                std::env::set_var("LD_PRELOAD", "/probe/fake/preload.so");
                std::env::set_var("LD_LIBRARY_PATH", "/probe/fake/lib");
                std::env::set_var(
                    "CODEX_TEST_UNRELATED_M1_B9_SURVIVING_VAR",
                    "m1_b9_surviving_exact_value_77192",
                );

                let inputs = TermuxBaseEnvInputs {
                    compat_dir: OsStr::new("/probe/synthetic/compat/bin"),
                    prefix_bin_dir: OsStr::new("/probe/synthetic/prefix/bin"),
                    temp_dir: OsStr::new("/probe/synthetic/isolated/tmp"),
                    cert_file: OsStr::new("/probe/synthetic/tls/cert.pem"),
                    cert_dir: Some(OsStr::new("/probe/synthetic/tls/certs.d")),
                    inherited_path: Some(OsStr::new("/probe/inherited/bin1:/probe/inherited/bin2")),
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };
                let plan = plan_termux_base_env(&inputs).expect("plan base env");

                use std::os::unix::ffi::OsStrExt;
                let non_utf8_arg = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
                let user_args: Vec<OsString> = vec![
                    OsString::from("exec"),
                    OsString::from("custom_task"),
                    OsString::from("--custom-flag=val1"),
                    OsString::from("ordinary arg with spaces and ="),
                    non_utf8_arg.to_os_string(),
                ];

                let err = launch_upstream_with_env(
                    fake_upstream_path,
                    resolver_path,
                    config_dir_path,
                    user_args,
                    &plan,
                );
                panic!("launch_upstream_with_env failed to replace process: {err}");
            }
            "m1_b9_failed_exec_launcher" => {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::io::AsRawFd;
                use std::os::unix::io::FromRawFd;

                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let parent_dir = std::path::Path::new(&resolver_path)
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("/tmp"));
                let pid = std::process::id();
                let sentinel33_path = parent_dir.join(format!("codex-m1-b9-sentinel-33-{pid}.tmp"));
                let sentinel34_path = parent_dir.join(format!("codex-m1-b9-sentinel-34-{pid}.tmp"));

                let s33_bytes = b"ORIGINAL_M1_B9_SENTINEL_33_DATA";
                let s34_bytes = b"ORIGINAL_M1_B9_SENTINEL_34_DATA";
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

                let orig_tmpdir = OsString::from("/parent/orig/tmpdir");
                let orig_tmp = OsString::from("/parent/orig/tmp");
                let orig_temp = OsString::from("/parent/orig/temp");
                let orig_sqlite = OsString::from("/parent/orig/sqlite_tmpdir");
                let orig_cert_file = OsString::from("/parent/orig/cert.pem");
                let orig_cert_dir = OsString::from("/parent/orig/certs.d");
                let orig_path = OsString::from("/parent/orig/bin:/parent/orig/usr/bin");

                std::env::set_var("TMPDIR", &orig_tmpdir);
                std::env::set_var("TMP", &orig_tmp);
                std::env::set_var("TEMP", &orig_temp);
                std::env::set_var("SQLITE_TMPDIR", &orig_sqlite);
                std::env::set_var("SSL_CERT_FILE", &orig_cert_file);
                std::env::set_var("SSL_CERT_DIR", &orig_cert_dir);
                std::env::set_var("PATH", &orig_path);

                let inputs = TermuxBaseEnvInputs {
                    compat_dir: OsStr::new("/plan/compat"),
                    prefix_bin_dir: OsStr::new("/plan/prefix/bin"),
                    temp_dir: OsStr::new("/plan/isolated/tmp"),
                    cert_file: OsStr::new("/plan/tls/cert.pem"),
                    cert_dir: Some(OsStr::new("/plan/tls/certs.d")),
                    inherited_path: Some(OsStr::new("/plan/inherited/bin")),
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };
                let plan = plan_termux_base_env(&inputs).expect("plan");

                let err = launch_upstream_with_env(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b9-nonexistent-bin"),
                    resolver_path,
                    config_dir_path,
                    &[OsStr::new("--version")],
                    &plan,
                );
                match err {
                    LaunchError::Exec(io_err) => {
                        assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
                    }
                    LaunchError::Policy(p_err) => {
                        panic!("unexpected policy error on valid args: {p_err}");
                    }
                }

                assert_eq!(std::env::var_os("TMPDIR").as_ref(), Some(&orig_tmpdir));
                assert_eq!(std::env::var_os("TMP").as_ref(), Some(&orig_tmp));
                assert_eq!(std::env::var_os("TEMP").as_ref(), Some(&orig_temp));
                assert_eq!(
                    std::env::var_os("SQLITE_TMPDIR").as_ref(),
                    Some(&orig_sqlite)
                );
                assert_eq!(
                    std::env::var_os("SSL_CERT_FILE").as_ref(),
                    Some(&orig_cert_file)
                );
                assert_eq!(
                    std::env::var_os("SSL_CERT_DIR").as_ref(),
                    Some(&orig_cert_dir)
                );
                assert_eq!(std::env::var_os("PATH").as_ref(), Some(&orig_path));

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
                    file.write_all(b"_APPENDED")
                        .expect("write to restored fd 33");
                    std::mem::forget(file);
                }
                assert_eq!(&buf33[..s33_bytes.len()], s33_bytes);

                let restored_meta34 = std::fs::metadata("/proc/self/fd/34").expect("stat fd 34");
                assert_eq!(restored_meta34.dev(), meta34.dev());
                assert_eq!(restored_meta34.ino(), meta34.ino());

                let _ = std::fs::remove_file(&sentinel33_path);
                let _ = std::fs::remove_file(&sentinel34_path);

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_B9_FAILED_EXEC_PRESERVED_SUCCESS\n");
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
            "m1_b9_failed_exec_absent_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");

                let orig_tmpdir = OsString::from("/parent/orig/tmpdir2");
                let orig_tmp = OsString::from("/parent/orig/tmp2");
                let orig_temp = OsString::from("/parent/orig/temp2");
                let orig_sqlite = OsString::from("/parent/orig/sqlite_tmpdir2");
                let orig_cert_file = OsString::from("/parent/orig/cert2.pem");
                let orig_cert_dir = OsString::from("/parent/orig/certs2.d");
                let orig_path = OsString::from("/parent/orig/bin2:/parent/orig/usr/bin2");

                std::env::set_var("TMPDIR", &orig_tmpdir);
                std::env::set_var("TMP", &orig_tmp);
                std::env::set_var("TEMP", &orig_temp);
                std::env::set_var("SQLITE_TMPDIR", &orig_sqlite);
                std::env::set_var("SSL_CERT_FILE", &orig_cert_file);
                std::env::set_var("SSL_CERT_DIR", &orig_cert_dir);
                std::env::set_var("PATH", &orig_path);

                unsafe {
                    close(33);
                    close(34);
                }
                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                let inputs = TermuxBaseEnvInputs {
                    compat_dir: OsStr::new("/plan/compat"),
                    prefix_bin_dir: OsStr::new("/plan/prefix/bin"),
                    temp_dir: OsStr::new("/plan/isolated/tmp"),
                    cert_file: OsStr::new("/plan/tls/cert.pem"),
                    cert_dir: None,
                    inherited_path: None,
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };
                let plan = plan_termux_base_env(&inputs).expect("plan");

                let err = launch_upstream_with_env(
                    OsStr::new("/path/that/does/not/exist/codex-m1-b9-nonexistent-bin"),
                    resolver_path,
                    config_dir_path,
                    &[OsStr::new("--version")],
                    &plan,
                );
                match err {
                    LaunchError::Exec(io_err) => {
                        assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
                    }
                    LaunchError::Policy(p_err) => {
                        panic!("unexpected policy error on valid args: {p_err}");
                    }
                }

                assert_eq!(std::env::var_os("TMPDIR").as_ref(), Some(&orig_tmpdir));
                assert_eq!(std::env::var_os("TMP").as_ref(), Some(&orig_tmp));
                assert_eq!(std::env::var_os("TEMP").as_ref(), Some(&orig_temp));
                assert_eq!(
                    std::env::var_os("SQLITE_TMPDIR").as_ref(),
                    Some(&orig_sqlite)
                );
                assert_eq!(
                    std::env::var_os("SSL_CERT_FILE").as_ref(),
                    Some(&orig_cert_file)
                );
                assert_eq!(
                    std::env::var_os("SSL_CERT_DIR").as_ref(),
                    Some(&orig_cert_dir)
                );
                assert_eq!(std::env::var_os("PATH").as_ref(), Some(&orig_path));

                assert!(unsafe { fcntl(33, F_GETFD) < 0 });
                assert!(unsafe { fcntl(34, F_GETFD) < 0 });

                use std::io::Write;
                let _ = std::io::stdout().write_all(b"M1_B9_FAILED_EXEC_ABSENT_SUCCESS\n");
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
        let test_root = temp_dir.join(format!("codex-test-m1-r1-no-res-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("create test root");
        let resolver = test_root.join("missing-resolv.conf");
        let config = test_root.join("managed-config");
        std::fs::create_dir_all(&config).expect("create config dir");
        let result = run_exec_probe_with_env(
            "m1_r1_nonexistent_resolver",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"M1_R1_NONEXISTENT_RESOLVER_OK\n");
        assert_eq!(result.stderr, b"");
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_nonexistent_config_dir_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-r1-no-cfg-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("create test root");
        let resolver = test_root.join("resolv.conf");
        std::fs::write(&resolver, b"nameserver 127.0.0.1\n").expect("write resolver");
        let config = test_root.join("missing-config");
        let result = run_exec_probe_with_env(
            "m1_r1_nonexistent_config",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"M1_R1_NONEXISTENT_CONFIG_OK\n");
        assert_eq!(result.stderr, b"");
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_config_dir_is_file_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-r1-cfg-file-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("create test root");
        let resolver = test_root.join("resolv.conf");
        let config = test_root.join("config-file");
        std::fs::write(&resolver, b"nameserver 127.0.0.1\n").expect("write resolver");
        std::fs::write(&config, b"not a directory").expect("write config file");
        let result = run_exec_probe_with_env(
            "m1_r1_config_is_file",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"M1_R1_CONFIG_IS_FILE_OK\n");
        assert_eq!(result.stderr, b"");
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_with_runtime_fds_resolver_is_dir_fails_and_restores() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-r1-res-dir-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("create test root");
        let resolver = test_root.join("resolver-dir");
        let config = test_root.join("managed-config");
        std::fs::create_dir_all(&resolver).expect("create resolver dir");
        std::fs::create_dir_all(&config).expect("create config dir");
        let result = run_exec_probe_with_env(
            "m1_r1_resolver_is_dir",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"M1_R1_RESOLVER_IS_DIR_OK\n");
        assert_eq!(result.stderr, b"");
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

    #[cfg(unix)]
    extern "C" {
        fn kill(pid: std::os::raw::c_int, sig: std::os::raw::c_int) -> std::os::raw::c_int;
    }

    #[cfg(unix)]
    struct ChildCleanupGuard(Option<std::process::Child>);

    #[cfg(unix)]
    impl Drop for ChildCleanupGuard {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    #[cfg(target_os = "android")]
    #[test]
    fn test_exec_upstream_preserves_tty_attachment_on_android() {
        let current_exe = std::env::current_exe().expect("failed to get current_exe");
        let shell = resolve_test_shell();

        let script_check = std::process::Command::new("script")
            .arg("--version")
            .output();
        if script_check.is_err() {
            panic!("util-linux script utility is required for Android PTY test");
        }

        let probe_cmd = format!(
            "\"{}\" tests::exec_probe_subprocess_entry --exact",
            current_exe.display()
        );

        let mut cmd = std::process::Command::new("script");
        cmd.arg("-q")
            .arg("-e")
            .arg("-c")
            .arg(probe_cmd)
            .arg("/dev/null")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .env(PROBE_ROLE_ENV, PROBE_ROLE_LAUNCHER)
            .env(PROBE_SHELL_ENV, &shell)
            .env(PROBE_SCENARIO_ENV, "tty_evidence");

        let output = cmd.output().expect("failed to execute script probe");

        assert!(
            output.status.success(),
            "script probe process failed with status {:?}, stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout_str
            .lines()
            .map(|l| l.trim_end_matches('\r').trim())
            .filter(|l| !l.is_empty())
            .collect();

        assert!(
            lines.contains(&"UPSTREAM_TTY_STDIN:1"),
            "expected UPSTREAM_TTY_STDIN:1 in output: {:?}",
            lines
        );
        assert!(
            lines.contains(&"UPSTREAM_TTY_STDOUT:1"),
            "expected UPSTREAM_TTY_STDOUT:1 in output: {:?}",
            lines
        );
        assert!(
            lines.contains(&"UPSTREAM_TTY_STDERR:1"),
            "expected UPSTREAM_TTY_STDERR:1 in output: {:?}",
            lines
        );
        assert!(
            lines.contains(&"UPSTREAM_TTY_SUCCESS"),
            "expected UPSTREAM_TTY_SUCCESS in output: {:?}",
            lines
        );

        assert!(
            !lines.contains(&"UPSTREAM_TTY_STDIN:0"),
            "stdin was not attached to TTY"
        );
        assert!(
            !lines.contains(&"UPSTREAM_TTY_STDOUT:0"),
            "stdout was not attached to TTY"
        );
        assert!(
            !lines.contains(&"UPSTREAM_TTY_STDERR:0"),
            "stderr was not attached to TTY"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_exec_upstream_preserves_process_identity_and_signal_delivery() {
        let current_exe = std::env::current_exe().expect("failed to get current_exe");
        let shell = resolve_test_shell();

        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("tests::exec_probe_subprocess_entry")
            .arg("--exact")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .env(PROBE_ROLE_ENV, PROBE_ROLE_LAUNCHER)
            .env(PROBE_SHELL_ENV, &shell)
            .env(PROBE_SCENARIO_ENV, "external_sigterm_evidence");

        let mut child = cmd.spawn().expect("failed to spawn probe child");
        let child_pid = child.id();

        let stdout = child.stdout.take().expect("stdout must be piped");
        let mut guard = ChildCleanupGuard(Some(child));

        let (tx, rx) = std::sync::mpsc::channel();
        let reader_handle = std::thread::spawn(move || {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(stdout);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim_end_matches(&['\r', '\n'][..]).trim();
                if trimmed.starts_with("READY:PID:") {
                    let _ = tx.send(Ok(trimmed.to_string()));
                    return;
                }
                line.clear();
            }
            let _ = tx.send(Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "readiness marker not found before EOF",
            )));
        });

        let ready_timeout = std::time::Duration::from_secs(5);
        let ready_line = match rx.recv_timeout(ready_timeout) {
            Ok(Ok(line)) => line,
            Ok(Err(err)) => panic!("I/O error reading readiness line from probe: {err}"),
            Err(err) => panic!("timeout waiting for upstream probe readiness: {err}"),
        };

        let reported_pid_str = ready_line
            .strip_prefix("READY:PID:")
            .unwrap_or_else(|| panic!("unexpected readiness line: {:?}", ready_line));
        let reported_pid: u32 = reported_pid_str
            .parse()
            .unwrap_or_else(|e| panic!("failed to parse pid from {:?}: {e}", reported_pid_str));

        assert_eq!(
            reported_pid, child_pid,
            "upstream process identity ($$) must equal spawned child PID"
        );

        const SIGTERM: std::os::raw::c_int = 15;
        let kill_ret = unsafe { kill(reported_pid as std::os::raw::c_int, SIGTERM) };
        assert_eq!(
            kill_ret, 0,
            "failed to deliver SIGTERM to process {}",
            reported_pid
        );

        let child_ref = guard.0.as_mut().expect("child must be present");
        let wait_timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        let status = loop {
            match child_ref.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if start.elapsed() > wait_timeout {
                        panic!("timed out waiting for child to exit after SIGTERM");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(err) => panic!("error while waiting for child: {err}"),
            }
        };

        assert_eq!(
            status.code(),
            Some(73),
            "expected upstream trap exit code 73, got {:?}",
            status
        );

        guard.0 = None;
        let _ = reader_handle.join();
    }

    #[test]
    fn test_passthrough_rejects_unsupported_sandbox_modes_all_syntaxes() {
        let modes = ["read-only", "workspace-write"];
        for mode in modes {
            let test_cases: Vec<Vec<String>> = vec![
                vec!["-s".into(), mode.into()],
                vec!["--sandbox".into(), mode.into()],
                vec![format!("--sandbox={mode}")],
                vec![format!("-s{mode}")],
                vec!["-c".into(), format!("sandbox_mode={mode}")],
                vec!["--config".into(), format!("sandbox_mode={mode}")],
                vec![format!("--config=sandbox_mode={mode}")],
                // Quoted config values
                vec!["-c".into(), format!("sandbox_mode=\"{mode}\"")],
                vec!["-c".into(), format!("sandbox_mode='{mode}'")],
                vec!["--config".into(), format!("sandbox_mode=\"{mode}\"")],
                vec!["--config".into(), format!("sandbox_mode='{mode}'")],
                vec![format!("--config=sandbox_mode=\"{mode}\"")],
                vec![format!("--config=sandbox_mode='{mode}'")],
            ];

            for case in test_cases {
                let result = plan_passthrough_args(case.clone());
                assert_eq!(
                    result,
                    Err(PassthroughError::UnsupportedSandboxMode(mode.to_string())),
                    "expected rejection for case {:?}",
                    case
                );
                let msg = result.unwrap_err().to_string();
                assert!(
                    msg.contains("Termux"),
                    "error message '{msg}' must mention Termux"
                );
                assert!(
                    msg.contains("Linux sandbox"),
                    "error message '{msg}' must mention Linux sandbox"
                );
                assert!(
                    msg.contains(mode),
                    "error message '{msg}' must mention mode '{mode}'"
                );
                assert!(
                    msg.contains("cannot be enforced"),
                    "error message '{msg}' must mention cannot be enforced"
                );
            }
        }
    }

    #[test]
    fn test_passthrough_rejects_leading_sandbox_linux_subcommand() {
        let cases: Vec<Vec<&str>> = vec![
            vec!["sandbox", "linux"],
            vec!["sandbox", "linux", "--help"],
            vec!["sandbox", "linux", "run", "--some-flag"],
        ];

        for case in cases {
            let result = plan_passthrough_args(case.clone());
            assert_eq!(
                result,
                Err(PassthroughError::UnsupportedSandboxSubcommand),
                "expected rejection for leading sandbox linux case {:?}",
                case
            );
            let msg = result.unwrap_err().to_string();
            assert!(
                msg.contains("Termux"),
                "error message '{msg}' must mention Termux"
            );
            assert!(
                msg.contains("'sandbox linux'"),
                "error message '{msg}' must mention 'sandbox linux'"
            );
            assert!(
                msg.contains("cannot be enforced"),
                "error message '{msg}' must mention cannot be enforced"
            );
        }
    }

    #[test]
    fn test_passthrough_allows_danger_full_access_forms() {
        let cases: Vec<Vec<&str>> = vec![
            vec!["--sandbox", "danger-full-access"],
            vec!["--sandbox=danger-full-access"],
            vec!["-s", "danger-full-access"],
            vec!["-sdanger-full-access"],
            vec!["-c", "sandbox_mode=danger-full-access"],
            vec!["-c", "sandbox_mode=\"danger-full-access\""],
            vec!["-c", "sandbox_mode='danger-full-access'"],
            vec!["--config", "sandbox_mode=danger-full-access"],
            vec!["--config", "sandbox_mode=\"danger-full-access\""],
            vec!["--config=sandbox_mode=danger-full-access"],
            vec!["--config=sandbox_mode=\"danger-full-access\""],
            vec!["--config=sandbox_mode='danger-full-access'"],
        ];

        for case in cases {
            let res = plan_passthrough_args(case.clone())
                .unwrap_or_else(|e| panic!("expected {:?} to be accepted, got error: {e}", case));
            assert_eq!(res.len(), case.len() + 2);
            assert_eq!(res[0], OsStr::new("-c"));
            assert_eq!(res[1], OsStr::new("sandbox_mode=\"danger-full-access\""));
            for (out_elem, in_elem) in res[2..].iter().zip(case.iter()) {
                assert_eq!(out_elem, OsStr::new(in_elem));
            }
        }
    }

    #[test]
    fn test_passthrough_double_dash_stops_scanning() {
        let cases: Vec<Vec<&str>> = vec![
            vec!["--", "--sandbox", "read-only"],
            vec!["run", "--", "-sworkspace-write"],
            vec!["--", "sandbox", "linux"],
            vec!["--", "-c", "sandbox_mode=read-only"],
            vec![
                "exec",
                "task",
                "--",
                "--config=sandbox_mode=workspace-write",
                "-sread-only",
            ],
        ];

        for case in cases {
            let res = plan_passthrough_args(case.clone()).unwrap_or_else(|e| {
                panic!("expected {:?} to be accepted after '--', got: {e}", case)
            });
            assert_eq!(res.len(), case.len() + 2);
            assert_eq!(res[0], OsStr::new("-c"));
            assert_eq!(res[1], OsStr::new("sandbox_mode=\"danger-full-access\""));
            for (out_elem, in_elem) in res[2..].iter().zip(case.iter()) {
                assert_eq!(out_elem, OsStr::new(in_elem));
            }
        }
    }

    #[test]
    fn test_passthrough_preserves_arbitrary_and_non_utf8_arguments() {
        let arbitrary_cases: Vec<Vec<&str>> = vec![
            vec!["exec", "foo", "bar", "--flag=value"],
            vec!["run", "--opt=123", "arg with space and =", "-j8"],
        ];

        for case in arbitrary_cases {
            let res = plan_passthrough_args(case.clone()).expect("arbitrary args should succeed");
            assert_eq!(res[0], OsStr::new("-c"));
            assert_eq!(res[1], OsStr::new("sandbox_mode=\"danger-full-access\""));
            for (out_elem, in_elem) in res[2..].iter().zip(case.iter()) {
                assert_eq!(out_elem, OsStr::new(in_elem));
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let non_utf8 = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
            let raw_args = vec![
                OsString::from("exec"),
                non_utf8.to_os_string(),
                OsString::from("--custom-flag"),
            ];
            let res = plan_passthrough_args(raw_args.clone())
                .expect("non-utf8 passthrough should succeed");
            assert_eq!(res[0], OsStr::new("-c"));
            assert_eq!(res[1], OsStr::new("sandbox_mode=\"danger-full-access\""));
            assert_eq!(res[2], OsStr::new("exec"));
            assert_eq!(res[3].as_bytes(), &[0xff, 0xfe, 0x80, 0x7f]);
            assert_eq!(res[4], OsStr::new("--custom-flag"));
        }
    }

    #[test]
    fn test_passthrough_ordinary_input_prelude_and_no_synthesized_bypass() {
        // Empty argv
        let empty_res = plan_passthrough_args(Vec::<&str>::new()).expect("empty argv");
        assert_eq!(
            empty_res,
            vec![
                OsString::from("-c"),
                OsString::from("sandbox_mode=\"danger-full-access\""),
            ]
        );

        // Ordinary argv
        let ordinary = vec!["run", "my_app", "--verbose"];
        let ord_res = plan_passthrough_args(ordinary.clone()).expect("ordinary args");
        assert_eq!(ord_res[0], OsStr::new("-c"));
        assert_eq!(
            ord_res[1],
            OsStr::new("sandbox_mode=\"danger-full-access\"")
        );
        for (out_elem, in_elem) in ord_res[2..].iter().zip(ordinary.iter()) {
            assert_eq!(out_elem, OsStr::new(in_elem));
        }
        assert!(
            !ord_res
                .iter()
                .any(|a| a == "--dangerously-bypass-approvals-and-sandbox"),
            "Core must never synthesize --dangerously-bypass-approvals-and-sandbox"
        );

        // Explicit user-supplied bypass option is preserved unchanged without synthesis
        let user_bypass = vec!["--dangerously-bypass-approvals-and-sandbox", "run"];
        let bypass_res = plan_passthrough_args(user_bypass.clone()).expect("user bypass");
        assert_eq!(
            bypass_res,
            vec![
                OsString::from("-c"),
                OsString::from("sandbox_mode=\"danger-full-access\""),
                OsString::from("--dangerously-bypass-approvals-and-sandbox"),
                OsString::from("run"),
            ]
        );
        let count = bypass_res
            .iter()
            .filter(|a| *a == "--dangerously-bypass-approvals-and-sandbox")
            .count();
        assert_eq!(
            count, 1,
            "must preserve exactly the user-supplied token without duplicates"
        );
    }

    #[test]
    fn test_passthrough_missing_option_values_and_unrelated_configs_accepted() {
        let cases: Vec<Vec<&str>> = vec![
            // Missing option values are preserved for upstream error handling
            vec!["-s"],
            vec!["--sandbox"],
            vec!["-c"],
            vec!["--config"],
            vec!["-s", "--other-flag"],
            vec!["--config", "-s"],
            // Unrelated configs and flags containing read-only or workspace-write
            vec!["-c", "model=read-only"],
            vec!["-c", "prompt=file-is-read-only"],
            vec!["--config", "workspace_root=workspace-write"],
            vec!["--config=custom_setting=read-only"],
            vec!["exec", "read-only"],
            vec!["exec", "workspace-write"],
            vec!["run", "sandbox", "linux"], // Not leading
            vec!["sandbox", "macos"],
            vec!["sandbox"],
            vec!["linux", "sandbox"],
        ];

        for case in cases {
            let res = plan_passthrough_args(case.clone())
                .unwrap_or_else(|e| panic!("expected {:?} to be accepted, got: {e}", case));
            assert_eq!(res.len(), case.len() + 2);
            assert_eq!(res[0], OsStr::new("-c"));
            assert_eq!(res[1], OsStr::new("sandbox_mode=\"danger-full-access\""));
            for (out_elem, in_elem) in res[2..].iter().zip(case.iter()) {
                assert_eq!(out_elem, OsStr::new(in_elem));
            }
        }
    }

    #[test]
    fn test_passthrough_separate_option_consumption_and_unrelated_forms_regression() {
        let cases: Vec<Vec<&str>> = vec![
            vec!["--config", "--sandbox", "read-only"],
            vec!["-s", "--sandbox", "workspace-write"],
            vec!["-c", "-s", "read-only"],
            vec!["--sandbox", "-c", "sandbox_mode=read-only"],
            vec!["-s", "--config", "sandbox_mode=workspace-write"],
            vec!["-c", "sandbox_mode_extra=read-only"],
            vec!["-c", "sandbox_mode_custom=workspace-write"],
            vec!["--config", "sandbox_mode_extra=read-only"],
            vec!["--config=sandbox_mode_extra=read-only"],
            vec!["--config=sandbox_mode_custom=workspace-write"],
        ];
        for case in cases {
            let res = plan_passthrough_args(case.clone())
                .unwrap_or_else(|e| panic!("expected {:?} to be accepted, got error: {e}", case));
            assert_eq!(&res[2..], case.as_slice());
        }

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let non_utf8 = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
            let raw_args = vec![
                OsString::from("-s"),
                non_utf8.to_os_string(),
                OsString::from("read-only"),
            ];
            let res = plan_passthrough_args(raw_args.clone())
                .expect("non-utf8 option value consumption should succeed");
            assert_eq!(res[3].as_bytes(), &[0xff, 0xfe, 0x80, 0x7f]);
            assert_eq!(res[4], OsStr::new("read-only"));
        }
    }

    #[test]
    fn test_m1_r1_sandbox_config_whitespace_attached_and_unknown_fail_closed() {
        let cases: Vec<(Vec<&str>, &str)> = vec![
            (vec!["-c", "sandbox_mode =read-only"], "read-only"),
            (
                vec!["-c", " sandbox_mode=workspace-write"],
                "workspace-write",
            ),
            (vec!["-c", " sandbox_mode = \"read-only\" "], "read-only"),
            (vec!["-csandbox_mode=read-only"], "read-only"),
            (
                vec!["-csandbox_mode = 'workspace-write'"],
                "workspace-write",
            ),
            (
                vec!["--config", "sandbox_mode = \"read-only\""],
                "read-only",
            ),
            (
                vec!["--config=sandbox_mode = 'workspace-write'"],
                "workspace-write",
            ),
            (
                vec!["-c", "sandbox_mode=future-linux-sandbox"],
                "future-linux-sandbox",
            ),
            (vec!["--sandbox", "\"read-only\""], "read-only"),
            (vec!["--sandbox='workspace-write'"], "workspace-write"),
            (vec!["-s=read-only"], "read-only"),
            (vec!["-s='workspace-write'"], "workspace-write"),
            (vec!["-c=sandbox_mode=read-only"], "read-only"),
            (vec!["-c", "\"sandbox_mode\"=\"read-only\""], "read-only"),
            (
                vec!["-c", "'sandbox_mode'='workspace-write'"],
                "workspace-write",
            ),
        ];
        for (args, expected) in cases {
            assert_eq!(
                plan_passthrough_args(args),
                Err(PassthroughError::UnsupportedSandboxMode(
                    expected.to_string()
                ))
            );
        }
    }

    #[test]
    fn test_m1_r1_sandbox_danger_full_access_normalizes_without_rewriting_user_argv() {
        let cases: Vec<Vec<&str>> = vec![
            vec!["-c", " sandbox_mode = \"danger-full-access\" "],
            vec!["-csandbox_mode=danger-full-access"],
            vec!["-c=sandbox_mode=danger-full-access"],
            vec!["--config=sandbox_mode = 'danger-full-access'"],
            vec!["-s=danger-full-access"],
            vec!["--sandbox", "\"danger-full-access\""],
        ];
        for case in cases {
            let planned = plan_passthrough_args(case.clone()).expect("supported sandbox mode");
            assert_eq!(planned[0], OsStr::new("-c"));
            assert_eq!(
                planned[1],
                OsStr::new("sandbox_mode=\"danger-full-access\"")
            );
            for (actual, original) in planned[2..].iter().zip(case.iter()) {
                assert_eq!(actual, OsStr::new(original));
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_r1_fd_restore_reports_syscall_failure() {
        use std::os::unix::io::AsRawFd;
        let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
        let target_fd = unsafe { fcntl(devnull.as_raw_fd(), F_DUPFD_CLOEXEC, 100) };
        assert!(target_fd >= 100);
        let mut state = unsafe { PriorFdState::capture(target_fd) }.expect("capture high fd");
        let backup_fd = match &state {
            PriorFdState::Present { backup_fd, .. } => *backup_fd,
            PriorFdState::Absent => panic!("high target fd unexpectedly absent"),
        };
        assert_eq!(unsafe { close(backup_fd) }, 0);
        let err = unsafe { state.restore_and_cleanup(target_fd) }
            .expect_err("closed backup must make restoration observable");
        assert_eq!(err.raw_os_error(), Some(EBADF));
        let _ = unsafe { close(target_fd) };
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b7_policy_before_io_rejection() {
        let nonexistent_prog = OsStr::new("/path/that/does/not/exist/codex-bin-999");
        let nonexistent_resolver =
            std::path::Path::new("/path/that/does/not/exist/resolv-999.conf");
        let nonexistent_config = std::path::Path::new("/path/that/does/not/exist/config-dir-999");

        let cases: Vec<(Vec<&str>, PassthroughError)> = vec![
            (
                vec!["-s", "read-only"],
                PassthroughError::UnsupportedSandboxMode("read-only".to_string()),
            ),
            (
                vec!["--sandbox", "workspace-write"],
                PassthroughError::UnsupportedSandboxMode("workspace-write".to_string()),
            ),
            (
                vec!["--sandbox=read-only"],
                PassthroughError::UnsupportedSandboxMode("read-only".to_string()),
            ),
            (
                vec!["-sworkspace-write"],
                PassthroughError::UnsupportedSandboxMode("workspace-write".to_string()),
            ),
            (
                vec!["sandbox", "linux"],
                PassthroughError::UnsupportedSandboxSubcommand,
            ),
            (
                vec!["sandbox", "linux", "run", "--flag"],
                PassthroughError::UnsupportedSandboxSubcommand,
            ),
            (
                vec!["-c", "sandbox_mode=read-only"],
                PassthroughError::UnsupportedSandboxMode("read-only".to_string()),
            ),
            (
                vec!["--config=sandbox_mode=workspace-write"],
                PassthroughError::UnsupportedSandboxMode("workspace-write".to_string()),
            ),
            (
                vec!["-c", "sandbox_mode=\"read-only\""],
                PassthroughError::UnsupportedSandboxMode("read-only".to_string()),
            ),
            (
                vec!["--config", "sandbox_mode='workspace-write'"],
                PassthroughError::UnsupportedSandboxMode("workspace-write".to_string()),
            ),
        ];

        for (args, expected_policy_err) in cases {
            let err = launch_upstream(
                nonexistent_prog,
                nonexistent_resolver,
                nonexistent_config,
                args.clone(),
            );

            match err {
                LaunchError::Policy(policy_err) => {
                    assert_eq!(
                        policy_err, expected_policy_err,
                        "expected policy error {:?} for args {:?}",
                        expected_policy_err, args
                    );
                    let msg = policy_err.to_string();
                    assert!(
                        msg.contains("Termux"),
                        "error msg '{msg}' must mention Termux"
                    );
                    assert!(
                        msg.contains("cannot be enforced"),
                        "error msg '{msg}' must mention cannot be enforced"
                    );
                }
                LaunchError::Exec(exec_err) => {
                    panic!(
                        "launch_upstream must reject policy before I/O, but got Exec error: {exec_err} for args {:?}",
                        args
                    );
                }
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b7_accepted_real_exec_composition() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b7-real-exec-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");

        let marker_path = config_dir_path.join("marker.txt");
        std::fs::write(&marker_path, b"CONFIG_DIR_MARKER_CONTENT").expect("write marker");
        let resolver_bytes = b"# synthetic resolv.conf\nnameserver 198.51.100.1\n";
        std::fs::write(&resolver_path, resolver_bytes).expect("write resolver");

        let shell = resolve_test_shell();
        let fake_upstream_path = test_root.join("fake_upstream.sh");

        let script_content = format!(
            r##"#!{}
if [ "$1" != "-c" ]; then
    printf "ARGV_MISMATCH: 1 is '%s', expected '-c'\n" "$1" >&2
    exit 11
fi

if [ "$2" != 'sandbox_mode="danger-full-access"' ]; then
    printf "ARGV_MISMATCH: 2 is '%s', expected 'sandbox_mode=\"danger-full-access\"'\n" "$2" >&2
    exit 12
fi

if [ "$3" != "exec" ]; then
    printf "ARGV_MISMATCH: 3 is '%s', expected 'exec'\n" "$3" >&2
    exit 13
fi

if [ "$4" != "custom_task" ]; then
    printf "ARGV_MISMATCH: 4 is '%s', expected 'custom_task'\n" "$4" >&2
    exit 14
fi

if [ "$5" != "--custom-flag=val1" ]; then
    printf "ARGV_MISMATCH: 5 is '%s', expected '--custom-flag=val1'\n" "$5" >&2
    exit 15
fi

if [ "$6" != "ordinary arg with spaces and =" ]; then
    printf "ARGV_MISMATCH: 6 is '%s', expected 'ordinary arg with spaces and ='\n" "$6" >&2
    exit 16
fi

for a in "$@"; do
    printf "ARG:%s\n" "$a"
done

res_content=$(cat /proc/self/fd/33 2>/dev/null)
expected_res="# synthetic resolv.conf
nameserver 198.51.100.1"
if [ "$res_content" != "$expected_res" ]; then
    printf "RESOLVER_FD33_MISMATCH: got '%s'\n" "$res_content" >&2
    exit 20
fi

while read -r key val; do
    if [ "$key" = "flags:" ]; then
        case "$val" in
            *0|*4) ;;
            *)
                printf "RESOLVER_FD33_NOT_RDONLY: flags '%s'\n" "$val" >&2
                exit 21
                ;;
        esac
        break
    fi
done < /proc/self/fdinfo/33

if [ ! -d /proc/self/fd/34 ]; then
    printf "CONFIG_FD34_NOT_DIRECTORY\n" >&2
    exit 30
fi
if [ ! -f /proc/self/fd/34/marker.txt ]; then
    printf "CONFIG_FD34_MARKER_MISSING\n" >&2
    exit 31
fi
marker_content=$(cat /proc/self/fd/34/marker.txt)
if [ "$marker_content" != "CONFIG_DIR_MARKER_CONTENT" ]; then
    printf "CONFIG_FD34_MARKER_MISMATCH: '%s'\n" "$marker_content" >&2
    exit 32
fi

if [ -n "${{CODEX_MANAGED_BY_NPM+x}}" ]; then
    printf "ENV_FENCE_FAILED: CODEX_MANAGED_BY_NPM is present: %s\n" "$CODEX_MANAGED_BY_NPM" >&2
    exit 40
fi
if [ -n "${{CODEX_MANAGED_BY_BUN+x}}" ]; then
    printf "ENV_FENCE_FAILED: CODEX_MANAGED_BY_BUN is present: %s\n" "$CODEX_MANAGED_BY_BUN" >&2
    exit 41
fi
if [ -n "${{CODEX_MANAGED_PACKAGE_ROOT+x}}" ]; then
    printf "ENV_FENCE_FAILED: CODEX_MANAGED_PACKAGE_ROOT is present: %s\n" "$CODEX_MANAGED_PACKAGE_ROOT" >&2
    exit 42
fi
if [ -n "${{LD_PRELOAD+x}}" ]; then
    printf "ENV_FENCE_FAILED: LD_PRELOAD is present: %s\n" "$LD_PRELOAD" >&2
    exit 43
fi
if [ -n "${{LD_LIBRARY_PATH+x}}" ]; then
    printf "ENV_FENCE_FAILED: LD_LIBRARY_PATH is present: %s\n" "$LD_LIBRARY_PATH" >&2
    exit 44
fi

if [ "$CODEX_TEST_UNRELATED_M1_B7_SURVIVING_VAR" != "m1_b7_surviving_exact_value_84920" ]; then
    printf "UNRELATED_ENV_MISMATCH: got '%s'\n" "$CODEX_TEST_UNRELATED_M1_B7_SURVIVING_VAR" >&2
    exit 50
fi

printf "M1_B7_REAL_EXEC_SUCCESS\n"
exit 0
"##,
            shell.to_str().expect("valid shell path")
        );

        std::fs::write(&fake_upstream_path, script_content).expect("write fake upstream");
        let mut perms = std::fs::metadata(&fake_upstream_path)
            .expect("metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_upstream_path, perms).expect("set permissions 0755");

        let result = run_exec_probe_with_env(
            "m1_b7_fake_upstream_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, fake_upstream_path.as_os_str()),
            ],
        );

        assert_eq!(
            result.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stderr, b"");

        let mut expected_stdout = Vec::new();
        expected_stdout.extend_from_slice(b"ARG:-c\n");
        expected_stdout.extend_from_slice(b"ARG:sandbox_mode=\"danger-full-access\"\n");
        expected_stdout.extend_from_slice(b"ARG:exec\n");
        expected_stdout.extend_from_slice(b"ARG:custom_task\n");
        expected_stdout.extend_from_slice(b"ARG:--custom-flag=val1\n");
        expected_stdout.extend_from_slice(b"ARG:ordinary arg with spaces and =\n");
        expected_stdout.extend_from_slice(b"ARG:");
        expected_stdout.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).as_bytes());
        expected_stdout.extend_from_slice(b"\n");
        expected_stdout.extend_from_slice(b"M1_B7_REAL_EXEC_SUCCESS\n");

        assert_eq!(result.stdout, expected_stdout);

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b7_failed_exec_restores_absent() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b7-fail-absent-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result = run_exec_probe_with_env(
            "m1_b7_failed_exec_restoration_absent",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"M1_B7_RESTORED_ABSENT_SUCCESS\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b7_failed_exec_restores_sentinels() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b7-fail-sentinels-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result = run_exec_probe_with_env(
            "m1_b7_failed_exec_restoration_sentinels",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );

        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stdout, b"M1_B7_RESTORED_SENTINELS_SUCCESS\n");
        assert_eq!(result.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_a_exact_temp_vars_order_and_fallback_cert_file() {
        let inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/test/compat"),
            prefix_bin_dir: OsStr::new("/test/prefix/bin"),
            temp_dir: OsStr::new("/test/isolated/tmp"),
            cert_file: OsStr::new("/test/prefix/etc/tls/cert.pem"),
            cert_dir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let plan = plan_termux_base_env(&inputs).expect("plan must succeed");
        let assignments = plan.assignments();

        assert_eq!(assignments.len(), 6);
        assert_eq!(
            assignments[0],
            (
                OsString::from("TMPDIR"),
                OsString::from("/test/isolated/tmp")
            )
        );
        assert_eq!(
            assignments[1],
            (OsString::from("TMP"), OsString::from("/test/isolated/tmp"))
        );
        assert_eq!(
            assignments[2],
            (OsString::from("TEMP"), OsString::from("/test/isolated/tmp"))
        );
        assert_eq!(
            assignments[3],
            (
                OsString::from("SQLITE_TMPDIR"),
                OsString::from("/test/isolated/tmp")
            )
        );
        assert_eq!(
            assignments[4],
            (
                OsString::from("SSL_CERT_FILE"),
                OsString::from("/test/prefix/etc/tls/cert.pem")
            )
        );
        assert_eq!(
            assignments[5],
            (
                OsString::from("PATH"),
                OsString::from("/test/compat:/test/prefix/bin")
            )
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_b_ssl_cert_file_precedence() {
        use std::os::unix::ffi::OsStrExt;

        let base_inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/test/compat"),
            prefix_bin_dir: OsStr::new("/test/prefix/bin"),
            temp_dir: OsStr::new("/test/tmp"),
            cert_file: OsStr::new("/test/selected/cert.pem"),
            cert_dir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        // 1. Non-empty inherited wins
        let mut inputs1 = base_inputs.clone();
        inputs1.inherited_ssl_cert_file = Some(OsStr::new("/custom/inherited/cert.pem"));
        let plan1 = plan_termux_base_env(&inputs1).expect("plan must succeed");
        assert_eq!(
            plan1.get("SSL_CERT_FILE"),
            Some(OsStr::new("/custom/inherited/cert.pem"))
        );

        // 2. Non-empty inherited non-UTF-8 bytes win byte-for-byte
        {
            let non_utf8_cert = OsStr::from_bytes(b"/custom/inherited/\xff\xfe/cert.pem");
            let mut inputs_raw = base_inputs.clone();
            inputs_raw.inherited_ssl_cert_file = Some(non_utf8_cert);
            let plan_raw = plan_termux_base_env(&inputs_raw).expect("plan must succeed");
            assert_eq!(
                plan_raw.get("SSL_CERT_FILE").map(|s| s.as_bytes()),
                Some(b"/custom/inherited/\xff\xfe/cert.pem".as_slice())
            );
        }

        // 3. Empty inherited falls back to selected cert file
        let mut inputs2 = base_inputs.clone();
        inputs2.inherited_ssl_cert_file = Some(OsStr::new(""));
        let plan2 = plan_termux_base_env(&inputs2).expect("plan must succeed");
        assert_eq!(
            plan2.get("SSL_CERT_FILE"),
            Some(OsStr::new("/test/selected/cert.pem"))
        );

        // 4. Unset inherited falls back to selected cert file
        let mut inputs3 = base_inputs.clone();
        inputs3.inherited_ssl_cert_file = None;
        let plan3 = plan_termux_base_env(&inputs3).expect("plan must succeed");
        assert_eq!(
            plan3.get("SSL_CERT_FILE"),
            Some(OsStr::new("/test/selected/cert.pem"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_c_ssl_cert_dir_precedence() {
        let base_inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/test/compat"),
            prefix_bin_dir: OsStr::new("/test/prefix/bin"),
            temp_dir: OsStr::new("/test/tmp"),
            cert_file: OsStr::new("/test/cert.pem"),
            cert_dir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        // 1. Inherited non-empty wins over selected cert dir
        let mut inputs1 = base_inputs.clone();
        inputs1.cert_dir = Some(OsStr::new("/selected/certs"));
        inputs1.inherited_ssl_cert_dir = Some(OsStr::new("/inherited/certs"));
        let plan1 = plan_termux_base_env(&inputs1).expect("plan must succeed");
        assert_eq!(
            plan1.get("SSL_CERT_DIR"),
            Some(OsStr::new("/inherited/certs"))
        );

        // 2. Inherited non-empty wins when selected cert dir is None
        let mut inputs2 = base_inputs.clone();
        inputs2.cert_dir = None;
        inputs2.inherited_ssl_cert_dir = Some(OsStr::new("/inherited/certs"));
        let plan2 = plan_termux_base_env(&inputs2).expect("plan must succeed");
        assert_eq!(
            plan2.get("SSL_CERT_DIR"),
            Some(OsStr::new("/inherited/certs"))
        );

        // 3. Inherited empty falls back to selected optional dir
        let mut inputs3 = base_inputs.clone();
        inputs3.cert_dir = Some(OsStr::new("/selected/certs"));
        inputs3.inherited_ssl_cert_dir = Some(OsStr::new(""));
        let plan3 = plan_termux_base_env(&inputs3).expect("plan must succeed");
        assert_eq!(
            plan3.get("SSL_CERT_DIR"),
            Some(OsStr::new("/selected/certs"))
        );

        // 4. Inherited unset falls back to selected optional dir
        let mut inputs4 = base_inputs.clone();
        inputs4.cert_dir = Some(OsStr::new("/selected/certs"));
        inputs4.inherited_ssl_cert_dir = None;
        let plan4 = plan_termux_base_env(&inputs4).expect("plan must succeed");
        assert_eq!(
            plan4.get("SSL_CERT_DIR"),
            Some(OsStr::new("/selected/certs"))
        );

        // 5. Inherited unset and selected None => no assignment exists
        let mut inputs5 = base_inputs.clone();
        inputs5.cert_dir = None;
        inputs5.inherited_ssl_cert_dir = None;
        let plan5 = plan_termux_base_env(&inputs5).expect("plan must succeed");
        assert_eq!(plan5.get("SSL_CERT_DIR"), None);
        assert!(!plan5.contains_key("SSL_CERT_DIR"));

        // 6. Inherited empty and selected None => no assignment exists
        let mut inputs6 = base_inputs.clone();
        inputs6.cert_dir = None;
        inputs6.inherited_ssl_cert_dir = Some(OsStr::new(""));
        let plan6 = plan_termux_base_env(&inputs6).expect("plan must succeed");
        assert_eq!(plan6.get("SSL_CERT_DIR"), None);
        assert!(!plan6.contains_key("SSL_CERT_DIR"));

        // 7. Inherited unset and selected empty => no assignment exists
        let mut inputs7 = base_inputs.clone();
        inputs7.cert_dir = Some(OsStr::new(""));
        inputs7.inherited_ssl_cert_dir = None;
        let plan7 = plan_termux_base_env(&inputs7).expect("plan must succeed");
        assert_eq!(plan7.get("SSL_CERT_DIR"), None);
        assert!(!plan7.contains_key("SSL_CERT_DIR"));

        // 8. Inherited empty and selected empty => no assignment exists
        let mut inputs8 = base_inputs.clone();
        inputs8.cert_dir = Some(OsStr::new(""));
        inputs8.inherited_ssl_cert_dir = Some(OsStr::new(""));
        let plan8 = plan_termux_base_env(&inputs8).expect("plan must succeed");
        assert_eq!(plan8.get("SSL_CERT_DIR"), None);
        assert!(!plan8.contains_key("SSL_CERT_DIR"));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_d_path_exact_ordering_normal_absent_empty() {
        let base_inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/custom/compat/bin"),
            prefix_bin_dir: OsStr::new("/custom/prefix/bin"),
            temp_dir: OsStr::new("/custom/tmp"),
            cert_file: OsStr::new("/custom/cert.pem"),
            cert_dir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        // 1. Normal inherited PATH
        let mut inputs_normal = base_inputs.clone();
        inputs_normal.inherited_path = Some(OsStr::new("/usr/local/bin:/usr/bin:/bin"));
        let plan_normal = plan_termux_base_env(&inputs_normal).expect("plan must succeed");
        assert_eq!(
            plan_normal.get("PATH"),
            Some(OsStr::new(
                "/custom/compat/bin:/custom/prefix/bin:/usr/local/bin:/usr/bin:/bin"
            ))
        );

        // 2. Absent inherited PATH (None)
        let mut inputs_absent = base_inputs.clone();
        inputs_absent.inherited_path = None;
        let plan_absent = plan_termux_base_env(&inputs_absent).expect("plan must succeed");
        assert_eq!(
            plan_absent.get("PATH"),
            Some(OsStr::new("/custom/compat/bin:/custom/prefix/bin"))
        );

        // 3. Empty inherited PATH ("")
        let mut inputs_empty = base_inputs.clone();
        inputs_empty.inherited_path = Some(OsStr::new(""));
        let plan_empty = plan_termux_base_env(&inputs_empty).expect("plan must succeed");
        assert_eq!(
            plan_empty.get("PATH"),
            Some(OsStr::new("/custom/compat/bin:/custom/prefix/bin"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_e_unix_non_utf8_inherited_path_bytes() {
        use std::os::unix::ffi::OsStrExt;

        let raw_inherited_bytes: &[u8] = b"/custom/bin\xff\xfe:/other/\x80\x81/bin:/system/bin";
        let non_utf8_path = OsStr::from_bytes(raw_inherited_bytes);

        let inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/opt/compat"),
            prefix_bin_dir: OsStr::new("/opt/prefix/bin"),
            temp_dir: OsStr::new("/opt/tmp"),
            cert_file: OsStr::new("/opt/cert.pem"),
            cert_dir: None,
            inherited_path: Some(non_utf8_path),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let plan = plan_termux_base_env(&inputs).expect("plan must succeed");
        let path_val = plan.get("PATH").expect("PATH assignment must exist");

        let mut expected_bytes = Vec::new();
        expected_bytes.extend_from_slice(b"/opt/compat:/opt/prefix/bin:");
        expected_bytes.extend_from_slice(raw_inherited_bytes);

        assert_eq!(path_val.as_bytes(), expected_bytes.as_slice());
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_f_synthetic_unusual_explicit_paths_no_hardcoded_roots() {
        use std::os::unix::ffi::OsStrExt;

        let inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/synthetic/custom_root_99/compat_tools"),
            prefix_bin_dir: OsStr::new("/opt/custom_distro/bin_arch64"),
            temp_dir: OsStr::new("/var/volatile/isolated_run_42/tmp"),
            cert_file: OsStr::new("/etc/ssl_custom/bundle_99.crt"),
            cert_dir: Some(OsStr::new("/etc/ssl_custom/certs.d")),
            inherited_path: Some(OsStr::new("/vendor/bin:/system_alt/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let plan = plan_termux_base_env(&inputs).expect("plan must succeed");

        assert_eq!(
            plan.get("TMPDIR"),
            Some(OsStr::new("/var/volatile/isolated_run_42/tmp"))
        );
        assert_eq!(
            plan.get("TMP"),
            Some(OsStr::new("/var/volatile/isolated_run_42/tmp"))
        );
        assert_eq!(
            plan.get("TEMP"),
            Some(OsStr::new("/var/volatile/isolated_run_42/tmp"))
        );
        assert_eq!(
            plan.get("SQLITE_TMPDIR"),
            Some(OsStr::new("/var/volatile/isolated_run_42/tmp"))
        );
        assert_eq!(
            plan.get("SSL_CERT_FILE"),
            Some(OsStr::new("/etc/ssl_custom/bundle_99.crt"))
        );
        assert_eq!(
            plan.get("SSL_CERT_DIR"),
            Some(OsStr::new("/etc/ssl_custom/certs.d"))
        );
        assert_eq!(
            plan.get("PATH"),
            Some(OsStr::new(
                "/synthetic/custom_root_99/compat_tools:/opt/custom_distro/bin_arch64:/vendor/bin:/system_alt/bin"
            ))
        );

        // Verify none of the standard Android / Termux paths appear
        for (_k, v) in plan.assignments() {
            let bytes = v.as_bytes();
            assert!(
                !bytes
                    .windows(b"/data/data".len())
                    .any(|w| w == b"/data/data"),
                "found hardcoded /data/data in planned value"
            );
            assert!(
                !bytes
                    .windows(b"com.termux".len())
                    .any(|w| w == b"com.termux"),
                "found hardcoded com.termux in planned value"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_g_invalid_explicit_path_components() {
        use std::os::unix::ffi::OsStrExt;

        let base_inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/test/compat"),
            prefix_bin_dir: OsStr::new("/test/prefix/bin"),
            temp_dir: OsStr::new("/test/tmp"),
            cert_file: OsStr::new("/test/cert.pem"),
            cert_dir: None,
            inherited_path: Some(OsStr::new("/usr/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        // 1. Empty compat_dir
        let mut in_empty_compat = base_inputs.clone();
        in_empty_compat.compat_dir = OsStr::new("");
        assert_eq!(
            plan_termux_base_env(&in_empty_compat),
            Err(TermuxBaseEnvError::EmptyPathComponent("compat_dir"))
        );

        // 2. Colon in compat_dir
        let mut in_colon_compat = base_inputs.clone();
        in_colon_compat.compat_dir = OsStr::new("/bin:/usr/bin");
        assert_eq!(
            plan_termux_base_env(&in_colon_compat),
            Err(TermuxBaseEnvError::ColonInPathComponent("compat_dir"))
        );

        // 3. Empty prefix_bin_dir
        let mut in_empty_prefix = base_inputs.clone();
        in_empty_prefix.prefix_bin_dir = OsStr::new("");
        assert_eq!(
            plan_termux_base_env(&in_empty_prefix),
            Err(TermuxBaseEnvError::EmptyPathComponent("prefix_bin_dir"))
        );

        // 4. Colon in prefix_bin_dir
        let mut in_colon_prefix = base_inputs.clone();
        in_colon_prefix.prefix_bin_dir = OsStr::new("/usr/local/bin:/usr/bin");
        assert_eq!(
            plan_termux_base_env(&in_colon_prefix),
            Err(TermuxBaseEnvError::ColonInPathComponent("prefix_bin_dir"))
        );

        // 5. NUL byte in explicit path components on Unix
        let mut in_nul_compat = base_inputs.clone();
        in_nul_compat.compat_dir = OsStr::from_bytes(b"/test/\0compat");
        assert_eq!(
            plan_termux_base_env(&in_nul_compat),
            Err(TermuxBaseEnvError::NulInPathComponent("compat_dir"))
        );

        let mut in_nul_prefix = base_inputs.clone();
        in_nul_prefix.prefix_bin_dir = OsStr::from_bytes(b"/test/prefix\0bin");
        assert_eq!(
            plan_termux_base_env(&in_nul_prefix),
            Err(TermuxBaseEnvError::NulInPathComponent("prefix_bin_dir"))
        );

        // Verify Display implementations
        let err_empty = TermuxBaseEnvError::EmptyPathComponent("compat_dir");
        assert_eq!(
            err_empty.to_string(),
            "explicit PATH component 'compat_dir' must not be empty"
        );
        let err_colon = TermuxBaseEnvError::ColonInPathComponent("prefix_bin_dir");
        assert_eq!(
            err_colon.to_string(),
            "explicit PATH component 'prefix_bin_dir' must not contain ':'"
        );
        let err_nul = TermuxBaseEnvError::NulInPathComponent("compat_dir");
        assert_eq!(
            err_nul.to_string(),
            "explicit PATH component 'compat_dir' must not contain NUL byte"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_h_negative_assertion_excluded_keys() {
        let excluded_keys = [
            "HOME",
            "CODEX_HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "GODEBUG",
            "BROWSER",
            "CODEX_SELF_EXE",
            "CODEX_CODE_MODE_HOST_PATH",
            "CODEX_MANAGED_BY_NPM",
            "CODEX_MANAGED_BY_BUN",
            "CODEX_MANAGED_PACKAGE_ROOT",
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
        ];

        let scenarios = [
            // Scenario 1: without SSL_CERT_DIR
            TermuxBaseEnvInputs {
                compat_dir: OsStr::new("/test/compat"),
                prefix_bin_dir: OsStr::new("/test/prefix/bin"),
                temp_dir: OsStr::new("/test/tmp"),
                cert_file: OsStr::new("/test/cert.pem"),
                cert_dir: None,
                inherited_path: Some(OsStr::new("/usr/bin")),
                inherited_ssl_cert_file: None,
                inherited_ssl_cert_dir: None,
            },
            // Scenario 2: with SSL_CERT_DIR
            TermuxBaseEnvInputs {
                compat_dir: OsStr::new("/test/compat"),
                prefix_bin_dir: OsStr::new("/test/prefix/bin"),
                temp_dir: OsStr::new("/test/tmp"),
                cert_file: OsStr::new("/test/cert.pem"),
                cert_dir: Some(OsStr::new("/test/certs")),
                inherited_path: Some(OsStr::new("/bin")),
                inherited_ssl_cert_file: Some(OsStr::new("/inherited/cert.pem")),
                inherited_ssl_cert_dir: Some(OsStr::new("/inherited/certs")),
            },
        ];

        for inputs in scenarios {
            let plan = plan_termux_base_env(&inputs).expect("plan must succeed");

            for key in excluded_keys {
                assert!(
                    !plan.contains_key(key),
                    "plan must NOT contain excluded key '{key}'"
                );
                assert!(
                    !plan.assignments().iter().any(|(k, _)| k == key),
                    "plan assignments must NOT contain excluded key '{key}'"
                );
            }

            // Assert only the exact allowed keys are present
            let allowed_keys: &[&str] =
                if inputs.cert_dir.is_some() || inputs.inherited_ssl_cert_dir.is_some() {
                    &[
                        "TMPDIR",
                        "TMP",
                        "TEMP",
                        "SQLITE_TMPDIR",
                        "SSL_CERT_FILE",
                        "SSL_CERT_DIR",
                        "PATH",
                    ]
                } else {
                    &[
                        "TMPDIR",
                        "TMP",
                        "TEMP",
                        "SQLITE_TMPDIR",
                        "SSL_CERT_FILE",
                        "PATH",
                    ]
                };

            for (k, _) in plan.assignments() {
                let k_str = k.to_str().expect("valid key utf-8");
                assert!(
                    allowed_keys.contains(&k_str),
                    "unexpected key '{k_str}' found in planned assignments"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b8_i_planner_purity() {
        let inputs_a = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/purity/compat"),
            prefix_bin_dir: OsStr::new("/purity/prefix/bin"),
            temp_dir: OsStr::new("/purity/tmp"),
            cert_file: OsStr::new("/purity/cert.pem"),
            cert_dir: Some(OsStr::new("/purity/certs")),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        // 1. Multiple executions produce identical results (determinism)
        let plan_1 = plan_termux_base_env(&inputs_a).expect("plan 1");
        let plan_2 = plan_termux_base_env(&inputs_a).expect("plan 2");
        let plan_3 = plan_termux_base_env(&inputs_a).expect("plan 3");
        assert_eq!(plan_1, plan_2);
        assert_eq!(plan_2, plan_3);

        // 2. Deterministic input/output: changing a single input changes only the corresponding output
        let mut inputs_b = inputs_a.clone();
        inputs_b.temp_dir = OsStr::new("/other/isolated/tmp");
        let plan_b = plan_termux_base_env(&inputs_b).expect("plan b");
        assert_eq!(
            plan_b.get("TMPDIR"),
            Some(OsStr::new("/other/isolated/tmp"))
        );
        assert_eq!(plan_b.get("TMP"), Some(OsStr::new("/other/isolated/tmp")));
        assert_eq!(plan_b.get("TEMP"), Some(OsStr::new("/other/isolated/tmp")));
        assert_eq!(
            plan_b.get("SQLITE_TMPDIR"),
            Some(OsStr::new("/other/isolated/tmp"))
        );
        assert_eq!(plan_b.get("SSL_CERT_FILE"), plan_1.get("SSL_CERT_FILE"));
        assert_eq!(plan_b.get("SSL_CERT_DIR"), plan_1.get("SSL_CERT_DIR"));
        assert_eq!(plan_b.get("PATH"), plan_1.get("PATH"));

        // 3. When inherited_path is None, planned PATH has only explicit components,
        // proving no reading of ambient PATH.
        assert_eq!(
            plan_1.get("PATH"),
            Some(OsStr::new("/purity/compat:/purity/prefix/bin"))
        );

        // 4. Repeated planning on mutated input is also deterministic
        let plan_b_repeat = plan_termux_base_env(&inputs_b).expect("plan b repeat");
        assert_eq!(plan_b, plan_b_repeat);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b10_a_exact_prefix_bin_derivation_and_assignment_order() {
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(OsString::from("/synthetic/prefix-root")),
            tmpdir: Some(OsString::from("/synthetic/tmp")),
            inherited_path: Some(OsString::from("/inherited/one:/inherited/two")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let plan = plan_termux_base_env_from_snapshot(
            &snapshot,
            OsStr::new("/selected/compat"),
            OsStr::new("/selected/tls/cert.pem"),
            Some(OsStr::new("/selected/tls/certs")),
        )
        .expect("snapshot plan must succeed");

        let expected = vec![
            (OsString::from("TMPDIR"), OsString::from("/synthetic/tmp")),
            (OsString::from("TMP"), OsString::from("/synthetic/tmp")),
            (OsString::from("TEMP"), OsString::from("/synthetic/tmp")),
            (
                OsString::from("SQLITE_TMPDIR"),
                OsString::from("/synthetic/tmp"),
            ),
            (
                OsString::from("SSL_CERT_FILE"),
                OsString::from("/selected/tls/cert.pem"),
            ),
            (
                OsString::from("SSL_CERT_DIR"),
                OsString::from("/selected/tls/certs"),
            ),
            (
                OsString::from("PATH"),
                OsString::from(
                    "/selected/compat:/synthetic/prefix-root/bin:/inherited/one:/inherited/two",
                ),
            ),
        ];
        assert_eq!(plan.into_assignments(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b10_b_required_prefix_and_tmpdir_errors_are_typed() {
        let base = TermuxProcessEnvSnapshot {
            prefix: Some(OsString::from("/prefix")),
            tmpdir: Some(OsString::from("/tmp")),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let mut missing_prefix = base.clone();
        missing_prefix.prefix = None;
        assert_eq!(
            plan_termux_base_env_from_snapshot(
                &missing_prefix,
                OsStr::new("/compat"),
                OsStr::new("/cert.pem"),
                None,
            ),
            Err(TermuxProcessEnvError::MissingRequired("PREFIX"))
        );

        let mut empty_prefix = base.clone();
        empty_prefix.prefix = Some(OsString::new());
        assert_eq!(
            plan_termux_base_env_from_snapshot(
                &empty_prefix,
                OsStr::new("/compat"),
                OsStr::new("/cert.pem"),
                None,
            ),
            Err(TermuxProcessEnvError::EmptyRequired("PREFIX"))
        );

        let mut missing_tmpdir = base.clone();
        missing_tmpdir.tmpdir = None;
        assert_eq!(
            plan_termux_base_env_from_snapshot(
                &missing_tmpdir,
                OsStr::new("/compat"),
                OsStr::new("/cert.pem"),
                None,
            ),
            Err(TermuxProcessEnvError::MissingRequired("TMPDIR"))
        );

        let mut empty_tmpdir = base;
        empty_tmpdir.tmpdir = Some(OsString::new());
        assert_eq!(
            plan_termux_base_env_from_snapshot(
                &empty_tmpdir,
                OsStr::new("/compat"),
                OsStr::new("/cert.pem"),
                None,
            ),
            Err(TermuxProcessEnvError::EmptyRequired("TMPDIR"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b10_c_raw_non_utf8_inherited_values_are_preserved() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let raw_path = b"/raw/one\xff:/raw/two\x80".to_vec();
        let raw_cert_file = b"/raw/cert\xfe.pem".to_vec();
        let raw_cert_dir = b"/raw/certs\xfd".to_vec();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(OsString::from("/prefix")),
            tmpdir: Some(OsString::from("/tmp")),
            inherited_path: Some(OsString::from_vec(raw_path.clone())),
            inherited_ssl_cert_file: Some(OsString::from_vec(raw_cert_file.clone())),
            inherited_ssl_cert_dir: Some(OsString::from_vec(raw_cert_dir.clone())),
        };
        let plan = plan_termux_base_env_from_snapshot(
            &snapshot,
            OsStr::new("/compat"),
            OsStr::new("/fallback/cert.pem"),
            Some(OsStr::new("/fallback/certs")),
        )
        .expect("raw snapshot plan must succeed");

        assert_eq!(
            plan.get("SSL_CERT_FILE").map(OsStrExt::as_bytes),
            Some(raw_cert_file.as_slice())
        );
        assert_eq!(
            plan.get("SSL_CERT_DIR").map(OsStrExt::as_bytes),
            Some(raw_cert_dir.as_slice())
        );
        let path = plan.get("PATH").expect("PATH must exist").as_bytes();
        let mut expected_path = b"/compat:/prefix/bin:".to_vec();
        expected_path.extend_from_slice(&raw_path);
        assert_eq!(path, expected_path.as_slice());
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b10_d_unusual_prefix_derives_only_native_bin_without_fixed_root() {
        use std::os::unix::ffi::OsStrExt;

        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(OsString::from("/odd/distribution/root_77/prefix")),
            tmpdir: Some(OsString::from("/volatile/run/tmp")),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let plan = plan_termux_base_env_from_snapshot(
            &snapshot,
            OsStr::new("/selected/compat"),
            OsStr::new("/selected/cert.pem"),
            None,
        )
        .expect("unusual prefix plan must succeed");

        assert_eq!(
            plan.get("PATH"),
            Some(OsStr::new(
                "/selected/compat:/odd/distribution/root_77/prefix/bin"
            ))
        );
        for (_, value) in plan.assignments() {
            let bytes = value.as_bytes();
            assert!(!bytes
                .windows(b"/data/data".len())
                .any(|w| w == b"/data/data"));
            assert!(!bytes
                .windows(b"com.termux".len())
                .any(|w| w == b"com.termux"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b10_e_base_env_errors_remain_typed() {
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(OsString::from("/prefix:invalid")),
            tmpdir: Some(OsString::from("/tmp")),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        assert_eq!(
            plan_termux_base_env_from_snapshot(
                &snapshot,
                OsStr::new("/compat"),
                OsStr::new("/cert.pem"),
                None,
            ),
            Err(TermuxProcessEnvError::BaseEnv(
                TermuxBaseEnvError::ColonInPathComponent("prefix_bin_dir")
            ))
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b10_f_process_capture_matches_exact_five_ambient_values_read_only() {
        let before = [
            ("PREFIX", std::env::var_os("PREFIX")),
            ("TMPDIR", std::env::var_os("TMPDIR")),
            ("PATH", std::env::var_os("PATH")),
            ("SSL_CERT_FILE", std::env::var_os("SSL_CERT_FILE")),
            ("SSL_CERT_DIR", std::env::var_os("SSL_CERT_DIR")),
        ];
        let snapshot = capture_termux_process_env();
        let after = [
            ("PREFIX", std::env::var_os("PREFIX")),
            ("TMPDIR", std::env::var_os("TMPDIR")),
            ("PATH", std::env::var_os("PATH")),
            ("SSL_CERT_FILE", std::env::var_os("SSL_CERT_FILE")),
            ("SSL_CERT_DIR", std::env::var_os("SSL_CERT_DIR")),
        ];

        assert_eq!(before, after, "capture must not mutate process environment");
        assert_eq!(snapshot.prefix, before[0].1);
        assert_eq!(snapshot.tmpdir, before[1].1);
        assert_eq!(snapshot.inherited_path, before[2].1);
        assert_eq!(snapshot.inherited_ssl_cert_file, before[3].1);
        assert_eq!(snapshot.inherited_ssl_cert_dir, before[4].1);
    }

    fn m1_b11_valid_manifest() -> GenerationManifest {
        GenerationManifest {
            upstream_package_identity: "@openai/codex".to_string(),
            upstream_package_version: "0.0.0-test".to_string(),
            source_artifact_digest: "opaque-source-digest:v1:001122".to_string(),
            expected_platform: "aarch64-linux-android".to_string(),
            expected_architecture: "aarch64".to_string(),
            patch_policy_id: "termux-policy-v1".to_string(),
            patch_report: "opaque patch report: source occurrences verified".to_string(),
            runtime_digest: "opaque-runtime-digest:v1:aabbcc".to_string(),
            helper_digests: vec![
                GenerationHelperDigest {
                    identity: "compat-helper".to_string(),
                    digest: "opaque-helper-digest:01".to_string(),
                },
                GenerationHelperDigest {
                    identity: "runtime-helper".to_string(),
                    digest: "opaque-helper-digest:02".to_string(),
                },
            ],
            core_artifact_digest: "opaque-core-digest:v1:334455".to_string(),
            manager_artifact_digest: None,
            core_api_identity: "core-api-test-v1".to_string(),
            persistent_schema_identity: "core-schema-test-v1".to_string(),
            qualification: GenerationQualification::Qualified,
            creation_metadata: "created-by-test;opaque=true".to_string(),
        }
    }

    fn m1_b11_requirements() -> GenerationManifestRequirements<'static> {
        GenerationManifestRequirements {
            platform: "aarch64-linux-android",
            architecture: "aarch64",
            core_api_identity: "core-api-test-v1",
            persistent_schema_identity: "core-schema-test-v1",
        }
    }

    #[test]
    fn test_m1_b11_a_valid_manifest_returns_borrowed_qualified_wrapper() {
        let manifest = m1_b11_valid_manifest();
        let qualified = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("valid manifest must qualify");
        assert!(std::ptr::eq(qualified.manifest(), &manifest));
        assert_eq!(qualified.manifest(), &manifest);
    }

    #[test]
    fn test_m1_b11_b_each_compatibility_mismatch_is_rejected() {
        let requirements = m1_b11_requirements();

        let mut manifest = m1_b11_valid_manifest();
        manifest.expected_platform = "other-platform".to_string();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::PlatformMismatch
        );

        let mut manifest = m1_b11_valid_manifest();
        manifest.expected_architecture = "x86_64".to_string();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::ArchitectureMismatch
        );

        let mut manifest = m1_b11_valid_manifest();
        manifest.core_api_identity = "other-api".to_string();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::CoreApiMismatch
        );

        let mut manifest = m1_b11_valid_manifest();
        manifest.persistent_schema_identity = "other-schema".to_string();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::PersistentSchemaMismatch
        );
    }

    #[test]
    fn test_m1_b11_c_rejected_qualification_does_not_promote_manifest() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.qualification = GenerationQualification::Rejected;
        assert_eq!(
            qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap_err(),
            GenerationManifestError::RejectedQualification
        );
    }

    #[test]
    fn test_m1_b11_d_required_manifest_bindings_reject_empty_representatives() {
        let requirements = m1_b11_requirements();
        let cases: [(&str, fn(&mut GenerationManifest)); 7] = [
            ("upstream_package_identity", |m| {
                m.upstream_package_identity.clear()
            }),
            ("source_artifact_digest", |m| {
                m.source_artifact_digest.clear()
            }),
            ("expected_platform", |m| m.expected_platform.clear()),
            ("patch_report", |m| m.patch_report.clear()),
            ("runtime_digest", |m| m.runtime_digest.clear()),
            ("core_artifact_digest", |m| m.core_artifact_digest.clear()),
            ("creation_metadata", |m| m.creation_metadata.clear()),
        ];

        for (field, clear) in cases {
            let mut manifest = m1_b11_valid_manifest();
            clear(&mut manifest);
            assert_eq!(
                qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
                GenerationManifestError::EmptyRequired(field),
                "field {field} must be required"
            );
        }
    }

    #[test]
    fn test_m1_b11_e_all_other_required_scalar_bindings_are_enforced() {
        let requirements = m1_b11_requirements();
        let cases: [(&str, fn(&mut GenerationManifest)); 5] = [
            ("upstream_package_version", |m| {
                m.upstream_package_version.clear()
            }),
            ("expected_architecture", |m| m.expected_architecture.clear()),
            ("patch_policy_id", |m| m.patch_policy_id.clear()),
            ("core_api_identity", |m| m.core_api_identity.clear()),
            ("persistent_schema_identity", |m| {
                m.persistent_schema_identity.clear()
            }),
        ];

        for (field, clear) in cases {
            let mut manifest = m1_b11_valid_manifest();
            clear(&mut manifest);
            assert_eq!(
                qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
                GenerationManifestError::EmptyRequired(field),
                "field {field} must be required"
            );
        }
    }

    #[test]
    fn test_m1_b11_f_helper_bindings_are_complete_and_unique() {
        let requirements = m1_b11_requirements();

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests[0].identity.clear();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::EmptyHelperIdentity(0)
        );

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests[1].digest.clear();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::EmptyHelperDigest(1)
        );

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests[1].identity = manifest.helper_digests[0].identity.clone();
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::DuplicateHelperIdentity {
                first: 0,
                duplicate: 1,
            }
        );

        let mut no_helpers = m1_b11_valid_manifest();
        no_helpers.helper_digests.clear();
        qualify_generation_manifest(&no_helpers, &requirements)
            .expect("zero helpers is an explicit valid manifest shape");
    }

    #[test]
    fn test_m1_b11_g_optional_manager_digest_absent_or_nonempty_only() {
        let requirements = m1_b11_requirements();

        let absent = m1_b11_valid_manifest();
        qualify_generation_manifest(&absent, &requirements)
            .expect("absent optional Manager digest must be accepted");

        let mut present = m1_b11_valid_manifest();
        present.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        qualify_generation_manifest(&present, &requirements)
            .expect("non-empty optional Manager digest must be accepted");

        let mut empty = m1_b11_valid_manifest();
        empty.manager_artifact_digest = Some(String::new());
        assert_eq!(
            qualify_generation_manifest(&empty, &requirements).unwrap_err(),
            GenerationManifestError::EmptyManagerDigest
        );
    }

    #[test]
    fn test_m1_b11_h_opaque_non_ascii_values_are_retained_exactly() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.patch_report = "패치 보고서 :: Δ ::  그대로  ".to_string();
        manifest.creation_metadata = "생성 메타데이터=値; spaces  preserved".to_string();
        manifest.helper_digests[0].digest = "opaque:다이제스트:ß:001".to_string();
        let patch_ptr = manifest.patch_report.as_ptr();
        let metadata_ptr = manifest.creation_metadata.as_ptr();

        let qualified = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("opaque manifest must qualify");
        assert_eq!(
            qualified.manifest().patch_report,
            "패치 보고서 :: Δ ::  그대로  "
        );
        assert_eq!(
            qualified.manifest().creation_metadata,
            "생성 메타데이터=値; spaces  preserved"
        );
        assert_eq!(
            qualified.manifest().helper_digests[0].digest,
            "opaque:다이제스트:ß:001"
        );
        assert_eq!(qualified.manifest().patch_report.as_ptr(), patch_ptr);
        assert_eq!(
            qualified.manifest().creation_metadata.as_ptr(),
            metadata_ptr
        );
    }

    #[test]
    fn test_m1_b11_i_empty_validation_requirements_fail_closed() {
        let manifest = m1_b11_valid_manifest();
        let mut requirements = m1_b11_requirements();
        requirements.platform = "";
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::EmptyRequirement("platform")
        );

        let mut requirements = m1_b11_requirements();
        requirements.architecture = "";
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::EmptyRequirement("architecture")
        );

        let mut requirements = m1_b11_requirements();
        requirements.core_api_identity = "";
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::EmptyRequirement("core_api_identity")
        );

        let mut requirements = m1_b11_requirements();
        requirements.persistent_schema_identity = "";
        assert_eq!(
            qualify_generation_manifest(&manifest, &requirements).unwrap_err(),
            GenerationManifestError::EmptyRequirement("persistent_schema_identity")
        );
    }

    #[test]
    fn test_m1_b11_j_validator_is_deterministic_and_has_no_process_env_side_effect() {
        let manifest = m1_b11_valid_manifest();
        let requirements = m1_b11_requirements();
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];

        let first = qualify_generation_manifest(&manifest, &requirements)
            .expect("first qualification must succeed");
        let second = qualify_generation_manifest(&manifest, &requirements)
            .expect("second qualification must succeed");

        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        assert_eq!(before, after);
        assert!(std::ptr::eq(first.manifest(), second.manifest()));
        assert!(std::ptr::eq(first.manifest(), &manifest));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_a_valid_multi_helper_assets_are_qualified_and_borrowed() {
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let helpers = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/generation/helpers/compat-helper"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "runtime-helper",
                asset_path: OsStr::new("/generation/helpers/runtime-helper"),
                observed_digest: "opaque-helper-digest:02",
            },
        ];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/generation/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/generation/compat/bin"),
            helpers: &helpers,
        };

        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("valid runtime assets must qualify");
        assert!(std::ptr::eq(qualified.selection(), &selection));
        assert!(std::ptr::eq(qualified.generation().manifest(), &manifest));
        assert!(std::ptr::eq(
            qualified.selection().helpers,
            helpers.as_slice()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_b_zero_helpers_requires_zero_helper_manifest() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("zero-helper manifest must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/generation/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/generation/compat/bin"),
            helpers: &[],
        };
        qualify_runtime_assets(generation, &selection)
            .expect("zero selected helpers must match zero-helper manifest");
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_c_runtime_path_shape_fails_closed() {
        use std::os::unix::ffi::OsStrExt;
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let helpers = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "runtime-helper",
                asset_path: OsStr::new("/h/runtime"),
                observed_digest: "opaque-helper-digest:02",
            },
        ];
        for (path, expected) in [
            (
                OsStr::new(""),
                RuntimeAssetError::EmptyPath("runtime_program"),
            ),
            (
                OsStr::new("relative/codex"),
                RuntimeAssetError::RelativePath("runtime_program"),
            ),
            (
                OsStr::from_bytes(b"/runtime/co\0dex"),
                RuntimeAssetError::NulPath("runtime_program"),
            ),
        ] {
            let selection = RuntimeAssetSelection {
                runtime: RuntimeAssetBinding {
                    program_path: path,
                    observed_digest: "opaque-runtime-digest:v1:aabbcc",
                },
                compatibility_dir: OsStr::new("/compat/bin"),
                helpers: &helpers,
            };
            assert_eq!(
                qualify_runtime_assets(generation, &selection).unwrap_err(),
                expected
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_d_compatibility_directory_shape_fails_closed() {
        use std::os::unix::ffi::OsStrExt;
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let helpers = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "runtime-helper",
                asset_path: OsStr::new("/h/runtime"),
                observed_digest: "opaque-helper-digest:02",
            },
        ];
        let cases = [
            (
                OsStr::new(""),
                RuntimeAssetError::EmptyPath("compatibility_dir"),
            ),
            (
                OsStr::new("relative/compat"),
                RuntimeAssetError::RelativePath("compatibility_dir"),
            ),
            (
                OsStr::from_bytes(b"/compat/bi\0n"),
                RuntimeAssetError::NulPath("compatibility_dir"),
            ),
            (
                OsStr::new("/compat:/other"),
                RuntimeAssetError::CompatibilityPath(TermuxBaseEnvError::ColonInPathComponent(
                    "compat_dir",
                )),
            ),
        ];
        for (compatibility_dir, expected) in cases {
            let selection = RuntimeAssetSelection {
                runtime: RuntimeAssetBinding {
                    program_path: OsStr::new("/runtime/codex"),
                    observed_digest: "opaque-runtime-digest:v1:aabbcc",
                },
                compatibility_dir,
                helpers: &helpers,
            };
            assert_eq!(
                qualify_runtime_assets(generation, &selection).unwrap_err(),
                expected
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_e_runtime_digest_is_required_and_manifest_bound() {
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let helpers = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "runtime-helper",
                asset_path: OsStr::new("/h/runtime"),
                observed_digest: "opaque-helper-digest:02",
            },
        ];
        for (digest, expected) in [
            ("", RuntimeAssetError::EmptyRuntimeDigest),
            (
                "different-runtime-digest",
                RuntimeAssetError::RuntimeDigestMismatch,
            ),
        ] {
            let selection = RuntimeAssetSelection {
                runtime: RuntimeAssetBinding {
                    program_path: OsStr::new("/runtime/codex"),
                    observed_digest: digest,
                },
                compatibility_dir: OsStr::new("/compat/bin"),
                helpers: &helpers,
            };
            assert_eq!(
                qualify_runtime_assets(generation, &selection).unwrap_err(),
                expected
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_f_helper_binding_shape_fails_closed() {
        use std::os::unix::ffi::OsStrExt;
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let cases = [
            HelperAssetBinding {
                identity: "",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new(""),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("relative/helper"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::from_bytes(b"/h/co\0mpat"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "",
            },
        ];
        let expected = [
            RuntimeAssetError::EmptyHelperIdentity(0),
            RuntimeAssetError::EmptyPath("helper_asset"),
            RuntimeAssetError::RelativePath("helper_asset"),
            RuntimeAssetError::NulPath("helper_asset"),
            RuntimeAssetError::EmptyHelperDigest(0),
        ];
        for (first, expected) in cases.into_iter().zip(expected) {
            let helpers = [
                first,
                HelperAssetBinding {
                    identity: "runtime-helper",
                    asset_path: OsStr::new("/h/runtime"),
                    observed_digest: "opaque-helper-digest:02",
                },
            ];
            let selection = RuntimeAssetSelection {
                runtime: RuntimeAssetBinding {
                    program_path: OsStr::new("/runtime/codex"),
                    observed_digest: "opaque-runtime-digest:v1:aabbcc",
                },
                compatibility_dir: OsStr::new("/compat/bin"),
                helpers: &helpers,
            };
            assert_eq!(
                qualify_runtime_assets(generation, &selection).unwrap_err(),
                expected
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_g_helper_set_must_match_manifest_exactly() {
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");

        let missing = [HelperAssetBinding {
            identity: "compat-helper",
            asset_path: OsStr::new("/h/compat"),
            observed_digest: "opaque-helper-digest:01",
        }];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/compat/bin"),
            helpers: &missing,
        };
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::MissingHelperIdentity(1)
        );

        let extra = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "runtime-helper",
                asset_path: OsStr::new("/h/runtime"),
                observed_digest: "opaque-helper-digest:02",
            },
            HelperAssetBinding {
                identity: "unexpected-helper",
                asset_path: OsStr::new("/h/extra"),
                observed_digest: "opaque-extra-digest",
            },
        ];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/compat/bin"),
            helpers: &extra,
        };
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::ExtraHelperIdentity(2)
        );

        let duplicate = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat-a"),
                observed_digest: "opaque-helper-digest:01",
            },
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat-b"),
                observed_digest: "opaque-helper-digest:01",
            },
        ];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/compat/bin"),
            helpers: &duplicate,
        };
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::DuplicateHelperIdentity {
                first: 0,
                duplicate: 1,
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_h_helper_digest_mismatch_is_rejected() {
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let helpers = [
            HelperAssetBinding {
                identity: "compat-helper",
                asset_path: OsStr::new("/h/compat"),
                observed_digest: "wrong-digest",
            },
            HelperAssetBinding {
                identity: "runtime-helper",
                asset_path: OsStr::new("/h/runtime"),
                observed_digest: "opaque-helper-digest:02",
            },
        ];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/compat/bin"),
            helpers: &helpers,
        };
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::HelperDigestMismatch(0)
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_i_raw_non_utf8_paths_are_retained_exactly() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let runtime_raw = b"/runtime/co\xffdex".to_vec();
        let compat_raw = b"/compat/bi\x80n".to_vec();
        let runtime_path = OsString::from_vec(runtime_raw.clone());
        let compat_dir = OsString::from_vec(compat_raw.clone());
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime_path.as_os_str(),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: compat_dir.as_os_str(),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("raw non-UTF8 absolute paths must qualify");
        assert_eq!(
            qualified.selection().runtime.program_path.as_bytes(),
            runtime_raw.as_slice()
        );
        assert_eq!(
            qualified.selection().compatibility_dir.as_bytes(),
            compat_raw.as_slice()
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b13_j_qualification_is_deterministic_and_has_no_environment_side_effect() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/runtime/codex"),
                observed_digest: "opaque-runtime-digest:v1:aabbcc",
            },
            compatibility_dir: OsStr::new("/compat/bin"),
            helpers: &[],
        };
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        let first = qualify_runtime_assets(generation, &selection).expect("first qualification");
        let second = qualify_runtime_assets(generation, &selection).expect("second qualification");
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        assert_eq!(before, after);
        assert!(std::ptr::eq(first.selection(), second.selection()));
        assert!(std::ptr::eq(first.selection(), &selection));
    }

    fn m1_b12_remote_request() -> UpdateRequest<'static> {
        UpdateRequest {
            source: UpdateArtifactSource::Remote {
                immutable_locator: "release://immutable/codex/aarch64/001",
            },
            evidence: UpdateAdmissionEvidence {
                signed_release_manifest_identity: "signed-release:stable:001",
                expected_source_artifact_digest: "opaque-source-digest:v1:001122",
                release_signature: UpdateEvidenceVerdict::Satisfied,
                architecture_policy: UpdateEvidenceVerdict::Satisfied,
                core_api_policy: UpdateEvidenceVerdict::Satisfied,
                channel_policy: UpdateEvidenceVerdict::Satisfied,
                anti_rollback_policy: UpdateEvidenceVerdict::Satisfied,
                resolver_dependency: UpdaterResolverDependency::Independent,
            },
        }
    }

    fn m1_b12_staged_ok() -> StagedArtifactEvidence {
        StagedArtifactEvidence {
            artifact_digest: UpdateEvidenceVerdict::Satisfied,
            archive_safety: UpdateEvidenceVerdict::Satisfied,
            compatibility_metadata: UpdateEvidenceVerdict::Satisfied,
        }
    }

    fn m1_b12_readiness_ok() -> CandidateReadinessEvidence {
        CandidateReadinessEvidence {
            candidate_probe: UpdateEvidenceVerdict::Satisfied,
            rollback_state_ready: UpdateEvidenceVerdict::Satisfied,
        }
    }

    #[test]
    fn test_m1_b12_a_valid_remote_and_local_requests_are_admitted() {
        let remote = m1_b12_remote_request();
        let admitted = admit_update_request(&remote).expect("remote request must be admitted");
        assert!(std::ptr::eq(admitted.request(), &remote));

        let local = UpdateRequest {
            source: UpdateArtifactSource::LocalArtifact {
                path: OsStr::new("/explicit/local/release.tar.zst"),
            },
            ..remote
        };
        let admitted = admit_update_request(&local).expect("local request must be admitted");
        assert!(std::ptr::eq(admitted.request(), &local));
    }

    #[test]
    fn test_m1_b12_b_empty_release_digest_and_sources_fail_closed() {
        let mut request = m1_b12_remote_request();
        request.evidence.signed_release_manifest_identity = "";
        assert_eq!(
            admit_update_request(&request).unwrap_err(),
            UpdateInterfaceError::EmptySignedReleaseManifestIdentity
        );

        let mut request = m1_b12_remote_request();
        request.evidence.expected_source_artifact_digest = "";
        assert_eq!(
            admit_update_request(&request).unwrap_err(),
            UpdateInterfaceError::EmptyExpectedSourceArtifactDigest
        );

        let mut request = m1_b12_remote_request();
        request.source = UpdateArtifactSource::Remote {
            immutable_locator: "",
        };
        assert_eq!(
            admit_update_request(&request).unwrap_err(),
            UpdateInterfaceError::EmptyRemoteLocator
        );

        let mut request = m1_b12_remote_request();
        request.source = UpdateArtifactSource::LocalArtifact {
            path: OsStr::new(""),
        };
        assert_eq!(
            admit_update_request(&request).unwrap_err(),
            UpdateInterfaceError::EmptyLocalArtifactPath
        );
    }

    #[test]
    fn test_m1_b12_c_each_admission_verdict_rejection_is_distinct() {
        let cases: [(
            fn(&mut UpdateAdmissionEvidence<'static>),
            UpdateInterfaceError,
        ); 5] = [
            (
                |e| e.release_signature = UpdateEvidenceVerdict::Rejected,
                UpdateInterfaceError::ReleaseSignatureRejected,
            ),
            (
                |e| e.architecture_policy = UpdateEvidenceVerdict::Rejected,
                UpdateInterfaceError::ArchitecturePolicyRejected,
            ),
            (
                |e| e.core_api_policy = UpdateEvidenceVerdict::Rejected,
                UpdateInterfaceError::CoreApiPolicyRejected,
            ),
            (
                |e| e.channel_policy = UpdateEvidenceVerdict::Rejected,
                UpdateInterfaceError::ChannelPolicyRejected,
            ),
            (
                |e| e.anti_rollback_policy = UpdateEvidenceVerdict::Rejected,
                UpdateInterfaceError::AntiRollbackPolicyRejected,
            ),
        ];

        for (reject, expected) in cases {
            let mut request = m1_b12_remote_request();
            reject(&mut request.evidence);
            assert_eq!(admit_update_request(&request).unwrap_err(), expected);
        }
    }

    #[test]
    fn test_m1_b12_d_shared_resolver_requires_explicit_qualification() {
        let mut request = m1_b12_remote_request();
        request.evidence.resolver_dependency = UpdaterResolverDependency::SharedRuntimeResolver {
            qualification_identity: "",
        };
        assert_eq!(
            admit_update_request(&request).unwrap_err(),
            UpdateInterfaceError::SharedResolverMissingQualification
        );

        request.evidence.resolver_dependency = UpdaterResolverDependency::SharedRuntimeResolver {
            qualification_identity: "resolver-qualification:termux:v1",
        };
        let admitted = admit_update_request(&request)
            .expect("explicit resolver qualification must permit admission");
        assert!(std::ptr::eq(admitted.request(), &request));
    }

    #[test]
    fn test_m1_b12_e_each_staged_artifact_verdict_rejection_is_distinct() {
        let request = m1_b12_remote_request();
        let admitted = admit_update_request(&request).expect("admit request");
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("qualify generation");
        let readiness = m1_b12_readiness_ok();

        let mut staged = m1_b12_staged_ok();
        staged.artifact_digest = UpdateEvidenceVerdict::Rejected;
        assert_eq!(
            qualify_update_candidate(admitted, &staged, generation, &readiness).unwrap_err(),
            UpdateInterfaceError::ArtifactDigestRejected
        );

        let mut staged = m1_b12_staged_ok();
        staged.archive_safety = UpdateEvidenceVerdict::Rejected;
        assert_eq!(
            qualify_update_candidate(admitted, &staged, generation, &readiness).unwrap_err(),
            UpdateInterfaceError::ArchiveSafetyRejected
        );

        let mut staged = m1_b12_staged_ok();
        staged.compatibility_metadata = UpdateEvidenceVerdict::Rejected;
        assert_eq!(
            qualify_update_candidate(admitted, &staged, generation, &readiness).unwrap_err(),
            UpdateInterfaceError::CompatibilityMetadataRejected
        );
    }

    #[test]
    fn test_m1_b12_f_generation_source_digest_must_match_admitted_release() {
        let request = m1_b12_remote_request();
        let admitted = admit_update_request(&request).expect("admit request");
        let mut manifest = m1_b11_valid_manifest();
        manifest.source_artifact_digest = "different-source-digest".to_string();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation itself remains qualified");
        assert_eq!(
            qualify_update_candidate(
                admitted,
                &m1_b12_staged_ok(),
                generation,
                &m1_b12_readiness_ok(),
            )
            .unwrap_err(),
            UpdateInterfaceError::SourceArtifactDigestMismatch
        );
    }

    #[test]
    fn test_m1_b12_g_candidate_probe_failure_blocks_promotion() {
        let request = m1_b12_remote_request();
        let admitted = admit_update_request(&request).expect("admit request");
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("qualify generation");
        let readiness = CandidateReadinessEvidence {
            candidate_probe: UpdateEvidenceVerdict::Rejected,
            rollback_state_ready: UpdateEvidenceVerdict::Satisfied,
        };
        assert_eq!(
            qualify_update_candidate(admitted, &m1_b12_staged_ok(), generation, &readiness)
                .unwrap_err(),
            UpdateInterfaceError::CandidateProbeRejected
        );
    }

    #[test]
    fn test_m1_b12_h_rollback_readiness_failure_blocks_promotion() {
        let request = m1_b12_remote_request();
        let admitted = admit_update_request(&request).expect("admit request");
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("qualify generation");
        let readiness = CandidateReadinessEvidence {
            candidate_probe: UpdateEvidenceVerdict::Satisfied,
            rollback_state_ready: UpdateEvidenceVerdict::Rejected,
        };
        assert_eq!(
            qualify_update_candidate(admitted, &m1_b12_staged_ok(), generation, &readiness)
                .unwrap_err(),
            UpdateInterfaceError::RollbackStateNotReady
        );
    }

    #[test]
    fn test_m1_b12_i_activation_ready_wrapper_retains_exact_opaque_bindings() {
        let request = UpdateRequest {
            source: UpdateArtifactSource::Remote {
                immutable_locator: "opaque://릴리스/Δ/  exact  ",
            },
            evidence: UpdateAdmissionEvidence {
                signed_release_manifest_identity: "서명된-release::値::  exact  ",
                expected_source_artifact_digest: "opaque-source-digest:v1:001122",
                release_signature: UpdateEvidenceVerdict::Satisfied,
                architecture_policy: UpdateEvidenceVerdict::Satisfied,
                core_api_policy: UpdateEvidenceVerdict::Satisfied,
                channel_policy: UpdateEvidenceVerdict::Satisfied,
                anti_rollback_policy: UpdateEvidenceVerdict::Satisfied,
                resolver_dependency: UpdaterResolverDependency::Independent,
            },
        };
        let admitted = admit_update_request(&request).expect("admit opaque request");
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("qualify generation");
        let ready = qualify_update_candidate(
            admitted,
            &m1_b12_staged_ok(),
            generation,
            &m1_b12_readiness_ok(),
        )
        .expect("candidate must become activation-ready");

        assert!(std::ptr::eq(ready.admitted().request(), &request));
        assert!(std::ptr::eq(ready.generation().manifest(), &manifest));
        assert_eq!(
            ready
                .admitted()
                .request()
                .evidence
                .signed_release_manifest_identity,
            "서명된-release::値::  exact  "
        );
        assert_eq!(
            ready.admitted().request().source,
            UpdateArtifactSource::Remote {
                immutable_locator: "opaque://릴리스/Δ/  exact  "
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b12_j_raw_non_utf8_local_artifact_path_is_retained() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let raw = b"/local/release-\xff-\x80.tar".to_vec();
        let path = OsString::from_vec(raw.clone());
        let remote = m1_b12_remote_request();
        let request = UpdateRequest {
            source: UpdateArtifactSource::LocalArtifact {
                path: path.as_os_str(),
            },
            evidence: remote.evidence,
        };
        let admitted = admit_update_request(&request).expect("raw local path must be admitted");
        match admitted.request().source {
            UpdateArtifactSource::LocalArtifact { path } => {
                assert_eq!(path.as_bytes(), raw.as_slice());
            }
            UpdateArtifactSource::Remote { .. } => panic!("expected local source"),
        }
    }

    #[test]
    fn test_m1_b12_k_interface_is_deterministic_and_has_no_environment_side_effect() {
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        let request = m1_b12_remote_request();
        let first = admit_update_request(&request).expect("first admission");
        let second = admit_update_request(&request).expect("second admission");
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("qualify generation");
        let ready1 = qualify_update_candidate(
            first,
            &m1_b12_staged_ok(),
            generation,
            &m1_b12_readiness_ok(),
        )
        .expect("first ready");
        let ready2 = qualify_update_candidate(
            second,
            &m1_b12_staged_ok(),
            generation,
            &m1_b12_readiness_ok(),
        )
        .expect("second ready");
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        assert_eq!(before, after);
        assert!(std::ptr::eq(
            ready1.admitted().request(),
            ready2.admitted().request()
        ));
        assert!(std::ptr::eq(
            ready1.generation().manifest(),
            ready2.generation().manifest()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b9_a_real_exec_composition() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b9-real-exec-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");

        let marker_path = config_dir_path.join("marker.txt");
        std::fs::write(&marker_path, b"CONFIG_DIR_MARKER_CONTENT").expect("write marker");
        let resolver_bytes = b"# synthetic resolv.conf\nnameserver 198.51.100.1\n";
        std::fs::write(&resolver_path, resolver_bytes).expect("write resolver");

        let shell = resolve_test_shell();
        let fake_upstream_path = test_root.join("fake_upstream.sh");

        let script_content = format!(
            r##"#!{}
if [ "$1" != "-c" ]; then
    printf "ARGV_MISMATCH: 1 is '%s', expected '-c'\n" "$1" >&2
    exit 11
fi

if [ "$2" != 'sandbox_mode="danger-full-access"' ]; then
    printf "ARGV_MISMATCH: 2 is '%s', expected 'sandbox_mode=\"danger-full-access\"'\n" "$2" >&2
    exit 12
fi

if [ "$3" != "exec" ]; then
    printf "ARGV_MISMATCH: 3 is '%s', expected 'exec'\n" "$3" >&2
    exit 13
fi

if [ "$4" != "custom_task" ]; then
    printf "ARGV_MISMATCH: 4 is '%s', expected 'custom_task'\n" "$4" >&2
    exit 14
fi

if [ "$5" != "--custom-flag=val1" ]; then
    printf "ARGV_MISMATCH: 5 is '%s', expected '--custom-flag=val1'\n" "$5" >&2
    exit 15
fi

if [ "$6" != "ordinary arg with spaces and =" ]; then
    printf "ARGV_MISMATCH: 6 is '%s', expected 'ordinary arg with spaces and ='\n" "$6" >&2
    exit 16
fi

for a in "$@"; do
    printf "ARG:%s\n" "$a"
done

res_content=""
while IFS= read -r line || [ -n "$line" ]; do
    if [ -z "$res_content" ]; then
        res_content="$line"
    else
        res_content="$res_content
$line"
    fi
done < /proc/self/fd/33

expected_res="# synthetic resolv.conf
nameserver 198.51.100.1"
if [ "$res_content" != "$expected_res" ]; then
    printf "RESOLVER_FD33_MISMATCH: got '%s'\n" "$res_content" >&2
    exit 20
fi

while read -r key val; do
    if [ "$key" = "flags:" ]; then
        case "$val" in
            *0|*4) ;;
            *)
                printf "RESOLVER_FD33_NOT_RDONLY: flags '%s'\n" "$val" >&2
                exit 21
                ;;
        esac
        break
    fi
done < /proc/self/fdinfo/33

if [ ! -d /proc/self/fd/34 ]; then
    printf "CONFIG_FD34_NOT_DIRECTORY\n" >&2
    exit 30
fi
if [ ! -f /proc/self/fd/34/marker.txt ]; then
    printf "CONFIG_FD34_MARKER_MISSING\n" >&2
    exit 31
fi

marker_content=""
while IFS= read -r line || [ -n "$line" ]; do
    if [ -z "$marker_content" ]; then
        marker_content="$line"
    else
        marker_content="$marker_content
$line"
    fi
done < /proc/self/fd/34/marker.txt

if [ "$marker_content" != "CONFIG_DIR_MARKER_CONTENT" ]; then
    printf "CONFIG_FD34_MARKER_MISMATCH: '%s'\n" "$marker_content" >&2
    exit 32
fi

if [ "$TMPDIR" != "/probe/synthetic/isolated/tmp" ]; then
    printf "TMPDIR_MISMATCH: got '%s'\n" "$TMPDIR" >&2
    exit 51
fi
if [ "$TMP" != "/probe/synthetic/isolated/tmp" ]; then
    printf "TMP_MISMATCH: got '%s'\n" "$TMP" >&2
    exit 52
fi
if [ "$TEMP" != "/probe/synthetic/isolated/tmp" ]; then
    printf "TEMP_MISMATCH: got '%s'\n" "$TEMP" >&2
    exit 53
fi
if [ "$SQLITE_TMPDIR" != "/probe/synthetic/isolated/tmp" ]; then
    printf "SQLITE_TMPDIR_MISMATCH: got '%s'\n" "$SQLITE_TMPDIR" >&2
    exit 54
fi
if [ "$SSL_CERT_FILE" != "/probe/synthetic/tls/cert.pem" ]; then
    printf "SSL_CERT_FILE_MISMATCH: got '%s'\n" "$SSL_CERT_FILE" >&2
    exit 55
fi
if [ "$SSL_CERT_DIR" != "/probe/synthetic/tls/certs.d" ]; then
    printf "SSL_CERT_DIR_MISMATCH: got '%s'\n" "$SSL_CERT_DIR" >&2
    exit 56
fi
if [ "$PATH" != "/probe/synthetic/compat/bin:/probe/synthetic/prefix/bin:/probe/inherited/bin1:/probe/inherited/bin2" ]; then
    printf "PATH_MISMATCH: got '%s'\n" "$PATH" >&2
    exit 57
fi

if [ -n "${{CODEX_MANAGED_BY_NPM+x}}" ]; then
    printf "ENV_FENCE_FAILED: CODEX_MANAGED_BY_NPM is present: %s\n" "$CODEX_MANAGED_BY_NPM" >&2
    exit 40
fi
if [ -n "${{CODEX_MANAGED_BY_BUN+x}}" ]; then
    printf "ENV_FENCE_FAILED: CODEX_MANAGED_BY_BUN is present: %s\n" "$CODEX_MANAGED_BY_BUN" >&2
    exit 41
fi
if [ -n "${{CODEX_MANAGED_PACKAGE_ROOT+x}}" ]; then
    printf "ENV_FENCE_FAILED: CODEX_MANAGED_PACKAGE_ROOT is present: %s\n" "$CODEX_MANAGED_PACKAGE_ROOT" >&2
    exit 42
fi
if [ -n "${{LD_PRELOAD+x}}" ]; then
    printf "ENV_FENCE_FAILED: LD_PRELOAD is present: %s\n" "$LD_PRELOAD" >&2
    exit 43
fi
if [ -n "${{LD_LIBRARY_PATH+x}}" ]; then
    printf "ENV_FENCE_FAILED: LD_LIBRARY_PATH is present: %s\n" "$LD_LIBRARY_PATH" >&2
    exit 44
fi

if [ "$CODEX_TEST_UNRELATED_M1_B9_SURVIVING_VAR" != "m1_b9_surviving_exact_value_77192" ]; then
    printf "UNRELATED_ENV_MISMATCH: got '%s'\n" "$CODEX_TEST_UNRELATED_M1_B9_SURVIVING_VAR" >&2
    exit 50
fi

printf "M1_B9_REAL_EXEC_SUCCESS\n"
exit 0
"##,
            shell.to_str().expect("valid shell path")
        );

        std::fs::write(&fake_upstream_path, script_content).expect("write fake upstream");
        let mut perms = std::fs::metadata(&fake_upstream_path)
            .expect("metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_upstream_path, perms).expect("set permissions 0755");

        let result = run_exec_probe_with_env(
            "m1_b9_fake_upstream_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, fake_upstream_path.as_os_str()),
            ],
        );

        assert_eq!(
            result.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stderr, b"");

        let mut expected_stdout = Vec::new();
        expected_stdout.extend_from_slice(b"ARG:-c\n");
        expected_stdout.extend_from_slice(b"ARG:sandbox_mode=\"danger-full-access\"\n");
        expected_stdout.extend_from_slice(b"ARG:exec\n");
        expected_stdout.extend_from_slice(b"ARG:custom_task\n");
        expected_stdout.extend_from_slice(b"ARG:--custom-flag=val1\n");
        expected_stdout.extend_from_slice(b"ARG:ordinary arg with spaces and =\n");
        expected_stdout.extend_from_slice(b"ARG:");
        expected_stdout.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).as_bytes());
        expected_stdout.extend_from_slice(b"\n");
        expected_stdout.extend_from_slice(b"M1_B9_REAL_EXEC_SUCCESS\n");

        assert_eq!(result.stdout, expected_stdout);

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b9_b_failed_exec_preserves_parent_env_and_restores_fds() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b9-fail-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create config dir");
        std::fs::write(&resolver_path, b"nameserver 1.1.1.1\n").expect("write resolver");

        let result_sentinels = run_exec_probe_with_env(
            "m1_b9_failed_exec_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );
        assert_eq!(result_sentinels.status.code(), Some(0));
        assert_eq!(
            result_sentinels.stdout,
            b"M1_B9_FAILED_EXEC_PRESERVED_SUCCESS\n"
        );
        assert_eq!(result_sentinels.stderr, b"");

        let result_absent = run_exec_probe_with_env(
            "m1_b9_failed_exec_absent_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver_path.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir_path.as_os_str()),
            ],
        );
        assert_eq!(result_absent.status.code(), Some(0));
        assert_eq!(result_absent.stdout, b"M1_B9_FAILED_EXEC_ABSENT_SUCCESS\n");
        assert_eq!(result_absent.stderr, b"");

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b9_c_policy_before_io() {
        let nonexistent_prog = OsStr::new("/path/that/does/not/exist/codex-bin-m1-b9-nonexistent");
        let nonexistent_resolver =
            std::path::Path::new("/path/that/does/not/exist/resolv-m1-b9-nonexistent.conf");
        let nonexistent_config =
            std::path::Path::new("/path/that/does/not/exist/config-dir-m1-b9-nonexistent");

        let inputs = TermuxBaseEnvInputs {
            compat_dir: OsStr::new("/test/compat"),
            prefix_bin_dir: OsStr::new("/test/prefix/bin"),
            temp_dir: OsStr::new("/test/tmp"),
            cert_file: OsStr::new("/test/cert.pem"),
            cert_dir: None,
            inherited_path: Some(OsStr::new("/usr/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let plan = plan_termux_base_env(&inputs).expect("plan must succeed");

        let cases: Vec<(Vec<&str>, PassthroughError)> = vec![
            (
                vec!["-s", "read-only"],
                PassthroughError::UnsupportedSandboxMode("read-only".to_string()),
            ),
            (
                vec!["--sandbox", "workspace-write"],
                PassthroughError::UnsupportedSandboxMode("workspace-write".to_string()),
            ),
            (
                vec!["sandbox", "linux"],
                PassthroughError::UnsupportedSandboxSubcommand,
            ),
            (
                vec!["-c", "sandbox_mode=read-only"],
                PassthroughError::UnsupportedSandboxMode("read-only".to_string()),
            ),
        ];

        for (args, expected_policy_err) in cases {
            let err = launch_upstream_with_env(
                nonexistent_prog,
                nonexistent_resolver,
                nonexistent_config,
                args.clone(),
                &plan,
            );

            match err {
                LaunchError::Policy(policy_err) => {
                    assert_eq!(
                        policy_err, expected_policy_err,
                        "expected policy error {:?} for args {:?}",
                        expected_policy_err, args
                    );
                    let msg = policy_err.to_string();
                    assert!(
                        msg.contains("Termux"),
                        "error msg '{msg}' must mention Termux"
                    );
                    assert!(
                        msg.contains("cannot be enforced"),
                        "error msg '{msg}' must mention cannot be enforced"
                    );
                }
                LaunchError::Exec(exec_err) => {
                    panic!(
                        "launch_upstream_with_env must reject policy before I/O, but got Exec error: {exec_err} for args {:?}",
                        args
                    );
                }
            }
        }
    }
}
