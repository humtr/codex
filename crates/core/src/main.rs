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

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicDispatchRoute {
    Update(Vec<OsString>),
    Doctor(Vec<OsString>),
    Termux(Vec<OsString>),
    Upstream(Vec<OsString>),
}

/// Plans public Core interception without executing any route.
///
/// Exact first-token `update`, `doctor`, and `termux` are consumed by Core while
/// every trailing raw argument is retained for the selected handler. Every other
/// shape remains an upstream route with the complete original argv unchanged.
#[allow(dead_code)]
fn plan_public_dispatch<I, S>(args: I) -> PublicDispatchRoute
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let original: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let class = classify_first_arg(original.first().map(OsString::as_os_str));
    match class {
        CommandClass::Update => PublicDispatchRoute::Update(original.into_iter().skip(1).collect()),
        CommandClass::Doctor => PublicDispatchRoute::Doctor(original.into_iter().skip(1).collect()),
        CommandClass::Termux => PublicDispatchRoute::Termux(original.into_iter().skip(1).collect()),
        CommandClass::Passthrough => PublicDispatchRoute::Upstream(original),
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
fn with_runtime_fds<R, C, T, F>(resolver_path: R, config_dir: C, operation: F) -> std::io::Result<T>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
    F: FnOnce() -> std::io::Result<T>,
{
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

    // Once both prior states are captured, keep the operation result separate
    // from restoration. Restoration failure takes precedence over either an
    // operation success or failure.
    let operation_result = (|| -> std::io::Result<T> {
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

        // Close temporary duplicates before running the operation. The mapped
        // FD 33/34 descriptors remain open and non-CLOEXEC for either final exec
        // or a spawned doctor child.
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

        operation()
    })();

    let restore_34 = unsafe { guard_34.restore() };
    let restore_33 = unsafe { guard_33.restore() };
    if let Err(err) = restore_34 {
        return Err(err);
    }
    if let Err(err) = restore_33 {
        return Err(err);
    }
    operation_result
}

#[cfg(unix)]
fn apply_child_env_plan_and_fence(
    cmd: &mut std::process::Command,
    env_plan: Option<&TermuxBaseEnvPlan>,
) {
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
    let result = with_runtime_fds(resolver_path, config_dir, || {
        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new(program.as_ref());
        cmd.args(args);
        apply_child_env_plan_and_fence(&mut cmd, env_plan);
        Err(cmd.exec())
    });

    match result {
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

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManagerArtifactSelection<'a> {
    program_path: &'a OsStr,
    observed_digest: &'a str,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ManagerArtifactError {
    UnexpectedSelection,
    MissingSelection,
    Path(RuntimeAssetError),
    EmptyDigest,
    DigestMismatch,
}

#[cfg(unix)]
impl std::fmt::Display for ManagerArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerArtifactError::UnexpectedSelection => write!(
                f,
                "Manager artifact was selected but the qualified generation declares no Manager"
            ),
            ManagerArtifactError::MissingSelection => write!(
                f,
                "qualified generation declares a Manager artifact but no artifact was selected"
            ),
            ManagerArtifactError::Path(err) => err.fmt(f),
            ManagerArtifactError::EmptyDigest => {
                write!(f, "selected Manager artifact observed digest is empty")
            }
            ManagerArtifactError::DigestMismatch => write!(
                f,
                "selected Manager artifact digest does not match qualified generation"
            ),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for ManagerArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ManagerArtifactError::Path(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct QualifiedManagerArtifact<'selection, 'asset, 'generation> {
    generation: QualifiedGenerationManifest<'generation>,
    selection: &'selection ManagerArtifactSelection<'asset>,
}

#[cfg(unix)]
#[allow(dead_code)]
impl<'selection, 'asset, 'generation> QualifiedManagerArtifact<'selection, 'asset, 'generation> {
    fn generation(self) -> QualifiedGenerationManifest<'generation> {
        self.generation
    }

    fn selection(self) -> &'selection ManagerArtifactSelection<'asset> {
        self.selection
    }
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum ManagerArtifactQualification<'selection, 'asset, 'generation> {
    Unavailable(QualifiedGenerationManifest<'generation>),
    Available(QualifiedManagerArtifact<'selection, 'asset, 'generation>),
}

#[cfg(unix)]
#[allow(dead_code)]
impl<'selection, 'asset, 'generation>
    ManagerArtifactQualification<'selection, 'asset, 'generation>
{
    fn generation(self) -> QualifiedGenerationManifest<'generation> {
        match self {
            ManagerArtifactQualification::Unavailable(generation) => generation,
            ManagerArtifactQualification::Available(qualified) => qualified.generation(),
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
fn qualify_manager_artifact<'selection, 'asset, 'generation>(
    generation: QualifiedGenerationManifest<'generation>,
    selection: Option<&'selection ManagerArtifactSelection<'asset>>,
) -> Result<ManagerArtifactQualification<'selection, 'asset, 'generation>, ManagerArtifactError> {
    let declared_digest = generation.manifest().manager_artifact_digest.as_deref();
    match (declared_digest, selection) {
        (None, None) => Ok(ManagerArtifactQualification::Unavailable(generation)),
        (None, Some(_)) => Err(ManagerArtifactError::UnexpectedSelection),
        (Some(_), None) => Err(ManagerArtifactError::MissingSelection),
        (Some(expected_digest), Some(selection)) => {
            validate_absolute_runtime_asset_path(selection.program_path, "manager_artifact")
                .map_err(ManagerArtifactError::Path)?;
            if selection.observed_digest.is_empty() {
                return Err(ManagerArtifactError::EmptyDigest);
            }
            if selection.observed_digest != expected_digest {
                return Err(ManagerArtifactError::DigestMismatch);
            }
            Ok(ManagerArtifactQualification::Available(
                QualifiedManagerArtifact {
                    generation,
                    selection,
                },
            ))
        }
    }
}

#[cfg(unix)]
const TERMUX_MANAGER_UNAVAILABLE_MESSAGE: &str = "Codex Termux Manager is unavailable.";

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TermuxManagerOutcome {
    Unavailable,
}

#[cfg(unix)]
#[allow(dead_code)]
impl TermuxManagerOutcome {
    fn message(self) -> &'static str {
        match self {
            TermuxManagerOutcome::Unavailable => TERMUX_MANAGER_UNAVAILABLE_MESSAGE,
        }
    }
}

#[cfg(unix)]
#[derive(Debug)]
enum ManagerLaunchError {
    Exec(std::io::Error),
}

#[cfg(unix)]
impl std::fmt::Display for ManagerLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerLaunchError::Exec(err) => {
                write!(f, "failed to execute qualified Manager: {err}")
            }
        }
    }
}

#[cfg(unix)]
impl std::error::Error for ManagerLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ManagerLaunchError::Exec(err) => Some(err),
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
fn execute_termux_manager<'selection, 'asset, 'generation, I, S>(
    qualification: ManagerArtifactQualification<'selection, 'asset, 'generation>,
    args: I,
) -> Result<TermuxManagerOutcome, ManagerLaunchError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    match qualification {
        ManagerArtifactQualification::Unavailable(_) => Ok(TermuxManagerOutcome::Unavailable),
        ManagerArtifactQualification::Available(qualified) => {
            use std::os::unix::process::CommandExt;

            let mut command = std::process::Command::new(qualified.selection().program_path);
            command.args(args);
            Err(ManagerLaunchError::Exec(command.exec()))
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug)]
enum QualifiedRuntimeLaunchError {
    Environment(TermuxProcessEnvError),
    Launch(LaunchError),
}

#[cfg(unix)]
impl std::fmt::Display for QualifiedRuntimeLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedRuntimeLaunchError::Environment(err) => err.fmt(f),
            QualifiedRuntimeLaunchError::Launch(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for QualifiedRuntimeLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QualifiedRuntimeLaunchError::Environment(err) => Some(err),
            QualifiedRuntimeLaunchError::Launch(err) => Some(err),
        }
    }
}

/// Composes a previously qualified runtime selection into the existing final launch path.
///
/// The runtime program and compatibility directory come only from `QualifiedRuntimeAssets`.
/// Process-environment planning is pure and occurs before any resolver/config descriptor I/O.
/// Once it succeeds, the existing launch boundary retains sandbox-policy-before-I/O ordering,
/// FD 33/34 handling, environment fencing, raw argv, and final `exec` process semantics.
#[cfg(unix)]
#[allow(dead_code)]
fn launch_qualified_runtime<'selection, 'asset, 'generation, R, C, I, S>(
    assets: QualifiedRuntimeAssets<'selection, 'asset, 'generation>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    args: I,
) -> QualifiedRuntimeLaunchError
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let selection = assets.selection();
    let env_plan = match plan_termux_base_env_from_snapshot(
        process_env,
        selection.compatibility_dir,
        cert_file,
        cert_dir,
    ) {
        Ok(plan) => plan,
        Err(err) => return QualifiedRuntimeLaunchError::Environment(err),
    };

    QualifiedRuntimeLaunchError::Launch(launch_upstream_with_env(
        selection.runtime.program_path,
        resolver_path,
        config_dir,
        args,
        &env_plan,
    ))
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug)]
enum QualifiedUpstreamDoctorProbeError {
    Environment(TermuxProcessEnvError),
    Policy(PassthroughError),
    Io(std::io::Error),
}

#[cfg(unix)]
impl std::fmt::Display for QualifiedUpstreamDoctorProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedUpstreamDoctorProbeError::Environment(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::Policy(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::Io(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for QualifiedUpstreamDoctorProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QualifiedUpstreamDoctorProbeError::Environment(err) => Some(err),
            QualifiedUpstreamDoctorProbeError::Policy(err) => Some(err),
            QualifiedUpstreamDoctorProbeError::Io(err) => Some(err),
        }
    }
}

/// Runs the supported raw upstream doctor directly as a child of Core.
///
/// Runtime and compatibility authority come only from `QualifiedRuntimeAssets`.
/// The child receives the same B10 environment plan, B3 contamination fence, and
/// FD33/34 runtime contract as final launch. Raw child stdout/stderr are discarded
/// so arbitrary upstream diagnostics cannot bypass the bounded B15 report model.
#[cfg(unix)]
#[allow(dead_code)]
fn probe_qualified_upstream_doctor<'selection, 'asset, 'generation, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset, 'generation>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
) -> Result<UpstreamDoctorStatus, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let selection = assets.selection();
    let env_plan = plan_termux_base_env_from_snapshot(
        process_env,
        selection.compatibility_dir,
        cert_file,
        cert_dir,
    )
    .map_err(QualifiedUpstreamDoctorProbeError::Environment)?;
    let doctor_args = plan_passthrough_args([OsString::from("doctor")])
        .map_err(QualifiedUpstreamDoctorProbeError::Policy)?;

    let status = with_runtime_fds(resolver_path, config_dir, || {
        let mut cmd = std::process::Command::new(selection.runtime.program_path);
        cmd.args(&doctor_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
        cmd.status()
    })
    .map_err(QualifiedUpstreamDoctorProbeError::Io)?;

    Ok(if status.success() {
        UpstreamDoctorStatus::Healthy
    } else {
        UpstreamDoctorStatus::Unhealthy
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpstreamDoctorCapability {
    Supported,
    Unsupported,
}

/// Coordinates the bounded local doctor report without inventing Core or Manager probes.
///
/// A supported upstream invokes the B16 qualified child probe exactly once. An unsupported
/// upstream skips process-environment planning, runtime FD setup, and child execution entirely.
/// Core and Manager states are already-typed caller inputs and are composed through the B15
/// report model without broadening its output vocabulary.
#[cfg(unix)]
#[allow(dead_code)]
fn compose_local_doctor<'selection, 'asset, 'generation, R, C>(
    capability: UpstreamDoctorCapability,
    assets: QualifiedRuntimeAssets<'selection, 'asset, 'generation>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    termux_core: CoreDoctorStatus,
    manager: ManagerDoctorStatus,
) -> Result<DoctorReport, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let upstream = match capability {
        UpstreamDoctorCapability::Supported => probe_qualified_upstream_doctor(
            assets,
            process_env,
            cert_file,
            cert_dir,
            resolver_path,
            config_dir,
        )?,
        UpstreamDoctorCapability::Unsupported => UpstreamDoctorStatus::Unsupported,
    };

    Ok(compose_doctor_report(upstream, termux_core, manager))
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpstreamDoctorStatus {
    Healthy,
    Unhealthy,
    Unsupported,
}

impl UpstreamDoctorStatus {
    fn as_str(self) -> &'static str {
        match self {
            UpstreamDoctorStatus::Healthy => "healthy",
            UpstreamDoctorStatus::Unhealthy => "unhealthy",
            UpstreamDoctorStatus::Unsupported => "unsupported",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreDoctorStatus {
    Healthy,
    Unhealthy,
    ApiIncompatible,
}

impl CoreDoctorStatus {
    fn as_str(self) -> &'static str {
        match self {
            CoreDoctorStatus::Healthy => "healthy",
            CoreDoctorStatus::Unhealthy => "unhealthy",
            CoreDoctorStatus::ApiIncompatible => "api_incompatible",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagerDoctorStatus {
    Healthy,
    Unhealthy,
    Unavailable,
    ApiIncompatible,
}

impl ManagerDoctorStatus {
    fn as_str(self) -> &'static str {
        match self {
            ManagerDoctorStatus::Healthy => "healthy",
            ManagerDoctorStatus::Unhealthy => "unhealthy",
            ManagerDoctorStatus::Unavailable => "unavailable",
            ManagerDoctorStatus::ApiIncompatible => "api_incompatible",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorSummaryStatus {
    Healthy,
    Degraded,
    Unhealthy,
    ApiIncompatible,
}

impl DoctorSummaryStatus {
    fn as_str(self) -> &'static str {
        match self {
            DoctorSummaryStatus::Healthy => "healthy",
            DoctorSummaryStatus::Degraded => "degraded",
            DoctorSummaryStatus::Unhealthy => "unhealthy",
            DoctorSummaryStatus::ApiIncompatible => "api_incompatible",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorExitClass {
    Success,
    HealthFailure,
    ApiIncompatibility,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoctorReport {
    upstream: UpstreamDoctorStatus,
    termux_core: CoreDoctorStatus,
    manager: ManagerDoctorStatus,
    summary: DoctorSummaryStatus,
}

#[allow(dead_code)]
fn compose_doctor_report(
    upstream: UpstreamDoctorStatus,
    termux_core: CoreDoctorStatus,
    manager: ManagerDoctorStatus,
) -> DoctorReport {
    let summary = if termux_core == CoreDoctorStatus::ApiIncompatible
        || manager == ManagerDoctorStatus::ApiIncompatible
    {
        DoctorSummaryStatus::ApiIncompatible
    } else if upstream == UpstreamDoctorStatus::Unhealthy
        || termux_core == CoreDoctorStatus::Unhealthy
        || manager == ManagerDoctorStatus::Unhealthy
    {
        DoctorSummaryStatus::Unhealthy
    } else if upstream == UpstreamDoctorStatus::Unsupported
        || manager == ManagerDoctorStatus::Unavailable
    {
        DoctorSummaryStatus::Degraded
    } else {
        DoctorSummaryStatus::Healthy
    };

    DoctorReport {
        upstream,
        termux_core,
        manager,
        summary,
    }
}

#[allow(dead_code)]
fn doctor_exit_class(report: &DoctorReport) -> DoctorExitClass {
    match report.summary {
        DoctorSummaryStatus::Healthy => DoctorExitClass::Success,
        DoctorSummaryStatus::Degraded | DoctorSummaryStatus::Unhealthy => {
            DoctorExitClass::HealthFailure
        }
        DoctorSummaryStatus::ApiIncompatible => DoctorExitClass::ApiIncompatibility,
    }
}

#[allow(dead_code)]
fn render_doctor_human(report: &DoctorReport) -> String {
    format!(
        "[Upstream]\nstatus: {}\n\n[Termux Core]\nstatus: {}\n\n[Manager]\nstatus: {}\n\n[Summary]\nstatus: {}\n",
        report.upstream.as_str(),
        report.termux_core.as_str(),
        report.manager.as_str(),
        report.summary.as_str(),
    )
}

#[allow(dead_code)]
fn render_doctor_json(report: &DoctorReport) -> String {
    format!(
        "{{\"schema_version\":1,\"upstream\":{{\"status\":\"{}\"}},\"termux_core\":{{\"status\":\"{}\"}},\"manager\":{{\"status\":\"{}\"}},\"summary\":{{\"status\":\"{}\"}}}}\n",
        report.upstream.as_str(),
        report.termux_core.as_str(),
        report.manager.as_str(),
        report.summary.as_str(),
    )
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorOutputMode {
    Human,
    Json,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoctorInvocationPlan {
    output_mode: DoctorOutputMode,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorUsageError {
    InvalidArguments,
}

impl std::fmt::Display for DoctorUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoctorUsageError::InvalidArguments => write!(f, "usage: codex doctor [--json]"),
        }
    }
}

impl std::error::Error for DoctorUsageError {}

/// Plans arguments following the exact leading `doctor` token.
///
/// No trailing argument selects human output; exactly one raw `--json` token selects JSON.
/// Every other shape is rejected without UTF-8 decoding or echoing caller-controlled argv.
#[allow(dead_code)]
fn plan_doctor_invocation<I, S>(args: I) -> Result<DoctorInvocationPlan, DoctorUsageError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    match (args.next(), args.next()) {
        (None, None) => Ok(DoctorInvocationPlan {
            output_mode: DoctorOutputMode::Human,
        }),
        (Some(arg), None) if arg.as_os_str() == OsStr::new("--json") => Ok(DoctorInvocationPlan {
            output_mode: DoctorOutputMode::Json,
        }),
        _ => Err(DoctorUsageError::InvalidArguments),
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorCommandOutcome {
    output: String,
    exit_class: DoctorExitClass,
}

/// Renders one already-composed bounded doctor report according to a validated invocation plan.
#[allow(dead_code)]
fn render_doctor_command(
    plan: DoctorInvocationPlan,
    report: &DoctorReport,
) -> DoctorCommandOutcome {
    let output = match plan.output_mode {
        DoctorOutputMode::Human => render_doctor_human(report),
        DoctorOutputMode::Json => render_doctor_json(report),
    };
    DoctorCommandOutcome {
        output,
        exit_class: doctor_exit_class(report),
    }
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug)]
enum LocalDoctorCommandError {
    Usage(DoctorUsageError),
    Probe(QualifiedUpstreamDoctorProbeError),
}

#[cfg(unix)]
impl std::fmt::Display for LocalDoctorCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalDoctorCommandError::Usage(err) => err.fmt(f),
            LocalDoctorCommandError::Probe(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for LocalDoctorCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocalDoctorCommandError::Usage(err) => Some(err),
            LocalDoctorCommandError::Probe(err) => Some(err),
        }
    }
}

/// Runs one bounded local doctor command with usage validation before any probe I/O.
#[cfg(unix)]
#[allow(dead_code)]
fn run_local_doctor_command<'selection, 'asset, 'generation, I, S, R, C>(
    args: I,
    capability: UpstreamDoctorCapability,
    assets: QualifiedRuntimeAssets<'selection, 'asset, 'generation>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    termux_core: CoreDoctorStatus,
    manager: ManagerDoctorStatus,
) -> Result<DoctorCommandOutcome, LocalDoctorCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let plan = plan_doctor_invocation(args).map_err(LocalDoctorCommandError::Usage)?;
    let report = compose_local_doctor(
        capability,
        assets,
        process_env,
        cert_file,
        cert_dir,
        resolver_path,
        config_dir,
        termux_core,
        manager,
    )
    .map_err(LocalDoctorCommandError::Probe)?;
    Ok(render_doctor_command(plan, &report))
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPublicDispatchContextError {
    GenerationMismatch,
}

#[cfg(unix)]
impl std::fmt::Display for LocalPublicDispatchContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalPublicDispatchContextError::GenerationMismatch => {
                f.write_str("runtime and Manager qualifications come from different generations")
            }
        }
    }
}

#[cfg(unix)]
impl std::error::Error for LocalPublicDispatchContextError {}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct LocalPublicDispatchContext<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
    'generation,
> {
    runtime_assets: QualifiedRuntimeAssets<'runtime_selection, 'runtime_asset, 'generation>,
    manager_artifact: ManagerArtifactQualification<'manager_selection, 'manager_asset, 'generation>,
    process_env: &'context TermuxProcessEnvSnapshot,
    cert_file: &'context OsStr,
    cert_dir: Option<&'context OsStr>,
    resolver_path: &'context std::path::Path,
    config_dir: &'context std::path::Path,
    doctor_capability: UpstreamDoctorCapability,
    core_doctor_status: CoreDoctorStatus,
    manager_doctor_status: ManagerDoctorStatus,
}

#[cfg(unix)]
#[allow(dead_code)]
fn build_local_public_dispatch_context<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
    'generation,
>(
    runtime_assets: QualifiedRuntimeAssets<'runtime_selection, 'runtime_asset, 'generation>,
    manager_artifact: ManagerArtifactQualification<'manager_selection, 'manager_asset, 'generation>,
    process_env: &'context TermuxProcessEnvSnapshot,
    cert_file: &'context OsStr,
    cert_dir: Option<&'context OsStr>,
    resolver_path: &'context std::path::Path,
    config_dir: &'context std::path::Path,
    doctor_capability: UpstreamDoctorCapability,
    core_doctor_status: CoreDoctorStatus,
    manager_doctor_status: ManagerDoctorStatus,
) -> Result<
    LocalPublicDispatchContext<
        'context,
        'runtime_selection,
        'runtime_asset,
        'manager_selection,
        'manager_asset,
        'generation,
    >,
    LocalPublicDispatchContextError,
> {
    if !std::ptr::eq(
        runtime_assets.generation().manifest(),
        manager_artifact.generation().manifest(),
    ) {
        return Err(LocalPublicDispatchContextError::GenerationMismatch);
    }

    Ok(LocalPublicDispatchContext {
        runtime_assets,
        manager_artifact,
        process_env,
        cert_file,
        cert_dir,
        resolver_path,
        config_dir,
        doctor_capability,
        core_doctor_status,
        manager_doctor_status,
    })
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicDispatchCompletion {
    Update(Vec<OsString>),
    Doctor(DoctorCommandOutcome),
    TermuxUnavailable(TermuxManagerOutcome),
}

#[cfg(unix)]
#[derive(Debug)]
enum PublicDispatchExecutionError {
    Upstream(QualifiedRuntimeLaunchError),
    Doctor(LocalDoctorCommandError),
    Manager(ManagerLaunchError),
}

#[cfg(unix)]
impl std::fmt::Display for PublicDispatchExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicDispatchExecutionError::Upstream(err) => err.fmt(f),
            PublicDispatchExecutionError::Doctor(err) => err.fmt(f),
            PublicDispatchExecutionError::Manager(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for PublicDispatchExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublicDispatchExecutionError::Upstream(err) => Some(err),
            PublicDispatchExecutionError::Doctor(err) => Some(err),
            PublicDispatchExecutionError::Manager(err) => Some(err),
        }
    }
}

#[cfg(unix)]
#[allow(dead_code)]
fn execute_public_dispatch<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
    'generation,
>(
    route: PublicDispatchRoute,
    context: LocalPublicDispatchContext<
        'context,
        'runtime_selection,
        'runtime_asset,
        'manager_selection,
        'manager_asset,
        'generation,
    >,
) -> Result<PublicDispatchCompletion, PublicDispatchExecutionError> {
    match route {
        PublicDispatchRoute::Update(args) => Ok(PublicDispatchCompletion::Update(args)),
        PublicDispatchRoute::Doctor(args) => run_local_doctor_command(
            args,
            context.doctor_capability,
            context.runtime_assets,
            context.process_env,
            context.cert_file,
            context.cert_dir,
            context.resolver_path,
            context.config_dir,
            context.core_doctor_status,
            context.manager_doctor_status,
        )
        .map(PublicDispatchCompletion::Doctor)
        .map_err(PublicDispatchExecutionError::Doctor),
        PublicDispatchRoute::Termux(args) => execute_termux_manager(context.manager_artifact, args)
            .map(PublicDispatchCompletion::TermuxUnavailable)
            .map_err(PublicDispatchExecutionError::Manager),
        PublicDispatchRoute::Upstream(args) => Err(PublicDispatchExecutionError::Upstream(
            launch_qualified_runtime(
                context.runtime_assets,
                context.process_env,
                context.cert_file,
                context.cert_dir,
                context.resolver_path,
                context.config_dir,
                args,
            ),
        )),
    }
}

#[cfg(unix)]
#[allow(dead_code)]
mod m2_generation_state {
    use std::io::{Read, Write};

    const GENERATION_ID_MAX_BYTES: usize = 512;
    const STATE_FILE_MAX_BYTES: usize = 16 * 1024;
    const STATE_FORMAT: &str = "codex-activation-state-v1";
    const JOURNAL_FORMAT: &str = "codex-activation-journal-v1";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct CoreStatePaths {
        pub(super) root: std::path::PathBuf,
        pub(super) generations: std::path::PathBuf,
        pub(super) activation_state: std::path::PathBuf,
        pub(super) activation_journal: std::path::PathBuf,
        pub(super) activation_journal_temp: std::path::PathBuf,
        pub(super) activation_state_temp: std::path::PathBuf,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct GenerationPointerState {
        pub(super) current: String,
        pub(super) verified: String,
        pub(super) previous: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct ActivationJournal {
        pub(super) before: Option<GenerationPointerState>,
        pub(super) after: GenerationPointerState,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum StateFormatError {
        EmptyRoot,
        RelativeRoot,
        NulRoot,
        EmptyIdentity(&'static str),
        IdentityTooLong(&'static str),
        IdentityControl(&'static str),
        FileTooLarge(&'static str),
        InvalidUtf8(&'static str),
        MissingFinalNewline(&'static str),
        InvalidRecordCount(&'static str),
        InvalidField(&'static str),
        InvalidPresence(&'static str),
        InconsistentAbsent(&'static str),
        AmbiguousJournal,
        NoRollbackGeneration,
        NoChange,
    }

    impl std::fmt::Display for StateFormatError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                StateFormatError::EmptyRoot => f.write_str("Core state root is empty"),
                StateFormatError::RelativeRoot => {
                    f.write_str("Core state root must be an absolute path")
                }
                StateFormatError::NulRoot => f.write_str("Core state root contains NUL"),
                StateFormatError::EmptyIdentity(field) => {
                    write!(f, "generation identity '{field}' is empty")
                }
                StateFormatError::IdentityTooLong(field) => {
                    write!(f, "generation identity '{field}' exceeds the size limit")
                }
                StateFormatError::IdentityControl(field) => write!(
                    f,
                    "generation identity '{field}' contains a forbidden line/control byte"
                ),
                StateFormatError::FileTooLarge(label) => {
                    write!(f, "{label} exceeds the bounded state-file size")
                }
                StateFormatError::InvalidUtf8(label) => {
                    write!(f, "{label} is not valid UTF-8")
                }
                StateFormatError::MissingFinalNewline(label) => {
                    write!(f, "{label} is missing its canonical final newline")
                }
                StateFormatError::InvalidRecordCount(label) => {
                    write!(f, "{label} has an invalid record count")
                }
                StateFormatError::InvalidField(label) => {
                    write!(f, "{label} has an invalid or out-of-order field")
                }
                StateFormatError::InvalidPresence(label) => {
                    write!(f, "{label} has an invalid presence marker")
                }
                StateFormatError::InconsistentAbsent(label) => {
                    write!(f, "{label} encodes data for an absent value")
                }
                StateFormatError::AmbiguousJournal => {
                    f.write_str("activation journal before/after states are identical")
                }
                StateFormatError::NoRollbackGeneration => {
                    f.write_str("activation state has no rollback generation")
                }
                StateFormatError::NoChange => {
                    f.write_str("activation transition would not change the current generation")
                }
            }
        }
    }

    impl std::error::Error for StateFormatError {}

    #[derive(Debug)]
    pub(super) enum ActivationTransactionError {
        Format(StateFormatError),
        Io {
            operation: &'static str,
            source: std::io::Error,
        },
        UnsafeFileType(&'static str),
        StaleAuthoritativeState,
        PendingJournal,
        OrphanJournalTemporary,
        OrphanTemporaryState,
        RecoveryConflict,
    }

    impl std::fmt::Display for ActivationTransactionError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ActivationTransactionError::Format(err) => err.fmt(f),
                ActivationTransactionError::Io { operation, source } => {
                    write!(f, "{operation} failed: {source}")
                }
                ActivationTransactionError::UnsafeFileType(label) => {
                    write!(f, "{label} has an unsafe file type")
                }
                ActivationTransactionError::StaleAuthoritativeState => f.write_str(
                    "authoritative activation state does not match expected before state",
                ),
                ActivationTransactionError::PendingJournal => {
                    f.write_str("activation journal already exists; recovery is required")
                }
                ActivationTransactionError::OrphanJournalTemporary => {
                    f.write_str("orphan activation-journal temporary exists")
                }
                ActivationTransactionError::OrphanTemporaryState => f.write_str(
                    "orphan activation-state temporary exists without recoverable ownership",
                ),
                ActivationTransactionError::RecoveryConflict => f.write_str(
                    "activation recovery cannot match authoritative state to journal before/after",
                ),
            }
        }
    }

    impl std::error::Error for ActivationTransactionError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ActivationTransactionError::Format(err) => Some(err),
                ActivationTransactionError::Io { source, .. } => Some(source),
                _ => None,
            }
        }
    }

    impl From<StateFormatError> for ActivationTransactionError {
        fn from(value: StateFormatError) -> Self {
            Self::Format(value)
        }
    }

    fn io_error(operation: &'static str, source: std::io::Error) -> ActivationTransactionError {
        ActivationTransactionError::Io { operation, source }
    }

    impl CoreStatePaths {
        pub(super) fn new(root: &std::path::Path) -> Result<Self, StateFormatError> {
            use std::os::unix::ffi::OsStrExt;

            if root.as_os_str().is_empty() {
                return Err(StateFormatError::EmptyRoot);
            }
            if root.as_os_str().as_bytes().contains(&0) {
                return Err(StateFormatError::NulRoot);
            }
            if !root.is_absolute() {
                return Err(StateFormatError::RelativeRoot);
            }
            Ok(Self {
                root: root.to_path_buf(),
                generations: root.join("generations"),
                activation_state: root.join("activation-state"),
                activation_journal: root.join("activation-journal"),
                activation_journal_temp: root.join("activation-journal.tmp"),
                activation_state_temp: root.join("activation-state.tmp"),
            })
        }
    }

    fn ensure_directory(
        path: &std::path::Path,
        label: &'static str,
    ) -> Result<(), ActivationTransactionError> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                if !metadata.file_type().is_dir() {
                    return Err(ActivationTransactionError::UnsafeFileType(label));
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(path)
                    .map_err(|err| io_error("create Core state directory", err))?;
                let metadata = std::fs::symlink_metadata(path)
                    .map_err(|err| io_error("inspect created Core state directory", err))?;
                if !metadata.file_type().is_dir() {
                    return Err(ActivationTransactionError::UnsafeFileType(label));
                }
            }
            Err(err) => return Err(io_error("inspect Core state directory", err)),
        }
        Ok(())
    }

    pub(super) fn prepare_core_state_paths(
        paths: &CoreStatePaths,
    ) -> Result<(), ActivationTransactionError> {
        ensure_directory(&paths.root, "Core state root")?;
        ensure_directory(&paths.generations, "generation directory")?;
        let directory = std::fs::File::open(&paths.root)
            .map_err(|err| io_error("open Core state root for sync", err))?;
        directory
            .sync_all()
            .map_err(|err| io_error("sync Core state root", err))?;
        Ok(())
    }

    fn validate_generation_identity(
        value: &str,
        field: &'static str,
    ) -> Result<(), StateFormatError> {
        if value.is_empty() {
            return Err(StateFormatError::EmptyIdentity(field));
        }
        if value.as_bytes().len() > GENERATION_ID_MAX_BYTES {
            return Err(StateFormatError::IdentityTooLong(field));
        }
        if value
            .as_bytes()
            .iter()
            .any(|byte| matches!(*byte, 0 | b'\n' | b'\r'))
        {
            return Err(StateFormatError::IdentityControl(field));
        }
        Ok(())
    }

    fn validate_pointer_state(state: &GenerationPointerState) -> Result<(), StateFormatError> {
        validate_generation_identity(&state.current, "current")?;
        validate_generation_identity(&state.verified, "verified")?;
        if let Some(previous) = state.previous.as_deref() {
            validate_generation_identity(previous, "previous")?;
        }
        Ok(())
    }

    pub(super) fn plan_initial_pointer_state(
        complete_candidate_identity: &str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        Ok(GenerationPointerState {
            current: complete_candidate_identity.to_owned(),
            verified: complete_candidate_identity.to_owned(),
            previous: None,
        })
    }

    pub(super) fn plan_activation_pointer_state(
        before: &GenerationPointerState,
        complete_candidate_identity: &str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        if before.current == complete_candidate_identity {
            return Err(StateFormatError::NoChange);
        }
        Ok(GenerationPointerState {
            current: complete_candidate_identity.to_owned(),
            verified: complete_candidate_identity.to_owned(),
            previous: Some(before.current.clone()),
        })
    }

    pub(super) fn plan_rollback_pointer_state(
        before: &GenerationPointerState,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        let previous = before
            .previous
            .as_deref()
            .ok_or(StateFormatError::NoRollbackGeneration)?;
        if previous == before.current {
            return Err(StateFormatError::NoChange);
        }
        Ok(GenerationPointerState {
            current: previous.to_owned(),
            verified: previous.to_owned(),
            previous: Some(before.current.clone()),
        })
    }

    pub(super) fn encode_pointer_state(
        state: &GenerationPointerState,
    ) -> Result<Vec<u8>, StateFormatError> {
        validate_pointer_state(state)?;
        let (previous_present, previous) = match state.previous.as_deref() {
            Some(previous) => ("1", previous),
            None => ("0", ""),
        };
        Ok(format!(
            "format={STATE_FORMAT}\ncurrent={}\nverified={}\nprevious_present={previous_present}\nprevious={previous}\n",
            state.current, state.verified
        )
        .into_bytes())
    }

    fn parse_lines<'a>(
        bytes: &'a [u8],
        label: &'static str,
        expected_records: usize,
    ) -> Result<Vec<&'a str>, StateFormatError> {
        if bytes.len() > STATE_FILE_MAX_BYTES {
            return Err(StateFormatError::FileTooLarge(label));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| StateFormatError::InvalidUtf8(label))?;
        if !text.ends_with('\n') {
            return Err(StateFormatError::MissingFinalNewline(label));
        }
        let body = &text[..text.len() - 1];
        let records: Vec<_> = body.split('\n').collect();
        if records.len() != expected_records {
            return Err(StateFormatError::InvalidRecordCount(label));
        }
        Ok(records)
    }

    fn parse_field<'a>(
        line: &'a str,
        prefix: &str,
        label: &'static str,
    ) -> Result<&'a str, StateFormatError> {
        line.strip_prefix(prefix)
            .ok_or(StateFormatError::InvalidField(label))
    }

    fn parse_presence(value: &str, label: &'static str) -> Result<bool, StateFormatError> {
        match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(StateFormatError::InvalidPresence(label)),
        }
    }

    fn parse_pointer_values(
        current: &str,
        verified: &str,
        previous_present: &str,
        previous: &str,
        label: &'static str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        let has_previous = parse_presence(previous_present, label)?;
        if !has_previous && !previous.is_empty() {
            return Err(StateFormatError::InconsistentAbsent(label));
        }
        let state = GenerationPointerState {
            current: current.to_owned(),
            verified: verified.to_owned(),
            previous: has_previous.then(|| previous.to_owned()),
        };
        validate_pointer_state(&state)?;
        Ok(state)
    }

    pub(super) fn parse_pointer_state(
        bytes: &[u8],
    ) -> Result<GenerationPointerState, StateFormatError> {
        let records = parse_lines(bytes, "activation state", 5)?;
        if records[0] != format!("format={STATE_FORMAT}") {
            return Err(StateFormatError::InvalidField("activation state format"));
        }
        let current = parse_field(records[1], "current=", "activation state current")?;
        let verified = parse_field(records[2], "verified=", "activation state verified")?;
        let previous_present = parse_field(
            records[3],
            "previous_present=",
            "activation state previous presence",
        )?;
        let previous = parse_field(records[4], "previous=", "activation state previous")?;
        parse_pointer_values(
            current,
            verified,
            previous_present,
            previous,
            "activation state previous",
        )
    }

    pub(super) fn encode_activation_journal(
        journal: &ActivationJournal,
    ) -> Result<Vec<u8>, StateFormatError> {
        if journal.before.as_ref() == Some(&journal.after) {
            return Err(StateFormatError::AmbiguousJournal);
        }
        if let Some(before) = journal.before.as_ref() {
            validate_pointer_state(before)?;
        }
        validate_pointer_state(&journal.after)?;

        let (
            before_present,
            before_current,
            before_verified,
            before_previous_present,
            before_previous,
        ) = match journal.before.as_ref() {
            Some(before) => {
                let (previous_present, previous) = match before.previous.as_deref() {
                    Some(previous) => ("1", previous),
                    None => ("0", ""),
                };
                (
                    "1",
                    before.current.as_str(),
                    before.verified.as_str(),
                    previous_present,
                    previous,
                )
            }
            None => ("0", "", "", "0", ""),
        };
        let (after_previous_present, after_previous) = match journal.after.previous.as_deref() {
            Some(previous) => ("1", previous),
            None => ("0", ""),
        };
        Ok(format!(
            "format={JOURNAL_FORMAT}\nbefore_present={before_present}\nbefore_current={before_current}\nbefore_verified={before_verified}\nbefore_previous_present={before_previous_present}\nbefore_previous={before_previous}\nafter_current={}\nafter_verified={}\nafter_previous_present={after_previous_present}\nafter_previous={after_previous}\n",
            journal.after.current, journal.after.verified
        )
        .into_bytes())
    }

    pub(super) fn parse_activation_journal(
        bytes: &[u8],
    ) -> Result<ActivationJournal, StateFormatError> {
        let records = parse_lines(bytes, "activation journal", 10)?;
        if records[0] != format!("format={JOURNAL_FORMAT}") {
            return Err(StateFormatError::InvalidField("activation journal format"));
        }
        let before_present = parse_presence(
            parse_field(records[1], "before_present=", "journal before presence")?,
            "journal before presence",
        )?;
        let before_current = parse_field(records[2], "before_current=", "journal before current")?;
        let before_verified =
            parse_field(records[3], "before_verified=", "journal before verified")?;
        let before_previous_present = parse_field(
            records[4],
            "before_previous_present=",
            "journal before previous presence",
        )?;
        let before_previous =
            parse_field(records[5], "before_previous=", "journal before previous")?;
        let before = if before_present {
            Some(parse_pointer_values(
                before_current,
                before_verified,
                before_previous_present,
                before_previous,
                "journal before state",
            )?)
        } else {
            if !before_current.is_empty()
                || !before_verified.is_empty()
                || before_previous_present != "0"
                || !before_previous.is_empty()
            {
                return Err(StateFormatError::InconsistentAbsent("journal before state"));
            }
            None
        };

        let after_current = parse_field(records[6], "after_current=", "journal after current")?;
        let after_verified = parse_field(records[7], "after_verified=", "journal after verified")?;
        let after_previous_present = parse_field(
            records[8],
            "after_previous_present=",
            "journal after previous presence",
        )?;
        let after_previous = parse_field(records[9], "after_previous=", "journal after previous")?;
        let after = parse_pointer_values(
            after_current,
            after_verified,
            after_previous_present,
            after_previous,
            "journal after state",
        )?;
        if before.as_ref() == Some(&after) {
            return Err(StateFormatError::AmbiguousJournal);
        }
        Ok(ActivationJournal { before, after })
    }

    pub(super) trait ActivationIo {
        fn write_new_synced(&mut self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()>;
        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()>;
        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()>;
        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()>;
    }

    pub(super) struct FsActivationIo;

    impl ActivationIo for FsActivationIo {
        fn write_new_synced(&mut self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            file.write_all(data)?;
            file.sync_all()
        }

        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            std::fs::File::open(path)?.sync_all()
        }

        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
            std::fs::rename(from, to)
        }

        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            std::fs::remove_file(path)
        }
    }

    fn path_exists(
        path: &std::path::Path,
        operation: &'static str,
    ) -> Result<bool, ActivationTransactionError> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(io_error(operation, err)),
        }
    }

    fn read_bounded_regular_file(
        path: &std::path::Path,
        label: &'static str,
    ) -> Result<Option<Vec<u8>>, ActivationTransactionError> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(io_error("inspect activation state file", err)),
        };
        if !metadata.file_type().is_file() {
            return Err(ActivationTransactionError::UnsafeFileType(label));
        }
        if metadata.len() > STATE_FILE_MAX_BYTES as u64 {
            return Err(StateFormatError::FileTooLarge(label).into());
        }
        let file =
            std::fs::File::open(path).map_err(|err| io_error("open activation state file", err))?;
        let mut bytes = Vec::new();
        file.take((STATE_FILE_MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|err| io_error("read activation state file", err))?;
        if bytes.len() > STATE_FILE_MAX_BYTES {
            return Err(StateFormatError::FileTooLarge(label).into());
        }
        Ok(Some(bytes))
    }

    pub(super) fn read_pointer_state(
        paths: &CoreStatePaths,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        read_bounded_regular_file(&paths.activation_state, "activation state")?
            .map(|bytes| parse_pointer_state(&bytes).map_err(ActivationTransactionError::from))
            .transpose()
    }

    fn read_activation_journal(
        paths: &CoreStatePaths,
    ) -> Result<Option<ActivationJournal>, ActivationTransactionError> {
        read_bounded_regular_file(&paths.activation_journal, "activation journal")?
            .map(|bytes| parse_activation_journal(&bytes).map_err(ActivationTransactionError::from))
            .transpose()
    }

    pub(super) fn activate_pointer_state_with_io<I: ActivationIo>(
        paths: &CoreStatePaths,
        before: Option<&GenerationPointerState>,
        after: &GenerationPointerState,
        io: &mut I,
    ) -> Result<(), ActivationTransactionError> {
        if let Some(before) = before {
            validate_pointer_state(before)?;
        }
        validate_pointer_state(after)?;
        if before == Some(after) {
            return Err(StateFormatError::AmbiguousJournal.into());
        }
        let authoritative = read_pointer_state(paths)?;
        if authoritative.as_ref() != before {
            return Err(ActivationTransactionError::StaleAuthoritativeState);
        }
        if path_exists(&paths.activation_journal, "inspect activation journal")? {
            return Err(ActivationTransactionError::PendingJournal);
        }
        if path_exists(
            &paths.activation_journal_temp,
            "inspect activation journal temporary",
        )? {
            return Err(ActivationTransactionError::OrphanJournalTemporary);
        }
        if path_exists(&paths.activation_state_temp, "inspect activation temporary")? {
            return Err(ActivationTransactionError::OrphanTemporaryState);
        }

        let journal = ActivationJournal {
            before: before.cloned(),
            after: after.clone(),
        };
        let journal_bytes = encode_activation_journal(&journal)?;
        let state_bytes = encode_pointer_state(after)?;

        io.write_new_synced(&paths.activation_journal_temp, &journal_bytes)
            .map_err(|err| io_error("write and sync activation journal temporary", err))?;
        io.rename(&paths.activation_journal_temp, &paths.activation_journal)
            .map_err(|err| io_error("publish activation journal", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync journal directory", err))?;
        io.write_new_synced(&paths.activation_state_temp, &state_bytes)
            .map_err(|err| io_error("write and sync activation state temporary", err))?;
        io.rename(&paths.activation_state_temp, &paths.activation_state)
            .map_err(|err| io_error("atomically replace activation state", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync activation state directory", err))?;
        io.remove_file(&paths.activation_journal)
            .map_err(|err| io_error("remove activation journal", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync final activation directory", err))?;
        Ok(())
    }

    pub(super) fn activate_pointer_state(
        paths: &CoreStatePaths,
        before: Option<&GenerationPointerState>,
        after: &GenerationPointerState,
    ) -> Result<(), ActivationTransactionError> {
        let mut io = FsActivationIo;
        activate_pointer_state_with_io(paths, before, after, &mut io)
    }

    pub(super) fn recover_activation_state_with_io<I: ActivationIo>(
        paths: &CoreStatePaths,
        io: &mut I,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        let journal_temporary_exists = path_exists(
            &paths.activation_journal_temp,
            "inspect activation journal temporary",
        )?;
        let state_temporary_exists =
            path_exists(&paths.activation_state_temp, "inspect activation temporary")?;
        let journal = read_activation_journal(paths)?;
        let Some(journal) = journal else {
            if journal_temporary_exists {
                if state_temporary_exists {
                    return Err(ActivationTransactionError::RecoveryConflict);
                }
                io.remove_file(&paths.activation_journal_temp)
                    .map_err(|err| io_error("remove stale activation journal temporary", err))?;
                io.sync_dir(&paths.root)
                    .map_err(|err| io_error("sync recovered journal-temporary directory", err))?;
                return read_pointer_state(paths);
            }
            if state_temporary_exists {
                return Err(ActivationTransactionError::OrphanTemporaryState);
            }
            return read_pointer_state(paths);
        };
        if journal_temporary_exists {
            return Err(ActivationTransactionError::OrphanJournalTemporary);
        }

        if journal.before.as_ref() == Some(&journal.after) {
            return Err(StateFormatError::AmbiguousJournal.into());
        }
        let authoritative = read_pointer_state(paths)?;
        let matches_before = authoritative == journal.before;
        let matches_after = authoritative.as_ref() == Some(&journal.after);
        if matches_before == matches_after {
            return Err(ActivationTransactionError::RecoveryConflict);
        }

        if state_temporary_exists {
            io.remove_file(&paths.activation_state_temp)
                .map_err(|err| io_error("remove stale activation temporary", err))?;
        }
        io.remove_file(&paths.activation_journal)
            .map_err(|err| io_error("remove recovered activation journal", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync recovered activation directory", err))?;
        Ok(authoritative)
    }

    pub(super) fn recover_activation_state(
        paths: &CoreStatePaths,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        let mut io = FsActivationIo;
        recover_activation_state_with_io(paths, &mut io)
    }
}

#[cfg(unix)]
#[allow(dead_code)]
fn execute_public_entrypoint<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
    'generation,
    I,
    S,
>(
    raw_args: I,
    context: LocalPublicDispatchContext<
        'context,
        'runtime_selection,
        'runtime_asset,
        'manager_selection,
        'manager_asset,
        'generation,
    >,
) -> Result<PublicDispatchCompletion, PublicDispatchExecutionError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    execute_public_dispatch(plan_public_dispatch(raw_args), context)
}

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let _ = classify_first_arg(args.next().as_deref());
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::m2_generation_state::*;
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

    #[test]
    fn test_m1_b20_a_exact_core_routes_consume_only_first_token_and_preserve_tail() {
        assert_eq!(
            plan_public_dispatch([
                OsString::from("update"),
                OsString::from("--channel"),
                OsString::from("stable"),
            ]),
            PublicDispatchRoute::Update(vec![
                OsString::from("--channel"),
                OsString::from("stable"),
            ])
        );
        assert_eq!(
            plan_public_dispatch([OsString::from("doctor"), OsString::from("--json")]),
            PublicDispatchRoute::Doctor(vec![OsString::from("--json")])
        );
        assert_eq!(
            plan_public_dispatch([
                OsString::from("termux"),
                OsString::from("status"),
                OsString::from("--raw"),
            ]),
            PublicDispatchRoute::Termux(vec![OsString::from("status"), OsString::from("--raw"),])
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b20_b_core_route_preserves_raw_non_utf8_trailing_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let raw = vec![0xff, 0xfe, 0x80, b'x'];
        let route =
            plan_public_dispatch([OsString::from("doctor"), OsString::from_vec(raw.clone())]);
        match route {
            PublicDispatchRoute::Doctor(tail) => {
                assert_eq!(tail.len(), 1);
                assert_eq!(tail[0].as_os_str().as_bytes(), raw.as_slice());
            }
            other => panic!("expected doctor route, got {other:?}"),
        }
    }

    #[test]
    fn test_m1_b20_c_upstream_route_preserves_complete_version_nearmiss_and_delimiter_argv() {
        let cases: Vec<Vec<OsString>> = vec![
            vec![],
            vec![OsString::from("--version")],
            vec![OsString::from("-V")],
            vec![OsString::from("--"), OsString::from("doctor")],
            vec![OsString::from("Doctor")],
            vec![OsString::from("doctorx")],
            vec![OsString::from("--doctor")],
            vec![OsString::from("exec"), OsString::from("termux")],
            vec![OsString::from("sandbox"), OsString::from("linux")],
        ];

        for original in cases {
            assert_eq!(
                plan_public_dispatch(original.clone()),
                PublicDispatchRoute::Upstream(original)
            );
        }
        assert_eq!(
            plan_public_dispatch([OsString::from("exec"), OsString::from("task")]),
            PublicDispatchRoute::Upstream(vec![OsString::from("exec"), OsString::from("task"),])
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b20_d_non_utf8_first_token_is_upstream_and_all_bytes_remain_exact() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let first_raw = b"update\xff".to_vec();
        let second_raw = vec![0x80, b'a', 0xfe];
        let route = plan_public_dispatch(vec![
            OsString::from_vec(first_raw.clone()),
            OsString::from_vec(second_raw.clone()),
        ]);
        match route {
            PublicDispatchRoute::Upstream(argv) => {
                assert_eq!(argv.len(), 2);
                assert_eq!(argv[0].as_os_str().as_bytes(), first_raw.as_slice());
                assert_eq!(argv[1].as_os_str().as_bytes(), second_raw.as_slice());
            }
            other => panic!("non-UTF-8 first token must route upstream, got {other:?}"),
        }
    }

    #[test]
    fn test_m1_b20_e_planning_is_deterministic_environment_pure_and_does_not_mutate_input() {
        let before_env = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        let original = vec![
            OsString::from("exec"),
            OsString::from("ordinary arg with spaces"),
            OsString::from("--custom=value"),
        ];
        let original_copy = original.clone();
        let first = plan_public_dispatch(original.clone());
        let second = plan_public_dispatch(original.clone());
        let after_env = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];

        assert_eq!(original, original_copy);
        assert_eq!(first, second);
        assert_eq!(first, PublicDispatchRoute::Upstream(original_copy));
        assert_eq!(before_env, after_env);
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
    const PROBE_B17_MODE_ENV: &str = "CODEX_TEST_B17_MODE";
    #[cfg(unix)]
    const PROBE_B19_MODE_ENV: &str = "CODEX_TEST_B19_MODE";

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
    fn run_m1_b19_command_probe() -> ! {
        let resolver_path =
            std::env::var_os(PROBE_RESOLVER_PATH_ENV).expect("PROBE_RESOLVER_PATH_ENV must be set");
        let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
            .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
        let runtime_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
            .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");
        let mode = std::env::var(PROBE_B19_MODE_ENV).expect("PROBE_B19_MODE_ENV must be set");
        let root = std::path::Path::new(&runtime_path)
            .parent()
            .expect("B19 runtime path must have parent");
        let compatibility_dir = root.join("doctor-compat-bin");
        let prefix = root.join("doctor-prefix");
        let temp_dir = root.join("doctor-tmp");
        let cert_file = root.join("doctor-tls/cert.pem");
        let cert_dir = root.join("doctor-tls/certs.d");

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("B19 probe generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime_path.as_os_str(),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: compatibility_dir.as_os_str(),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("B19 probe runtime assets must qualify");
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(prefix.into_os_string()),
            tmpdir: Some(temp_dir.into_os_string()),
            inherited_path: Some(OsString::from(
                "/probe/b16/inherited-a:/probe/b16/inherited-b",
            )),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-b19-npm-contam");
        std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-b19-bun-contam");
        std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/b19/pkg/root");
        std::env::set_var("LD_PRELOAD", "/probe/b19/preload.so");
        std::env::set_var("LD_LIBRARY_PATH", "/probe/b19/lib");
        std::env::set_var(
            "CODEX_TEST_UNRELATED_M1_B16_SURVIVING_VAR",
            "m1_b16_surviving_exact_value_27182",
        );

        let (args, core, manager) = match mode.as_str() {
            "healthy-human" => (
                Vec::<OsString>::new(),
                CoreDoctorStatus::Healthy,
                ManagerDoctorStatus::Healthy,
            ),
            "unhealthy-json" => (
                vec![OsString::from("--json")],
                CoreDoctorStatus::Healthy,
                ManagerDoctorStatus::Healthy,
            ),
            "missing-json" => (
                vec![OsString::from("--json")],
                CoreDoctorStatus::Healthy,
                ManagerDoctorStatus::Healthy,
            ),
            other => panic!("unknown B19 command probe mode: {other}"),
        };

        match run_local_doctor_command(
            args,
            UpstreamDoctorCapability::Supported,
            qualified,
            &snapshot,
            cert_file.as_os_str(),
            Some(cert_dir.as_os_str()),
            resolver_path,
            config_dir_path,
            core,
            manager,
        ) {
            Ok(outcome) => {
                let exit = match outcome.exit_class {
                    DoctorExitClass::Success => "success",
                    DoctorExitClass::HealthFailure => "health_failure",
                    DoctorExitClass::ApiIncompatibility => "api_incompatibility",
                };
                use std::io::Write;
                writeln!(std::io::stdout(), "B19_EXIT:{exit}").expect("write B19 exit class");
                write!(std::io::stdout(), "B19_OUTPUT:{}", outcome.output)
                    .expect("write B19 output");
                std::io::stdout().flush().expect("flush B19 output");
                std::process::exit(0);
            }
            Err(LocalDoctorCommandError::Probe(QualifiedUpstreamDoctorProbeError::Io(err)))
                if mode == "missing-json" && err.kind() == std::io::ErrorKind::NotFound =>
            {
                use std::io::Write;
                writeln!(std::io::stdout(), "B19_PROBE_NOT_FOUND")
                    .expect("write B19 missing-runtime marker");
                std::io::stdout()
                    .flush()
                    .expect("flush B19 missing-runtime marker");
                std::process::exit(0);
            }
            Err(err) => panic!("B19 command probe failed: {err}"),
        }
    }

    #[cfg(unix)]
    fn run_m1_b17_coordinator_probe() -> ! {
        let resolver_path =
            std::env::var_os(PROBE_RESOLVER_PATH_ENV).expect("PROBE_RESOLVER_PATH_ENV must be set");
        let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
            .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
        let runtime_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
            .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");
        let mode = std::env::var(PROBE_B17_MODE_ENV).expect("PROBE_B17_MODE_ENV must be set");
        let root = std::path::Path::new(&runtime_path)
            .parent()
            .expect("B17 runtime path must have parent");
        let compatibility_dir = root.join("doctor-compat-bin");
        let prefix = root.join("doctor-prefix");
        let temp_dir = root.join("doctor-tmp");
        let cert_file = root.join("doctor-tls/cert.pem");
        let cert_dir = root.join("doctor-tls/certs.d");

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("B17 probe generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime_path.as_os_str(),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: compatibility_dir.as_os_str(),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("B17 probe runtime assets must qualify");
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(prefix.into_os_string()),
            tmpdir: Some(temp_dir.into_os_string()),
            inherited_path: Some(OsString::from(
                "/probe/b16/inherited-a:/probe/b16/inherited-b",
            )),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-b17-npm-contam");
        std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-b17-bun-contam");
        std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/b17/pkg/root");
        std::env::set_var("LD_PRELOAD", "/probe/b17/preload.so");
        std::env::set_var("LD_LIBRARY_PATH", "/probe/b17/lib");
        std::env::set_var(
            "CODEX_TEST_UNRELATED_M1_B16_SURVIVING_VAR",
            "m1_b16_surviving_exact_value_27182",
        );

        let (termux_core, manager) = match mode.as_str() {
            "healthy-degraded" => (CoreDoctorStatus::Healthy, ManagerDoctorStatus::Unavailable),
            "unhealthy-api" => (
                CoreDoctorStatus::ApiIncompatible,
                ManagerDoctorStatus::Healthy,
            ),
            "missing-runtime" => (CoreDoctorStatus::Healthy, ManagerDoctorStatus::Healthy),
            other => panic!("unknown B17 coordinator probe mode: {other}"),
        };

        match compose_local_doctor(
            UpstreamDoctorCapability::Supported,
            qualified,
            &snapshot,
            cert_file.as_os_str(),
            Some(cert_dir.as_os_str()),
            resolver_path,
            config_dir_path,
            termux_core,
            manager,
        ) {
            Ok(report) => {
                use std::io::Write;
                writeln!(
                    std::io::stdout(),
                    "B17_REPORT:{}:{}:{}:{}",
                    report.upstream.as_str(),
                    report.termux_core.as_str(),
                    report.manager.as_str(),
                    report.summary.as_str(),
                )
                .expect("write B17 report");
                write!(
                    std::io::stdout(),
                    "B17_HUMAN:{}",
                    render_doctor_human(&report)
                )
                .expect("write B17 human report");
                write!(
                    std::io::stdout(),
                    "B17_JSON:{}",
                    render_doctor_json(&report)
                )
                .expect("write B17 JSON report");
                std::io::stdout().flush().expect("flush B17 report");
                std::process::exit(0);
            }
            Err(QualifiedUpstreamDoctorProbeError::Io(err))
                if mode == "missing-runtime" && err.kind() == std::io::ErrorKind::NotFound =>
            {
                use std::io::Write;
                writeln!(std::io::stdout(), "B17_IO_NOT_FOUND")
                    .expect("write B17 missing-runtime marker");
                std::io::stdout()
                    .flush()
                    .expect("flush B17 missing-runtime marker");
                std::process::exit(0);
            }
            Err(err) => panic!("B17 coordinator probe failed: {err}"),
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
            "m1_b19_command_launcher" => run_m1_b19_command_probe(),
            "m1_b17_coordinator_launcher" => run_m1_b17_coordinator_probe(),
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
            "m1_b24_real_termux_entrypoint" => {
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("B24 config dir path must be set");
                let fake_upstream_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("B24 fake upstream path must be set");
                let root = std::path::Path::new(&fake_upstream_path)
                    .parent()
                    .expect("B24 fake runtime must have parent");
                let compatibility_dir = root.join("compat");
                let cert_file = root.join("cert.pem");

                let snapshot = capture_termux_process_env();
                let prefix = snapshot
                    .prefix
                    .as_deref()
                    .expect("B24 real Termux smoke requires PREFIX");
                let resolver_path = std::path::Path::new(prefix).join("etc/resolv.conf");

                let mut manifest = m1_b11_valid_manifest();
                manifest.helper_digests.clear();
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B24 generation must qualify");
                let selection = RuntimeAssetSelection {
                    runtime: RuntimeAssetBinding {
                        program_path: fake_upstream_path.as_os_str(),
                        observed_digest: manifest.runtime_digest.as_str(),
                    },
                    compatibility_dir: compatibility_dir.as_os_str(),
                    helpers: &[],
                };
                let runtime = qualify_runtime_assets(generation, &selection)
                    .expect("B24 fake runtime must qualify");
                let manager = qualify_manager_artifact(generation, None)
                    .expect("B24 absent Manager must qualify");
                let context = build_local_public_dispatch_context(
                    runtime,
                    manager,
                    &snapshot,
                    cert_file.as_os_str(),
                    None,
                    resolver_path.as_path(),
                    std::path::Path::new(&config_dir_path),
                    UpstreamDoctorCapability::Unsupported,
                    CoreDoctorStatus::Healthy,
                    ManagerDoctorStatus::Unavailable,
                )
                .expect("B24 context must be generation-coherent");

                let err = execute_public_entrypoint([OsString::from("--version")], context)
                    .expect_err("B24 upstream entrypoint must replace the process");
                panic!("B24 public entrypoint failed to replace process: {err}");
            }
            "m1_b23_upstream_dispatch" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("B23 resolver path must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("B23 config dir path must be set");
                let fake_upstream_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("B23 fake upstream path must be set");
                let root = std::path::Path::new(&fake_upstream_path)
                    .parent()
                    .expect("B23 fake runtime must have parent");
                let compatibility_dir = root.join("b23-compat");
                let prefix = root.join("b23-prefix");
                let temp_dir = root.join("b23-tmp");
                let cert_file = root.join("b23-cert.pem");

                let mut manifest = m1_b11_valid_manifest();
                manifest.helper_digests.clear();
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B23 upstream generation must qualify");
                let selection = RuntimeAssetSelection {
                    runtime: RuntimeAssetBinding {
                        program_path: fake_upstream_path.as_os_str(),
                        observed_digest: manifest.runtime_digest.as_str(),
                    },
                    compatibility_dir: compatibility_dir.as_os_str(),
                    helpers: &[],
                };
                let runtime = qualify_runtime_assets(generation, &selection)
                    .expect("B23 upstream runtime must qualify");
                let manager = qualify_manager_artifact(generation, None)
                    .expect("B23 absent Manager must qualify");
                let snapshot = TermuxProcessEnvSnapshot {
                    prefix: Some(prefix.into_os_string()),
                    tmpdir: Some(temp_dir.into_os_string()),
                    inherited_path: Some(OsString::from("/probe/b23/inherited")),
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };
                let context = build_local_public_dispatch_context(
                    runtime,
                    manager,
                    &snapshot,
                    cert_file.as_os_str(),
                    None,
                    std::path::Path::new(&resolver_path),
                    std::path::Path::new(&config_dir_path),
                    UpstreamDoctorCapability::Unsupported,
                    CoreDoctorStatus::Healthy,
                    ManagerDoctorStatus::Unavailable,
                )
                .expect("B23 upstream context must be coherent");

                use std::os::unix::ffi::OsStrExt;
                let route = PublicDispatchRoute::Upstream(vec![
                    OsString::from("exec"),
                    OsString::from("b23-task"),
                    OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).to_os_string(),
                ]);
                let err = execute_public_dispatch(route, context)
                    .expect_err("B23 upstream success must replace the process");
                panic!("B23 upstream dispatcher failed to replace process: {err}");
            }
            "m1_b23_manager_dispatch" => {
                use std::os::unix::ffi::OsStrExt;

                let mut manifest = m1_b11_valid_manifest();
                manifest.helper_digests.clear();
                manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:b23".to_string());
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B23 Manager generation must qualify");
                let runtime_selection = RuntimeAssetSelection {
                    runtime: RuntimeAssetBinding {
                        program_path: OsStr::new("/unused/b23/runtime"),
                        observed_digest: manifest.runtime_digest.as_str(),
                    },
                    compatibility_dir: OsStr::new("/unused/b23/compat"),
                    helpers: &[],
                };
                let runtime = qualify_runtime_assets(generation, &runtime_selection)
                    .expect("B23 unused runtime shape must qualify");
                let manager_selection = ManagerArtifactSelection {
                    program_path: shell.as_os_str(),
                    observed_digest: "opaque-manager-digest:v1:b23",
                };
                let manager = qualify_manager_artifact(generation, Some(&manager_selection))
                    .expect("B23 Manager must qualify");
                let snapshot = TermuxProcessEnvSnapshot {
                    prefix: None,
                    tmpdir: None,
                    inherited_path: None,
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };
                let context = build_local_public_dispatch_context(
                    runtime,
                    manager,
                    &snapshot,
                    OsStr::new("/unused/b23/cert"),
                    None,
                    std::path::Path::new("/unused/b23/resolver"),
                    std::path::Path::new("/unused/b23/config"),
                    UpstreamDoctorCapability::Supported,
                    CoreDoctorStatus::Unhealthy,
                    ManagerDoctorStatus::ApiIncompatible,
                )
                .expect("B23 Manager context must be coherent");

                let script = r#"
printf "B23_MANAGER_ENV:%s\n" "${CODEX_MANAGED_BY_NPM-unset}"
for a in "$@"; do
    printf "ARG:"
    printf "%s" "$a"
    printf "\n"
done
exit 71
"#;
                let route = PublicDispatchRoute::Termux(vec![
                    OsString::from("-c"),
                    OsString::from(script),
                    OsString::from("manager-probe"),
                    OsString::from("b23 ordinary"),
                    OsStr::from_bytes(&[0xff, 0xfe, 0x80]).to_os_string(),
                ]);
                let err = execute_public_dispatch(route, context)
                    .expect_err("B23 Manager success must replace the process");
                panic!("B23 Manager dispatcher failed to replace process: {err}");
            }
            "m1_b22_manager_exec" => {
                use std::os::unix::ffi::OsStrExt;

                let mut manifest = m1_b11_valid_manifest();
                manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:b22".to_string());
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B22 Manager generation must qualify");
                let selection = ManagerArtifactSelection {
                    program_path: shell.as_os_str(),
                    observed_digest: "opaque-manager-digest:v1:b22",
                };
                let qualification = qualify_manager_artifact(generation, Some(&selection))
                    .expect("B22 Manager artifact must qualify");

                let script = r#"
printf "MANAGER_NPM:%s\n" "${CODEX_MANAGED_BY_NPM-unset}"
for a in "$@"; do
    printf "ARG:"
    printf "%s" "$a"
    printf "\n"
done
printf "MANAGER_STDOUT:\001\002\003\377\376\n"
printf "MANAGER_STDERR:\004\005\006\200\201\n" >&2
exit 73
"#;
                let non_utf8_arg = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
                let args = vec![
                    OsString::from("-c"),
                    OsString::from(script),
                    OsString::from("manager-probe"),
                    OsString::from("ordinary arg with spaces and ="),
                    non_utf8_arg.to_os_string(),
                ];

                let err = execute_termux_manager(qualification, args)
                    .expect_err("available Manager exec must replace the process");
                panic!("B22 Manager failed to replace process: {err}");
            }
            "m1_b22_manager_signal" => {
                let mut manifest = m1_b11_valid_manifest();
                manifest.manager_artifact_digest =
                    Some("opaque-manager-digest:v1:b22-signal".to_string());
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B22 signal generation must qualify");
                let selection = ManagerArtifactSelection {
                    program_path: shell.as_os_str(),
                    observed_digest: "opaque-manager-digest:v1:b22-signal",
                };
                let qualification = qualify_manager_artifact(generation, Some(&selection))
                    .expect("B22 signal Manager must qualify");

                let script = r#"
trap 'exit 73' TERM
printf "READY:PID:%d\n" "$$"
while true; do
    :
done
"#;
                let args = vec![
                    OsString::from("-c"),
                    OsString::from(script),
                    OsString::from("manager-probe"),
                ];
                let err = execute_termux_manager(qualification, args)
                    .expect_err("available Manager signal probe must replace process");
                panic!("B22 signal Manager failed to replace process: {err}");
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
            "m1_b16_doctor_probe_launcher" => {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::io::FromRawFd;

                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                let fake_upstream_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");
                let root = std::path::Path::new(&fake_upstream_path)
                    .parent()
                    .expect("fake doctor runtime must have parent");
                let compatibility_dir = root.join("doctor-compat-bin");
                let prefix = root.join("doctor-prefix");
                let temp_dir = root.join("doctor-tmp");
                let cert_file = root.join("doctor-tls/cert.pem");
                let cert_dir = root.join("doctor-tls/certs.d");

                let mut manifest = m1_b11_valid_manifest();
                manifest.helper_digests.clear();
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B16 probe generation must qualify");
                let selection = RuntimeAssetSelection {
                    runtime: RuntimeAssetBinding {
                        program_path: fake_upstream_path.as_os_str(),
                        observed_digest: manifest.runtime_digest.as_str(),
                    },
                    compatibility_dir: compatibility_dir.as_os_str(),
                    helpers: &[],
                };
                let qualified = qualify_runtime_assets(generation, &selection)
                    .expect("B16 probe runtime assets must qualify");
                let snapshot = TermuxProcessEnvSnapshot {
                    prefix: Some(prefix.into_os_string()),
                    tmpdir: Some(temp_dir.into_os_string()),
                    inherited_path: Some(OsString::from(
                        "/probe/b16/inherited-a:/probe/b16/inherited-b",
                    )),
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };

                std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-b16-npm-contam");
                std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-b16-bun-contam");
                std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/b16/pkg/root");
                std::env::set_var("LD_PRELOAD", "/probe/b16/preload.so");
                std::env::set_var("LD_LIBRARY_PATH", "/probe/b16/lib");
                std::env::set_var(
                    "CODEX_TEST_UNRELATED_M1_B16_SURVIVING_VAR",
                    "m1_b16_surviving_exact_value_27182",
                );

                let sentinel33_path = root.join("b16-sentinel-33.bin");
                let sentinel34_path = root.join("b16-sentinel-34.bin");
                let sentinel33 = b"B16_SENTINEL_FD33_EXACT";
                let sentinel34 = b"B16_SENTINEL_FD34_EXACT";
                std::fs::write(&sentinel33_path, sentinel33).expect("write B16 sentinel 33");
                std::fs::write(&sentinel34_path, sentinel34).expect("write B16 sentinel 34");
                let f33 = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&sentinel33_path)
                    .expect("open B16 sentinel 33");
                let f34 = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&sentinel34_path)
                    .expect("open B16 sentinel 34");
                let meta33 = f33.metadata().expect("B16 sentinel 33 metadata");
                let meta34 = f34.metadata().expect("B16 sentinel 34 metadata");
                unsafe {
                    dup2(f33.as_raw_fd(), 33);
                    dup2(f34.as_raw_fd(), 34);
                }
                drop(f33);
                drop(f34);

                let status = probe_qualified_upstream_doctor(
                    qualified,
                    &snapshot,
                    cert_file.as_os_str(),
                    Some(cert_dir.as_os_str()),
                    resolver_path,
                    config_dir_path,
                )
                .expect("B16 doctor probe must complete");

                let restored33 =
                    std::fs::metadata("/proc/self/fd/33").expect("B16 restored FD33 metadata");
                let restored34 =
                    std::fs::metadata("/proc/self/fd/34").expect("B16 restored FD34 metadata");
                assert_eq!(
                    (restored33.dev(), restored33.ino()),
                    (meta33.dev(), meta33.ino())
                );
                assert_eq!(
                    (restored34.dev(), restored34.ino()),
                    (meta34.dev(), meta34.ino())
                );

                let mut restored33_bytes = Vec::new();
                let mut restored34_bytes = Vec::new();
                unsafe {
                    use std::io::{Read, Seek};
                    let mut file33 = std::fs::File::from_raw_fd(33);
                    let _ = file33.rewind();
                    file33
                        .read_to_end(&mut restored33_bytes)
                        .expect("read B16 restored FD33");
                    std::mem::forget(file33);
                    let mut file34 = std::fs::File::from_raw_fd(34);
                    let _ = file34.rewind();
                    file34
                        .read_to_end(&mut restored34_bytes)
                        .expect("read B16 restored FD34");
                    std::mem::forget(file34);
                }
                assert_eq!(restored33_bytes, sentinel33);
                assert_eq!(restored34_bytes, sentinel34);

                use std::io::Write;
                writeln!(std::io::stdout(), "B16_STATUS:{}", status.as_str())
                    .expect("write B16 status");
                writeln!(std::io::stdout(), "B16_FD_RESTORED").expect("write B16 restore marker");
                std::io::stdout().flush().expect("flush B16 status");
                std::process::exit(0);
            }
            "m1_b16_missing_runtime_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                let missing_runtime = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");
                let root = std::path::Path::new(&missing_runtime)
                    .parent()
                    .expect("missing runtime must have parent");
                let compatibility_dir = root.join("doctor-compat-bin");
                let prefix = root.join("doctor-prefix");
                let temp_dir = root.join("doctor-tmp");
                let cert_file = root.join("doctor-tls/cert.pem");

                let mut manifest = m1_b11_valid_manifest();
                manifest.helper_digests.clear();
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B16 missing-runtime generation must qualify");
                let selection = RuntimeAssetSelection {
                    runtime: RuntimeAssetBinding {
                        program_path: missing_runtime.as_os_str(),
                        observed_digest: manifest.runtime_digest.as_str(),
                    },
                    compatibility_dir: compatibility_dir.as_os_str(),
                    helpers: &[],
                };
                let qualified = qualify_runtime_assets(generation, &selection)
                    .expect("B16 missing-runtime shape must qualify");
                let snapshot = TermuxProcessEnvSnapshot {
                    prefix: Some(prefix.into_os_string()),
                    tmpdir: Some(temp_dir.into_os_string()),
                    inherited_path: None,
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };

                match probe_qualified_upstream_doctor(
                    qualified,
                    &snapshot,
                    cert_file.as_os_str(),
                    None,
                    resolver_path,
                    config_dir_path,
                ) {
                    Err(QualifiedUpstreamDoctorProbeError::Io(err)) => {
                        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
                    }
                    other => panic!("expected typed B16 NotFound I/O error, got {other:?}"),
                }
                use std::io::Write;
                writeln!(std::io::stdout(), "B16_IO_NOT_FOUND").expect("write B16 I/O marker");
                std::io::stdout().flush().expect("flush B16 I/O marker");
                std::process::exit(0);
            }
            "m1_b14_qualified_runtime_launcher" => {
                let resolver_path = std::env::var_os(PROBE_RESOLVER_PATH_ENV)
                    .expect("PROBE_RESOLVER_PATH_ENV must be set");
                let config_dir_path = std::env::var_os(PROBE_CONFIG_DIR_PATH_ENV)
                    .expect("PROBE_CONFIG_DIR_PATH_ENV must be set");
                let fake_upstream_path = std::env::var_os(PROBE_FAKE_UPSTREAM_PATH_ENV)
                    .expect("PROBE_FAKE_UPSTREAM_PATH_ENV must be set");
                let root = std::path::Path::new(&fake_upstream_path)
                    .parent()
                    .expect("fake runtime must have parent");
                let compatibility_dir = root.join("qualified-compat-bin");
                let prefix = root.join("qualified-prefix");
                let temp_dir = root.join("qualified-tmp");
                let cert_file = root.join("qualified-tls/cert.pem");
                let cert_dir = root.join("qualified-tls/certs.d");

                let mut manifest = m1_b11_valid_manifest();
                manifest.helper_digests.clear();
                let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
                    .expect("B14 probe generation must qualify");
                let selection = RuntimeAssetSelection {
                    runtime: RuntimeAssetBinding {
                        program_path: fake_upstream_path.as_os_str(),
                        observed_digest: manifest.runtime_digest.as_str(),
                    },
                    compatibility_dir: compatibility_dir.as_os_str(),
                    helpers: &[],
                };
                let qualified = qualify_runtime_assets(generation, &selection)
                    .expect("B14 probe runtime assets must qualify");
                let snapshot = TermuxProcessEnvSnapshot {
                    prefix: Some(prefix.into_os_string()),
                    tmpdir: Some(temp_dir.into_os_string()),
                    inherited_path: Some(OsString::from(
                        "/probe/b14/inherited-a:/probe/b14/inherited-b",
                    )),
                    inherited_ssl_cert_file: None,
                    inherited_ssl_cert_dir: None,
                };

                std::env::set_var("CODEX_MANAGED_BY_NPM", "probe-b14-npm-contam");
                std::env::set_var("CODEX_MANAGED_BY_BUN", "probe-b14-bun-contam");
                std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/probe/b14/pkg/root");
                std::env::set_var("LD_PRELOAD", "/probe/b14/preload.so");
                std::env::set_var("LD_LIBRARY_PATH", "/probe/b14/lib");
                std::env::set_var(
                    "CODEX_TEST_UNRELATED_M1_B14_SURVIVING_VAR",
                    "m1_b14_surviving_exact_value_31415",
                );

                use std::os::unix::ffi::OsStrExt;
                let non_utf8_arg = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]);
                let user_args: Vec<OsString> = vec![
                    OsString::from("exec"),
                    OsString::from("qualified_task"),
                    OsString::from("--qualified-flag=value"),
                    OsString::from("ordinary qualified arg with spaces"),
                    non_utf8_arg.to_os_string(),
                ];

                let err = launch_qualified_runtime(
                    qualified,
                    &snapshot,
                    cert_file.as_os_str(),
                    Some(cert_dir.as_os_str()),
                    resolver_path,
                    config_dir_path,
                    user_args,
                );
                panic!("launch_qualified_runtime failed to replace process: {err}");
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

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_a_absent_manifest_and_selection_is_explicitly_unavailable() {
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation without Manager must qualify");
        let result = qualify_manager_artifact(generation, None).expect("absence is valid");
        let ManagerArtifactQualification::Unavailable(bound_generation) = result else {
            panic!("absence must remain explicitly unavailable");
        };
        assert!(std::ptr::eq(bound_generation.manifest(), &manifest));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_b_matching_manifest_and_selection_becomes_available_borrowed() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation with Manager must qualify");
        let selection = ManagerArtifactSelection {
            program_path: OsStr::new("/manager/codex-manager"),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        let result = qualify_manager_artifact(generation, Some(&selection))
            .expect("matching Manager must qualify");
        let ManagerArtifactQualification::Available(qualified) = result else {
            panic!("matching Manager must be available");
        };
        assert!(std::ptr::eq(qualified.selection(), &selection));
        assert!(std::ptr::eq(qualified.generation().manifest(), &manifest));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_c_manifest_selection_presence_must_agree_both_directions() {
        let no_manager = m1_b11_valid_manifest();
        let no_manager_generation =
            qualify_generation_manifest(&no_manager, &m1_b11_requirements()).unwrap();
        let unexpected = ManagerArtifactSelection {
            program_path: OsStr::new("/manager/codex-manager"),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        assert_eq!(
            qualify_manager_artifact(no_manager_generation, Some(&unexpected)).unwrap_err(),
            ManagerArtifactError::UnexpectedSelection
        );

        let mut with_manager = m1_b11_valid_manifest();
        with_manager.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        let with_manager_generation =
            qualify_generation_manifest(&with_manager, &m1_b11_requirements()).unwrap();
        assert_eq!(
            qualify_manager_artifact(with_manager_generation, None).unwrap_err(),
            ManagerArtifactError::MissingSelection
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_d_path_shape_reuses_runtime_asset_fail_closed_semantics() {
        use std::os::unix::ffi::OsStringExt;

        let mut manifest = m1_b11_valid_manifest();
        manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();

        let empty = ManagerArtifactSelection {
            program_path: OsStr::new(""),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        assert_eq!(
            qualify_manager_artifact(generation, Some(&empty)).unwrap_err(),
            ManagerArtifactError::Path(RuntimeAssetError::EmptyPath("manager_artifact"))
        );

        let relative = ManagerArtifactSelection {
            program_path: OsStr::new("manager/codex-manager"),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        assert_eq!(
            qualify_manager_artifact(generation, Some(&relative)).unwrap_err(),
            ManagerArtifactError::Path(RuntimeAssetError::RelativePath("manager_artifact"))
        );

        let nul_path = OsString::from_vec(vec![
            b'/', b'm', b'a', b'n', b'a', b'g', b'e', b'r', b'/', b'c', b'o', b'd', b'e', b'x', 0,
            b'm', b'a', b'n', b'a', b'g', b'e', b'r',
        ]);
        let nul = ManagerArtifactSelection {
            program_path: nul_path.as_os_str(),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        assert_eq!(
            qualify_manager_artifact(generation, Some(&nul)).unwrap_err(),
            ManagerArtifactError::Path(RuntimeAssetError::NulPath("manager_artifact"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_e_empty_and_mismatched_digests_fail_distinctly() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();

        let empty = ManagerArtifactSelection {
            program_path: OsStr::new("/manager/codex-manager"),
            observed_digest: "",
        };
        assert_eq!(
            qualify_manager_artifact(generation, Some(&empty)).unwrap_err(),
            ManagerArtifactError::EmptyDigest
        );

        let mismatch = ManagerArtifactSelection {
            program_path: OsStr::new("/manager/codex-manager"),
            observed_digest: "opaque-manager-digest:v1:different",
        };
        assert_eq!(
            qualify_manager_artifact(generation, Some(&mismatch)).unwrap_err(),
            ManagerArtifactError::DigestMismatch
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_f_raw_non_utf8_absolute_path_is_retained_exactly() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let mut manifest = m1_b11_valid_manifest();
        manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let raw = vec![
            b'/', b'm', b'a', b'n', b'a', b'g', b'e', b'r', b'/', 0xff, b'x',
        ];
        let path = OsString::from_vec(raw.clone());
        let selection = ManagerArtifactSelection {
            program_path: path.as_os_str(),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        let result = qualify_manager_artifact(generation, Some(&selection)).unwrap();
        let ManagerArtifactQualification::Available(qualified) = result else {
            panic!("raw Manager path must qualify");
        };
        assert_eq!(
            qualified.selection().program_path.as_bytes(),
            raw.as_slice()
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b21_g_qualification_is_deterministic_and_environment_pure() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:778899".to_string());
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let selection = ManagerArtifactSelection {
            program_path: OsStr::new("/manager/codex-manager"),
            observed_digest: "opaque-manager-digest:v1:778899",
        };
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        let first = qualify_manager_artifact(generation, Some(&selection)).unwrap();
        let second = qualify_manager_artifact(generation, Some(&selection)).unwrap();
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
        ];
        assert_eq!(before, after);
        let ManagerArtifactQualification::Available(first) = first else {
            panic!("first Manager qualification must be available");
        };
        let ManagerArtifactQualification::Available(second) = second else {
            panic!("second Manager qualification must be available");
        };
        assert!(std::ptr::eq(first.selection(), second.selection()));
        assert!(std::ptr::eq(first.selection(), &selection));
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b22_a_unavailable_is_static_zero_execution_and_does_not_consume_args() {
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("PATH"),
            std::env::var_os("CODEX_MANAGED_BY_NPM"),
        ];
        let manifest = m1_b11_valid_manifest();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let unavailable = qualify_manager_artifact(generation, None).unwrap();
        let args = std::iter::from_fn(|| -> Option<OsString> {
            panic!("Unavailable Manager must not consume trailing argv")
        });
        let outcome = execute_termux_manager(unavailable, args)
            .expect("Unavailable Manager is a bounded non-execution outcome");
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("PATH"),
            std::env::var_os("CODEX_MANAGED_BY_NPM"),
        ];

        assert_eq!(outcome, TermuxManagerOutcome::Unavailable);
        assert_eq!(outcome.message(), "Codex Termux Manager is unavailable.");
        assert!(!outcome.message().contains('/'));
        assert!(!outcome.message().contains("digest"));
        assert_eq!(before, after);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b22_b_failed_exec_is_typed_and_preserves_parent_environment() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.manager_artifact_digest = Some("opaque-manager-digest:v1:b22-missing".to_string());
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let selection = ManagerArtifactSelection {
            program_path: OsStr::new("/path/that/does/not/exist/codex-manager-b22"),
            observed_digest: "opaque-manager-digest:v1:b22-missing",
        };
        let qualification = qualify_manager_artifact(generation, Some(&selection)).unwrap();
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("PATH"),
            std::env::var_os("CODEX_MANAGED_BY_NPM"),
        ];

        let err = execute_termux_manager(
            qualification,
            [OsString::from("status"), OsString::from("--raw")],
        )
        .unwrap_err();
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("PATH"),
            std::env::var_os("CODEX_MANAGED_BY_NPM"),
        ];

        match err {
            ManagerLaunchError::Exec(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound)
            }
        }
        assert_eq!(before, after);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b22_c_real_exec_preserves_raw_args_environment_streams_and_exit_status() {
        use std::os::unix::ffi::OsStrExt;

        let result = run_exec_probe_with_env(
            "m1_b22_manager_exec",
            &[("CODEX_MANAGED_BY_NPM", OsStr::new("manager-must-inherit"))],
        );

        assert_eq!(result.status.code(), Some(73));
        let mut expected_stdout =
            b"MANAGER_NPM:manager-must-inherit\nARG:ordinary arg with spaces and =\nARG:".to_vec();
        expected_stdout.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).as_bytes());
        expected_stdout.extend_from_slice(b"\nMANAGER_STDOUT:");
        expected_stdout.extend_from_slice(&[1, 2, 3, 255, 254]);
        expected_stdout.extend_from_slice(b"\n");
        assert_eq!(result.stdout, expected_stdout);
        let mut expected_stderr = b"MANAGER_STDERR:".to_vec();
        expected_stderr.extend_from_slice(&[4, 5, 6, 128, 129]);
        expected_stderr.push(b'\n');
        assert_eq!(result.stderr, expected_stderr);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b22_d_real_exec_preserves_process_identity_and_sigterm_delivery() {
        let current_exe = std::env::current_exe().expect("failed to get current_exe");
        let shell = resolve_test_shell();

        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("tests::exec_probe_subprocess_entry")
            .arg("--exact")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .env(PROBE_ROLE_ENV, PROBE_ROLE_LAUNCHER)
            .env(PROBE_SHELL_ENV, &shell)
            .env(PROBE_SCENARIO_ENV, "m1_b22_manager_signal");

        let mut child = cmd.spawn().expect("failed to spawn B22 probe child");
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
                "B22 readiness marker not found before EOF",
            )));
        });

        let ready_line = match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(line)) => line,
            Ok(Err(err)) => panic!("I/O error reading B22 readiness: {err}"),
            Err(err) => panic!("timeout waiting for B22 Manager readiness: {err}"),
        };
        let reported_pid: u32 = ready_line
            .strip_prefix("READY:PID:")
            .unwrap_or_else(|| panic!("unexpected B22 readiness line: {ready_line:?}"))
            .parse()
            .expect("parse B22 Manager pid");
        assert_eq!(reported_pid, child_pid);

        const SIGTERM: std::os::raw::c_int = 15;
        assert_eq!(
            unsafe { kill(reported_pid as std::os::raw::c_int, SIGTERM) },
            0
        );

        let child_ref = guard.0.as_mut().expect("child must be present");
        let start = std::time::Instant::now();
        let status = loop {
            match child_ref.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if start.elapsed() > std::time::Duration::from_secs(5) {
                        panic!("timed out waiting for B22 Manager after SIGTERM");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(err) => panic!("error waiting for B22 Manager: {err}"),
            }
        };
        assert_eq!(status.code(), Some(73));
        guard.0 = None;
        let _ = reader_handle.join();
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b23_a_context_rejects_mixed_generation_before_any_route() {
        let mut runtime_manifest = m1_b11_valid_manifest();
        runtime_manifest.helper_digests.clear();
        let manager_manifest = runtime_manifest.clone();
        let runtime_generation =
            qualify_generation_manifest(&runtime_manifest, &m1_b11_requirements()).unwrap();
        let manager_generation =
            qualify_generation_manifest(&manager_manifest, &m1_b11_requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/unused/b23/runtime"),
                observed_digest: runtime_manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/unused/b23/compat"),
            helpers: &[],
        };
        let runtime = qualify_runtime_assets(runtime_generation, &selection).unwrap();
        let manager = qualify_manager_artifact(manager_generation, None).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        assert_eq!(
            build_local_public_dispatch_context(
                runtime,
                manager,
                &snapshot,
                OsStr::new("/unused/b23/cert"),
                None,
                std::path::Path::new("/unused/b23/resolver"),
                std::path::Path::new("/unused/b23/config"),
                UpstreamDoctorCapability::Supported,
                CoreDoctorStatus::Healthy,
                ManagerDoctorStatus::Healthy,
            )
            .unwrap_err(),
            LocalPublicDispatchContextError::GenerationMismatch
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b23_b_update_is_zero_io_and_preserves_raw_trailing_argv() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/unused/b23/runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/unused/b23/compat"),
            helpers: &[],
        };
        let runtime = qualify_runtime_assets(generation, &selection).unwrap();
        let manager = qualify_manager_artifact(generation, None).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let context = build_local_public_dispatch_context(
            runtime,
            manager,
            &snapshot,
            OsStr::new("/path/that/does/not/exist/b23-cert"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b23-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b23-config"),
            UpstreamDoctorCapability::Supported,
            CoreDoctorStatus::ApiIncompatible,
            ManagerDoctorStatus::Unhealthy,
        )
        .unwrap();
        let raw = OsString::from_vec(vec![0xff, 0xfe, 0x80]);
        let expected = vec![OsString::from("--local"), raw.clone()];
        let before = [std::env::var_os("PREFIX"), std::env::var_os("PATH")];
        let outcome =
            execute_public_dispatch(PublicDispatchRoute::Update(expected.clone()), context)
                .unwrap();
        let after = [std::env::var_os("PREFIX"), std::env::var_os("PATH")];
        let PublicDispatchCompletion::Update(actual) = outcome else {
            panic!("Update must remain a typed M1 handoff");
        };
        assert_eq!(actual, expected);
        assert_eq!(actual[1].as_os_str().as_bytes(), raw.as_os_str().as_bytes());
        assert_eq!(before, after);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b23_c_doctor_preserves_usage_before_io_and_bounded_success() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/unused/b23/runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/unused/b23/compat"),
            helpers: &[],
        };
        let runtime = qualify_runtime_assets(generation, &selection).unwrap();
        let manager = qualify_manager_artifact(generation, None).unwrap();
        let invalid_snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let invalid_context = build_local_public_dispatch_context(
            runtime,
            manager,
            &invalid_snapshot,
            OsStr::new("/missing/b23/cert"),
            None,
            std::path::Path::new("/missing/b23/resolver"),
            std::path::Path::new("/missing/b23/config"),
            UpstreamDoctorCapability::Supported,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        )
        .unwrap();
        match execute_public_dispatch(
            PublicDispatchRoute::Doctor(vec![OsString::from("--bad")]),
            invalid_context,
        ) {
            Err(PublicDispatchExecutionError::Doctor(LocalDoctorCommandError::Usage(
                DoctorUsageError::InvalidArguments,
            ))) => {}
            other => panic!("doctor usage must fail before I/O, got {other:?}"),
        }

        let supported_without_probe = build_local_public_dispatch_context(
            runtime,
            manager,
            &invalid_snapshot,
            OsStr::new("/missing/b23/cert"),
            None,
            std::path::Path::new("/missing/b23/resolver"),
            std::path::Path::new("/missing/b23/config"),
            UpstreamDoctorCapability::Unsupported,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        )
        .unwrap();
        let outcome = execute_public_dispatch(
            PublicDispatchRoute::Doctor(vec![OsString::from("--json")]),
            supported_without_probe,
        )
        .unwrap();
        let PublicDispatchCompletion::Doctor(outcome) = outcome else {
            panic!("Doctor route must return bounded doctor outcome");
        };
        assert_eq!(outcome.exit_class, DoctorExitClass::HealthFailure);
        assert_eq!(
            outcome.output,
            concat!(
                r#"{"schema_version":1,"upstream":{"status":"unsupported"},"termux_core":{"status":"healthy"},"manager":{"status":"unavailable"},"summary":{"status":"degraded"}}"#,
                "\n"
            )
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b23_d_termux_unavailable_skips_all_other_context_io() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/missing/b23/runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/missing/b23/compat"),
            helpers: &[],
        };
        let runtime = qualify_runtime_assets(generation, &selection).unwrap();
        let manager = qualify_manager_artifact(generation, None).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let context = build_local_public_dispatch_context(
            runtime,
            manager,
            &snapshot,
            OsStr::new("/missing/b23/cert"),
            None,
            std::path::Path::new("/missing/b23/resolver"),
            std::path::Path::new("/missing/b23/config"),
            UpstreamDoctorCapability::Supported,
            CoreDoctorStatus::ApiIncompatible,
            ManagerDoctorStatus::ApiIncompatible,
        )
        .unwrap();
        let outcome = execute_public_dispatch(
            PublicDispatchRoute::Termux(vec![OsString::from("status")]),
            context,
        )
        .unwrap();
        assert_eq!(
            outcome,
            PublicDispatchCompletion::TermuxUnavailable(TermuxManagerOutcome::Unavailable)
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b23_e_upstream_route_crosses_b14_with_complete_raw_argv() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        let root =
            std::env::temp_dir().join(format!("codex-m1-b23-upstream-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let resolver = root.join("resolv.conf");
        let config = root.join("config");
        let runtime = root.join("runtime.sh");
        std::fs::write(&resolver, b"nameserver 203.0.113.23\n").unwrap();
        std::fs::create_dir_all(&config).unwrap();
        let shell = resolve_test_shell();
        let script = format!(
            r#"#!{}
for a in "$@"; do
    printf 'ARG:'
    printf '%s' "$a"
    printf '\n'
done
exit 0
"#,
            shell.to_str().unwrap()
        );
        std::fs::write(&runtime, script).unwrap();
        let mut perms = std::fs::metadata(&runtime).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&runtime, perms).unwrap();

        let result = run_exec_probe_with_env(
            "m1_b23_upstream_dispatch",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        let mut expected = Vec::new();
        expected.extend_from_slice(b"ARG:-c\n");
        expected.extend_from_slice(b"ARG:sandbox_mode=\"danger-full-access\"\n");
        expected.extend_from_slice(b"ARG:exec\n");
        expected.extend_from_slice(b"ARG:b23-task\n");
        expected.extend_from_slice(b"ARG:");
        expected.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).as_bytes());
        expected.push(b'\n');
        assert_eq!(result.stdout, expected);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b23_f_available_termux_route_crosses_b22_only() {
        use std::os::unix::ffi::OsStrExt;

        let result = run_exec_probe_with_env(
            "m1_b23_manager_dispatch",
            &[("CODEX_MANAGED_BY_NPM", OsStr::new("b23-manager-inherited"))],
        );
        assert_eq!(result.status.code(), Some(71));
        assert_eq!(result.stderr, b"");
        let mut expected = Vec::new();
        expected.extend_from_slice(b"B23_MANAGER_ENV:b23-manager-inherited\n");
        expected.extend_from_slice(b"ARG:b23 ordinary\n");
        expected.extend_from_slice(b"ARG:");
        expected.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80]).as_bytes());
        expected.push(b'\n');
        assert_eq!(result.stdout, expected);
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct M1B24StableMetadata {
        dev: u64,
        ino: u64,
        mode: u32,
        uid: u32,
        gid: u32,
        size: u64,
        mtime: i64,
        mtime_nsec: i64,
    }

    #[cfg(unix)]
    fn m1_b24_stable_metadata(metadata: &std::fs::Metadata) -> M1B24StableMetadata {
        use std::os::unix::fs::MetadataExt;

        M1B24StableMetadata {
            dev: metadata.dev(),
            ino: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        }
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct M1B24ProtectedFileSnapshot {
        path: std::path::PathBuf,
        link_metadata: M1B24StableMetadata,
        link_target: Option<std::path::PathBuf>,
        target_metadata: M1B24StableMetadata,
        content: Vec<u8>,
    }

    #[cfg(unix)]
    fn m1_b24_snapshot_protected_file(
        path: &std::path::Path,
    ) -> std::io::Result<M1B24ProtectedFileSnapshot> {
        let link_metadata = std::fs::symlink_metadata(path)?;
        let link_target = if link_metadata.file_type().is_symlink() {
            Some(std::fs::read_link(path)?)
        } else {
            None
        };
        let target_metadata = std::fs::metadata(path)?;
        let content = std::fs::read(path)?;
        Ok(M1B24ProtectedFileSnapshot {
            path: path.to_path_buf(),
            link_metadata: m1_b24_stable_metadata(&link_metadata),
            link_target,
            target_metadata: m1_b24_stable_metadata(&target_metadata),
            content,
        })
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b24_a_raw_entrypoint_composes_b20_and_b23_without_update_io() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/missing/b24/runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/missing/b24/compat"),
            helpers: &[],
        };
        let runtime = qualify_runtime_assets(generation, &selection).unwrap();
        let manager = qualify_manager_artifact(generation, None).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let context = build_local_public_dispatch_context(
            runtime,
            manager,
            &snapshot,
            OsStr::new("/missing/b24/cert"),
            None,
            std::path::Path::new("/missing/b24/resolver"),
            std::path::Path::new("/missing/b24/config"),
            UpstreamDoctorCapability::Supported,
            CoreDoctorStatus::ApiIncompatible,
            ManagerDoctorStatus::Unhealthy,
        )
        .unwrap();
        let raw = OsString::from_vec(vec![0xff, 0xfe, 0x80]);
        let before = [std::env::var_os("PREFIX"), std::env::var_os("PATH")];
        let outcome = execute_public_entrypoint(
            [
                OsString::from("update"),
                OsString::from("--local"),
                raw.clone(),
            ],
            context,
        )
        .unwrap();
        let after = [std::env::var_os("PREFIX"), std::env::var_os("PATH")];

        let PublicDispatchCompletion::Update(args) = outcome else {
            panic!("B24 update entrypoint must remain an M1 typed handoff");
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], OsString::from("--local"));
        assert_eq!(args[1].as_os_str().as_bytes(), raw.as_os_str().as_bytes());
        assert_eq!(before, after);
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "explicit real-Termux Milestone 1 smoke gate"]
    fn test_m1_b24_real_termux_smoke_live_resolver_read_only() {
        use std::os::unix::fs::PermissionsExt;

        assert_eq!(std::env::consts::ARCH, "aarch64");
        assert_eq!(std::env::consts::OS, "android");
        assert!(
            std::env::var_os("TERMUX_VERSION").is_some(),
            "B24 smoke requires a real Termux environment"
        );

        let prefix = std::env::var_os("PREFIX").expect("B24 smoke requires PREFIX");
        let prefix = std::path::PathBuf::from(prefix);
        let resolver = prefix.join("etc/resolv.conf");
        let launcher = prefix.join("bin/codex");
        let resolver_before = m1_b24_snapshot_protected_file(&resolver)
            .expect("snapshot live resolver before B24 smoke");
        let launcher_before = m1_b24_snapshot_protected_file(&launcher)
            .expect("snapshot installed launcher before B24 smoke");

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock must be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "codex-m1-b24-smoke-{}-{unique}",
            std::process::id()
        ));
        assert!(
            !root.starts_with(&prefix),
            "B24 writable smoke root must not be under PREFIX"
        );
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::create_dir_all(root.join("compat")).unwrap();
        std::fs::write(root.join("cert.pem"), b"test-owned-b24-cert\n").unwrap();
        let runtime = root.join("fake-upstream.sh");
        let shell = resolve_test_shell();
        let script = format!(
            r#"#!{}
if [ "$#" -eq 3 ] && [ "$1" = "-c" ] && [ "$2" = 'sandbox_mode="danger-full-access"' ] && [ "$3" = "--version" ]; then
    [ -r /proc/self/fd/33 ] || exit 91
    [ -d /proc/self/fd/34 ] || exit 92
elif [ "$#" -eq 1 ] && [ "$1" = "--version" ]; then
    :
else
    printf 'unexpected argv\n' >&2
    exit 93
fi
printf 'codex-upstream 9.9.9\n'
printf 'upstream-version-stderr\n' >&2
exit 0
"#,
            shell.to_str().expect("test shell path must be UTF-8")
        );
        std::fs::write(&runtime, script).unwrap();
        let mut permissions = std::fs::metadata(&runtime).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&runtime, permissions).unwrap();

        let direct = std::process::Command::new(&runtime)
            .arg("--version")
            .output()
            .expect("run direct fake upstream version");
        let through_core = run_exec_probe_with_env(
            "m1_b24_real_termux_entrypoint",
            &[
                (PROBE_CONFIG_DIR_PATH_ENV, root.join("config").as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
            ],
        );

        assert_eq!(through_core.status.code(), direct.status.code());
        assert_eq!(through_core.stdout, direct.stdout);
        assert_eq!(through_core.stderr, direct.stderr);
        assert_eq!(direct.status.code(), Some(0));
        assert_eq!(direct.stdout, b"codex-upstream 9.9.9\n");
        assert_eq!(direct.stderr, b"upstream-version-stderr\n");

        let resolver_after = m1_b24_snapshot_protected_file(&resolver)
            .expect("snapshot live resolver after B24 smoke");
        let launcher_after = m1_b24_snapshot_protected_file(&launcher)
            .expect("snapshot installed launcher after B24 smoke");
        assert_eq!(resolver_after, resolver_before);
        assert_eq!(launcher_after, launcher_before);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(unix)]
    fn m2_b1_unique_paths(label: &str) -> CoreStatePaths {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-m2-b1-{label}-{}-{counter}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let paths = CoreStatePaths::new(&root).expect("M2-B1 temp root must be valid");
        prepare_core_state_paths(&paths).expect("prepare M2-B1 temp root");
        paths
    }

    #[cfg(unix)]
    fn m2_b1_cleanup(paths: &CoreStatePaths) {
        let _ = std::fs::remove_dir_all(&paths.root);
    }

    #[cfg(unix)]
    fn m2_b1_write_state(paths: &CoreStatePaths, state: &GenerationPointerState) {
        std::fs::write(
            &paths.activation_state,
            encode_pointer_state(state).expect("encode test pointer state"),
        )
        .expect("write test pointer state");
    }

    #[cfg(unix)]
    fn m2_b1_write_journal(paths: &CoreStatePaths, journal: &ActivationJournal) {
        std::fs::write(
            &paths.activation_journal,
            encode_activation_journal(journal).expect("encode test activation journal"),
        )
        .expect("write test activation journal");
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum M2B1FaultTiming {
        Before,
        After,
    }

    #[cfg(unix)]
    struct M2B1FaultIo {
        fail_call: usize,
        timing: M2B1FaultTiming,
        calls: usize,
        kind: std::io::ErrorKind,
        inner: FsActivationIo,
    }

    #[cfg(unix)]
    impl M2B1FaultIo {
        fn new(fail_call: usize, timing: M2B1FaultTiming) -> Self {
            Self {
                fail_call,
                timing,
                calls: 0,
                kind: std::io::ErrorKind::Other,
                inner: FsActivationIo,
            }
        }

        fn with_kind(fail_call: usize, timing: M2B1FaultTiming, kind: std::io::ErrorKind) -> Self {
            Self {
                fail_call,
                timing,
                calls: 0,
                kind,
                inner: FsActivationIo,
            }
        }

        fn around<T>(
            &mut self,
            action: impl FnOnce(&mut FsActivationIo) -> std::io::Result<T>,
        ) -> std::io::Result<T> {
            self.calls += 1;
            let current = self.calls;
            if current == self.fail_call && self.timing == M2B1FaultTiming::Before {
                return Err(std::io::Error::new(
                    self.kind,
                    "injected M2-B1 fault before durable call",
                ));
            }
            let result = action(&mut self.inner)?;
            if current == self.fail_call && self.timing == M2B1FaultTiming::After {
                return Err(std::io::Error::new(
                    self.kind,
                    "injected M2-B1 fault after durable call",
                ));
            }
            Ok(result)
        }
    }

    #[cfg(unix)]
    impl ActivationIo for M2B1FaultIo {
        fn write_new_synced(&mut self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
            self.around(|inner| inner.write_new_synced(path, data))
        }

        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.around(|inner| inner.sync_dir(path))
        }

        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
            self.around(|inner| inner.rename(from, to))
        }

        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.around(|inner| inner.remove_file(path))
        }
    }

    #[cfg(unix)]
    fn m2_b1_assert_no_transaction_files(paths: &CoreStatePaths) {
        assert!(!paths.activation_journal.exists());
        assert!(!paths.activation_journal_temp.exists());
        assert!(!paths.activation_state_temp.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_a_paths_state_codec_and_identity_validation_are_strict() {
        use std::os::unix::ffi::OsStringExt;

        assert_eq!(
            CoreStatePaths::new(std::path::Path::new("")),
            Err(StateFormatError::EmptyRoot)
        );
        assert_eq!(
            CoreStatePaths::new(std::path::Path::new("relative/root")),
            Err(StateFormatError::RelativeRoot)
        );
        let nul = std::ffi::OsString::from_vec(vec![b'/', b't', b'm', b'p', 0, b'x']);
        assert_eq!(
            CoreStatePaths::new(std::path::Path::new(&nul)),
            Err(StateFormatError::NulRoot)
        );

        let paths = m2_b1_unique_paths("codec");
        assert_eq!(paths.generations, paths.root.join("generations"));
        assert_eq!(paths.activation_state, paths.root.join("activation-state"));
        assert_eq!(
            paths.activation_journal,
            paths.root.join("activation-journal")
        );
        assert_eq!(
            paths.activation_journal_temp,
            paths.root.join("activation-journal.tmp")
        );
        assert_eq!(
            paths.activation_state_temp,
            paths.root.join("activation-state.tmp")
        );

        let state = GenerationPointerState {
            current: "generation = alpha 한국어".to_string(),
            verified: "verified:β".to_string(),
            previous: Some("previous value".to_string()),
        };
        let encoded = encode_pointer_state(&state).unwrap();
        assert_eq!(
            encoded,
            "format=codex-activation-state-v1\ncurrent=generation = alpha 한국어\nverified=verified:β\nprevious_present=1\nprevious=previous value\n".as_bytes()
        );
        assert_eq!(parse_pointer_state(&encoded).unwrap(), state);

        for bad in ["", "line\nbreak", "line\rbreak", "nul\0byte"] {
            assert!(
                plan_initial_pointer_state(bad).is_err(),
                "bad identity {bad:?}"
            );
        }
        let too_long = "x".repeat(513);
        assert_eq!(
            plan_initial_pointer_state(&too_long),
            Err(StateFormatError::IdentityTooLong("candidate"))
        );
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_b_state_and_journal_parsers_fail_closed_on_malformed_inputs() {
        let valid = GenerationPointerState {
            current: "g1".to_string(),
            verified: "g1".to_string(),
            previous: None,
        };
        let valid_bytes = encode_pointer_state(&valid).unwrap();
        assert_eq!(parse_pointer_state(&valid_bytes).unwrap(), valid);

        let malformed_states: Vec<Vec<u8>> = vec![
            b"format=codex-activation-state-v2\ncurrent=g1\nverified=g1\nprevious_present=0\nprevious=\n".to_vec(),
            b"format=codex-activation-state-v1\nverified=g1\ncurrent=g1\nprevious_present=0\nprevious=\n".to_vec(),
            b"format=codex-activation-state-v1\ncurrent=g1\nverified=g1\nprevious_present=2\nprevious=\n".to_vec(),
            b"format=codex-activation-state-v1\ncurrent=g1\nverified=g1\nprevious_present=0\nprevious=ghost\n".to_vec(),
            b"format=codex-activation-state-v1\ncurrent=g1\nverified=g1\nprevious_present=0\n".to_vec(),
            b"format=codex-activation-state-v1\ncurrent=g1\nverified=g1\nprevious_present=0\nprevious=\nextra=x\n".to_vec(),
            b"format=codex-activation-state-v1\ncurrent=g1\nverified=g1\nprevious_present=0\nprevious=".to_vec(),
            vec![0xff, 0xfe, 0xfd, b'\n'],
            vec![b'x'; 20_000],
        ];
        for malformed in malformed_states {
            assert!(parse_pointer_state(&malformed).is_err());
        }

        let after = GenerationPointerState {
            current: "g2".to_string(),
            verified: "g2".to_string(),
            previous: Some("g1".to_string()),
        };
        let journal = ActivationJournal {
            before: Some(valid.clone()),
            after: after.clone(),
        };
        let encoded = encode_activation_journal(&journal).unwrap();
        assert_eq!(parse_activation_journal(&encoded).unwrap(), journal);
        assert_eq!(
            encode_activation_journal(&ActivationJournal {
                before: Some(after.clone()),
                after: after.clone(),
            }),
            Err(StateFormatError::AmbiguousJournal)
        );
        let absent_with_data = b"format=codex-activation-journal-v1\nbefore_present=0\nbefore_current=g1\nbefore_verified=\nbefore_previous_present=0\nbefore_previous=\nafter_current=g2\nafter_verified=g2\nafter_previous_present=1\nafter_previous=g1\n";
        assert!(matches!(
            parse_activation_journal(absent_with_data),
            Err(StateFormatError::InconsistentAbsent("journal before state"))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_c_initial_activation_upgrade_and_rollback_semantics_are_exact() {
        let initial = plan_initial_pointer_state("g1").unwrap();
        assert_eq!(
            initial,
            GenerationPointerState {
                current: "g1".to_string(),
                verified: "g1".to_string(),
                previous: None,
            }
        );
        let upgraded = plan_activation_pointer_state(&initial, "g2").unwrap();
        assert_eq!(
            upgraded,
            GenerationPointerState {
                current: "g2".to_string(),
                verified: "g2".to_string(),
                previous: Some("g1".to_string()),
            }
        );
        let rollback = plan_rollback_pointer_state(&upgraded).unwrap();
        assert_eq!(
            rollback,
            GenerationPointerState {
                current: "g1".to_string(),
                verified: "g1".to_string(),
                previous: Some("g2".to_string()),
            }
        );
        assert_eq!(
            plan_activation_pointer_state(&initial, "g1"),
            Err(StateFormatError::NoChange)
        );
        assert_eq!(
            plan_rollback_pointer_state(&initial),
            Err(StateFormatError::NoRollbackGeneration)
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_d_real_activation_and_rollback_leave_only_one_authoritative_state() {
        let paths = m2_b1_unique_paths("real");
        let initial = plan_initial_pointer_state("g1").unwrap();
        activate_pointer_state(&paths, None, &initial).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(initial.clone()));
        m2_b1_assert_no_transaction_files(&paths);

        let upgraded = plan_activation_pointer_state(&initial, "g2").unwrap();
        activate_pointer_state(&paths, Some(&initial), &upgraded).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(upgraded.clone()));
        m2_b1_assert_no_transaction_files(&paths);

        let rollback = plan_rollback_pointer_state(&upgraded).unwrap();
        activate_pointer_state(&paths, Some(&upgraded), &rollback).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(rollback));
        m2_b1_assert_no_transaction_files(&paths);
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_e_every_durable_boundary_recovers_to_exact_old_or_new_state() {
        for timing in [M2B1FaultTiming::Before, M2B1FaultTiming::After] {
            for fail_call in 1..=8 {
                let paths = m2_b1_unique_paths(&format!("fault-{timing:?}-{fail_call}"));
                let old = plan_initial_pointer_state("g1").unwrap();
                activate_pointer_state(&paths, None, &old).unwrap();
                let new = plan_activation_pointer_state(&old, "g2").unwrap();
                let mut io = M2B1FaultIo::new(fail_call, timing);
                let err = activate_pointer_state_with_io(&paths, Some(&old), &new, &mut io)
                    .expect_err("injected durable-boundary fault must abort activation call");
                assert!(matches!(err, ActivationTransactionError::Io { .. }));
                assert_eq!(io.calls, fail_call);

                let recovered = recover_activation_state(&paths).unwrap();
                let rename_completed = match timing {
                    M2B1FaultTiming::Before => fail_call >= 6,
                    M2B1FaultTiming::After => fail_call >= 5,
                };
                assert_eq!(
                    recovered,
                    Some(if rename_completed {
                        new.clone()
                    } else {
                        old.clone()
                    }),
                    "timing={timing:?} fail_call={fail_call}"
                );
                m2_b1_assert_no_transaction_files(&paths);
                assert_eq!(recover_activation_state(&paths).unwrap(), recovered);
                m2_b1_cleanup(&paths);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_f_initial_activation_fault_matrix_never_fabricates_previous_state() {
        for timing in [M2B1FaultTiming::Before, M2B1FaultTiming::After] {
            for fail_call in 1..=8 {
                let paths = m2_b1_unique_paths(&format!("initial-{timing:?}-{fail_call}"));
                let new = plan_initial_pointer_state("g1").unwrap();
                let mut io = M2B1FaultIo::new(fail_call, timing);
                activate_pointer_state_with_io(&paths, None, &new, &mut io)
                    .expect_err("injected initial-activation fault must abort call");
                let recovered = recover_activation_state(&paths).unwrap();
                let rename_completed = match timing {
                    M2B1FaultTiming::Before => fail_call >= 6,
                    M2B1FaultTiming::After => fail_call >= 5,
                };
                if rename_completed {
                    assert_eq!(recovered, Some(new.clone()));
                    assert_eq!(recovered.as_ref().unwrap().previous, None);
                } else {
                    assert_eq!(recovered, None);
                }
                m2_b1_assert_no_transaction_files(&paths);
                m2_b1_cleanup(&paths);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_g_partial_temporaries_and_stale_journal_recover_idempotently() {
        let paths = m2_b1_unique_paths("partial-journal-temp");
        let old = plan_initial_pointer_state("g1").unwrap();
        activate_pointer_state(&paths, None, &old).unwrap();
        std::fs::write(&paths.activation_journal_temp, b"partial-journal").unwrap();
        assert_eq!(recover_activation_state(&paths).unwrap(), Some(old.clone()));
        m2_b1_assert_no_transaction_files(&paths);
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("partial-state-temp");
        activate_pointer_state(&paths, None, &old).unwrap();
        let new = plan_activation_pointer_state(&old, "g2").unwrap();
        m2_b1_write_journal(
            &paths,
            &ActivationJournal {
                before: Some(old.clone()),
                after: new.clone(),
            },
        );
        std::fs::write(&paths.activation_state_temp, b"partial-state").unwrap();
        assert_eq!(recover_activation_state(&paths).unwrap(), Some(old.clone()));
        m2_b1_assert_no_transaction_files(&paths);
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("stale-journal-new");
        activate_pointer_state(&paths, None, &old).unwrap();
        m2_b1_write_state(&paths, &new);
        m2_b1_write_journal(
            &paths,
            &ActivationJournal {
                before: Some(old.clone()),
                after: new.clone(),
            },
        );
        assert_eq!(recover_activation_state(&paths).unwrap(), Some(new.clone()));
        assert_eq!(recover_activation_state(&paths).unwrap(), Some(new));
        m2_b1_assert_no_transaction_files(&paths);
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_h_recovery_conflicts_and_malformed_files_fail_closed() {
        let paths = m2_b1_unique_paths("conflict-third-state");
        let old = plan_initial_pointer_state("g1").unwrap();
        activate_pointer_state(&paths, None, &old).unwrap();
        let expected_new = plan_activation_pointer_state(&old, "g2").unwrap();
        let third = plan_activation_pointer_state(&old, "g3").unwrap();
        m2_b1_write_state(&paths, &third);
        m2_b1_write_journal(
            &paths,
            &ActivationJournal {
                before: Some(old.clone()),
                after: expected_new.clone(),
            },
        );
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::RecoveryConflict)
        ));
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(third));
        assert!(paths.activation_journal.exists());
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("conflict-missing-state");
        m2_b1_write_journal(
            &paths,
            &ActivationJournal {
                before: Some(old.clone()),
                after: expected_new.clone(),
            },
        );
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::RecoveryConflict)
        ));
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("malformed-journal");
        activate_pointer_state(&paths, None, &old).unwrap();
        std::fs::write(&paths.activation_journal, b"partial-canonical-journal").unwrap();
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::Format(_))
        ));
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(old.clone()));
        assert!(paths.activation_journal.exists());
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("orphan-state-temp");
        activate_pointer_state(&paths, None, &old).unwrap();
        std::fs::write(&paths.activation_state_temp, b"orphan").unwrap();
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::OrphanTemporaryState)
        ));
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("both-temps-no-journal");
        activate_pointer_state(&paths, None, &old).unwrap();
        std::fs::write(&paths.activation_journal_temp, b"journal-temp").unwrap();
        std::fs::write(&paths.activation_state_temp, b"state-temp").unwrap();
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::RecoveryConflict)
        ));
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_i_collisions_permissions_and_unsafe_file_types_fail_closed() {
        use std::os::unix::fs::symlink;

        let paths = m2_b1_unique_paths("pending-journal");
        let initial = plan_initial_pointer_state("g1").unwrap();
        std::fs::write(&paths.activation_journal, b"collision").unwrap();
        assert!(matches!(
            activate_pointer_state(&paths, None, &initial),
            Err(ActivationTransactionError::PendingJournal)
        ));
        assert_eq!(read_pointer_state(&paths).unwrap(), None);
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("pending-journal-temp");
        std::fs::write(&paths.activation_journal_temp, b"collision").unwrap();
        assert!(matches!(
            activate_pointer_state(&paths, None, &initial),
            Err(ActivationTransactionError::OrphanJournalTemporary)
        ));
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("pending-state-temp");
        std::fs::write(&paths.activation_state_temp, b"collision").unwrap();
        assert!(matches!(
            activate_pointer_state(&paths, None, &initial),
            Err(ActivationTransactionError::OrphanTemporaryState)
        ));
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("permission");
        let mut io = M2B1FaultIo::with_kind(
            1,
            M2B1FaultTiming::Before,
            std::io::ErrorKind::PermissionDenied,
        );
        match activate_pointer_state_with_io(&paths, None, &initial, &mut io) {
            Err(ActivationTransactionError::Io { source, .. }) => {
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied)
            }
            other => panic!("expected typed PermissionDenied, got {other:?}"),
        }
        assert_eq!(recover_activation_state(&paths).unwrap(), None);
        m2_b1_cleanup(&paths);

        let paths = m2_b1_unique_paths("state-symlink");
        let outside = paths.root.with_extension("outside-state");
        std::fs::write(&outside, encode_pointer_state(&initial).unwrap()).unwrap();
        symlink(&outside, &paths.activation_state).unwrap();
        assert!(matches!(
            read_pointer_state(&paths),
            Err(ActivationTransactionError::UnsafeFileType(
                "activation state"
            ))
        ));
        assert_eq!(
            std::fs::read(&outside).unwrap(),
            encode_pointer_state(&initial).unwrap()
        );
        let _ = std::fs::remove_file(&outside);
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_j_activation_never_writes_generation_or_outside_root_content() {
        let missing_parent = std::env::temp_dir().join(format!(
            "codex-m2-b1-missing-parent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested_root = missing_parent.join("state-root");
        let nested_paths = CoreStatePaths::new(&nested_root).unwrap();
        assert!(prepare_core_state_paths(&nested_paths).is_err());
        assert!(
            !missing_parent.exists(),
            "preparing an explicit root must not create missing ancestors"
        );

        let paths = m2_b1_unique_paths("boundaries");
        let outside = paths.root.with_extension("outside-sentinel");
        let unrelated = paths.root.join("unrelated-sentinel");
        let generation_dir = paths.generations.join("opaque-g1").join("nested");
        std::fs::create_dir_all(&generation_dir).unwrap();
        let generation_file = generation_dir.join("runtime.bin");
        std::fs::write(&generation_file, b"immutable-generation-bytes\0\xff").unwrap();
        std::fs::write(&outside, b"outside-root-sentinel").unwrap();
        std::fs::write(&unrelated, b"unrelated-root-sentinel").unwrap();
        let generation_before = std::fs::read(&generation_file).unwrap();
        let outside_before = std::fs::read(&outside).unwrap();
        let unrelated_before = std::fs::read(&unrelated).unwrap();

        let initial = plan_initial_pointer_state("opaque-g1").unwrap();
        activate_pointer_state(&paths, None, &initial).unwrap();
        let upgraded = plan_activation_pointer_state(&initial, "opaque-g2").unwrap();
        activate_pointer_state(&paths, Some(&initial), &upgraded).unwrap();

        assert_eq!(std::fs::read(&generation_file).unwrap(), generation_before);
        assert_eq!(std::fs::read(&outside).unwrap(), outside_before);
        assert_eq!(std::fs::read(&unrelated).unwrap(), unrelated_before);
        assert!(!paths.generations.join("opaque-g2").exists());
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(upgraded));
        m2_b1_assert_no_transaction_files(&paths);

        let _ = std::fs::remove_file(&outside);
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_k_recovery_rejects_canonical_journal_plus_journal_temporary() {
        let paths = m2_b1_unique_paths("double-journal");
        let old = plan_initial_pointer_state("g1").unwrap();
        activate_pointer_state(&paths, None, &old).unwrap();
        let new = plan_activation_pointer_state(&old, "g2").unwrap();
        m2_b1_write_journal(
            &paths,
            &ActivationJournal {
                before: Some(old.clone()),
                after: new,
            },
        );
        std::fs::write(&paths.activation_journal_temp, b"unexpected-second-temp").unwrap();
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::OrphanJournalTemporary)
        ));
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(old));
        assert!(paths.activation_journal.exists());
        assert!(paths.activation_journal_temp.exists());
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_l_stale_before_state_rejects_new_activation_without_mutation() {
        let paths = m2_b1_unique_paths("stale-before");
        let actual = plan_initial_pointer_state("g1").unwrap();
        activate_pointer_state(&paths, None, &actual).unwrap();
        let stale = plan_initial_pointer_state("stale").unwrap();
        let requested = plan_activation_pointer_state(&stale, "g2").unwrap();
        assert!(matches!(
            activate_pointer_state(&paths, Some(&stale), &requested),
            Err(ActivationTransactionError::StaleAuthoritativeState)
        ));
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(actual));
        m2_b1_assert_no_transaction_files(&paths);
        m2_b1_cleanup(&paths);
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

    #[cfg(unix)]
    #[test]
    fn test_m1_b14_a_invalid_process_snapshot_fails_before_runtime_io() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/path/that/does/not/exist/b14-runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/qualified/b14/compat"),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("runtime asset shape must qualify");
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: Some(OsString::from("/qualified/b14/tmp")),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let err = launch_qualified_runtime(
            qualified,
            &snapshot,
            OsStr::new("/qualified/b14/cert.pem"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b14-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b14-config"),
            [OsStr::new("--version")],
        );
        match err {
            QualifiedRuntimeLaunchError::Environment(env_err) => {
                assert_eq!(env_err, TermuxProcessEnvError::MissingRequired("PREFIX"));
            }
            QualifiedRuntimeLaunchError::Launch(launch_err) => {
                panic!("environment planning must fail before runtime I/O: {launch_err}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b14_b_sandbox_policy_still_fails_before_runtime_io() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/path/that/does/not/exist/b14-runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/qualified/b14/compat"),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("runtime asset shape must qualify");
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(OsString::from("/qualified/b14/prefix")),
            tmpdir: Some(OsString::from("/qualified/b14/tmp")),
            inherited_path: Some(OsString::from("/inherited/b14/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let err = launch_qualified_runtime(
            qualified,
            &snapshot,
            OsStr::new("/qualified/b14/cert.pem"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b14-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b14-config"),
            [OsStr::new("-s"), OsStr::new("read-only")],
        );
        match err {
            QualifiedRuntimeLaunchError::Launch(LaunchError::Policy(policy_err)) => {
                assert_eq!(
                    policy_err,
                    PassthroughError::UnsupportedSandboxMode("read-only".to_string())
                );
            }
            QualifiedRuntimeLaunchError::Launch(LaunchError::Exec(exec_err)) => {
                panic!("sandbox policy must fail before resolver/config I/O: {exec_err}");
            }
            QualifiedRuntimeLaunchError::Environment(env_err) => {
                panic!("valid process snapshot unexpectedly failed: {env_err}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b14_c_qualified_assets_drive_real_exec_composition() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let test_root = temp_dir.join(format!("codex-test-m1-b14-real-exec-{pid}"));
        let _ = std::fs::remove_dir_all(&test_root);
        std::fs::create_dir_all(&test_root).expect("failed to create B14 test root");

        let resolver_path = test_root.join("resolv.conf");
        let config_dir_path = test_root.join("managed-config");
        std::fs::create_dir_all(&config_dir_path).expect("failed to create B14 config dir");
        std::fs::write(
            config_dir_path.join("marker.txt"),
            b"B14_CONFIG_DIR_MARKER_CONTENT",
        )
        .expect("write B14 config marker");
        std::fs::write(
            &resolver_path,
            b"# synthetic B14 resolv.conf\nnameserver 203.0.113.14\n",
        )
        .expect("write B14 resolver");

        let shell = resolve_test_shell();
        let fake_upstream_path = test_root.join("qualified-runtime.sh");
        let compatibility_dir = test_root.join("qualified-compat-bin");
        let prefix = test_root.join("qualified-prefix");
        let qualified_tmp = test_root.join("qualified-tmp");
        let cert_file = test_root.join("qualified-tls/cert.pem");
        let cert_dir = test_root.join("qualified-tls/certs.d");

        let script_content = format!(
            r##"#!{}
if [ "$1" != "-c" ]; then
    printf "B14_ARGV_MISMATCH_1:%s\n" "$1" >&2
    exit 11
fi
if [ "$2" != 'sandbox_mode="danger-full-access"' ]; then
    printf "B14_ARGV_MISMATCH_2:%s\n" "$2" >&2
    exit 12
fi
if [ "$3" != "exec" ] || [ "$4" != "qualified_task" ]; then
    printf "B14_ARGV_MISMATCH_TASK:%s:%s\n" "$3" "$4" >&2
    exit 13
fi
if [ "$5" != "--qualified-flag=value" ]; then
    printf "B14_ARGV_MISMATCH_FLAG:%s\n" "$5" >&2
    exit 14
fi
if [ "$6" != "ordinary qualified arg with spaces" ]; then
    printf "B14_ARGV_MISMATCH_ORDINARY:%s\n" "$6" >&2
    exit 15
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
expected_res="# synthetic B14 resolv.conf
nameserver 203.0.113.14"
if [ "$res_content" != "$expected_res" ]; then
    printf "B14_RESOLVER_MISMATCH:%s\n" "$res_content" >&2
    exit 20
fi
if [ ! -d /proc/self/fd/34 ] || [ ! -f /proc/self/fd/34/marker.txt ]; then
    printf "B14_CONFIG_FD_MISSING\n" >&2
    exit 21
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
if [ "$marker_content" != "B14_CONFIG_DIR_MARKER_CONTENT" ]; then
    printf "B14_CONFIG_MARKER_MISMATCH:%s\n" "$marker_content" >&2
    exit 22
fi

if [ "$TMPDIR" != "{}" ] || [ "$TMP" != "{}" ] || [ "$TEMP" != "{}" ] || [ "$SQLITE_TMPDIR" != "{}" ]; then
    printf "B14_TMP_ENV_MISMATCH:%s:%s:%s:%s\n" "$TMPDIR" "$TMP" "$TEMP" "$SQLITE_TMPDIR" >&2
    exit 30
fi
if [ "$SSL_CERT_FILE" != "{}" ] || [ "$SSL_CERT_DIR" != "{}" ]; then
    printf "B14_CERT_ENV_MISMATCH:%s:%s\n" "$SSL_CERT_FILE" "$SSL_CERT_DIR" >&2
    exit 31
fi
if [ "$PATH" != "{}:{}/bin:/probe/b14/inherited-a:/probe/b14/inherited-b" ]; then
    printf "B14_PATH_MISMATCH:%s\n" "$PATH" >&2
    exit 32
fi

if [ -n "${{CODEX_MANAGED_BY_NPM+x}}" ]; then
    printf "B14_ENV_FENCE_FAILED:CODEX_MANAGED_BY_NPM\n" >&2
    exit 40
fi
if [ -n "${{CODEX_MANAGED_BY_BUN+x}}" ]; then
    printf "B14_ENV_FENCE_FAILED:CODEX_MANAGED_BY_BUN\n" >&2
    exit 40
fi
if [ -n "${{CODEX_MANAGED_PACKAGE_ROOT+x}}" ]; then
    printf "B14_ENV_FENCE_FAILED:CODEX_MANAGED_PACKAGE_ROOT\n" >&2
    exit 40
fi
if [ -n "${{LD_PRELOAD+x}}" ]; then
    printf "B14_ENV_FENCE_FAILED:LD_PRELOAD\n" >&2
    exit 40
fi
if [ -n "${{LD_LIBRARY_PATH+x}}" ]; then
    printf "B14_ENV_FENCE_FAILED:LD_LIBRARY_PATH\n" >&2
    exit 40
fi
if [ "$CODEX_TEST_UNRELATED_M1_B14_SURVIVING_VAR" != "m1_b14_surviving_exact_value_31415" ]; then
    printf "B14_UNRELATED_ENV_MISMATCH:%s\n" "$CODEX_TEST_UNRELATED_M1_B14_SURVIVING_VAR" >&2
    exit 41
fi

printf "M1_B14_QUALIFIED_REAL_EXEC_SUCCESS\n"
exit 0
"##,
            shell.to_str().expect("valid shell path"),
            qualified_tmp.display(),
            qualified_tmp.display(),
            qualified_tmp.display(),
            qualified_tmp.display(),
            cert_file.display(),
            cert_dir.display(),
            compatibility_dir.display(),
            prefix.display(),
        );

        std::fs::write(&fake_upstream_path, script_content).expect("write B14 fake runtime");
        let mut perms = std::fs::metadata(&fake_upstream_path)
            .expect("B14 runtime metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_upstream_path, perms).expect("set B14 runtime permissions");

        let result = run_exec_probe_with_env(
            "m1_b14_qualified_runtime_launcher",
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
        expected_stdout.extend_from_slice(b"ARG:qualified_task\n");
        expected_stdout.extend_from_slice(b"ARG:--qualified-flag=value\n");
        expected_stdout.extend_from_slice(b"ARG:ordinary qualified arg with spaces\n");
        expected_stdout.extend_from_slice(b"ARG:");
        expected_stdout.extend_from_slice(OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x7f]).as_bytes());
        expected_stdout.extend_from_slice(b"\nM1_B14_QUALIFIED_REAL_EXEC_SUCCESS\n");
        assert_eq!(result.stdout, expected_stdout);

        let _ = std::fs::remove_dir_all(&test_root);
    }

    fn m1_b15_expected_summary(
        upstream: UpstreamDoctorStatus,
        core: CoreDoctorStatus,
        manager: ManagerDoctorStatus,
    ) -> DoctorSummaryStatus {
        if core == CoreDoctorStatus::ApiIncompatible
            || manager == ManagerDoctorStatus::ApiIncompatible
        {
            DoctorSummaryStatus::ApiIncompatible
        } else if upstream == UpstreamDoctorStatus::Unhealthy
            || core == CoreDoctorStatus::Unhealthy
            || manager == ManagerDoctorStatus::Unhealthy
        {
            DoctorSummaryStatus::Unhealthy
        } else if upstream == UpstreamDoctorStatus::Unsupported
            || manager == ManagerDoctorStatus::Unavailable
        {
            DoctorSummaryStatus::Degraded
        } else {
            DoctorSummaryStatus::Healthy
        }
    }

    #[test]
    fn test_m1_b15_a_all_section_state_combinations_follow_summary_precedence() {
        let upstream_states = [
            UpstreamDoctorStatus::Healthy,
            UpstreamDoctorStatus::Unhealthy,
            UpstreamDoctorStatus::Unsupported,
        ];
        let core_states = [
            CoreDoctorStatus::Healthy,
            CoreDoctorStatus::Unhealthy,
            CoreDoctorStatus::ApiIncompatible,
        ];
        let manager_states = [
            ManagerDoctorStatus::Healthy,
            ManagerDoctorStatus::Unhealthy,
            ManagerDoctorStatus::Unavailable,
            ManagerDoctorStatus::ApiIncompatible,
        ];

        let mut count = 0;
        for upstream in upstream_states {
            for core in core_states {
                for manager in manager_states {
                    let report = compose_doctor_report(upstream, core, manager);
                    assert_eq!(report.upstream, upstream);
                    assert_eq!(report.termux_core, core);
                    assert_eq!(report.manager, manager);
                    assert_eq!(
                        report.summary,
                        m1_b15_expected_summary(upstream, core, manager)
                    );
                    let expected_exit = match report.summary {
                        DoctorSummaryStatus::Healthy => DoctorExitClass::Success,
                        DoctorSummaryStatus::Degraded | DoctorSummaryStatus::Unhealthy => {
                            DoctorExitClass::HealthFailure
                        }
                        DoctorSummaryStatus::ApiIncompatible => DoctorExitClass::ApiIncompatibility,
                    };
                    assert_eq!(doctor_exit_class(&report), expected_exit);
                    count += 1;
                }
            }
        }
        assert_eq!(count, 36);
    }

    #[test]
    fn test_m1_b15_b_human_output_has_exact_separated_sections() {
        let report = compose_doctor_report(
            UpstreamDoctorStatus::Unsupported,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        );
        assert_eq!(report.summary, DoctorSummaryStatus::Degraded);
        assert_eq!(doctor_exit_class(&report), DoctorExitClass::HealthFailure);
        assert_eq!(
            render_doctor_human(&report),
            "[Upstream]\nstatus: unsupported\n\n[Termux Core]\nstatus: healthy\n\n[Manager]\nstatus: unavailable\n\n[Summary]\nstatus: degraded\n"
        );
    }

    #[test]
    fn test_m1_b15_c_json_output_is_one_exact_redacted_envelope() {
        let report = compose_doctor_report(
            UpstreamDoctorStatus::Healthy,
            CoreDoctorStatus::ApiIncompatible,
            ManagerDoctorStatus::Healthy,
        );
        assert_eq!(
            render_doctor_json(&report),
            "{\"schema_version\":1,\"upstream\":{\"status\":\"healthy\"},\"termux_core\":{\"status\":\"api_incompatible\"},\"manager\":{\"status\":\"healthy\"},\"summary\":{\"status\":\"api_incompatible\"}}\n"
        );
        assert_eq!(
            doctor_exit_class(&report),
            DoctorExitClass::ApiIncompatibility
        );
    }

    #[test]
    fn test_m1_b15_d_rendered_vocabulary_is_bounded_and_deterministic() {
        let upstream_states = [
            UpstreamDoctorStatus::Healthy,
            UpstreamDoctorStatus::Unhealthy,
            UpstreamDoctorStatus::Unsupported,
        ];
        let core_states = [
            CoreDoctorStatus::Healthy,
            CoreDoctorStatus::Unhealthy,
            CoreDoctorStatus::ApiIncompatible,
        ];
        let manager_states = [
            ManagerDoctorStatus::Healthy,
            ManagerDoctorStatus::Unhealthy,
            ManagerDoctorStatus::Unavailable,
            ManagerDoctorStatus::ApiIncompatible,
        ];
        let allowed_statuses = [
            "healthy",
            "unhealthy",
            "unsupported",
            "unavailable",
            "degraded",
            "api_incompatible",
        ];

        for upstream in upstream_states {
            for core in core_states {
                for manager in manager_states {
                    let first = compose_doctor_report(upstream, core, manager);
                    let second = compose_doctor_report(upstream, core, manager);
                    assert_eq!(first, second);
                    assert_eq!(render_doctor_human(&first), render_doctor_human(&second));
                    assert_eq!(render_doctor_json(&first), render_doctor_json(&second));
                    for status in [
                        first.upstream.as_str(),
                        first.termux_core.as_str(),
                        first.manager.as_str(),
                        first.summary.as_str(),
                    ] {
                        assert!(allowed_statuses.contains(&status));
                    }
                }
            }
        }
    }

    #[test]
    fn test_m1_b15_e_composition_and_rendering_do_not_touch_process_environment() {
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
            std::env::var_os("SSL_CERT_FILE"),
            std::env::var_os("SSL_CERT_DIR"),
        ];
        let report = compose_doctor_report(
            UpstreamDoctorStatus::Unsupported,
            CoreDoctorStatus::Unhealthy,
            ManagerDoctorStatus::ApiIncompatible,
        );
        let _ = render_doctor_human(&report);
        let _ = render_doctor_json(&report);
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
            std::env::var_os("SSL_CERT_FILE"),
            std::env::var_os("SSL_CERT_DIR"),
        ];
        assert_eq!(before, after);
        assert_eq!(report.summary, DoctorSummaryStatus::ApiIncompatible);
    }

    #[cfg(unix)]
    fn m1_b16_write_doctor_runtime(
        root: &std::path::Path,
        name: &str,
        exit_code: i32,
    ) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let shell = resolve_test_shell();
        let runtime_path = root.join(name);
        let compatibility_dir = root.join("doctor-compat-bin");
        let prefix = root.join("doctor-prefix");
        let temp_dir = root.join("doctor-tmp");
        let cert_file = root.join("doctor-tls/cert.pem");
        let cert_dir = root.join("doctor-tls/certs.d");
        let script = format!(
            r##"#!{}
printf "B16_SECRET_TOKEN=stdout-secret-must-not-surface\n"
printf "B16_SECRET_COOKIE=stderr-secret-must-not-surface\n" >&2
if [ "$#" -ne 3 ] || [ "$1" != "-c" ] || [ "$2" != 'sandbox_mode="danger-full-access"' ] || [ "$3" != "doctor" ]; then
    exit 81
fi
res_content=""
while IFS= read -r line || [ -n "$line" ]; do
    if [ -z "$res_content" ]; then
        res_content="$line"
    else
        res_content="$res_content
$line"
    fi
done < /proc/self/fd/33
expected_res="# synthetic B16 resolv.conf
nameserver 203.0.113.16"
if [ "$res_content" != "$expected_res" ]; then
    exit 82
fi
if [ ! -d /proc/self/fd/34 ] || [ ! -f /proc/self/fd/34/marker.txt ]; then
    exit 83
fi
marker=""
while IFS= read -r line || [ -n "$line" ]; do
    if [ -z "$marker" ]; then
        marker="$line"
    else
        marker="$marker
$line"
    fi
done < /proc/self/fd/34/marker.txt
if [ "$marker" != "B16_CONFIG_MARKER_EXACT" ]; then
    exit 84
fi
if [ "$TMPDIR" != "{}" ] || [ "$TMP" != "{}" ] || [ "$TEMP" != "{}" ] || [ "$SQLITE_TMPDIR" != "{}" ]; then
    exit 85
fi
if [ "$SSL_CERT_FILE" != "{}" ] || [ "$SSL_CERT_DIR" != "{}" ]; then
    exit 86
fi
if [ "$PATH" != "{}:{}/bin:/probe/b16/inherited-a:/probe/b16/inherited-b" ]; then
    exit 87
fi
if [ -n "${{CODEX_MANAGED_BY_NPM+x}}" ] || [ -n "${{CODEX_MANAGED_BY_BUN+x}}" ] || [ -n "${{CODEX_MANAGED_PACKAGE_ROOT+x}}" ] || [ -n "${{LD_PRELOAD+x}}" ] || [ -n "${{LD_LIBRARY_PATH+x}}" ]; then
    exit 88
fi
if [ "$CODEX_TEST_UNRELATED_M1_B16_SURVIVING_VAR" != "m1_b16_surviving_exact_value_27182" ]; then
    exit 89
fi
printf "upstream says unsupported and includes private session text\n" >&2
exit {}
"##,
            shell.to_str().expect("valid B16 shell path"),
            temp_dir.display(),
            temp_dir.display(),
            temp_dir.display(),
            temp_dir.display(),
            cert_file.display(),
            cert_dir.display(),
            compatibility_dir.display(),
            prefix.display(),
            exit_code,
        );
        std::fs::write(&runtime_path, script).expect("write B16 fake doctor runtime");
        let mut permissions = std::fs::metadata(&runtime_path)
            .expect("B16 fake runtime metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&runtime_path, permissions)
            .expect("set B16 fake runtime permissions");
        runtime_path
    }

    #[cfg(unix)]
    fn m1_b16_create_runtime_root(
        name: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("codex-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config_dir = root.join("managed-config");
        let resolver = root.join("resolv.conf");
        std::fs::create_dir_all(&config_dir).expect("create B16 config dir");
        std::fs::write(config_dir.join("marker.txt"), b"B16_CONFIG_MARKER_EXACT")
            .expect("write B16 config marker");
        std::fs::write(
            &resolver,
            b"# synthetic B16 resolv.conf\nnameserver 203.0.113.16\n",
        )
        .expect("write B16 resolver");
        (root, resolver, config_dir)
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b16_a_healthy_probe_uses_qualified_runtime_and_suppresses_raw_output() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b16-healthy");
        let runtime = m1_b16_write_doctor_runtime(&root, "doctor-healthy.sh", 0);
        let resolver_before = std::fs::read(&resolver).expect("read B16 resolver before");
        let marker_path = config_dir.join("marker.txt");
        let marker_before = std::fs::read(&marker_path).expect("read B16 marker before");

        let result = run_exec_probe_with_env(
            "m1_b16_doctor_probe_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(result.stdout, b"B16_STATUS:healthy\nB16_FD_RESTORED\n");
        assert!(!result
            .stdout
            .windows(b"SECRET".len())
            .any(|w| w == b"SECRET"));
        assert_eq!(std::fs::read(&resolver).unwrap(), resolver_before);
        assert_eq!(std::fs::read(&marker_path).unwrap(), marker_before);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b16_b_nonzero_and_unsupported_text_map_only_to_unhealthy() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b16-unhealthy");
        let runtime = m1_b16_write_doctor_runtime(&root, "doctor-unhealthy.sh", 17);
        let result = run_exec_probe_with_env(
            "m1_b16_doctor_probe_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(result.stdout, b"B16_STATUS:unhealthy\nB16_FD_RESTORED\n");
        assert!(!result
            .stdout
            .windows(b"unsupported".len())
            .any(|w| w == b"unsupported"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b16_c_missing_runtime_is_typed_io_failure() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b16-missing");
        let missing_runtime = root.join("does-not-exist-doctor-runtime");
        let result = run_exec_probe_with_env(
            "m1_b16_missing_runtime_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, missing_runtime.as_os_str()),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(result.stdout, b"B16_IO_NOT_FOUND\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b16_d_invalid_process_snapshot_fails_before_runtime_fd_io() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("B16 environment test generation");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/path/that/does/not/exist/b16-runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/qualified/b16/compat"),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("B16 environment test runtime qualification");
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: Some(OsString::from("/qualified/b16/tmp")),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        match probe_qualified_upstream_doctor(
            qualified,
            &snapshot,
            OsStr::new("/qualified/b16/cert.pem"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b16-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b16-config"),
        ) {
            Err(QualifiedUpstreamDoctorProbeError::Environment(err)) => {
                assert_eq!(err, TermuxProcessEnvError::MissingRequired("PREFIX"));
            }
            other => panic!("expected B16 environment failure before FD I/O, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b17_a_supported_healthy_composes_real_probe_into_degraded_report() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b17-healthy");
        let runtime = m1_b16_write_doctor_runtime(&root, "doctor-healthy.sh", 0);
        let mode = OsStr::new("healthy-degraded");
        let result = run_exec_probe_with_env(
            "m1_b17_coordinator_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
                (PROBE_B17_MODE_ENV, mode),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(
            result.stdout,
            b"B17_REPORT:healthy:healthy:unavailable:degraded\nB17_HUMAN:[Upstream]\nstatus: healthy\n\n[Termux Core]\nstatus: healthy\n\n[Manager]\nstatus: unavailable\n\n[Summary]\nstatus: degraded\nB17_JSON:{\"schema_version\":1,\"upstream\":{\"status\":\"healthy\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"unavailable\"},\"summary\":{\"status\":\"degraded\"}}\n"
        );
        assert!(!result
            .stdout
            .windows(b"SECRET".len())
            .any(|window| window == b"SECRET"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b17_b_supported_unhealthy_respects_api_incompatible_precedence() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b17-api");
        let runtime = m1_b16_write_doctor_runtime(&root, "doctor-unhealthy.sh", 17);
        let mode = OsStr::new("unhealthy-api");
        let result = run_exec_probe_with_env(
            "m1_b17_coordinator_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
                (PROBE_B17_MODE_ENV, mode),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(
            result.stdout,
            b"B17_REPORT:unhealthy:api_incompatible:healthy:api_incompatible\nB17_HUMAN:[Upstream]\nstatus: unhealthy\n\n[Termux Core]\nstatus: api_incompatible\n\n[Manager]\nstatus: healthy\n\n[Summary]\nstatus: api_incompatible\nB17_JSON:{\"schema_version\":1,\"upstream\":{\"status\":\"unhealthy\"},\"termux_core\":{\"status\":\"api_incompatible\"},\"manager\":{\"status\":\"healthy\"},\"summary\":{\"status\":\"api_incompatible\"}}\n"
        );
        assert!(!result
            .stdout
            .windows(b"unsupported".len())
            .any(|window| window == b"unsupported"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b17_c_unsupported_skips_probe_io_and_renders_exactly() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("B17 unsupported generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/path/that/does/not/exist/b17-runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/path/that/does/not/exist/b17-compat"),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("B17 unsupported runtime shape must qualify");
        let invalid_snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let report = compose_local_doctor(
            UpstreamDoctorCapability::Unsupported,
            qualified,
            &invalid_snapshot,
            OsStr::new("/path/that/does/not/exist/b17-cert.pem"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b17-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b17-config"),
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        )
        .expect("unsupported capability must skip all probe-only I/O");

        assert_eq!(report.upstream, UpstreamDoctorStatus::Unsupported);
        assert_eq!(report.termux_core, CoreDoctorStatus::Healthy);
        assert_eq!(report.manager, ManagerDoctorStatus::Unavailable);
        assert_eq!(report.summary, DoctorSummaryStatus::Degraded);
        assert_eq!(
            render_doctor_human(&report),
            "[Upstream]\nstatus: unsupported\n\n[Termux Core]\nstatus: healthy\n\n[Manager]\nstatus: unavailable\n\n[Summary]\nstatus: degraded\n"
        );
        assert_eq!(
            render_doctor_json(&report),
            "{\"schema_version\":1,\"upstream\":{\"status\":\"unsupported\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"unavailable\"},\"summary\":{\"status\":\"degraded\"}}\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b17_d_supported_spawn_error_propagates_without_report() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b17-missing");
        let missing_runtime = root.join("does-not-exist-doctor-runtime");
        let mode = OsStr::new("missing-runtime");
        let result = run_exec_probe_with_env(
            "m1_b17_coordinator_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, missing_runtime.as_os_str()),
                (PROBE_B17_MODE_ENV, mode),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(result.stdout, b"B17_IO_NOT_FOUND\n");
        assert!(!result
            .stdout
            .windows(b"B17_REPORT".len())
            .any(|window| window == b"B17_REPORT"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_m1_b18_a_exact_human_and_json_invocation_plans() {
        assert_eq!(
            plan_doctor_invocation(std::iter::empty::<OsString>()).unwrap(),
            DoctorInvocationPlan {
                output_mode: DoctorOutputMode::Human
            }
        );
        assert_eq!(
            plan_doctor_invocation([OsString::from("--json")]).unwrap(),
            DoctorInvocationPlan {
                output_mode: DoctorOutputMode::Json
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b18_b_invalid_and_non_utf8_forms_fail_with_one_bounded_usage_error() {
        use std::os::unix::ffi::OsStringExt;

        let cases: Vec<Vec<OsString>> = vec![
            vec![OsString::from("")],
            vec![OsString::from("--")],
            vec![OsString::from("--help")],
            vec![OsString::from("extra")],
            vec![OsString::from("--json"), OsString::from("--json")],
            vec![OsString::from("--json"), OsString::from("extra")],
            vec![OsString::from_vec(vec![0xff, 0xfe, 0x80])],
        ];

        for args in cases {
            assert_eq!(
                plan_doctor_invocation(args).unwrap_err(),
                DoctorUsageError::InvalidArguments
            );
        }
    }

    #[test]
    fn test_m1_b18_c_usage_error_text_is_static_and_does_not_echo_rejected_argv() {
        let secret = "--token=super-secret-doctor-value";
        let err = plan_doctor_invocation([OsString::from(secret)]).unwrap_err();
        let text = err.to_string();
        assert_eq!(text, "usage: codex doctor [--json]");
        assert!(!text.contains("super-secret"));
        assert!(!text.contains("token"));
    }

    #[test]
    fn test_m1_b18_d_rendering_preserves_every_semantic_exit_class_and_json_on_failure() {
        let human = plan_doctor_invocation(std::iter::empty::<OsString>()).unwrap();
        let json = plan_doctor_invocation([OsString::from("--json")]).unwrap();

        let healthy = compose_doctor_report(
            UpstreamDoctorStatus::Healthy,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Healthy,
        );
        let healthy_outcome = render_doctor_command(human, &healthy);
        assert_eq!(healthy_outcome.exit_class, DoctorExitClass::Success);
        assert_eq!(
            healthy_outcome.output,
            "[Upstream]\nstatus: healthy\n\n[Termux Core]\nstatus: healthy\n\n[Manager]\nstatus: healthy\n\n[Summary]\nstatus: healthy\n"
        );

        let degraded = compose_doctor_report(
            UpstreamDoctorStatus::Unsupported,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        );
        let degraded_outcome = render_doctor_command(json, &degraded);
        assert_eq!(degraded_outcome.exit_class, DoctorExitClass::HealthFailure);
        assert_eq!(
            degraded_outcome.output,
            "{\"schema_version\":1,\"upstream\":{\"status\":\"unsupported\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"unavailable\"},\"summary\":{\"status\":\"degraded\"}}\n"
        );

        let unhealthy = compose_doctor_report(
            UpstreamDoctorStatus::Unhealthy,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Healthy,
        );
        let unhealthy_outcome = render_doctor_command(json, &unhealthy);
        assert_eq!(unhealthy_outcome.exit_class, DoctorExitClass::HealthFailure);
        assert_eq!(
            unhealthy_outcome.output,
            "{\"schema_version\":1,\"upstream\":{\"status\":\"unhealthy\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"healthy\"},\"summary\":{\"status\":\"unhealthy\"}}\n"
        );

        let incompatible = compose_doctor_report(
            UpstreamDoctorStatus::Healthy,
            CoreDoctorStatus::ApiIncompatible,
            ManagerDoctorStatus::Healthy,
        );
        let incompatible_outcome = render_doctor_command(json, &incompatible);
        assert_eq!(
            incompatible_outcome.exit_class,
            DoctorExitClass::ApiIncompatibility
        );
        assert_eq!(
            incompatible_outcome.output,
            "{\"schema_version\":1,\"upstream\":{\"status\":\"healthy\"},\"termux_core\":{\"status\":\"api_incompatible\"},\"manager\":{\"status\":\"healthy\"},\"summary\":{\"status\":\"api_incompatible\"}}\n"
        );
    }

    #[test]
    fn test_m1_b18_e_planning_and_rendering_are_deterministic_and_environment_pure() {
        let before = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
            std::env::var_os("SSL_CERT_FILE"),
            std::env::var_os("SSL_CERT_DIR"),
        ];
        let first_plan = plan_doctor_invocation([OsString::from("--json")]).unwrap();
        let second_plan = plan_doctor_invocation([OsString::from("--json")]).unwrap();
        let report = compose_doctor_report(
            UpstreamDoctorStatus::Unsupported,
            CoreDoctorStatus::Unhealthy,
            ManagerDoctorStatus::Unavailable,
        );
        let first = render_doctor_command(first_plan, &report);
        let second = render_doctor_command(second_plan, &report);
        let after = [
            std::env::var_os("PREFIX"),
            std::env::var_os("TMPDIR"),
            std::env::var_os("PATH"),
            std::env::var_os("SSL_CERT_FILE"),
            std::env::var_os("SSL_CERT_DIR"),
        ];
        assert_eq!(first_plan, second_plan);
        assert_eq!(first, second);
        assert_eq!(before, after);
    }

    #[cfg(unix)]
    fn m1_b19_assert_usage_before_probe_io(args: Vec<OsString>) {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("B19 usage-order generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/path/that/does/not/exist/b19-runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/path/that/does/not/exist/b19-compat"),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("B19 usage-order runtime shape must qualify");
        let invalid_snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        match run_local_doctor_command(
            args,
            UpstreamDoctorCapability::Supported,
            qualified,
            &invalid_snapshot,
            OsStr::new("/path/that/does/not/exist/b19-cert.pem"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b19-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b19-config"),
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Healthy,
        ) {
            Err(LocalDoctorCommandError::Usage(DoctorUsageError::InvalidArguments)) => {}
            Err(LocalDoctorCommandError::Probe(err)) => {
                panic!("B19 usage must fail before probe I/O, got probe error: {err}")
            }
            Ok(outcome) => panic!("invalid B19 usage unexpectedly produced outcome: {outcome:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b19_a_usage_rejection_precedes_probe_io_for_utf8_and_non_utf8() {
        use std::os::unix::ffi::OsStringExt;

        m1_b19_assert_usage_before_probe_io(vec![OsString::from("--secret-invalid-option")]);
        m1_b19_assert_usage_before_probe_io(vec![
            OsString::from("--json"),
            OsString::from("--json"),
        ]);
        m1_b19_assert_usage_before_probe_io(vec![OsString::from_vec(vec![0xff, 0xfe, 0x80])]);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b19_b_supported_healthy_human_crosses_real_probe_and_renders() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b19-healthy");
        let runtime = m1_b16_write_doctor_runtime(&root, "doctor-healthy.sh", 0);
        let mode = OsStr::new("healthy-human");
        let result = run_exec_probe_with_env(
            "m1_b19_command_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
                (PROBE_B19_MODE_ENV, mode),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(
            result.stdout,
            b"B19_EXIT:success\nB19_OUTPUT:[Upstream]\nstatus: healthy\n\n[Termux Core]\nstatus: healthy\n\n[Manager]\nstatus: healthy\n\n[Summary]\nstatus: healthy\n"
        );
        assert!(!result
            .stdout
            .windows(b"SECRET".len())
            .any(|window| window == b"SECRET"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b19_c_supported_nonzero_json_preserves_bounded_failure_envelope() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b19-unhealthy");
        let runtime = m1_b16_write_doctor_runtime(&root, "doctor-unhealthy.sh", 17);
        let mode = OsStr::new("unhealthy-json");
        let result = run_exec_probe_with_env(
            "m1_b19_command_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, runtime.as_os_str()),
                (PROBE_B19_MODE_ENV, mode),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(
            result.stdout,
            b"B19_EXIT:health_failure\nB19_OUTPUT:{\"schema_version\":1,\"upstream\":{\"status\":\"unhealthy\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"healthy\"},\"summary\":{\"status\":\"unhealthy\"}}\n"
        );
        assert!(!result
            .stdout
            .windows(b"unsupported".len())
            .any(|window| window == b"unsupported"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b19_d_unsupported_valid_json_skips_invalid_probe_inputs() {
        let mut manifest = m1_b11_valid_manifest();
        manifest.helper_digests.clear();
        let generation = qualify_generation_manifest(&manifest, &m1_b11_requirements())
            .expect("B19 unsupported generation must qualify");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/path/that/does/not/exist/b19-runtime"),
                observed_digest: manifest.runtime_digest.as_str(),
            },
            compatibility_dir: OsStr::new("/path/that/does/not/exist/b19-compat"),
            helpers: &[],
        };
        let qualified = qualify_runtime_assets(generation, &selection)
            .expect("B19 unsupported runtime shape must qualify");
        let invalid_snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };

        let outcome = run_local_doctor_command(
            [OsString::from("--json")],
            UpstreamDoctorCapability::Unsupported,
            qualified,
            &invalid_snapshot,
            OsStr::new("/path/that/does/not/exist/b19-cert.pem"),
            None,
            std::path::Path::new("/path/that/does/not/exist/b19-resolver"),
            std::path::Path::new("/path/that/does/not/exist/b19-config"),
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        )
        .expect("B19 unsupported command must skip probe-only I/O");
        assert_eq!(outcome.exit_class, DoctorExitClass::HealthFailure);
        assert_eq!(
            outcome.output,
            "{\"schema_version\":1,\"upstream\":{\"status\":\"unsupported\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"unavailable\"},\"summary\":{\"status\":\"degraded\"}}\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m1_b19_e_valid_supported_spawn_failure_propagates_probe_without_outcome() {
        let (root, resolver, config_dir) = m1_b16_create_runtime_root("m1-b19-missing");
        let missing_runtime = root.join("does-not-exist-doctor-runtime");
        let mode = OsStr::new("missing-json");
        let result = run_exec_probe_with_env(
            "m1_b19_command_launcher",
            &[
                (PROBE_RESOLVER_PATH_ENV, resolver.as_os_str()),
                (PROBE_CONFIG_DIR_PATH_ENV, config_dir.as_os_str()),
                (PROBE_FAKE_UPSTREAM_PATH_ENV, missing_runtime.as_os_str()),
                (PROBE_B19_MODE_ENV, mode),
            ],
        );
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        assert_eq!(result.stdout, b"B19_PROBE_NOT_FOUND\n");
        assert!(!result
            .stdout
            .windows(b"B19_OUTPUT".len())
            .any(|window| window == b"B19_OUTPUT"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
