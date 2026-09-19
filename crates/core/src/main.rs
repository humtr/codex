use std::ffi::{OsStr, OsString};

#[cfg(unix)]
mod rollback_guard;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicDispatchRoute {
    Update(Vec<OsString>),
    Doctor(Vec<OsString>),
    Termux(Vec<OsString>),
    UnsupportedDaemon,
    Upstream(Vec<OsString>),
}

const TERMUX_DAEMON_UNSUPPORTED: &str = "upstream app-server daemon lifecycle is unsupported on Termux because it is bound to standalone installer/update authority; use foreground 'codex remote-control' when remote control is needed";

fn upstream_global_option_takes_separate_value(value: &str) -> bool {
    matches!(value, "-c" | "--config" | "--enable" | "--disable")
}

fn root_option_takes_separate_value(value: &str) -> bool {
    upstream_global_option_takes_separate_value(value)
        || matches!(
            value,
            "-m" | "--model"
                | "--local-provider"
                | "-p"
                | "--profile"
                | "-s"
                | "--sandbox"
                | "-C"
                | "--cd"
                | "--add-dir"
                | "-a"
                | "--ask-for-approval"
                | "--remote"
                | "--remote-auth-token-env"
        )
}

fn upstream_root_command_index(args: &[OsString]) -> Option<usize> {
    let mut index = 0;
    while index < args.len() {
        let value = args[index].to_str()?;
        if value == "--" {
            return None;
        }
        if !value.starts_with('-') || value == "-" {
            return Some(index);
        }
        if matches!(value, "-i" | "--image") {
            index += 1;
            while index < args.len() {
                let Some(candidate) = args[index].to_str() else {
                    index += 1;
                    continue;
                };
                if candidate.starts_with('-')
                    || matches!(candidate, "app-server" | "remote-control")
                {
                    break;
                }
                index += 1;
            }
        } else if root_option_takes_separate_value(value) {
            index = index.saturating_add(2);
        } else {
            index += 1;
        }
    }
    None
}

fn app_server_option_takes_separate_value(value: &str) -> bool {
    matches!(
        value,
        "--code-mode-host"
            | "--listen"
            | "--ws-auth"
            | "--ws-token-file"
            | "--ws-token-sha256"
            | "--ws-shared-secret-file"
            | "--ws-issuer"
            | "--ws-audience"
            | "--ws-max-clock-skew-seconds"
    )
}

fn app_server_daemon_subcommand_present(args: &[OsString]) -> bool {
    let mut index = 0;
    while index < args.len() {
        let Some(value) = args[index].to_str() else {
            return false;
        };
        if value == "--" {
            return false;
        }
        if !value.starts_with('-') || value == "-" {
            return value == "daemon";
        }
        if upstream_global_option_takes_separate_value(value)
            || app_server_option_takes_separate_value(value)
        {
            index = index.saturating_add(2);
        } else {
            index += 1;
        }
    }
    false
}

fn remote_control_daemon_subcommand_present(args: &[OsString]) -> bool {
    let mut index = 0;
    while index < args.len() {
        let Some(value) = args[index].to_str() else {
            return false;
        };
        if value == "--" {
            return false;
        }
        if !value.starts_with('-') || value == "-" {
            return matches!(value, "start" | "stop" | "pair");
        }
        if upstream_global_option_takes_separate_value(value) {
            index = index.saturating_add(2);
        } else {
            index += 1;
        }
    }
    false
}

fn daemon_backed_upstream_route(args: &[OsString]) -> bool {
    let Some(command_index) = upstream_root_command_index(args) else {
        return false;
    };
    let Some(command) = args[command_index].to_str() else {
        return false;
    };
    let tail = &args[command_index + 1..];
    match command {
        "app-server" => app_server_daemon_subcommand_present(tail),
        "remote-control" => remote_control_daemon_subcommand_present(tail),
        _ => false,
    }
}

/// Selects the exact public Core route and fully plans upstream argv.
fn plan_public_dispatch<I, S>(args: I) -> Result<PublicDispatchRoute, PassthroughError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let original: Vec<OsString> = args.into_iter().map(Into::into).collect();
    if daemon_backed_upstream_route(&original) {
        return Ok(PublicDispatchRoute::UnsupportedDaemon);
    }
    match original.first().map(OsString::as_os_str) {
        Some(value) if value == OsStr::new("update") => Ok(PublicDispatchRoute::Update(
            original.into_iter().skip(1).collect(),
        )),
        Some(value) if value == OsStr::new("doctor") => Ok(PublicDispatchRoute::Doctor(
            original.into_iter().skip(1).collect(),
        )),
        Some(value) if value == OsStr::new("termux") => Ok(PublicDispatchRoute::Termux(
            original.into_iter().skip(1).collect(),
        )),
        _ => plan_passthrough_args(original).map(PublicDispatchRoute::Upstream),
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

#[cfg(unix)]
pub const RESOLVER_FD: std::os::raw::c_int = 33;
#[cfg(unix)]
pub const CONFIG_DIR_FD: std::os::raw::c_int = 34;
#[cfg(unix)]
const SAFE_MIN_FD: std::os::raw::c_int = 35;
#[cfg(unix)]
const F_DUPFD_CLOEXEC: std::os::raw::c_int = 1030;
#[cfg(unix)]
const FLOCK_EX: std::os::raw::c_int = 2;
#[cfg(unix)]
const FLOCK_NB: std::os::raw::c_int = 4;
#[cfg(unix)]
const FLOCK_UN: std::os::raw::c_int = 8;

#[cfg(unix)]
extern "C" {
    fn dup2(oldfd: std::os::raw::c_int, newfd: std::os::raw::c_int) -> std::os::raw::c_int;
    fn fcntl(fd: std::os::raw::c_int, cmd: std::os::raw::c_int, ...) -> std::os::raw::c_int;
    fn flock(fd: std::os::raw::c_int, operation: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(unix)]
struct RuntimeFdSources {
    resolver: std::os::fd::OwnedFd,
    config_dir: std::os::fd::OwnedFd,
}

#[cfg(unix)]
impl RuntimeFdSources {
    fn open<R, C>(resolver_path: R, config_dir: C) -> std::io::Result<Self>
    where
        R: AsRef<std::path::Path>,
        C: AsRef<std::path::Path>,
    {
        use std::os::fd::{AsRawFd, FromRawFd};

        let resolver = std::fs::File::open(resolver_path)?;
        if resolver.metadata()?.is_dir() {
            return Err(std::io::Error::from_raw_os_error(21));
        }
        let config = std::fs::File::open(config_dir)?;
        if !config.metadata()?.is_dir() {
            return Err(std::io::Error::from_raw_os_error(20));
        }

        let resolver_fd = unsafe { fcntl(resolver.as_raw_fd(), F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
        if resolver_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let resolver = unsafe { std::os::fd::OwnedFd::from_raw_fd(resolver_fd) };

        let config_fd = unsafe { fcntl(config.as_raw_fd(), F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
        if config_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let config_dir = unsafe { std::os::fd::OwnedFd::from_raw_fd(config_fd) };

        Ok(Self {
            resolver,
            config_dir,
        })
    }

    fn configure(&self, command: &mut std::process::Command) {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;

        let resolver = self.resolver.as_raw_fd();
        let config_dir = self.config_dir.as_raw_fd();
        unsafe {
            command.pre_exec(move || {
                if dup2(resolver, RESOLVER_FD) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if dup2(config_dir, CONFIG_DIR_FD) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
}

#[cfg(unix)]
fn apply_child_env_plan_and_fence(
    cmd: &mut std::process::Command,
    env_plan: Option<&TermuxBaseEnvPlan>,
) {
    if let Some(plan) = env_plan {
        for (k, v) in &plan.assignments {
            cmd.env(k, v);
        }
        for name in &plan.removals {
            cmd.env_remove(name);
        }
    }
    cmd.env_remove("CODEX_MANAGED_BY_NPM")
        .env_remove("CODEX_MANAGED_BY_BUN")
        .env_remove("CODEX_MANAGED_PACKAGE_ROOT")
        .env_remove("LD_PRELOAD")
        .env_remove("LD_LIBRARY_PATH");
}

#[cfg(unix)]
fn exec_runtime<P, I, S, R, C>(
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
    use std::os::unix::process::CommandExt;

    let runtime_fds = match RuntimeFdSources::open(resolver_path, config_dir) {
        Ok(fds) => fds,
        Err(err) => return err,
    };
    let mut command = std::process::Command::new(program.as_ref());
    command.args(args);
    apply_child_env_plan_and_fence(&mut command, env_plan);
    runtime_fds.configure(&mut command);
    command.exec()
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxProcessEnvSnapshot {
    wsl_kernel: Option<bool>,
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
    InvalidPathComponent(&'static str),
    UnsupportedHost(&'static str),
    MissingQualifiedHelper(&'static str),
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
            TermuxProcessEnvError::InvalidPathComponent(name) => {
                write!(f, "PATH component '{name}' is empty or contains ':' or NUL")
            }
            TermuxProcessEnvError::UnsupportedHost(host) => {
                write!(f, "unsupported Termux host classification '{host}'")
            }
            TermuxProcessEnvError::MissingQualifiedHelper(identity) => {
                write!(
                    f,
                    "qualified generation is missing required helper '{identity}'"
                )
            }
        }
    }
}

#[cfg(unix)]
impl std::error::Error for TermuxProcessEnvError {}

#[cfg(unix)]
fn detect_wsl_kernel_signature() -> Option<bool> {
    std::fs::read_to_string("/proc/version")
        .ok()
        .map(|version| {
            let version = version.to_lowercase();
            version.contains("microsoft") || version.contains("wsl")
        })
}

#[cfg(unix)]
fn capture_termux_process_env() -> TermuxProcessEnvSnapshot {
    TermuxProcessEnvSnapshot {
        wsl_kernel: detect_wsl_kernel_signature(),
        prefix: std::env::var_os("PREFIX"),
        tmpdir: std::env::var_os("TMPDIR"),
        inherited_path: std::env::var_os("PATH"),
        inherited_ssl_cert_file: std::env::var_os("SSL_CERT_FILE"),
        inherited_ssl_cert_dir: std::env::var_os("SSL_CERT_DIR"),
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxBaseEnvPlan {
    assignments: Vec<(OsString, OsString)>,
    removals: Vec<OsString>,
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
fn valid_path_component(component: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let bytes = component.as_bytes();
    !bytes.is_empty() && !bytes.contains(&b':') && !bytes.contains(&b'\0')
}

// webbrowser 1.2.2 treats a command whose basename is `curl` as a text browser,
// so it waits for the helper exit status before trying the next BROWSER entry.
// Keep the signed helper basename `curl` unless that dependency contract is requalified.
#[cfg(unix)]
const TERMUX_BROWSER_OPEN_HELPER_IDENTITY: &str = "termux-browser-open-v1";
#[cfg(unix)]
const TERMUX_BROWSER_MANUAL_HELPER_IDENTITY: &str = "termux-browser-manual-v1";
#[cfg(unix)]
const R10_BROWSER_HELPER_BRIDGE_METADATA: &str = "r10-browser-helper-bridge-v1";

#[cfg(unix)]
fn generation_helper_relative_path(index: usize, identity: &str) -> std::path::PathBuf {
    match identity {
        TERMUX_BROWSER_OPEN_HELPER_IDENTITY => std::path::PathBuf::from("browser/open/curl"),
        TERMUX_BROWSER_MANUAL_HELPER_IDENTITY => std::path::PathBuf::from("browser/manual/curl"),
        _ => std::path::PathBuf::from("helpers").join(index.to_string()),
    }
}

#[cfg(unix)]
fn generation_helper_relative_path_for_layout(
    index: usize,
    identity: &str,
    r10_browser_helper_bridge: bool,
) -> std::path::PathBuf {
    if r10_browser_helper_bridge
        && matches!(
            identity,
            TERMUX_BROWSER_OPEN_HELPER_IDENTITY | TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
        )
    {
        std::path::PathBuf::from("helpers").join(index.to_string())
    } else {
        generation_helper_relative_path(index, identity)
    }
}

#[cfg(unix)]
fn r10_browser_helper_bridge(manifest: &GenerationManifest) -> Result<bool, LocalProductError> {
    if manifest.creation_metadata != R10_BROWSER_HELPER_BRIDGE_METADATA {
        return Ok(false);
    }
    if manifest.helper_digests.len() != 2
        || manifest.helper_digests[0].identity != TERMUX_BROWSER_OPEN_HELPER_IDENTITY
        || manifest.helper_digests[1].identity != TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
    {
        return Err(LocalProductError::Descriptor(
            "R10 browser helper bridge contract is invalid",
        ));
    }
    Ok(true)
}

#[cfg(unix)]
fn valid_browser_helper_path(path: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_bytes();
    !bytes.is_empty()
        && std::path::Path::new(path).is_absolute()
        && !bytes.contains(&b':')
        && !bytes.contains(&b'\0')
        && !bytes.iter().any(u8::is_ascii_whitespace)
}

#[cfg(unix)]
fn plan_termux_env(
    snapshot: &TermuxProcessEnvSnapshot,
    compat_dir: &OsStr,
    browser_open_helper: &OsStr,
    browser_manual_helper: &OsStr,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
) -> Result<TermuxBaseEnvPlan, TermuxProcessEnvError> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    // Upstream 0.154 checks /proc/version first, then only WSL_DISTRO_NAME/WSL_INTEROP.
    // Reject a positive kernel signature. If /proc/version is unreadable here, it is also
    // unreadable to the immediately exec'd child under the same credentials; removing both
    // environment selectors below therefore keeps the upstream WSL fallback unreachable.
    if snapshot.wsl_kernel == Some(true) {
        return Err(TermuxProcessEnvError::UnsupportedHost("WSL"));
    }
    let prefix = required_process_env(&snapshot.prefix, "PREFIX")?;
    let temp_dir = required_process_env(&snapshot.tmpdir, "TMPDIR")?;
    let prefix_bin = std::path::PathBuf::from(prefix).join("bin");
    if !valid_path_component(compat_dir) {
        return Err(TermuxProcessEnvError::InvalidPathComponent("compat_dir"));
    }
    if !valid_path_component(prefix_bin.as_os_str()) {
        return Err(TermuxProcessEnvError::InvalidPathComponent("prefix_bin"));
    }
    if !valid_browser_helper_path(browser_open_helper) {
        return Err(TermuxProcessEnvError::InvalidPathComponent(
            "browser_open_helper",
        ));
    }
    if !valid_browser_helper_path(browser_manual_helper) {
        return Err(TermuxProcessEnvError::InvalidPathComponent(
            "browser_manual_helper",
        ));
    }

    let inherited_path = snapshot.inherited_path.as_deref().unwrap_or_default();
    let mut path = Vec::with_capacity(
        compat_dir.as_bytes().len()
            + prefix_bin.as_os_str().as_bytes().len()
            + inherited_path.as_bytes().len()
            + 2,
    );
    path.extend_from_slice(compat_dir.as_bytes());
    path.push(b':');
    path.extend_from_slice(prefix_bin.as_os_str().as_bytes());
    if !inherited_path.is_empty() {
        path.push(b':');
        path.extend_from_slice(inherited_path.as_bytes());
    }

    let mut assignments = Vec::with_capacity(11);
    for name in ["TMPDIR", "TMP", "TEMP", "SQLITE_TMPDIR"] {
        assignments.push((OsString::from(name), temp_dir.to_os_string()));
    }
    assignments.push((
        OsString::from("SSL_CERT_FILE"),
        snapshot
            .inherited_ssl_cert_file
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(cert_file)
            .to_os_string(),
    ));
    if let Some(value) = snapshot
        .inherited_ssl_cert_dir
        .as_deref()
        .filter(|value| !value.is_empty())
        .or_else(|| cert_dir.filter(|value| !value.is_empty()))
    {
        assignments.push((OsString::from("SSL_CERT_DIR"), value.to_os_string()));
    }
    assignments.push((OsString::from("PATH"), OsString::from_vec(path)));

    let opener = prefix_bin.join("termux-open-url");
    let mut browser = Vec::with_capacity(
        browser_open_helper.as_bytes().len() + browser_manual_helper.as_bytes().len() + 9,
    );
    browser.extend_from_slice(browser_open_helper.as_bytes());
    browser.extend_from_slice(b" %s:");
    browser.extend_from_slice(browser_manual_helper.as_bytes());
    browser.extend_from_slice(b" %s");
    assignments.push((OsString::from("BROWSER"), OsString::from_vec(browser)));
    assignments.push((
        OsString::from("CODEX_TERMUX_URL_OPENER"),
        opener.into_os_string(),
    ));
    assignments.push((OsString::from("DISPLAY"), OsString::new()));
    assignments.push((
        OsString::from("WAYLAND_DISPLAY"),
        OsString::from("/__codex_termux_unavailable__"),
    ));

    let removals = ["WAYLAND_SOCKET", "WSL_DISTRO_NAME", "WSL_INTEROP"]
        .into_iter()
        .map(OsString::from)
        .collect();

    Ok(TermuxBaseEnvPlan {
        assignments,
        removals,
    })
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

#[derive(Debug, Clone, Copy)]
struct QualifiedGenerationManifest<'a> {
    manifest: &'a GenerationManifest,
}

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
    InvalidCompatibilityDir,
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
            RuntimeAssetError::InvalidCompatibilityDir => {
                write!(f, "compatibility directory is not a valid PATH component")
            }
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
impl std::error::Error for RuntimeAssetError {}

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct QualifiedRuntimeAssets<'selection, 'asset> {
    selection: &'selection RuntimeAssetSelection<'asset>,
}

#[cfg(unix)]
impl<'selection, 'asset> QualifiedRuntimeAssets<'selection, 'asset> {
    fn selection(self) -> &'selection RuntimeAssetSelection<'asset> {
        self.selection
    }
}

#[cfg(unix)]
fn termux_browser_helper_paths<'asset>(
    selection: &RuntimeAssetSelection<'asset>,
) -> Result<(&'asset OsStr, &'asset OsStr), TermuxProcessEnvError> {
    let find = |identity: &'static str| {
        selection
            .helpers
            .iter()
            .find(|helper| helper.identity == identity)
            .map(|helper| helper.asset_path)
            .ok_or(TermuxProcessEnvError::MissingQualifiedHelper(identity))
    };
    Ok((
        find(TERMUX_BROWSER_OPEN_HELPER_IDENTITY)?,
        find(TERMUX_BROWSER_MANUAL_HELPER_IDENTITY)?,
    ))
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
fn qualify_runtime_assets<'selection, 'asset, 'generation>(
    generation: QualifiedGenerationManifest<'generation>,
    selection: &'selection RuntimeAssetSelection<'asset>,
) -> Result<QualifiedRuntimeAssets<'selection, 'asset>, RuntimeAssetError> {
    validate_absolute_runtime_asset_path(selection.runtime.program_path, "runtime_program")?;
    validate_absolute_runtime_asset_path(selection.compatibility_dir, "compatibility_dir")?;
    if !valid_path_component(selection.compatibility_dir) {
        return Err(RuntimeAssetError::InvalidCompatibilityDir);
    }

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

    Ok(QualifiedRuntimeAssets { selection })
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
            ManagerArtifactError::UnexpectedSelection => {
                f.write_str("Manager artifact was selected but the generation declares no Manager")
            }
            ManagerArtifactError::MissingSelection => {
                f.write_str("generation declares a Manager artifact but no artifact was selected")
            }
            ManagerArtifactError::Path(err) => err.fmt(f),
            ManagerArtifactError::EmptyDigest => {
                f.write_str("selected Manager artifact observed digest is empty")
            }
            ManagerArtifactError::DigestMismatch => {
                f.write_str("selected Manager artifact digest does not match generation")
            }
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
#[derive(Debug, Clone, Copy)]
enum ManagerArtifact<'selection, 'asset> {
    Unavailable,
    Available(&'selection ManagerArtifactSelection<'asset>),
}

#[cfg(unix)]
fn qualify_manager_artifact<'selection, 'asset>(
    generation: QualifiedGenerationManifest<'_>,
    selection: Option<&'selection ManagerArtifactSelection<'asset>>,
) -> Result<ManagerArtifact<'selection, 'asset>, ManagerArtifactError> {
    match (
        generation.manifest().manager_artifact_digest.as_deref(),
        selection,
    ) {
        (None, None) => Ok(ManagerArtifact::Unavailable),
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
            Ok(ManagerArtifact::Available(selection))
        }
    }
}

#[cfg(unix)]
const TERMUX_MANAGER_UNAVAILABLE_MESSAGE: &str = "Codex Termux Manager is unavailable.";
#[cfg(unix)]
const MANAGER_CORE_API_ENV: &str = "CODEX_TERMUX_CORE_API";
#[cfg(unix)]
const MANAGER_CORE_ENTRYPOINT_ENV: &str = "CODEX_TERMUX_CORE_ENTRYPOINT";
#[cfg(unix)]
const MANAGER_CORE_API: &str = "codex-manager-core-v1";
#[cfg(unix)]
const MANAGER_ARTIFACT_PROBE_ENV: &str = "CODEX_MANAGER_ARTIFACT_PROBE";
#[cfg(unix)]
const CORE_REPAIR_REQUEST_ENV: &str = "CODEX_TERMUX_CORE_REQUEST";
#[cfg(unix)]
const CORE_REPAIR_OPERATION_ENV: &str = "CODEX_TERMUX_CORE_OPERATION";
#[cfg(unix)]
const CORE_REPAIR_REQUEST: &str = "codex-manager-repair-v1";

#[cfg(unix)]
fn validated_core_entrypoint() -> std::io::Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let path = std::env::current_exe()?;
    if !path.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Core entrypoint is not absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink()
        || !metadata.file_type().is_file()
        || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Core entrypoint is not a regular executable",
        ));
    }
    Ok(path)
}

#[cfg(unix)]
fn execute_termux_manager<I, S>(
    manager: ManagerArtifact<'_, '_>,
    args: I,
) -> Result<&'static str, std::io::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    match manager {
        ManagerArtifact::Unavailable => Ok(TERMUX_MANAGER_UNAVAILABLE_MESSAGE),
        ManagerArtifact::Available(selection) => {
            use std::os::unix::process::CommandExt;
            let core_entrypoint = validated_core_entrypoint()?;
            let mut command = std::process::Command::new(selection.program_path);
            command
                .env(MANAGER_CORE_API_ENV, MANAGER_CORE_API)
                .env(MANAGER_CORE_ENTRYPOINT_ENV, core_entrypoint)
                .env_remove(MANAGER_ARTIFACT_PROBE_ENV)
                .args(args);
            Err(command.exec())
        }
    }
}

#[cfg(unix)]
#[derive(Debug)]
enum RuntimeLaunchError {
    Environment(TermuxProcessEnvError),
    Exec(std::io::Error),
}

#[cfg(unix)]
impl std::fmt::Display for RuntimeLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeLaunchError::Environment(err) => err.fmt(f),
            RuntimeLaunchError::Exec(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for RuntimeLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuntimeLaunchError::Environment(err) => Some(err),
            RuntimeLaunchError::Exec(err) => Some(err),
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
#[derive(Debug, Clone, Copy)]
struct QualifiedRuntimeLaunchOptions<'args> {
    planned_args: &'args [OsString],
    manager_available: bool,
}

#[cfg(unix)]
fn launch_qualified_runtime<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    options: QualifiedRuntimeLaunchOptions<'_>,
) -> RuntimeLaunchError
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let selection = assets.selection();
    let (browser_open_helper, browser_manual_helper) = match termux_browser_helper_paths(selection)
    {
        Ok(paths) => paths,
        Err(err) => return RuntimeLaunchError::Environment(err),
    };
    let env_plan = match plan_termux_env(
        process_env,
        selection.compatibility_dir,
        browser_open_helper,
        browser_manual_helper,
        cert_file,
        cert_dir,
    ) {
        Ok(plan) => plan,
        Err(err) => return RuntimeLaunchError::Environment(err),
    };

    prepare_core_notification_config(config_dir.as_ref(), options.manager_available);
    RuntimeLaunchError::Exec(exec_runtime(
        selection.runtime.program_path,
        options.planned_args,
        resolver_path,
        config_dir,
        Some(&env_plan),
    ))
}

#[cfg(unix)]
const CORE_NOTIFY_RECORD_MAX_BYTES: usize = 4096;

#[cfg(unix)]
const CORE_NOTIFY_EVENTS: [&str; 10] = [
    "SessionStart",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "UserPromptSubmit",
    "SubagentStart",
    "SubagentStop",
    "Stop",
];

#[cfg(unix)]
const CORE_NOTIFY_MARKER: &str = "# codex-termux-notify-v1\n";

#[cfg(unix)]
fn core_notify_safe_absolute_path(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}

#[cfg(unix)]
fn core_notify_real_path(path: &std::path::Path) -> Option<std::path::PathBuf> {
    if !core_notify_safe_absolute_path(path) {
        return None;
    }
    let mut current = std::path::PathBuf::from("/");
    let components: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::RootDir => None,
            std::path::Component::Normal(value) => Some(value.to_owned()),
            _ => None,
        })
        .collect();
    for (index, component) in components.iter().enumerate() {
        current.push(component);
        let metadata = std::fs::symlink_metadata(&current).ok()?;
        if metadata.file_type().is_symlink()
            || (index + 1 != components.len() && !metadata.is_dir())
        {
            return None;
        }
    }
    Some(path.to_owned())
}

#[cfg(unix)]
fn core_manager_notify_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from)?;
    core_notify_real_path(&home)?;
    let config = home.join(".local/share/codex/manager/notifications/config-v1");
    core_notify_real_path(&config)
}

#[cfg(unix)]
fn core_notify_record_field<'a>(line: Option<&'a str>, key: &str) -> Option<&'a str> {
    line?.strip_prefix(&format!("{key}\t"))
}

#[cfg(unix)]
fn core_notify_valid_bool(value: &str) -> bool {
    matches!(value, "0" | "1")
}

#[cfg(unix)]
fn core_notify_valid_color(value: &str) -> bool {
    if value == "empty" {
        return true;
    }
    let bytes = value.as_bytes();
    bytes.len() == 7
        && bytes[0] == b'#'
        && bytes[1..].iter().all(u8::is_ascii_hexdigit)
        && value
            .chars()
            .skip(1)
            .all(|character| !character.is_ascii_uppercase())
}

#[cfg(unix)]
fn core_notify_parse_hooks(value: &str) -> Option<Vec<&'static str>> {
    if value == "none" {
        return Some(Vec::new());
    }
    if value == "all" {
        return Some(CORE_NOTIFY_EVENTS.to_vec());
    }
    if value.is_empty() {
        return None;
    }
    let mut enabled = [false; CORE_NOTIFY_EVENTS.len()];
    for item in value.split(',') {
        let index = CORE_NOTIFY_EVENTS.iter().position(|event| *event == item)?;
        if enabled[index] {
            return None;
        }
        enabled[index] = true;
    }
    Some(
        CORE_NOTIFY_EVENTS
            .iter()
            .enumerate()
            .filter_map(|(index, event)| enabled[index].then_some(*event))
            .collect(),
    )
}

#[cfg(unix)]
fn core_notify_parse_record(bytes: &[u8]) -> Option<Vec<&'static str>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.split('\n');
    if lines.next() != Some("codex-manager-notify-v1") {
        return None;
    }
    let channel = core_notify_record_field(lines.next(), "channel")?;
    if !matches!(channel, "notification" | "toast" | "both") {
        return None;
    }
    let hooks = core_notify_parse_hooks(core_notify_record_field(lines.next(), "hooks")?)?;
    let content_chars = core_notify_record_field(lines.next(), "content_chars")?;
    if content_chars.is_empty()
        || content_chars.bytes().any(|byte| !byte.is_ascii_digit())
        || content_chars.parse::<usize>().ok()? > 4096
    {
        return None;
    }
    if !core_notify_valid_bool(core_notify_record_field(lines.next(), "preserve_newlines")?) {
        return None;
    }
    let gravity = core_notify_record_field(lines.next(), "toast_gravity")?;
    if !matches!(gravity, "top" | "middle" | "bottom") {
        return None;
    }
    if !core_notify_valid_bool(core_notify_record_field(lines.next(), "toast_short")?) {
        return None;
    }
    if !core_notify_valid_color(core_notify_record_field(lines.next(), "toast_background")?)
        || !core_notify_valid_color(core_notify_record_field(lines.next(), "toast_color")?)
    {
        return None;
    }
    let group = core_notify_record_field(lines.next(), "group")?;
    let group_bytes = group.as_bytes();
    if group_bytes.is_empty()
        || group_bytes.len() > 64
        || !group_bytes[0].is_ascii_alphanumeric()
        || !group_bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return None;
    }
    if lines.next() != Some("") || lines.next().is_some() {
        return None;
    }
    Some(hooks)
}

#[cfg(unix)]
fn core_read_manager_notify_hooks() -> Option<Vec<&'static str>> {
    use std::io::Read as _;
    use std::os::unix::fs::PermissionsExt;

    let path = core_manager_notify_path()?;
    let metadata = std::fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o7777 != 0o600
    {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take((CORE_NOTIFY_RECORD_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > CORE_NOTIFY_RECORD_MAX_BYTES {
        return None;
    }
    core_notify_parse_record(&bytes)
}

#[cfg(unix)]
fn core_notify_status_message(event: &str) -> &'static str {
    match event {
        "SessionStart" => "Notify session start",
        "PreToolUse" => "Notify tool start",
        "PermissionRequest" => "Notify permission request",
        "PostToolUse" => "Notify tool finish",
        "PreCompact" => "Notify before compact",
        "PostCompact" => "Notify after compact",
        "UserPromptSubmit" => "Notify prompt submit",
        "SubagentStart" => "Notify subagent start",
        "SubagentStop" => "Notify subagent stop",
        "Stop" => "Notify turn completion",
        _ => "Notify Codex event",
    }
}

#[cfg(unix)]
fn render_core_notification_config(events: &[&str]) -> Vec<u8> {
    let mut output = String::from(CORE_NOTIFY_MARKER);
    output.push_str("mcp_oauth_credentials_store = \"file\"\n\n");
    for event in events {
        output.push_str(&format!(
            "[[hooks.{event}]]\n\n[[hooks.{event}.hooks]]\ntype = \"command\"\ncommand = \"codex termux notify emit {event}\"\ntimeout = 10\nstatusMessage = \"{}\"\n\n",
            core_notify_status_message(event)
        ));
    }
    output.into_bytes()
}

#[cfg(unix)]
fn prepare_core_notification_config(config_dir: &std::path::Path, manager_available: bool) {
    use std::io::{Read as _, Write as _};
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let Ok(directory_metadata) = std::fs::symlink_metadata(config_dir) else {
        return;
    };
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return;
    }
    let destination = config_dir.join("config.toml");
    if let Ok(metadata) = std::fs::symlink_metadata(&destination) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return;
        }
        if metadata.len() < CORE_NOTIFY_MARKER.len() as u64 {
            return;
        }
        let existing = std::fs::File::open(&destination).ok().and_then(|file| {
            let mut bytes = Vec::new();
            file.take(CORE_NOTIFY_MARKER.len() as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            Some(bytes)
        });
        if existing.as_deref() != Some(CORE_NOTIFY_MARKER.as_bytes()) {
            return;
        }
    }
    let events = manager_available
        .then(core_read_manager_notify_hooks)
        .flatten()
        .unwrap_or_default();
    let bytes = render_core_notification_config(&events);
    let temporary = config_dir.join(format!(
        ".codex-termux-notify-{}-{}",
        std::process::id(),
        LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .ok()?;
        output.write_all(&bytes).ok()?;
        output.sync_all().ok()?;
        if output.metadata().ok()?.permissions().mode() & 0o7777 != 0o600 {
            return None;
        }
        std::fs::rename(&temporary, &destination).ok()?;
        std::fs::File::open(config_dir).ok()?.sync_all().ok()?;
        Some(())
    })();
    if result.is_none() {
        let _ = std::fs::remove_file(&temporary);
    }
}

#[cfg(unix)]
#[derive(Debug)]
enum QualifiedUpstreamDoctorProbeError {
    Environment(TermuxProcessEnvError),
    Io(std::io::Error),
    OutputTooLarge,
}

#[cfg(unix)]
impl std::fmt::Display for QualifiedUpstreamDoctorProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedUpstreamDoctorProbeError::Environment(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::Io(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::OutputTooLarge => {
                f.write_str("upstream doctor output exceeds its byte bound")
            }
        }
    }
}

#[cfg(unix)]
impl std::error::Error for QualifiedUpstreamDoctorProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QualifiedUpstreamDoctorProbeError::Environment(err) => Some(err),
            QualifiedUpstreamDoctorProbeError::Io(err) => Some(err),
            QualifiedUpstreamDoctorProbeError::OutputTooLarge => None,
        }
    }
}

/// Runs one read-only qualified upstream command as a child of Core.
#[cfg(unix)]
fn probe_qualified_upstream_command<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    args: &[&str],
) -> Result<bool, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    let selection = assets.selection();
    let (browser_open_helper, browser_manual_helper) = termux_browser_helper_paths(selection)
        .map_err(QualifiedUpstreamDoctorProbeError::Environment)?;
    let env_plan = plan_termux_env(
        process_env,
        selection.compatibility_dir,
        browser_open_helper,
        browser_manual_helper,
        cert_file,
        cert_dir,
    )
    .map_err(QualifiedUpstreamDoctorProbeError::Environment)?;
    let runtime_fds = RuntimeFdSources::open(resolver_path, config_dir)
        .map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    let mut cmd = std::process::Command::new(selection.runtime.program_path);
    cmd.args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
    runtime_fds.configure(&mut cmd);
    Ok(cmd
        .status()
        .map_err(QualifiedUpstreamDoctorProbeError::Io)?
        .success())
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct QualifiedUpstreamDoctorResult {
    status: UpstreamDoctorStatus,
    output: String,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct DoctorCaptureOptions {
    json: bool,
    use_color: bool,
    force_color: bool,
}

#[cfg(unix)]
fn sanitize_doctor_terminal_controls(value: &str, preserve_sgr: bool) -> String {
    fn erase_current_line(output: &mut String) {
        if let Some(newline) = output.rfind('\n') {
            output.truncate(newline + 1);
        } else {
            output.clear();
        }
    }

    let mut result = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                if bytes.get(index + 1) != Some(&b'\n') {
                    erase_current_line(&mut result);
                }
                index += 1;
            }
            b'\n' | b'\t' => {
                result.push(bytes[index] as char);
                index += 1;
            }
            0x1b => {
                if bytes.get(index + 1) == Some(&b'[') {
                    let parameter_start = index + 2;
                    let Some(final_offset) = bytes[parameter_start..]
                        .iter()
                        .position(|byte| (b'@'..=b'~').contains(byte))
                    else {
                        break;
                    };
                    let final_index = parameter_start + final_offset;
                    let parameters = &bytes[parameter_start..final_index];
                    if bytes[final_index] == b'K' {
                        erase_current_line(&mut result);
                    } else if preserve_sgr
                        && bytes[final_index] == b'm'
                        && parameters
                            .iter()
                            .all(|byte| byte.is_ascii_digit() || *byte == b';')
                    {
                        result.push_str(&value[index..=final_index]);
                    }
                    index = final_index + 1;
                } else if bytes.get(index + 1) == Some(&b']') {
                    index += 2;
                    while index < bytes.len() {
                        if bytes[index] == 0x07 {
                            index += 1;
                            break;
                        }
                        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
                            index += 2;
                            break;
                        }
                        index += 1;
                    }
                } else {
                    index = (index + 2).min(bytes.len());
                }
            }
            byte if byte.is_ascii_control() => {
                index += 1;
            }
            _ => {
                let character = value[index..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 input has a character at every byte boundary");
                result.push(character);
                index += character.len_utf8();
            }
        }
    }
    result
}

#[cfg(unix)]
fn replace_doctor_redaction(
    line: &mut String,
    boundaries: &mut Vec<usize>,
    start: usize,
    end: usize,
) -> (usize, usize) {
    const REDACTION: &str = "[redacted]";

    let original_start = boundaries[start];
    let original_end = boundaries[end];
    line.replace_range(start..end, REDACTION);

    let mut updated = Vec::with_capacity(boundaries.len() - (end - start) + REDACTION.len());
    updated.extend_from_slice(&boundaries[..=start]);
    for _ in 1..REDACTION.len() {
        updated.push(original_start);
    }
    updated.push(original_end);
    updated.extend_from_slice(&boundaries[end + 1..]);
    *boundaries = updated;
    (original_start, original_end)
}

#[cfg(unix)]
fn redact_doctor_line_with_ranges(line: &mut String) -> (String, Vec<(usize, usize)>) {
    const SENSITIVE_KEYS: [&str; 11] = [
        "access_token",
        "refresh_token",
        "oauth_token",
        "token",
        "api_key",
        "authorization",
        "cookie",
        "client_secret",
        "secret",
        "private_key",
        "password",
    ];
    let mut boundaries: Vec<usize> = (0..=line.len()).collect();
    let mut ranges = Vec::new();

    for key in SENSITIVE_KEYS {
        let mut search_from = 0;
        loop {
            let lowered = line.to_ascii_lowercase();
            let Some(key_offset) = lowered[search_from..].find(key) else {
                break;
            };
            let key_start = search_from + key_offset;
            let after_key = key_start + key.len();
            let Some(delimiter_offset) = line[after_key..].find([':', '=']) else {
                search_from = after_key;
                continue;
            };
            let delimiter = after_key + delimiter_offset;
            let mut value_start = delimiter + 1;
            while let Some(character) = line[value_start..].chars().next() {
                if character == ' ' || character == '\t' {
                    value_start += character.len_utf8();
                } else {
                    break;
                }
            }
            let quoted = line[value_start..].starts_with('"');
            if quoted {
                value_start += 1;
            }
            let mut value_end = value_start;
            for (offset, character) in line[value_start..].char_indices() {
                let is_end = if quoted {
                    character == '"'
                } else if matches!(key, "authorization" | "cookie") {
                    character == ',' || character == '}' || character == ']'
                } else {
                    character == '"'
                        || character == ','
                        || character == '}'
                        || character.is_ascii_whitespace()
                };
                if is_end {
                    break;
                }
                value_end = value_start + offset + character.len_utf8();
            }
            if value_end == value_start {
                search_from = after_key;
                continue;
            }
            if line[value_start..].starts_with("[redacted]") {
                search_from = value_start + "[redacted]".len();
                continue;
            }
            ranges.push(replace_doctor_redaction(
                line,
                &mut boundaries,
                value_start,
                value_end,
            ));
            search_from = value_start + "[redacted]".len();
            if search_from >= line.len() {
                break;
            }
        }
    }

    for marker in ["bearer ", "sk-", "sess-", "ghp_"] {
        let mut search_from = 0;
        loop {
            let lowered = line.to_ascii_lowercase();
            let Some(marker_offset) = lowered[search_from..].find(marker) else {
                break;
            };
            let start = search_from + marker_offset;
            let value_start = start + marker.len();
            let mut value_end = value_start;
            for (offset, character) in line[value_start..].char_indices() {
                if character == '"'
                    || character == '\''
                    || character == ','
                    || character.is_ascii_whitespace()
                {
                    break;
                }
                value_end = value_start + offset + character.len_utf8();
            }
            if value_end == value_start {
                search_from = value_start;
                continue;
            }
            ranges.push(replace_doctor_redaction(
                line,
                &mut boundaries,
                value_start,
                value_end,
            ));
            search_from = value_start + "[redacted]".len();
            if search_from >= line.len() {
                break;
            }
        }
    }
    (line.to_owned(), ranges)
}

#[cfg(unix)]
fn doctor_sgr_end(value: &str, start: usize) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.get(start) != Some(&0x1b) || bytes.get(start + 1) != Some(&b'[') {
        return None;
    }
    let mut index = start + 2;
    while index < bytes.len() {
        if bytes[index] == b'm' {
            return Some(index + 1);
        }
        index += 1;
    }
    None
}

#[cfg(unix)]
fn doctor_styled_boundaries(styled: &str) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut plain_length = 0;
    let mut index = 0;
    while index < styled.len() {
        if let Some(end) = doctor_sgr_end(styled, index) {
            index = end;
            boundaries[plain_length] = index;
            continue;
        }
        let character = styled[index..]
            .chars()
            .next()
            .expect("valid UTF-8 styled doctor output has a character at every byte boundary");
        let width = character.len_utf8();
        for offset in 1..=width {
            boundaries.push(index + offset);
        }
        plain_length += width;
        index += width;
    }
    boundaries
}

#[cfg(unix)]
fn doctor_styled_redaction_segment(segment: &str) -> String {
    let mut replacement = String::new();
    let mut inserted = false;
    let mut index = 0;
    while index < segment.len() {
        if let Some(end) = doctor_sgr_end(segment, index) {
            replacement.push_str(&segment[index..end]);
            index = end;
            continue;
        }
        let character = segment[index..]
            .chars()
            .next()
            .expect("valid UTF-8 styled doctor segment has a character at every byte boundary");
        if !inserted {
            replacement.push_str("[redacted]");
            inserted = true;
        }
        index += character.len_utf8();
    }
    if !inserted {
        replacement.push_str("[redacted]");
    }
    replacement
}

#[cfg(unix)]
fn doctor_styled_redactions(styled: &str, ranges: &[(usize, usize)]) -> Option<String> {
    if ranges.is_empty() {
        return Some(styled.to_owned());
    }
    let boundaries = doctor_styled_boundaries(styled);
    let mut output = styled.to_owned();
    let mut ranges = ranges.to_vec();
    ranges.sort_by_key(|range| std::cmp::Reverse(range.0));
    for (start, end) in ranges {
        let styled_start = *boundaries.get(start)?;
        let styled_end = *boundaries.get(end)?;
        if styled_start > styled_end || styled_end > output.len() {
            return None;
        }
        let replacement = doctor_styled_redaction_segment(&output[styled_start..styled_end]);
        output.replace_range(styled_start..styled_end, &replacement);
    }
    Some(output)
}

#[cfg(unix)]
fn redact_doctor_output(bytes: &[u8], preserve_sgr: bool) -> String {
    let text = String::from_utf8_lossy(bytes);
    let styled = sanitize_doctor_terminal_controls(&text, preserve_sgr);
    let plain = sanitize_doctor_terminal_controls(&text, false);
    let mut redacted = String::with_capacity(plain.len());
    let mut styled_chunks = styled.split_inclusive('\n');
    for chunk in plain.split_inclusive('\n') {
        let (line, newline) = chunk
            .strip_suffix('\n')
            .map_or((chunk, ""), |line| (line, "\n"));
        let styled_chunk = styled_chunks.next().unwrap_or_default();
        let (styled_line, _) = styled_chunk
            .strip_suffix('\n')
            .map_or((styled_chunk, ""), |line| (line, "\n"));
        let mut plain_line = line.to_owned();
        let (redacted_line, ranges) = redact_doctor_line_with_ranges(&mut plain_line);
        if preserve_sgr {
            if let Some(styled_line) = doctor_styled_redactions(styled_line, &ranges) {
                redacted.push_str(&styled_line);
            } else {
                redacted.push_str(&redacted_line);
            }
        } else {
            redacted.push_str(&redacted_line);
        }
        redacted.push_str(newline);
    }
    redacted
}

#[cfg(unix)]
fn doctor_color_enabled(force_color: bool) -> bool {
    use std::io::IsTerminal as _;

    doctor_color_policy(
        std::io::stdout().is_terminal(),
        std::env::var_os("NO_COLOR").is_some(),
        force_color,
    )
}

#[cfg(unix)]
fn doctor_color_policy(
    stdout_is_terminal: bool,
    no_color_present: bool,
    force_color: bool,
) -> bool {
    stdout_is_terminal && (force_color || !no_color_present)
}

#[cfg(unix)]
fn apply_doctor_color_override(
    cmd: &mut std::process::Command,
    use_color: bool,
    force_color: bool,
) {
    if use_color && force_color {
        cmd.env_remove("NO_COLOR");
    }
}

#[cfg(unix)]
fn doctor_shell_quote(value: &OsStr) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let mut quoted = Vec::with_capacity(value.as_bytes().len() + 2);
    quoted.push(b'\'');
    for byte in value.as_bytes() {
        if *byte == b'\'' {
            quoted.extend_from_slice(b"'\"'\"'");
        } else {
            quoted.push(*byte);
        }
    }
    quoted.push(b'\'');
    OsString::from_vec(quoted)
}

#[cfg(unix)]
fn doctor_script_command(runtime: &OsStr, json: bool) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let quoted_runtime = doctor_shell_quote(runtime);
    let mut command = quoted_runtime.as_bytes().to_vec();
    command.extend_from_slice(b" -c 'sandbox_mode=\"danger-full-access\"' doctor");
    if json {
        command.extend_from_slice(b" --json");
    }
    OsString::from_vec(command)
}

#[cfg(unix)]
fn doctor_pty_path(process_env: &TermuxProcessEnvSnapshot) -> Option<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let prefix = process_env.prefix.as_deref()?;
    let path = std::path::Path::new(prefix).join("bin/script");
    let metadata = std::fs::symlink_metadata(&path).ok()?;
    (metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0).then_some(path)
}

#[cfg(unix)]
fn capture_qualified_upstream_doctor<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    options: DoctorCaptureOptions,
) -> Result<QualifiedUpstreamDoctorResult, QualifiedUpstreamDoctorProbeError>
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
{
    use std::io::Read as _;

    let json = options.json;
    let force_color = options.force_color;
    let mut use_color = options.use_color;
    let selection = assets.selection();
    let (browser_open_helper, browser_manual_helper) = termux_browser_helper_paths(selection)
        .map_err(QualifiedUpstreamDoctorProbeError::Environment)?;
    let env_plan = plan_termux_env(
        process_env,
        selection.compatibility_dir,
        browser_open_helper,
        browser_manual_helper,
        cert_file,
        cert_dir,
    )
    .map_err(QualifiedUpstreamDoctorProbeError::Environment)?;
    let runtime_fds = RuntimeFdSources::open(resolver_path, config_dir)
        .map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    let pty_path = (!json && use_color)
        .then(|| doctor_pty_path(process_env))
        .flatten();
    let mut cmd = if let Some(script) = pty_path.as_ref() {
        let mut command = std::process::Command::new(script);
        command
            .args(["-qefc"])
            .arg(doctor_script_command(selection.runtime.program_path, json))
            .arg("/dev/null");
        command
    } else {
        use_color = false;
        std::process::Command::new(selection.runtime.program_path)
    };
    if pty_path.is_none() {
        cmd.args(["-c", "sandbox_mode=\"danger-full-access\"", "doctor"]);
        if json {
            cmd.arg("--json");
        }
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());
    apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
    apply_doctor_color_override(&mut cmd, use_color, force_color);
    runtime_fds.configure(&mut cmd);
    let mut child = cmd.spawn().map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    let stdout = child.stdout.take().ok_or_else(|| {
        QualifiedUpstreamDoctorProbeError::Io(std::io::Error::other(
            "upstream doctor stdout is unavailable",
        ))
    })?;
    let mut bytes = Vec::new();
    stdout
        .take((DOCTOR_OUTPUT_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    if bytes.len() > DOCTOR_OUTPUT_MAX_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err(QualifiedUpstreamDoctorProbeError::OutputTooLarge);
    }
    let status = child
        .wait()
        .map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    Ok(QualifiedUpstreamDoctorResult {
        status: if status.success() {
            UpstreamDoctorStatus::Healthy
        } else {
            UpstreamDoctorStatus::Unhealthy
        },
        output: redact_doctor_output(&bytes, use_color && !json),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpstreamDoctorCapability {
    Supported,
    Unsupported,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorExitClass {
    Success,
    HealthFailure,
    ApiIncompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxCoreDoctorReport {
    status: CoreDoctorStatus,
    generation_id: String,
    generation_layout: GenerationLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorReport {
    upstream: QualifiedUpstreamDoctorResult,
    termux_core: TermuxCoreDoctorReport,
    manager: ManagerDoctorStatus,
    summary: DoctorSummaryStatus,
}

fn compose_doctor_report(
    upstream: QualifiedUpstreamDoctorResult,
    termux_core: TermuxCoreDoctorReport,
    manager: ManagerDoctorStatus,
) -> DoctorReport {
    let summary = if termux_core.status == CoreDoctorStatus::ApiIncompatible
        || manager == ManagerDoctorStatus::ApiIncompatible
    {
        DoctorSummaryStatus::ApiIncompatible
    } else if upstream.status == UpstreamDoctorStatus::Unhealthy
        || termux_core.status == CoreDoctorStatus::Unhealthy
        || manager == ManagerDoctorStatus::Unhealthy
    {
        DoctorSummaryStatus::Unhealthy
    } else if upstream.status == UpstreamDoctorStatus::Unsupported
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

fn doctor_exit_class(report: &DoctorReport) -> DoctorExitClass {
    match report.summary {
        DoctorSummaryStatus::Healthy => DoctorExitClass::Success,
        DoctorSummaryStatus::Degraded | DoctorSummaryStatus::Unhealthy => {
            DoctorExitClass::HealthFailure
        }
        DoctorSummaryStatus::ApiIncompatible => DoctorExitClass::ApiIncompatibility,
    }
}

fn doctor_color(text: &str, code: u16, enabled: bool) -> String {
    if enabled {
        format!("\x1b[38;5;{code}m{text}\x1b[39m")
    } else {
        text.to_owned()
    }
}

fn doctor_bold(text: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[1m{text}\x1b[22m")
    } else {
        text.to_owned()
    }
}

fn doctor_dim(text: &str, enabled: bool) -> String {
    doctor_color(text, 240, enabled)
}

fn doctor_row(output: &mut String, name: &str, description: &str, healthy: bool, color: bool) {
    let marker = if healthy {
        doctor_color("✓", 10, color)
    } else {
        doctor_color("✗", 196, color)
    };
    let description = if healthy {
        doctor_dim(description, color)
    } else {
        description.to_owned()
    };
    output.push_str(&format!("  {marker} {name:<14} {description}\n"));
}

fn doctor_warning(output: &mut String, name: &str, description: &str, color: bool) {
    let marker = doctor_color("⚠", 214, color);
    output.push_str(&format!("  {marker} {name:<14} {description}\n"));
}

fn doctor_detail(output: &mut String, name: &str, value: &str, color: bool) {
    output.push_str("    ");
    output.push_str(&doctor_dim(name, color));
    output.push_str(": ");
    output.push_str(value);
    output.push('\n');
}

fn render_doctor_human(report: &DoctorReport, color: bool) -> String {
    let mut output = String::new();
    if report.upstream.output.is_empty() {
        output.push_str("Upstream doctor output unavailable: ");
        output.push_str(report.upstream.status.as_str());
        output.push('\n');
    } else {
        output.push_str(&report.upstream.output);
        if !report.upstream.output.ends_with('\n') {
            output.push('\n');
        }
        if color {
            output.push_str("\x1b[0m");
        }
    }

    let code_mode_status = match report.termux_core.generation_layout {
        GenerationLayout::RootCodeModeHost => "healthy",
        GenerationLayout::LegacyCompat => "migration_required",
    };
    let core_healthy = report.termux_core.status == CoreDoctorStatus::Healthy;
    let code_mode_healthy = code_mode_status == "healthy";
    let manager_healthy = report.manager == ManagerDoctorStatus::Healthy;
    let summary_healthy = report.summary == DoctorSummaryStatus::Healthy;
    let mut ok_count = 0;
    let mut warning_count = 1;
    let mut failure_count = 0;

    output.push_str("\n[Termux doctor]\n");
    let header = format!(
        "Codex Termux Wrapper Doctor · generation {} · status {}",
        report.termux_core.generation_id,
        report.termux_core.status.as_str()
    );
    output.push_str(&doctor_bold(&header, color));
    output.push('\n');

    output.push('\n');
    output.push_str(&doctor_bold("Runtime", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "runtime",
        "selected runtime is qualified",
        true,
        color,
    );
    ok_count += 1;
    doctor_detail(
        &mut output,
        "generation_id",
        &report.termux_core.generation_id,
        color,
    );
    doctor_detail(
        &mut output,
        "layout",
        report.termux_core.generation_layout.as_str(),
        color,
    );
    doctor_row(
        &mut output,
        "code host",
        "code-mode companion is beside the runtime",
        code_mode_healthy,
        color,
    );
    if code_mode_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    doctor_detail(&mut output, "code_mode_host", code_mode_status, color);

    output.push('\n');
    output.push_str(&doctor_bold("Support", color));
    output.push('\n');
    doctor_warning(
        &mut output,
        "sandbox",
        "Linux namespace/mount sandboxing is unsupported on Termux; bwrap is not used",
        color,
    );
    doctor_detail(
        &mut output,
        "sandbox",
        "unsupported (bwrap is not used)",
        color,
    );

    output.push('\n');
    output.push_str(&doctor_bold("Wrapper", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "core",
        "Core API and selected generation are compatible",
        core_healthy,
        color,
    );
    if core_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    doctor_detail(
        &mut output,
        "core",
        report.termux_core.status.as_str(),
        color,
    );

    output.push('\n');
    output.push_str(&doctor_bold("State", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "current",
        "one complete generation is selected",
        true,
        color,
    );
    ok_count += 1;
    doctor_detail(
        &mut output,
        "generation",
        &report.termux_core.generation_id,
        color,
    );

    output.push('\n');
    output.push_str(&doctor_bold("Store", color));
    output.push('\n');
    doctor_row(
        &mut output,
        "generation",
        "immutable generation is loaded from the Core store",
        true,
        color,
    );
    ok_count += 1;

    output.push_str("\n[Manager]\n");
    if manager_healthy {
        doctor_row(
            &mut output,
            "status",
            "Manager artifact is available",
            true,
            color,
        );
        ok_count += 1;
    } else {
        doctor_warning(&mut output, "status", report.manager.as_str(), color);
        warning_count += 1;
    }

    output.push_str("\n[Summary]\n");
    doctor_row(
        &mut output,
        "status",
        report.summary.as_str(),
        summary_healthy,
        color,
    );
    if summary_healthy {
        ok_count += 1;
    } else {
        failure_count += 1;
    }
    output.push('\n');
    output.push_str(&doctor_dim(
        "─────────────────────────────────────────────────────────────",
        color,
    ));
    output.push('\n');
    let summary_status = if summary_healthy {
        doctor_color("healthy", 10, color)
    } else {
        doctor_color(report.summary.as_str(), 196, color)
    };
    output.push_str(&format!(
        "{} ok · {} warn · {} fail {}\n",
        ok_count, warning_count, failure_count, summary_status
    ));
    output
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write as _;
                write!(&mut escaped, "\\u{:04x}", character as u32)
                    .expect("writing into String cannot fail");
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn render_doctor_json(report: &DoctorReport) -> String {
    format!(
        "{{\"schema_version\":2,\"upstream\":{{\"status\":\"{}\",\"output\":\"{}\"}},\"termux_core\":{{\"status\":\"{}\",\"generation_id\":\"{}\",\"layout\":\"{}\",\"runtime\":{{\"status\":\"healthy\"}},\"code_mode_host\":{{\"status\":\"{}\"}},\"sandbox\":{{\"status\":\"unsupported\",\"reason\":\"bwrap is not used\"}}}},\"manager\":{{\"status\":\"{}\"}},\"summary\":{{\"status\":\"{}\"}}}}\n",
        report.upstream.status.as_str(),
        json_escape(&report.upstream.output),
        report.termux_core.status.as_str(),
        json_escape(&report.termux_core.generation_id),
        report.termux_core.generation_layout.as_str(),
        match report.termux_core.generation_layout {
            GenerationLayout::RootCodeModeHost => "healthy",
            GenerationLayout::LegacyCompat => "migration_required",
        },
        report.manager.as_str(),
        report.summary.as_str(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorOutputMode {
    Human,
    Color,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorCommandOutcome {
    output: String,
    exit_class: DoctorExitClass,
}

#[cfg(unix)]
#[derive(Debug)]
enum LocalDoctorCommandError {
    Usage,
}

#[cfg(unix)]
impl std::fmt::Display for LocalDoctorCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalDoctorCommandError::Usage => f.write_str("usage: codex doctor [--json|--color]"),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for LocalDoctorCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocalDoctorCommandError::Usage => None,
        }
    }
}

fn doctor_output_mode<I, S>(args: I) -> Result<DoctorOutputMode, LocalDoctorCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into);
    match (args.next(), args.next()) {
        (None, None) => Ok(DoctorOutputMode::Human),
        (Some(arg), None) if arg.as_os_str() == OsStr::new("--color") => {
            Ok(DoctorOutputMode::Color)
        }
        (Some(arg), None) if arg.as_os_str() == OsStr::new("--json") => Ok(DoctorOutputMode::Json),
        _ => Err(LocalDoctorCommandError::Usage),
    }
}

#[cfg(unix)]
fn run_local_doctor_command<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
    I,
    S,
>(
    args: I,
    context: LocalPublicDispatchContext<
        'context,
        'runtime_selection,
        'runtime_asset,
        'manager_selection,
        'manager_asset,
    >,
) -> Result<DoctorCommandOutcome, LocalDoctorCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mode = doctor_output_mode(args)?;
    let color = match mode {
        DoctorOutputMode::Human => doctor_color_enabled(false),
        DoctorOutputMode::Color => doctor_color_enabled(true),
        DoctorOutputMode::Json => false,
    };
    let json = mode == DoctorOutputMode::Json;
    let force_color = mode == DoctorOutputMode::Color;
    let upstream = match context.doctor_capability {
        UpstreamDoctorCapability::Supported => match capture_qualified_upstream_doctor(
            context.runtime_assets,
            context.process_env,
            context.cert_file,
            context.cert_dir,
            context.resolver_path,
            context.config_dir,
            DoctorCaptureOptions {
                json,
                use_color: color,
                force_color,
            },
        ) {
            Ok(result) => result,
            Err(_) => QualifiedUpstreamDoctorResult {
                status: UpstreamDoctorStatus::Unhealthy,
                output: String::new(),
            },
        },
        UpstreamDoctorCapability::Unsupported => QualifiedUpstreamDoctorResult {
            status: UpstreamDoctorStatus::Unsupported,
            output: String::new(),
        },
    };
    let report = compose_doctor_report(
        upstream,
        TermuxCoreDoctorReport {
            status: context.core_doctor_status,
            generation_id: context.generation_id.to_owned(),
            generation_layout: context.generation_layout,
        },
        context.manager_doctor_status,
    );
    let output = match mode {
        DoctorOutputMode::Human | DoctorOutputMode::Color => render_doctor_human(&report, color),
        DoctorOutputMode::Json => render_doctor_json(&report),
    };
    Ok(DoctorCommandOutcome {
        output,
        exit_class: doctor_exit_class(&report),
    })
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct LocalPublicDispatchContext<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
> {
    runtime_assets: QualifiedRuntimeAssets<'runtime_selection, 'runtime_asset>,
    manager_artifact: ManagerArtifact<'manager_selection, 'manager_asset>,
    process_env: &'context TermuxProcessEnvSnapshot,
    cert_file: &'context OsStr,
    cert_dir: Option<&'context OsStr>,
    resolver_path: &'context std::path::Path,
    config_dir: &'context std::path::Path,
    doctor_capability: UpstreamDoctorCapability,
    core_doctor_status: CoreDoctorStatus,
    generation_id: &'context str,
    generation_layout: GenerationLayout,
    manager_doctor_status: ManagerDoctorStatus,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicDispatchCompletion {
    Update(Vec<OsString>),
    Doctor(DoctorCommandOutcome),
    TermuxUnavailable(&'static str),
}

#[cfg(unix)]
#[derive(Debug)]
enum PublicDispatchExecutionError {
    UnsupportedDaemon,
    Upstream(RuntimeLaunchError),
    Doctor(LocalDoctorCommandError),
    Manager(std::io::Error),
}

#[cfg(unix)]
impl std::fmt::Display for PublicDispatchExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicDispatchExecutionError::UnsupportedDaemon => {
                f.write_str(TERMUX_DAEMON_UNSUPPORTED)
            }
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
            PublicDispatchExecutionError::UnsupportedDaemon => None,
            PublicDispatchExecutionError::Upstream(err) => Some(err),
            PublicDispatchExecutionError::Doctor(err) => Some(err),
            PublicDispatchExecutionError::Manager(err) => Some(err),
        }
    }
}

#[cfg(unix)]
fn execute_public_dispatch<
    'context,
    'runtime_selection,
    'runtime_asset,
    'manager_selection,
    'manager_asset,
>(
    route: PublicDispatchRoute,
    context: LocalPublicDispatchContext<
        'context,
        'runtime_selection,
        'runtime_asset,
        'manager_selection,
        'manager_asset,
    >,
) -> Result<PublicDispatchCompletion, PublicDispatchExecutionError> {
    match route {
        PublicDispatchRoute::UnsupportedDaemon => {
            Err(PublicDispatchExecutionError::UnsupportedDaemon)
        }
        PublicDispatchRoute::Update(args) => Ok(PublicDispatchCompletion::Update(args)),
        PublicDispatchRoute::Doctor(args) => run_local_doctor_command(args, context)
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
                QualifiedRuntimeLaunchOptions {
                    planned_args: &args,
                    manager_available: matches!(
                        context.manager_artifact,
                        ManagerArtifact::Available(_)
                    ),
                },
            ),
        )),
    }
}

#[cfg(unix)]
mod m2_generation_state {
    use super::{flock, ReleasePublicKey, FLOCK_EX, FLOCK_NB, FLOCK_UN};
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;

    const GENERATION_ID_MAX_BYTES: usize = 512;
    const STATE_FILE_MAX_BYTES: usize = 16 * 1024;
    const STATE_FORMAT: &str = "codex-activation-state-v3";
    const JOURNAL_FORMAT: &str = "codex-activation-journal-v3";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct CoreStatePaths {
        pub(super) root: std::path::PathBuf,
        pub(super) activation_state: std::path::PathBuf,
        pub(super) activation_journal: std::path::PathBuf,
        pub(super) activation_journal_temp: std::path::PathBuf,
        pub(super) activation_state_temp: std::path::PathBuf,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct GenerationPointerState {
        pub(super) update_key: ReleasePublicKey,
        pub(super) current: String,
        pub(super) current_key: ReleasePublicKey,
        pub(super) previous: Option<String>,
        pub(super) previous_key: Option<ReleasePublicKey>,
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
                    "generation identity '{field}' is not a safe path component"
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
        WriterBusy,
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
                ActivationTransactionError::WriterBusy => {
                    f.write_str("another activation transaction is in progress; retry later")
                }
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

    #[derive(Debug)]
    pub(super) struct ActivationLockGuard {
        file: Option<std::fs::File>,
    }

    impl Drop for ActivationLockGuard {
        fn drop(&mut self) {
            if let Some(file) = self.file.as_ref() {
                let _ = unsafe { flock(file.as_raw_fd(), FLOCK_UN) };
            }
        }
    }

    pub(super) fn acquire_activation_lock(
        paths: &CoreStatePaths,
    ) -> Result<ActivationLockGuard, ActivationTransactionError> {
        let metadata = match std::fs::symlink_metadata(&paths.root) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ActivationLockGuard { file: None })
            }
            Err(source) => return Err(io_error("inspect Core state root for writer lock", source)),
        };
        if !metadata.file_type().is_dir() {
            return Err(ActivationTransactionError::UnsafeFileType(
                "Core state root",
            ));
        }
        let file = std::fs::File::open(&paths.root)
            .map_err(|source| io_error("open Core state root for writer lock", source))?;
        let result = unsafe { flock(file.as_raw_fd(), FLOCK_EX | FLOCK_NB) };
        if result == 0 {
            return Ok(ActivationLockGuard { file: Some(file) });
        }
        let source = std::io::Error::last_os_error();
        if source.kind() == std::io::ErrorKind::WouldBlock {
            return Err(ActivationTransactionError::WriterBusy);
        }
        Err(io_error("acquire Core state writer lock", source))
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
        let directory = std::fs::File::open(&paths.root)
            .map_err(|err| io_error("open Core state root for sync", err))?;
        directory
            .sync_all()
            .map_err(|err| io_error("sync Core state root", err))?;
        Ok(())
    }

    pub(super) fn validate_generation_identity(
        value: &str,
        field: &'static str,
    ) -> Result<(), StateFormatError> {
        if value.is_empty() {
            return Err(StateFormatError::EmptyIdentity(field));
        }
        if value.len() > GENERATION_ID_MAX_BYTES {
            return Err(StateFormatError::IdentityTooLong(field));
        }
        if value == "."
            || value == ".."
            || value
                .as_bytes()
                .iter()
                .any(|byte| *byte == b'/' || byte.is_ascii_control())
        {
            return Err(StateFormatError::IdentityControl(field));
        }
        Ok(())
    }

    fn validate_pointer_state(state: &GenerationPointerState) -> Result<(), StateFormatError> {
        validate_generation_identity(&state.current, "current")?;
        if state.previous.is_some() != state.previous_key.is_some() {
            return Err(StateFormatError::InconsistentAbsent(
                "previous verifier key",
            ));
        }
        if let Some(previous) = state.previous.as_deref() {
            validate_generation_identity(previous, "previous")?;
            if previous == state.current {
                return Err(StateFormatError::NoChange);
            }
        }
        Ok(())
    }

    pub(super) fn plan_initial_pointer_state_with_key(
        complete_candidate_identity: &str,
        candidate_key: ReleasePublicKey,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        Ok(GenerationPointerState {
            update_key: candidate_key,
            current: complete_candidate_identity.to_owned(),
            current_key: candidate_key,
            previous: None,
            previous_key: None,
        })
    }

    #[cfg(test)]
    pub(super) fn plan_initial_pointer_state(
        complete_candidate_identity: &str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        plan_initial_pointer_state_with_key(
            complete_candidate_identity,
            ReleasePublicKey([0x11; 32]),
        )
    }

    pub(super) fn plan_activation_pointer_state_with_key(
        before: &GenerationPointerState,
        complete_candidate_identity: &str,
        candidate_key: ReleasePublicKey,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        if before.current == complete_candidate_identity {
            return Err(StateFormatError::NoChange);
        }
        Ok(GenerationPointerState {
            update_key: candidate_key,
            current: complete_candidate_identity.to_owned(),
            current_key: candidate_key,
            previous: Some(before.current.clone()),
            previous_key: Some(before.current_key),
        })
    }

    pub(super) fn plan_local_activation_pointer_state_with_key(
        before: &GenerationPointerState,
        complete_candidate_identity: &str,
        candidate_key: ReleasePublicKey,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        if before.current == complete_candidate_identity {
            return Err(StateFormatError::NoChange);
        }
        Ok(GenerationPointerState {
            update_key: before.update_key,
            current: complete_candidate_identity.to_owned(),
            current_key: candidate_key,
            previous: Some(before.current.clone()),
            previous_key: Some(before.current_key),
        })
    }

    #[cfg(test)]
    pub(super) fn plan_activation_pointer_state(
        before: &GenerationPointerState,
        complete_candidate_identity: &str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        plan_activation_pointer_state_with_key(
            before,
            complete_candidate_identity,
            before.update_key,
        )
    }

    pub(super) fn plan_rollback_pointer_state(
        before: &GenerationPointerState,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        let previous = before
            .previous
            .as_deref()
            .ok_or(StateFormatError::NoRollbackGeneration)?;
        let previous_key = before
            .previous_key
            .ok_or(StateFormatError::NoRollbackGeneration)?;
        Ok(GenerationPointerState {
            update_key: before.update_key,
            current: previous.to_owned(),
            current_key: previous_key,
            previous: Some(before.current.clone()),
            previous_key: Some(before.current_key),
        })
    }

    pub(super) fn encode_pointer_state(
        state: &GenerationPointerState,
    ) -> Result<Vec<u8>, StateFormatError> {
        validate_pointer_state(state)?;
        let (previous_present, previous, previous_key) =
            match (state.previous.as_deref(), state.previous_key) {
                (Some(previous), Some(previous_key)) => ("1", previous, previous_key.to_hex()),
                (None, None) => ("0", "", String::new()),
                _ => {
                    return Err(StateFormatError::InconsistentAbsent(
                        "previous verifier key",
                    ))
                }
            };
        Ok(format!(
            "format={STATE_FORMAT}\nupdate_key={}\ncurrent={}\ncurrent_key={}\nprevious_present={previous_present}\nprevious={previous}\nprevious_key={previous_key}\n",
            state.update_key.to_hex(),
            state.current,
            state.current_key.to_hex(),
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

    fn parse_key(value: &str, label: &'static str) -> Result<ReleasePublicKey, StateFormatError> {
        ReleasePublicKey::parse_hex(value).ok_or(StateFormatError::InvalidField(label))
    }

    fn parse_pointer_values(
        update_key: &str,
        current: &str,
        current_key: &str,
        previous_present: &str,
        previous: &str,
        previous_key: &str,
        label: &'static str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        let has_previous = parse_presence(previous_present, label)?;
        if !has_previous && (!previous.is_empty() || !previous_key.is_empty()) {
            return Err(StateFormatError::InconsistentAbsent(label));
        }
        if has_previous && (previous.is_empty() || previous_key.is_empty()) {
            return Err(StateFormatError::InconsistentAbsent(label));
        }
        let state = GenerationPointerState {
            update_key: parse_key(update_key, "update key")?,
            current: current.to_owned(),
            current_key: parse_key(current_key, "current verifier key")?,
            previous: has_previous.then(|| previous.to_owned()),
            previous_key: if has_previous {
                Some(parse_key(previous_key, "previous verifier key")?)
            } else {
                None
            },
        };
        validate_pointer_state(&state)?;
        Ok(state)
    }

    pub(super) fn parse_pointer_state(
        bytes: &[u8],
    ) -> Result<GenerationPointerState, StateFormatError> {
        let records = parse_lines(bytes, "activation state", 7)?;
        if records[0] != format!("format={STATE_FORMAT}") {
            return Err(StateFormatError::InvalidField("activation state format"));
        }
        let update_key = parse_field(records[1], "update_key=", "activation state update key")?;
        let current = parse_field(records[2], "current=", "activation state current")?;
        let current_key = parse_field(records[3], "current_key=", "activation state current key")?;
        let previous_present = parse_field(
            records[4],
            "previous_present=",
            "activation state previous presence",
        )?;
        let previous = parse_field(records[5], "previous=", "activation state previous")?;
        let previous_key =
            parse_field(records[6], "previous_key=", "activation state previous key")?;
        parse_pointer_values(
            update_key,
            current,
            current_key,
            previous_present,
            previous,
            previous_key,
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
            before_update_key,
            before_current,
            before_current_key,
            before_previous_present,
            before_previous,
            before_previous_key,
        ) = match journal.before.as_ref() {
            Some(before) => {
                let (previous_present, previous, previous_key) =
                    match (before.previous.as_deref(), before.previous_key) {
                        (Some(previous), Some(previous_key)) => {
                            ("1", previous, previous_key.to_hex())
                        }
                        (None, None) => ("0", "", String::new()),
                        _ => {
                            return Err(StateFormatError::InconsistentAbsent(
                                "journal before previous",
                            ))
                        }
                    };
                (
                    "1",
                    before.update_key.to_hex(),
                    before.current.as_str(),
                    before.current_key.to_hex(),
                    previous_present,
                    previous,
                    previous_key,
                )
            }
            None => (
                "0",
                String::new(),
                "",
                String::new(),
                "0",
                "",
                String::new(),
            ),
        };
        let (after_previous_present, after_previous, after_previous_key) = match (
            journal.after.previous.as_deref(),
            journal.after.previous_key,
        ) {
            (Some(previous), Some(previous_key)) => ("1", previous, previous_key.to_hex()),
            (None, None) => ("0", "", String::new()),
            _ => {
                return Err(StateFormatError::InconsistentAbsent(
                    "journal after previous",
                ))
            }
        };
        Ok(format!(
            "format={JOURNAL_FORMAT}\nbefore_present={before_present}\nbefore_update_key={before_update_key}\nbefore_current={before_current}\nbefore_current_key={before_current_key}\nbefore_previous_present={before_previous_present}\nbefore_previous={before_previous}\nbefore_previous_key={before_previous_key}\nafter_update_key={}\nafter_current={}\nafter_current_key={}\nafter_previous_present={after_previous_present}\nafter_previous={after_previous}\nafter_previous_key={after_previous_key}\n",
            journal.after.update_key.to_hex(),
            journal.after.current,
            journal.after.current_key.to_hex(),
        )
        .into_bytes())
    }

    pub(super) fn parse_activation_journal(
        bytes: &[u8],
    ) -> Result<ActivationJournal, StateFormatError> {
        let records = parse_lines(bytes, "activation journal", 14)?;
        if records[0] != format!("format={JOURNAL_FORMAT}") {
            return Err(StateFormatError::InvalidField("activation journal format"));
        }
        let before_present = parse_presence(
            parse_field(records[1], "before_present=", "journal before presence")?,
            "journal before presence",
        )?;
        let before_update_key = parse_field(
            records[2],
            "before_update_key=",
            "journal before update key",
        )?;
        let before_current = parse_field(records[3], "before_current=", "journal before current")?;
        let before_current_key = parse_field(
            records[4],
            "before_current_key=",
            "journal before current key",
        )?;
        let before_previous_present = parse_field(
            records[5],
            "before_previous_present=",
            "journal before previous presence",
        )?;
        let before_previous =
            parse_field(records[6], "before_previous=", "journal before previous")?;
        let before_previous_key = parse_field(
            records[7],
            "before_previous_key=",
            "journal before previous key",
        )?;
        let before = if before_present {
            Some(parse_pointer_values(
                before_update_key,
                before_current,
                before_current_key,
                before_previous_present,
                before_previous,
                before_previous_key,
                "journal before state",
            )?)
        } else {
            if !before_update_key.is_empty()
                || !before_current.is_empty()
                || !before_current_key.is_empty()
                || before_previous_present != "0"
                || !before_previous.is_empty()
                || !before_previous_key.is_empty()
            {
                return Err(StateFormatError::InconsistentAbsent("journal before state"));
            }
            None
        };

        let after_update_key =
            parse_field(records[8], "after_update_key=", "journal after update key")?;
        let after_current = parse_field(records[9], "after_current=", "journal after current")?;
        let after_current_key = parse_field(
            records[10],
            "after_current_key=",
            "journal after current key",
        )?;
        let after_previous_present = parse_field(
            records[11],
            "after_previous_present=",
            "journal after previous presence",
        )?;
        let after_previous = parse_field(records[12], "after_previous=", "journal after previous")?;
        let after_previous_key = parse_field(
            records[13],
            "after_previous_key=",
            "journal after previous key",
        )?;
        let after = parse_pointer_values(
            after_update_key,
            after_current,
            after_current_key,
            after_previous_present,
            after_previous,
            after_previous_key,
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
        let _lock = acquire_activation_lock(paths)?;
        activate_pointer_state_locked(paths, before, after, io, &_lock)
    }

    pub(super) fn activate_pointer_state_locked<I: ActivationIo>(
        paths: &CoreStatePaths,
        before: Option<&GenerationPointerState>,
        after: &GenerationPointerState,
        io: &mut I,
        _lock: &ActivationLockGuard,
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
        let _lock = acquire_activation_lock(paths)?;
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
const LEGACY_GENERATION_FORMAT: &str = "codex-local-generation-v1";
const LOCAL_GENERATION_FORMAT: &str = "codex-local-generation-v2";
const CODE_MODE_HOST_FILE: &str = "codex-code-mode-host";
#[cfg(unix)]
const LOCAL_GENERATION_MAX_BYTES: usize = 64 * 1024;
#[cfg(unix)]
const CORE_API_IDENTITY: &str = "core-api-v1";
#[cfg(unix)]
const PERSISTENT_SCHEMA_IDENTITY: &str = "schema-v1";
#[cfg(unix)]
const LOCAL_RELEASE_FORMAT: &str = "codex-release-v3";
#[cfg(unix)]
const LOCAL_RELEASE_FORMAT_V4: &str = "codex-release-v4";
#[cfg(unix)]
const LOCAL_RELEASE_CHANNEL: &str = "stable";
#[cfg(unix)]
const LOCAL_RELEASE_MAX_BYTES: usize = 128 * 1024;
#[cfg(unix)]
const LOCAL_RELEASE_MAX_FILES: usize = 4096;
#[cfg(unix)]
const LOCAL_RELEASE_SIGNATURE_MAX_BYTES: u64 = 1024;
#[cfg(unix)]
const DOCTOR_OUTPUT_MAX_BYTES: usize = 64 * 1024;
#[cfg(unix)]
const UPDATE_INDEX_MAX_BYTES: usize = 16 * 1024;
#[cfg(unix)]
const DEFAULT_UPDATE_INDEX_URL: &str =
    "https://raw.githubusercontent.com/humtr/codex/main/update-index-v1";
#[cfg(unix)]
const UPDATE_INDEX_URL_ENV: &str = "CODEX_TERMUX_UPDATE_INDEX_URL";
#[cfg(unix)]
const UPDATE_VERSION_ENV: &str = "CODEX_TERMUX_UPDATE_VERSION";
#[cfg(test)]
const UPDATE_PRIVATE_KEY_ENV: &str = "CODEX_TERMUX_UPDATE_PRIVATE_KEY";
#[cfg(unix)]
const DEFAULT_UPSTREAM_LATEST_URL: &str = "https://releases.openai.com/codex/channels/latest";
#[cfg(unix)]
const UPSTREAM_RELEASE_METADATA_BASE: &str = "https://releases.openai.com/codex/releases";
#[cfg(unix)]
const UPSTREAM_PACKAGE_ASSET: &str = "codex-package-aarch64-unknown-linux-musl.tar.gz";
#[cfg(unix)]
const UPSTREAM_RELEASE_METADATA_MAX_BYTES: usize = 128 * 1024;
#[cfg(unix)]
const UPDATE_PRIVATE_KEY_MAX_BYTES: u64 = 16 * 1024;
#[cfg(test)]
const LOCAL_PUBLICATION_ROOT_RELATIVE: &str = ".local/lib/codex/core/publications";
#[cfg(unix)]
const LOCAL_DERIVED_METADATA_FORMAT: &str = "codex-local-derived-v1";
#[cfg(unix)]
const LOCAL_DERIVED_PRIVATE_KEY_FILE: &str = "local-derived-private.pem";
#[cfg(unix)]
const BOOTSTRAP_PUBLIC_KEY_MAX_BYTES: u64 = 16 * 1024;
#[cfg(unix)]
const CORE_ARTIFACT_MAX_BYTES: u64 = 64 * 1024 * 1024;
#[cfg(unix)]
const CORE_ROLLBACK_FORMAT: &str = "codex-core-entrypoint-rollback-v1";
#[cfg(unix)]
const CORE_ROLLBACK_FILE: &str = "core-entrypoint-rollback";
#[cfg(unix)]
const CORE_ROLLBACK_META_FILE: &str = "core-entrypoint-rollback.meta";
#[cfg(unix)]
const CORE_ROLLBACK_META_MAX_BYTES: usize = 4096;
#[cfg(unix)]
const REMOTE_RELEASE_URL_MAX_BYTES: usize = 4096;
#[cfg(unix)]
const REMOTE_RELEASE_FILE_MAX_BYTES: u64 = 512 * 1024 * 1024;
#[cfg(unix)]
const REMOTE_RELEASE_TOTAL_MAX_BYTES: u64 = 1024 * 1024 * 1024;
#[cfg(unix)]
const REMOTE_CONNECT_TIMEOUT_SECONDS: &str = "15";
#[cfg(unix)]
const REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS: &str = "30";
#[cfg(unix)]
const REMOTE_TRANSFER_TIMEOUT_SECONDS: &str = "300";
#[cfg(unix)]
const INTERNAL_BOOTSTRAP_MODE_ENV: &str = "CODEX_TERMUX_INTERNAL_BOOTSTRAP";
#[cfg(unix)]
const INTERNAL_BOOTSTRAP_SOURCE_ENV: &str = "CODEX_TERMUX_INTERNAL_BOOTSTRAP_SOURCE";
#[cfg(unix)]
const INTERNAL_BOOTSTRAP_KEY_ENV: &str = "CODEX_TERMUX_INTERNAL_BOOTSTRAP_KEY";
#[cfg(unix)]
const INTERNAL_BOOTSTRAP_CORE_ENV: &str = "CODEX_TERMUX_INTERNAL_BOOTSTRAP_CORE";
#[cfg(unix)]
const INTERNAL_BOOTSTRAP_EXPECTED_ENTRYPOINT_DIGEST_ENV: &str =
    "CODEX_TERMUX_INTERNAL_BOOTSTRAP_EXPECTED_LEGACY_ENTRYPOINT_SHA256";

#[cfg(unix)]
#[derive(Debug, Clone)]
struct LocalCoreRoots {
    generation_root: std::path::PathBuf,
    state_root: std::path::PathBuf,
    config_dir: std::path::PathBuf,
    resolver_path: std::path::PathBuf,
    cert_file: std::path::PathBuf,
    cert_dir: std::path::PathBuf,
    openssl: std::path::PathBuf,
    curl: std::path::PathBuf,
}

#[cfg(unix)]
impl LocalCoreRoots {
    fn from_environment() -> Result<Self, LocalProductError> {
        let home = required_absolute_env_path("HOME")?;
        let prefix = required_absolute_env_path("PREFIX")?;
        let state_root = home.join(".local/share/codex/core");
        Ok(Self {
            generation_root: home.join(".local/lib/codex/core/generations"),
            config_dir: state_root.join("config"),
            state_root,
            resolver_path: prefix.join("etc/resolv.conf"),
            cert_file: prefix.join("etc/tls/cert.pem"),
            cert_dir: prefix.join("etc/tls/certs"),
            openssl: prefix.join("bin/openssl"),
            curl: prefix.join("bin/curl"),
        })
    }
}

#[cfg(unix)]
#[derive(Debug)]
enum LocalProductError {
    MissingEnvironment(&'static str),
    InvalidEnvironmentPath(&'static str),
    StateFormat(m2_generation_state::StateFormatError),
    State(m2_generation_state::ActivationTransactionError),
    NoCurrentGeneration,
    Io {
        operation: &'static str,
        source: std::io::Error,
    },
    Descriptor(&'static str),
    UnsafeSource(&'static str),
    GenerationCollision,
    Release(&'static str),
    Bootstrap(&'static str),
    LegacyHandoff(&'static str),
    CoreEntrypoint(&'static str),
    OpenSslUnavailable,
    CurlUnavailable,
    TrustedReleaseKeyUnavailable,
    OpenSslFailed(&'static str),
    Remote(&'static str),
    UpdateIndex(&'static str),
    RemoteTransportFailed,
    RemoteResponseTooLarge,
    LocalUpdate(&'static str),
    SignatureRejected,
    ReleasePolicy(&'static str),
    ReleaseDigestMismatch,
    ReleaseModeMismatch,
    ReleaseSequenceRollback,
    UpdateHold(&'static str),
    UpdateHeld {
        generation_id: Box<str>,
        release_sequence: u64,
    },
    RollbackHoldAfterCommit(Box<LocalProductError>),
    CandidateProbe(&'static str),
    Manifest(GenerationManifestError),
    Runtime(RuntimeAssetError),
    Manager(ManagerArtifactError),
    Dispatch(PublicDispatchExecutionError),
}

#[cfg(unix)]
impl std::fmt::Display for LocalProductError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalProductError::MissingEnvironment(name) => {
                write!(f, "required environment variable {name} is missing")
            }
            LocalProductError::InvalidEnvironmentPath(name) => {
                write!(
                    f,
                    "environment path {name} must be a non-empty absolute path"
                )
            }
            LocalProductError::StateFormat(err) => err.fmt(f),
            LocalProductError::State(err) => err.fmt(f),
            LocalProductError::NoCurrentGeneration => {
                f.write_str("no activated Codex generation is available")
            }
            LocalProductError::Io { operation, source } => {
                write!(f, "{operation} failed: {source}")
            }
            LocalProductError::Descriptor(message) => f.write_str(message),
            LocalProductError::UnsafeSource(message) => f.write_str(message),
            LocalProductError::GenerationCollision => {
                f.write_str("generation id is already present in the immutable generation root")
            }
            LocalProductError::Release(message) => f.write_str(message),
            LocalProductError::Bootstrap(message) => f.write_str(message),
            LocalProductError::LegacyHandoff(message) => f.write_str(message),
            LocalProductError::CoreEntrypoint(message) => f.write_str(message),
            LocalProductError::OpenSslUnavailable => f.write_str("Termux OpenSSL is unavailable"),
            LocalProductError::CurlUnavailable => f.write_str("Termux curl is unavailable"),
            LocalProductError::TrustedReleaseKeyUnavailable => {
                f.write_str("trusted release public key is unavailable")
            }
            LocalProductError::OpenSslFailed(operation) => {
                write!(f, "OpenSSL {operation} failed")
            }
            LocalProductError::Remote(message) => f.write_str(message),
            LocalProductError::UpdateIndex(message) => f.write_str(message),
            LocalProductError::RemoteTransportFailed => {
                f.write_str("remote release transport failed")
            }
            LocalProductError::RemoteResponseTooLarge => {
                f.write_str("remote release response exceeds its byte bound")
            }
            LocalProductError::LocalUpdate(message) => f.write_str(message),
            LocalProductError::SignatureRejected => {
                f.write_str("release signature verification failed")
            }
            LocalProductError::ReleasePolicy(message) => f.write_str(message),
            LocalProductError::ReleaseDigestMismatch => {
                f.write_str("release file inventory digest mismatch")
            }
            LocalProductError::ReleaseModeMismatch => {
                f.write_str("release file inventory mode mismatch")
            }
            LocalProductError::ReleaseSequenceRollback => {
                f.write_str("release sequence is not newer than the active release")
            }
            LocalProductError::UpdateHold(message) => f.write_str(message),
            LocalProductError::UpdateHeld {
                generation_id,
                release_sequence,
            } => write!(
                f,
                "release sequence {release_sequence} is locally held after rollback (generation {generation_id}); use 'codex update --force' to retry"
            ),
            LocalProductError::RollbackHoldAfterCommit(source) => {
                write!(f, "rollback completed but update hold could not be recorded: {source}")
            }
            LocalProductError::CandidateProbe(message) => f.write_str(message),
            LocalProductError::Manifest(err) => err.fmt(f),
            LocalProductError::Runtime(err) => err.fmt(f),
            LocalProductError::Manager(err) => err.fmt(f),
            LocalProductError::Dispatch(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for LocalProductError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocalProductError::StateFormat(err) => Some(err),
            LocalProductError::State(err) => Some(err),
            LocalProductError::Io { source, .. } => Some(source),
            LocalProductError::Manifest(err) => Some(err),
            LocalProductError::Runtime(err) => Some(err),
            LocalProductError::Manager(err) => Some(err),
            LocalProductError::Dispatch(err) => Some(err),
            LocalProductError::RollbackHoldAfterCommit(source) => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[cfg(unix)]
fn required_absolute_env_path(name: &'static str) -> Result<std::path::PathBuf, LocalProductError> {
    let value = std::env::var_os(name).ok_or(LocalProductError::MissingEnvironment(name))?;
    let path = std::path::PathBuf::from(value);
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(LocalProductError::InvalidEnvironmentPath(name));
    }
    Ok(path)
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteReleaseBase {
    value: String,
}

#[cfg(unix)]
fn valid_remote_port(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes().iter().all(u8::is_ascii_digit)
        && value.parse::<u16>().is_ok_and(|port| port != 0)
}

#[cfg(unix)]
fn valid_remote_authority(value: &str) -> bool {
    if value.is_empty() || value.contains('@') {
        return false;
    }
    if let Some(ipv6) = value.strip_prefix('[') {
        let Some((address, suffix)) = ipv6.split_once(']') else {
            return false;
        };
        if address.parse::<std::net::Ipv6Addr>().is_err() {
            return false;
        }
        return suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_remote_port);
    }

    let mut authority = value.split(':');
    let host = authority.next().unwrap_or_default();
    let port = authority.next();
    if authority.next().is_some() || port.is_some_and(|port| !valid_remote_port(port)) {
        return false;
    }
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

#[cfg(unix)]
fn remote_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

#[cfg(unix)]
fn uppercase_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(unix)]
fn valid_canonical_remote_path_component(value: &str) -> bool {
    if value.is_empty() || matches!(value, "." | "..") {
        return false;
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if remote_unreserved(bytes[index]) {
            index += 1;
            continue;
        }
        if bytes[index] != b'%' || index + 2 >= bytes.len() {
            return false;
        }
        let Some(high) = uppercase_hex_value(bytes[index + 1]) else {
            return false;
        };
        let Some(low) = uppercase_hex_value(bytes[index + 2]) else {
            return false;
        };
        let decoded = (high << 4) | low;
        if remote_unreserved(decoded)
            || decoded.is_ascii_control()
            || matches!(decoded, b'/' | b'\\')
        {
            return false;
        }
        index += 3;
    }
    true
}

#[cfg(unix)]
fn percent_encode_remote_path(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte == b'/' || remote_unreserved(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

#[cfg(unix)]
impl RemoteReleaseBase {
    fn parse(value: &OsStr) -> Result<Self, LocalProductError> {
        let value = value.to_str().ok_or(LocalProductError::Remote(
            "remote release base URL is not UTF-8",
        ))?;
        if value.is_empty()
            || value.len() > REMOTE_RELEASE_URL_MAX_BYTES
            || !value.is_ascii()
            || value
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
            || value.contains(['?', '#', '\\'])
            || !value.ends_with('/')
        {
            return Err(LocalProductError::Remote(
                "remote release base URL is not canonical",
            ));
        }
        let remainder = value
            .strip_prefix("https://")
            .ok_or(LocalProductError::Remote(
                "remote release base URL must use HTTPS",
            ))?;
        let (authority, path) = remainder.split_once('/').ok_or(LocalProductError::Remote(
            "remote release base URL has no generation path",
        ))?;
        let path = path.strip_suffix('/').unwrap_or_default();
        if !valid_remote_authority(authority)
            || path.is_empty()
            || !path.split('/').all(valid_canonical_remote_path_component)
        {
            return Err(LocalProductError::Remote(
                "remote release base URL is not canonical",
            ));
        }
        Ok(Self {
            value: value.to_owned(),
        })
    }

    fn resource_url(&self, relative_path: &str) -> Result<String, LocalProductError> {
        if !matches!(
            relative_path,
            "release.manifest" | "release.sig" | "release-authority.sig"
        ) && !valid_release_relative_path(relative_path)
        {
            return Err(LocalProductError::Remote(
                "remote release resource path is invalid",
            ));
        }
        let url = format!(
            "{}{}",
            self.value,
            percent_encode_remote_path(relative_path)
        );
        if url.len() > REMOTE_RELEASE_URL_MAX_BYTES {
            return Err(LocalProductError::Remote(
                "remote release resource URL is too large",
            ));
        }
        Ok(url)
    }

    fn matches_generation_identity(&self, generation_id: &str) -> Result<bool, LocalProductError> {
        m2_generation_state::validate_generation_identity(generation_id, "remote generation_id")
            .map_err(LocalProductError::StateFormat)?;
        let expected = percent_encode_remote_path(generation_id);
        Ok(self
            .value
            .strip_suffix('/')
            .and_then(|value| value.rsplit('/').next())
            == Some(expected.as_str()))
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SignedUpdateIndex {
    generation_id: String,
    release_base: RemoteReleaseBase,
}

#[cfg(unix)]
fn parse_update_index_url(value: &OsStr) -> Result<String, LocalProductError> {
    let value = value.to_str().ok_or(LocalProductError::UpdateIndex(
        "update index URL is not UTF-8",
    ))?;
    if value.is_empty()
        || value.len() > REMOTE_RELEASE_URL_MAX_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        || value.contains(['?', '#', '\\'])
    {
        return Err(LocalProductError::UpdateIndex(
            "update index URL is not canonical",
        ));
    }
    let remainder = value
        .strip_prefix("https://")
        .ok_or(LocalProductError::UpdateIndex(
            "update index URL must use HTTPS",
        ))?;
    let (authority, path) = remainder
        .split_once('/')
        .ok_or(LocalProductError::UpdateIndex(
            "update index URL has no resource path",
        ))?;
    if !valid_remote_authority(authority)
        || path.is_empty()
        || path.ends_with('/')
        || !path.split('/').all(valid_canonical_remote_path_component)
    {
        return Err(LocalProductError::UpdateIndex(
            "update index URL is not canonical",
        ));
    }
    Ok(value.to_owned())
}

#[cfg(unix)]
fn configured_update_index_url() -> Result<String, LocalProductError> {
    let value = std::env::var_os(UPDATE_INDEX_URL_ENV)
        .unwrap_or_else(|| OsString::from(DEFAULT_UPDATE_INDEX_URL));
    parse_update_index_url(&value)
}

#[cfg(unix)]
fn update_index_field<'a>(
    line: Option<&'a str>,
    expected: &'static str,
) -> Result<&'a str, LocalProductError> {
    let line = line.ok_or(LocalProductError::UpdateIndex("update index is incomplete"))?;
    let mut fields = line.split('\t');
    if fields.next() != Some(expected) {
        return Err(LocalProductError::UpdateIndex(
            "update index field is invalid",
        ));
    }
    let value =
        fields
            .next()
            .filter(|value| !value.is_empty())
            .ok_or(LocalProductError::UpdateIndex(
                "update index field is invalid",
            ))?;
    if fields.next().is_some() || value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(LocalProductError::UpdateIndex(
            "update index field is invalid",
        ));
    }
    Ok(value)
}

#[cfg(unix)]
fn parse_signed_update_index(bytes: &[u8]) -> Result<SignedUpdateIndex, LocalProductError> {
    if !bytes.ends_with(b"\n") {
        return Err(LocalProductError::UpdateIndex(
            "update index is missing its final newline",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LocalProductError::UpdateIndex("update index is not UTF-8"))?;
    let mut lines = text.lines();
    if lines.next() != Some("codex-update-index-v1") {
        return Err(LocalProductError::UpdateIndex(
            "update index format is unsupported",
        ));
    }
    if update_index_field(lines.next(), "channel")? != LOCAL_RELEASE_CHANNEL {
        return Err(LocalProductError::UpdateIndex(
            "update index channel is not supported",
        ));
    }
    let generation_id = update_index_field(lines.next(), "generation_id")?;
    m2_generation_state::validate_generation_identity(generation_id, "update index generation_id")
        .map_err(LocalProductError::StateFormat)?;
    let release_base = update_index_field(lines.next(), "release_base")?;
    if lines.next().is_some() {
        return Err(LocalProductError::UpdateIndex(
            "update index has unexpected trailing fields",
        ));
    }
    let release_base = RemoteReleaseBase::parse(OsStr::new(release_base))?;
    if !release_base.matches_generation_identity(generation_id)? {
        return Err(LocalProductError::UpdateIndex(
            "update index release base does not match generation identity",
        ));
    }
    Ok(SignedUpdateIndex {
        generation_id: generation_id.to_owned(),
        release_base,
    })
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct OfficialReleaseMetadata {
    version: String,
    package_sha256: String,
}

#[cfg(unix)]
fn valid_update_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return false;
    }
    let mut components = value.split('.');
    let mut count = 0;
    for component in components.by_ref() {
        count += 1;
        if component.is_empty()
            || !component.bytes().all(|byte| byte.is_ascii_digit())
            || (component.len() > 1 && component.starts_with('0'))
            || component.parse::<u64>().is_err()
        {
            return false;
        }
    }
    count == 3
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicReleaseBaseline {
    generation_component: String,
    release_sequence: u64,
}

#[cfg(unix)]
fn render_local_derived_metadata(
    metadata: &OfficialReleaseMetadata,
    baseline: &PublicReleaseBaseline,
) -> String {
    format!(
        "{LOCAL_DERIVED_METADATA_FORMAT};upstream_version={};archive_sha256={};public_generation={};public_sequence={}",
        metadata.version,
        metadata.package_sha256,
        baseline.generation_component,
        baseline.release_sequence,
    )
}

#[cfg(unix)]
fn parse_local_derived_metadata(
    release: &LocalReleaseManifest,
    loaded: &LoadedLocalGeneration,
) -> Result<Option<PublicReleaseBaseline>, LocalProductError> {
    let mut fields = loaded.manifest.creation_metadata.split(';');
    if fields.next() != Some(LOCAL_DERIVED_METADATA_FORMAT) {
        return Ok(None);
    }
    let upstream_version = fields
        .next()
        .and_then(|value| value.strip_prefix("upstream_version="))
        .ok_or(LocalProductError::LocalUpdate(
            "local-derived provenance is malformed",
        ))?;
    let archive_sha256 = fields
        .next()
        .and_then(|value| value.strip_prefix("archive_sha256="))
        .ok_or(LocalProductError::LocalUpdate(
            "local-derived provenance is malformed",
        ))?;
    let public_generation = fields
        .next()
        .and_then(|value| value.strip_prefix("public_generation="))
        .ok_or(LocalProductError::LocalUpdate(
            "local-derived provenance is malformed",
        ))?;
    let public_sequence_text = fields
        .next()
        .and_then(|value| value.strip_prefix("public_sequence="))
        .ok_or(LocalProductError::LocalUpdate(
            "local-derived provenance is malformed",
        ))?;
    if fields.next().is_some()
        || !valid_update_version(upstream_version)
        || upstream_version != loaded.manifest.upstream_package_version
        || !valid_sha256_hex(archive_sha256)
        || archive_sha256 != loaded.manifest.source_artifact_digest
        || !valid_canonical_remote_path_component(public_generation)
    {
        return Err(LocalProductError::LocalUpdate(
            "local-derived provenance does not match the signed generation",
        ));
    }
    let public_sequence = public_sequence_text.parse::<u64>().map_err(|_| {
        LocalProductError::LocalUpdate("local-derived public release sequence is invalid")
    })?;
    if public_sequence == 0
        || public_sequence.to_string() != public_sequence_text
        || release.release_sequence != public_sequence
    {
        return Err(LocalProductError::LocalUpdate(
            "local-derived public release sequence is invalid",
        ));
    }
    Ok(Some(PublicReleaseBaseline {
        generation_component: public_generation.to_owned(),
        release_sequence: public_sequence,
    }))
}

#[cfg(unix)]
fn authenticated_public_baseline(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
) -> Result<
    (
        PublicReleaseBaseline,
        LocalReleaseManifest,
        LoadedLocalGeneration,
    ),
    LocalProductError,
> {
    let (release, loaded) = verify_installed_local_release(
        roots,
        &state.current,
        state.current_key,
        "active generation descriptor id does not match current",
    )?;
    let baseline = match parse_local_derived_metadata(&release, &loaded)? {
        Some(baseline) => baseline,
        None => PublicReleaseBaseline {
            generation_component: percent_encode_remote_path(&state.current),
            release_sequence: release.release_sequence,
        },
    };
    Ok((baseline, release, loaded))
}

#[cfg(unix)]
fn generate_ephemeral_local_derived_key(
    openssl: &std::path::Path,
    staging_root: &std::path::Path,
) -> Result<(std::path::PathBuf, ReleasePublicKey), LocalProductError> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    let private_key = staging_root.join(LOCAL_DERIVED_PRIVATE_KEY_FILE);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&private_key)
        .map_err(|source| LocalProductError::Io {
            operation: "create ephemeral local-derived signing key",
            source,
        })?;
    drop(file);
    let status = std::process::Command::new(openssl)
        .args(["genpkey", "-algorithm", "ED25519", "-out"])
        .arg(&private_key)
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|_| LocalProductError::OpenSslUnavailable)?;
    if !status.success() {
        return Err(LocalProductError::OpenSslFailed(
            "ephemeral local-derived signing key generation",
        ));
    }
    let metadata =
        std::fs::symlink_metadata(&private_key).map_err(|source| LocalProductError::Io {
            operation: "inspect ephemeral local-derived signing key",
            source,
        })?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > UPDATE_PRIVATE_KEY_MAX_BYTES
        || metadata.permissions().mode() & 0o7777 != 0o600
    {
        return Err(LocalProductError::LocalUpdate(
            "ephemeral local-derived signing key is unsafe",
        ));
    }
    let key = release_public_key_from_private_pem(openssl, &private_key)?;
    Ok((private_key, key))
}

#[cfg(unix)]
struct JsonCursor<'a> {
    bytes: &'a [u8],
    index: usize,
}

#[cfg(unix)]
impl<'a> JsonCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn error<T>(&self) -> Result<T, LocalProductError> {
        Err(LocalProductError::LocalUpdate(
            "upstream release metadata is malformed",
        ))
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.index += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), LocalProductError> {
        self.skip_whitespace();
        if self.bytes.get(self.index) != Some(&expected) {
            return self.error();
        }
        self.index += 1;
        Ok(())
    }

    fn string(&mut self) -> Result<String, LocalProductError> {
        self.skip_whitespace();
        if self.bytes.get(self.index) != Some(&b'"') {
            return self.error();
        }
        self.index += 1;
        let mut value = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.index) else {
                return self.error();
            };
            match byte {
                b'"' => {
                    self.index += 1;
                    return String::from_utf8(value).map_err(|_| {
                        LocalProductError::LocalUpdate("upstream release metadata is malformed")
                    });
                }
                b'\\' => {
                    self.index += 1;
                    let Some(&escape) = self.bytes.get(self.index) else {
                        return self.error();
                    };
                    self.index += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => value.push(escape),
                        b'b' => value.push(0x08),
                        b'f' => value.push(0x0c),
                        b'n' => value.push(b'\n'),
                        b'r' => value.push(b'\r'),
                        b't' => value.push(b'\t'),
                        b'u' => {
                            let Some(codepoint) = self.hex_codepoint() else {
                                return self.error();
                            };
                            let Some(character) = char::from_u32(codepoint) else {
                                return self.error();
                            };
                            let mut encoded = [0; 4];
                            value.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                        }
                        _ => return self.error(),
                    }
                }
                0..=0x1f => return self.error(),
                _ => {
                    let start = self.index;
                    while let Some(&next) = self.bytes.get(self.index) {
                        if matches!(next, b'"' | b'\\' | 0..=0x1f) {
                            break;
                        }
                        self.index += 1;
                    }
                    let chunk =
                        std::str::from_utf8(&self.bytes[start..self.index]).map_err(|_| {
                            LocalProductError::LocalUpdate("upstream release metadata is malformed")
                        })?;
                    value.extend_from_slice(chunk.as_bytes());
                }
            }
        }
    }

    fn hex_codepoint(&mut self) -> Option<u32> {
        let end = self.index.checked_add(4)?;
        let bytes = self.bytes.get(self.index..end)?;
        let mut value: u32 = 0;
        for byte in bytes {
            value = value.checked_mul(16)?;
            value += match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a' + 10),
                b'A'..=b'F' => u32::from(byte - b'A' + 10),
                _ => return None,
            };
        }
        self.index = end;
        Some(value)
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), LocalProductError> {
        self.skip_whitespace();
        let end = self.index.saturating_add(literal.len());
        if self.bytes.get(self.index..end) != Some(literal) {
            return self.error();
        }
        self.index = end;
        Ok(())
    }

    fn skip_number(&mut self) -> Result<(), LocalProductError> {
        self.skip_whitespace();
        let mut index = self.index;
        if self.bytes.get(index) == Some(&b'-') {
            index += 1;
        }
        match self.bytes.get(index) {
            Some(b'0') => {
                index += 1;
                if self
                    .bytes
                    .get(index)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    return self.error();
                }
            }
            Some(b'1'..=b'9') => {
                index += 1;
                while self
                    .bytes
                    .get(index)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    index += 1;
                }
            }
            _ => return self.error(),
        }
        if self.bytes.get(index) == Some(&b'.') {
            index += 1;
            let start = index;
            while self
                .bytes
                .get(index)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                index += 1;
            }
            if start == index {
                return self.error();
            }
        }
        if self
            .bytes
            .get(index)
            .is_some_and(|byte| matches!(*byte, b'e' | b'E'))
        {
            index += 1;
            if self
                .bytes
                .get(index)
                .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
            {
                index += 1;
            }
            let start = index;
            while self
                .bytes
                .get(index)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                index += 1;
            }
            if start == index {
                return self.error();
            }
        }
        if self
            .bytes
            .get(index)
            .is_some_and(|byte| !matches!(*byte, b' ' | b'\t' | b'\r' | b'\n' | b',' | b']' | b'}'))
        {
            return self.error();
        }
        self.index = index;
        Ok(())
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), LocalProductError> {
        if depth > 32 {
            return self.error();
        }
        self.skip_whitespace();
        match self.bytes.get(self.index) {
            Some(b'"') => self.string().map(|_| ()),
            Some(b'{') => {
                self.index += 1;
                self.skip_whitespace();
                if self.bytes.get(self.index) == Some(&b'}') {
                    self.index += 1;
                    return Ok(());
                }
                loop {
                    self.string()?;
                    self.expect(b':')?;
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.index) {
                        Some(b',') => self.index += 1,
                        Some(b'}') => {
                            self.index += 1;
                            return Ok(());
                        }
                        _ => return self.error(),
                    }
                }
            }
            Some(b'[') => {
                self.index += 1;
                self.skip_whitespace();
                if self.bytes.get(self.index) == Some(&b']') {
                    self.index += 1;
                    return Ok(());
                }
                loop {
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.index) {
                        Some(b',') => self.index += 1,
                        Some(b']') => {
                            self.index += 1;
                            return Ok(());
                        }
                        _ => return self.error(),
                    }
                }
            }
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.skip_number(),
            _ => self.error(),
        }
    }

    fn parse_assets(&mut self, wanted_asset: &str) -> Result<Option<String>, LocalProductError> {
        self.expect(b'[')?;
        self.skip_whitespace();
        if self.bytes.get(self.index) == Some(&b']') {
            self.index += 1;
            return Ok(None);
        }
        let mut wanted_digest = None;
        loop {
            self.expect(b'{')?;
            let mut name = None;
            let mut digest = None;
            self.skip_whitespace();
            if self.bytes.get(self.index) == Some(&b'}') {
                self.index += 1;
            } else {
                loop {
                    let key = self.string()?;
                    self.expect(b':')?;
                    match key.as_str() {
                        "name" => {
                            if name.is_some() {
                                return self.error();
                            }
                            name = Some(self.string()?);
                        }
                        "digest" => {
                            if digest.is_some() {
                                return self.error();
                            }
                            digest = Some(self.string()?);
                        }
                        _ => self.skip_value(0)?,
                    }
                    self.skip_whitespace();
                    match self.bytes.get(self.index) {
                        Some(b',') => self.index += 1,
                        Some(b'}') => {
                            self.index += 1;
                            break;
                        }
                        _ => return self.error(),
                    }
                }
            }
            if name.as_deref() == Some(wanted_asset) {
                if wanted_digest.is_some() {
                    return self.error();
                }
                let digest = digest.ok_or(LocalProductError::LocalUpdate(
                    "upstream release metadata is missing the package digest",
                ))?;
                wanted_digest = Some(digest);
            }
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    return Ok(wanted_digest);
                }
                _ => return self.error(),
            }
        }
    }
}

#[cfg(unix)]
fn parse_official_release_metadata(
    bytes: &[u8],
    requested_version: Option<&str>,
) -> Result<OfficialReleaseMetadata, LocalProductError> {
    let mut cursor = JsonCursor::new(bytes);
    cursor.expect(b'{')?;
    let mut tag_name = None;
    let mut package_digest = None;
    let mut assets_seen = false;
    cursor.skip_whitespace();
    if cursor.bytes.get(cursor.index) == Some(&b'}') {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata is incomplete",
        ));
    }
    loop {
        let key = cursor.string()?;
        cursor.expect(b':')?;
        match key.as_str() {
            "tag_name" => {
                if tag_name.is_some() {
                    return cursor.error();
                }
                tag_name = Some(cursor.string()?);
            }
            "assets" => {
                if assets_seen {
                    return cursor.error();
                }
                assets_seen = true;
                package_digest = cursor.parse_assets(UPSTREAM_PACKAGE_ASSET)?;
            }
            _ => cursor.skip_value(0)?,
        }
        cursor.skip_whitespace();
        match cursor.bytes.get(cursor.index) {
            Some(b',') => cursor.index += 1,
            Some(b'}') => {
                cursor.index += 1;
                break;
            }
            _ => return cursor.error(),
        }
    }
    cursor.skip_whitespace();
    if cursor.index != cursor.bytes.len() {
        return cursor.error();
    }
    let tag_name = tag_name.ok_or(LocalProductError::LocalUpdate(
        "upstream release metadata has no release tag",
    ))?;
    let version = tag_name
        .strip_prefix("rust-v")
        .ok_or(LocalProductError::LocalUpdate(
            "upstream release metadata has an unsupported tag",
        ))?;
    if !valid_update_version(version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata has a non-stable version",
        ));
    }
    if requested_version.is_some_and(|requested| requested != version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream release metadata version does not match the requested version",
        ));
    }
    let package_sha256 = package_digest
        .ok_or(LocalProductError::LocalUpdate(
            "upstream release metadata has no target package",
        ))?
        .strip_prefix("sha256:")
        .filter(|digest| valid_sha256_hex(digest))
        .ok_or(LocalProductError::LocalUpdate(
            "upstream release metadata package digest is invalid",
        ))?
        .to_owned();
    Ok(OfficialReleaseMetadata {
        version: version.to_owned(),
        package_sha256,
    })
}

#[cfg(unix)]
fn resolve_official_release_metadata(
    roots: &LocalCoreRoots,
    staging_root: &std::path::Path,
) -> Result<OfficialReleaseMetadata, LocalProductError> {
    let requested = std::env::var_os(UPDATE_VERSION_ENV);
    let requested_version = requested.as_deref().and_then(OsStr::to_str);
    if requested.is_some() && requested_version.is_none() {
        return Err(LocalProductError::LocalUpdate(
            "local update version is not UTF-8",
        ));
    }
    if let Some(version) = requested_version {
        if !valid_update_version(version) {
            return Err(LocalProductError::LocalUpdate(
                "local update version must be MAJOR.MINOR.PATCH",
            ));
        }
    }
    let url = requested_version
        .map(|version| format!("{UPSTREAM_RELEASE_METADATA_BASE}/{version}/release.json"))
        .unwrap_or_else(|| DEFAULT_UPSTREAM_LATEST_URL.to_owned());
    parse_update_index_url(OsStr::new(&url)).map_err(|_| {
        LocalProductError::LocalUpdate("upstream release metadata URL is not canonical")
    })?;
    let path = staging_root.join("upstream-release.json");
    let output = create_remote_output(&path)?;
    fetch_remote_file(
        roots,
        &url,
        &output,
        UPSTREAM_RELEASE_METADATA_MAX_BYTES as u64,
    )?;
    let bytes = read_bounded_regular_file(
        &path,
        UPSTREAM_RELEASE_METADATA_MAX_BYTES,
        "read upstream release metadata",
        LocalProductError::LocalUpdate("upstream release metadata exceeds its byte bound"),
        LocalProductError::LocalUpdate("upstream release metadata is not a regular file"),
    )?;
    parse_official_release_metadata(&bytes, requested_version)
}

#[cfg(unix)]
fn ensure_curl_available(curl: &std::path::Path) -> Result<(), LocalProductError> {
    let metadata =
        std::fs::symlink_metadata(curl).map_err(|_| LocalProductError::CurlUnavailable)?;
    if !metadata.file_type().is_file() {
        return Err(LocalProductError::CurlUnavailable);
    }
    Ok(())
}

#[cfg(unix)]
fn fetch_remote_file(
    roots: &LocalCoreRoots,
    url: &str,
    output: &std::fs::File,
    max_bytes: u64,
) -> Result<u64, LocalProductError> {
    fetch_remote_file_with_timeout(
        roots,
        url,
        output,
        max_bytes,
        REMOTE_TRANSFER_TIMEOUT_SECONDS,
    )
}

#[cfg(unix)]
fn fetch_remote_control_file(
    roots: &LocalCoreRoots,
    url: &str,
    output: &std::fs::File,
    max_bytes: u64,
) -> Result<u64, LocalProductError> {
    fetch_remote_file_with_timeout(
        roots,
        url,
        output,
        max_bytes,
        REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS,
    )
}

#[cfg(unix)]
fn fetch_remote_file_with_timeout(
    roots: &LocalCoreRoots,
    url: &str,
    output: &std::fs::File,
    max_bytes: u64,
    transfer_timeout_seconds: &str,
) -> Result<u64, LocalProductError> {
    ensure_curl_available(&roots.curl)?;
    if max_bytes == 0 || max_bytes > REMOTE_RELEASE_TOTAL_MAX_BYTES {
        return Err(LocalProductError::Remote(
            "remote release response bound is invalid",
        ));
    }
    let metadata = output.metadata().map_err(|source| LocalProductError::Io {
        operation: "inspect remote release output",
        source,
    })?;
    if !metadata.file_type().is_file() || metadata.len() != 0 {
        return Err(LocalProductError::Remote(
            "remote release output must be an empty regular file",
        ));
    }
    let child_output = output.try_clone().map_err(|source| LocalProductError::Io {
        operation: "duplicate remote release output",
        source,
    })?;
    let status = std::process::Command::new(&roots.curl)
        .arg("--disable")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--location",
        ])
        .arg("--cacert")
        .arg(&roots.cert_file)
        .arg("--capath")
        .arg(&roots.cert_dir)
        .args([
            "--connect-timeout",
            REMOTE_CONNECT_TIMEOUT_SECONDS,
            "--max-time",
            transfer_timeout_seconds,
            "--max-filesize",
        ])
        .arg(max_bytes.to_string())
        .arg("--url")
        .arg(url)
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(child_output))
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|_| LocalProductError::CurlUnavailable)?;
    if !status.success() {
        return Err(LocalProductError::RemoteTransportFailed);
    }
    output.sync_all().map_err(|source| LocalProductError::Io {
        operation: "sync remote release output",
        source,
    })?;
    let observed = output
        .metadata()
        .map_err(|source| LocalProductError::Io {
            operation: "inspect downloaded remote release output",
            source,
        })?
        .len();
    if observed > max_bytes {
        return Err(LocalProductError::RemoteResponseTooLarge);
    }
    Ok(observed)
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseFileEntry {
    relative_path: String,
    sha256: String,
    mode: u32,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReleasePublicKey([u8; 32]);

#[cfg(unix)]
impl ReleasePublicKey {
    fn parse_hex(value: &str) -> Option<Self> {
        if value.len() != 64 {
            return None;
        }
        fn nibble(byte: u8) -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            }
        }
        let bytes = value.as_bytes();
        let mut raw = [0u8; 32];
        for (index, output) in raw.iter_mut().enumerate() {
            *output = (nibble(bytes[index * 2])? << 4) | nibble(bytes[index * 2 + 1])?;
        }
        Some(Self(raw))
    }

    fn to_hex(self) -> String {
        let mut value = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
        }
        value
    }

    fn subject_public_key_info_der(self) -> [u8; 44] {
        const PREFIX: [u8; 12] = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ];
        let mut der = [0u8; 44];
        der[..PREFIX.len()].copy_from_slice(&PREFIX);
        der[PREFIX.len()..].copy_from_slice(&self.0);
        der
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalReleaseFormat {
    LegacyV3,
    CoordinatedV4,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalReleaseManifest {
    format: LocalReleaseFormat,
    generation_id: String,
    release_sequence: u64,
    channel: String,
    expected_platform: String,
    expected_architecture: String,
    core_api_identity: String,
    persistent_schema_identity: String,
    release_public_key: ReleasePublicKey,
    files: Vec<ReleaseFileEntry>,
}

#[cfg(unix)]
fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(*byte, b'a'..=b'f'))
}

#[cfg(unix)]
fn valid_positive_decimal(value: &str) -> bool {
    let Some(first) = value.as_bytes().first() else {
        return false;
    };
    matches!(*first, b'1'..=b'9') && value.as_bytes()[1..].iter().all(u8::is_ascii_digit)
}

#[cfg(unix)]
fn valid_nonnegative_decimal(value: &str) -> bool {
    value == "0" || valid_positive_decimal(value)
}

#[cfg(unix)]
fn parse_release_file_mode(value: &str, relative_path: &str) -> Result<u32, LocalProductError> {
    if value.len() != 4
        || !value.starts_with('0')
        || !value.as_bytes()[1..]
            .iter()
            .all(|byte| matches!(*byte, b'0'..=b'7'))
    {
        return Err(LocalProductError::Release(
            "release file inventory mode is invalid",
        ));
    }
    let mode = u32::from_str_radix(value, 8)
        .map_err(|_| LocalProductError::Release("release file inventory mode is invalid"))?;
    if mode & 0o400 == 0 {
        return Err(LocalProductError::ReleasePolicy(
            "release file is not owner-readable",
        ));
    }
    if (matches!(
        relative_path,
        "core"
            | "runtime"
            | "manager"
            | CODE_MODE_HOST_FILE
            | "browser/open/curl"
            | "browser/manual/curl"
    ) || relative_path.starts_with("helpers/"))
        && mode & 0o100 == 0
    {
        return Err(LocalProductError::ReleasePolicy(
            "release executable file is not owner-executable",
        ));
    }
    Ok(mode)
}

#[cfg(unix)]
fn valid_release_relative_path(value: &str) -> bool {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.chars().any(char::is_control)
    {
        return false;
    }
    if value
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return false;
    }
    if matches!(
        value,
        "core" | "generation.meta" | "runtime" | "manager" | CODE_MODE_HOST_FILE
    ) {
        return true;
    }
    if matches!(value, "browser/open/curl" | "browser/manual/curl") {
        return true;
    }
    if let Some(index) = value.strip_prefix("helpers/") {
        return !index.contains('/') && valid_nonnegative_decimal(index);
    }
    value
        .strip_prefix("compat/")
        .is_some_and(|rest| !rest.is_empty())
}

#[cfg(unix)]
fn parse_local_release_manifest(bytes: &[u8]) -> Result<LocalReleaseManifest, LocalProductError> {
    if bytes.len() > LOCAL_RELEASE_MAX_BYTES {
        return Err(LocalProductError::Release("release manifest is too large"));
    }
    if bytes.contains(&b'\r') {
        return Err(LocalProductError::Release(
            "release manifest line endings are unsupported",
        ));
    }
    if !bytes.ends_with(b"\n") {
        return Err(LocalProductError::Release(
            "release manifest is missing its final newline",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LocalProductError::Release("release manifest is not UTF-8"))?;
    let mut lines = text.lines();
    let format = match lines.next() {
        Some(LOCAL_RELEASE_FORMAT) => LocalReleaseFormat::LegacyV3,
        Some(LOCAL_RELEASE_FORMAT_V4) => LocalReleaseFormat::CoordinatedV4,
        _ => {
            return Err(LocalProductError::Release(
                "release manifest format is unsupported",
            ))
        }
    };
    let generation_id = descriptor_field(lines.next(), "generation_id")?;
    m2_generation_state::validate_generation_identity(generation_id, "release generation_id")
        .map_err(LocalProductError::StateFormat)?;
    let release_sequence = descriptor_field(lines.next(), "release_sequence")?;
    if !valid_positive_decimal(release_sequence) {
        return Err(LocalProductError::Release("release sequence is invalid"));
    }
    let release_sequence: u64 = release_sequence
        .parse()
        .map_err(|_| LocalProductError::Release("release sequence is invalid"))?;
    let channel = descriptor_field(lines.next(), "channel")?;
    let expected_platform = descriptor_field(lines.next(), "expected_platform")?;
    let expected_architecture = descriptor_field(lines.next(), "expected_architecture")?;
    let core_api_identity = descriptor_field(lines.next(), "core_api_identity")?;
    let persistent_schema_identity = descriptor_field(lines.next(), "persistent_schema_identity")?;
    let release_public_key = descriptor_field(lines.next(), "release_public_key")?;
    let release_public_key = ReleasePublicKey::parse_hex(release_public_key).ok_or(
        LocalProductError::Release("release public key is not canonical Ed25519 hex"),
    )?;
    let file_count = descriptor_field(lines.next(), "file_count")?;
    if !valid_positive_decimal(file_count) {
        return Err(LocalProductError::Release("release file count is invalid"));
    }
    let file_count: usize = file_count
        .parse()
        .map_err(|_| LocalProductError::Release("release file count is invalid"))?;
    if file_count > LOCAL_RELEASE_MAX_FILES {
        return Err(LocalProductError::Release(
            "release file count is outside the supported bound",
        ));
    }
    let mut files = Vec::with_capacity(file_count);
    let mut previous: Option<String> = None;
    for _ in 0..file_count {
        let line = lines.next().ok_or(LocalProductError::Release(
            "release file inventory is incomplete",
        ))?;
        let mut parts = line.split('\t');
        if parts.next() != Some("file") {
            return Err(LocalProductError::Release(
                "release file inventory entry is invalid",
            ));
        }
        let relative_path = parts
            .next()
            .filter(|path| valid_release_relative_path(path))
            .ok_or(LocalProductError::Release(
                "release file inventory path is invalid",
            ))?;
        let sha256 = parts
            .next()
            .filter(|digest| valid_sha256_hex(digest))
            .ok_or(LocalProductError::Release(
                "release file inventory digest is invalid",
            ))?;
        let mode = parts.next().ok_or(LocalProductError::Release(
            "release file inventory mode is missing",
        ))?;
        let mode = parse_release_file_mode(mode, relative_path)?;
        if parts.next().is_some() {
            return Err(LocalProductError::Release(
                "release file inventory entry has extra fields",
            ));
        }
        if previous
            .as_deref()
            .is_some_and(|prior| prior >= relative_path)
        {
            return Err(LocalProductError::Release(
                "release file inventory is not strictly sorted",
            ));
        }
        previous = Some(relative_path.to_owned());
        files.push(ReleaseFileEntry {
            relative_path: relative_path.to_owned(),
            sha256: sha256.to_owned(),
            mode,
        });
    }
    if lines.next().is_some() {
        return Err(LocalProductError::Release(
            "release manifest has unexpected trailing fields",
        ));
    }
    Ok(LocalReleaseManifest {
        format,
        generation_id: generation_id.to_owned(),
        release_sequence,
        channel: channel.to_owned(),
        expected_platform: expected_platform.to_owned(),
        expected_architecture: expected_architecture.to_owned(),
        core_api_identity: core_api_identity.to_owned(),
        persistent_schema_identity: persistent_schema_identity.to_owned(),
        release_public_key,
        files,
    })
}

#[cfg(unix)]
fn release_platform_matches_this_build(platform: &str, architecture: &str) -> bool {
    if platform == std::env::consts::OS && architecture == std::env::consts::ARCH {
        return true;
    }

    #[cfg(test)]
    if std::env::var_os("CODEX_B10_RELEASE_CORE").is_some()
        && platform == "android"
        && architecture == "aarch64"
    {
        return true;
    }

    false
}

#[cfg(unix)]
fn validate_local_release_policy(manifest: &LocalReleaseManifest) -> Result<(), LocalProductError> {
    if manifest.channel != LOCAL_RELEASE_CHANNEL {
        return Err(LocalProductError::ReleasePolicy(
            "release channel is not supported",
        ));
    }
    if !release_platform_matches_this_build(
        &manifest.expected_platform,
        &manifest.expected_architecture,
    ) {
        if manifest.expected_platform != std::env::consts::OS {
            return Err(LocalProductError::ReleasePolicy(
                "release platform does not match this build",
            ));
        }
        return Err(LocalProductError::ReleasePolicy(
            "release architecture does not match this build",
        ));
    }
    if manifest.core_api_identity != CORE_API_IDENTITY {
        return Err(LocalProductError::ReleasePolicy(
            "release Core API identity is incompatible",
        ));
    }
    if manifest.persistent_schema_identity != PERSISTENT_SCHEMA_IDENTITY {
        return Err(LocalProductError::ReleasePolicy(
            "release persistent schema identity is incompatible",
        ));
    }
    let has_core = manifest
        .files
        .iter()
        .any(|file| file.relative_path == "core");
    match manifest.format {
        LocalReleaseFormat::LegacyV3 if has_core => {
            return Err(LocalProductError::ReleasePolicy(
                "legacy v3 release must not contain a Core entrypoint",
            ));
        }
        LocalReleaseFormat::CoordinatedV4 if !has_core => {
            return Err(LocalProductError::ReleasePolicy(
                "coordinated v4 release must contain a Core entrypoint",
            ));
        }
        _ => {}
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_openssl_available(openssl: &std::path::Path) -> Result<(), LocalProductError> {
    if !openssl.is_file() {
        return Err(LocalProductError::OpenSslUnavailable);
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_trusted_release_key(key: &std::path::Path) -> Result<(), LocalProductError> {
    let metadata = std::fs::symlink_metadata(key)
        .map_err(|_| LocalProductError::TrustedReleaseKeyUnavailable)?;
    if !metadata.file_type().is_file() || metadata.len() > BOOTSTRAP_PUBLIC_KEY_MAX_BYTES {
        return Err(LocalProductError::TrustedReleaseKeyUnavailable);
    }
    Ok(())
}

#[cfg(unix)]
fn release_public_key_from_pem(
    openssl: &std::path::Path,
    trusted_key: &std::path::Path,
) -> Result<ReleasePublicKey, LocalProductError> {
    ensure_openssl_available(openssl)?;
    ensure_trusted_release_key(trusted_key)?;
    let output = std::process::Command::new(openssl)
        .args(["pkey", "-pubin", "-in"])
        .arg(trusted_key)
        .args(["-outform", "DER"])
        .env_clear()
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| LocalProductError::OpenSslUnavailable)?;
    const PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    if !output.status.success()
        || output.stdout.len() != 44
        || output.stdout[..PREFIX.len()] != PREFIX
    {
        return Err(LocalProductError::TrustedReleaseKeyUnavailable);
    }
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&output.stdout[PREFIX.len()..]);
    Ok(ReleasePublicKey(raw))
}

#[cfg(unix)]
fn release_public_key_from_private_pem(
    openssl: &std::path::Path,
    private_key: &std::path::Path,
) -> Result<ReleasePublicKey, LocalProductError> {
    use std::os::unix::fs::PermissionsExt;

    ensure_openssl_available(openssl)?;
    let metadata = std::fs::symlink_metadata(private_key)
        .map_err(|_| LocalProductError::LocalUpdate("local update signing key is unavailable"))?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > UPDATE_PRIVATE_KEY_MAX_BYTES
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.permissions().mode() & 0o400 == 0
    {
        return Err(LocalProductError::LocalUpdate(
            "local update signing key is unavailable or insecure",
        ));
    }
    let output = std::process::Command::new(openssl)
        .args(["pkey", "-in"])
        .arg(private_key)
        .args(["-pubout", "-outform", "DER"])
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| LocalProductError::OpenSslUnavailable)?;
    const PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    if !output.status.success()
        || output.stdout.len() != 44
        || output.stdout[..PREFIX.len()] != PREFIX
    {
        return Err(LocalProductError::LocalUpdate(
            "local update signing key is not a supported Ed25519 key",
        ));
    }
    let mut raw = [0u8; 32];
    raw.copy_from_slice(&output.stdout[PREFIX.len()..]);
    Ok(ReleasePublicKey(raw))
}

#[cfg(unix)]
fn verify_release_signature_with_key(
    openssl: &std::path::Path,
    trusted_key: ReleasePublicKey,
    manifest_path: &std::path::Path,
    signature_path: &std::path::Path,
) -> Result<(), LocalProductError> {
    use std::io::Write as _;

    let signature_metadata =
        std::fs::symlink_metadata(signature_path).map_err(|source| LocalProductError::Io {
            operation: "inspect release signature",
            source,
        })?;
    if !signature_metadata.file_type().is_file()
        || signature_metadata.len() > LOCAL_RELEASE_SIGNATURE_MAX_BYTES
    {
        return Err(LocalProductError::Release(
            "release signature is not a bounded regular file",
        ));
    }
    let mut child = std::process::Command::new(openssl)
        .args([
            "pkeyutl",
            "-verify",
            "-rawin",
            "-pubin",
            "-keyform",
            "DER",
            "-inkey",
            "/dev/stdin",
        ])
        .arg("-in")
        .arg(manifest_path)
        .arg("-sigfile")
        .arg(signature_path)
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| LocalProductError::OpenSslUnavailable)?;
    child
        .stdin
        .take()
        .ok_or(LocalProductError::OpenSslFailed("public-key input"))?
        .write_all(&trusted_key.subject_public_key_info_der())
        .map_err(|_| LocalProductError::OpenSslFailed("public-key input"))?;
    let status = child
        .wait()
        .map_err(|_| LocalProductError::OpenSslFailed("signature verification"))?;
    if status.success() {
        Ok(())
    } else {
        Err(LocalProductError::SignatureRejected)
    }
}

#[cfg(unix)]
fn release_authority_signature_present(path: &std::path::Path) -> Result<bool, LocalProductError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            if metadata.len() > LOCAL_RELEASE_SIGNATURE_MAX_BYTES {
                return Err(LocalProductError::Release(
                    "release authority signature is not a bounded regular file",
                ));
            }
            Ok(true)
        }
        Ok(_) => Err(LocalProductError::Release(
            "release authority signature is not a bounded regular file",
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(LocalProductError::Io {
            operation: "inspect release authority signature",
            source,
        }),
    }
}

#[cfg(unix)]
fn openssl_sha256(
    openssl: &std::path::Path,
    file: &std::path::Path,
) -> Result<String, LocalProductError> {
    ensure_openssl_available(openssl)?;
    let output = std::process::Command::new(openssl)
        .args(["dgst", "-sha256", "-binary"])
        .arg(file)
        .env_clear()
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| LocalProductError::OpenSslUnavailable)?;
    if !output.status.success() || output.stdout.len() != 32 {
        return Err(LocalProductError::OpenSslFailed("SHA-256"));
    }
    let mut hex = String::with_capacity(64);
    for byte in output.stdout {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing into String cannot fail");
    }
    Ok(hex)
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerationLayout {
    RootCodeModeHost,
    LegacyCompat,
}

#[cfg(unix)]
impl GenerationLayout {
    fn as_str(self) -> &'static str {
        match self {
            GenerationLayout::RootCodeModeHost => "root-code-mode-host-v2",
            GenerationLayout::LegacyCompat => "legacy-compat-v1",
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Clone)]
struct LoadedLocalGeneration {
    generation_id: String,
    manifest: GenerationManifest,
    doctor_capability: UpstreamDoctorCapability,
    generation_layout: GenerationLayout,
    core_path: Option<std::path::PathBuf>,
    runtime_path: std::path::PathBuf,
    code_mode_host_path: std::path::PathBuf,
    compatibility_dir: std::path::PathBuf,
    manager_path: Option<std::path::PathBuf>,
    helper_paths: Vec<std::path::PathBuf>,
}

#[cfg(unix)]
fn descriptor_field<'a>(
    line: Option<&'a str>,
    expected: &'static str,
) -> Result<&'a str, LocalProductError> {
    let line = line.ok_or(LocalProductError::Descriptor(
        "generation descriptor is incomplete",
    ))?;
    let Some((name, value)) = line.split_once('\t') else {
        return Err(LocalProductError::Descriptor(
            "generation descriptor field is malformed",
        ));
    };
    if name != expected || value.is_empty() {
        return Err(LocalProductError::Descriptor(
            "generation descriptor field is invalid",
        ));
    }
    Ok(value)
}

#[cfg(unix)]
fn load_local_generation(
    generation_dir: &std::path::Path,
) -> Result<LoadedLocalGeneration, LocalProductError> {
    ensure_real_directory(
        generation_dir,
        "read activated generation descriptor",
        "activated generation directory must be a real directory",
    )?;
    let descriptor_path = generation_dir.join("generation.meta");
    let bytes = read_bounded_regular_file(
        &descriptor_path,
        LOCAL_GENERATION_MAX_BYTES,
        "read activated generation descriptor",
        LocalProductError::Descriptor("generation descriptor is too large"),
        LocalProductError::UnsafeSource("generation descriptor must be a regular file"),
    )?;
    if !bytes.ends_with(b"\n") {
        return Err(LocalProductError::Descriptor(
            "generation descriptor is missing its final newline",
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| LocalProductError::Descriptor("generation descriptor is not UTF-8"))?;
    let mut lines = text.lines();
    let generation_layout = match lines.next() {
        Some(LOCAL_GENERATION_FORMAT) => GenerationLayout::RootCodeModeHost,
        Some(LEGACY_GENERATION_FORMAT) => GenerationLayout::LegacyCompat,
        _ => {
            return Err(LocalProductError::Descriptor(
                "generation descriptor format is unsupported",
            ))
        }
    };

    let generation_id = descriptor_field(lines.next(), "generation_id")?;
    m2_generation_state::validate_generation_identity(generation_id, "generation_id")
        .map_err(LocalProductError::StateFormat)?;
    let upstream_package_identity = descriptor_field(lines.next(), "upstream_package_identity")?;
    let upstream_package_version = descriptor_field(lines.next(), "upstream_package_version")?;
    let source_artifact_digest = descriptor_field(lines.next(), "source_artifact_digest")?;
    let expected_platform = descriptor_field(lines.next(), "expected_platform")?;
    let expected_architecture = descriptor_field(lines.next(), "expected_architecture")?;
    let patch_policy_id = descriptor_field(lines.next(), "patch_policy_id")?;
    let patch_report = descriptor_field(lines.next(), "patch_report")?;
    let runtime_digest = descriptor_field(lines.next(), "runtime_digest")?;
    let core_artifact_digest = descriptor_field(lines.next(), "core_artifact_digest")?;
    let manager_digest = descriptor_field(lines.next(), "manager_artifact_digest")?;
    let core_api_identity = descriptor_field(lines.next(), "core_api_identity")?;
    let persistent_schema_identity = descriptor_field(lines.next(), "persistent_schema_identity")?;
    if descriptor_field(lines.next(), "qualification")? != "qualified" {
        return Err(LocalProductError::Descriptor(
            "activated generation is not qualified",
        ));
    }
    let creation_metadata = descriptor_field(lines.next(), "creation_metadata")?;
    let doctor_capability = match descriptor_field(lines.next(), "upstream_doctor")? {
        "supported" => UpstreamDoctorCapability::Supported,
        "unsupported" => UpstreamDoctorCapability::Unsupported,
        _ => {
            return Err(LocalProductError::Descriptor(
                "generation descriptor doctor capability is invalid",
            ))
        }
    };
    let helper_count = descriptor_field(lines.next(), "helper_count")?;
    if !valid_nonnegative_decimal(helper_count) {
        return Err(LocalProductError::Descriptor(
            "generation helper count is invalid",
        ));
    }
    let helper_count: usize = helper_count
        .parse()
        .map_err(|_| LocalProductError::Descriptor("generation helper count is invalid"))?;
    if helper_count > LOCAL_RELEASE_MAX_FILES.saturating_sub(2) {
        return Err(LocalProductError::Descriptor(
            "generation helper count is outside the supported bound",
        ));
    }
    let mut helper_digests = Vec::with_capacity(helper_count);
    for _ in 0..helper_count {
        let line = lines.next().ok_or(LocalProductError::Descriptor(
            "generation helper list is incomplete",
        ))?;
        let mut parts = line.split('\t');
        if parts.next() != Some("helper") {
            return Err(LocalProductError::Descriptor(
                "generation helper entry is invalid",
            ));
        }
        let identity =
            parts
                .next()
                .filter(|value| !value.is_empty())
                .ok_or(LocalProductError::Descriptor(
                    "generation helper identity is invalid",
                ))?;
        let digest =
            parts
                .next()
                .filter(|value| !value.is_empty())
                .ok_or(LocalProductError::Descriptor(
                    "generation helper digest is invalid",
                ))?;
        if parts.next().is_some() {
            return Err(LocalProductError::Descriptor(
                "generation helper entry is invalid",
            ));
        }
        helper_digests.push(GenerationHelperDigest {
            identity: identity.to_owned(),
            digest: digest.to_owned(),
        });
    }
    if lines.next().is_some() {
        return Err(LocalProductError::Descriptor(
            "generation descriptor has unexpected trailing fields",
        ));
    }

    let manifest = GenerationManifest {
        upstream_package_identity: upstream_package_identity.to_owned(),
        upstream_package_version: upstream_package_version.to_owned(),
        source_artifact_digest: source_artifact_digest.to_owned(),
        expected_platform: expected_platform.to_owned(),
        expected_architecture: expected_architecture.to_owned(),
        patch_policy_id: patch_policy_id.to_owned(),
        patch_report: patch_report.to_owned(),
        runtime_digest: runtime_digest.to_owned(),
        helper_digests,
        core_artifact_digest: core_artifact_digest.to_owned(),
        manager_artifact_digest: (manager_digest != "-").then(|| manager_digest.to_owned()),
        core_api_identity: core_api_identity.to_owned(),
        persistent_schema_identity: persistent_schema_identity.to_owned(),
        creation_metadata: creation_metadata.to_owned(),
    };

    let runtime_path = generation_dir.join("runtime");
    if !runtime_path.is_file() {
        return Err(LocalProductError::Descriptor(
            "activated generation runtime is missing",
        ));
    }
    ensure_regular_file(
        &runtime_path,
        "inspect activated generation runtime",
        "activated generation runtime must be a regular file",
    )?;
    let core_path = match std::fs::symlink_metadata(generation_dir.join("core")) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(LocalProductError::UnsafeSource(
                    "activated generation Core must be a regular file",
                ));
            }
            let path = generation_dir.join("core");
            ensure_regular_file(
                &path,
                "inspect activated generation Core",
                "activated generation Core must be a regular file",
            )?;
            Some(path)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect activated generation Core",
                source,
            })
        }
    };
    let (code_mode_host_path, compatibility_dir) = match generation_layout {
        GenerationLayout::RootCodeModeHost => {
            let code_mode_host_path = generation_dir.join(CODE_MODE_HOST_FILE);
            ensure_regular_file(
                &code_mode_host_path,
                "inspect activated generation code-mode host",
                "activated generation code-mode host must be a regular file",
            )?;
            let legacy_compatibility_dir = generation_dir.join("compat");
            match std::fs::symlink_metadata(&legacy_compatibility_dir) {
                Ok(_) => {
                    return Err(LocalProductError::UnsafeSource(
                        "v2 generation must not contain a compatibility directory",
                    ))
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(LocalProductError::Io {
                        operation: "inspect activated generation compatibility directory",
                        source,
                    })
                }
            }
            (code_mode_host_path, generation_dir.to_path_buf())
        }
        GenerationLayout::LegacyCompat => {
            let compatibility_dir = generation_dir.join("compat");
            if !compatibility_dir.is_dir() {
                return Err(LocalProductError::Descriptor(
                    "activated generation compatibility directory is missing",
                ));
            }
            ensure_real_directory(
                &compatibility_dir,
                "inspect activated generation compatibility directory",
                "activated generation compatibility directory must be a real directory",
            )?;
            let code_mode_host_path = compatibility_dir.join(CODE_MODE_HOST_FILE);
            ensure_regular_file(
                &code_mode_host_path,
                "inspect legacy generation code-mode host",
                "legacy generation code-mode host must be a regular file",
            )?;
            (code_mode_host_path, compatibility_dir)
        }
    };
    let manager_path = manifest
        .manager_artifact_digest
        .as_ref()
        .map(|_| generation_dir.join("manager"));
    if manager_path.as_ref().is_some_and(|path| !path.is_file()) {
        return Err(LocalProductError::Descriptor(
            "activated generation Manager is missing",
        ));
    }
    if let Some(manager_path) = manager_path.as_ref() {
        ensure_regular_file(
            manager_path,
            "inspect activated generation Manager",
            "activated generation Manager must be a regular file",
        )?;
    }
    let r10_browser_helper_bridge = r10_browser_helper_bridge(&manifest)?;
    let helper_paths: Vec<_> = manifest
        .helper_digests
        .iter()
        .enumerate()
        .map(|(index, helper)| {
            generation_dir.join(generation_helper_relative_path_for_layout(
                index,
                &helper.identity,
                r10_browser_helper_bridge,
            ))
        })
        .collect();
    if helper_paths.iter().any(|path| !path.is_file()) {
        return Err(LocalProductError::Descriptor(
            "activated generation helper is missing",
        ));
    }
    for (index, helper_path) in helper_paths.iter().enumerate() {
        let identity = manifest.helper_digests[index].identity.as_str();
        match identity {
            TERMUX_BROWSER_OPEN_HELPER_IDENTITY | TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
                if r10_browser_helper_bridge =>
            {
                ensure_real_directory(
                    &generation_dir.join("helpers"),
                    "inspect R10 bridge browser helper directory",
                    "R10 bridge browser helper directory must be a real directory",
                )?;
            }
            TERMUX_BROWSER_OPEN_HELPER_IDENTITY | TERMUX_BROWSER_MANUAL_HELPER_IDENTITY => {
                ensure_real_directory(
                    &generation_dir.join("browser"),
                    "inspect activated generation browser helper root",
                    "activated generation browser helper root must be a real directory",
                )?;
                let leaf = if identity == TERMUX_BROWSER_OPEN_HELPER_IDENTITY {
                    "open"
                } else {
                    "manual"
                };
                ensure_real_directory(
                    &generation_dir.join("browser").join(leaf),
                    "inspect activated generation browser helper directory",
                    "activated generation browser helper directory must be a real directory",
                )?;
            }
            _ => ensure_real_directory(
                &generation_dir.join("helpers"),
                "inspect activated generation helper directory",
                "activated generation helper directory must be a real directory",
            )?,
        }
        ensure_regular_file(
            helper_path,
            "inspect activated generation helper",
            "activated generation helper must be a regular file",
        )?;
    }

    Ok(LoadedLocalGeneration {
        generation_id: generation_id.to_owned(),
        manifest,
        doctor_capability,
        generation_layout,
        core_path,
        runtime_path,
        code_mode_host_path,
        compatibility_dir,
        manager_path,
        helper_paths,
    })
}

#[cfg(unix)]
fn ensure_real_directory(
    path: &std::path::Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if !metadata.file_type().is_dir() {
        return Err(LocalProductError::UnsafeSource(message));
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_real_directory_if_present(
    path: &std::path::Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(LocalProductError::UnsafeSource(message)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LocalProductError::Io { operation, source }),
    }
}

#[cfg(unix)]
fn ensure_real_directory_tree(
    path: &std::path::Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(LocalProductError::UnsafeSource(message)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(LocalProductError::Io {
                operation,
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "directory path has no parent",
                ),
            })?;
            if parent == path {
                return Err(LocalProductError::Io {
                    operation,
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "directory path cannot be created",
                    ),
                });
            }
            ensure_real_directory_tree(parent, operation, message)?;
            match std::fs::create_dir(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(LocalProductError::Io { operation, source }),
            }
            ensure_real_directory(path, operation, message)
        }
        Err(source) => Err(LocalProductError::Io { operation, source }),
    }
}

#[cfg(unix)]
fn read_bounded_regular_file(
    path: &std::path::Path,
    maximum: usize,
    operation: &'static str,
    too_large: LocalProductError,
    not_regular: LocalProductError,
) -> Result<Vec<u8>, LocalProductError> {
    use std::io::Read as _;

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if !metadata.file_type().is_file() {
        return Err(not_regular);
    }
    if metadata.len() > maximum as u64 {
        return Err(too_large);
    }
    let file =
        std::fs::File::open(path).map_err(|source| LocalProductError::Io { operation, source })?;
    let mut bytes = Vec::with_capacity(std::cmp::min(metadata.len() as usize, maximum));
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if bytes.len() > maximum {
        return Err(too_large);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn ensure_regular_file(
    path: &std::path::Path,
    operation: &'static str,
    message: &'static str,
) -> Result<(), LocalProductError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if !metadata.file_type().is_file() {
        return Err(LocalProductError::UnsafeSource(message));
    }
    Ok(())
}

#[cfg(unix)]
fn inspect_authenticated_core_artifact(
    path: &std::path::Path,
) -> Result<std::fs::Metadata, LocalProductError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| LocalProductError::Io {
        operation: "inspect authenticated Core artifact",
        source,
    })?;
    if !metadata.file_type().is_file() {
        return Err(LocalProductError::UnsafeSource(
            "authenticated Core artifact must be a regular non-symlink file",
        ));
    }
    if metadata.len() > CORE_ARTIFACT_MAX_BYTES {
        return Err(LocalProductError::Bootstrap(
            "authenticated Core artifact exceeds its byte bound",
        ));
    }
    Ok(metadata)
}

#[cfg(unix)]
fn authenticated_core_digest(
    roots: &LocalCoreRoots,
    path: &std::path::Path,
) -> Result<String, LocalProductError> {
    inspect_authenticated_core_artifact(path)?;
    openssl_sha256(&roots.openssl, path)
}

#[cfg(unix)]
fn copy_bounded_file_contents(
    input: &mut std::fs::File,
    output: &mut std::fs::File,
    maximum: u64,
    operation: &'static str,
    too_large: &'static str,
) -> Result<(), LocalProductError> {
    use std::io::Read as _;

    let mut bounded = input.take(maximum.saturating_add(1));
    let copied = std::io::copy(&mut bounded, output)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if copied > maximum {
        return Err(LocalProductError::Bootstrap(too_large));
    }
    Ok(())
}

#[cfg(unix)]
fn collect_compat_release_files(
    generation_dir: &std::path::Path,
    directory: &std::path::Path,
    files: &mut Vec<String>,
) -> Result<(), LocalProductError> {
    ensure_real_directory(
        directory,
        "inspect release compatibility directory",
        "release compatibility tree contains a symlink or special file",
    )?;
    for entry in std::fs::read_dir(directory).map_err(|source| LocalProductError::Io {
        operation: "read release compatibility directory",
        source,
    })? {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "read release compatibility entry",
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| LocalProductError::Io {
            operation: "inspect release compatibility entry",
            source,
        })?;
        if file_type.is_dir() {
            collect_compat_release_files(generation_dir, &entry.path(), files)?;
        } else if file_type.is_file() {
            let path = entry.path();
            let relative = path
                .strip_prefix(generation_dir)
                .map_err(|_| LocalProductError::Release("release file escaped generation root"))?
                .to_str()
                .filter(|path| valid_release_relative_path(path))
                .ok_or(LocalProductError::Release(
                    "release file path is not supported UTF-8",
                ))?
                .to_owned();
            if files.len() == LOCAL_RELEASE_MAX_FILES {
                return Err(LocalProductError::Release(
                    "release file count is outside the supported bound",
                ));
            }
            files.push(relative);
        } else {
            return Err(LocalProductError::UnsafeSource(
                "release compatibility tree contains a symlink or special file",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn exact_release_file_paths(
    generation_dir: &std::path::Path,
    loaded: &LoadedLocalGeneration,
) -> Result<Vec<String>, LocalProductError> {
    ensure_regular_file(
        &loaded.runtime_path,
        "inspect release runtime",
        "release runtime must be a regular file",
    )?;
    ensure_regular_file(
        &loaded.code_mode_host_path,
        "inspect release code-mode host",
        "release code-mode host must be a regular file",
    )?;
    let mut files = vec!["generation.meta".to_owned(), "runtime".to_owned()];
    if let Some(core_path) = loaded.core_path.as_ref() {
        ensure_regular_file(
            core_path,
            "inspect release Core",
            "release Core must be a regular file",
        )?;
        files.push("core".to_owned());
    }
    match loaded.generation_layout {
        GenerationLayout::RootCodeModeHost => files.push(CODE_MODE_HOST_FILE.to_owned()),
        GenerationLayout::LegacyCompat => {}
    }
    if let Some(manager_path) = loaded.manager_path.as_ref() {
        ensure_regular_file(
            manager_path,
            "inspect release Manager",
            "release Manager must be a regular file",
        )?;
        files.push("manager".to_owned());
    }
    for (index, helper_path) in loaded.helper_paths.iter().enumerate() {
        ensure_regular_file(
            helper_path,
            "inspect release helper",
            "release helper must be a regular file",
        )?;
        files.push(
            generation_helper_relative_path_for_layout(
                index,
                &loaded.manifest.helper_digests[index].identity,
                loaded.manifest.creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA,
            )
            .to_str()
            .ok_or(LocalProductError::Release(
                "release helper path is not supported UTF-8",
            ))?
            .to_owned(),
        );
    }
    if files.len() > LOCAL_RELEASE_MAX_FILES {
        return Err(LocalProductError::Release(
            "release file count is outside the supported bound",
        ));
    }
    if loaded.generation_layout == GenerationLayout::LegacyCompat {
        collect_compat_release_files(generation_dir, &loaded.compatibility_dir, &mut files)?;
    }
    files.sort();
    Ok(files)
}

#[cfg(unix)]
fn verify_release_inventory(
    openssl: &std::path::Path,
    generation_dir: &std::path::Path,
    manifest: &LocalReleaseManifest,
) -> Result<LoadedLocalGeneration, LocalProductError> {
    ensure_regular_file(
        &generation_dir.join("generation.meta"),
        "inspect release generation descriptor",
        "release generation descriptor must be a regular file",
    )?;
    let loaded = load_local_generation(generation_dir)?;
    if loaded.generation_id != manifest.generation_id {
        return Err(LocalProductError::Release(
            "release generation id does not match generation descriptor",
        ));
    }
    let signed_core = manifest
        .files
        .iter()
        .find(|file| file.relative_path == "core");
    match (manifest.format, signed_core, loaded.core_path.as_ref()) {
        (LocalReleaseFormat::CoordinatedV4, None, _) => {
            return Err(LocalProductError::Release(
                "coordinated release is missing its signed Core entry",
            ));
        }
        (LocalReleaseFormat::LegacyV3, Some(_), _) => {
            return Err(LocalProductError::Release(
                "legacy release contains a signed Core entry",
            ));
        }
        (_, Some(_), None) => {
            return Err(LocalProductError::Release(
                "signed Core entry is missing from generation content",
            ));
        }
        (_, None, Some(_)) => {
            return Err(LocalProductError::Release(
                "generation Core is absent from the signed inventory",
            ));
        }
        _ => {}
    }
    if let Some(core) = signed_core {
        if core.sha256 != loaded.manifest.core_artifact_digest {
            return Err(LocalProductError::Release(
                "signed Core entry does not match its generation descriptor",
            ));
        }
    }
    let actual = exact_release_file_paths(generation_dir, &loaded)?;
    if actual.len() != manifest.files.len()
        || !actual
            .iter()
            .zip(&manifest.files)
            .all(|(actual, signed)| actual == &signed.relative_path)
    {
        return Err(LocalProductError::Release(
            "release file inventory does not exactly match generation content",
        ));
    }
    for file in &manifest.files {
        if openssl_sha256(openssl, &generation_dir.join(&file.relative_path))? != file.sha256 {
            return Err(LocalProductError::ReleaseDigestMismatch);
        }
        use std::os::unix::fs::PermissionsExt as _;
        let metadata = std::fs::symlink_metadata(generation_dir.join(&file.relative_path))
            .map_err(|source| LocalProductError::Io {
                operation: "inspect release file mode",
                source,
            })?;
        if metadata.permissions().mode() & 0o7777 != file.mode {
            return Err(LocalProductError::ReleaseModeMismatch);
        }
    }
    Ok(loaded)
}

#[cfg(unix)]
fn read_local_release_manifest(
    generation_dir: &std::path::Path,
) -> Result<(std::path::PathBuf, LocalReleaseManifest), LocalProductError> {
    ensure_real_directory(
        generation_dir,
        "inspect release generation root",
        "release generation root must be a real directory",
    )?;
    let manifest_path = generation_dir.join("release.manifest");
    let bytes = read_bounded_regular_file(
        &manifest_path,
        LOCAL_RELEASE_MAX_BYTES,
        "read release manifest",
        LocalProductError::Release("release manifest is not a bounded regular file"),
        LocalProductError::Release("release manifest is not a bounded regular file"),
    )?;
    Ok((manifest_path, parse_local_release_manifest(&bytes)?))
}

#[cfg(unix)]
fn verify_local_release_control_with_key(
    generation_dir: &std::path::Path,
    openssl: &std::path::Path,
    update_key: ReleasePublicKey,
) -> Result<LocalReleaseManifest, LocalProductError> {
    ensure_openssl_available(openssl)?;
    let (manifest_path, manifest) = read_local_release_manifest(generation_dir)?;
    let authority_path = generation_dir.join("release-authority.sig");
    let has_authority = release_authority_signature_present(&authority_path)?;
    if manifest.release_public_key == update_key {
        if has_authority {
            return Err(LocalProductError::Release(
                "release authority signature is unexpected for a non-rotation",
            ));
        }
    } else {
        if !has_authority {
            return Err(LocalProductError::Release(
                "release authority signature is required for key rotation",
            ));
        }
        verify_release_signature_with_key(openssl, update_key, &manifest_path, &authority_path)?;
    }
    verify_release_signature_with_key(
        openssl,
        manifest.release_public_key,
        &manifest_path,
        &generation_dir.join("release.sig"),
    )?;
    validate_local_release_policy(&manifest)?;
    Ok(manifest)
}

#[cfg(unix)]
fn verify_local_release_bundle_with_key(
    generation_dir: &std::path::Path,
    openssl: &std::path::Path,
    update_key: ReleasePublicKey,
) -> Result<(LocalReleaseManifest, LoadedLocalGeneration), LocalProductError> {
    let manifest = verify_local_release_control_with_key(generation_dir, openssl, update_key)?;
    let loaded = verify_release_inventory(openssl, generation_dir, &manifest)?;
    Ok((manifest, loaded))
}

#[cfg(all(unix, test))]
fn verify_local_release_bundle(
    generation_dir: &std::path::Path,
    openssl: &std::path::Path,
    trusted_key: &std::path::Path,
) -> Result<(LocalReleaseManifest, LoadedLocalGeneration), LocalProductError> {
    let update_key = release_public_key_from_pem(openssl, trusted_key)?;
    verify_local_release_bundle_with_key(generation_dir, openssl, update_key)
}

#[cfg(unix)]
fn verify_installed_local_release(
    roots: &LocalCoreRoots,
    generation_id: &str,
    verifier_key: ReleasePublicKey,
    mismatch: &'static str,
) -> Result<(LocalReleaseManifest, LoadedLocalGeneration), LocalProductError> {
    ensure_real_directory(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    ensure_openssl_available(&roots.openssl)?;
    let generation_dir = roots.generation_root.join(generation_id);
    let (manifest_path, manifest) = read_local_release_manifest(&generation_dir)?;
    if manifest.release_public_key != verifier_key {
        return Err(LocalProductError::Release(
            "installed release key does not match state verifier key",
        ));
    }
    verify_release_signature_with_key(
        &roots.openssl,
        verifier_key,
        &manifest_path,
        &generation_dir.join("release.sig"),
    )?;
    validate_local_release_policy(&manifest)?;
    let loaded = verify_release_inventory(&roots.openssl, &generation_dir, &manifest)?;
    if loaded.generation_id != generation_id {
        return Err(LocalProductError::Descriptor(mismatch));
    }
    Ok((manifest, loaded))
}

#[cfg(unix)]
static LOCAL_STAGING_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(unix)]
fn copy_local_regular_file(
    source: &std::path::Path,
    destination: &std::path::Path,
    label: &'static str,
) -> Result<(), LocalProductError> {
    ensure_regular_file(source, "inspect local generation source", label)?;
    std::fs::copy(source, destination).map_err(|source| LocalProductError::Io {
        operation: "copy local generation file",
        source,
    })?;
    Ok(())
}

#[cfg(unix)]
fn copy_local_directory_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), LocalProductError> {
    ensure_real_directory(
        source,
        "inspect local generation directory",
        "local generation directory is not a real directory",
    )?;
    std::fs::create_dir(destination).map_err(|source| LocalProductError::Io {
        operation: "create staged generation directory",
        source,
    })?;
    for entry in std::fs::read_dir(source).map_err(|source| LocalProductError::Io {
        operation: "read local generation directory",
        source,
    })? {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "read local generation directory entry",
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| LocalProductError::Io {
            operation: "inspect local generation directory entry",
            source,
        })?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_local_directory_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            copy_local_regular_file(
                &entry.path(),
                &target,
                "local generation compatibility tree contains a non-regular file",
            )?;
        } else {
            return Err(LocalProductError::UnsafeSource(
                "local generation compatibility tree contains a symlink or special file",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn generation_path_exists(path: &std::path::Path) -> Result<bool, LocalProductError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(LocalProductError::Io {
            operation: "inspect immutable generation path",
            source,
        }),
    }
}

#[cfg(unix)]
trait GenerationPublishIo {
    fn sync_file(&mut self, path: &std::path::Path) -> std::io::Result<()>;
    fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()>;
    fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()>;
}

#[cfg(unix)]
struct FsGenerationPublishIo;

#[cfg(unix)]
impl GenerationPublishIo for FsGenerationPublishIo {
    fn sync_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::File::open(path)?.sync_all()
    }

    fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::File::open(path)?.sync_all()
    }

    fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
        rename_noreplace(from, to)
    }
}

#[cfg(unix)]
fn sync_generation_tree<I: GenerationPublishIo>(
    path: &std::path::Path,
    io: &mut I,
) -> Result<(), LocalProductError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| LocalProductError::Io {
        operation: "inspect generation publication path",
        source,
    })?;
    let file_type = metadata.file_type();
    if file_type.is_file() {
        io.sync_file(path).map_err(|source| LocalProductError::Io {
            operation: "sync immutable generation file",
            source,
        })?;
        return Ok(());
    }
    if !file_type.is_dir() {
        return Err(LocalProductError::UnsafeSource(
            "generation publication tree contains a symlink or special file",
        ));
    }

    for entry in std::fs::read_dir(path).map_err(|source| LocalProductError::Io {
        operation: "read generation publication directory",
        source,
    })? {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "read generation publication entry",
            source,
        })?;
        let entry_type = entry.file_type().map_err(|source| LocalProductError::Io {
            operation: "inspect generation publication entry",
            source,
        })?;
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(LocalProductError::UnsafeSource(
                "generation publication tree contains a symlink or special file",
            ));
        }
        sync_generation_tree(&entry.path(), io)?;
    }
    io.sync_dir(path).map_err(|source| LocalProductError::Io {
        operation: "sync immutable generation directory",
        source,
    })?;
    Ok(())
}

#[cfg(unix)]
fn sync_complete_generation(
    generation_path: &std::path::Path,
    generation_root: &std::path::Path,
) -> Result<(), LocalProductError> {
    let mut io = FsGenerationPublishIo;
    sync_generation_tree(generation_path, &mut io)?;
    io.sync_dir(generation_root)
        .map_err(|source| LocalProductError::Io {
            operation: "sync immutable generation root",
            source,
        })
}

#[cfg(unix)]
fn stage_local_generation(
    source_dir: &std::path::Path,
    generation_root: &std::path::Path,
) -> Result<String, LocalProductError> {
    let mut io = FsGenerationPublishIo;
    stage_local_generation_with_io(source_dir, generation_root, &mut io)
}

#[cfg(unix)]
fn stage_local_generation_with_io<I: GenerationPublishIo>(
    source_dir: &std::path::Path,
    generation_root: &std::path::Path,
    io: &mut I,
) -> Result<String, LocalProductError> {
    ensure_real_directory(
        source_dir,
        "inspect local generation source root",
        "local generation source must be a real directory",
    )?;
    ensure_real_directory(
        generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;

    let source = load_local_generation(source_dir)?;
    let final_path = generation_root.join(&source.generation_id);
    if generation_path_exists(&final_path)? {
        return Err(LocalProductError::GenerationCollision);
    }
    let sequence = LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let candidate = generation_root.join(format!(".candidate-{}-{sequence}", std::process::id()));
    std::fs::create_dir(&candidate).map_err(|source| LocalProductError::Io {
        operation: "create private generation candidate",
        source,
    })?;

    let result = (|| {
        copy_local_regular_file(
            &source_dir.join("generation.meta"),
            &candidate.join("generation.meta"),
            "generation descriptor must be a regular file",
        )?;
        copy_local_regular_file(
            &source_dir.join("release.manifest"),
            &candidate.join("release.manifest"),
            "release manifest must be a regular file",
        )?;
        copy_local_regular_file(
            &source_dir.join("release.sig"),
            &candidate.join("release.sig"),
            "release signature must be a regular file",
        )?;
        let authority_signature = source_dir.join("release-authority.sig");
        if release_authority_signature_present(&authority_signature)? {
            copy_local_regular_file(
                &authority_signature,
                &candidate.join("release-authority.sig"),
                "release authority signature must be a regular file",
            )?;
        }
        copy_local_regular_file(
            &source.runtime_path,
            &candidate.join("runtime"),
            "runtime must be a regular file",
        )?;
        if let Some(core) = source.core_path.as_ref() {
            copy_local_regular_file(core, &candidate.join("core"), "Core must be a regular file")?;
        }
        match source.generation_layout {
            GenerationLayout::RootCodeModeHost => copy_local_regular_file(
                &source.code_mode_host_path,
                &candidate.join(CODE_MODE_HOST_FILE),
                "code-mode host must be a regular file",
            )?,
            GenerationLayout::LegacyCompat => {
                copy_local_directory_tree(&source.compatibility_dir, &candidate.join("compat"))?
            }
        }
        if let Some(manager) = source.manager_path.as_ref() {
            copy_local_regular_file(
                manager,
                &candidate.join("manager"),
                "Manager must be a regular file",
            )?;
        }
        for (index, helper) in source.helper_paths.iter().enumerate() {
            let relative = generation_helper_relative_path_for_layout(
                index,
                &source.manifest.helper_digests[index].identity,
                source.manifest.creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA,
            );
            let destination = candidate.join(relative);
            let parent = destination.parent().ok_or(LocalProductError::Descriptor(
                "staged helper path has no parent",
            ))?;
            std::fs::create_dir_all(parent).map_err(|source| LocalProductError::Io {
                operation: "create staged helper directory",
                source,
            })?;
            copy_local_regular_file(helper, &destination, "helper must be a regular file")?;
        }
        let copied = load_local_generation(&candidate)?;
        if copied.generation_id != source.generation_id {
            return Err(LocalProductError::Descriptor(
                "copied generation id changed during staging",
            ));
        }
        sync_generation_tree(&candidate, io)?;
        if generation_path_exists(&final_path)? {
            return Err(LocalProductError::GenerationCollision);
        }
        io.rename(&candidate, &final_path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                LocalProductError::GenerationCollision
            } else {
                LocalProductError::Io {
                    operation: "publish immutable local generation",
                    source,
                }
            }
        })?;
        io.sync_dir(generation_root)
            .map_err(|source| LocalProductError::Io {
                operation: "sync immutable generation root",
                source,
            })?;
        Ok(source.generation_id)
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&candidate);
    }
    result
}

#[cfg(unix)]
fn load_activated_generation(
    roots: &LocalCoreRoots,
) -> Result<LoadedLocalGeneration, LocalProductError> {
    ensure_real_directory(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let state = m2_generation_state::read_pointer_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    let loaded = load_local_generation(&roots.generation_root.join(&state.current))?;
    if loaded.generation_id != state.current {
        return Err(LocalProductError::Descriptor(
            "activated generation descriptor id does not match current",
        ));
    }
    Ok(loaded)
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreRepairAction {
    None,
    Update,
    Unavailable,
}

#[cfg(unix)]
impl CoreRepairAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Update => "update",
            Self::Unavailable => "unavailable",
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreRepairReason {
    Healthy,
    LegacyGeneration,
    CoreStateUnavailable,
}

#[cfg(unix)]
impl CoreRepairReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::LegacyGeneration => "legacy-generation",
            Self::CoreStateUnavailable => "core-state-unavailable",
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CoreRepairPlan {
    action: CoreRepairAction,
    reason: CoreRepairReason,
}

#[cfg(unix)]
fn core_repair_plan(roots: &LocalCoreRoots) -> CoreRepairPlan {
    let result = (|| {
        let loaded = load_activated_generation(roots).map_err(|_| ())?;
        let layout = loaded.generation_layout;
        with_qualified_loaded_runtime(&loaded, |_, _| Ok::<_, LocalProductError>(()))
            .map_err(|_| ())?;
        Ok::<_, ()>(layout)
    })();
    match result {
        Ok(GenerationLayout::RootCodeModeHost) => CoreRepairPlan {
            action: CoreRepairAction::None,
            reason: CoreRepairReason::Healthy,
        },
        Ok(GenerationLayout::LegacyCompat) => CoreRepairPlan {
            action: CoreRepairAction::Update,
            reason: CoreRepairReason::LegacyGeneration,
        },
        Err(()) => CoreRepairPlan {
            action: CoreRepairAction::Unavailable,
            reason: CoreRepairReason::CoreStateUnavailable,
        },
    }
}

#[cfg(unix)]
fn render_core_repair_plan(plan: CoreRepairPlan) -> String {
    format!(
        "codex-core-repair-v1\naction={}\nreason={}\n",
        plan.action.as_str(),
        plan.reason.as_str()
    )
}

#[cfg(unix)]
fn generation_requirements_for_loaded(
    _manifest: &GenerationManifest,
) -> GenerationManifestRequirements<'static> {
    #[cfg(test)]
    if std::env::var_os("CODEX_B10_RELEASE_CORE").is_some()
        && _manifest.expected_platform == "android"
        && _manifest.expected_architecture == "aarch64"
    {
        return GenerationManifestRequirements {
            platform: "android",
            architecture: "aarch64",
            core_api_identity: CORE_API_IDENTITY,
            persistent_schema_identity: PERSISTENT_SCHEMA_IDENTITY,
        };
    }

    GenerationManifestRequirements {
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        core_api_identity: CORE_API_IDENTITY,
        persistent_schema_identity: PERSISTENT_SCHEMA_IDENTITY,
    }
}

#[cfg(unix)]
fn with_qualified_loaded_runtime<'loaded, T, F>(
    loaded: &'loaded LoadedLocalGeneration,
    operation: F,
) -> Result<T, LocalProductError>
where
    F: for<'selection, 'asset> FnOnce(
        QualifiedGenerationManifest<'loaded>,
        QualifiedRuntimeAssets<'selection, 'asset>,
    ) -> Result<T, LocalProductError>,
{
    let requirements = generation_requirements_for_loaded(&loaded.manifest);
    let generation = qualify_generation_manifest(&loaded.manifest, &requirements)
        .map_err(LocalProductError::Manifest)?;
    let helper_bindings: Vec<_> = loaded
        .manifest
        .helper_digests
        .iter()
        .zip(&loaded.helper_paths)
        .map(|(helper, path)| HelperAssetBinding {
            identity: &helper.identity,
            asset_path: path.as_os_str(),
            observed_digest: &helper.digest,
        })
        .collect();
    let runtime_selection = RuntimeAssetSelection {
        runtime: RuntimeAssetBinding {
            program_path: loaded.runtime_path.as_os_str(),
            observed_digest: &loaded.manifest.runtime_digest,
        },
        compatibility_dir: loaded.compatibility_dir.as_os_str(),
        helpers: &helper_bindings,
    };
    let runtime_assets = qualify_runtime_assets(generation, &runtime_selection)
        .map_err(LocalProductError::Runtime)?;
    operation(generation, runtime_assets)
}

#[cfg(unix)]
fn public_doctor_capability(
    creation_metadata: &str,
    declared: UpstreamDoctorCapability,
) -> UpstreamDoctorCapability {
    if creation_metadata == R10_BROWSER_HELPER_BRIDGE_METADATA
        && declared == UpstreamDoctorCapability::Unsupported
    {
        // RALD-4.5 transition generations preserve the exact R10 layout marker so
        // the public stable Core can stage them, while `unsupported` tells only
        // that legacy activation gate not to couple activation to user/provider
        // health. The corrected public doctor must still execute upstream doctor.
        UpstreamDoctorCapability::Supported
    } else {
        declared
    }
}

#[cfg(unix)]
fn execute_activated_route(
    route: PublicDispatchRoute,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<PublicDispatchCompletion, LocalProductError> {
    let loaded = load_activated_generation(roots)?;
    with_qualified_loaded_runtime(&loaded, |generation, runtime_assets| {
        let manager_selection = loaded
            .manager_path
            .as_ref()
            .map(|path| ManagerArtifactSelection {
                program_path: path.as_os_str(),
                observed_digest: loaded
                    .manifest
                    .manager_artifact_digest
                    .as_deref()
                    .expect("Manager path is created only for a declared Manager"),
            });
        let manager_artifact = qualify_manager_artifact(generation, manager_selection.as_ref())
            .map_err(LocalProductError::Manager)?;
        let manager_doctor_status = match manager_artifact {
            ManagerArtifact::Unavailable => ManagerDoctorStatus::Unavailable,
            ManagerArtifact::Available(_) => ManagerDoctorStatus::Healthy,
        };
        let core_doctor_status = match loaded.generation_layout {
            GenerationLayout::RootCodeModeHost => CoreDoctorStatus::Healthy,
            GenerationLayout::LegacyCompat => CoreDoctorStatus::Unhealthy,
        };
        let context = LocalPublicDispatchContext {
            runtime_assets,
            manager_artifact,
            process_env,
            cert_file: roots.cert_file.as_os_str(),
            cert_dir: Some(roots.cert_dir.as_os_str()),
            resolver_path: &roots.resolver_path,
            config_dir: &roots.config_dir,
            doctor_capability: public_doctor_capability(
                &loaded.manifest.creation_metadata,
                loaded.doctor_capability,
            ),
            core_doctor_status,
            generation_id: &loaded.generation_id,
            generation_layout: loaded.generation_layout,
            manager_doctor_status,
        };
        execute_public_dispatch(route, context).map_err(LocalProductError::Dispatch)
    })
}

#[cfg(unix)]
fn doctor_exit_code(class: DoctorExitClass) -> i32 {
    match class {
        DoctorExitClass::Success => 0,
        DoctorExitClass::HealthFailure => 1,
        DoctorExitClass::ApiIncompatibility => 2,
    }
}

#[cfg(unix)]
fn probe_release_candidate(
    loaded: &LoadedLocalGeneration,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<(), LocalProductError> {
    with_qualified_loaded_runtime(loaded, |_, assets| {
        if !probe_qualified_upstream_command(
            assets,
            process_env,
            roots.cert_file.as_os_str(),
            Some(roots.cert_dir.as_os_str()),
            &roots.resolver_path,
            &roots.config_dir,
            &["-c", "sandbox_mode=\"danger-full-access\"", "--version"],
        )
        .map_err(|_| LocalProductError::CandidateProbe("candidate version probe failed"))?
        {
            return Err(LocalProductError::CandidateProbe(
                "candidate version probe was unhealthy",
            ));
        }
        Ok(())
    })
}

#[cfg(unix)]
fn update_progress_enabled(stdout_is_terminal: bool, stderr_is_terminal: bool) -> bool {
    stdout_is_terminal && stderr_is_terminal
}

#[cfg(unix)]
fn render_update_header(previous_version: &str, current_version: &str) -> String {
    if previous_version == current_version {
        format!("Updating the Termux release for Codex {current_version}...")
    } else {
        format!("Updating Codex {previous_version} -> {current_version}...")
    }
}

#[cfg(unix)]
fn render_update_failure(error: &LocalProductError) -> String {
    let raw = error.to_string();
    let raw = raw.trim().trim_end_matches(['.', '!', '?']);
    let mut chars = raw.chars();
    let sentence = match chars.next() {
        Some(first) => {
            let mut sentence = first.to_uppercase().collect::<String>();
            sentence.push_str(chars.as_str());
            sentence
        }
        None => "Update failed".to_owned(),
    };
    format!("{sentence}. ❌")
}

#[cfg(unix)]
const UPDATE_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[cfg(unix)]
fn update_spinner_frame(index: usize) -> &'static str {
    UPDATE_SPINNER_FRAMES[index % UPDATE_SPINNER_FRAMES.len()]
}

#[cfg(unix)]
fn update_spinner_frame_index(glyph: &str) -> usize {
    UPDATE_SPINNER_FRAMES
        .iter()
        .position(|candidate| *candidate == glyph)
        .unwrap_or(0)
}

#[cfg(unix)]
fn render_update_transient(frame_index: usize, message: &str) {
    use std::io::Write as _;
    let mut stderr = std::io::stderr().lock();
    let _ = write!(
        stderr,
        "\r\x1b[2K{} {}",
        update_spinner_frame(frame_index),
        message
    );
    let _ = stderr.flush();
}

#[cfg(unix)]
#[derive(Debug)]
enum UpdateTransientCommand {
    Set {
        frame_index: usize,
        message: String,
        rendered: std::sync::mpsc::Sender<()>,
    },
    Stop,
}

#[cfg(unix)]
#[derive(Debug)]
struct UpdateTransientWorker {
    sender: std::sync::mpsc::Sender<UpdateTransientCommand>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
#[derive(Debug)]
struct UpdatePresentation {
    transient_enabled: bool,
    transient_visible: bool,
    transient_worker: Option<UpdateTransientWorker>,
    update_started: bool,
}

#[cfg(unix)]
impl UpdatePresentation {
    fn new() -> Self {
        use std::io::IsTerminal as _;
        Self {
            transient_enabled: update_progress_enabled(
                std::io::stdout().is_terminal(),
                std::io::stderr().is_terminal(),
            ),
            transient_visible: false,
            transient_worker: None,
            update_started: false,
        }
    }

    fn transient(&mut self, glyph: &str, message: &str) {
        if !self.transient_enabled {
            return;
        }
        let frame_index = update_spinner_frame_index(glyph);
        if let Some(transient) = self.transient_worker.as_mut() {
            let (rendered, rendered_rx) = std::sync::mpsc::channel();
            if transient
                .sender
                .send(UpdateTransientCommand::Set {
                    frame_index,
                    message: message.to_owned(),
                    rendered,
                })
                .is_ok()
            {
                let _ = rendered_rx.recv_timeout(std::time::Duration::from_millis(250));
                self.transient_visible = true;
                return;
            }
            if let Some(worker) = transient.worker.take() {
                let _ = worker.join();
            }
            self.transient_worker = None;
        }

        render_update_transient(frame_index, message);
        let (sender, receiver) = std::sync::mpsc::channel();
        let initial_message = message.to_owned();
        let worker = std::thread::Builder::new()
            .name("codex-update-spinner".to_owned())
            .spawn(move || {
                let mut frame_index = frame_index;
                let mut message = initial_message;
                loop {
                    match receiver.recv_timeout(std::time::Duration::from_millis(80)) {
                        Ok(UpdateTransientCommand::Set {
                            frame_index: next_frame,
                            message: next_message,
                            rendered,
                        }) => {
                            frame_index = next_frame;
                            message = next_message;
                            render_update_transient(frame_index, &message);
                            let _ = rendered.send(());
                        }
                        Ok(UpdateTransientCommand::Stop)
                        | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            frame_index = frame_index.wrapping_add(1);
                            render_update_transient(frame_index, &message);
                        }
                    }
                }
            });

        if let Ok(worker) = worker {
            self.transient_worker = Some(UpdateTransientWorker {
                sender,
                worker: Some(worker),
            });
        }
        self.transient_visible = true;
    }

    fn clear_transient(&mut self) {
        if let Some(mut transient) = self.transient_worker.take() {
            let _ = transient.sender.send(UpdateTransientCommand::Stop);
            if let Some(worker) = transient.worker.take() {
                let _ = worker.join();
            }
        }
        if !self.transient_visible {
            return;
        }
        use std::io::Write as _;
        let mut stderr = std::io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[2K");
        let _ = stderr.flush();
        self.transient_visible = false;
    }

    fn begin_update(&mut self, previous_version: &str, current_version: &str) {
        if self.update_started {
            return;
        }
        self.clear_transient();
        println!(
            "{}",
            render_update_header(previous_version, current_version)
        );
        use std::io::Write as _;
        let _ = std::io::stdout().flush();
        self.update_started = true;
    }

    fn finish_signed_activation(&mut self, current_version: &str) {
        self.clear_transient();
        println!("Verified and activated the signed Termux release.");
        println!("Codex {current_version} is now active. ✅");
    }

    fn finish_local_activation(&mut self, current_version: &str) {
        self.clear_transient();
        println!("Verified and activated the locally built Termux release.");
        println!("Codex {current_version} is now active. ✅");
    }

    fn finish_current(&mut self, current_version: &str) {
        self.clear_transient();
        println!("Codex {current_version} is already up to date. ✅");
    }

    fn finish_rollback(&mut self, previous_version: &str, current_version: &str) {
        self.clear_transient();
        if previous_version == current_version {
            println!("Rolled back the Termux release for Codex {current_version}. ✅");
        } else {
            println!("Rolled back Codex {previous_version} -> {current_version}. ✅");
        }
    }

    fn fail(&mut self, error: &LocalProductError) {
        self.clear_transient();
        eprintln!("{}", render_update_failure(error));
    }
}

#[cfg(unix)]
impl Drop for UpdatePresentation {
    fn drop(&mut self) {
        self.clear_transient();
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateTarget {
    generation_id: String,
    version: String,
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActivatedUpdate {
    generation_id: String,
    previous_version: String,
    current_version: String,
}

#[cfg(unix)]
#[derive(Debug)]
struct PreparedLocalActivation {
    before: m2_generation_state::GenerationPointerState,
    generation_id: String,
    release_sequence: u64,
    release_key: ReleasePublicKey,
    previous_version: String,
    candidate_version: String,
    staged_loaded: LoadedLocalGeneration,
    validated_hold: Option<UpdateHoldRecord>,
    preserve_update_key: bool,
}

#[cfg(unix)]
#[derive(Debug)]
enum PreparedSignedLocalRelease {
    AlreadyCurrent(UpdateTarget),
    Activation(Box<PreparedLocalActivation>),
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum SignedUpdateOutcome {
    Activated(ActivatedUpdate),
    AlreadyCurrent(UpdateTarget),
}

#[cfg(unix)]
impl SignedUpdateOutcome {
    fn generation_id(&self) -> &str {
        match self {
            Self::Activated(outcome) => &outcome.generation_id,
            Self::AlreadyCurrent(target) => &target.generation_id,
        }
    }
}

#[cfg(unix)]
fn prepare_signed_local_release_with_hold_policy(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    hold_policy: UpdateHoldPolicy,
) -> Result<PreparedSignedLocalRelease, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    ensure_real_directory(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    let (source_release, source_loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, before.update_key)?;
    if source_loaded.manager_path.is_some() && source_loaded.core_path.is_none() {
        return Err(LocalProductError::ReleasePolicy(
            "new Manager-bearing releases require a coordinated Core artifact",
        ));
    }
    let (current_release, current_loaded) = verify_installed_local_release(
        roots,
        &before.current,
        before.current_key,
        "active generation descriptor id does not match current",
    )?;
    if source_release.release_sequence < current_release.release_sequence {
        return Err(LocalProductError::ReleaseSequenceRollback);
    }
    let validated_hold = rollback_guard::effective_update_hold(roots)?;
    if source_release.release_sequence == current_release.release_sequence {
        if source_loaded.generation_id == before.current && source_release == current_release {
            if hold_policy == UpdateHoldPolicy::ForceHeld {
                let hold = validated_hold
                    .as_ref()
                    .ok_or(LocalProductError::UpdateHold(
                        "forced update requires an active rollback hold",
                    ))?;
                if source_release.release_sequence != hold.release_sequence {
                    return Err(LocalProductError::UpdateHold(
                        "forced update candidate does not match the held release sequence",
                    ));
                }
            }
            return Ok(PreparedSignedLocalRelease::AlreadyCurrent(UpdateTarget {
                generation_id: before.current,
                version: current_loaded.manifest.upstream_package_version.clone(),
            }));
        }
        return Err(LocalProductError::ReleaseSequenceRollback);
    }
    match hold_policy {
        UpdateHoldPolicy::Enforce => {
            if let Some(hold) = validated_hold.as_ref() {
                if source_release.release_sequence <= hold.release_sequence {
                    return Err(LocalProductError::UpdateHeld {
                        generation_id: hold.generation_id.clone().into_boxed_str(),
                        release_sequence: hold.release_sequence,
                    });
                }
            }
        }
        UpdateHoldPolicy::ForceHeld => {
            let hold = validated_hold
                .as_ref()
                .ok_or(LocalProductError::UpdateHold(
                    "forced update requires an active rollback hold",
                ))?;
            if source_release.release_sequence != hold.release_sequence {
                return Err(LocalProductError::UpdateHold(
                    "forced update candidate does not match the held release sequence",
                ));
            }
        }
    }

    let destination = roots.generation_root.join(&source_loaded.generation_id);
    let (generation_id, staged_loaded) = if generation_path_exists(&destination)? {
        let (staged_release, staged_loaded) =
            verify_local_release_bundle_with_key(&destination, &roots.openssl, before.update_key)?;
        if staged_release != source_release
            || staged_loaded.generation_id != source_loaded.generation_id
        {
            return Err(LocalProductError::GenerationCollision);
        }
        sync_complete_generation(&destination, &roots.generation_root)?;
        (source_loaded.generation_id.clone(), staged_loaded)
    } else {
        let generation_id = stage_local_generation(source_dir, &roots.generation_root)?;
        let staged_result = verify_local_release_bundle_with_key(
            &roots.generation_root.join(&generation_id),
            &roots.openssl,
            before.update_key,
        );
        match staged_result {
            Ok((staged_release, staged_loaded))
                if staged_release == source_release
                    && staged_loaded.generation_id == source_loaded.generation_id =>
            {
                (generation_id, staged_loaded)
            }
            Ok(_) => {
                let _ = std::fs::remove_dir_all(&destination);
                return Err(LocalProductError::Release(
                    "staged signed release differs from admitted source",
                ));
            }
            Err(err) => {
                let _ = std::fs::remove_dir_all(&destination);
                return Err(err);
            }
        }
    };

    Ok(PreparedSignedLocalRelease::Activation(Box::new(
        PreparedLocalActivation {
            before,
            generation_id,
            release_sequence: source_release.release_sequence,
            release_key: source_release.release_public_key,
            previous_version: current_loaded.manifest.upstream_package_version.clone(),
            candidate_version: staged_loaded.manifest.upstream_package_version.clone(),
            staged_loaded,
            validated_hold,
            preserve_update_key: false,
        },
    )))
}

#[cfg(unix)]
fn prepare_local_derived_release(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    local_key: ReleasePublicKey,
    expected_baseline: &PublicReleaseBaseline,
) -> Result<PreparedLocalActivation, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    let (_, current_loaded) = verify_installed_local_release(
        roots,
        &before.current,
        before.current_key,
        "active generation descriptor id does not match current",
    )?;
    ensure_real_directory(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    if rollback_guard::effective_update_hold(roots)?.is_some() {
        return Err(LocalProductError::LocalUpdate(
            "local-derived update is unavailable while a rollback hold is active",
        ));
    }
    let (observed_baseline, _, _) = authenticated_public_baseline(roots, &before)?;
    if &observed_baseline != expected_baseline {
        return Err(LocalProductError::LocalUpdate(
            "local-derived public baseline changed during construction",
        ));
    }
    let (source_release, source_loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, local_key)?;
    if source_release.release_public_key != local_key {
        return Err(LocalProductError::LocalUpdate(
            "local-derived release verifier does not match its ephemeral authority",
        ));
    }
    let provenance = parse_local_derived_metadata(&source_release, &source_loaded)?.ok_or(
        LocalProductError::LocalUpdate("local-derived release is missing bound provenance"),
    )?;
    if provenance != *expected_baseline {
        return Err(LocalProductError::LocalUpdate(
            "local-derived release does not match its authenticated public baseline",
        ));
    }
    if source_loaded.manager_path.is_some() && source_loaded.core_path.is_none() {
        return Err(LocalProductError::ReleasePolicy(
            "new Manager-bearing releases require a coordinated Core artifact",
        ));
    }

    let destination = roots.generation_root.join(&source_loaded.generation_id);
    if generation_path_exists(&destination)? {
        return Err(LocalProductError::GenerationCollision);
    }
    let generation_id = stage_local_generation(source_dir, &roots.generation_root)?;
    let staged_result = verify_local_release_bundle_with_key(
        &roots.generation_root.join(&generation_id),
        &roots.openssl,
        local_key,
    );
    let (staged_release, staged_loaded) = match staged_result {
        Ok(result) => result,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&destination);
            return Err(error);
        }
    };
    if staged_release != source_release
        || staged_loaded.generation_id != source_loaded.generation_id
    {
        let _ = std::fs::remove_dir_all(&destination);
        return Err(LocalProductError::Release(
            "staged local-derived release differs from admitted source",
        ));
    }

    Ok(PreparedLocalActivation {
        before,
        generation_id,
        release_sequence: source_release.release_sequence,
        release_key: local_key,
        previous_version: current_loaded.manifest.upstream_package_version.clone(),
        candidate_version: staged_loaded.manifest.upstream_package_version.clone(),
        staged_loaded,
        validated_hold: None,
        preserve_update_key: true,
    })
}

#[cfg(all(unix, test))]
fn prepare_signed_local_release(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
) -> Result<PreparedSignedLocalRelease, LocalProductError> {
    prepare_signed_local_release_with_hold_policy(source_dir, roots, UpdateHoldPolicy::Enforce)
}

#[cfg(unix)]
fn activate_prepared_local_release(
    prepared: PreparedLocalActivation,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<ActivatedUpdate, LocalProductError> {
    let previous_version = prepared.previous_version.clone();
    let current_version = prepared.candidate_version.clone();
    if let Some(presentation) = presentation.as_deref_mut() {
        presentation.begin_update(&previous_version, &current_version);
        presentation.transient("⠹", "Checking candidate runtime...");
    }
    std::fs::create_dir_all(&roots.config_dir).map_err(|source| LocalProductError::Io {
        operation: "create Core config directory",
        source,
    })?;
    probe_release_candidate(&prepared.staged_loaded, roots, process_env)?;
    if let Some(presentation) = presentation {
        presentation.transient("⠸", &format!("Activating Codex {current_version}..."));
    }

    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    m2_generation_state::prepare_core_state_paths(&state_paths)
        .map_err(LocalProductError::State)?;
    let after = if prepared.preserve_update_key {
        m2_generation_state::plan_local_activation_pointer_state_with_key(
            &prepared.before,
            &prepared.generation_id,
            prepared.release_key,
        )
    } else {
        m2_generation_state::plan_activation_pointer_state_with_key(
            &prepared.before,
            &prepared.generation_id,
            prepared.release_key,
        )
    }
    .map_err(LocalProductError::StateFormat)?;
    let lock = m2_generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let authoritative =
        m2_generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
    if authoritative.as_ref() != Some(&prepared.before) {
        return Err(LocalProductError::State(
            m2_generation_state::ActivationTransactionError::StaleAuthoritativeState,
        ));
    }
    let (_, active_loaded) = verify_installed_local_release(
        roots,
        &prepared.before.current,
        prepared.before.current_key,
        "active generation descriptor id does not match current",
    )?;
    let active_core_binding =
        active_core_entrypoint_binding_with_state(roots, &active_loaded, &prepared.before)?;

    let rollback = if let Some(core_path) = prepared.staged_loaded.core_path.as_ref() {
        let candidate_digest = &prepared.staged_loaded.manifest.core_artifact_digest;
        if active_core_binding
            .as_ref()
            .is_some_and(|binding| binding.digest == *candidate_digest)
        {
            None
        } else {
            let snapshot_generation = active_core_binding
                .as_ref()
                .map(|binding| binding.generation_id.as_str())
                .unwrap_or(prepared.before.current.as_str());
            let record = snapshot_core_entrypoint_to_rollback(roots, snapshot_generation)?;
            if let Err(error) = install_core_entrypoint(roots, core_path, candidate_digest) {
                let _ = install_core_entrypoint(roots, &core_rollback_path(roots), &record.digest);
                return Err(error);
            }
            Some(record)
        }
    } else {
        None
    };
    let mut io = m2_generation_state::FsActivationIo;
    let state_result = m2_generation_state::activate_pointer_state_locked(
        &state_paths,
        Some(&prepared.before),
        &after,
        &mut io,
        &lock,
    );
    if let Err(error) = state_result {
        let authoritative = m2_generation_state::read_pointer_state(&state_paths)
            .map_err(LocalProductError::State)?;
        if authoritative.as_ref() != Some(&after) {
            if let Some(record) = rollback.as_ref() {
                install_core_entrypoint(roots, &core_rollback_path(roots), &record.digest)?;
            }
        }
        return Err(LocalProductError::State(error));
    }
    finalize_validated_update_hold_locked(
        roots,
        prepared.validated_hold.as_ref(),
        prepared.release_sequence,
    )?;
    drop(lock);
    Ok(ActivatedUpdate {
        generation_id: prepared.generation_id,
        previous_version,
        current_version,
    })
}

#[cfg(unix)]
fn activate_signed_local_release_outcome_with_hold_policy(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    hold_policy: UpdateHoldPolicy,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<SignedUpdateOutcome, LocalProductError> {
    if let Some(presentation) = presentation.as_deref_mut() {
        presentation.transient("⠹", "Verifying release signature and contents...");
    }
    match prepare_signed_local_release_with_hold_policy(source_dir, roots, hold_policy)? {
        PreparedSignedLocalRelease::AlreadyCurrent(target) => {
            Ok(SignedUpdateOutcome::AlreadyCurrent(target))
        }
        PreparedSignedLocalRelease::Activation(prepared) => {
            activate_prepared_local_release(*prepared, roots, process_env, presentation)
                .map(SignedUpdateOutcome::Activated)
        }
    }
}

#[cfg(unix)]
fn activate_signed_local_release_outcome(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: Option<&mut UpdatePresentation>,
) -> Result<SignedUpdateOutcome, LocalProductError> {
    activate_signed_local_release_outcome_with_hold_policy(
        source_dir,
        roots,
        process_env,
        UpdateHoldPolicy::Enforce,
        presentation,
    )
}

#[cfg(unix)]
fn create_local_update_staging_root(
    state_root: &std::path::Path,
) -> Result<std::path::PathBuf, LocalProductError> {
    use std::os::unix::fs::DirBuilderExt;

    ensure_real_directory(
        state_root,
        "inspect Core update state root",
        "Core update state root is not a real directory",
    )?;
    for _ in 0..32 {
        let sequence = LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = state_root.join(format!(".local-update-{}-{sequence}", std::process::id()));
        match std::fs::symlink_metadata(&path) {
            Ok(_) => continue,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = std::fs::DirBuilder::new();
                builder.mode(0o700);
                match builder.create(&path) {
                    Ok(()) => return Ok(path),
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(source) => {
                        return Err(LocalProductError::Io {
                            operation: "create private local update staging root",
                            source,
                        })
                    }
                }
            }
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "inspect local update staging root",
                    source,
                })
            }
        }
    }
    Err(LocalProductError::LocalUpdate(
        "could not allocate a unique local update staging root",
    ))
}

#[cfg(unix)]
fn local_update_generation_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let sequence = LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("local-{timestamp}-{}-{sequence}", std::process::id())
}

#[cfg(unix)]
fn running_core_artifact_for_local_build() -> Result<std::path::PathBuf, LocalProductError> {
    #[cfg(test)]
    if let Some(input) = std::env::var_os("CODEX_B10_RELEASE_CORE") {
        return std::fs::canonicalize(input).map_err(|source| LocalProductError::Io {
            operation: "resolve test release Core artifact",
            source,
        });
    }

    let input = std::env::current_exe().map_err(|source| LocalProductError::Io {
        operation: "resolve running Core artifact",
        source,
    })?;
    std::fs::canonicalize(input).map_err(|source| LocalProductError::Io {
        operation: "resolve running Core artifact",
        source,
    })
}

#[cfg(unix)]
fn activate_local_built_update(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<ActivatedUpdate, LocalProductError> {
    if let Some(presentation) = presentation.as_deref_mut() {
        presentation.transient("⠋", "Checking official Codex release...");
    }
    let metadata_staging = create_local_update_staging_root(&roots.state_root)?;
    let metadata = resolve_official_release_metadata(roots, &metadata_staging);
    let cleanup =
        std::fs::remove_dir_all(&metadata_staging).map_err(|source| LocalProductError::Io {
            operation: "remove upstream metadata staging",
            source,
        });
    let metadata = match (metadata, cleanup) {
        (_, Err(error)) => return Err(error),
        (Err(error), Ok(())) => return Err(error),
        (Ok(metadata), Ok(())) => metadata,
    };

    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    if rollback_guard::effective_update_hold(roots)?.is_some() {
        return Err(LocalProductError::LocalUpdate(
            "local-derived update is unavailable while a rollback hold is active",
        ));
    }
    let (baseline, _, current_loaded) = authenticated_public_baseline(roots, &before)?;
    let staging_root = create_local_update_staging_root(&roots.state_root)?;
    let result = (|| {
        let (private_key, local_key) =
            generate_ephemeral_local_derived_key(&roots.openssl, &staging_root)?;
        let generation_id = local_update_generation_id();
        let archive = staging_root.join(UPSTREAM_PACKAGE_ASSET);
        if let Some(presentation) = presentation.as_deref_mut() {
            presentation.transient("⠙", "Downloading official Codex source...");
        }
        let archive_digest = codex_release_builder::fetch_archive(
            &metadata.version,
            &roots.curl,
            &roots.openssl,
            &archive,
        )
        .map_err(|_| {
            LocalProductError::LocalUpdate("official upstream archive acquisition failed")
        })?;
        if archive_digest != metadata.package_sha256 {
            return Err(LocalProductError::LocalUpdate(
                "official upstream archive digest does not match release metadata",
            ));
        }
        let core = running_core_artifact_for_local_build()?;
        let gzip = roots
            .curl
            .parent()
            .ok_or(LocalProductError::LocalUpdate(
                "Termux tool directory is unavailable",
            ))?
            .join("gzip");
        let unsigned_generation = staging_root.join("unsigned-generation");
        let creation_metadata = render_local_derived_metadata(&metadata, &baseline);
        if let Some(presentation) = presentation.as_deref_mut() {
            presentation.transient("⠹", "Building Termux release...");
        }
        codex_release_builder::build_generation_with_manager(
            &metadata.version,
            &archive,
            &archive_digest,
            &generation_id,
            &core,
            current_loaded.manager_path.as_deref(),
            &creation_metadata,
            &gzip,
            &roots.openssl,
            &unsigned_generation,
        )
        .map_err(|_| LocalProductError::LocalUpdate("local upstream adaptation failed"))?;
        let publication = staging_root.join("local-derived-publication");
        let release_base = format!("https://local.invalid/codex/{generation_id}/");
        codex_release_builder::publish_generation(
            &unsigned_generation,
            &baseline.release_sequence.to_string(),
            &release_base,
            &private_key,
            &roots.openssl,
            &publication,
        )
        .map_err(|_| LocalProductError::LocalUpdate("local-derived signing failed"))?;
        std::fs::remove_file(&private_key).map_err(|source| LocalProductError::Io {
            operation: "remove ephemeral local-derived signing key",
            source,
        })?;
        sync_directory(&staging_root)?;
        let published_generation = publication.join("releases").join(&generation_id);
        if let Some(presentation) = presentation.as_deref_mut() {
            presentation.transient("⠼", "Verifying locally built Termux release...");
        }
        let prepared =
            prepare_local_derived_release(&published_generation, roots, local_key, &baseline)?;
        activate_prepared_local_release(prepared, roots, process_env, presentation.as_deref_mut())
    })();
    let cleanup = std::fs::remove_dir_all(&staging_root).map_err(|source| LocalProductError::Io {
        operation: "remove private local-derived staging",
        source,
    });
    match (result, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(outcome), Ok(())) => Ok(outcome),
    }
}

#[cfg(unix)]
static REMOTE_ACQUISITION_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[cfg(unix)]
fn create_remote_acquisition_root(path: &std::path::Path) -> Result<(), LocalProductError> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(|source| LocalProductError::Io {
            operation: "create private remote acquisition root",
            source,
        })
}

#[cfg(unix)]
fn ensure_remote_private_directory(path: &std::path::Path) -> Result<(), LocalProductError> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => {
            return Err(LocalProductError::UnsafeSource(
                "remote acquisition path contains a non-directory",
            ))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder
                .create(path)
                .map_err(|source| LocalProductError::Io {
                    operation: "create remote acquisition directory",
                    source,
                })?;
        }
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect remote acquisition directory",
                source,
            })
        }
    }
    let mut permissions = std::fs::symlink_metadata(path)
        .map_err(|source| LocalProductError::Io {
            operation: "inspect created remote acquisition directory",
            source,
        })?
        .permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions).map_err(|source| LocalProductError::Io {
        operation: "set remote acquisition directory permissions",
        source,
    })?;
    Ok(())
}

#[cfg(unix)]
fn create_remote_output(path: &std::path::Path) -> Result<std::fs::File, LocalProductError> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|source| LocalProductError::Io {
            operation: "create remote release output",
            source,
        })?;
    let mut permissions = file
        .metadata()
        .map_err(|source| LocalProductError::Io {
            operation: "inspect created remote release output",
            source,
        })?
        .permissions();
    permissions.set_mode(0o600);
    file.set_permissions(permissions)
        .map_err(|source| LocalProductError::Io {
            operation: "set remote release output permissions",
            source,
        })?;
    Ok(file)
}

#[cfg(unix)]
fn ensure_remote_resource_parent(
    acquisition_root: &std::path::Path,
    relative_path: &str,
) -> Result<(), LocalProductError> {
    if !valid_release_relative_path(relative_path) {
        return Err(LocalProductError::Remote(
            "remote release resource path is invalid",
        ));
    }
    let Some(parent) = std::path::Path::new(relative_path).parent() else {
        return Ok(());
    };
    let mut current = acquisition_root.to_path_buf();
    for component in parent.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(LocalProductError::Remote(
                "remote release resource path is invalid",
            ));
        };
        current.push(component);
        ensure_remote_private_directory(&current)?;
    }
    Ok(())
}

#[cfg(all(unix, test))]
fn fetch_remote_resource(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    relative_path: &str,
    destination: &std::path::Path,
    response_limit: u64,
    acquired_bytes: &mut u64,
) -> Result<(), LocalProductError> {
    fetch_remote_resource_with_timeout(
        roots,
        base,
        relative_path,
        destination,
        response_limit,
        acquired_bytes,
        REMOTE_TRANSFER_TIMEOUT_SECONDS,
    )
}

#[cfg(unix)]
fn fetch_remote_control_resource(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    relative_path: &str,
    destination: &std::path::Path,
    response_limit: u64,
    acquired_bytes: &mut u64,
) -> Result<(), LocalProductError> {
    fetch_remote_resource_with_timeout(
        roots,
        base,
        relative_path,
        destination,
        response_limit,
        acquired_bytes,
        REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS,
    )
}

#[cfg(unix)]
fn fetch_remote_resource_with_timeout(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    relative_path: &str,
    destination: &std::path::Path,
    response_limit: u64,
    acquired_bytes: &mut u64,
    transfer_timeout_seconds: &str,
) -> Result<(), LocalProductError> {
    let remaining = REMOTE_RELEASE_TOTAL_MAX_BYTES
        .checked_sub(*acquired_bytes)
        .ok_or(LocalProductError::RemoteResponseTooLarge)?;
    let limit = response_limit.min(remaining);
    if limit == 0 {
        return Err(LocalProductError::RemoteResponseTooLarge);
    }
    let url = base.resource_url(relative_path)?;
    let output = create_remote_output(destination)?;
    let observed =
        fetch_remote_file_with_timeout(roots, &url, &output, limit, transfer_timeout_seconds)?;
    *acquired_bytes = acquired_bytes
        .checked_add(observed)
        .filter(|total| *total <= REMOTE_RELEASE_TOTAL_MAX_BYTES)
        .ok_or(LocalProductError::RemoteResponseTooLarge)?;
    Ok(())
}

#[cfg(unix)]
fn fetch_verified_remote_release_file(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    acquisition_root: &std::path::Path,
    file: &ReleaseFileEntry,
    acquired_bytes: &mut u64,
) -> Result<(), LocalProductError> {
    fetch_verified_remote_release_file_with_timeout(
        roots,
        base,
        acquisition_root,
        file,
        acquired_bytes,
        REMOTE_TRANSFER_TIMEOUT_SECONDS,
    )
}

#[cfg(unix)]
fn fetch_verified_remote_release_control_file(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    acquisition_root: &std::path::Path,
    file: &ReleaseFileEntry,
    acquired_bytes: &mut u64,
) -> Result<(), LocalProductError> {
    fetch_verified_remote_release_file_with_timeout(
        roots,
        base,
        acquisition_root,
        file,
        acquired_bytes,
        REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS,
    )
}

#[cfg(unix)]
fn fetch_verified_remote_release_file_with_timeout(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    acquisition_root: &std::path::Path,
    file: &ReleaseFileEntry,
    acquired_bytes: &mut u64,
    transfer_timeout_seconds: &str,
) -> Result<(), LocalProductError> {
    use std::os::unix::fs::PermissionsExt;

    ensure_remote_resource_parent(acquisition_root, &file.relative_path)?;
    let destination = acquisition_root.join(&file.relative_path);
    fetch_remote_resource_with_timeout(
        roots,
        base,
        &file.relative_path,
        &destination,
        REMOTE_RELEASE_FILE_MAX_BYTES,
        acquired_bytes,
        transfer_timeout_seconds,
    )?;
    if openssl_sha256(&roots.openssl, &destination)? != file.sha256 {
        return Err(LocalProductError::ReleaseDigestMismatch);
    }
    let mut permissions = std::fs::symlink_metadata(&destination)
        .map_err(|source| LocalProductError::Io {
            operation: "inspect acquired remote release file",
            source,
        })?
        .permissions();
    permissions.set_mode(file.mode);
    std::fs::set_permissions(&destination, permissions).map_err(|source| LocalProductError::Io {
        operation: "apply signed remote release file mode",
        source,
    })
}

#[cfg(unix)]
fn authenticated_remote_generation_version(
    acquisition_root: &std::path::Path,
    expected_generation_id: &str,
) -> Result<String, LocalProductError> {
    let descriptor = read_bounded_regular_file(
        &acquisition_root.join("generation.meta"),
        LOCAL_GENERATION_MAX_BYTES,
        "read authenticated remote generation descriptor",
        LocalProductError::Descriptor("generation descriptor is too large"),
        LocalProductError::UnsafeSource("generation descriptor must be a regular file"),
    )?;
    if !descriptor.ends_with(b"\n") {
        return Err(LocalProductError::Descriptor(
            "generation descriptor is missing its final newline",
        ));
    }
    let text = std::str::from_utf8(&descriptor)
        .map_err(|_| LocalProductError::Descriptor("generation descriptor is not UTF-8"))?;
    let mut lines = text.lines();
    match lines.next() {
        Some(LOCAL_GENERATION_FORMAT | LEGACY_GENERATION_FORMAT) => {}
        _ => {
            return Err(LocalProductError::Descriptor(
                "generation descriptor format is unsupported",
            ));
        }
    }
    let generation_id = descriptor_field(lines.next(), "generation_id")?;
    m2_generation_state::validate_generation_identity(generation_id, "generation_id")
        .map_err(LocalProductError::StateFormat)?;
    if generation_id != expected_generation_id {
        return Err(LocalProductError::Descriptor(
            "generation descriptor id does not match signed release",
        ));
    }
    let _ = descriptor_field(lines.next(), "upstream_package_identity")?;
    Ok(descriptor_field(lines.next(), "upstream_package_version")?.to_owned())
}

#[cfg(unix)]
fn acquire_remote_release_source(
    roots: &LocalCoreRoots,
    base: &RemoteReleaseBase,
    acquisition_root: &std::path::Path,
    update_key: ReleasePublicKey,
    current_release: &LocalReleaseManifest,
    current_version: &str,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<(), LocalProductError> {
    let mut acquired_bytes = 0;
    fetch_remote_control_resource(
        roots,
        base,
        "release.manifest",
        &acquisition_root.join("release.manifest"),
        LOCAL_RELEASE_MAX_BYTES as u64,
        &mut acquired_bytes,
    )?;
    let (_, parsed_manifest) = read_local_release_manifest(acquisition_root)?;
    fetch_remote_control_resource(
        roots,
        base,
        "release.sig",
        &acquisition_root.join("release.sig"),
        LOCAL_RELEASE_SIGNATURE_MAX_BYTES,
        &mut acquired_bytes,
    )?;
    if parsed_manifest.release_public_key != update_key {
        fetch_remote_control_resource(
            roots,
            base,
            "release-authority.sig",
            &acquisition_root.join("release-authority.sig"),
            LOCAL_RELEASE_SIGNATURE_MAX_BYTES,
            &mut acquired_bytes,
        )?;
    }
    let manifest =
        verify_local_release_control_with_key(acquisition_root, &roots.openssl, update_key)?;
    if !base.matches_generation_identity(&manifest.generation_id)? {
        return Err(LocalProductError::Remote(
            "remote release base does not match signed generation identity",
        ));
    }

    let descriptor = manifest
        .files
        .iter()
        .find(|file| file.relative_path == "generation.meta")
        .ok_or(LocalProductError::Release(
            "release inventory is missing generation descriptor",
        ))?;
    fetch_verified_remote_release_control_file(
        roots,
        base,
        acquisition_root,
        descriptor,
        &mut acquired_bytes,
    )?;
    let candidate_version =
        authenticated_remote_generation_version(acquisition_root, &manifest.generation_id)?;

    if &manifest != current_release {
        if let Some(presentation) = presentation.as_deref_mut() {
            presentation.begin_update(current_version, &candidate_version);
            presentation.transient("⠙", "Downloading signed Termux release...");
        }
    }

    for file in &manifest.files {
        if file.relative_path == "generation.meta" {
            continue;
        }
        fetch_verified_remote_release_file(
            roots,
            base,
            acquisition_root,
            file,
            &mut acquired_bytes,
        )?;
    }
    if let Some(presentation) = presentation {
        if &manifest != current_release {
            presentation.transient("⠹", "Verifying release signature and contents...");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn activate_signed_update_channel_with_hold_policy(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    hold_policy: UpdateHoldPolicy,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<SignedUpdateOutcome, LocalProductError> {
    if let Some(presentation) = presentation.as_deref_mut() {
        presentation.transient("⠋", "Checking for updates...");
    }
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    ensure_real_directory(
        &roots.state_root,
        "inspect Core update state root",
        "Core update state root is not a real directory",
    )?;
    ensure_real_directory(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    ensure_openssl_available(&roots.openssl)?;
    ensure_curl_available(&roots.curl)?;

    let index_url = configured_update_index_url()?;
    let signature_url = format!("{index_url}.sig");
    parse_update_index_url(OsStr::new(&signature_url))?;
    let sequence = REMOTE_ACQUISITION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let acquisition_root = roots
        .state_root
        .join(format!(".update-index-{}-{sequence}", std::process::id()));
    create_remote_acquisition_root(&acquisition_root)?;

    let result = (|| {
        let index_path = acquisition_root.join("update-index");
        let signature_path = acquisition_root.join("update-index.sig");
        let index_output = create_remote_output(&index_path)?;
        fetch_remote_control_file(
            roots,
            &index_url,
            &index_output,
            UPDATE_INDEX_MAX_BYTES as u64,
        )?;
        let signature_output = create_remote_output(&signature_path)?;
        fetch_remote_control_file(
            roots,
            &signature_url,
            &signature_output,
            LOCAL_RELEASE_SIGNATURE_MAX_BYTES,
        )?;
        verify_release_signature_with_key(
            &roots.openssl,
            before.update_key,
            &index_path,
            &signature_path,
        )?;
        let index = read_bounded_regular_file(
            &index_path,
            UPDATE_INDEX_MAX_BYTES,
            "read signed update index",
            LocalProductError::UpdateIndex("update index exceeds its byte bound"),
            LocalProductError::UpdateIndex("update index is not a bounded regular file"),
        )?;
        parse_signed_update_index(&index)
    })();
    let cleanup =
        std::fs::remove_dir_all(&acquisition_root).map_err(|source| LocalProductError::Io {
            operation: "remove update index acquisition directory",
            source,
        });
    let index = match (result, cleanup) {
        (_, Err(err)) => return Err(err),
        (Err(err), Ok(())) => return Err(err),
        (Ok(index), Ok(())) => index,
    };

    if hold_policy == UpdateHoldPolicy::Enforce
        && index.generation_id == before.current
        && before.current_key == before.update_key
    {
        let base = RemoteReleaseBase::parse(OsStr::new(&index.release_base.value))?;
        if !base.matches_generation_identity(&index.generation_id)? {
            return Err(LocalProductError::Remote(
                "remote release base does not match signed generation identity",
            ));
        }
        let _ = rollback_guard::effective_update_hold(roots)?;
        let (_, current_loaded) = verify_installed_local_release(
            roots,
            &before.current,
            before.current_key,
            "active generation descriptor id does not match current",
        )?;
        return Ok(SignedUpdateOutcome::AlreadyCurrent(UpdateTarget {
            generation_id: before.current,
            version: current_loaded.manifest.upstream_package_version.clone(),
        }));
    }

    let outcome = activate_signed_remote_release_outcome_with_hold_policy(
        OsStr::new(&index.release_base.value),
        roots,
        process_env,
        hold_policy,
        presentation,
    )?;
    if outcome.generation_id() != index.generation_id {
        return Err(LocalProductError::UpdateIndex(
            "activated generation does not match update index identity",
        ));
    }
    Ok(outcome)
}

#[cfg(unix)]
fn activate_unified_update_with_hold_policy(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    hold_policy: UpdateHoldPolicy,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<(SignedUpdateOutcome, bool), LocalProductError> {
    match activate_signed_update_channel_with_hold_policy(
        roots,
        process_env,
        hold_policy,
        presentation.as_deref_mut(),
    ) {
        Ok(channel_outcome) => Ok((channel_outcome, false)),
        Err(LocalProductError::RemoteTransportFailed)
            if hold_policy == UpdateHoldPolicy::Enforce =>
        {
            let outcome = activate_local_built_update(roots, process_env, presentation)?;
            Ok((SignedUpdateOutcome::Activated(outcome), true))
        }
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn activate_signed_remote_release_outcome_with_hold_policy(
    base: &OsStr,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    hold_policy: UpdateHoldPolicy,
    mut presentation: Option<&mut UpdatePresentation>,
) -> Result<SignedUpdateOutcome, LocalProductError> {
    let base = RemoteReleaseBase::parse(base)?;
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    ensure_real_directory(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    ensure_openssl_available(&roots.openssl)?;
    ensure_curl_available(&roots.curl)?;
    let (current_release, current_loaded) = verify_installed_local_release(
        roots,
        &before.current,
        before.current_key,
        "active generation descriptor id does not match current",
    )?;
    let sequence = REMOTE_ACQUISITION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let acquisition_root = roots
        .generation_root
        .join(format!(".acquire-{}-{sequence}", std::process::id()));
    create_remote_acquisition_root(&acquisition_root)?;

    let prepared = (|| {
        acquire_remote_release_source(
            roots,
            &base,
            &acquisition_root,
            before.update_key,
            &current_release,
            &current_loaded.manifest.upstream_package_version,
            presentation.as_deref_mut(),
        )?;
        prepare_signed_local_release_with_hold_policy(&acquisition_root, roots, hold_policy)
    })();
    let cleanup =
        std::fs::remove_dir_all(&acquisition_root).map_err(|source| LocalProductError::Io {
            operation: "remove remote acquisition directory",
            source,
        });
    let prepared = match (prepared, cleanup) {
        (_, Err(err)) => return Err(err),
        (Err(err), Ok(())) => return Err(err),
        (Ok(prepared), Ok(())) => prepared,
    };
    match prepared {
        PreparedSignedLocalRelease::AlreadyCurrent(target) => {
            Ok(SignedUpdateOutcome::AlreadyCurrent(target))
        }
        PreparedSignedLocalRelease::Activation(prepared) => {
            activate_prepared_local_release(*prepared, roots, process_env, presentation)
                .map(SignedUpdateOutcome::Activated)
        }
    }
}

#[cfg(unix)]
fn activate_signed_remote_release_outcome(
    base: &OsStr,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: Option<&mut UpdatePresentation>,
) -> Result<SignedUpdateOutcome, LocalProductError> {
    activate_signed_remote_release_outcome_with_hold_policy(
        base,
        roots,
        process_env,
        UpdateHoldPolicy::Enforce,
        presentation,
    )
}

#[cfg(unix)]
fn bootstrap_public_key_path() -> Result<std::path::PathBuf, LocalProductError> {
    Ok(required_absolute_env_path("HOME")?.join(".local/lib/codex/core/release-public-key.pem"))
}

#[cfg(unix)]
static BOOTSTRAP_TRUST_SEED_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[cfg(unix)]
fn verify_bootstrap_trust_seed(
    roots: &LocalCoreRoots,
    destination: &std::path::Path,
    expected_key: ReleasePublicKey,
) -> Result<(), LocalProductError> {
    use std::os::unix::fs::PermissionsExt;

    let actual_key = release_public_key_from_pem(&roots.openssl, destination)?;
    if actual_key != expected_key {
        return Err(LocalProductError::Bootstrap(
            "existing bootstrap trust seed does not match requested key",
        ));
    }
    let metadata =
        std::fs::symlink_metadata(destination).map_err(|source| LocalProductError::Io {
            operation: "inspect bootstrap trust seed",
            source,
        })?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o644);
    if metadata.permissions().mode() & 0o7777 != 0o644 {
        std::fs::set_permissions(destination, permissions).map_err(|source| {
            LocalProductError::Io {
                operation: "set bootstrap trust seed mode",
                source,
            }
        })?;
        let updated =
            std::fs::symlink_metadata(destination).map_err(|source| LocalProductError::Io {
                operation: "inspect bootstrap trust seed mode",
                source,
            })?;
        if !updated.file_type().is_file() || updated.permissions().mode() & 0o7777 != 0o644 {
            return Err(LocalProductError::Bootstrap(
                "bootstrap trust seed mode could not be established",
            ));
        }
    }
    std::fs::File::open(destination)
        .and_then(|file| file.sync_all())
        .map_err(|source| LocalProductError::Io {
            operation: "sync bootstrap trust seed",
            source,
        })?;
    Ok(())
}

#[cfg(unix)]
fn publish_bootstrap_trust_seed(
    roots: &LocalCoreRoots,
    key_source: &std::path::Path,
) -> Result<(), LocalProductError> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    ensure_regular_file(
        key_source,
        "inspect bootstrap public key",
        "bootstrap public key must be a regular non-symlink file",
    )?;
    let expected_key = release_public_key_from_pem(&roots.openssl, key_source)?;
    let destination = bootstrap_public_key_path()?;
    let parent = destination.parent().ok_or(LocalProductError::Bootstrap(
        "bootstrap trust seed has no parent directory",
    ))?;
    std::fs::create_dir_all(parent).map_err(|source| LocalProductError::Io {
        operation: "create bootstrap trust seed parent",
        source,
    })?;
    ensure_real_directory(
        parent,
        "inspect bootstrap trust seed parent",
        "bootstrap trust seed parent must be a real directory",
    )?;

    match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            verify_bootstrap_trust_seed(roots, &destination, expected_key)?;
            return sync_directory(parent);
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect bootstrap trust seed",
                source,
            });
        }
    }

    let sequence = BOOTSTRAP_TRUST_SEED_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".release-public-key.pem.bootstrap-{}-{sequence}",
        std::process::id()
    ));
    let result = (|| {
        let mut input =
            std::fs::File::open(key_source).map_err(|source| LocalProductError::Io {
                operation: "open bootstrap public key",
                source,
            })?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|source| LocalProductError::Io {
                operation: "create bootstrap trust seed temporary",
                source,
            })?;
        copy_bounded_file_contents(
            &mut input,
            &mut output,
            BOOTSTRAP_PUBLIC_KEY_MAX_BYTES,
            "copy bootstrap public key",
            "bootstrap public key exceeds its byte bound",
        )?;
        let mut permissions = output
            .metadata()
            .map_err(|source| LocalProductError::Io {
                operation: "inspect bootstrap trust seed temporary",
                source,
            })?
            .permissions();
        permissions.set_mode(0o644);
        output
            .set_permissions(permissions)
            .map_err(|source| LocalProductError::Io {
                operation: "set bootstrap trust seed temporary mode",
                source,
            })?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync bootstrap trust seed temporary",
            source,
        })?;
        drop(output);
        verify_bootstrap_trust_seed(roots, &temporary, expected_key)?;
        match rename_noreplace(&temporary, &destination) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                std::fs::remove_file(&temporary).map_err(|source| LocalProductError::Io {
                    operation: "remove collided bootstrap trust seed temporary",
                    source,
                })?;
                verify_bootstrap_trust_seed(roots, &destination, expected_key)?;
            }
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "atomically publish bootstrap trust seed",
                    source,
                });
            }
        }
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn bootstrap_self_test(
    roots: &LocalCoreRoots,
    bootstrap_public_key: &std::path::Path,
) -> Result<(), LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    if m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .is_some()
    {
        return Err(LocalProductError::Release(
            "fresh bootstrap requires absent authoritative v3 state",
        ));
    }
    ensure_real_directory_if_present(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;
    ensure_openssl_available(&roots.openssl)?;
    let _ = release_public_key_from_pem(&roots.openssl, bootstrap_public_key)?;
    Ok(())
}

#[cfg(unix)]
fn bootstrap_initial_signed_local_release(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    bootstrap_public_key: &std::path::Path,
    core_artifact: &std::path::Path,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<String, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    if m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .is_some()
    {
        return Err(LocalProductError::Release(
            "fresh bootstrap requires absent authoritative v3 state",
        ));
    }

    let bootstrap_key = release_public_key_from_pem(&roots.openssl, bootstrap_public_key)?;
    let core_digest = authenticated_core_digest(roots, core_artifact)?;
    let (manifest, loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, bootstrap_key)?;
    if manifest.release_public_key != bootstrap_key {
        return Err(LocalProductError::Release(
            "bootstrap release key does not match pinned key",
        ));
    }
    if loaded.manifest.core_artifact_digest != core_digest {
        return Err(LocalProductError::Bootstrap(
            "bootstrap release does not match authenticated Core artifact",
        ));
    }

    ensure_real_directory_tree(
        &roots.generation_root,
        "inspect immutable generation root",
        "immutable generation root is not a real directory",
    )?;

    std::fs::create_dir_all(&roots.config_dir).map_err(|source| LocalProductError::Io {
        operation: "create bootstrap config directory",
        source,
    })?;
    probe_release_candidate(&loaded, roots, process_env)?;
    m2_generation_state::prepare_core_state_paths(&state_paths)
        .map_err(LocalProductError::State)?;
    let after = m2_generation_state::plan_initial_pointer_state_with_key(
        &loaded.generation_id,
        bootstrap_key,
    )
    .map_err(LocalProductError::StateFormat)?;

    let destination = roots.generation_root.join(&loaded.generation_id);
    let mut published_new = false;
    let installed = if generation_path_exists(&destination)? {
        let (installed_manifest, installed) =
            verify_local_release_bundle_with_key(&destination, &roots.openssl, bootstrap_key)?;
        if installed_manifest != manifest {
            return Err(LocalProductError::GenerationCollision);
        }
        sync_complete_generation(&destination, &roots.generation_root)?;
        installed
    } else {
        let generation_id = stage_local_generation(source_dir, &roots.generation_root)?;
        if generation_id != loaded.generation_id {
            return Err(LocalProductError::Descriptor(
                "bootstrap generation descriptor id does not match source",
            ));
        }
        published_new = true;
        match verify_local_release_bundle_with_key(
            &roots.generation_root.join(&generation_id),
            &roots.openssl,
            bootstrap_key,
        ) {
            Ok((installed_manifest, installed)) if installed_manifest == manifest => installed,
            Ok(_) => {
                let _ = std::fs::remove_dir_all(&destination);
                return Err(LocalProductError::Release(
                    "bootstrap installed release does not match authenticated source",
                ));
            }
            Err(err) => {
                let _ = std::fs::remove_dir_all(&destination);
                return Err(err);
            }
        }
    };
    if installed.generation_id != loaded.generation_id {
        if published_new {
            let _ = std::fs::remove_dir_all(&destination);
        }
        return Err(LocalProductError::Descriptor(
            "bootstrap installed generation id does not match authenticated source",
        ));
    }

    m2_generation_state::activate_pointer_state(&state_paths, None, &after)
        .map_err(LocalProductError::State)?;
    Ok(after.current)
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyEntrypointClass {
    Legacy,
    Core,
}

#[cfg(unix)]
fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(unix)]
fn classify_legacy_entrypoint_digest(
    actual_digest: &str,
    expected_legacy_digest: &str,
    core_digest: &str,
) -> Result<LegacyEntrypointClass, LocalProductError> {
    if !is_canonical_sha256(expected_legacy_digest) {
        return Err(LocalProductError::LegacyHandoff(
            "expected legacy entrypoint digest must be exactly 64 lowercase hexadecimal digits",
        ));
    }
    if expected_legacy_digest == core_digest {
        return Err(LocalProductError::LegacyHandoff(
            "expected legacy entrypoint digest must differ from authenticated Core digest",
        ));
    }
    if actual_digest == expected_legacy_digest {
        Ok(LegacyEntrypointClass::Legacy)
    } else if actual_digest == core_digest {
        Ok(LegacyEntrypointClass::Core)
    } else {
        Err(LocalProductError::LegacyHandoff(
            "entrypoint digest matches neither explicit legacy target nor authenticated Core",
        ))
    }
}

#[cfg(unix)]
fn stable_core_entrypoint_path(
    roots: &LocalCoreRoots,
) -> Result<std::path::PathBuf, LocalProductError> {
    let etc_dir = roots
        .resolver_path
        .parent()
        .ok_or(LocalProductError::LegacyHandoff(
            "Core resolver path has no parent directory",
        ))?;
    let prefix = etc_dir.parent().ok_or(LocalProductError::LegacyHandoff(
        "Core resolver path has no Termux prefix",
    ))?;
    Ok(prefix.join("bin/codex"))
}

#[cfg(unix)]
fn read_legacy_handoff_entrypoint(
    roots: &LocalCoreRoots,
    expected_legacy_digest: &str,
    core_digest: &str,
) -> Result<LegacyEntrypointClass, LocalProductError> {
    let destination = stable_core_entrypoint_path(roots)?;
    let parent = destination
        .parent()
        .ok_or(LocalProductError::LegacyHandoff(
            "Core entrypoint has no parent directory",
        ))?;
    ensure_real_directory(
        parent,
        "inspect Core entrypoint parent",
        "Core entrypoint parent must be a real directory",
    )?;
    let metadata = std::fs::symlink_metadata(&destination).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            LocalProductError::LegacyHandoff("legacy handoff requires an existing Codex entrypoint")
        } else {
            LocalProductError::Io {
                operation: "inspect existing Codex entrypoint",
                source,
            }
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(LocalProductError::LegacyHandoff(
            "existing Codex entrypoint must be a regular non-symlink file",
        ));
    }
    let actual_digest = openssl_sha256(&roots.openssl, &destination)?;
    classify_legacy_entrypoint_digest(&actual_digest, expected_legacy_digest, core_digest)
}

#[cfg(unix)]
fn bootstrap_handoff_self_test(
    roots: &LocalCoreRoots,
    bootstrap_public_key: &std::path::Path,
) -> Result<(), LocalProductError> {
    ensure_openssl_available(&roots.openssl)?;
    let _ = release_public_key_from_pem(&roots.openssl, bootstrap_public_key)?;
    Ok(())
}

#[cfg(unix)]
fn verify_handoff_state_matches_candidate(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
    supplied_key: ReleasePublicKey,
    core_digest: &str,
) -> Result<m2_generation_state::GenerationPointerState, LocalProductError> {
    if state.update_key != supplied_key || state.current_key != supplied_key {
        return Err(LocalProductError::LegacyHandoff(
            "prepared handoff state does not match supplied bootstrap key",
        ));
    }
    if state.previous.is_some() || state.previous_key.is_some() {
        return Err(LocalProductError::LegacyHandoff(
            "prepared handoff state must not contain a previous generation",
        ));
    }
    let (source_release, source_loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, supplied_key)?;
    if source_release.release_public_key != state.current_key
        || source_loaded.generation_id != state.current
    {
        return Err(LocalProductError::LegacyHandoff(
            "signed handoff release does not match prepared generation state",
        ));
    }
    if source_loaded.manifest.core_artifact_digest != core_digest {
        return Err(LocalProductError::LegacyHandoff(
            "signed handoff release does not match authenticated Core artifact",
        ));
    }
    let (installed_release, installed_loaded) = verify_installed_local_release(
        roots,
        &state.current,
        state.current_key,
        "prepared handoff generation descriptor id does not match current",
    )?;
    if installed_release != source_release
        || installed_loaded.generation_id != source_loaded.generation_id
    {
        return Err(LocalProductError::LegacyHandoff(
            "installed prepared generation does not match signed handoff release",
        ));
    }
    Ok(state.clone())
}

#[cfg(unix)]
fn bootstrap_handoff_prepare_state(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    bootstrap_public_key: &std::path::Path,
    core_artifact: &std::path::Path,
    core_digest: &str,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<m2_generation_state::GenerationPointerState, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let supplied_key = release_public_key_from_pem(&roots.openssl, bootstrap_public_key)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?;
    if let Some(state) = before {
        return verify_handoff_state_matches_candidate(
            source_dir,
            roots,
            &state,
            supplied_key,
            core_digest,
        );
    }

    let (source_release, source_loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, supplied_key)?;
    if source_release.release_public_key != supplied_key {
        return Err(LocalProductError::LegacyHandoff(
            "bootstrap release key does not match supplied bootstrap key",
        ));
    }
    if source_loaded.manifest.core_artifact_digest != core_digest {
        return Err(LocalProductError::LegacyHandoff(
            "signed handoff release does not match authenticated Core artifact",
        ));
    }
    bootstrap_initial_signed_local_release(
        source_dir,
        roots,
        bootstrap_public_key,
        core_artifact,
        process_env,
    )?;
    let state = m2_generation_state::read_pointer_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::LegacyHandoff(
            "initial handoff activation did not publish authoritative state",
        ))?;
    verify_handoff_state_matches_candidate(source_dir, roots, &state, supplied_key, core_digest)
}

#[cfg(unix)]
static LEGACY_HANDOFF_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(unix)]
fn create_handoff_entrypoint_temp(
    roots: &LocalCoreRoots,
    core_artifact: &std::path::Path,
    core_digest: &str,
) -> Result<std::path::PathBuf, LocalProductError> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    inspect_authenticated_core_artifact(core_artifact)?;
    let destination = stable_core_entrypoint_path(roots)?;
    let parent = destination
        .parent()
        .ok_or(LocalProductError::LegacyHandoff(
            "Core entrypoint has no parent directory",
        ))?;
    ensure_real_directory(
        parent,
        "inspect Core entrypoint parent",
        "Core entrypoint parent must be a real directory",
    )?;
    let sequence = LEGACY_HANDOFF_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".codex.legacy-handoff-{}-{sequence}",
        std::process::id()
    ));
    let result = (|| {
        let mut input =
            std::fs::File::open(core_artifact).map_err(|source| LocalProductError::Io {
                operation: "open authenticated Core artifact",
                source,
            })?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|source| LocalProductError::Io {
                operation: "create private Core entrypoint temporary",
                source,
            })?;
        copy_bounded_file_contents(
            &mut input,
            &mut output,
            CORE_ARTIFACT_MAX_BYTES,
            "copy authenticated Core artifact",
            "authenticated Core artifact exceeds its byte bound",
        )?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync private Core entrypoint temporary",
            source,
        })?;
        let mut permissions = output
            .metadata()
            .map_err(|source| LocalProductError::Io {
                operation: "inspect private Core entrypoint temporary",
                source,
            })?
            .permissions();
        permissions.set_mode(0o755);
        output
            .set_permissions(permissions)
            .map_err(|source| LocalProductError::Io {
                operation: "set private Core entrypoint temporary mode",
                source,
            })?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "resync private Core entrypoint temporary",
            source,
        })?;
        drop(output);
        let metadata =
            std::fs::symlink_metadata(&temporary).map_err(|source| LocalProductError::Io {
                operation: "inspect prepared Core entrypoint temporary",
                source,
            })?;
        if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o7777 != 0o755 {
            return Err(LocalProductError::LegacyHandoff(
                "private Core entrypoint temporary is not an executable 0755 regular file",
            ));
        }
        if openssl_sha256(&roots.openssl, &temporary)? != core_digest {
            return Err(LocalProductError::LegacyHandoff(
                "private Core entrypoint temporary digest does not match authenticated Core",
            ));
        }
        Ok(temporary.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn rename_noreplace(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
    #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
    unsafe extern "C" {
        fn renameat2(
            olddirfd: i32,
            oldpath: *const std::ffi::c_char,
            newdirfd: i32,
            newpath: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    let source = CString::new(source.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "source path contains NUL")
    })?;
    let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination path contains NUL",
        )
    })?;
    // Android API 24 does not export the renameat2 libc symbol, although the arm64
    // kernel ABI provides the syscall. Use bionic's long-lived syscall wrapper there
    // so the release target keeps RENAME_NOREPLACE semantics without raising minSdk.
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    let result = unsafe {
        unsafe extern "C" {
            fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
        }
        const SYS_RENAMEAT2_AARCH64: std::ffi::c_long = 276;
        syscall(
            SYS_RENAMEAT2_AARCH64,
            AT_FDCWD as std::ffi::c_long,
            source.as_ptr() as std::ffi::c_long,
            AT_FDCWD as std::ffi::c_long,
            destination.as_ptr() as std::ffi::c_long,
            RENAME_NOREPLACE as std::ffi::c_long,
        ) as i32
    };
    #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
    // SAFETY: both C strings are NUL-terminated and live for the duration of the call.
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            source.as_ptr(),
            AT_FDCWD,
            destination.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn sync_directory(parent: &std::path::Path) -> Result<(), LocalProductError> {
    let directory = std::fs::File::open(parent).map_err(|source| LocalProductError::Io {
        operation: "open directory for sync",
        source,
    })?;
    directory
        .sync_all()
        .map_err(|source| LocalProductError::Io {
            operation: "sync directory",
            source,
        })
}

#[cfg(unix)]
const UPDATE_HOLD_FORMAT: &str = "codex-update-hold-v1";
#[cfg(unix)]
const UPDATE_HOLD_FILE: &str = "update-hold";
#[cfg(unix)]
const UPDATE_HOLD_MAX_BYTES: usize = 4096;
#[cfg(unix)]
static UPDATE_HOLD_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateHoldRecord {
    generation_id: String,
    release_sequence: u64,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateHoldPolicy {
    Enforce,
    ForceHeld,
}

#[cfg(unix)]
fn update_hold_path(roots: &LocalCoreRoots) -> std::path::PathBuf {
    roots.state_root.join(UPDATE_HOLD_FILE)
}

#[cfg(unix)]
fn render_update_hold(record: &UpdateHoldRecord) -> String {
    format!(
        "{UPDATE_HOLD_FORMAT}\ngeneration_id\t{}\nrelease_sequence\t{}\n",
        record.generation_id, record.release_sequence
    )
}

#[cfg(unix)]
fn parse_update_hold(bytes: &[u8]) -> Result<UpdateHoldRecord, LocalProductError> {
    if bytes.len() > UPDATE_HOLD_MAX_BYTES {
        return Err(LocalProductError::UpdateHold(
            "update hold exceeds its byte bound",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LocalProductError::UpdateHold("update hold is not UTF-8"))?;
    if text.contains('\r') || !text.ends_with('\n') {
        return Err(LocalProductError::UpdateHold(
            "update hold format is invalid",
        ));
    }
    let lines: Vec<&str> = text
        .strip_suffix('\n')
        .unwrap_or(text)
        .split('\n')
        .collect();
    if lines.len() != 3 || lines[0] != UPDATE_HOLD_FORMAT {
        return Err(LocalProductError::UpdateHold(
            "update hold format is invalid",
        ));
    }
    let generation_id =
        lines[1]
            .strip_prefix("generation_id\t")
            .ok_or(LocalProductError::UpdateHold(
                "update hold generation record is invalid",
            ))?;
    m2_generation_state::validate_generation_identity(generation_id, "update hold generation_id")
        .map_err(LocalProductError::StateFormat)?;
    let sequence_text =
        lines[2]
            .strip_prefix("release_sequence\t")
            .ok_or(LocalProductError::UpdateHold(
                "update hold sequence record is invalid",
            ))?;
    let release_sequence = sequence_text
        .parse::<u64>()
        .map_err(|_| LocalProductError::UpdateHold("update hold release sequence is invalid"))?;
    if release_sequence == 0 || release_sequence.to_string() != sequence_text {
        return Err(LocalProductError::UpdateHold(
            "update hold release sequence is invalid",
        ));
    }
    Ok(UpdateHoldRecord {
        generation_id: generation_id.to_owned(),
        release_sequence,
    })
}

#[cfg(unix)]
fn read_update_hold(roots: &LocalCoreRoots) -> Result<Option<UpdateHoldRecord>, LocalProductError> {
    let path = update_hold_path(roots);
    match std::fs::symlink_metadata(&path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect update hold",
                source,
            })
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(LocalProductError::UpdateHold(
                "update hold must be a regular file",
            ))
        }
        Ok(_) => {}
    }
    let bytes = read_bounded_regular_file(
        &path,
        UPDATE_HOLD_MAX_BYTES,
        "read update hold",
        LocalProductError::UpdateHold("update hold exceeds its byte bound"),
        LocalProductError::UpdateHold("update hold must be a regular file"),
    )?;
    parse_update_hold(&bytes).map(Some)
}

#[cfg(unix)]
fn write_update_hold_locked(
    roots: &LocalCoreRoots,
    record: &UpdateHoldRecord,
) -> Result<(), LocalProductError> {
    use std::io::Write as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    ensure_real_directory(
        &roots.state_root,
        "inspect Core state root for update hold",
        "Core state root for update hold is not a real directory",
    )?;
    let destination = update_hold_path(roots);
    match std::fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => {
            return Err(LocalProductError::UpdateHold(
                "update hold destination has an unsafe file type",
            ))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect update hold destination",
                source,
            })
        }
    }
    let sequence = UPDATE_HOLD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = roots.state_root.join(format!(
        ".update-hold.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|source| LocalProductError::Io {
                operation: "create update hold temporary",
                source,
            })?;
        output
            .write_all(render_update_hold(record).as_bytes())
            .map_err(|source| LocalProductError::Io {
                operation: "write update hold temporary",
                source,
            })?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync update hold temporary",
            source,
        })?;
        let metadata =
            std::fs::symlink_metadata(&temporary).map_err(|source| LocalProductError::Io {
                operation: "inspect update hold temporary",
                source,
            })?;
        if !metadata.file_type().is_file() {
            return Err(LocalProductError::UpdateHold(
                "update hold temporary is not a regular file",
            ));
        }
        if metadata.permissions().mode() & 0o7777 != 0o600 {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&temporary, permissions).map_err(|source| {
                LocalProductError::Io {
                    operation: "set update hold temporary mode",
                    source,
                }
            })?;
        }
        drop(output);
        match std::fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => {
                return Err(LocalProductError::UpdateHold(
                    "update hold destination has an unsafe file type",
                ))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "reinspect update hold destination",
                    source,
                })
            }
        }
        std::fs::rename(&temporary, &destination).map_err(|source| LocalProductError::Io {
            operation: "replace update hold",
            source,
        })?;
        sync_directory(&roots.state_root)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn finalize_validated_update_hold_locked(
    roots: &LocalCoreRoots,
    validated_hold: Option<&UpdateHoldRecord>,
    activated_sequence: u64,
) -> Result<(), LocalProductError> {
    let Some(hold) = validated_hold else {
        return Ok(());
    };
    let observed = read_update_hold(roots)?;
    if observed.as_ref().is_some_and(|observed| observed != hold) {
        return Err(LocalProductError::UpdateHold(
            "update hold changed during activation",
        ));
    }
    if activated_sequence > hold.release_sequence {
        if observed.is_some() {
            std::fs::remove_file(update_hold_path(roots)).map_err(|source| {
                LocalProductError::Io {
                    operation: "remove superseded update hold",
                    source,
                }
            })?;
        }
        rollback_guard::remove_guard_if_present(roots)?;
        return sync_directory(&roots.state_root);
    }
    if activated_sequence == hold.release_sequence {
        if observed.is_none() {
            write_update_hold_locked(roots, hold)?;
        }
        rollback_guard::remove_guard_if_present(roots)?;
        return sync_directory(&roots.state_root);
    }
    Err(LocalProductError::UpdateHold(
        "activation did not supersede the validated rollback hold",
    ))
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CoreRollbackRecord {
    generation_id: String,
    digest: String,
}

#[cfg(unix)]
static CORE_ROLLBACK_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(unix)]
fn core_rollback_path(roots: &LocalCoreRoots) -> std::path::PathBuf {
    roots.state_root.join(CORE_ROLLBACK_FILE)
}

#[cfg(unix)]
fn core_rollback_meta_path(roots: &LocalCoreRoots) -> std::path::PathBuf {
    roots.state_root.join(CORE_ROLLBACK_META_FILE)
}

#[cfg(unix)]
fn inspect_core_entrypoint_source(
    path: &std::path::Path,
    operation: &'static str,
) -> Result<std::fs::Metadata, LocalProductError> {
    use std::os::unix::fs::PermissionsExt as _;

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|source| LocalProductError::Io { operation, source })?;
    if !metadata.file_type().is_file() {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint source must be a regular non-symlink file",
        ));
    }
    if metadata.len() > CORE_ARTIFACT_MAX_BYTES {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint source exceeds its byte bound",
        ));
    }
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint source is not executable",
        ));
    }
    Ok(metadata)
}

#[cfg(unix)]
fn copy_core_entrypoint_to_state_temp(
    roots: &LocalCoreRoots,
    source: &std::path::Path,
    label: &'static str,
) -> Result<std::path::PathBuf, LocalProductError> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    inspect_core_entrypoint_source(source, "inspect Core entrypoint snapshot source")?;
    ensure_real_directory(
        &roots.state_root,
        "inspect Core state root for entrypoint snapshot",
        "Core state root must be a real directory",
    )?;
    let sequence = CORE_ROLLBACK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = roots
        .state_root
        .join(format!(".{label}-{}-{sequence}", std::process::id()));
    let result = (|| {
        let mut input = std::fs::File::open(source).map_err(|source| LocalProductError::Io {
            operation: "open Core entrypoint snapshot source",
            source,
        })?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|source| LocalProductError::Io {
                operation: "create Core entrypoint snapshot temporary",
                source,
            })?;
        copy_bounded_file_contents(
            &mut input,
            &mut output,
            CORE_ARTIFACT_MAX_BYTES,
            "copy Core entrypoint snapshot",
            "Core entrypoint snapshot exceeds its byte bound",
        )?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync Core entrypoint snapshot temporary",
            source,
        })?;
        let mut permissions = output
            .metadata()
            .map_err(|source| LocalProductError::Io {
                operation: "inspect Core entrypoint snapshot temporary",
                source,
            })?
            .permissions();
        permissions.set_mode(0o755);
        output
            .set_permissions(permissions)
            .map_err(|source| LocalProductError::Io {
                operation: "set Core entrypoint snapshot temporary mode",
                source,
            })?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "resync Core entrypoint snapshot temporary",
            source,
        })?;
        drop(output);
        let metadata =
            std::fs::symlink_metadata(&temporary).map_err(|source| LocalProductError::Io {
                operation: "inspect prepared Core entrypoint snapshot",
                source,
            })?;
        if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o7777 != 0o755 {
            return Err(LocalProductError::CoreEntrypoint(
                "Core entrypoint snapshot temporary is not an executable 0755 file",
            ));
        }
        Ok(temporary.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn snapshot_core_entrypoint_to_rollback(
    roots: &LocalCoreRoots,
    generation_id: &str,
) -> Result<CoreRollbackRecord, LocalProductError> {
    use std::io::Write as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    m2_generation_state::validate_generation_identity(generation_id, "rollback generation")
        .map_err(LocalProductError::StateFormat)?;
    let source = stable_core_entrypoint_path(roots)?;
    let metadata = inspect_core_entrypoint_source(&source, "inspect current Core entrypoint")?;
    if metadata.permissions().mode() & 0o7777 != 0o755 {
        return Err(LocalProductError::CoreEntrypoint(
            "current Core entrypoint must have mode 0755",
        ));
    }
    let digest = openssl_sha256(&roots.openssl, &source)?;
    let temporary = copy_core_entrypoint_to_state_temp(roots, &source, "core-rollback")?;
    let destination = core_rollback_path(roots);
    let metadata_temporary = roots.state_root.join(format!(
        ".{CORE_ROLLBACK_META_FILE}.{}-{}",
        std::process::id(),
        CORE_ROLLBACK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        std::fs::rename(&temporary, &destination).map_err(|source| LocalProductError::Io {
            operation: "publish Core entrypoint rollback snapshot",
            source,
        })?;
        let bytes = format!(
            "format={CORE_ROLLBACK_FORMAT}\ngeneration_id={generation_id}\nsha256={digest}\n"
        );
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&metadata_temporary)
            .map_err(|source| LocalProductError::Io {
                operation: "create Core entrypoint rollback metadata temporary",
                source,
            })?;
        file.write_all(bytes.as_bytes())
            .map_err(|source| LocalProductError::Io {
                operation: "write Core entrypoint rollback metadata",
                source,
            })?;
        file.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync Core entrypoint rollback metadata",
            source,
        })?;
        drop(file);
        let mut permissions = std::fs::symlink_metadata(&metadata_temporary)
            .map_err(|source| LocalProductError::Io {
                operation: "inspect Core entrypoint rollback metadata temporary",
                source,
            })?
            .permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&metadata_temporary, permissions).map_err(|source| {
            LocalProductError::Io {
                operation: "set Core entrypoint rollback metadata mode",
                source,
            }
        })?;
        std::fs::File::open(&metadata_temporary)
            .and_then(|file| file.sync_all())
            .map_err(|source| LocalProductError::Io {
                operation: "resync Core entrypoint rollback metadata",
                source,
            })?;
        std::fs::rename(&metadata_temporary, core_rollback_meta_path(roots)).map_err(|source| {
            LocalProductError::Io {
                operation: "publish Core entrypoint rollback metadata",
                source,
            }
        })?;
        sync_directory(&roots.state_root)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
        let _ = std::fs::remove_file(&metadata_temporary);
    }
    result.map(|_| CoreRollbackRecord {
        generation_id: generation_id.to_owned(),
        digest,
    })
}

#[cfg(unix)]
fn read_core_entrypoint_rollback(
    roots: &LocalCoreRoots,
) -> Result<CoreRollbackRecord, LocalProductError> {
    let bytes = read_bounded_regular_file(
        &core_rollback_meta_path(roots),
        CORE_ROLLBACK_META_MAX_BYTES,
        "read Core entrypoint rollback metadata",
        LocalProductError::CoreEntrypoint("Core entrypoint rollback metadata is too large"),
        LocalProductError::CoreEntrypoint(
            "Core entrypoint rollback metadata must be a regular file",
        ),
    )?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        LocalProductError::CoreEntrypoint("Core entrypoint rollback metadata is not UTF-8")
    })?;
    let mut lines = text.lines();
    if lines.next() != Some(&format!("format={CORE_ROLLBACK_FORMAT}")) {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint rollback metadata format is invalid",
        ));
    }
    let generation_id = lines
        .next()
        .and_then(|line| line.strip_prefix("generation_id="))
        .filter(|value| !value.is_empty())
        .ok_or(LocalProductError::CoreEntrypoint(
            "Core entrypoint rollback generation is invalid",
        ))?;
    m2_generation_state::validate_generation_identity(generation_id, "rollback generation")
        .map_err(LocalProductError::StateFormat)?;
    let digest = lines
        .next()
        .and_then(|line| line.strip_prefix("sha256="))
        .filter(|value| is_canonical_sha256(value))
        .ok_or(LocalProductError::CoreEntrypoint(
            "Core entrypoint rollback digest is invalid",
        ))?;
    if lines.next().is_some() || !text.ends_with('\n') {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint rollback metadata is malformed",
        ));
    }
    let rollback_path = core_rollback_path(roots);
    let metadata = inspect_core_entrypoint_source(
        &rollback_path,
        "inspect Core entrypoint rollback snapshot",
    )?;
    use std::os::unix::fs::PermissionsExt as _;
    if metadata.permissions().mode() & 0o7777 != 0o755
        || openssl_sha256(&roots.openssl, &rollback_path)? != digest
    {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint rollback snapshot does not match its metadata",
        ));
    }
    Ok(CoreRollbackRecord {
        generation_id: generation_id.to_owned(),
        digest: digest.to_owned(),
    })
}

#[cfg(unix)]
fn install_core_entrypoint(
    roots: &LocalCoreRoots,
    source: &std::path::Path,
    expected_digest: &str,
) -> Result<(), LocalProductError> {
    inspect_core_entrypoint_source(source, "inspect Core entrypoint replacement source")?;
    if openssl_sha256(&roots.openssl, source)? != expected_digest {
        return Err(LocalProductError::CoreEntrypoint(
            "Core entrypoint replacement source digest does not match its binding",
        ));
    }
    let destination = stable_core_entrypoint_path(roots)?;
    let parent = destination
        .parent()
        .ok_or(LocalProductError::CoreEntrypoint(
            "Core entrypoint has no parent directory",
        ))?;
    ensure_real_directory(
        parent,
        "inspect Core entrypoint parent",
        "Core entrypoint parent must be a real directory",
    )?;
    match std::fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => {
            return Err(LocalProductError::CoreEntrypoint(
                "stable Core entrypoint must be a regular non-symlink file",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect stable Core entrypoint",
                source,
            })
        }
    }
    let temporary = create_handoff_entrypoint_temp(roots, source, expected_digest)?;
    let result = (|| {
        std::fs::rename(&temporary, &destination).map_err(|source| LocalProductError::Io {
            operation: "atomically replace coordinated Core entrypoint",
            source,
        })?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveCoreEntrypointBinding {
    generation_id: String,
    digest: String,
}

fn active_core_entrypoint_binding_with_state(
    roots: &LocalCoreRoots,
    loaded: &LoadedLocalGeneration,
    authoritative_state: &m2_generation_state::GenerationPointerState,
) -> Result<Option<ActiveCoreEntrypointBinding>, LocalProductError> {
    use std::os::unix::fs::PermissionsExt as _;

    if loaded.core_path.is_none() {
        return Ok(None);
    }
    let destination = stable_core_entrypoint_path(roots)?;
    let metadata = inspect_core_entrypoint_source(&destination, "inspect active Core entrypoint")?;
    if metadata.permissions().mode() & 0o7777 != 0o755 {
        return Err(LocalProductError::CoreEntrypoint(
            "active Core entrypoint mode is invalid",
        ));
    }
    let observed_digest = openssl_sha256(&roots.openssl, &destination)?;
    if observed_digest == loaded.manifest.core_artifact_digest {
        return Ok(Some(ActiveCoreEntrypointBinding {
            generation_id: loaded.generation_id.clone(),
            digest: observed_digest,
        }));
    }
    if let Some((generation_id, digest)) = rollback_guard::guarded_core_binding_with_state(
        roots,
        authoritative_state,
        loaded,
        &observed_digest,
    )? {
        return Ok(Some(ActiveCoreEntrypointBinding {
            generation_id,
            digest,
        }));
    }
    Err(LocalProductError::CoreEntrypoint(
        "active generation Core does not match the stable Core entrypoint",
    ))
}

fn verify_active_core_entrypoint_pair_with_state(
    roots: &LocalCoreRoots,
    loaded: &LoadedLocalGeneration,
    state: &m2_generation_state::GenerationPointerState,
) -> Result<(), LocalProductError> {
    active_core_entrypoint_binding_with_state(roots, loaded, state).map(|_| ())
}

#[cfg(unix)]
fn verify_fresh_core_entrypoint(
    roots: &LocalCoreRoots,
    destination: &std::path::Path,
    core_digest: &str,
) -> Result<(), LocalProductError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::symlink_metadata(destination).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            LocalProductError::Bootstrap("fresh Core entrypoint is absent")
        } else {
            LocalProductError::Io {
                operation: "inspect fresh Core entrypoint",
                source,
            }
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(LocalProductError::Bootstrap(
            "fresh Core entrypoint must be a regular non-symlink file",
        ));
    }
    if openssl_sha256(&roots.openssl, destination)? != core_digest {
        return Err(LocalProductError::Bootstrap(
            "fresh Core entrypoint differs from authenticated Core artifact",
        ));
    }
    if metadata.permissions().mode() & 0o7777 != 0o755 {
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(destination, permissions).map_err(|source| {
            LocalProductError::Io {
                operation: "set fresh Core entrypoint mode",
                source,
            }
        })?;
        let updated =
            std::fs::symlink_metadata(destination).map_err(|source| LocalProductError::Io {
                operation: "inspect fresh Core entrypoint mode",
                source,
            })?;
        if !updated.file_type().is_file() || updated.permissions().mode() & 0o7777 != 0o755 {
            return Err(LocalProductError::Bootstrap(
                "fresh Core entrypoint mode could not be established",
            ));
        }
        std::fs::File::open(destination)
            .and_then(|file| file.sync_all())
            .map_err(|source| LocalProductError::Io {
                operation: "sync fresh Core entrypoint after final mode",
                source,
            })?;
    }
    Ok(())
}

#[cfg(unix)]
fn publish_fresh_core_entrypoint(
    roots: &LocalCoreRoots,
    core_artifact: &std::path::Path,
) -> Result<(), LocalProductError> {
    use std::os::unix::fs::PermissionsExt;

    let core_metadata = inspect_authenticated_core_artifact(core_artifact)?;
    if core_metadata.permissions().mode() & 0o7777 != 0o755 {
        return Err(LocalProductError::Bootstrap(
            "authenticated Core artifact must have mode 0755",
        ));
    }
    let core_digest = authenticated_core_digest(roots, core_artifact)?;
    let destination = stable_core_entrypoint_path(roots)?;
    let parent = destination.parent().ok_or(LocalProductError::Bootstrap(
        "Core entrypoint has no parent directory",
    ))?;
    ensure_real_directory(
        parent,
        "inspect Core entrypoint parent",
        "Core entrypoint parent must be a real directory",
    )?;

    match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            verify_fresh_core_entrypoint(roots, &destination, &core_digest)?;
            return sync_directory(parent);
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect fresh Core entrypoint",
                source,
            });
        }
    }

    let temporary = create_handoff_entrypoint_temp(roots, core_artifact, &core_digest)?;
    let result = (|| {
        match rename_noreplace(&temporary, &destination) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                std::fs::remove_file(&temporary).map_err(|source| LocalProductError::Io {
                    operation: "remove collided fresh Core entrypoint temporary",
                    source,
                })?;
                verify_fresh_core_entrypoint(roots, &destination, &core_digest)?;
            }
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "atomically publish fresh Core entrypoint",
                    source,
                });
            }
        }
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn commit_handoff_entrypoint(
    roots: &LocalCoreRoots,
    core_artifact: &std::path::Path,
    expected_legacy_digest: &str,
    core_digest: &str,
) -> Result<(), LocalProductError> {
    let destination = stable_core_entrypoint_path(roots)?;
    let parent = destination
        .parent()
        .ok_or(LocalProductError::LegacyHandoff(
            "Core entrypoint has no parent directory",
        ))?;
    let current = read_legacy_handoff_entrypoint(roots, expected_legacy_digest, core_digest)?;
    if current == LegacyEntrypointClass::Core {
        return sync_directory(parent);
    }
    let temporary = create_handoff_entrypoint_temp(roots, core_artifact, core_digest)?;
    let result = (|| {
        let current = read_legacy_handoff_entrypoint(roots, expected_legacy_digest, core_digest)?;
        if current == LegacyEntrypointClass::Core {
            std::fs::remove_file(&temporary).map_err(|source| LocalProductError::Io {
                operation: "remove obsolete Core entrypoint temporary",
                source,
            })?;
            return sync_directory(parent);
        }
        std::fs::rename(&temporary, &destination).map_err(|source| LocalProductError::Io {
            operation: "atomically replace legacy Codex entrypoint",
            source,
        })?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn bootstrap_legacy_handoff(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    bootstrap_public_key: &std::path::Path,
    core_artifact: &std::path::Path,
    expected_legacy_digest: &str,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<String, LocalProductError> {
    ensure_openssl_available(&roots.openssl)?;
    let core_metadata = inspect_authenticated_core_artifact(core_artifact)?;
    use std::os::unix::fs::PermissionsExt;
    if core_metadata.permissions().mode() & 0o7777 != 0o755 {
        return Err(LocalProductError::LegacyHandoff(
            "authenticated Core artifact must have mode 0755",
        ));
    }
    let core_digest = authenticated_core_digest(roots, core_artifact)?;
    let _ = read_legacy_handoff_entrypoint(roots, expected_legacy_digest, &core_digest)?;
    bootstrap_handoff_self_test(roots, bootstrap_public_key)?;
    let state = bootstrap_handoff_prepare_state(
        source_dir,
        roots,
        bootstrap_public_key,
        core_artifact,
        &core_digest,
        process_env,
    )?;
    #[cfg(test)]
    if std::env::var_os("CODEX_TEST_LEGACY_HANDOFF_STOP_BEFORE_COMMIT").as_deref()
        == Some(OsStr::new("1"))
    {
        return Err(LocalProductError::LegacyHandoff(
            "test interruption before legacy entrypoint commit",
        ));
    }
    commit_handoff_entrypoint(roots, core_artifact, expected_legacy_digest, &core_digest)?;
    Ok(state.current)
}

#[cfg(unix)]
fn run_internal_bootstrap_mode() -> Option<i32> {
    let mode = std::env::var_os(INTERNAL_BOOTSTRAP_MODE_ENV)?;
    let roots = match LocalCoreRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            eprintln!("codex bootstrap: {err}");
            return Some(1);
        }
    };
    let supplied_key_mode = mode == OsStr::new("self-test")
        || mode == OsStr::new("activate")
        || mode == OsStr::new("handoff-self-test")
        || mode == OsStr::new("handoff")
        || mode == OsStr::new("publish-bootstrap-pin");
    let bootstrap_public_key = match if supplied_key_mode {
        required_absolute_env_path(INTERNAL_BOOTSTRAP_KEY_ENV)
    } else {
        bootstrap_public_key_path()
    } {
        Ok(path) => path,
        Err(err) => {
            eprintln!("codex bootstrap: {err}");
            return Some(1);
        }
    };
    let result = if mode == OsStr::new("self-test") {
        if std::env::var_os(INTERNAL_BOOTSTRAP_SOURCE_ENV).is_some() {
            Err(LocalProductError::Release(
                "bootstrap self-test does not accept a release source",
            ))
        } else {
            bootstrap_self_test(&roots, &bootstrap_public_key).map(|_| String::new())
        }
    } else if mode == OsStr::new("handoff-self-test") {
        if std::env::var_os(INTERNAL_BOOTSTRAP_SOURCE_ENV).is_some() {
            Err(LocalProductError::LegacyHandoff(
                "legacy handoff self-test does not accept a release source",
            ))
        } else {
            bootstrap_handoff_self_test(&roots, &bootstrap_public_key).map(|_| String::new())
        }
    } else if mode == OsStr::new("publish-bootstrap-pin") {
        publish_bootstrap_trust_seed(&roots, &bootstrap_public_key).map(|_| String::new())
    } else if mode == OsStr::new("publish-fresh-entrypoint") {
        match required_absolute_env_path(INTERNAL_BOOTSTRAP_CORE_ENV) {
            Ok(core) => publish_fresh_core_entrypoint(&roots, &core).map(|_| String::new()),
            Err(err) => Err(err),
        }
    } else if mode == OsStr::new("activate") {
        match (
            required_absolute_env_path(INTERNAL_BOOTSTRAP_SOURCE_ENV),
            required_absolute_env_path(INTERNAL_BOOTSTRAP_CORE_ENV),
        ) {
            (Ok(source), Ok(core)) => {
                let process_env = capture_termux_process_env();
                bootstrap_initial_signed_local_release(
                    &source,
                    &roots,
                    &bootstrap_public_key,
                    &core,
                    &process_env,
                )
            }
            (Err(err), _) | (_, Err(err)) => Err(err),
        }
    } else if mode == OsStr::new("handoff") {
        match (
            required_absolute_env_path(INTERNAL_BOOTSTRAP_SOURCE_ENV),
            required_absolute_env_path(INTERNAL_BOOTSTRAP_CORE_ENV),
            std::env::var(INTERNAL_BOOTSTRAP_EXPECTED_ENTRYPOINT_DIGEST_ENV).map_err(|_| {
                LocalProductError::LegacyHandoff(
                    "expected legacy entrypoint digest is missing or not Unicode",
                )
            }),
        ) {
            (Ok(source), Ok(core), Ok(expected)) => {
                let process_env = capture_termux_process_env();
                bootstrap_legacy_handoff(
                    &source,
                    &roots,
                    &bootstrap_public_key,
                    &core,
                    &expected,
                    &process_env,
                )
            }
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => Err(err),
        }
    } else {
        eprintln!("codex bootstrap: unsupported internal bootstrap mode");
        return Some(2);
    };
    match result {
        Ok(generation_id) => {
            if !generation_id.is_empty() {
                if mode == OsStr::new("handoff") {
                    println!("completed legacy handoff to local generation {generation_id}");
                } else {
                    println!("activated initial local generation {generation_id}");
                }
            }
            Some(0)
        }
        Err(err) => {
            eprintln!("codex bootstrap: {err}");
            Some(1)
        }
    }
}

#[cfg(unix)]
fn rollback_signed_local_release(
    roots: &LocalCoreRoots,
) -> Result<ActivatedUpdate, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    let after = m2_generation_state::plan_rollback_pointer_state(&before)
        .map_err(LocalProductError::StateFormat)?;
    let (current_release, current_loaded) = verify_installed_local_release(
        roots,
        &before.current,
        before.current_key,
        "rollback current generation descriptor id does not match current",
    )?;
    let (target_release, target_loaded) = verify_installed_local_release(
        roots,
        &after.current,
        after.current_key,
        "rollback generation descriptor id does not match previous",
    )?;
    let lock = m2_generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let authoritative =
        m2_generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
    if authoritative.as_ref() != Some(&before) {
        return Err(LocalProductError::State(
            m2_generation_state::ActivationTransactionError::StaleAuthoritativeState,
        ));
    }
    verify_active_core_entrypoint_pair_with_state(roots, &current_loaded, &before)?;

    let current_is_local_derived =
        parse_local_derived_metadata(&current_release, &current_loaded)?.is_some();
    let hold = if current_is_local_derived {
        None
    } else {
        Some(UpdateHoldRecord {
            generation_id: before.current.clone(),
            release_sequence: current_release.release_sequence,
        })
    };
    let guard = if hold.is_some() && current_loaded.core_path.is_some() {
        Some(rollback_guard::guard_record_for_rollback(
            &before,
            &after,
            &current_release,
            &current_loaded,
            &target_release,
            &target_loaded,
        )?)
    } else {
        None
    };
    let target_hold_aware = if guard.is_some() {
        rollback_guard::target_core_supports_hold(target_loaded.core_path.as_deref())?
    } else {
        false
    };
    if let Some(guard) = guard.as_ref() {
        rollback_guard::write_guard_locked(roots, guard)?;
    }
    let mut rollback_source = None;
    let core_transition = if guard.is_some() {
        if target_hold_aware {
            let target_core =
                target_loaded
                    .core_path
                    .as_ref()
                    .ok_or(LocalProductError::CoreEntrypoint(
                        "hold-aware rollback target has no Core artifact",
                    ))?;
            let record = snapshot_core_entrypoint_to_rollback(roots, &before.current)?;
            if let Err(error) = install_core_entrypoint(
                roots,
                target_core,
                &target_loaded.manifest.core_artifact_digest,
            ) {
                let _ = install_core_entrypoint(roots, &core_rollback_path(roots), &record.digest);
                let _ = rollback_guard::remove_guard_if_present(roots);
                return Err(error);
            }
            Some(record)
        } else {
            None
        }
    } else {
        match (
            current_loaded.core_path.as_ref(),
            target_loaded.core_path.as_ref(),
        ) {
            (_, Some(target_core)) => {
                let record = snapshot_core_entrypoint_to_rollback(roots, &before.current)?;
                if let Err(error) = install_core_entrypoint(
                    roots,
                    target_core,
                    &target_loaded.manifest.core_artifact_digest,
                ) {
                    let _ =
                        install_core_entrypoint(roots, &core_rollback_path(roots), &record.digest);
                    return Err(error);
                }
                Some(record)
            }
            (Some(_), None) => {
                let retained = read_core_entrypoint_rollback(roots)?;
                if retained.generation_id != after.current {
                    return Err(LocalProductError::CoreEntrypoint(
                        "retained Core rollback entrypoint is not bound to the target generation",
                    ));
                }
                let source = copy_core_entrypoint_to_state_temp(
                    roots,
                    &core_rollback_path(roots),
                    "core-rollback-source",
                )?;
                let record = snapshot_core_entrypoint_to_rollback(roots, &before.current)?;
                if let Err(error) = install_core_entrypoint(roots, &source, &retained.digest) {
                    let _ =
                        install_core_entrypoint(roots, &core_rollback_path(roots), &record.digest);
                    let _ = std::fs::remove_file(&source);
                    return Err(error);
                }
                rollback_source = Some(source);
                Some(record)
            }
            (None, None) => None,
        }
    };

    let mut io = m2_generation_state::FsActivationIo;
    let state_result = m2_generation_state::activate_pointer_state_locked(
        &state_paths,
        Some(&before),
        &after,
        &mut io,
        &lock,
    );
    if let Some(source) = rollback_source {
        let _ = std::fs::remove_file(source);
    }
    if let Err(error) = state_result {
        let authoritative = m2_generation_state::read_pointer_state(&state_paths)
            .map_err(LocalProductError::State)?;
        if authoritative.as_ref() != Some(&after) {
            if let Some(record) = core_transition.as_ref() {
                install_core_entrypoint(roots, &core_rollback_path(roots), &record.digest)?;
            }
            if guard.is_some() {
                let _ = rollback_guard::remove_guard_if_present(roots);
            }
        }
        return Err(LocalProductError::State(error));
    }
    if let Some(hold) = hold.as_ref() {
        if let Err(error) = write_update_hold_locked(roots, hold) {
            return Err(LocalProductError::RollbackHoldAfterCommit(Box::new(error)));
        }
    }
    if guard.is_some() && target_hold_aware {
        rollback_guard::remove_guard_if_present(roots)?;
    }
    Ok(ActivatedUpdate {
        generation_id: after.current,
        previous_version: current_loaded.manifest.upstream_package_version.clone(),
        current_version: target_loaded.manifest.upstream_package_version.clone(),
    })
}

#[cfg(unix)]
const UPDATE_USAGE: &str =
    "usage: codex update [--help] | --force | --build-local | --local <DIRECTORY> | --remote <HTTPS_BASE_URL> | --rollback";
#[cfg(unix)]
fn print_update_usage() {
    println!("{UPDATE_USAGE}");
    println!(
        "automatic update retrieves a signed, Termux-patched release from the wrapper channel"
    );
    println!("{}", rollback_guard::UPDATE_HOLD_CAPABILITY_LINE);
}

#[cfg(unix)]
fn run_core_update(args: Vec<OsString>) -> i32 {
    let force = args.len() == 1 && args[0] == OsStr::new("--force");
    if args.is_empty() || force {
        let mut presentation = UpdatePresentation::new();
        let roots = match LocalCoreRoots::from_environment() {
            Ok(roots) => roots,
            Err(err) => {
                presentation.fail(&err);
                return 1;
            }
        };
        let process_env = capture_termux_process_env();
        let hold_policy = if force {
            UpdateHoldPolicy::ForceHeld
        } else {
            UpdateHoldPolicy::Enforce
        };
        return match activate_unified_update_with_hold_policy(
            &roots,
            &process_env,
            hold_policy,
            Some(&mut presentation),
        ) {
            Ok((SignedUpdateOutcome::Activated(outcome), local_derived)) => {
                if local_derived {
                    presentation.finish_local_activation(&outcome.current_version);
                } else {
                    presentation.finish_signed_activation(&outcome.current_version);
                }
                0
            }
            Ok((SignedUpdateOutcome::AlreadyCurrent(target), _)) => {
                presentation.finish_current(&target.version);
                0
            }
            Err(err) => {
                presentation.fail(&err);
                1
            }
        };
    }
    if args.len() == 1 && args[0] == OsStr::new("--help") {
        print_update_usage();
        return 0;
    }
    let build_local = args.len() == 1 && args[0] == OsStr::new("--build-local");
    let local = args.len() == 2 && args[0] == OsStr::new("--local") && !args[1].is_empty();
    let remote = args.len() == 2 && args[0] == OsStr::new("--remote") && !args[1].is_empty();
    let rollback = args.len() == 1 && args[0] == OsStr::new("--rollback");
    if !build_local && !local && !remote && !rollback {
        eprintln!("{UPDATE_USAGE}");
        return 2;
    }
    let mut presentation = UpdatePresentation::new();
    let roots = match LocalCoreRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            presentation.fail(&err);
            return 1;
        }
    };
    if rollback {
        return run_core_rollback_with_presentation(&roots, &mut presentation);
    }
    let process_env = capture_termux_process_env();
    if build_local {
        return match activate_local_built_update(&roots, &process_env, Some(&mut presentation)) {
            Ok(outcome) => {
                presentation.finish_local_activation(&outcome.current_version);
                0
            }
            Err(err) => {
                presentation.fail(&err);
                1
            }
        };
    }
    let result = if local {
        let source = std::path::PathBuf::from(&args[1]);
        activate_signed_local_release_outcome(
            &source,
            &roots,
            &process_env,
            Some(&mut presentation),
        )
    } else {
        activate_signed_remote_release_outcome(
            &args[1],
            &roots,
            &process_env,
            Some(&mut presentation),
        )
    };
    match result {
        Ok(SignedUpdateOutcome::Activated(outcome)) => {
            presentation.finish_signed_activation(&outcome.current_version);
            0
        }
        Ok(SignedUpdateOutcome::AlreadyCurrent(target)) => {
            presentation.finish_current(&target.version);
            0
        }
        Err(err) => {
            presentation.fail(&err);
            1
        }
    }
}

#[cfg(unix)]
fn run_core_rollback_with_presentation(
    roots: &LocalCoreRoots,
    presentation: &mut UpdatePresentation,
) -> i32 {
    match rollback_signed_local_release(roots) {
        Ok(outcome) => {
            presentation.finish_rollback(&outcome.previous_version, &outcome.current_version);
            0
        }
        Err(err) => {
            presentation.fail(&err);
            1
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreRepairOperation {
    Plan,
    Apply,
}

#[cfg(unix)]
fn parse_core_repair_operation(
    request: Option<&OsStr>,
    operation: Option<&OsStr>,
) -> Result<Option<CoreRepairOperation>, ()> {
    match (request, operation) {
        (None, None) => Ok(None),
        (Some(request), Some(operation)) if request == OsStr::new(CORE_REPAIR_REQUEST) => {
            match operation {
                value if value == OsStr::new("plan") => Ok(Some(CoreRepairOperation::Plan)),
                value if value == OsStr::new("apply") => Ok(Some(CoreRepairOperation::Apply)),
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

#[cfg(unix)]
fn core_repair_operation() -> Result<Option<CoreRepairOperation>, ()> {
    parse_core_repair_operation(
        std::env::var_os(CORE_REPAIR_REQUEST_ENV).as_deref(),
        std::env::var_os(CORE_REPAIR_OPERATION_ENV).as_deref(),
    )
}

#[cfg(unix)]
fn core_repair_route_matches(operation: CoreRepairOperation, route: &PublicDispatchRoute) -> bool {
    match (operation, route) {
        (CoreRepairOperation::Plan, PublicDispatchRoute::Doctor(args)) => {
            args.len() == 1 && args[0] == OsStr::new("--json")
        }
        (CoreRepairOperation::Apply, PublicDispatchRoute::Update(args)) => args.is_empty(),
        _ => false,
    }
}

#[cfg(unix)]
fn run_core_repair_request(operation: CoreRepairOperation, route: &PublicDispatchRoute) -> i32 {
    if !core_repair_route_matches(operation, route) {
        eprintln!("codex: invalid Manager repair request");
        return 2;
    }
    let plan = match LocalCoreRoots::from_environment() {
        Ok(roots) => core_repair_plan(&roots),
        Err(_) => CoreRepairPlan {
            action: CoreRepairAction::Unavailable,
            reason: CoreRepairReason::CoreStateUnavailable,
        },
    };
    match operation {
        CoreRepairOperation::Plan => {
            print!("{}", render_core_repair_plan(plan));
            if plan.action == CoreRepairAction::Unavailable {
                1
            } else {
                0
            }
        }
        CoreRepairOperation::Apply => match plan.action {
            CoreRepairAction::None => {
                println!("codex repair: no repair needed");
                0
            }
            CoreRepairAction::Update => {
                std::env::remove_var(CORE_REPAIR_REQUEST_ENV);
                std::env::remove_var(CORE_REPAIR_OPERATION_ENV);
                run_core_update(Vec::new())
            }
            CoreRepairAction::Unavailable => {
                eprintln!("codex repair: plan unavailable");
                1
            }
        },
    }
}

#[cfg(unix)]
fn run_public_main<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let route = match plan_public_dispatch(args) {
        Ok(route) => route,
        Err(err) => {
            eprintln!("codex: {err}");
            return 2;
        }
    };
    if route == PublicDispatchRoute::UnsupportedDaemon {
        eprintln!("codex: {TERMUX_DAEMON_UNSUPPORTED}");
        return 2;
    }
    match core_repair_operation() {
        Ok(Some(operation)) => return run_core_repair_request(operation, &route),
        Ok(None) => {}
        Err(()) => {
            eprintln!("codex: invalid Manager repair request");
            return 2;
        }
    }
    let route = match route {
        PublicDispatchRoute::Update(args) => return run_core_update(args),
        route => route,
    };
    let roots = match LocalCoreRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            eprintln!("codex: {err}");
            return 1;
        }
    };
    let process_env = capture_termux_process_env();
    match execute_activated_route(route, &roots, &process_env) {
        Ok(PublicDispatchCompletion::Doctor(outcome)) => {
            print!("{}", outcome.output);
            doctor_exit_code(outcome.exit_class)
        }
        Ok(PublicDispatchCompletion::TermuxUnavailable(message)) => {
            eprintln!("{message}");
            1
        }
        Ok(PublicDispatchCompletion::Update(_)) => 2,
        Err(err) => {
            eprintln!("codex: {err}");
            1
        }
    }
}

#[cfg(unix)]
fn main() {
    if let Some(code) = run_internal_bootstrap_mode() {
        std::process::exit(code);
    }
    let mut args = std::env::args_os();
    let _ = args.next();
    std::process::exit(run_public_main(args));
}

#[cfg(not(unix))]
fn main() {
    eprintln!("codex: this build requires a Unix/Termux target");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::m2_generation_state::*;
    use super::*;

    fn valid_manifest(manager: bool, with_helper: bool) -> GenerationManifest {
        GenerationManifest {
            upstream_package_identity: "@openai/codex".to_string(),
            upstream_package_version: "9.9.9".to_string(),
            source_artifact_digest: "source-digest".to_string(),
            expected_platform: "android".to_string(),
            expected_architecture: "aarch64".to_string(),
            patch_policy_id: "termux-policy-v1".to_string(),
            patch_report: "qualified".to_string(),
            runtime_digest: "runtime-digest".to_string(),
            helper_digests: if with_helper {
                vec![GenerationHelperDigest {
                    identity: "helper-a".to_string(),
                    digest: "helper-digest".to_string(),
                }]
            } else {
                vec![]
            },
            core_artifact_digest: "core-digest".to_string(),
            manager_artifact_digest: manager.then(|| "manager-digest".to_string()),
            core_api_identity: "core-api-v1".to_string(),
            persistent_schema_identity: "schema-v1".to_string(),
            creation_metadata: "test-fixture".to_string(),
        }
    }

    fn tc2_manifest(manager: bool) -> GenerationManifest {
        let mut manifest = valid_manifest(manager, false);
        manifest.helper_digests = vec![
            GenerationHelperDigest {
                identity: TERMUX_BROWSER_OPEN_HELPER_IDENTITY.to_string(),
                digest: "browser-open-digest".to_string(),
            },
            GenerationHelperDigest {
                identity: TERMUX_BROWSER_MANUAL_HELPER_IDENTITY.to_string(),
                digest: "browser-manual-digest".to_string(),
            },
        ];
        manifest
    }

    #[cfg(unix)]
    fn tc2_helper_bindings<'a>(
        browser_open: &'a OsStr,
        browser_manual: &'a OsStr,
    ) -> [HelperAssetBinding<'a>; 2] {
        [
            HelperAssetBinding {
                identity: TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                asset_path: browser_open,
                observed_digest: "browser-open-digest",
            },
            HelperAssetBinding {
                identity: TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
                asset_path: browser_manual,
                observed_digest: "browser-manual-digest",
            },
        ]
    }

    fn requirements() -> GenerationManifestRequirements<'static> {
        GenerationManifestRequirements {
            platform: "android",
            architecture: "aarch64",
            core_api_identity: "core-api-v1",
            persistent_schema_identity: "schema-v1",
        }
    }

    #[test]
    fn test_tc_live_bridge_layout_is_exact_and_marker_bound() {
        let canonical = tc2_manifest(false);
        assert!(!r10_browser_helper_bridge(&canonical).unwrap());
        assert_eq!(
            generation_helper_relative_path_for_layout(
                0,
                TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                false,
            ),
            std::path::PathBuf::from("browser/open/curl")
        );
        assert_eq!(
            generation_helper_relative_path_for_layout(
                1,
                TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
                false,
            ),
            std::path::PathBuf::from("browser/manual/curl")
        );

        let mut bridge = canonical.clone();
        bridge.creation_metadata = R10_BROWSER_HELPER_BRIDGE_METADATA.to_string();
        assert!(r10_browser_helper_bridge(&bridge).unwrap());
        assert_eq!(
            generation_helper_relative_path_for_layout(
                0,
                TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                true,
            ),
            std::path::PathBuf::from("helpers/0")
        );
        assert_eq!(
            generation_helper_relative_path_for_layout(
                1,
                TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
                true,
            ),
            std::path::PathBuf::from("helpers/1")
        );

        let mut swapped = bridge.clone();
        swapped.helper_digests.swap(0, 1);
        assert!(r10_browser_helper_bridge(&swapped).is_err());
        let mut incomplete = bridge;
        incomplete.helper_digests.pop();
        assert!(r10_browser_helper_bridge(&incomplete).is_err());
    }

    #[test]
    fn test_public_dispatch_exact_routes_and_upstream_preservation() {
        assert_eq!(
            plan_public_dispatch(["update", "--channel", "stable"]).unwrap(),
            PublicDispatchRoute::Update(vec!["--channel".into(), "stable".into()])
        );
        assert_eq!(
            plan_public_dispatch(["doctor", "--json"]).unwrap(),
            PublicDispatchRoute::Doctor(vec!["--json".into()])
        );
        assert_eq!(
            plan_public_dispatch(["doctor", "--color"]).unwrap(),
            PublicDispatchRoute::Doctor(vec!["--color".into()])
        );
        assert_eq!(
            plan_public_dispatch(["termux", "status"]).unwrap(),
            PublicDispatchRoute::Termux(vec!["status".into()])
        );
        for original in [
            vec![],
            vec![OsString::from("--version")],
            vec![OsString::from("-V")],
            vec![OsString::from("--"), OsString::from("doctor")],
            vec![OsString::from("Doctor")],
            vec![OsString::from("doctorx")],
            vec![OsString::from("rollback")],
            vec![OsString::from("rollback"), OsString::from("extra")],
            vec![OsString::from("exec"), OsString::from("termux")],
        ] {
            match plan_public_dispatch(original.clone()).unwrap() {
                PublicDispatchRoute::Upstream(planned) => {
                    assert_eq!(planned[0], "-c");
                    assert_eq!(planned[1], "sandbox_mode=\"danger-full-access\"");
                    assert_eq!(&planned[2..], original.as_slice());
                }
                other => panic!("unexpected route: {other:?}"),
            }
        }
    }

    #[test]
    fn test_tc1_daemon_authority_fence_is_fail_closed_before_upstream_planning() {
        for argv in [
            vec!["remote-control", "start"],
            vec!["remote-control", "--json", "stop"],
            vec!["remote-control", "pair", "--json"],
            vec!["app-server", "daemon", "start"],
            vec!["app-server", "daemon", "bootstrap", "--remote-control"],
            vec!["app-server", "--strict-config", "daemon", "pid-update-loop"],
            vec!["app-server", "--ws-issuer", "issuer", "daemon", "start"],
            vec!["app-server", "-c", "foo=bar", "daemon", "start"],
            vec!["app-server", "--enable", "foo", "daemon", "start"],
            vec!["-c", "model=\"gpt-5.6\"", "remote-control", "start"],
            vec!["remote-control", "-c", "foo=bar", "start"],
            vec!["--image", "a.png", "b.png", "remote-control", "start"],
            vec!["-C", "/tmp/work", "app-server", "daemon", "restart"],
        ] {
            assert_eq!(
                plan_public_dispatch(argv.clone()).unwrap(),
                PublicDispatchRoute::UnsupportedDaemon,
                "{argv:?}"
            );
        }
        assert_eq!(run_public_main(["remote-control", "start"]), 2);
    }

    #[test]
    fn test_tc1_foreground_remote_control_and_non_daemon_app_server_stay_upstream() {
        for original in [
            vec![OsString::from("remote-control")],
            vec![OsString::from("remote-control"), OsString::from("--json")],
            vec![OsString::from("remote-control"), OsString::from("--help")],
            vec![OsString::from("help"), OsString::from("remote-control")],
            vec![OsString::from("app-server")],
            vec![
                OsString::from("app-server"),
                OsString::from("--listen"),
                OsString::from("stdio://"),
            ],
            vec![
                OsString::from("app-server"),
                OsString::from("--ws-issuer"),
                OsString::from("daemon"),
            ],
            vec![
                OsString::from("app-server"),
                OsString::from("--enable"),
                OsString::from("daemon"),
            ],
            vec![
                OsString::from("remote-control"),
                OsString::from("--enable"),
                OsString::from("start"),
            ],
            vec![
                OsString::from("--"),
                OsString::from("remote-control"),
                OsString::from("start"),
            ],
        ] {
            match plan_public_dispatch(original.clone()).unwrap() {
                PublicDispatchRoute::Upstream(planned) => {
                    assert_eq!(planned[0], "-c");
                    assert_eq!(planned[1], "sandbox_mode=\"danger-full-access\"");
                    assert_eq!(&planned[2..], original.as_slice());
                }
                other => panic!("unexpected route: {other:?}"),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_public_dispatch_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let first = OsString::from_vec(vec![b'u', b'p', 0xff]);
        let tail = OsString::from_vec(vec![0x80, b'x', 0xfe]);
        match plan_public_dispatch(vec![first.clone(), tail.clone()]).unwrap() {
            PublicDispatchRoute::Upstream(argv) => {
                assert_eq!(argv[2].as_bytes(), first.as_bytes());
                assert_eq!(argv[3].as_bytes(), tail.as_bytes());
            }
            other => panic!("unexpected route: {other:?}"),
        }
        match plan_public_dispatch(vec![OsString::from("doctor"), tail.clone()]).unwrap() {
            PublicDispatchRoute::Doctor(argv) => assert_eq!(argv[0].as_bytes(), tail.as_bytes()),
            other => panic!("unexpected route: {other:?}"),
        }
    }

    #[test]
    fn test_sandbox_policy_is_one_direct_fail_closed_planner() {
        for argv in [
            vec!["--sandbox", "read-only"],
            vec!["--sandbox=workspace-write", "exec"],
            vec!["-sread-only", "exec"],
            vec!["--config", " sandbox_mode = 'workspace-write' "],
            vec!["-c=sandbox_mode=\"read-only\""],
            vec!["sandbox", "linux"],
        ] {
            assert!(plan_public_dispatch(argv.clone()).is_err(), "{argv:?}");
        }
        let original = vec![
            OsString::from("exec"),
            OsString::from("--"),
            OsString::from("--sandbox=read-only"),
        ];
        let planned = plan_passthrough_args(original.clone()).unwrap();
        assert_eq!(planned[0], "-c");
        assert_eq!(planned[1], "sandbox_mode=\"danger-full-access\"");
        assert_eq!(&planned[2..], original.as_slice());
    }

    #[cfg(unix)]
    #[test]
    fn test_sandbox_planner_preserves_raw_non_utf8_argv() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let raw = OsString::from_vec(vec![0xff, 0x80, b'z']);
        let planned = plan_passthrough_args(vec![raw.clone()]).unwrap();
        assert_eq!(planned[2].as_bytes(), raw.as_bytes());
    }

    #[cfg(unix)]
    #[test]
    fn test_termux_environment_plan_is_minimal_and_exact() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let inherited = OsString::from_vec(vec![b'/', b'i', b'n', b'h', 0xff]);
        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(OsString::from("/test/prefix")),
            tmpdir: Some(OsString::from("/test/tmp")),
            inherited_path: Some(inherited.clone()),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: Some(OsString::from("/test/certs")),
        };
        let plan = plan_termux_env(
            &snapshot,
            OsStr::new("/test/compat"),
            OsStr::new("/test/browser/open/curl"),
            OsStr::new("/test/browser/manual/curl"),
            OsStr::new("/fallback/cert.pem"),
            None,
        )
        .unwrap();
        assert_eq!(plan.assignments.len(), 11);
        for name in ["TMPDIR", "TMP", "TEMP", "SQLITE_TMPDIR"] {
            assert!(plan
                .assignments
                .iter()
                .any(|(k, v)| k == name && v == "/test/tmp"));
        }
        assert!(plan
            .assignments
            .iter()
            .any(|(k, v)| { k == "SSL_CERT_FILE" && v == "/fallback/cert.pem" }));
        assert!(plan
            .assignments
            .iter()
            .any(|(k, v)| { k == "SSL_CERT_DIR" && v == "/test/certs" }));
        let path = plan
            .assignments
            .iter()
            .find(|(k, _)| k == "PATH")
            .unwrap()
            .1
            .as_bytes();
        let mut expected = b"/test/compat:/test/prefix/bin:".to_vec();
        expected.extend_from_slice(inherited.as_bytes());
        assert_eq!(path, expected.as_slice());
        assert!(plan.assignments.iter().any(|(k, v)| {
            k == "BROWSER" && v == "/test/browser/open/curl %s:/test/browser/manual/curl %s"
        }));
        assert!(plan.assignments.iter().any(|(k, v)| {
            k == "CODEX_TERMUX_URL_OPENER" && v == "/test/prefix/bin/termux-open-url"
        }));
        assert!(plan
            .assignments
            .iter()
            .any(|(k, v)| k == "DISPLAY" && v.is_empty()));
        assert!(plan
            .assignments
            .iter()
            .any(|(k, v)| { k == "WAYLAND_DISPLAY" && v == "/__codex_termux_unavailable__" }));
        assert_eq!(
            plan.removals,
            vec![
                OsString::from("WAYLAND_SOCKET"),
                OsString::from("WSL_DISTRO_NAME"),
                OsString::from("WSL_INTEROP")
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_tc2_browser_policy_and_linux_android_capability_fence_are_exact() {
        use std::os::unix::ffi::OsStringExt;

        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(OsString::from("/data/data/com.termux/files/usr")),
            tmpdir: Some(OsString::from("/data/data/com.termux/files/usr/tmp")),
            inherited_path: Some(OsString::from("/usr/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let open = OsStr::new("/generation/browser/open/curl");
        let manual = OsStr::new("/generation/browser/manual/curl");
        let plan = plan_termux_env(
            &snapshot,
            OsStr::new("/generation"),
            open,
            manual,
            OsStr::new("/cert.pem"),
            None,
        )
        .unwrap();
        assert!(plan.assignments.iter().any(|(name, value)| {
            name == "BROWSER"
                && value == "/generation/browser/open/curl %s:/generation/browser/manual/curl %s"
        }));
        assert!(plan.assignments.iter().any(|(name, value)| {
            name == "CODEX_TERMUX_URL_OPENER"
                && value == "/data/data/com.termux/files/usr/bin/termux-open-url"
        }));
        assert!(plan
            .assignments
            .iter()
            .any(|(name, value)| name == "DISPLAY" && value.is_empty()));
        assert!(plan.assignments.iter().any(|(name, value)| {
            name == "WAYLAND_DISPLAY" && value == "/__codex_termux_unavailable__"
        }));
        assert_eq!(
            plan.removals,
            vec![
                OsString::from("WAYLAND_SOCKET"),
                OsString::from("WSL_DISTRO_NAME"),
                OsString::from("WSL_INTEROP"),
            ]
        );

        let mut wsl_snapshot = snapshot.clone();
        wsl_snapshot.wsl_kernel = Some(true);
        assert_eq!(
            plan_termux_env(
                &wsl_snapshot,
                OsStr::new("/generation"),
                open,
                manual,
                OsStr::new("/cert.pem"),
                None,
            ),
            Err(TermuxProcessEnvError::UnsupportedHost("WSL"))
        );
        let mut unreadable_kernel_snapshot = snapshot.clone();
        unreadable_kernel_snapshot.wsl_kernel = None;
        assert_eq!(
            plan_termux_env(
                &unreadable_kernel_snapshot,
                OsStr::new("/generation"),
                open,
                manual,
                OsStr::new("/cert.pem"),
                None,
            )
            .unwrap(),
            plan
        );

        for invalid in [
            OsString::from("relative/browser/curl"),
            OsString::from("/browser:bad/curl"),
            OsString::from("/browser bad/curl"),
            OsString::from_vec(b"/browser/bad\0curl".to_vec()),
        ] {
            assert_eq!(
                plan_termux_env(
                    &snapshot,
                    OsStr::new("/generation"),
                    invalid.as_os_str(),
                    manual,
                    OsStr::new("/cert.pem"),
                    None,
                ),
                Err(TermuxProcessEnvError::InvalidPathComponent(
                    "browser_open_helper"
                ))
            );
        }

        assert_eq!(
            generation_helper_relative_path(0, TERMUX_BROWSER_OPEN_HELPER_IDENTITY),
            std::path::PathBuf::from("browser/open/curl")
        );
        assert_eq!(
            generation_helper_relative_path(1, TERMUX_BROWSER_MANUAL_HELPER_IDENTITY),
            std::path::PathBuf::from("browser/manual/curl")
        );
        assert!(valid_release_relative_path("browser/open/curl"));
        assert!(valid_release_relative_path("browser/manual/curl"));
        for invalid in [
            "browser/open",
            "browser/manual",
            "browser/open/curl/extra",
            "browser/../open/curl",
        ] {
            assert!(!valid_release_relative_path(invalid), "{invalid}");
        }

        let manifest = tc2_manifest(false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let only_open = [HelperAssetBinding {
            identity: TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
            asset_path: open,
            observed_digest: "browser-open-digest",
        }];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/generation/runtime"),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: OsStr::new("/generation"),
            helpers: &only_open,
        };
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::MissingHelperIdentity(1)
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_tc3_mcp_file_store_policy_is_exact() {
        let empty = render_core_notification_config(&[]);
        assert_eq!(
            empty,
            b"# codex-termux-notify-v1\nmcp_oauth_credentials_store = \"file\"\n\n".to_vec()
        );
        let with_hook =
            String::from_utf8(render_core_notification_config(&["SessionStart"])).unwrap();
        assert!(with_hook
            .starts_with("# codex-termux-notify-v1\nmcp_oauth_credentials_store = \"file\"\n\n"));
        assert!(with_hook.contains("[[hooks.SessionStart]]"));
        assert!(!with_hook.contains("keyring"));
    }

    #[cfg(unix)]
    #[test]
    fn test_termux_environment_errors_and_capture_are_direct() {
        let mut snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: None,
            tmpdir: Some("/tmp".into()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        assert_eq!(
            plan_termux_env(
                &snapshot,
                OsStr::new("/compat"),
                OsStr::new("/browser/open/curl"),
                OsStr::new("/browser/manual/curl"),
                OsStr::new("/cert"),
                None,
            ),
            Err(TermuxProcessEnvError::MissingRequired("PREFIX"))
        );
        snapshot.prefix = Some("/prefix".into());
        snapshot.tmpdir = Some(OsString::new());
        assert_eq!(
            plan_termux_env(
                &snapshot,
                OsStr::new("/compat"),
                OsStr::new("/browser/open/curl"),
                OsStr::new("/browser/manual/curl"),
                OsStr::new("/cert"),
                None,
            ),
            Err(TermuxProcessEnvError::EmptyRequired("TMPDIR"))
        );
        let captured = capture_termux_process_env();
        assert_eq!(captured.wsl_kernel, detect_wsl_kernel_signature());
        assert_eq!(captured.prefix, std::env::var_os("PREFIX"));
        assert_eq!(captured.tmpdir, std::env::var_os("TMPDIR"));
        assert_eq!(captured.inherited_path, std::env::var_os("PATH"));
        assert_eq!(
            captured.inherited_ssl_cert_file,
            std::env::var_os("SSL_CERT_FILE")
        );
        assert_eq!(
            captured.inherited_ssl_cert_dir,
            std::env::var_os("SSL_CERT_DIR")
        );
    }

    #[test]
    fn test_generation_manifest_qualification_keeps_only_load_bearing_checks() {
        let manifest = valid_manifest(false, true);
        let qualified = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        assert_eq!(qualified.manifest().runtime_digest, "runtime-digest");
        let mut bad = manifest.clone();
        bad.expected_architecture = "x86_64".to_string();
        assert_eq!(
            qualify_generation_manifest(&bad, &requirements()).unwrap_err(),
            GenerationManifestError::ArchitectureMismatch
        );
        let mut bad = manifest.clone();
        bad.helper_digests.push(GenerationHelperDigest {
            identity: "helper-a".to_string(),
            digest: "other".to_string(),
        });
        assert!(matches!(
            qualify_generation_manifest(&bad, &requirements()),
            Err(GenerationManifestError::DuplicateHelperIdentity { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_runtime_and_manager_qualification_share_one_generation_authority() {
        let manifest = valid_manifest(true, true);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let helpers = [HelperAssetBinding {
            identity: "helper-a",
            asset_path: OsStr::new("/test/helper"),
            observed_digest: "helper-digest",
        }];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/test/runtime"),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: OsStr::new("/test/compat"),
            helpers: &helpers,
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        assert_eq!(
            assets.selection().runtime.program_path,
            OsStr::new("/test/runtime")
        );
        let manager_selection = ManagerArtifactSelection {
            program_path: OsStr::new("/test/manager"),
            observed_digest: "manager-digest",
        };
        assert!(matches!(
            qualify_manager_artifact(generation, Some(&manager_selection)).unwrap(),
            ManagerArtifact::Available(_)
        ));
        let mut bad = manager_selection;
        bad.observed_digest = "wrong";
        assert_eq!(
            qualify_manager_artifact(generation, Some(&bad)).unwrap_err(),
            ManagerArtifactError::DigestMismatch
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_runtime_qualification_rejects_path_digest_and_helper_mismatch() {
        let manifest = valid_manifest(false, true);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let helper = [HelperAssetBinding {
            identity: "helper-a",
            asset_path: OsStr::new("/helper"),
            observed_digest: "helper-digest",
        }];
        let mut selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("relative"),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: OsStr::new("/compat"),
            helpers: &helper,
        };
        assert!(matches!(
            qualify_runtime_assets(generation, &selection),
            Err(RuntimeAssetError::RelativePath("runtime_program"))
        ));
        selection.runtime.program_path = OsStr::new("/runtime");
        selection.runtime.observed_digest = "wrong";
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::RuntimeDigestMismatch
        );
        selection.runtime.observed_digest = "runtime-digest";
        selection.helpers = &[];
        assert_eq!(
            qualify_runtime_assets(generation, &selection).unwrap_err(),
            RuntimeAssetError::MissingHelperIdentity(0)
        );
    }

    #[test]
    fn test_doctor_report_and_usage_keep_bounded_public_contract() {
        let report = compose_doctor_report(
            QualifiedUpstreamDoctorResult {
                status: UpstreamDoctorStatus::Unsupported,
                output: String::new(),
            },
            TermuxCoreDoctorReport {
                status: CoreDoctorStatus::Healthy,
                generation_id: "test-generation".to_owned(),
                generation_layout: GenerationLayout::RootCodeModeHost,
            },
            ManagerDoctorStatus::Unavailable,
        );
        assert_eq!(report.summary, DoctorSummaryStatus::Degraded);
        assert_eq!(doctor_exit_class(&report), DoctorExitClass::HealthFailure);
        let unavailable_human = render_doctor_human(&report, false);
        assert!(unavailable_human.starts_with("Upstream doctor output unavailable: unsupported\n"));
        assert!(unavailable_human.contains("\n[Termux doctor]\n"));
        let json = render_doctor_json(&report);
        assert!(json.contains("\"schema_version\":2"));
        assert!(json.contains("\"upstream\":{\"status\":\"unsupported\",\"output\":\"\"}"));
        assert!(json.contains("\"generation_id\":\"test-generation\""));
        assert!(json.contains("\"layout\":\"root-code-mode-host-v2\""));
        assert!(json.contains("\"code_mode_host\":{\"status\":\"healthy\"}"));
        assert!(json.contains("\"reason\":\"bwrap is not used\""));
        assert_eq!(
            doctor_output_mode(Vec::<OsString>::new()).unwrap(),
            DoctorOutputMode::Human
        );
        assert_eq!(
            doctor_output_mode([OsString::from("--json")]).unwrap(),
            DoctorOutputMode::Json
        );
        assert_eq!(
            doctor_output_mode([OsString::from("--color")]).unwrap(),
            DoctorOutputMode::Color
        );
        assert!(doctor_output_mode([OsString::from("--color"), OsString::from("--json")]).is_err());
        let err = doctor_output_mode([OsString::from("secret-value")]).unwrap_err();
        assert_eq!(err.to_string(), "usage: codex doctor [--json|--color]");
        assert!(!err.to_string().contains("secret-value"));

        assert!(doctor_color_policy(true, false, false));
        assert!(!doctor_color_policy(true, true, false));
        assert!(doctor_color_policy(true, true, true));
        assert!(!doctor_color_policy(false, false, true));

        let shell = resolve_test_shell();
        let mut cleared = std::process::Command::new(&shell);
        cleared
            .args(["-c", "test -z \"${NO_COLOR+x}\""])
            .env("NO_COLOR", "1");
        apply_doctor_color_override(&mut cleared, true, true);
        assert!(cleared.status().unwrap().success());

        let mut retained = std::process::Command::new(&shell);
        retained
            .args(["-c", "test -n \"${NO_COLOR+x}\""])
            .env("NO_COLOR", "1");
        apply_doctor_color_override(&mut retained, true, false);
        assert!(retained.status().unwrap().success());

        let redacted = redact_doctor_output(
            b"status=ok api_key=sk-secret-value\n\x1b[31mBearer bearer-secret\x1b[0m\n",
            false,
        );
        assert!(redacted.contains("status=ok"));
        assert!(!redacted.contains("sk-secret-value"));
        assert!(!redacted.contains("bearer-secret"));
        assert!(!redacted.contains('\u{1b}'));

        let redacted_multiple = redact_doctor_output(
            "π api_key=first-secret api_key=second-secret token=third-secret authorization=Bearer bearer-secret\n"
                .as_bytes(),
            false,
        );
        assert!(redacted_multiple.contains("π"));
        assert_eq!(redacted_multiple.matches("[redacted]").count(), 4);
        assert!(!redacted_multiple.contains("first-secret"));
        assert!(!redacted_multiple.contains("second-secret"));
        assert!(!redacted_multiple.contains("third-secret"));
        assert!(!redacted_multiple.contains("bearer-secret"));

        let redacted_markers =
            redact_doctor_output(b"sk-first sess-second ghp_third sk-fourth\n", false);
        assert_eq!(redacted_markers.matches("[redacted]").count(), 4);
        assert!(!redacted_markers.contains("first"));
        assert!(!redacted_markers.contains("second"));
        assert!(!redacted_markers.contains("third"));
        assert!(!redacted_markers.contains("fourth"));

        let styled = redact_doctor_output(
            b"\x1b[32mdoctor ok\x1b[39m\r\x1b[2K\x1b[1mfinal\x1b[22m\n",
            true,
        );
        assert_eq!(styled, "\x1b[1mfinal\x1b[22m\n");
        let styled_redacted =
            redact_doctor_output(b"\x1b[31mapi_key=sk-secret-value\x1b[39m\n", true);
        assert_eq!(styled_redacted, "\x1b[31mapi_key=[redacted]\x1b[39m\n");
        assert!(!styled_redacted.contains("sk-secret-value"));
        let styled_multiple_redacted =
            redact_doctor_output(b"\x1b[31mapi_key=first token=second\x1b[39m\n", true);
        assert_eq!(
            styled_multiple_redacted,
            "\x1b[31mapi_key=[redacted] token=[redacted]\x1b[39m\n"
        );
        assert!(!render_doctor_human(&report, false).contains('\u{1b}'));
        assert!(render_doctor_human(&report, true).contains("\x1b[1m"));

        let pass_through_report = compose_doctor_report(
            QualifiedUpstreamDoctorResult {
                status: UpstreamDoctorStatus::Healthy,
                output: "\x1b[1mCodex Doctor v9.9.9\x1b[22m\n\nupstream checks\n".to_owned(),
            },
            TermuxCoreDoctorReport {
                status: CoreDoctorStatus::Healthy,
                generation_id: "test-generation".to_owned(),
                generation_layout: GenerationLayout::RootCodeModeHost,
            },
            ManagerDoctorStatus::Healthy,
        );
        let pass_through = render_doctor_human(&pass_through_report, true);
        assert!(pass_through.starts_with("\x1b[1mCodex Doctor v9.9.9\x1b[22m"));
        assert!(!pass_through.contains("[Upstream Codex doctor]"));
        assert!(!pass_through.contains("status: healthy\n"));
        assert!(pass_through.contains("\n[Termux doctor]\n"));
    }

    #[cfg(unix)]
    #[test]
    fn test_r4_doctor_pty_preserves_clean_upstream_sgr_and_layout() {
        use std::os::unix::fs::PermissionsExt;

        let (root, runtime, resolver, config) = prepare_exec_fixture("r4-doctor-pty");
        let prefix = root.join("prefix");
        let script_source =
            std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/script");
        assert!(
            script_source.is_file(),
            "Termux script is required for PTY proof"
        );
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        let script = prefix.join("bin/script");
        std::fs::copy(&script_source, &script).unwrap();
        let mut script_mode = std::fs::metadata(&script).unwrap().permissions();
        script_mode.set_mode(0o755);
        std::fs::set_permissions(&script, script_mode).unwrap();

        let clean_runtime = root.join("clean-runtime");
        let clean_body = std::fs::read_to_string(&runtime)
            .unwrap()
            .replace("api_key=sk-upstream-secret", "status=ok");
        std::fs::write(&clean_runtime, clean_body).unwrap();
        let mut runtime_mode = std::fs::metadata(&clean_runtime).unwrap().permissions();
        runtime_mode.set_mode(0o755);
        std::fs::set_permissions(&clean_runtime, runtime_mode).unwrap();

        let manifest = tc2_manifest(false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let compat = root.join("compat");
        let cert_file = root.join("cert.pem");
        let cert_dir = root.join("certs");
        let browser_open = root.join("browser/open/curl");
        let browser_manual = root.join("browser/manual/curl");
        let helpers = tc2_helper_bindings(browser_open.as_os_str(), browser_manual.as_os_str());
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: clean_runtime.as_os_str(),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: compat.as_os_str(),
            helpers: &helpers,
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(prefix.into_os_string()),
            tmpdir: Some(root.join("tmp").into_os_string()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let output = capture_qualified_upstream_doctor(
            assets,
            &snapshot,
            cert_file.as_os_str(),
            Some(cert_dir.as_os_str()),
            &resolver,
            &config,
            DoctorCaptureOptions {
                json: false,
                use_color: true,
                force_color: true,
            },
        )
        .unwrap();
        assert_eq!(output.status, UpstreamDoctorStatus::Healthy);
        assert!(output
            .output
            .contains("\x1b[1mupstream doctor ok\x1b[22m status=ok"));
        assert!(!output.output.contains("checking upstream"));
        assert!(!output.output.contains('\r'));
        assert!(!output.output.contains("\x1b[2K"));
        assert!(output.output.contains("\x1b[22m"));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn temp_root(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("codex-r2-{label}-{}-{id}", std::process::id()));
        remove_temp_root(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn remove_temp_root(path: impl AsRef<std::path::Path>) {
        let path = path.as_ref();
        match std::fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => panic!("remove test temporary root {}: {err}", path.display()),
        }
    }

    #[cfg(unix)]
    fn resolve_test_shell() -> OsString {
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                let candidate = dir.join("sh");
                if candidate.is_file() {
                    return candidate.into_os_string();
                }
            }
        }
        OsString::from("/data/data/com.termux/files/usr/bin/sh")
    }

    #[cfg(unix)]
    fn write_fake_runtime(root: &std::path::Path) -> std::path::PathBuf {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let path = root.join("fake-codex");
        let body = r#"
if [ "${CODEX_TEST_REQUIRE_NO_ACQUISITION:-}" = "1" ]; then
  for acquisition in "$HOME"/.local/lib/codex/core/generations/.acquire-*; do
    [ ! -e "$acquisition" ] || exit 96
  done
fi
if [ "$1" = "-c" ]; then
  [ "$2" = 'sandbox_mode="danger-full-access"' ] || exit 91
  shift 2
fi
if [ "$1" = "--version" ] || [ "$1" = "-V" ]; then
  printf 'codex-upstream 9.9.9\n'
  printf 'version-stderr\n' >&2
  exit 0
fi
if [ "$1" = "signal" ]; then
  printf '%s\n' "$$" > "$CODEX_TEST_PID_FILE"
  trap 'exit 143' TERM
  while :; do :; done
fi
if [ "$1" = "tty" ]; then
  [ -t 0 ] && [ -t 1 ] && [ -t 2 ] && exit 0
  exit 88
fi
if [ "$CODEX_TERMUX_CORE_API" = "codex-manager-core-v1" ] && [ -n "$CODEX_TERMUX_CORE_ENTRYPOINT" ]; then
  printf 'HANDOFF_OK\n'
else
  printf 'HANDOFF_BAD\n'
fi
if [ -z "${CODEX_MANAGER_ARTIFACT_PROBE+x}" ]; then
  printf 'ARTIFACT_PROBE_CLEARED\n'
else
  printf 'ARTIFACT_PROBE_PRESENT\n'
fi
if [ "$1" = "doctor" ]; then
  if [ "${CODEX_TEST_DOCTOR_LARGE:-}" = "1" ]; then
    i=0
    while [ "$i" -le 65536 ]; do
      printf x
      i=$((i + 1))
    done
    exit 0
  fi
  if [ "$2" = "--json" ]; then
    printf '{"status":"ok","api_key":"sk-upstream-secret"}\n'
  elif [ -t 1 ] && [ -z "${NO_COLOR+x}" ]; then
    printf '\033[2K\r'
    printf '\033[33mchecking upstream\033[39m'
    printf '\r\033[2K'
    printf '\033[1mupstream doctor ok\033[22m api_key=sk-upstream-secret\n'
  else
    printf 'upstream doctor ok api_key=sk-upstream-secret\n'
  fi
  printf 'SECRET-UPSTREAM-STDERR\n' >&2
  exit "${CODEX_TEST_DOCTOR_EXIT:-0}"
fi
printf 'ARGS:'
for a in "$@"; do printf '<%s>' "$a"; done
printf '\n'
if [ -r /proc/self/fd/33 ] && [ -d /proc/self/fd/34 ]; then printf 'FDS_OK\n'; else printf 'FDS_BAD\n'; fi
if [ -z "${CODEX_MANAGED_BY_NPM+x}" ] && [ -z "${CODEX_MANAGED_BY_BUN+x}" ] && [ -z "${CODEX_MANAGED_PACKAGE_ROOT+x}" ] && [ -z "${LD_PRELOAD+x}" ] && [ -z "${LD_LIBRARY_PATH+x}" ] && [ "$CODEX_TEST_SURVIVES" = "yes" ]; then printf 'ENV_OK\n'; else printf 'ENV_BAD\n'; fi
printf 'STDERR_MARK\n' >&2
exit 73
"#;
        std::fs::write(&path, format!("#!{shell}\n{body}")).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[cfg(unix)]
    const PROBE_ROLE: &str = "CODEX_R2_PROBE_ROLE";
    #[cfg(unix)]
    const PROBE_SCENARIO: &str = "CODEX_R2_PROBE_SCENARIO";
    #[cfg(unix)]
    const PROBE_RUNTIME: &str = "CODEX_R2_PROBE_RUNTIME";
    #[cfg(unix)]
    const PROBE_RESOLVER: &str = "CODEX_R2_PROBE_RESOLVER";
    #[cfg(unix)]
    const PROBE_CONFIG: &str = "CODEX_R2_PROBE_CONFIG";
    #[cfg(unix)]
    const PROBE_ROOT: &str = "CODEX_R2_PROBE_ROOT";
    #[cfg(unix)]
    const PROBE_STDOUT: &str = "CODEX_R2_PROBE_STDOUT";
    #[cfg(unix)]
    const PROBE_STDERR: &str = "CODEX_R2_PROBE_STDERR";

    #[cfg(unix)]
    #[test]
    fn product_exec_probe() {
        use std::os::fd::AsRawFd;
        use std::os::unix::ffi::OsStringExt;
        if std::env::var(PROBE_ROLE).as_deref() != Ok("1") {
            return;
        }
        if let Some(path) = std::env::var_os(PROBE_STDOUT) {
            let file = std::fs::File::create(path).unwrap();
            assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0);
        }
        if let Some(path) = std::env::var_os(PROBE_STDERR) {
            let file = std::fs::File::create(path).unwrap();
            assert!(unsafe { dup2(file.as_raw_fd(), 2) } >= 0);
        }
        let root = std::path::PathBuf::from(std::env::var_os(PROBE_ROOT).unwrap());
        let runtime = std::env::var_os(PROBE_RUNTIME).unwrap();
        let resolver = std::path::PathBuf::from(std::env::var_os(PROBE_RESOLVER).unwrap());
        let config = std::path::PathBuf::from(std::env::var_os(PROBE_CONFIG).unwrap());
        let compat = root.join("compat");
        let prefix = root.join("prefix");
        let tmp = root.join("tmp");
        let cert = root.join("cert.pem");
        let cert_dir = root.join("certs");
        let scenario = std::env::var(PROBE_SCENARIO).unwrap();
        let manager_enabled = scenario == "manager" || scenario == "notify-projection";
        let manifest = tc2_manifest(manager_enabled);
        let browser_open = root.join("browser/open/curl");
        let browser_manual = root.join("browser/manual/curl");
        let helper_bindings = [
            HelperAssetBinding {
                identity: TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                asset_path: browser_open.as_os_str(),
                observed_digest: "browser-open-digest",
            },
            HelperAssetBinding {
                identity: TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
                asset_path: browser_manual.as_os_str(),
                observed_digest: "browser-manual-digest",
            },
        ];
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime.as_os_str(),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: compat.as_os_str(),
            helpers: &helper_bindings,
        };
        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(prefix.into_os_string()),
            tmpdir: Some(tmp.into_os_string()),
            inherited_path: Some(OsString::from("/inherited/a:/inherited/b")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        if scenario != "manager" {
            std::env::set_var("CODEX_MANAGED_BY_NPM", "bad");
            std::env::set_var("CODEX_MANAGED_BY_BUN", "bad");
            std::env::set_var("CODEX_MANAGED_PACKAGE_ROOT", "/bad");
            std::env::set_var("LD_PRELOAD", "/bad.so");
            std::env::set_var("LD_LIBRARY_PATH", "/bad/lib");
            std::env::set_var("CODEX_TEST_SURVIVES", "yes");
        }
        let manager_selection = ManagerArtifactSelection {
            program_path: runtime.as_os_str(),
            observed_digest: "manager-digest",
        };
        let raw_args = match scenario.as_str() {
            "version" => vec![OsString::from("--version")],
            "signal" => vec![OsString::from("signal")],
            "tty" => vec![OsString::from("tty")],
            "exec" => vec![
                OsString::from("exec"),
                OsString::from("arg with spaces"),
                OsString::from_vec(vec![0xff, 0x80, b'z']),
            ],
            "notify-projection" => vec![OsString::from("--version")],
            "notify-projection-unavailable" => vec![OsString::from("--version")],
            "manager" => vec![
                OsString::from("termux"),
                OsString::from("status"),
                OsString::from_vec(vec![0xff, b'm']),
            ],
            other => panic!("unknown probe scenario {other}"),
        };
        let route = plan_public_dispatch(raw_args).unwrap();
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let manager =
            qualify_manager_artifact(generation, manager_enabled.then_some(&manager_selection))
                .unwrap();
        let context = LocalPublicDispatchContext {
            runtime_assets: assets,
            manager_artifact: manager,
            process_env: &snapshot,
            cert_file: cert.as_os_str(),
            cert_dir: Some(cert_dir.as_os_str()),
            resolver_path: &resolver,
            config_dir: &config,
            doctor_capability: UpstreamDoctorCapability::Supported,
            core_doctor_status: CoreDoctorStatus::Healthy,
            generation_id: "test-generation",
            generation_layout: GenerationLayout::RootCodeModeHost,
            manager_doctor_status: ManagerDoctorStatus::Unavailable,
        };
        match execute_public_dispatch(route, context) {
            Err(PublicDispatchExecutionError::Upstream(RuntimeLaunchError::Exec(err))) => {
                panic!("upstream exec failed: {err}")
            }
            Err(PublicDispatchExecutionError::Manager(err)) => {
                panic!("Manager exec failed: {err}")
            }
            other => panic!("exec unexpectedly returned: {other:?}"),
        }
    }

    #[cfg(unix)]
    const REPAIR_PROBE_ROLE: &str = "CODEX_MGR4_REPAIR_PROBE_ROLE";
    #[cfg(unix)]
    const REPAIR_PROBE_SCENARIO: &str = "CODEX_MGR4_REPAIR_PROBE_SCENARIO";
    #[cfg(unix)]
    const REPAIR_PROBE_ROOT: &str = "CODEX_MGR4_REPAIR_PROBE_ROOT";
    #[cfg(unix)]
    const REPAIR_PROBE_STDOUT: &str = "CODEX_MGR4_REPAIR_PROBE_STDOUT";
    #[cfg(unix)]
    const REPAIR_PROBE_STDERR: &str = "CODEX_MGR4_REPAIR_PROBE_STDERR";

    #[cfg(unix)]
    #[test]
    fn repair_request_probe() {
        use std::io::Write as _;
        use std::os::fd::AsRawFd;

        if std::env::var(REPAIR_PROBE_ROLE).as_deref() != Ok("1") {
            return;
        }
        if let Some(path) = std::env::var_os(REPAIR_PROBE_STDOUT) {
            let file = std::fs::File::create(path).unwrap();
            assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0);
        }
        if let Some(path) = std::env::var_os(REPAIR_PROBE_STDERR) {
            let file = std::fs::File::create(path).unwrap();
            assert!(unsafe { dup2(file.as_raw_fd(), 2) } >= 0);
        }
        let root = std::path::PathBuf::from(std::env::var_os(REPAIR_PROBE_ROOT).unwrap());
        let home = root.join("home");
        let prefix = root.join("prefix");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        std::fs::create_dir_all(prefix.join("etc/tls/certs")).unwrap();
        std::fs::write(prefix.join("etc/resolv.conf"), b"nameserver 127.0.0.1\n").unwrap();
        std::fs::write(prefix.join("etc/tls/cert.pem"), b"test-cert").unwrap();
        let roots = {
            std::env::set_var("HOME", &home);
            std::env::set_var("PREFIX", &prefix);
            LocalCoreRoots::from_environment().unwrap()
        };
        std::fs::create_dir_all(&roots.generation_root).unwrap();
        std::fs::create_dir_all(&roots.state_root).unwrap();
        std::fs::write(&roots.openssl, b"test-openssl").unwrap();
        std::fs::write(&roots.curl, b"test-curl").unwrap();

        let scenario = std::env::var(REPAIR_PROBE_SCENARIO).unwrap();
        let apply = scenario.ends_with("-apply");
        match scenario.as_str() {
            "healthy-plan" | "healthy-apply" => {
                b2_write_root_generation(&roots, "root-v2", false, "unsupported");
                b2_activate(&roots, "root-v2");
            }
            "legacy-plan" | "legacy-apply" => {
                b2_write_generation(&roots, "legacy-v1", false, "unsupported");
                b2_activate(&roots, "legacy-v1");
                if apply {
                    std::env::set_var(UPDATE_INDEX_URL_ENV, "http://invalid");
                }
            }
            "unavailable-plan" | "unavailable-apply" => {}
            other => panic!("unknown repair probe scenario {other}"),
        }
        std::env::set_var(CORE_REPAIR_REQUEST_ENV, CORE_REPAIR_REQUEST);
        std::env::set_var(
            CORE_REPAIR_OPERATION_ENV,
            if apply { "apply" } else { "plan" },
        );
        let args = if apply {
            vec![OsString::from("update")]
        } else {
            vec![OsString::from("doctor"), OsString::from("--json")]
        };
        let status = run_public_main(args);
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
        std::process::exit(status);
    }

    #[cfg(unix)]
    fn run_repair_probe(scenario: &str) -> (std::process::ExitStatus, Vec<u8>, Vec<u8>) {
        let root = temp_root("mgr4-request");
        let stdout = root.join("repair-stdout");
        let stderr = root.join("repair-stderr");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::repair_request_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(REPAIR_PROBE_ROLE, "1")
            .env(REPAIR_PROBE_SCENARIO, scenario)
            .env(REPAIR_PROBE_ROOT, &root)
            .env(REPAIR_PROBE_STDOUT, &stdout)
            .env(REPAIR_PROBE_STDERR, &stderr)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        let result = (
            status,
            std::fs::read(&stdout).unwrap_or_default(),
            std::fs::read(&stderr).unwrap_or_default(),
        );
        remove_temp_root(root);
        result
    }

    #[cfg(unix)]
    #[test]
    fn test_mgr4_public_repair_request_boundary_is_bounded() {
        let (status, stdout, stderr) = run_repair_probe("healthy-plan");
        assert_eq!(
            status.code(),
            Some(0),
            "repair probe failed: stdout={stdout:?} stderr={stderr:?}"
        );
        assert_eq!(
            stdout,
            b"codex-core-repair-v1\naction=none\nreason=healthy\n"
        );
        assert!(stderr.is_empty());

        let (status, stdout, stderr) = run_repair_probe("legacy-plan");
        assert_eq!(status.code(), Some(0));
        assert_eq!(
            stdout,
            b"codex-core-repair-v1\naction=update\nreason=legacy-generation\n"
        );
        assert!(stderr.is_empty());

        let (status, stdout, stderr) = run_repair_probe("healthy-apply");
        assert_eq!(status.code(), Some(0));
        assert_eq!(stdout, b"codex repair: no repair needed\n");
        assert!(stderr.is_empty());

        let (status, stdout, stderr) = run_repair_probe("unavailable-plan");
        assert_eq!(status.code(), Some(1));
        assert_eq!(
            stdout,
            b"codex-core-repair-v1\naction=unavailable\nreason=core-state-unavailable\n"
        );
        assert!(stderr.is_empty());

        let (status, stdout, stderr) = run_repair_probe("unavailable-apply");
        assert_eq!(status.code(), Some(1));
        assert!(stdout.is_empty());
        assert_eq!(stderr, b"codex repair: plan unavailable\n");

        let (status, stdout, stderr) = run_repair_probe("legacy-apply");
        assert_eq!(status.code(), Some(1));
        assert!(stdout.is_empty());
        assert_eq!(stderr, b"Update index URL must use HTTPS. \xE2\x9D\x8C\n");
    }

    #[cfg(unix)]
    struct ProbeResult {
        status: std::process::ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    }

    #[cfg(unix)]
    fn run_product_probe(
        scenario: &str,
        root: &std::path::Path,
        runtime: &std::path::Path,
        resolver: &std::path::Path,
        config: &std::path::Path,
    ) -> ProbeResult {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let stdout = root.join(format!("stdout-{id}"));
        let stderr = root.join(format!("stderr-{id}"));
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("tests::product_exec_probe")
            .arg("--exact")
            .env(PROBE_ROLE, "1")
            .env(PROBE_SCENARIO, scenario)
            .env(PROBE_ROOT, root)
            .env(PROBE_RUNTIME, runtime)
            .env(PROBE_RESOLVER, resolver)
            .env(PROBE_CONFIG, config)
            .env(PROBE_STDOUT, &stdout)
            .env(PROBE_STDERR, &stderr)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if scenario.starts_with("notify-projection") {
            command.env("HOME", root);
        }
        if scenario == "manager" {
            command.env(MANAGER_ARTIFACT_PROBE_ENV, "1");
        }
        let status = command.status().unwrap();
        ProbeResult {
            status,
            stdout: std::fs::read(&stdout).unwrap_or_default(),
            stderr: std::fs::read(&stderr).unwrap_or_default(),
        }
    }

    #[cfg(unix)]
    fn prepare_exec_fixture(
        label: &str,
    ) -> (
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        let root = temp_root(label);
        let runtime = write_fake_runtime(&root);
        let resolver = root.join("resolv.conf");
        let config = root.join("config");
        std::fs::write(&resolver, b"nameserver 127.0.0.1\n").unwrap();
        std::fs::create_dir(&config).unwrap();
        std::fs::create_dir(root.join("compat")).unwrap();
        std::fs::create_dir(root.join("prefix")).unwrap();
        std::fs::create_dir(root.join("tmp")).unwrap();
        std::fs::create_dir(root.join("certs")).unwrap();
        std::fs::create_dir(root.join("browser")).unwrap();
        std::fs::create_dir(root.join("browser/open")).unwrap();
        std::fs::create_dir(root.join("browser/manual")).unwrap();
        std::fs::write(root.join("browser/open/curl"), b"helper").unwrap();
        std::fs::write(root.join("browser/manual/curl"), b"helper").unwrap();
        std::fs::write(root.join("cert.pem"), b"test-cert").unwrap();
        (root, runtime, resolver, config)
    }

    #[cfg(unix)]
    #[test]
    fn test_final_upstream_exec_preserves_argv_stream_exit_env_fds_and_resolver() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("exec");
        let before = std::fs::read(&resolver).unwrap();
        let result = run_product_probe("exec", &root, &runtime, &resolver, &config);
        assert_eq!(result.status.code(), Some(73));
        assert!(result
            .stdout
            .windows(b"ARGS:<exec><arg with spaces><".len())
            .any(|w| w == b"ARGS:<exec><arg with spaces><"));
        assert!(result.stdout.windows(3).any(|w| w == [0xff, 0x80, b'z']));
        assert!(result.stdout.windows(6).any(|w| w == b"FDS_OK"));
        assert!(result.stdout.windows(6).any(|w| w == b"ENV_OK"));
        assert!(result.stderr.windows(11).any(|w| w == b"STDERR_MARK"));
        assert_eq!(std::fs::read(&resolver).unwrap(), before);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_upstream_version_is_exact_direct_output() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("version");
        let direct = std::process::Command::new(&runtime)
            .arg("--version")
            .output()
            .unwrap();
        let through = run_product_probe("version", &root, &runtime, &resolver, &config);
        assert_eq!(through.status.code(), direct.status.code());
        assert_eq!(through.stdout, direct.stdout);
        assert_eq!(through.stderr, direct.stderr);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_mgr3_notification_record_projects_hooks_on_real_upstream_launch() {
        use std::os::unix::fs::PermissionsExt;

        let (root, runtime, resolver, config) = prepare_exec_fixture("notify-projection");
        let notifications = root.join(".local/share/codex/manager/notifications");
        std::fs::create_dir_all(&notifications).unwrap();
        for directory in [
            root.join(".local"),
            root.join(".local/share"),
            root.join(".local/share/codex"),
            root.join(".local/share/codex/manager"),
            notifications.clone(),
        ] {
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let record = b"codex-manager-notify-v1\nchannel\tboth\nhooks\tStop,SessionStart\ncontent_chars\t0\npreserve_newlines\t1\ntoast_gravity\ttop\ntoast_short\t0\ntoast_background\tempty\ntoast_color\tempty\ngroup\tcodex-turns\n";
        let record_path = notifications.join("config-v1");
        std::fs::write(&record_path, record).unwrap();
        std::fs::set_permissions(&record_path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let projected = run_product_probe("notify-projection", &root, &runtime, &resolver, &config);
        assert_eq!(projected.status.code(), Some(0));
        assert_eq!(projected.stdout, b"codex-upstream 9.9.9\n");
        assert_eq!(projected.stderr, b"version-stderr\n");
        let expected = concat!(
            "# codex-termux-notify-v1\n",
            "mcp_oauth_credentials_store = \"file\"\n",
            "\n",
            "[[hooks.SessionStart]]\n",
            "\n",
            "[[hooks.SessionStart.hooks]]\n",
            "type = \"command\"\n",
            "command = \"codex termux notify emit SessionStart\"\n",
            "timeout = 10\n",
            "statusMessage = \"Notify session start\"\n",
            "\n",
            "[[hooks.Stop]]\n",
            "\n",
            "[[hooks.Stop.hooks]]\n",
            "type = \"command\"\n",
            "command = \"codex termux notify emit Stop\"\n",
            "timeout = 10\n",
            "statusMessage = \"Notify turn completion\"\n",
            "\n",
        );
        assert_eq!(
            std::fs::read_to_string(config.join("config.toml")).unwrap(),
            expected
        );
        assert_eq!(
            core_notify_parse_record(record).unwrap(),
            vec!["SessionStart", "Stop"]
        );
        assert!(core_notify_parse_record(b"codex-manager-notify-v1\nhooks\tStop\n").is_none());

        let unavailable = run_product_probe(
            "notify-projection-unavailable",
            &root,
            &runtime,
            &resolver,
            &config,
        );
        assert_eq!(unavailable.status.code(), Some(0));
        assert_eq!(
            std::fs::read_to_string(config.join("config.toml")).unwrap(),
            "# codex-termux-notify-v1\nmcp_oauth_credentials_store = \"file\"\n\n"
        );

        std::fs::write(
            &record_path,
            b"codex-manager-notify-v1\nchannel\tboth\nhooks\tnone\ncontent_chars\t0\npreserve_newlines\t1\ntoast_gravity\ttop\ntoast_short\t0\ntoast_background\tempty\ntoast_color\tempty\ngroup\tcodex-turns\n",
        )
        .unwrap();
        let cleared = run_product_probe("notify-projection", &root, &runtime, &resolver, &config);
        assert_eq!(cleared.status.code(), Some(0));
        assert_eq!(
            std::fs::read_to_string(config.join("config.toml")).unwrap(),
            "# codex-termux-notify-v1\nmcp_oauth_credentials_store = \"file\"\n\n"
        );
        std::fs::write(config.join("config.toml"), b"user_setting = true\n").unwrap();
        std::fs::write(&record_path, record).unwrap();
        let preserved = run_product_probe("notify-projection", &root, &runtime, &resolver, &config);
        assert_eq!(preserved.status.code(), Some(0));
        assert_eq!(
            std::fs::read_to_string(config.join("config.toml")).unwrap(),
            "user_setting = true\n"
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_mgr4_repair_plan_is_bounded_and_read_only() {
        let (root, roots) = b2_test_roots("mgr4-root-plan");
        let generation = b2_write_root_generation(&roots, "root-v2", false, "unsupported");
        b2_activate(&roots, "root-v2");
        let state_path = CoreStatePaths::new(&roots.state_root)
            .unwrap()
            .activation_state;
        let before_resolver = protected_snapshot(&roots.resolver_path).unwrap();
        let before_state = protected_snapshot(&state_path).unwrap();
        let before_descriptor = protected_snapshot(&generation.join("generation.meta")).unwrap();

        let healthy = core_repair_plan(&roots);
        assert_eq!(
            healthy,
            CoreRepairPlan {
                action: CoreRepairAction::None,
                reason: CoreRepairReason::Healthy,
            }
        );
        assert_eq!(
            render_core_repair_plan(healthy),
            "codex-core-repair-v1\naction=none\nreason=healthy\n"
        );
        assert_eq!(
            protected_snapshot(&roots.resolver_path).unwrap(),
            before_resolver
        );
        assert_eq!(protected_snapshot(&state_path).unwrap(), before_state);
        assert_eq!(
            protected_snapshot(&generation.join("generation.meta")).unwrap(),
            before_descriptor
        );
        remove_temp_root(root);

        let (root, roots) = b2_test_roots("mgr4-legacy-plan");
        b2_write_generation(&roots, "legacy-v1", false, "unsupported");
        b2_activate(&roots, "legacy-v1");
        let legacy = core_repair_plan(&roots);
        assert_eq!(
            legacy,
            CoreRepairPlan {
                action: CoreRepairAction::Update,
                reason: CoreRepairReason::LegacyGeneration,
            }
        );
        assert_eq!(
            render_core_repair_plan(legacy),
            "codex-core-repair-v1\naction=update\nreason=legacy-generation\n"
        );
        remove_temp_root(root);

        let (root, roots) = b2_test_roots("mgr4-unavailable-plan");
        let unavailable = core_repair_plan(&roots);
        assert_eq!(
            unavailable,
            CoreRepairPlan {
                action: CoreRepairAction::Unavailable,
                reason: CoreRepairReason::CoreStateUnavailable,
            }
        );
        assert_eq!(
            render_core_repair_plan(unavailable),
            "codex-core-repair-v1\naction=unavailable\nreason=core-state-unavailable\n"
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_mgr4_request_parser_and_route_admission_are_exact() {
        assert_eq!(parse_core_repair_operation(None, None), Ok(None));
        assert_eq!(
            parse_core_repair_operation(
                Some(OsStr::new(CORE_REPAIR_REQUEST)),
                Some(OsStr::new("plan"))
            ),
            Ok(Some(CoreRepairOperation::Plan))
        );
        assert_eq!(
            parse_core_repair_operation(
                Some(OsStr::new(CORE_REPAIR_REQUEST)),
                Some(OsStr::new("apply"))
            ),
            Ok(Some(CoreRepairOperation::Apply))
        );
        for pair in [
            (Some(OsStr::new(CORE_REPAIR_REQUEST)), None),
            (None, Some(OsStr::new("plan"))),
            (Some(OsStr::new("wrong")), Some(OsStr::new("plan"))),
            (
                Some(OsStr::new(CORE_REPAIR_REQUEST)),
                Some(OsStr::new("rollback")),
            ),
        ] {
            assert_eq!(parse_core_repair_operation(pair.0, pair.1), Err(()));
        }

        let plan_route = PublicDispatchRoute::Doctor(vec![OsString::from("--json")]);
        let apply_route = PublicDispatchRoute::Update(Vec::new());
        assert!(core_repair_route_matches(
            CoreRepairOperation::Plan,
            &plan_route
        ));
        assert!(core_repair_route_matches(
            CoreRepairOperation::Apply,
            &apply_route
        ));
        assert!(!core_repair_route_matches(
            CoreRepairOperation::Plan,
            &apply_route
        ));
        assert!(!core_repair_route_matches(
            CoreRepairOperation::Apply,
            &plan_route
        ));
        assert!(!core_repair_route_matches(
            CoreRepairOperation::Plan,
            &PublicDispatchRoute::Doctor(vec![])
        ));
    }

    #[cfg(unix)]
    extern "C" {
        fn kill(pid: std::os::raw::c_int, sig: std::os::raw::c_int) -> std::os::raw::c_int;
        fn posix_openpt(flags: std::os::raw::c_int) -> std::os::raw::c_int;
        fn grantpt(fd: std::os::raw::c_int) -> std::os::raw::c_int;
        fn unlockpt(fd: std::os::raw::c_int) -> std::os::raw::c_int;
        fn ptsname(fd: std::os::raw::c_int) -> *mut std::os::raw::c_char;
    }

    #[cfg(unix)]
    #[test]
    fn test_final_exec_preserves_process_identity_and_signal_delivery() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("signal");
        let pid_file = root.join("runtime.pid");
        let stdout = root.join("signal.out");
        let stderr = root.join("signal.err");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::product_exec_probe")
            .arg("--exact")
            .env(PROBE_ROLE, "1")
            .env(PROBE_SCENARIO, "signal")
            .env(PROBE_ROOT, &root)
            .env(PROBE_RUNTIME, &runtime)
            .env(PROBE_RESOLVER, &resolver)
            .env(PROBE_CONFIG, &config)
            .env(PROBE_STDOUT, &stdout)
            .env(PROBE_STDERR, &stderr)
            .env("CODEX_TEST_PID_FILE", &pid_file)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut runtime_pid = None;
        for _ in 0..500 {
            if let Ok(value) = std::fs::read_to_string(&pid_file) {
                if let Ok(parsed) = value.trim().parse::<u32>() {
                    runtime_pid = Some(parsed);
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let runtime_pid = runtime_pid.expect("runtime did not publish a parseable pid");
        assert_eq!(runtime_pid, child.id());
        assert_eq!(unsafe { kill(child.id() as i32, 15) }, 0);
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(143));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_final_exec_preserves_tty_attachment() {
        use std::ffi::CStr;
        use std::os::fd::{FromRawFd, OwnedFd};
        let (root, runtime, resolver, config) = prepare_exec_fixture("tty");
        let master_fd = unsafe { posix_openpt(2 | 0x100) };
        assert!(master_fd >= 0);
        let master = unsafe { OwnedFd::from_raw_fd(master_fd) };
        assert_eq!(unsafe { grantpt(master_fd) }, 0);
        assert_eq!(unsafe { unlockpt(master_fd) }, 0);
        let name = unsafe { CStr::from_ptr(ptsname(master_fd)) };
        let slave_path = std::path::PathBuf::from(std::str::from_utf8(name.to_bytes()).unwrap());
        let slave = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(slave_path)
            .unwrap();
        let stdin = slave.try_clone().unwrap();
        let stdout = slave.try_clone().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::product_exec_probe")
            .arg("--exact")
            .env(PROBE_ROLE, "1")
            .env(PROBE_SCENARIO, "tty")
            .env(PROBE_ROOT, &root)
            .env(PROBE_RUNTIME, &runtime)
            .env(PROBE_RESOLVER, &resolver)
            .env(PROBE_CONFIG, &config)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(slave)
            .status()
            .unwrap();
        drop(master);
        assert_eq!(status.code(), Some(0));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_policy_and_environment_fail_before_runtime_io() {
        assert!(plan_public_dispatch(["--sandbox=read-only"]).is_err());

        let manifest = tc2_manifest(false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let helpers = tc2_helper_bindings(
            OsStr::new("/browser/open/curl"),
            OsStr::new("/browser/manual/curl"),
        );
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/missing/runtime"),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: OsStr::new("/compat"),
            helpers: &helpers,
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: None,
            tmpdir: Some("/tmp".into()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let planned = match plan_public_dispatch(["--version"]).unwrap() {
            PublicDispatchRoute::Upstream(args) => args,
            _ => unreachable!(),
        };
        assert!(matches!(
            launch_qualified_runtime(
                assets,
                &snapshot,
                OsStr::new("/cert"),
                None,
                "/missing/resolver",
                "/missing/config",
                QualifiedRuntimeLaunchOptions {
                    planned_args: &planned,
                    manager_available: false,
                },
            ),
            RuntimeLaunchError::Environment(TermuxProcessEnvError::MissingRequired("PREFIX"))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_doctor_is_bounded_read_only_and_preserves_upstream_report() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("doctor");
        let manifest = tc2_manifest(false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let compat = root.join("compat");
        let browser_open = root.join("browser/open/curl");
        let browser_manual = root.join("browser/manual/curl");
        let helpers = tc2_helper_bindings(browser_open.as_os_str(), browser_manual.as_os_str());
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime.as_os_str(),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: compat.as_os_str(),
            helpers: &helpers,
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(root.join("prefix").into_os_string()),
            tmpdir: Some(root.join("tmp").into_os_string()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let cert_file = root.join("cert.pem");
        let cert_dir = root.join("certs");
        let before = std::fs::read(&resolver).unwrap();
        let outcome = run_local_doctor_command(
            [OsString::from("--json")],
            LocalPublicDispatchContext {
                runtime_assets: assets,
                manager_artifact: ManagerArtifact::Unavailable,
                process_env: &snapshot,
                cert_file: cert_file.as_os_str(),
                cert_dir: Some(cert_dir.as_os_str()),
                resolver_path: &resolver,
                config_dir: &config,
                doctor_capability: UpstreamDoctorCapability::Supported,
                core_doctor_status: CoreDoctorStatus::Healthy,
                generation_id: "test-generation",
                generation_layout: GenerationLayout::RootCodeModeHost,
                manager_doctor_status: ManagerDoctorStatus::Unavailable,
            },
        )
        .unwrap();
        assert_eq!(outcome.exit_class, DoctorExitClass::HealthFailure);
        assert!(outcome
            .output
            .contains("\"upstream\":{\"status\":\"healthy\",\"output\":\""));
        assert!(outcome.output.contains("{\\\"status\\\":\\\"ok\\\""));
        assert!(!outcome.output.contains("SECRET-UPSTREAM"));
        assert_eq!(std::fs::read(&resolver).unwrap(), before);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_public_dispatch_update_termux_unavailable_and_doctor_usage_skip_unneeded_io() {
        let manifest = valid_manifest(false, false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/not/executed"),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: OsStr::new("/compat"),
            helpers: &[],
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: None,
            tmpdir: None,
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let context = LocalPublicDispatchContext {
            runtime_assets: assets,
            manager_artifact: ManagerArtifact::Unavailable,
            process_env: &snapshot,
            cert_file: OsStr::new("/missing/cert"),
            cert_dir: None,
            resolver_path: std::path::Path::new("/missing/resolver"),
            config_dir: std::path::Path::new("/missing/config"),
            doctor_capability: UpstreamDoctorCapability::Supported,
            core_doctor_status: CoreDoctorStatus::Healthy,
            generation_id: "test-generation",
            generation_layout: GenerationLayout::RootCodeModeHost,
            manager_doctor_status: ManagerDoctorStatus::Unavailable,
        };
        assert_eq!(
            execute_public_dispatch(PublicDispatchRoute::Update(vec!["--x".into()]), context)
                .unwrap(),
            PublicDispatchCompletion::Update(vec!["--x".into()])
        );
        assert_eq!(
            execute_public_dispatch(PublicDispatchRoute::Termux(vec![]), context).unwrap(),
            PublicDispatchCompletion::TermuxUnavailable(TERMUX_MANAGER_UNAVAILABLE_MESSAGE)
        );
        assert!(matches!(
            execute_public_dispatch(PublicDispatchRoute::Doctor(vec!["bad".into()]), context),
            Err(PublicDispatchExecutionError::Doctor(
                LocalDoctorCommandError::Usage
            ))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_install_frontend_forwards_exact_bootstrap_argv_and_rejects_invalid_bundle() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::{symlink, PermissionsExt};

        let install_source =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install.sh");
        let shell = resolve_test_shell();
        let root = temp_root("install-frontend");
        let install = root.join("install.sh");
        std::fs::copy(&install_source, &install).unwrap();
        let mut install_mode = std::fs::metadata(&install).unwrap().permissions();
        install_mode.set_mode(0o755);
        std::fs::set_permissions(&install, install_mode).unwrap();

        let bootstrap_dir = root.join("bootstrap");
        std::fs::create_dir(&bootstrap_dir).unwrap();
        let bootstrap = bootstrap_dir.join("codex-bootstrap");
        let record = root.join("argv");
        std::fs::write(
            &bootstrap,
            format!(
                "#!{}\nprintf '%s\\n' \"$#\" > \"$CODEX_TEST_INSTALL_ARGV\"\nfor arg in \"$@\"; do printf '%s\\n' \"$arg\" >> \"$CODEX_TEST_INSTALL_ARGV\"; done\nexit 37\n",
                std::str::from_utf8(shell.as_bytes()).unwrap()
            ),
        )
        .unwrap();
        let mut bootstrap_mode = std::fs::metadata(&bootstrap).unwrap().permissions();
        bootstrap_mode.set_mode(0o755);
        std::fs::set_permissions(&bootstrap, bootstrap_mode).unwrap();

        let fresh_args = ["/tmp/core", "/tmp/release", "/tmp/key"];
        let fresh = std::process::Command::new(&shell)
            .arg(&install)
            .args(fresh_args)
            .env("CODEX_TEST_INSTALL_ARGV", &record)
            .output()
            .unwrap();
        assert_eq!(fresh.status.code(), Some(37));
        assert_eq!(
            std::fs::read_to_string(&record).unwrap(),
            "3\n/tmp/core\n/tmp/release\n/tmp/key\n"
        );

        let legacy_args = [
            "upgrade-legacy",
            "/tmp/core",
            "/tmp/release",
            "/tmp/key",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ];
        let legacy = std::process::Command::new(&shell)
            .arg(&install)
            .args(legacy_args)
            .env("CODEX_TEST_INSTALL_ARGV", &record)
            .output()
            .unwrap();
        assert_eq!(legacy.status.code(), Some(37));
        assert_eq!(
            std::fs::read_to_string(&record).unwrap(),
            "5\nupgrade-legacy\n/tmp/core\n/tmp/release\n/tmp/key\naaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n"
        );

        let missing_root = temp_root("install-frontend-missing");
        let missing_install = missing_root.join("install.sh");
        std::fs::copy(&install_source, &missing_install).unwrap();
        let missing = std::process::Command::new(&shell)
            .arg(&missing_install)
            .args(fresh_args)
            .output()
            .unwrap();
        assert_eq!(missing.status.code(), Some(1));
        assert!(missing
            .stderr
            .windows(b"bundled bootstrap must be a regular executable".len())
            .any(|window| window == b"bundled bootstrap must be a regular executable"));

        let symlink_root = temp_root("install-frontend-symlink");
        let symlink_install = symlink_root.join("install.sh");
        std::fs::copy(&install_source, &symlink_install).unwrap();
        let symlink_dir = symlink_root.join("bootstrap");
        std::fs::create_dir(&symlink_dir).unwrap();
        symlink(&bootstrap, symlink_dir.join("codex-bootstrap")).unwrap();
        let linked = std::process::Command::new(&shell)
            .arg(&symlink_install)
            .args(fresh_args)
            .output()
            .unwrap();
        assert_eq!(linked.status.code(), Some(1));
        assert!(linked
            .stderr
            .windows(b"bundled bootstrap must be a regular executable".len())
            .any(|window| window == b"bundled bootstrap must be a regular executable"));

        remove_temp_root(root);
        remove_temp_root(missing_root);
        remove_temp_root(symlink_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_manager_available_exec_uses_qualified_path_and_preserves_raw_argv() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("manager");
        let result = run_product_probe("manager", &root, &runtime, &resolver, &config);
        assert_eq!(result.status.code(), Some(73));
        assert!(result
            .stdout
            .windows(b"ARGS:<status><".len())
            .any(|w| w == b"ARGS:<status><"));
        assert!(result
            .stdout
            .windows(b"HANDOFF_OK\n".len())
            .any(|w| w == b"HANDOFF_OK\n"));
        assert!(result
            .stdout
            .windows(b"ARTIFACT_PROBE_CLEARED\n".len())
            .any(|w| w == b"ARTIFACT_PROBE_CLEARED\n"));
        assert!(result.stdout.windows(2).any(|w| w == [0xff, b'm']));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ProtectedSnapshot {
        bytes: Vec<u8>,
        dev: u64,
        ino: u64,
        mode: u32,
        uid: u32,
        gid: u32,
        len: u64,
        mtime: i64,
        mtime_nsec: i64,
    }

    #[cfg(unix)]
    fn protected_snapshot(path: &std::path::Path) -> std::io::Result<ProtectedSnapshot> {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(path)?;
        Ok(ProtectedSnapshot {
            bytes: std::fs::read(path)?,
            dev: metadata.dev(),
            ino: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            len: metadata.len(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        })
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "explicit real-Termux smoke"]
    fn test_real_termux_resolver_and_installed_launcher_remain_read_only() {
        let prefix = std::env::var_os("PREFIX").expect("Termux PREFIX required");
        let prefix = std::path::PathBuf::from(prefix);
        let resolver = prefix.join("etc/resolv.conf");
        let launcher = prefix.join("bin/codex");
        let resolver_before = protected_snapshot(&resolver).unwrap();
        let launcher_before = protected_snapshot(&launcher).unwrap();
        let (root, runtime, _, config) = prepare_exec_fixture("real-smoke");
        let direct = std::process::Command::new(&runtime)
            .arg("--version")
            .output()
            .unwrap();
        let through = run_product_probe("version", &root, &runtime, &resolver, &config);
        assert_eq!(through.status.code(), direct.status.code());
        assert_eq!(through.stdout, direct.stdout);
        assert_eq!(through.stderr, direct.stderr);
        assert_eq!(protected_snapshot(&resolver).unwrap(), resolver_before);
        assert_eq!(protected_snapshot(&launcher).unwrap(), launcher_before);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn b2_test_roots(label: &str) -> (std::path::PathBuf, LocalCoreRoots) {
        let root = temp_root(label);
        let roots = LocalCoreRoots {
            generation_root: root.join("generations"),
            state_root: root.join("state"),
            config_dir: root.join("state/config"),
            resolver_path: root.join("resolv.conf"),
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
            openssl: root.join("openssl"),
            curl: root.join("curl"),
        };
        std::fs::create_dir(&roots.generation_root).unwrap();
        std::fs::write(&roots.resolver_path, b"nameserver 127.0.0.1\n").unwrap();
        std::fs::write(&roots.cert_file, b"test-cert").unwrap();
        std::fs::create_dir(&roots.cert_dir).unwrap();
        (root, roots)
    }

    #[cfg(unix)]
    fn b2_write_generation(
        roots: &LocalCoreRoots,
        generation_id: &str,
        manager: bool,
        doctor: &str,
    ) -> std::path::PathBuf {
        let generation_dir = roots.generation_root.join(generation_id);
        std::fs::create_dir(&generation_dir).unwrap();
        std::fs::create_dir(generation_dir.join("compat")).unwrap();
        let fake = write_fake_runtime(&generation_dir);
        let runtime = generation_dir.join("runtime");
        std::fs::rename(fake, &runtime).unwrap();
        std::fs::copy(
            &runtime,
            generation_dir.join("compat").join(CODE_MODE_HOST_FILE),
        )
        .unwrap();
        if manager {
            std::fs::copy(&runtime, generation_dir.join("manager")).unwrap();
        }
        let descriptor = format!(
            concat!(
                "codex-local-generation-v1\n",
                "generation_id\t{}\n",
                "upstream_package_identity\t@openai/codex\n",
                "upstream_package_version\t9.9.9\n",
                "source_artifact_digest\tsource-digest\n",
                "expected_platform\t{}\n",
                "expected_architecture\t{}\n",
                "patch_policy_id\ttermux-policy-v1\n",
                "patch_report\tqualified\n",
                "runtime_digest\truntime-digest\n",
                "core_artifact_digest\tcore-digest\n",
                "manager_artifact_digest\t{}\n",
                "core_api_identity\t{}\n",
                "persistent_schema_identity\t{}\n",
                "qualification\tqualified\n",
                "creation_metadata\ttest-fixture\n",
                "upstream_doctor\t{}\n",
                "helper_count\t0\n",
            ),
            generation_id,
            std::env::consts::OS,
            std::env::consts::ARCH,
            if manager { "manager-digest" } else { "-" },
            CORE_API_IDENTITY,
            PERSISTENT_SCHEMA_IDENTITY,
            doctor,
        );
        std::fs::write(generation_dir.join("generation.meta"), descriptor).unwrap();
        generation_dir
    }

    #[cfg(unix)]
    fn b2_write_root_generation(
        roots: &LocalCoreRoots,
        generation_id: &str,
        manager: bool,
        doctor: &str,
    ) -> std::path::PathBuf {
        let generation_dir = b2_write_generation(roots, generation_id, manager, doctor);
        std::fs::rename(
            generation_dir.join("compat").join(CODE_MODE_HOST_FILE),
            generation_dir.join(CODE_MODE_HOST_FILE),
        )
        .unwrap();
        std::fs::remove_dir(generation_dir.join("compat")).unwrap();
        let descriptor = std::fs::read_to_string(generation_dir.join("generation.meta")).unwrap();
        let descriptor = descriptor.replacen(
            "codex-local-generation-v1\n",
            "codex-local-generation-v2\n",
            1,
        );
        std::fs::write(generation_dir.join("generation.meta"), descriptor).unwrap();
        generation_dir
    }

    #[cfg(unix)]
    fn b2_add_helper(generation_dir: &std::path::Path) {
        let descriptor_path = generation_dir.join("generation.meta");
        let original = std::fs::read_to_string(&descriptor_path).unwrap();
        let (descriptor, index) =
            if original.contains("helper\ttermux-browser-manual-v1\tbrowser-manual-digest\n") {
                (
                    original
                        .replacen("helper_count\t2\n", "helper_count\t3\n", 1)
                        .replace(
                            "helper\ttermux-browser-manual-v1\tbrowser-manual-digest\n",
                            concat!(
                                "helper\ttermux-browser-manual-v1\tbrowser-manual-digest\n",
                                "helper\thelper-a\thelper-digest\n",
                            ),
                        ),
                    2,
                )
            } else {
                (
                    original.replacen(
                        "helper_count\t0\n",
                        "helper_count\t1\nhelper\thelper-a\thelper-digest\n",
                        1,
                    ),
                    0,
                )
            };
        assert_ne!(descriptor, original);
        std::fs::create_dir(generation_dir.join("helpers")).unwrap();
        std::fs::write(
            generation_dir.join("helpers").join(index.to_string()),
            b"helper-content",
        )
        .unwrap();
        std::fs::write(descriptor_path, descriptor).unwrap();
    }

    #[cfg(unix)]
    #[cfg(unix)]
    fn b2_add_tc2_browser_helpers(generation_dir: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;

        let descriptor_path = generation_dir.join("generation.meta");
        let original = std::fs::read_to_string(&descriptor_path).unwrap();
        let descriptor = if original
            .contains("helper\ttermux-browser-open-v1\tbrowser-open-digest\n")
            && original.contains("helper\ttermux-browser-manual-v1\tbrowser-manual-digest\n")
        {
            original.clone()
        } else {
            let updated = original.replacen(
                "helper_count\t0\n",
                concat!(
                    "helper_count\t2\n",
                    "helper\ttermux-browser-open-v1\tbrowser-open-digest\n",
                    "helper\ttermux-browser-manual-v1\tbrowser-manual-digest\n",
                ),
                1,
            );
            assert_ne!(updated, original, "TC-2 fixture requires helper_count=0");
            updated
        };
        let open_dir = generation_dir.join("browser/open");
        let manual_dir = generation_dir.join("browser/manual");
        std::fs::create_dir_all(&open_dir).unwrap();
        std::fs::create_dir_all(&manual_dir).unwrap();
        for (path, contents) in [
            (open_dir.join("curl"), b"browser-open".as_slice()),
            (manual_dir.join("curl"), b"browser-manual".as_slice()),
        ] {
            if !path.exists() {
                std::fs::write(&path, contents).unwrap();
            }
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        if descriptor != original {
            std::fs::write(descriptor_path, descriptor).unwrap();
        }
    }

    #[cfg(unix)]
    #[cfg(unix)]
    fn b2_activate(roots: &LocalCoreRoots, generation_id: &str) -> GenerationPointerState {
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        prepare_core_state_paths(&paths).unwrap();
        std::fs::create_dir(&roots.config_dir).unwrap();
        let state = plan_initial_pointer_state(generation_id).unwrap();
        activate_pointer_state(&paths, None, &state).unwrap();
        state
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b2_loader_requires_one_current_generation_and_never_falls_back() {
        let (root, roots) = b2_test_roots("b2-current-only");
        assert!(matches!(
            load_activated_generation(&roots),
            Err(LocalProductError::NoCurrentGeneration)
        ));

        b2_write_generation(&roots, "good", false, "unsupported");
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        prepare_core_state_paths(&paths).unwrap();
        std::fs::create_dir(&roots.config_dir).unwrap();
        let good = plan_initial_pointer_state("good").unwrap();
        activate_pointer_state(&paths, None, &good).unwrap();
        let missing = plan_activation_pointer_state(&good, "missing-current").unwrap();
        activate_pointer_state(&paths, Some(&good), &missing).unwrap();

        assert!(matches!(
            load_activated_generation(&roots),
            Err(LocalProductError::Io {
                operation: "read activated generation descriptor",
                ..
            })
        ));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b2_loader_rejects_malformed_or_incomplete_descriptor() {
        let (root, roots) = b2_test_roots("b2-malformed");
        let generation_dir = roots.generation_root.join("broken");
        std::fs::create_dir(&generation_dir).unwrap();
        std::fs::write(generation_dir.join("generation.meta"), b"not-the-format\n").unwrap();
        b2_activate(&roots, "broken");
        assert!(matches!(
            load_activated_generation(&roots),
            Err(LocalProductError::Descriptor(
                "generation descriptor format is unsupported"
            ))
        ));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b2_loader_binds_runtime_and_optional_manager_from_one_generation() {
        let (root, roots) = b2_test_roots("b2-manager");
        let generation_dir = b2_write_generation(&roots, "g1", true, "supported");
        b2_activate(&roots, "g1");
        let loaded = load_activated_generation(&roots).unwrap();
        assert_eq!(loaded.runtime_path, generation_dir.join("runtime"));
        assert_eq!(loaded.generation_layout, GenerationLayout::LegacyCompat);
        assert_eq!(
            loaded.code_mode_host_path,
            generation_dir.join("compat").join(CODE_MODE_HOST_FILE)
        );
        assert_eq!(loaded.compatibility_dir, generation_dir.join("compat"));
        assert_eq!(loaded.manager_path, Some(generation_dir.join("manager")));
        assert_eq!(
            loaded.manifest.manager_artifact_digest.as_deref(),
            Some("manager-digest")
        );
        assert_eq!(
            loaded.doctor_capability,
            UpstreamDoctorCapability::Supported
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r3_root_code_mode_host_layout_is_selected_and_legacy_layout_stages() {
        let (root, roots) = b2_test_roots("r3-root-code-mode-host");
        let generation_dir = b2_write_root_generation(&roots, "root-v2", false, "supported");
        b2_activate(&roots, "root-v2");
        let loaded = load_activated_generation(&roots).unwrap();
        assert_eq!(loaded.generation_layout, GenerationLayout::RootCodeModeHost);
        assert_eq!(loaded.generation_layout.as_str(), "root-code-mode-host-v2");
        assert_eq!(
            loaded.code_mode_host_path,
            generation_dir.join(CODE_MODE_HOST_FILE)
        );
        assert_eq!(loaded.compatibility_dir, generation_dir);

        use std::os::unix::fs::symlink;
        let host_path = generation_dir.join(CODE_MODE_HOST_FILE);
        let outside_host = generation_dir
            .parent()
            .unwrap()
            .join("outside-code-mode-host");
        std::fs::rename(&host_path, &outside_host).unwrap();
        symlink(&outside_host, &host_path).unwrap();
        assert!(matches!(
            load_activated_generation(&roots),
            Err(LocalProductError::UnsafeSource(
                "activated generation code-mode host must be a regular file"
            ))
        ));

        let (legacy_root, legacy) = b2_test_roots("r3-legacy-code-mode-host");
        let legacy_generation = b2_write_generation(&legacy, "legacy-v1", false, "supported");
        b3_write_required_release_files(&legacy_generation);
        stage_local_generation(&legacy_generation, &roots.generation_root).unwrap();
        let staged = roots.generation_root.join("legacy-v1");
        let staged_loaded = load_local_generation(&staged).unwrap();
        assert_eq!(
            staged_loaded.generation_layout,
            GenerationLayout::LegacyCompat
        );
        assert_eq!(staged_loaded.generation_layout.as_str(), "legacy-compat-v1");
        assert_eq!(
            std::fs::read(staged.join("compat").join(CODE_MODE_HOST_FILE)).unwrap(),
            std::fs::read(legacy_generation.join("compat").join(CODE_MODE_HOST_FILE)).unwrap()
        );

        remove_temp_root(root);
        remove_temp_root(legacy_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b2_doctor_and_manager_unavailable_use_loaded_generation_without_fallback() {
        let (root, roots) = b2_test_roots("b2-local-routes");
        b2_write_generation(&roots, "g1", false, "unsupported");
        b2_activate(&roots, "g1");
        let process_env = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(root.join("prefix").into_os_string()),
            tmpdir: Some(root.join("tmp").into_os_string()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let doctor = execute_activated_route(
            PublicDispatchRoute::Doctor(vec![OsString::from("--json")]),
            &roots,
            &process_env,
        )
        .unwrap();
        match doctor {
            PublicDispatchCompletion::Doctor(outcome) => {
                assert_eq!(outcome.exit_class, DoctorExitClass::HealthFailure);
                assert!(outcome
                    .output
                    .contains("\"upstream\":{\"status\":\"unsupported\",\"output\":\"\"}"));
            }
            other => panic!("unexpected doctor result: {other:?}"),
        }
        assert_eq!(
            execute_activated_route(PublicDispatchRoute::Termux(vec![]), &roots, &process_env)
                .unwrap(),
            PublicDispatchCompletion::TermuxUnavailable(TERMUX_MANAGER_UNAVAILABLE_MESSAGE)
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    const MAIN_PROBE_ROLE: &str = "CODEX_R2_MAIN_PROBE";
    #[cfg(unix)]
    const MAIN_PROBE_ARGS: &str = "CODEX_R2_MAIN_ARGS";

    #[cfg(unix)]
    #[test]
    fn public_main_probe() {
        if std::env::var(MAIN_PROBE_ROLE).as_deref() != Ok("1") {
            return;
        }
        let scenario = std::env::var(MAIN_PROBE_ARGS).unwrap();
        let args = match scenario.as_str() {
            "version" => vec![OsString::from("--version")],
            "update" => vec![
                OsString::from("update"),
                OsString::from("--help"),
                OsString::from("--disable"),
                OsString::from("code_mode_host"),
            ],
            "update-bare" => vec![OsString::from("update")],
            "update-help" => vec![OsString::from("update"), OsString::from("--help")],
            "manager" => vec![OsString::from("termux"), OsString::from("status")],
            "doctor" => vec![OsString::from("doctor"), OsString::from("--json")],
            "doctor-human" => vec![OsString::from("doctor")],
            "doctor-color" => vec![OsString::from("doctor"), OsString::from("--color")],
            "doctor-overflow" => {
                std::env::set_var("CODEX_TEST_DOCTOR_LARGE", "1");
                vec![OsString::from("doctor"), OsString::from("--json")]
            }
            other => panic!("unknown main probe scenario {other}"),
        };
        let code = run_public_main(args);
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
        std::process::exit(code);
    }

    #[cfg(unix)]
    fn b2_public_main_fixture(label: &str, manager: bool) -> std::path::PathBuf {
        let root = temp_root(label);
        let home = root.join("home");
        let prefix = root.join("prefix");
        let generation_root = home.join(".local/lib/codex/core/generations");
        let state_root = home.join(".local/share/codex/core");
        let roots = LocalCoreRoots {
            generation_root,
            state_root: state_root.clone(),
            config_dir: state_root.join("config"),
            resolver_path: prefix.join("etc/resolv.conf"),
            cert_file: prefix.join("etc/tls/cert.pem"),
            cert_dir: prefix.join("etc/tls/certs"),
            openssl: prefix.join("bin/openssl"),
            curl: prefix.join("bin/curl"),
        };
        std::fs::create_dir_all(&roots.generation_root).unwrap();
        std::fs::create_dir_all(roots.state_root.parent().unwrap()).unwrap();
        std::fs::create_dir_all(prefix.join("etc/tls/certs")).unwrap();
        std::fs::write(&roots.resolver_path, b"nameserver 127.0.0.1\n").unwrap();
        std::fs::write(&roots.cert_file, b"test-cert").unwrap();
        let generation = b2_write_generation(&roots, "g1", manager, "unsupported");
        b2_add_tc2_browser_helpers(&generation);
        b2_activate(&roots, "g1");
        std::fs::create_dir(root.join("tmp")).unwrap();
        root
    }

    #[cfg(unix)]
    fn b2_public_main_doctor_fixture(label: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let root = b2_public_main_fixture(label, false);
        let script_source =
            std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/script");
        let script = root.join("prefix/bin/script");
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::copy(script_source, &script).unwrap();
        let mut script_mode = std::fs::metadata(&script).unwrap().permissions();
        script_mode.set_mode(0o755);
        std::fs::set_permissions(&script, script_mode).unwrap();
        let generation = root.join("home/.local/lib/codex/core/generations/g1");
        std::fs::rename(
            generation.join("compat").join(CODE_MODE_HOST_FILE),
            generation.join(CODE_MODE_HOST_FILE),
        )
        .unwrap();
        std::fs::remove_dir(generation.join("compat")).unwrap();
        let descriptor = generation.join("generation.meta");
        let contents = std::fs::read_to_string(&descriptor).unwrap();
        let contents = contents.replacen(
            "upstream_doctor\tunsupported\n",
            "upstream_doctor\tsupported\n",
            1,
        );
        let contents = contents.replacen(LEGACY_GENERATION_FORMAT, LOCAL_GENERATION_FORMAT, 1);
        assert_ne!(contents, std::fs::read_to_string(&descriptor).unwrap());
        std::fs::write(descriptor, contents).unwrap();
        root
    }

    #[cfg(unix)]
    fn run_public_main_probe(root: &std::path::Path, scenario: &str) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::public_main_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(MAIN_PROBE_ROLE, "1")
            .env(MAIN_PROBE_ARGS, scenario)
            .env("HOME", root.join("home"))
            .env("PREFIX", root.join("prefix"))
            .env("TMPDIR", root.join("tmp"))
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn run_public_main_probe_tty(root: &std::path::Path, scenario: &str) -> std::process::Output {
        use std::os::unix::ffi::OsStrExt;

        let script =
            std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/script");
        let executable = doctor_shell_quote(std::env::current_exe().unwrap().as_os_str());
        let executable = std::str::from_utf8(executable.as_bytes()).unwrap();
        let command = format!("{executable} tests::public_main_probe --exact --nocapture");
        std::process::Command::new(script)
            .args(["-qefc"])
            .arg(command)
            .arg("/dev/null")
            .env(MAIN_PROBE_ROLE, "1")
            .env(MAIN_PROBE_ARGS, scenario)
            .env("HOME", root.join("home"))
            .env("PREFIX", root.join("prefix"))
            .env("TMPDIR", root.join("tmp"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn test_ux1_update_human_output_contract_is_version_centric() {
        assert_eq!(
            render_update_header("0.154.0", "0.155.0"),
            "Updating Codex 0.154.0 -> 0.155.0..."
        );
        assert_eq!(
            render_update_header("0.155.0", "0.155.0"),
            "Updating the Termux release for Codex 0.155.0..."
        );
        assert!(update_progress_enabled(true, true));
        assert!(!update_progress_enabled(true, false));
        assert!(!update_progress_enabled(false, true));
        assert!(!update_progress_enabled(false, false));
        assert_eq!(
            render_update_failure(&LocalProductError::SignatureRejected),
            "Release signature verification failed. ❌"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b2_real_main_path_loads_current_and_execs_upstream_and_manager() {
        let root = b2_public_main_fixture("b2-main-version", false);
        let version = run_public_main_probe(&root, "version");
        assert_eq!(version.status.code(), Some(0));
        assert!(version.stdout.ends_with(b"codex-upstream 9.9.9\n"));
        assert_eq!(version.stderr, b"version-stderr\n");
        remove_temp_root(&root);

        let root = b2_public_main_fixture("b2-main-manager", true);
        let manager = run_public_main_probe(&root, "manager");
        assert_eq!(manager.status.code(), Some(73));
        assert!(manager
            .stdout
            .windows(b"ARGS:<status>".len())
            .any(|w| w == b"ARGS:<status>"));
        remove_temp_root(root);

        let root = b2_public_main_fixture("b2-main-update", false);
        let update = run_public_main_probe(&root, "update");
        assert_eq!(update.status.code(), Some(2));
        assert!(!update.stdout.windows(b"ARGS:".len()).any(|w| w == b"ARGS:"));
        assert!(update
            .stderr
            .windows(UPDATE_USAGE.len())
            .any(|window| window == UPDATE_USAGE.as_bytes()));
        remove_temp_root(root);

        let root = b2_public_main_fixture("b2-main-update-bare", false);
        let update = run_public_main_probe(&root, "update-bare");
        assert_eq!(update.status.code(), Some(1));
        assert!(!update.stdout.windows(b"ARGS:".len()).any(|w| w == b"ARGS:"));
        assert!(update
            .stderr
            .windows(b"OpenSSL".len())
            .any(|w| w == b"OpenSSL"));
        assert!(!update.stderr.windows(b"ARGS:".len()).any(|w| w == b"ARGS:"));
        remove_temp_root(root);

        let root = b2_public_main_fixture("b2-main-update-help", false);
        let update = run_public_main_probe(&root, "update-help");
        assert_eq!(update.status.code(), Some(0));
        assert!(update
            .stdout
            .windows(UPDATE_USAGE.len())
            .any(|window| window == UPDATE_USAGE.as_bytes()));
        assert!(update.stderr.is_empty());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r3_public_main_human_doctor_composes_upstream_and_termux_reports() {
        let root = b2_public_main_doctor_fixture("r3-main-doctor-human");
        let result = run_public_main_probe(&root, "doctor-human");
        assert_eq!(result.status.code(), Some(1));
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(stdout.contains("upstream doctor ok"));
        assert!(!stdout.contains("[Upstream Codex doctor]"));
        assert!(!stdout.contains("status: healthy\n"));
        assert!(stdout.contains("upstream doctor ok"));
        assert!(stdout.contains("[Termux doctor]"));
        for section in ["Runtime", "Support", "Wrapper", "State", "Store"] {
            assert!(stdout.contains(&format!("\n{section}\n")));
        }
        assert!(stdout.contains("Codex Termux Wrapper Doctor · generation g1"));
        assert!(stdout.contains("generation_id: g1"));
        assert!(stdout.contains("layout: root-code-mode-host-v2"));
        assert!(stdout.contains("code_mode_host: healthy"));
        assert!(stdout.contains("sandbox: unsupported (bwrap is not used)"));
        assert!(stdout.contains("[Summary]"));
        assert!(!stdout.contains("sk-upstream-secret"));
        assert!(!stdout.contains("SECRET-UPSTREAM-STDERR"));
        assert!(result.stderr.is_empty(), "stderr: {:?}", result.stderr);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r9_2_public_main_doctor_color_overrides_termux_no_color_on_tty() {
        let root = b2_public_main_doctor_fixture("r9-2-main-doctor-color");
        let result = run_public_main_probe_tty(&root, "doctor-color");
        assert_eq!(result.status.code(), Some(1));
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(
            stdout.contains("\x1b[1mupstream doctor ok\x1b[22m"),
            "stdout={stdout:?} stderr={:?}",
            result.stderr
        );
        assert!(stdout.contains("\x1b[1mCodex Termux Wrapper Doctor"));
        assert!(!stdout.contains("[Upstream Codex doctor]"));
        assert!(stdout.contains("[Termux doctor]"));
        assert!(!stdout.contains("sk-upstream-secret"));
        assert!(result.stderr.is_empty(), "stderr: {:?}", result.stderr);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r3_public_main_doctor_caps_upstream_output() {
        let root = b2_public_main_doctor_fixture("r3-main-doctor-overflow");
        let result = run_public_main_probe(&root, "doctor-overflow");
        assert_eq!(result.status.code(), Some(1));
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(stdout.contains("\"upstream\":{\"status\":\"unhealthy\",\"output\":\"\"}"));
        assert!(stdout.contains("\"summary\":{\"status\":\"unhealthy\"}"));
        assert!(!stdout.contains("xxxxxxxx"));
        assert!(result.stderr.is_empty(), "stderr: {:?}", result.stderr);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn b2_assert_public_version_rejects_symlink<F>(
        label: &str,
        manager: bool,
        helper: bool,
        mutate: F,
    ) where
        F: FnOnce(&std::path::Path),
    {
        let root = b2_public_main_fixture(label, manager);
        let generation_dir = root.join("home/.local/lib/codex/core/generations/g1");
        if helper {
            b2_add_helper(&generation_dir);
        }
        mutate(&generation_dir);
        let result = run_public_main_probe(&root, "version");
        assert_eq!(result.status.code(), Some(1));
        assert!(
            result.stderr.starts_with(b"codex: "),
            "stderr: {:?}",
            result.stderr
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r2_public_main_rejects_nested_generation_symlink_paths() {
        use std::os::unix::fs::symlink;

        b2_assert_public_version_rejects_symlink(
            "m2-r2-generation-root-symlink",
            false,
            false,
            |generation_dir| {
                let outside = generation_dir.parent().unwrap().join("outside-generation");
                std::fs::rename(generation_dir, &outside).unwrap();
                symlink(outside, generation_dir).unwrap();
            },
        );
        b2_assert_public_version_rejects_symlink(
            "m2-r2-runtime-symlink",
            false,
            false,
            |generation_dir| {
                let path = generation_dir.join("runtime");
                let outside = generation_dir.parent().unwrap().join("outside-runtime");
                std::fs::rename(&path, &outside).unwrap();
                symlink(outside, path).unwrap();
            },
        );
        b2_assert_public_version_rejects_symlink(
            "m2-r2-compat-symlink",
            false,
            false,
            |generation_dir| {
                let path = generation_dir.join("compat");
                let outside = generation_dir.parent().unwrap().join("outside-compat");
                std::fs::rename(&path, &outside).unwrap();
                symlink(outside, path).unwrap();
            },
        );
        b2_assert_public_version_rejects_symlink(
            "m2-r2-manager-symlink",
            true,
            false,
            |generation_dir| {
                let path = generation_dir.join("manager");
                let outside = generation_dir.parent().unwrap().join("outside-manager");
                std::fs::rename(&path, &outside).unwrap();
                symlink(outside, path).unwrap();
            },
        );
        b2_assert_public_version_rejects_symlink(
            "m2-r2-helper-parent-symlink",
            false,
            true,
            |generation_dir| {
                let path = generation_dir.join("helpers");
                let outside = generation_dir.parent().unwrap().join("outside-helpers");
                std::fs::rename(&path, &outside).unwrap();
                symlink(outside, path).unwrap();
            },
        );
        b2_assert_public_version_rejects_symlink(
            "m2-r2-helper-leaf-symlink",
            false,
            true,
            |generation_dir| {
                let path = generation_dir.join("helpers/2");
                let outside = generation_dir.parent().unwrap().join("outside-helper");
                std::fs::rename(&path, &outside).unwrap();
                symlink(outside, path).unwrap();
            },
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r2_public_main_doctor_json_preserves_probe_failure_envelope() {
        let root = b2_public_main_doctor_fixture("m2-r2-doctor-probe-failure");
        std::fs::remove_file(root.join("prefix/etc/resolv.conf")).unwrap();
        let result = run_public_main_probe(&root, "doctor");
        assert_eq!(result.status.code(), Some(1));
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(stdout.contains("\"upstream\":{\"status\":\"unhealthy\",\"output\":\"\"}"));
        assert!(stdout.contains("\"termux_core\":{\"status\":\"healthy\",\"generation_id\":"));
        assert!(stdout.contains("\"summary\":{\"status\":\"unhealthy\"}"));
        assert!(!stdout.contains("resolv.conf"));
        assert!(result.stderr.is_empty(), "stderr: {:?}", result.stderr);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b2_update_and_invalid_sandbox_need_no_generation_loader() {
        assert_eq!(
            plan_public_dispatch(["update"]).unwrap(),
            PublicDispatchRoute::Update(vec![])
        );
        assert_eq!(
            plan_public_dispatch(["update", "--local", "/tmp/release"]).unwrap(),
            PublicDispatchRoute::Update(vec![
                OsString::from("--local"),
                OsString::from("/tmp/release")
            ])
        );
        assert_eq!(
            plan_public_dispatch(["update", "--help"]).unwrap(),
            PublicDispatchRoute::Update(vec![OsString::from("--help")])
        );
        assert_eq!(run_public_main([OsString::from("--sandbox=read-only")]), 2);
    }

    #[cfg(unix)]
    fn b3_candidate_entries(generation_root: &std::path::Path) -> Vec<std::ffi::OsString> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(generation_root).unwrap() {
            let entry = entry.unwrap();
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with(".candidate-")
            {
                entries.push(entry.file_name());
            }
        }
        entries
    }

    #[cfg(unix)]
    fn b3_write_required_release_files(generation_dir: &std::path::Path) {
        std::fs::write(
            generation_dir.join("release.manifest"),
            b"staging-only release fixture\n",
        )
        .unwrap();
        std::fs::write(
            generation_dir.join("release.sig"),
            b"staging-only signature fixture",
        )
        .unwrap();
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum M2R1GenerationFaultTiming {
        Before,
        After,
    }

    #[cfg(unix)]
    struct M2R1GenerationFaultIo {
        fail_call: usize,
        timing: M2R1GenerationFaultTiming,
        calls: usize,
        inner: FsGenerationPublishIo,
    }

    #[cfg(unix)]
    impl M2R1GenerationFaultIo {
        fn new(fail_call: usize, timing: M2R1GenerationFaultTiming) -> Self {
            Self {
                fail_call,
                timing,
                calls: 0,
                inner: FsGenerationPublishIo,
            }
        }

        fn around(
            &mut self,
            action: impl FnOnce(&mut FsGenerationPublishIo) -> std::io::Result<()>,
        ) -> std::io::Result<()> {
            self.calls += 1;
            let current = self.calls;
            if current == self.fail_call && self.timing == M2R1GenerationFaultTiming::Before {
                return Err(std::io::Error::other(
                    "injected M2-R1 generation fault before durable call",
                ));
            }
            action(&mut self.inner)?;
            if current == self.fail_call && self.timing == M2R1GenerationFaultTiming::After {
                return Err(std::io::Error::other(
                    "injected M2-R1 generation fault after durable call",
                ));
            }
            Ok(())
        }
    }

    #[cfg(unix)]
    impl GenerationPublishIo for M2R1GenerationFaultIo {
        fn sync_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.around(|inner| inner.sync_file(path))
        }

        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.around(|inner| inner.sync_dir(path))
        }

        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
            self.inner.rename(from, to)
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_generation_publication_faults_preserve_complete_or_absent_boundary() {
        let (target_root, target) = b2_test_roots("m2-r1-generation-fault-target");
        let (source_root, source) = b2_test_roots("m2-r1-generation-fault-source");
        let source_generation =
            b2_write_generation(&source, "m2-r1-generation", false, "supported");
        b3_write_required_release_files(&source_generation);
        std::fs::create_dir(source_generation.join("compat/nested")).unwrap();
        std::fs::write(
            source_generation.join("compat/nested/asset"),
            b"durability-boundary",
        )
        .unwrap();

        let mut successful_io =
            M2R1GenerationFaultIo::new(usize::MAX, M2R1GenerationFaultTiming::Before);
        stage_local_generation_with_io(
            &source_generation,
            &target.generation_root,
            &mut successful_io,
        )
        .unwrap();
        let durable_calls = successful_io.calls;
        assert!(durable_calls >= 5);
        let final_path = target.generation_root.join("m2-r1-generation");
        assert!(final_path.is_dir());
        std::fs::remove_dir_all(&final_path).unwrap();

        for timing in [
            M2R1GenerationFaultTiming::Before,
            M2R1GenerationFaultTiming::After,
        ] {
            for fail_call in 1..=durable_calls {
                let mut fault_io = M2R1GenerationFaultIo::new(fail_call, timing);
                assert!(stage_local_generation_with_io(
                    &source_generation,
                    &target.generation_root,
                    &mut fault_io,
                )
                .is_err());
                if fail_call == durable_calls {
                    assert!(final_path.is_dir());
                    std::fs::remove_dir_all(&final_path).unwrap();
                } else {
                    assert!(!final_path.exists());
                }
                assert!(b3_candidate_entries(&target.generation_root).is_empty());
            }
        }

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    struct M2R1GenerationCollisionIo {
        inner: FsGenerationPublishIo,
    }

    #[cfg(unix)]
    impl GenerationPublishIo for M2R1GenerationCollisionIo {
        fn sync_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.inner.sync_file(path)
        }

        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            self.inner.sync_dir(path)
        }

        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
            std::fs::create_dir(to)?;
            std::fs::write(to.join("sentinel"), b"preserve")?;
            self.inner.rename(from, to)
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_generation_collision_race_never_replaces_existing_directory() {
        let (target_root, target) = b2_test_roots("m2-r1-generation-collision-race-target");
        let (source_root, source) = b2_test_roots("m2-r1-generation-collision-race-source");
        let source_generation =
            b2_write_generation(&source, "m2-r1-generation-race", false, "supported");
        b3_write_required_release_files(&source_generation);
        let final_path = target.generation_root.join("m2-r1-generation-race");
        let mut io = M2R1GenerationCollisionIo {
            inner: FsGenerationPublishIo,
        };
        assert!(matches!(
            stage_local_generation_with_io(&source_generation, &target.generation_root, &mut io,),
            Err(LocalProductError::GenerationCollision)
        ));
        assert_eq!(
            std::fs::read(final_path.join("sentinel")).unwrap(),
            b"preserve"
        );
        assert!(b3_candidate_entries(&target.generation_root).is_empty());

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b3_stages_complete_inactive_generation_and_preserves_active_state() {
        let (target_root, target) = b2_test_roots("b3-target");
        b2_write_generation(&target, "active", false, "unsupported");
        b2_activate(&target, "active");
        let state_paths = CoreStatePaths::new(&target.state_root).unwrap();
        let state_before = std::fs::read(&state_paths.activation_state).unwrap();

        let (source_root, source) = b2_test_roots("b3-source");
        let source_generation = b2_write_generation(&source, "next", false, "supported");
        b3_write_required_release_files(&source_generation);
        let nested = source_generation.join("compat/nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("asset.txt"), b"compat-asset").unwrap();
        std::fs::write(source_generation.join("ignored-source-file"), b"ignore-me").unwrap();

        assert_eq!(
            stage_local_generation(&source_generation, &target.generation_root).unwrap(),
            "next"
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
        assert_eq!(
            load_activated_generation(&target).unwrap().generation_id,
            "active"
        );
        let staged = target.generation_root.join("next");
        assert_eq!(
            load_local_generation(&staged).unwrap().generation_id,
            "next"
        );
        assert_eq!(
            std::fs::read(staged.join("compat/nested/asset.txt")).unwrap(),
            b"compat-asset"
        );
        assert!(staged.join("release.manifest").is_file());
        assert!(staged.join("release.sig").is_file());
        assert!(!staged.join("ignored-source-file").exists());
        assert!(b3_candidate_entries(&target.generation_root).is_empty());

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b3_stages_optional_manager_and_declared_helper_only() {
        let (target_root, target) = b2_test_roots("b3-target-manager");
        let (source_root, source) = b2_test_roots("b3-source-manager");
        let source_generation = b2_write_generation(&source, "with-manager", true, "unsupported");
        b3_write_required_release_files(&source_generation);
        let descriptor_path = source_generation.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap().replace(
            "helper_count\t0\n",
            "helper_count\t1\nhelper\thelper-a\thelper-digest\n",
        );
        std::fs::write(&descriptor_path, descriptor).unwrap();
        std::fs::create_dir(source_generation.join("helpers")).unwrap();
        std::fs::write(source_generation.join("helpers/0"), b"helper-content").unwrap();
        std::fs::write(source_generation.join("helpers/unlisted"), b"not-declared").unwrap();

        stage_local_generation(&source_generation, &target.generation_root).unwrap();
        let staged = target.generation_root.join("with-manager");
        let loaded = load_local_generation(&staged).unwrap();
        assert_eq!(loaded.manager_path, Some(staged.join("manager")));
        assert_eq!(loaded.helper_paths, vec![staged.join("helpers/0")]);
        assert_eq!(
            std::fs::read(staged.join("helpers/0")).unwrap(),
            b"helper-content"
        );
        assert!(!staged.join("helpers/unlisted").exists());

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b3_rejects_malformed_descriptor_without_candidate_residue() {
        let (target_root, target) = b2_test_roots("b3-target-malformed");
        let (source_root, source) = b2_test_roots("b3-source-malformed");
        let source_generation = source.generation_root.join("broken");
        std::fs::create_dir(&source_generation).unwrap();
        std::fs::write(source_generation.join("generation.meta"), b"broken\n").unwrap();

        assert!(matches!(
            stage_local_generation(&source_generation, &target.generation_root),
            Err(LocalProductError::Descriptor(
                "generation descriptor format is unsupported"
            ))
        ));
        assert!(!target.generation_root.join("broken").exists());
        assert!(b3_candidate_entries(&target.generation_root).is_empty());

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b3_rejects_symlink_content_and_cleans_private_candidate() {
        use std::os::unix::fs::symlink;
        let (target_root, target) = b2_test_roots("b3-target-symlink");
        let (source_root, source) = b2_test_roots("b3-source-symlink");
        let source_generation = b2_write_generation(&source, "unsafe", false, "unsupported");
        b3_write_required_release_files(&source_generation);
        let outside = source_root.join("outside-secret");
        std::fs::write(&outside, b"must-not-copy").unwrap();
        symlink(&outside, source_generation.join("compat/link")).unwrap();

        assert!(matches!(
            stage_local_generation(&source_generation, &target.generation_root),
            Err(LocalProductError::UnsafeSource(
                "local generation compatibility tree contains a symlink or special file"
            ))
        ));
        assert!(!target.generation_root.join("unsafe").exists());
        assert!(b3_candidate_entries(&target.generation_root).is_empty());

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b3_final_collision_fails_without_overwrite() {
        let (target_root, target) = b2_test_roots("b3-target-collision");
        let (source_root, source) = b2_test_roots("b3-source-collision");
        let source_generation = b2_write_generation(&source, "same", false, "unsupported");
        let final_path = target.generation_root.join("same");
        std::fs::create_dir(&final_path).unwrap();
        std::fs::write(final_path.join("sentinel"), b"keep").unwrap();

        assert!(matches!(
            stage_local_generation(&source_generation, &target.generation_root),
            Err(LocalProductError::GenerationCollision)
        ));
        assert_eq!(std::fs::read(final_path.join("sentinel")).unwrap(), b"keep");
        assert!(b3_candidate_entries(&target.generation_root).is_empty());

        remove_temp_root(target_root);
        remove_temp_root(source_root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b3_descriptor_generation_id_is_single_component_and_binds_current() {
        let (root, roots) = b2_test_roots("b3-id");
        let generation_dir = b2_write_generation(&roots, "good", false, "unsupported");
        let descriptor_path = generation_dir.join("generation.meta");
        let original = std::fs::read_to_string(&descriptor_path).unwrap();
        std::fs::write(
            &descriptor_path,
            original.replace("generation_id\tgood\n", "generation_id\t../escape\n"),
        )
        .unwrap();
        assert!(matches!(
            load_local_generation(&generation_dir),
            Err(LocalProductError::StateFormat(
                StateFormatError::IdentityControl("generation_id")
            ))
        ));

        std::fs::write(
            &descriptor_path,
            original.replace("generation_id\tgood\n", "generation_id\tother\n"),
        )
        .unwrap();
        b2_activate(&roots, "good");
        assert!(matches!(
            load_activated_generation(&roots),
            Err(LocalProductError::Descriptor(
                "activated generation descriptor id does not match current"
            ))
        ));
        remove_temp_root(root);
    }

    #[test]
    fn test_arh1_top_level_rollback_is_not_core_owned() {
        for original in [
            vec![OsString::from("rollback")],
            vec![OsString::from("rollback"), OsString::from("extra")],
        ] {
            match plan_public_dispatch(original.clone()).unwrap() {
                PublicDispatchRoute::Upstream(planned) => {
                    assert_eq!(&planned[2..], original.as_slice());
                }
                other => panic!("unexpected route: {other:?}"),
            }
        }
    }

    #[cfg(unix)]
    const UPDATE_PROBE_ROLE: &str = "CODEX_R2_UPDATE_PROBE";
    #[cfg(unix)]
    const UPDATE_PROBE_SOURCE: &str = "CODEX_R2_UPDATE_SOURCE";
    #[cfg(unix)]
    const UPDATE_PROBE_REMOTE: &str = "CODEX_R2_UPDATE_REMOTE";
    #[cfg(unix)]
    const UPDATE_PROBE_CHANNEL: &str = "CODEX_R4_UPDATE_CHANNEL";
    #[cfg(unix)]
    const UPDATE_PROBE_FORCE: &str = "CODEX_ARH1_UPDATE_FORCE";
    #[cfg(unix)]
    const UPDATE_PROBE_BUILD_LOCAL: &str = "CODEX_RALD1_BUILD_LOCAL";

    #[cfg(unix)]
    fn b4_termux_openssl() -> std::path::PathBuf {
        let prefix = std::env::var_os("PREFIX").expect("Termux PREFIX is required for B4 proof");
        let openssl = std::path::PathBuf::from(prefix).join("bin/openssl");
        assert!(openssl.is_file(), "Termux OpenSSL is required for B4 proof");
        openssl
    }

    #[cfg(unix)]
    fn b4_generate_release_keypair(
        openssl: &std::path::Path,
        private_key: &std::path::Path,
        public_key: &std::path::Path,
    ) {
        std::fs::create_dir_all(public_key.parent().unwrap()).unwrap();
        let generated = std::process::Command::new(openssl)
            .args(["genpkey", "-algorithm", "ED25519", "-out"])
            .arg(private_key)
            .env_clear()
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(generated.success(), "generate test Ed25519 private key");
        let exported = std::process::Command::new(openssl)
            .args(["pkey", "-in"])
            .arg(private_key)
            .args(["-pubout", "-out"])
            .arg(public_key)
            .env_clear()
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(exported.success(), "export test Ed25519 public key");
    }

    #[cfg(unix)]
    fn b4_sign_release_manifest_to(
        generation_dir: &std::path::Path,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
        output_name: &str,
    ) {
        let signed = std::process::Command::new(openssl)
            .args(["pkeyutl", "-sign", "-rawin", "-inkey"])
            .arg(private_key)
            .arg("-in")
            .arg(generation_dir.join("release.manifest"))
            .arg("-out")
            .arg(generation_dir.join(output_name))
            .env_clear()
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(signed.success(), "sign test release manifest");
    }

    #[cfg(unix)]
    fn b4_sign_release_manifest(
        generation_dir: &std::path::Path,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
    ) {
        b4_sign_release_manifest_to(generation_dir, openssl, private_key, "release.sig");
    }

    #[cfg(unix)]
    fn b4_sign_update_index(
        index_path: &std::path::Path,
        signature_path: &std::path::Path,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
    ) {
        let signed = std::process::Command::new(openssl)
            .args(["pkeyutl", "-sign", "-rawin", "-inkey"])
            .arg(private_key)
            .arg("-in")
            .arg(index_path)
            .arg("-out")
            .arg(signature_path)
            .env_clear()
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(signed.success(), "sign test update index");
    }

    #[cfg(unix)]
    fn b4_public_key_from_private(
        openssl: &std::path::Path,
        private_key: &std::path::Path,
    ) -> ReleasePublicKey {
        let output = std::process::Command::new(openssl)
            .args(["pkey", "-in"])
            .arg(private_key)
            .args(["-pubout", "-outform", "DER"])
            .env_clear()
            .stderr(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 44);
        let mut raw = [0u8; 32];
        raw.copy_from_slice(&output.stdout[12..]);
        ReleasePublicKey(raw)
    }

    #[cfg(unix)]
    fn b4_write_probe_runtime(
        generation_dir: &std::path::Path,
        version_exit: i32,
        doctor_exit: i32,
    ) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let runtime = generation_dir.join("runtime");
        std::fs::write(
            &runtime,
            format!(
                r#"#!{shell}
if [ "${{CODEX_TEST_REQUIRE_NO_ACQUISITION:-}}" = "1" ]; then
  for acquisition in "$HOME"/.local/lib/codex/core/generations/.acquire-*; do
    [ ! -e "$acquisition" ] || exit 96
  done
fi
if [ "$1" = "-c" ]; then
  [ "$2" = 'sandbox_mode="danger-full-access"' ] || exit 90
  shift 2
fi
case "$1" in
  --version) exit {version_exit} ;;
  doctor) exit {doctor_exit} ;;
  *) exit 92 ;;
esac
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&runtime).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(runtime, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b4_write_signed_release(
        generation_dir: &std::path::Path,
        release_sequence: u64,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
    ) {
        let descriptor = std::fs::read_to_string(generation_dir.join("generation.meta")).unwrap();
        if descriptor.contains("helper_count\t0\n") {
            b2_add_tc2_browser_helpers(generation_dir);
        }
        let files = b4_exact_release_inventory(generation_dir, openssl);
        b4_write_signed_release_inventory(
            generation_dir,
            release_sequence,
            openssl,
            private_key,
            &files,
        );
    }

    #[cfg(unix)]
    fn b4_exact_release_inventory(
        generation_dir: &std::path::Path,
        openssl: &std::path::Path,
    ) -> Vec<ReleaseFileEntry> {
        use std::os::unix::fs::PermissionsExt;

        let loaded = load_local_generation(generation_dir).unwrap();
        exact_release_file_paths(generation_dir, &loaded)
            .unwrap()
            .into_iter()
            .map(|relative_path| ReleaseFileEntry {
                sha256: openssl_sha256(openssl, &generation_dir.join(&relative_path)).unwrap(),
                mode: std::fs::symlink_metadata(generation_dir.join(&relative_path))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                relative_path,
            })
            .collect()
    }

    #[cfg(unix)]
    fn b4_write_signed_release_inventory(
        generation_dir: &std::path::Path,
        release_sequence: u64,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
        files: &[ReleaseFileEntry],
    ) {
        b4_write_signed_release_inventory_with_authority(
            generation_dir,
            release_sequence,
            openssl,
            private_key,
            None,
            files,
        );
    }

    #[cfg(unix)]
    fn b4_write_signed_release_inventory_with_authority(
        generation_dir: &std::path::Path,
        release_sequence: u64,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
        authority_private_key: Option<&std::path::Path>,
        files: &[ReleaseFileEntry],
    ) {
        use std::fmt::Write as _;

        let loaded = load_local_generation(generation_dir).unwrap();
        let release_public_key = b4_public_key_from_private(openssl, private_key).to_hex();
        let release_format = if files.iter().any(|file| file.relative_path == "core") {
            LOCAL_RELEASE_FORMAT_V4
        } else {
            LOCAL_RELEASE_FORMAT
        };
        let mut manifest = format!(
            concat!(
                "{}\n",
                "generation_id\t{}\n",
                "release_sequence\t{}\n",
                "channel\t{}\n",
                "expected_platform\t{}\n",
                "expected_architecture\t{}\n",
                "core_api_identity\t{}\n",
                "persistent_schema_identity\t{}\n",
                "release_public_key\t{}\n",
                "file_count\t{}\n",
            ),
            release_format,
            loaded.generation_id,
            release_sequence,
            LOCAL_RELEASE_CHANNEL,
            loaded.manifest.expected_platform,
            loaded.manifest.expected_architecture,
            CORE_API_IDENTITY,
            PERSISTENT_SCHEMA_IDENTITY,
            release_public_key,
            files.len(),
        );
        for relative_path in files {
            writeln!(
                &mut manifest,
                "file\t{}\t{}\t{:04o}",
                relative_path.relative_path, relative_path.sha256, relative_path.mode
            )
            .unwrap();
        }
        let manifest_path = generation_dir.join("release.manifest");
        std::fs::write(&manifest_path, manifest).unwrap();
        b4_sign_release_manifest(generation_dir, openssl, private_key);
        let authority_path = generation_dir.join("release-authority.sig");
        if let Some(authority_private_key) = authority_private_key {
            b4_sign_release_manifest_to(
                generation_dir,
                openssl,
                authority_private_key,
                "release-authority.sig",
            );
        } else if authority_path.exists() {
            std::fs::remove_file(authority_path).unwrap();
        }
    }

    #[cfg(unix)]
    fn b6_write_octal(field: &mut [u8], value: u64) {
        field.fill(b'0');
        let value = format!("{value:o}");
        let start = field.len() - value.len() - 1;
        field[start..start + value.len()].copy_from_slice(value.as_bytes());
        field[field.len() - 1] = 0;
    }

    #[cfg(unix)]
    fn b6_tar_header(path: &str, kind: u8, size: usize) -> [u8; 512] {
        let mut header = [0u8; 512];
        header[..path.len()].copy_from_slice(path.as_bytes());
        b6_write_octal(
            &mut header[100..108],
            if kind == b'5' { 0o755 } else { 0o644 },
        );
        b6_write_octal(&mut header[108..116], 1001);
        b6_write_octal(&mut header[116..124], 1001);
        b6_write_octal(&mut header[124..136], size as u64);
        b6_write_octal(&mut header[136..148], 1_787_793_845);
        header[148..156].fill(b' ');
        header[156] = kind;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
        let checksum = format!("{checksum:06o}");
        header[148..154].copy_from_slice(checksum.as_bytes());
        header[154] = 0;
        header[155] = b' ';
        header
    }

    #[cfg(unix)]
    fn b6_append_tar_entry(tar: &mut Vec<u8>, path: &str, kind: u8, data: &[u8]) {
        tar.extend_from_slice(&b6_tar_header(path, kind, data.len()));
        tar.extend_from_slice(data);
        let padding = (512 - data.len() % 512) % 512;
        tar.resize(tar.len() + padding, 0);
    }

    #[cfg(unix)]
    fn b6_static_aarch64_elf(runtime: bool) -> Vec<u8> {
        let mut bytes = vec![0u8; 120];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&183u16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        bytes[64..68].copy_from_slice(&1u32.to_le_bytes());
        if runtime {
            for source in [
                "/etc/resolv.conf",
                "/etc/codex/managed_config.toml",
                "/etc/codex/config.toml",
                "/etc/codex/requirements.toml",
                "/etc/resolv.conf",
            ] {
                bytes.extend_from_slice(source.as_bytes());
                bytes.push(0);
            }
        }
        bytes
    }

    #[cfg(unix)]
    fn b6_write_official_shape_archive_with_runtime(
        gzip: &std::path::Path,
        archive: &std::path::Path,
        runtime: &[u8],
    ) {
        use std::io::Write as _;

        let host = b6_static_aarch64_elf(false);
        let package = b"{\n  \"layoutVersion\": 1,\n  \"version\": \"0.150.1\",\n  \"target\": \"aarch64-unknown-linux-musl\",\n  \"variant\": \"codex\",\n  \"entrypoint\": \"bin/codex\",\n  \"resourcesDir\": \"codex-resources\",\n  \"pathDir\": \"codex-path\"\n}\n";
        let mut tar = Vec::new();
        for (path, kind, data) in [
            ("bin/", b'5', &[][..]),
            ("bin/codex", b'0', runtime),
            ("bin/codex-code-mode-host", b'0', host.as_slice()),
            ("codex-package.json", b'0', package.as_slice()),
            ("codex-path/", b'5', &[][..]),
            ("codex-path/rg", b'0', b"excluded-rg".as_slice()),
            ("codex-resources/", b'5', &[][..]),
            ("codex-resources/bwrap", b'0', b"excluded-bwrap".as_slice()),
            ("codex-resources/zsh/", b'5', &[][..]),
            ("codex-resources/zsh/bin/", b'5', &[][..]),
            (
                "codex-resources/zsh/bin/zsh",
                b'0',
                b"excluded-zsh".as_slice(),
            ),
        ] {
            b6_append_tar_entry(&mut tar, path, kind, data);
        }
        tar.resize(tar.len() + 1024, 0);

        let output = std::fs::File::create(archive).unwrap();
        let mut child = std::process::Command::new(gzip)
            .args(["-cn"])
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(output)
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&tar).unwrap();
        assert!(child.wait().unwrap().success());
    }

    #[cfg(unix)]
    fn b6_write_official_shape_archive(
        gzip: &std::path::Path,
        archive: &std::path::Path,
    ) -> Vec<u8> {
        let runtime = b6_static_aarch64_elf(true);
        b6_write_official_shape_archive_with_runtime(gzip, archive, &runtime);
        runtime
    }

    #[cfg(unix)]
    fn b8_compile_static_probe_runtime(root: &std::path::Path) -> std::path::PathBuf {
        let prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let clang = prefix.join("bin/clang");
        assert!(
            clang.is_file(),
            "Termux clang is required for B8 integration proof"
        );
        let source = root.join("b8-static-probe.S");
        let runtime = root.join("b8-static-probe");
        std::fs::write(
            &source,
            concat!(
                ".text\n",
                ".global _start\n",
                ".type _start,%function\n",
                "_start:\n",
                "  mov x0, #0\n",
                "  mov x8, #93\n",
                "  svc #0\n",
                ".section .rodata\n",
                ".ascii \"/etc/resolv.conf\\0\"\n",
                ".ascii \"/etc/resolv.conf\\0\"\n",
                ".ascii \"/etc/codex/managed_config.toml\\0\"\n",
                ".ascii \"/etc/codex/config.toml\\0\"\n",
                ".ascii \"/etc/codex/requirements.toml\\0\"\n",
            ),
        )
        .unwrap();
        let output = std::process::Command::new(&clang)
            .args([
                "-nostdlib",
                "-static",
                "-Wl,--build-id=none",
                "-Wl,-e,_start",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&runtime)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "clang stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert!(std::process::Command::new(&runtime)
            .args(["-c", "sandbox_mode=\"danger-full-access\"", "--version"])
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new(&runtime)
            .args(["-c", "sandbox_mode=\"danger-full-access\"", "doctor"])
            .status()
            .unwrap()
            .success());
        runtime
    }

    #[cfg(unix)]
    fn b10_release_core_from_env() -> Option<std::path::PathBuf> {
        let input = std::env::var_os("CODEX_B10_RELEASE_CORE")?;
        let core = std::fs::canonicalize(input).unwrap();
        assert!(core.is_absolute());
        assert!(core.is_file());
        Some(core)
    }

    #[cfg(unix)]
    fn b10_install_network_denial_sentinel(prefix: &std::path::Path) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let curl = prefix.join("bin/curl");
        let log = prefix.join("b10-network-acquisition-attempted");
        let shell = resolve_test_shell();
        let script = format!(
            "#!{}\nprintf '%s\\n' 'network acquisition attempted' >> \"$PREFIX/b10-network-acquisition-attempted\"\nexit 97\n",
            shell.display()
        );
        std::fs::write(&curl, script).unwrap();
        let mut permissions = std::fs::metadata(&curl).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&curl, permissions).unwrap();
        assert!(!log.exists());
        log
    }

    #[cfg(unix)]
    fn b10_build_signed_release(
        root: &std::path::Path,
        core: &std::path::Path,
        generation_id: &str,
        release_sequence: u64,
        openssl: &std::path::Path,
        private_key: &std::path::Path,
        public_key: &std::path::Path,
    ) -> std::path::PathBuf {
        use std::ffi::OsString;

        let release_root = root.join(format!("release-{generation_id}"));
        std::fs::create_dir_all(&release_root).unwrap();
        let live_prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let gzip = live_prefix.join("bin/gzip");
        assert!(
            gzip.is_file(),
            "Termux gzip is required for B10 qualification"
        );
        let runtime_source = b8_compile_static_probe_runtime(&release_root);
        let raw_runtime = std::fs::read(&runtime_source).unwrap();
        let archive = release_root.join("codex-package-aarch64-unknown-linux-musl.tar.gz");
        b6_write_official_shape_archive_with_runtime(&gzip, &archive, &raw_runtime);
        let archive_sha256 = openssl_sha256(openssl, &archive).unwrap();
        let core_sha256 = openssl_sha256(openssl, core).unwrap();
        let generation = release_root.join("generation");
        let args = vec![
            OsString::from("build"),
            OsString::from("--version"),
            OsString::from("0.150.1"),
            OsString::from("--archive"),
            archive.as_os_str().to_owned(),
            OsString::from("--archive-sha256"),
            OsString::from(&archive_sha256),
            OsString::from("--generation-id"),
            OsString::from(generation_id),
            OsString::from("--core"),
            core.as_os_str().to_owned(),
            OsString::from("--creation-metadata"),
            OsString::from(format!("m2-b10-offline-{generation_id}")),
            OsString::from("--gzip"),
            gzip.as_os_str().to_owned(),
            OsString::from("--openssl"),
            openssl.as_os_str().to_owned(),
            OsString::from("--output"),
            generation.as_os_str().to_owned(),
        ];
        assert_eq!(codex_release_builder::run_from_args(args), 0);
        b4_write_signed_release(&generation, release_sequence, openssl, private_key);
        let (manifest, loaded) =
            verify_local_release_bundle(&generation, openssl, public_key).unwrap();
        assert_eq!(manifest.generation_id, generation_id);
        assert_eq!(manifest.release_sequence, release_sequence);
        assert_eq!(loaded.manifest.core_artifact_digest, core_sha256);
        assert_eq!(loaded.manifest.source_artifact_digest, archive_sha256);
        generation
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b10_slice0_release_fixture_and_network_denial_boundary_are_exact() {
        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b10-slice0-offline-boundary");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let generation = b10_build_signed_release(
            &root,
            &core,
            "b10-offline-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let network_log = b10_install_network_denial_sentinel(&prefix);
        let (manifest, loaded) =
            verify_local_release_bundle(&generation, &openssl, &public_key).unwrap();
        assert_eq!(manifest.generation_id, "b10-offline-g0");
        assert_eq!(manifest.release_sequence, 1);
        assert_eq!(
            loaded.manifest.core_artifact_digest,
            openssl_sha256(&openssl, &core).unwrap()
        );
        assert!(!network_log.exists());
        assert!(!prefix.join("bin/codex").exists());
        assert!(!home
            .join(".local/share/codex/core/activation-state")
            .exists());
        assert!(tmp.is_dir());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b10_slice1_fresh_offline_install_from_release_artifact() {
        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b10-slice1-offline-install");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let generation = b10_build_signed_release(
            &root,
            &core,
            "b10-offline-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );
        let (signed_release, _) =
            verify_local_release_bundle(&generation, &openssl, &public_key).unwrap();
        let core_sha256 = openssl_sha256(&openssl, &core).unwrap();

        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let network_log = b10_install_network_denial_sentinel(&prefix);
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                core.as_os_str(),
                generation.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert!(!network_log.exists());

        let installed_core = prefix.join("bin/codex");
        assert_eq!(
            openssl_sha256(&openssl, &installed_core).unwrap(),
            core_sha256
        );
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "b10-offline-g0");
        assert_eq!(state.previous, None);
        assert_eq!(state.previous_key, None);
        assert_eq!(state.update_key, state.current_key);
        let (installed_release, installed_loaded) = verify_installed_local_release(
            &roots,
            "b10-offline-g0",
            state.current_key,
            "B10 offline bootstrap installed generation id mismatch",
        )
        .unwrap();
        assert_eq!(installed_release, signed_release);
        assert_eq!(installed_loaded.manifest.core_artifact_digest, core_sha256);
        assert!(std::process::Command::new(&installed_core)
            .arg("--version")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .status()
            .unwrap()
            .success());
        assert!(!network_log.exists());
        assert!(std::fs::read_dir(&tmp).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-bootstrap.")
        }));
        m2_b1_assert_no_transaction_files(&paths);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice1_fresh_root_public_path_includes_doctor() {
        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b11-slice1-fresh-public-path");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let g0 = b10_build_signed_release(
            &root,
            &core,
            "b11-fresh-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );
        let g1 = b10_build_signed_release(
            &root,
            &core,
            "b11-fresh-g1",
            2,
            &openssl,
            &private_key,
            &public_key,
        );

        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let network_log = b10_install_network_denial_sentinel(&prefix);
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let bootstrap_output = std::process::Command::new(&bootstrap)
            .args([core.as_os_str(), g0.as_os_str(), public_key.as_os_str()])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .output()
            .unwrap();
        assert_eq!(
            bootstrap_output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            bootstrap_output.stdout,
            bootstrap_output.stderr
        );
        assert!(!network_log.exists());

        let installed_core = prefix.join("bin/codex");
        let run_installed = |args: &[&str]| {
            std::process::Command::new(&installed_core)
                .args(args)
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .output()
                .unwrap()
        };

        let version = run_installed(&["--version"]);
        assert!(
            version.status.success(),
            "stdout={:?} stderr={:?}",
            version.stdout,
            version.stderr
        );

        let doctor = run_installed(&["doctor"]);
        assert_eq!(
            doctor.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            doctor.stdout,
            doctor.stderr
        );
        let doctor_output = String::from_utf8_lossy(&doctor.stdout);
        assert!(doctor_output.contains("[Upstream]\nstatus: healthy"));
        assert!(doctor_output.contains("[Manager]\nstatus: unavailable"));
        assert!(doctor_output.contains("[Summary]\nstatus: degraded"));

        let update = std::process::Command::new(&installed_core)
            .args(["update", "--local"])
            .arg(&g1)
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(
            update.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            update.stdout,
            update.stderr
        );
        assert!(!network_log.exists());

        let updated_doctor = run_installed(&["doctor"]);
        assert_eq!(updated_doctor.status.code(), Some(1));
        let updated_doctor_output = String::from_utf8_lossy(&updated_doctor.stdout);
        assert!(updated_doctor_output.contains("[Upstream]\nstatus: healthy"));
        assert!(updated_doctor_output.contains("[Summary]\nstatus: degraded"));

        let rollback = run_installed(&["update", "--rollback"]);
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        assert!(!network_log.exists());

        let rolled_back_doctor = run_installed(&["doctor"]);
        assert_eq!(rolled_back_doctor.status.code(), Some(1));
        let rolled_back_doctor_output = String::from_utf8_lossy(&rolled_back_doctor.stdout);
        assert!(rolled_back_doctor_output.contains("[Upstream]\nstatus: healthy"));
        assert!(rolled_back_doctor_output.contains("[Summary]\nstatus: degraded"));
        assert!(!network_log.exists());

        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "b11-fresh-g0");
        assert_eq!(state.previous.as_deref(), Some("b11-fresh-g1"));
        m2_b1_assert_no_transaction_files(&paths);
        assert!(std::fs::read_dir(&tmp).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-bootstrap.")
        }));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2_bootstrap_rejects_legacy_entrypoint_without_persistent_state() {
        use std::os::unix::fs::PermissionsExt;

        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b11-slice2-legacy-entrypoint");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let legacy_entrypoint = prefix.join("bin/codex");
        std::fs::write(&legacy_entrypoint, b"legacy-codex-entrypoint\n").unwrap();
        let mut legacy_permissions = std::fs::metadata(&legacy_entrypoint).unwrap().permissions();
        legacy_permissions.set_mode(0o755);
        std::fs::set_permissions(&legacy_entrypoint, legacy_permissions).unwrap();
        let legacy_bytes = std::fs::read(&legacy_entrypoint).unwrap();
        let legacy_mode = std::fs::metadata(&legacy_entrypoint)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        let network_log = b10_install_network_denial_sentinel(&prefix);
        let generation = b10_build_signed_release(
            &root,
            &core,
            "b11-legacy-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );

        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                core.as_os_str(),
                generation.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert!(output
            .stderr
            .windows(b"existing Codex entrypoint differs from authenticated Core artifact".len())
            .any(|window| {
                window == b"existing Codex entrypoint differs from authenticated Core artifact"
            }));
        assert_eq!(std::fs::read(&legacy_entrypoint).unwrap(), legacy_bytes);
        assert_eq!(
            std::fs::metadata(&legacy_entrypoint)
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            legacy_mode
        );
        assert!(!network_log.exists());
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());
        assert!(!home
            .join(".local/share/codex/core/activation-state")
            .exists());
        assert!(std::fs::read_dir(prefix.join("bin")).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex.bootstrap.")
        }));
        assert!(std::fs::read_dir(&tmp).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-bootstrap.")
        }));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn b11_run_legacy_handoff(
        core: &std::path::Path,
        release: &std::path::Path,
        public_key: &std::path::Path,
        expected_legacy_digest: &str,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        std::process::Command::new(bootstrap)
            .args([
                OsStr::new("upgrade-legacy"),
                core.as_os_str(),
                release.as_os_str(),
                public_key.as_os_str(),
                OsStr::new(expected_legacy_digest),
            ])
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_KEY_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_CORE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_EXPECTED_ENTRYPOINT_DIGEST_ENV)
            .output()
            .unwrap()
    }
    #[cfg(unix)]
    fn b11_run_legacy_handoff_before_commit(
        core: &std::path::Path,
        release: &std::path::Path,
        public_key: &std::path::Path,
        expected_legacy_digest: &str,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        std::process::Command::new(bootstrap)
            .args([
                OsStr::new("upgrade-legacy"),
                core.as_os_str(),
                release.as_os_str(),
                public_key.as_os_str(),
                OsStr::new(expected_legacy_digest),
            ])
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env("CODEX_TEST_LEGACY_HANDOFF_STOP_BEFORE_COMMIT", "1")
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_KEY_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_CORE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_EXPECTED_ENTRYPOINT_DIGEST_ENV)
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn b11_write_legacy_entrypoint(
        prefix: &std::path::Path,
        openssl: &std::path::Path,
        bytes: &[u8],
    ) -> (std::path::PathBuf, String, u32) {
        use std::os::unix::fs::PermissionsExt;

        let entrypoint = prefix.join("bin/codex");
        std::fs::write(&entrypoint, bytes).unwrap();
        let mut permissions = std::fs::metadata(&entrypoint).unwrap().permissions();
        permissions.set_mode(0o751);
        std::fs::set_permissions(&entrypoint, permissions).unwrap();
        let digest = openssl_sha256(openssl, &entrypoint).unwrap();
        let mode = std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777;
        (entrypoint, digest, mode)
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2a_classifies_exact_entrypoint_digests() {
        let expected = "a".repeat(64);
        let core = "b".repeat(64);
        assert_eq!(
            classify_legacy_entrypoint_digest(&expected, &expected, &core).unwrap(),
            LegacyEntrypointClass::Legacy
        );
        assert_eq!(
            classify_legacy_entrypoint_digest(&core, &expected, &core).unwrap(),
            LegacyEntrypointClass::Core
        );
        assert!(classify_legacy_entrypoint_digest("c", &expected, &core).is_err());
        assert!(classify_legacy_entrypoint_digest(&expected, &core, &core).is_err());
        assert!(classify_legacy_entrypoint_digest(&expected, &"A".repeat(64), &core).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2a_upgrade_grammar_and_conflicts_fail_without_mutation() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let usage = std::process::Command::new(&bootstrap)
            .arg("upgrade-legacy")
            .output()
            .unwrap();
        assert_eq!(usage.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&usage.stderr).contains("usage: codex-bootstrap"));

        let root = temp_root("b11-slice2a-conflicts");
        let openssl = b4_termux_openssl();
        let source = b4_source_roots(&root, &openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "b11-conflict-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &openssl, &private_key);
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let (entrypoint, legacy_digest, legacy_mode) =
            b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-conflict\n");
        let legacy_bytes = std::fs::read(&entrypoint).unwrap();

        let wrong_digest = "0".repeat(64);
        let wrong = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &wrong_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(wrong.status.code(), Some(1));
        assert_eq!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            legacy_mode
        );
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());
        assert!(!home
            .join(".local/share/codex/core/activation-state")
            .exists());

        let core_bytes = std::fs::read(&core).unwrap();
        std::fs::write(&entrypoint, &core_bytes).unwrap();
        let mut core_permissions = std::fs::metadata(&entrypoint).unwrap().permissions();
        core_permissions.set_mode(0o755);
        std::fs::set_permissions(&entrypoint, core_permissions).unwrap();
        let same_core = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            same_core.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            same_core.stdout,
            same_core.stderr
        );
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state_before_symlink = std::fs::read(&paths.activation_state).unwrap();
        assert_eq!(
            read_pointer_state(&paths).unwrap().unwrap().current,
            "b11-conflict-g0"
        );

        std::fs::remove_file(&entrypoint).unwrap();
        let legacy_target = root.join("legacy-target");
        std::fs::write(&legacy_target, &legacy_bytes).unwrap();
        symlink(&legacy_target, &entrypoint).unwrap();
        let symlink_attempt = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(symlink_attempt.status.code(), Some(1));
        assert!(std::fs::symlink_metadata(&entrypoint)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(home
            .join(".local/lib/codex/core/release-public-key.pem")
            .is_file());
        assert_eq!(
            std::fs::read(&paths.activation_state).unwrap(),
            state_before_symlink
        );

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2b_legacy_handoff_prepares_and_retries_exact_state() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b11-slice2b-prepared");
        let openssl = b4_termux_openssl();
        let source = b4_source_roots(&root, &openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "b11-prepared-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &openssl, &private_key);
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let roots = b7_public_roots(&home, &prefix);
        let initial = b7_seed_initial_release(&release, &home, &prefix, &public_key);
        assert_eq!(initial.current, "b11-prepared-g0");
        assert_eq!(initial.previous, None);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state_before = std::fs::read(&paths.activation_state).unwrap();
        let (entrypoint, expected_legacy_digest, legacy_mode) =
            b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-prepared\n");
        let legacy_bytes = std::fs::read(&entrypoint).unwrap();

        let prepared = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            prepared.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            prepared.stdout,
            prepared.stderr
        );
        assert_eq!(
            std::fs::read(&paths.activation_state).unwrap(),
            state_before
        );
        assert_eq!(
            openssl_sha256(&openssl, &entrypoint).unwrap(),
            openssl_sha256(&openssl, &core).unwrap()
        );
        assert_ne!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());
        m2_b1_assert_no_transaction_files(&paths);

        let completed_state = std::fs::read(&paths.activation_state).unwrap();
        let completed = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            completed.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            completed.stdout,
            completed.stderr
        );
        assert_eq!(
            std::fs::read(&paths.activation_state).unwrap(),
            completed_state
        );
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        assert!(
            !std::fs::read_dir(prefix.join("bin")).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".codex.legacy-handoff-")
            })
        );
        assert_eq!(legacy_mode, 0o751);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_legacy_handoff_retries_parent_sync_after_post_rename_failure() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("m2-r1-legacy-entrypoint-retry");
        let openssl = b4_termux_openssl();
        let source = b4_source_roots(&root, &openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "m2-r1-legacy-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &openssl, &private_key);
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let (entrypoint, expected_legacy_digest, legacy_mode) =
            b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-post-rename\n");
        let bin = prefix.join("bin");
        let mut bin_permissions = std::fs::metadata(&bin).unwrap().permissions();
        bin_permissions.set_mode(0o300);
        std::fs::set_permissions(&bin, bin_permissions).unwrap();

        let first = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(first.status.code(), Some(1));
        assert_eq!(
            openssl_sha256(&openssl, &entrypoint).unwrap(),
            openssl_sha256(&openssl, &core).unwrap()
        );
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let prepared = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(prepared.current, "m2-r1-legacy-g0");
        assert_eq!(prepared.previous, None);
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );

        let second = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(second.status.code(), Some(1));
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(prepared.clone()));

        let mut bin_permissions = std::fs::metadata(&bin).unwrap().permissions();
        bin_permissions.set_mode(0o755);
        std::fs::set_permissions(&bin, bin_permissions).unwrap();
        let completed = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            completed.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            completed.stdout,
            completed.stderr
        );
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(prepared));
        assert_eq!(legacy_mode, 0o751);
        assert!(std::fs::read_dir(&bin).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex.legacy-handoff-")
        }));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2c_legacy_handoff_replaces_only_digest_bound_entrypoint() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b11-slice2c-legacy");
        let openssl = b4_termux_openssl();
        let source = b4_source_roots(&root, &openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "b11-legacy-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &openssl, &private_key);
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let (entrypoint, expected_legacy_digest, legacy_mode) =
            b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-e2e\n");
        let legacy_bytes = std::fs::read(&entrypoint).unwrap();

        let output = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert_eq!(
            openssl_sha256(&openssl, &entrypoint).unwrap(),
            openssl_sha256(&openssl, &core).unwrap()
        );
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        assert_eq!(legacy_mode, 0o751);
        assert_ne!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);

        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "b11-legacy-g0");
        assert_eq!(state.previous, None);
        assert_eq!(state.previous_key, None);
        assert_eq!(state.update_key, state.current_key);
        let (_, installed) = verify_installed_local_release(
            &roots,
            "b11-legacy-g0",
            state.current_key,
            "B11 legacy handoff generation id mismatch",
        )
        .unwrap();
        assert_eq!(installed.generation_id, "b11-legacy-g0");
        assert!(home
            .join(".local/lib/codex/core/release-public-key.pem")
            .is_file());
        assert!(
            !std::fs::read_dir(prefix.join("bin")).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".codex.legacy-handoff-")
            })
        );
        assert!(std::fs::read_dir(&tmp).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-bootstrap.")
        }));
        m2_b1_assert_no_transaction_files(&paths);
        remove_temp_root(root);
    }
    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2b_interruption_leaves_prepared_state_and_legacy_entrypoint() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b11-slice2b-interruption");
        let openssl = b4_termux_openssl();
        let source = b4_source_roots(&root, &openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "b11-interrupt-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &openssl, &private_key);
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let (entrypoint, expected_legacy_digest, legacy_mode) =
            b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-interrupted\n");
        let legacy_bytes = std::fs::read(&entrypoint).unwrap();
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();

        let interrupted = b11_run_legacy_handoff_before_commit(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            interrupted.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            interrupted.stdout,
            interrupted.stderr
        );
        assert_eq!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            legacy_mode
        );
        let prepared = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(prepared.current, "b11-interrupt-g0");
        assert_eq!(prepared.previous, None);
        assert!(home
            .join(".local/lib/codex/core/release-public-key.pem")
            .is_file());
        m2_b1_assert_no_transaction_files(&paths);

        let state_before_retry = std::fs::read(&paths.activation_state).unwrap();
        let resumed = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            resumed.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            resumed.stdout,
            resumed.stderr
        );
        assert_eq!(
            std::fs::read(&paths.activation_state).unwrap(),
            state_before_retry
        );
        assert_eq!(
            openssl_sha256(&openssl, &entrypoint).unwrap(),
            openssl_sha256(&openssl, &core).unwrap()
        );
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        assert_ne!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);
        assert!(
            !std::fs::read_dir(prefix.join("bin")).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".codex.legacy-handoff-")
            })
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2c_release_core_legacy_handoff_public_path() {
        use std::os::unix::fs::PermissionsExt;

        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b11-slice2c-release-core");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let release = b10_build_signed_release(
            &root,
            &core,
            "b11-release-legacy-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );
        let next_release = b10_build_signed_release(
            &root,
            &core,
            "b11-release-legacy-g1",
            2,
            &openssl,
            &private_key,
            &public_key,
        );
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let network_log = b10_install_network_denial_sentinel(&prefix);
        let (entrypoint, expected_legacy_digest, legacy_mode) =
            b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-release-core\n");

        let output = b11_run_legacy_handoff(
            &core,
            &release,
            &public_key,
            &expected_legacy_digest,
            &home,
            &prefix,
            &tmp,
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert!(!network_log.exists());
        assert_eq!(
            openssl_sha256(&openssl, &entrypoint).unwrap(),
            openssl_sha256(&openssl, &core).unwrap()
        );
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        assert_eq!(legacy_mode, 0o751);

        let version = std::process::Command::new(&entrypoint)
            .arg("--version")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(
            version.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            version.stdout,
            version.stderr
        );
        let doctor = std::process::Command::new(&entrypoint)
            .arg("doctor")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(doctor.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&doctor.stdout).contains("[Summary]\nstatus: degraded"));
        let update = std::process::Command::new(&entrypoint)
            .args(["update", "--local"])
            .arg(&next_release)
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(
            update.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            update.stdout,
            update.stderr
        );
        assert!(!network_log.exists());
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let forward = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(forward.current, "b11-release-legacy-g1");
        assert_eq!(forward.previous.as_deref(), Some("b11-release-legacy-g0"));
        let updated_doctor = std::process::Command::new(&entrypoint)
            .arg("doctor")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(updated_doctor.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&updated_doctor.stdout).contains("[Summary]\nstatus: degraded")
        );
        let rollback = std::process::Command::new(&entrypoint)
            .args(["update", "--rollback"])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(rollback.status.code(), Some(0));
        assert!(!network_log.exists());

        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "b11-release-legacy-g0");
        assert_eq!(state.previous.as_deref(), Some("b11-release-legacy-g1"));
        let rolled_back_doctor = std::process::Command::new(&entrypoint)
            .arg("doctor")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(rolled_back_doctor.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&rolled_back_doctor.stdout)
            .contains("[Summary]\nstatus: degraded"));
        m2_b1_assert_no_transaction_files(&paths);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b11_slice2b_prepared_state_rejects_key_and_previous_conflicts() {
        {
            let root = temp_root("b11-slice2b-key-conflict");
            let openssl = b4_termux_openssl();
            let source = b4_source_roots(&root, &openssl);
            std::fs::create_dir(&source.generation_root).unwrap();
            let release = b2_write_generation(&source, "b11-key-conflict-g0", false, "supported");
            b4_write_probe_runtime(&release, 0, 0);
            let core = root.join("prebuilt-codex");
            b8_write_core_probe_wrapper(&core);
            b8_bind_core_digest(&release, &openssl, &core);
            let old_private_key = root.join("keys/old-private.pem");
            let old_public_key = root.join("keys/old-public.pem");
            let new_private_key = root.join("keys/new-private.pem");
            let new_public_key = root.join("keys/new-public.pem");
            b4_generate_release_keypair(&openssl, &old_private_key, &old_public_key);
            b4_generate_release_keypair(&openssl, &new_private_key, &new_public_key);
            b4_write_signed_release(&release, 1, &openssl, &old_private_key);
            let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
            b7_seed_initial_release(&release, &home, &prefix, &old_public_key);
            let (entrypoint, expected_legacy_digest, _) =
                b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-key-conflict\n");
            let legacy_bytes = std::fs::read(&entrypoint).unwrap();
            let roots = b7_public_roots(&home, &prefix);
            let paths = CoreStatePaths::new(&roots.state_root).unwrap();
            let state_before = std::fs::read(&paths.activation_state).unwrap();

            b4_write_signed_release(&release, 1, &openssl, &new_private_key);
            let output = b11_run_legacy_handoff(
                &core,
                &release,
                &new_public_key,
                &expected_legacy_digest,
                &home,
                &prefix,
                &tmp,
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "stdout={:?} stderr={:?}",
                output.stdout,
                output.stderr
            );
            assert_eq!(
                std::fs::read(&paths.activation_state).unwrap(),
                state_before
            );
            assert_eq!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);
            assert!(!home
                .join(".local/lib/codex/core/release-public-key.pem")
                .exists());
            remove_temp_root(root);
        }

        {
            let root = temp_root("b11-slice2b-previous-conflict");
            let openssl = b4_termux_openssl();
            let source = b4_source_roots(&root, &openssl);
            std::fs::create_dir(&source.generation_root).unwrap();
            let g0 = b2_write_generation(&source, "b11-previous-g0", false, "supported");
            b4_write_probe_runtime(&g0, 0, 0);
            let g1 = b2_write_generation(&source, "b11-previous-g1", false, "supported");
            b4_write_probe_runtime(&g1, 0, 0);
            let core = root.join("prebuilt-codex");
            b8_write_core_probe_wrapper(&core);
            b8_bind_core_digest(&g0, &openssl, &core);
            b8_bind_core_digest(&g1, &openssl, &core);
            let private_key = root.join("keys/private.pem");
            let public_key = root.join("keys/public.pem");
            b4_generate_release_keypair(&openssl, &private_key, &public_key);
            b4_write_signed_release(&g0, 1, &openssl, &private_key);
            b4_write_signed_release(&g1, 2, &openssl, &private_key);
            let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
            b7_seed_initial_release(&g0, &home, &prefix, &public_key);
            let roots = b7_public_roots(&home, &prefix);
            let paths = CoreStatePaths::new(&roots.state_root).unwrap();
            let before = read_pointer_state(&paths).unwrap().unwrap();
            let staged_id = stage_local_generation(&g1, &roots.generation_root).unwrap();
            assert_eq!(staged_id, "b11-previous-g1");
            let after =
                plan_activation_pointer_state_with_key(&before, &staged_id, before.current_key)
                    .unwrap();
            activate_pointer_state(&paths, Some(&before), &after).unwrap();
            let (entrypoint, expected_legacy_digest, _) =
                b11_write_legacy_entrypoint(&prefix, &openssl, b"legacy-previous-conflict\n");
            let legacy_bytes = std::fs::read(&entrypoint).unwrap();
            let state_before = std::fs::read(&paths.activation_state).unwrap();

            let output = b11_run_legacy_handoff(
                &core,
                &g1,
                &public_key,
                &expected_legacy_digest,
                &home,
                &prefix,
                &tmp,
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "stdout={:?} stderr={:?}",
                output.stdout,
                output.stderr
            );
            assert_eq!(
                std::fs::read(&paths.activation_state).unwrap(),
                state_before
            );
            assert_eq!(std::fs::read(&entrypoint).unwrap(), legacy_bytes);
            let state = read_pointer_state(&paths).unwrap().unwrap();
            assert_eq!(state.current, "b11-previous-g1");
            assert_eq!(state.previous.as_deref(), Some("b11-previous-g0"));
            assert!(!home
                .join(".local/lib/codex/core/release-public-key.pem")
                .exists());
            remove_temp_root(root);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b10_slice2_offline_local_update_and_rollback_use_signed_generations_only() {
        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b10-slice2-offline-update");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let g0 = b10_build_signed_release(
            &root,
            &core,
            "b10-offline-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );
        let g1 = b10_build_signed_release(
            &root,
            &core,
            "b10-offline-g1",
            2,
            &openssl,
            &private_key,
            &public_key,
        );
        let (g0_release, _) = verify_local_release_bundle(&g0, &openssl, &public_key).unwrap();
        let (g1_release, _) = verify_local_release_bundle(&g1, &openssl, &public_key).unwrap();
        assert_eq!(g0_release.release_sequence, 1);
        assert_eq!(g1_release.release_sequence, 2);

        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let network_log = b10_install_network_denial_sentinel(&prefix);
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let bootstrap_output = std::process::Command::new(&bootstrap)
            .args([core.as_os_str(), g0.as_os_str(), public_key.as_os_str()])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .output()
            .unwrap();
        assert_eq!(
            bootstrap_output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            bootstrap_output.stdout,
            bootstrap_output.stderr
        );
        assert!(!network_log.exists());

        let installed_core = prefix.join("bin/codex");
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let initial = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(initial.current, "b10-offline-g0");
        assert_eq!(initial.previous, None);

        let update = std::process::Command::new(&installed_core)
            .arg("update")
            .arg("--local")
            .arg(&g1)
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(
            update.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            update.stdout,
            update.stderr
        );
        assert!(!network_log.exists());
        let forward = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(forward.current, "b10-offline-g1");
        assert_eq!(forward.previous.as_deref(), Some("b10-offline-g0"));
        assert_eq!(forward.update_key, initial.update_key);
        assert_eq!(forward.current_key, initial.current_key);
        assert_eq!(forward.previous_key, Some(initial.current_key));
        let (installed_g1, _) = verify_installed_local_release(
            &roots,
            "b10-offline-g1",
            forward.current_key,
            "B10 offline local update generation id mismatch",
        )
        .unwrap();
        assert_eq!(installed_g1, g1_release);
        assert!(std::process::Command::new(&installed_core)
            .arg("--version")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .status()
            .unwrap()
            .success());
        assert!(!network_log.exists());

        let rollback = std::process::Command::new(&installed_core)
            .arg("update")
            .arg("--rollback")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        assert!(!network_log.exists());
        let rolled_back = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(rolled_back.current, "b10-offline-g0");
        assert_eq!(rolled_back.previous.as_deref(), Some("b10-offline-g1"));
        assert_eq!(rolled_back.update_key, forward.update_key);
        assert_eq!(rolled_back.current_key, initial.current_key);
        assert_eq!(rolled_back.previous_key, Some(forward.current_key));
        let (installed_g0, _) = verify_installed_local_release(
            &roots,
            "b10-offline-g0",
            rolled_back.current_key,
            "B10 offline rollback generation id mismatch",
        )
        .unwrap();
        assert_eq!(installed_g0, g0_release);
        assert!(std::process::Command::new(&installed_core)
            .arg("--version")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .status()
            .unwrap()
            .success());
        assert!(!network_log.exists());
        m2_b1_assert_no_transaction_files(&paths);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b10_slice3_offline_recovery_preserves_signed_rollback() {
        let Some(core) = b10_release_core_from_env() else {
            return;
        };
        let root = temp_root("b10-slice3-offline-recovery");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let g0 = b10_build_signed_release(
            &root,
            &core,
            "b10-offline-g0",
            1,
            &openssl,
            &private_key,
            &public_key,
        );
        let g1 = b10_build_signed_release(
            &root,
            &core,
            "b10-offline-g1",
            2,
            &openssl,
            &private_key,
            &public_key,
        );
        let (g0_release, _) = verify_local_release_bundle(&g0, &openssl, &public_key).unwrap();
        let (g1_release, _) = verify_local_release_bundle(&g1, &openssl, &public_key).unwrap();

        for fail_call in [3usize, 6usize] {
            let target_root = root.join(format!("target-{fail_call}"));
            let (home, prefix, tmp) = b4_prepare_public_environment(&target_root, &openssl, true);
            let network_log = b10_install_network_denial_sentinel(&prefix);
            let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../bootstrap/codex-bootstrap");
            let bootstrap_output = std::process::Command::new(&bootstrap)
                .args([core.as_os_str(), g0.as_os_str(), public_key.as_os_str()])
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
                .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
                .output()
                .unwrap();
            assert_eq!(
                bootstrap_output.status.code(),
                Some(0),
                "fail_call={fail_call} stdout={:?} stderr={:?}",
                bootstrap_output.stdout,
                bootstrap_output.stderr
            );

            let installed_core = prefix.join("bin/codex");
            let update = std::process::Command::new(&installed_core)
                .arg("update")
                .arg("--local")
                .arg(&g1)
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .output()
                .unwrap();
            assert_eq!(
                update.status.code(),
                Some(0),
                "fail_call={fail_call} stdout={:?} stderr={:?}",
                update.stdout,
                update.stderr
            );
            assert!(!network_log.exists());

            let roots = b7_public_roots(&home, &prefix);
            let paths = CoreStatePaths::new(&roots.state_root).unwrap();
            let forward = read_pointer_state(&paths).unwrap().unwrap();
            assert_eq!(forward.current, "b10-offline-g1");
            assert_eq!(forward.previous.as_deref(), Some("b10-offline-g0"));
            let planned_rollback = plan_rollback_pointer_state(&forward).unwrap();
            let mut io = M2B1FaultIo::new(fail_call, M2B1FaultTiming::After);
            let failure =
                activate_pointer_state_with_io(&paths, Some(&forward), &planned_rollback, &mut io)
                    .expect_err("injected recoverable rollback fault must abort activation call");
            assert!(matches!(failure, ActivationTransactionError::Io { .. }));
            assert_eq!(io.calls, fail_call);
            assert!(paths.activation_journal.is_file());
            assert!(!network_log.exists());

            let expected_recovered = if fail_call == 3 {
                forward.clone()
            } else {
                planned_rollback.clone()
            };
            assert_eq!(
                recover_activation_state(&paths).unwrap(),
                Some(expected_recovered.clone())
            );
            assert_eq!(
                read_pointer_state(&paths).unwrap(),
                Some(expected_recovered.clone())
            );
            m2_b1_assert_no_transaction_files(&paths);

            let expected_release = if expected_recovered.current == "b10-offline-g1" {
                &g1_release
            } else {
                &g0_release
            };
            let (recovered_release, _) = verify_installed_local_release(
                &roots,
                &expected_recovered.current,
                expected_recovered.current_key,
                "B10 offline recovered generation id mismatch",
            )
            .unwrap();
            assert_eq!(&recovered_release, expected_release);
            assert!(std::process::Command::new(&installed_core)
                .arg("--version")
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .status()
                .unwrap()
                .success());
            assert!(!network_log.exists());

            let expected_after_public_rollback =
                plan_rollback_pointer_state(&expected_recovered).unwrap();
            let rollback = std::process::Command::new(&installed_core)
                .arg("update")
                .arg("--rollback")
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .output()
                .unwrap();
            assert_eq!(
                rollback.status.code(),
                Some(0),
                "fail_call={fail_call} stdout={:?} stderr={:?}",
                rollback.stdout,
                rollback.stderr
            );
            let after_public_rollback = read_pointer_state(&paths).unwrap().unwrap();
            assert_eq!(after_public_rollback, expected_after_public_rollback);
            assert_eq!(after_public_rollback.update_key, forward.update_key);
            let expected_rollback_release = if after_public_rollback.current == "b10-offline-g1" {
                &g1_release
            } else {
                &g0_release
            };
            let (rolled_release, _) = verify_installed_local_release(
                &roots,
                &after_public_rollback.current,
                after_public_rollback.current_key,
                "B10 offline post-recovery rollback generation id mismatch",
            )
            .unwrap();
            assert_eq!(&rolled_release, expected_rollback_release);
            assert!(!network_log.exists());
            m2_b1_assert_no_transaction_files(&paths);
            remove_temp_root(target_root);
        }
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r6_builder_publish_output_enters_existing_signed_release_admission() {
        use std::ffi::OsString;

        let root = temp_root("b6-builder-admission");
        let openssl = b4_termux_openssl();
        let prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let gzip = prefix.join("bin/gzip");
        assert!(gzip.is_file(), "Termux gzip is required for B6 proof");
        let archive = root.join("codex-package-aarch64-unknown-linux-musl.tar.gz");
        let raw_runtime = b6_write_official_shape_archive(&gzip, &archive);
        let archive_sha256 = openssl_sha256(&openssl, &archive).unwrap();
        let core = b10_release_core_from_env()
            .unwrap_or_else(|| std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap());
        let core_sha256 = openssl_sha256(&openssl, &core).unwrap();
        let generation = root.join("unsigned-generation");
        let args = vec![
            OsString::from("build"),
            OsString::from("--version"),
            OsString::from("0.150.1"),
            OsString::from("--archive"),
            archive.as_os_str().to_owned(),
            OsString::from("--archive-sha256"),
            OsString::from(&archive_sha256),
            OsString::from("--generation-id"),
            OsString::from("b6-signed-admission"),
            OsString::from("--core"),
            core.as_os_str().to_owned(),
            OsString::from("--creation-metadata"),
            OsString::from("m2-b6-integration"),
            OsString::from("--gzip"),
            gzip.as_os_str().to_owned(),
            OsString::from("--openssl"),
            openssl.as_os_str().to_owned(),
            OsString::from("--output"),
            generation.as_os_str().to_owned(),
        ];
        assert_eq!(codex_release_builder::run_from_args(args), 0);

        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let publication = root.join("publication");
        let publish_args = vec![
            OsString::from("publish"),
            OsString::from("--generation"),
            generation.as_os_str().to_owned(),
            OsString::from("--release-sequence"),
            OsString::from("1"),
            OsString::from("--release-base"),
            OsString::from("https://releases.example.invalid/codex/releases/b6-signed-admission/"),
            OsString::from("--private-key"),
            private_key.as_os_str().to_owned(),
            OsString::from("--openssl"),
            openssl.as_os_str().to_owned(),
            OsString::from("--output"),
            publication.as_os_str().to_owned(),
        ];
        assert_eq!(codex_release_builder::run_from_args(publish_args), 0);
        let published_generation = publication.join("releases/b6-signed-admission");
        let (release, loaded) =
            verify_local_release_bundle(&published_generation, &openssl, &public_key).unwrap();

        assert_eq!(release.generation_id, "b6-signed-admission");
        assert_eq!(
            release
                .files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "browser/manual/curl",
                "browser/open/curl",
                "codex-code-mode-host",
                "core",
                "generation.meta",
                "runtime",
            ]
        );
        assert_eq!(loaded.generation_id, "b6-signed-admission");
        assert_eq!(
            loaded.manifest.upstream_package_identity,
            "openai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz"
        );
        assert_eq!(loaded.manifest.upstream_package_version, "0.150.1");
        assert_eq!(loaded.manifest.source_artifact_digest, archive_sha256);
        assert_eq!(loaded.manifest.expected_platform, "android");
        assert_eq!(loaded.manifest.expected_architecture, "aarch64");
        assert_eq!(loaded.manifest.patch_policy_id, "termux-fd-remap-v1");
        assert!(loaded
            .manifest
            .patch_report
            .ends_with("source_counts=2,1,1,1;changed_bytes=54"));
        assert_eq!(
            loaded.manifest.runtime_digest,
            openssl_sha256(&openssl, &published_generation.join("runtime")).unwrap()
        );
        assert_eq!(loaded.manifest.core_artifact_digest, core_sha256);
        assert_eq!(
            loaded
                .manifest
                .helper_digests
                .iter()
                .map(|helper| helper.identity.as_str())
                .collect::<Vec<_>>(),
            vec![
                TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
            ]
        );
        assert_eq!(loaded.manifest.manager_artifact_digest, None);
        assert_eq!(
            loaded.doctor_capability,
            UpstreamDoctorCapability::Supported
        );
        assert_eq!(
            std::fs::read(published_generation.join(CODE_MODE_HOST_FILE)).unwrap(),
            b6_static_aarch64_elf(false)
        );
        let runtime = std::fs::read(published_generation.join("runtime")).unwrap();
        assert_eq!(runtime.len(), raw_runtime.len());
        assert_ne!(runtime, raw_runtime);
        assert!(!generation.join("release.manifest").exists());
        assert!(!root.join("state").exists());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r7_official_release_metadata_binds_stable_version_and_target_digest() {
        let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let metadata = format!(
            "{{\"assets\":[{{\"name\":\"other\",\"digest\":\"sha256:{digest}\"}},{{\"name\":\"{UPSTREAM_PACKAGE_ASSET}\",\"digest\":\"sha256:{digest}\"}}],\"tag_name\":\"rust-v1.2.3\"}}"
        );
        assert_eq!(
            parse_official_release_metadata(metadata.as_bytes(), None).unwrap(),
            OfficialReleaseMetadata {
                version: "1.2.3".to_owned(),
                package_sha256: digest.to_owned(),
            }
        );
        assert!(matches!(
            parse_official_release_metadata(metadata.as_bytes(), Some("1.2.4")),
            Err(LocalProductError::LocalUpdate(
                "upstream release metadata version does not match the requested version"
            ))
        ));
        assert!(matches!(
            parse_official_release_metadata(
                br#"{"assets":[],"tag_name":"rust-v1.2.3",,"bad":true}"#,
                None
            ),
            Err(LocalProductError::LocalUpdate(
                "upstream release metadata is malformed"
            ))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_r7_signed_channel_hit_skips_local_build_and_publication() {
        let fixture = b5_channel_fixture("r7-channel-hit", "r7-channel-next");
        let output = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert!(output
            .stdout
            .windows(b"Codex 9.9.9 is now active. \xE2\x9C\x85\n".len())
            .any(|window| window == b"Codex 9.9.9 is now active. \xE2\x9C\x85\n"));
        assert!(!fixture.home.join(LOCAL_PUBLICATION_ROOT_RELATIVE).exists());
        let calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
        assert!(calls.contains(&fixture.index_url));
        assert!(calls.contains(&fixture.base));
        assert!(!calls.contains(DEFAULT_UPSTREAM_LATEST_URL));
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_rald2_core_update_never_enters_official_producer_with_maintainer_credentials() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture = b5_channel_fixture("rald2-consumer-only", "rald2-channel-next");
        b5_write_channel_curl(
            &fixture.prefix.join("bin/curl"),
            &fixture.curl_log,
            DEFAULT_UPDATE_INDEX_URL,
            &fixture.index_path,
            &fixture.signature_path,
            &fixture.base,
            &fixture
                .root
                .join("release-server/source-generations/rald2-channel-next"),
        );
        std::fs::write(&fixture.curl_log, b"").unwrap();

        let maintainer_key = fixture
            .home
            .join(".config/codex/termux/update-private-key.pem");
        std::fs::create_dir_all(maintainer_key.parent().unwrap()).unwrap();
        std::fs::copy(&fixture.private_key, &maintainer_key).unwrap();
        let mut key_permissions = std::fs::metadata(&maintainer_key).unwrap().permissions();
        key_permissions.set_mode(0o600);
        std::fs::set_permissions(&maintainer_key, key_permissions).unwrap();

        let gh_log = fixture.root.join("rald2-gh-log");
        b7_write_fake_github_cli(
            &fixture.prefix.join("bin/gh"),
            &gh_log,
            &fixture.root.join("rald2-gh-body"),
            0,
        );

        let ordinary = rald2_run_public_default_channel_update(
            false,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            ordinary.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            ordinary.stdout,
            ordinary.stderr
        );
        assert!(ordinary
            .stdout
            .windows(
                b"Codex 9.9.9 is now active. \xE2\x9C\x85
"
                .len()
            )
            .any(|window| window
                == b"Codex 9.9.9 is now active. \xE2\x9C\x85
"));
        assert!(!gh_log.exists(), "ordinary update must not invoke gh");
        let ordinary_calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
        assert!(!ordinary_calls.contains(DEFAULT_UPSTREAM_LATEST_URL));

        let rollback = arh1_run_public_rollback(&fixture.home, &fixture.prefix, &fixture.tmp);
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        assert!(!gh_log.exists(), "rollback must not invoke gh");

        std::fs::write(&fixture.curl_log, b"").unwrap();
        let held = rald2_run_public_default_channel_update(
            false,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(held.status.code(), Some(1), "stderr={:?}", held.stderr);
        assert_human_update_error_contains(
            &held.stderr,
            b"release sequence 2 is locally held after rollback",
        );
        assert!(!gh_log.exists(), "held update must not invoke gh");
        let held_calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
        assert!(!held_calls.contains(DEFAULT_UPSTREAM_LATEST_URL));

        std::fs::write(&fixture.curl_log, b"").unwrap();
        let forced = rald2_run_public_default_channel_update(
            true,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            forced.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            forced.stdout,
            forced.stderr
        );
        assert!(!gh_log.exists(), "forced held update must not invoke gh");
        let forced_calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
        assert!(!forced_calls.contains(DEFAULT_UPSTREAM_LATEST_URL));

        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "rald2-channel-next");
        assert_eq!(state.previous.as_deref(), Some("channel-current"));
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(UpdateHoldRecord {
                generation_id: "rald2-channel-next".to_owned(),
                release_sequence: 2,
            })
        );
        assert!(!fixture.home.join(LOCAL_PUBLICATION_ROOT_RELATIVE).exists());
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_rald1_local_derived_fallback_and_explicit_build_preserve_public_authority() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("rald1-local-derived");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let prefix_openssl = prefix.join("bin/openssl");
        std::fs::remove_file(&prefix_openssl).unwrap();
        std::fs::copy(&openssl, &prefix_openssl).unwrap();
        let live_gzip =
            std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/gzip");
        let fixture_gzip = prefix.join("bin/gzip");
        std::fs::copy(&live_gzip, &fixture_gzip).unwrap();

        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let current = b2_write_generation(&source_roots, "r7-current", false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b7_seed_initial_release(
            &current,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        let stable_entrypoint = prefix.join("bin/codex");
        std::fs::copy(std::env::current_exe().unwrap(), &stable_entrypoint).unwrap();
        let mut stable_mode = std::fs::metadata(&stable_entrypoint).unwrap().permissions();
        stable_mode.set_mode(0o755);
        std::fs::set_permissions(&stable_entrypoint, stable_mode).unwrap();
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();

        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let public_before = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(public_before.current, "r7-current");

        let runtime_source = b8_compile_static_probe_runtime(&root);
        let archive = root.join(UPSTREAM_PACKAGE_ASSET);
        let raw_runtime = std::fs::read(&runtime_source).unwrap();
        b6_write_official_shape_archive_with_runtime(&fixture_gzip, &archive, &raw_runtime);
        let archive_digest = openssl_sha256(&openssl, &archive).unwrap();
        let metadata_path = root.join("upstream-release.json");
        std::fs::write(
            &metadata_path,
            format!(
                "{{\"tag_name\":\"rust-v0.150.1\",\"assets\":[{{\"name\":\"{UPSTREAM_PACKAGE_ASSET}\",\"digest\":\"sha256:{archive_digest}\"}}]}}"
            ),
        )
        .unwrap();
        let index_url = "https://updates.example.invalid/codex/update-index-v1";
        let metadata_url = DEFAULT_UPSTREAM_LATEST_URL.to_owned();
        let archive_url =
            format!("{UPSTREAM_RELEASE_METADATA_BASE}/0.150.1/{UPSTREAM_PACKAGE_ASSET}");
        let curl_log = root.join("r7-curl-log");
        b7_write_local_update_curl(
            &prefix.join("bin/curl"),
            &curl_log,
            index_url,
            &metadata_url,
            &metadata_path,
            &archive_url,
            &archive,
        );
        let gh_log = root.join("r7-gh-log");
        b7_write_fake_github_cli(
            &prefix.join("bin/gh"),
            &gh_log,
            &root.join("r7-gh-body"),
            77,
        );

        let output =
            b7_run_public_local_fallback(index_url, None, &private_key, &home, &prefix, &tmp);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("Updating Codex 9.9.9 -> 0.150.1..."));
        assert!(stdout.contains("Verified and activated the locally built Termux release."));
        assert!(stdout.contains("Codex 0.150.1 is now active. ✅"));
        assert!(output.stderr.is_empty(), "stderr={:?}", output.stderr);
        assert!(
            !gh_log.exists(),
            "local-derived fallback must not invoke gh"
        );
        assert!(!home.join(LOCAL_PUBLICATION_ROOT_RELATIVE).exists());
        let calls = std::fs::read_to_string(&curl_log).unwrap();
        assert!(calls.contains(index_url));
        assert!(calls.contains(&metadata_url));
        assert!(calls.contains(&archive_url));

        let local_state = read_pointer_state(&paths).unwrap().unwrap();
        let first_local_generation = local_state.current.clone();
        assert!(first_local_generation.starts_with("local-"));
        assert_eq!(local_state.previous.as_deref(), Some("r7-current"));
        assert_eq!(local_state.update_key, public_before.update_key);
        assert_ne!(local_state.current_key, public_before.update_key);
        assert_eq!(local_state.previous_key, Some(public_before.current_key));
        let (local_release, local_loaded) = verify_installed_local_release(
            &roots,
            &first_local_generation,
            local_state.current_key,
            "RALD-1 local-derived generation id mismatch",
        )
        .unwrap();
        assert_eq!(local_release.release_sequence, 1);
        assert_eq!(local_loaded.manifest.upstream_package_version, "0.150.1");
        assert_eq!(local_loaded.manifest.source_artifact_digest, archive_digest);
        assert_eq!(
            local_loaded.generation_layout,
            GenerationLayout::RootCodeModeHost
        );
        assert_eq!(
            parse_local_derived_metadata(&local_release, &local_loaded).unwrap(),
            Some(PublicReleaseBaseline {
                generation_component: "r7-current".to_owned(),
                release_sequence: 1,
            })
        );
        assert!(!roots
            .generation_root
            .join(&first_local_generation)
            .join(LOCAL_DERIVED_PRIVATE_KEY_FILE)
            .exists());
        assert!(std::fs::read_dir(&roots.state_root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".local-update-")
        }));

        let rollback = b4_run_public_rollback(&home, &prefix, &tmp);
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        let rolled_back = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(rolled_back.current, "r7-current");
        assert_eq!(
            rolled_back.previous.as_deref(),
            Some(first_local_generation.as_str())
        );
        assert_eq!(rolled_back.update_key, public_before.update_key);
        assert_eq!(read_update_hold(&roots).unwrap(), None);
        assert_eq!(rollback_guard::read_guard(&roots).unwrap(), None);

        let explicit = b7_run_public_build_local(None, Some(&private_key), &home, &prefix, &tmp);
        assert_eq!(
            explicit.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            explicit.stdout,
            explicit.stderr
        );
        let explicit_stdout = String::from_utf8(explicit.stdout).unwrap();
        assert!(explicit_stdout.contains("Updating Codex 9.9.9 -> 0.150.1..."));
        assert!(
            explicit_stdout.contains("Verified and activated the locally built Termux release.")
        );
        assert!(explicit_stdout.contains("Codex 0.150.1 is now active. ✅"));
        assert!(explicit.stderr.is_empty(), "stderr={:?}", explicit.stderr);
        assert!(!gh_log.exists(), "--build-local must not invoke gh");
        let explicit_state = read_pointer_state(&paths).unwrap().unwrap();
        assert!(explicit_state.current.starts_with("local-"));
        assert_eq!(explicit_state.previous.as_deref(), Some("r7-current"));
        assert_eq!(explicit_state.update_key, public_before.update_key);
        assert_ne!(explicit_state.current_key, public_before.update_key);
        let explicit_local_generation = explicit_state.current.clone();
        let (explicit_release, explicit_loaded) = verify_installed_local_release(
            &roots,
            &explicit_local_generation,
            explicit_state.current_key,
            "RALD-1 explicit local-derived generation id mismatch",
        )
        .unwrap();
        assert_eq!(explicit_release.release_sequence, 1);
        assert!(
            parse_local_derived_metadata(&explicit_release, &explicit_loaded)
                .unwrap()
                .is_some()
        );

        let public_next =
            b2_write_generation(&source_roots, "r7-public-next", false, "unsupported");
        b4_write_signed_release(&public_next, 2, &openssl, &private_key);
        let official = b4_run_public_update(&public_next, &home, &prefix, &tmp);
        assert_eq!(
            official.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            official.stdout,
            official.stderr
        );
        let official_state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(official_state.current, "r7-public-next");
        assert_eq!(
            official_state.previous.as_deref(),
            Some(explicit_local_generation.as_str())
        );
        assert_eq!(official_state.update_key, public_before.update_key);
        assert_eq!(official_state.current_key, public_before.update_key);

        let official_rollback = b4_run_public_rollback(&home, &prefix, &tmp);
        assert_eq!(
            official_rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            official_rollback.stdout,
            official_rollback.stderr
        );
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(UpdateHoldRecord {
                generation_id: "r7-public-next".to_owned(),
                release_sequence: 2,
            })
        );

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_rald1_build_local_cli_rejects_combined_selectors() {
        assert_eq!(
            run_core_update(vec![
                OsString::from("--build-local"),
                OsString::from("--force"),
            ]),
            2
        );
        assert_eq!(
            run_core_update(vec![
                OsString::from("--build-local"),
                OsString::from("--local"),
                OsString::from("ignored"),
            ]),
            2
        );
        assert!(UPDATE_USAGE.contains("--build-local"));
    }

    #[cfg(unix)]
    #[test]
    fn test_rald1_self_signed_local_directory_cannot_claim_local_derived_authority() {
        let fixture = b5_channel_fixture("rald1-self-signed-local", "rald1-unused-channel");
        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let before = read_pointer_state(&paths).unwrap().unwrap();

        let alien_private = fixture.root.join("alien/private.pem");
        let alien_public = fixture.root.join("alien/public.pem");
        b4_generate_release_keypair(&fixture.openssl, &alien_private, &alien_public);
        let source_roots = b4_source_roots(&fixture.root.join("alien-source"), &fixture.openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let forged = b2_write_root_generation(
            &source_roots,
            "rald1-forged-local-derived",
            false,
            "supported",
        );
        let descriptor_path = forged.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap();
        let fake_digest = "0".repeat(64);
        let descriptor = descriptor
            .replace(
                "source_artifact_digest\tsource-digest\n",
                &format!("source_artifact_digest\t{fake_digest}\n"),
            )
            .replace(
                "creation_metadata\ttest-fixture\n",
                &format!(
                    "creation_metadata\t{LOCAL_DERIVED_METADATA_FORMAT};upstream_version=9.9.9;archive_sha256={fake_digest};public_generation={};public_sequence=1\n",
                    fixture.current_id
                ),
            );
        std::fs::write(&descriptor_path, descriptor).unwrap();
        b4_write_signed_release(&forged, 1, &fixture.openssl, &alien_private);

        let output = b4_run_public_update(&forged, &fixture.home, &fixture.prefix, &fixture.tmp);
        assert_eq!(
            output.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert_eq!(read_pointer_state(&paths).unwrap().unwrap(), before);
        assert!(!roots
            .generation_root
            .join("rald1-forged-local-derived")
            .exists());

        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_rald1_ephemeral_key_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("rald1-ephemeral-key-mode");
        let openssl = b4_termux_openssl();
        let (private_key, public_key) =
            generate_ephemeral_local_derived_key(&openssl, &root).unwrap();
        let metadata = std::fs::symlink_metadata(&private_key).unwrap();
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
        assert_ne!(public_key, ReleasePublicKey([0; 32]));
        std::fs::remove_file(private_key).unwrap();
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b8_slice3_release_production_to_bootstrap_flow() {
        use std::ffi::OsString;

        let Some(core_input) = std::env::var_os("CODEX_B8_RELEASE_CORE") else {
            return;
        };
        let core = std::fs::canonicalize(core_input).unwrap();
        assert!(core.is_absolute());
        assert!(core.is_file());

        let root = temp_root("b8-slice3-release-production");
        let openssl = b4_termux_openssl();
        let live_prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let gzip = live_prefix.join("bin/gzip");
        assert!(
            gzip.is_file(),
            "Termux gzip is required for B8 integration proof"
        );
        let runtime_source = b8_compile_static_probe_runtime(&root);
        let raw_runtime = std::fs::read(&runtime_source).unwrap();
        let archive = root.join("codex-package-aarch64-unknown-linux-musl.tar.gz");
        b6_write_official_shape_archive_with_runtime(&gzip, &archive, &raw_runtime);
        let archive_sha256 = openssl_sha256(&openssl, &archive).unwrap();
        let core_sha256 = openssl_sha256(&openssl, &core).unwrap();
        let generation = root.join("unsigned-generation");
        let args = vec![
            OsString::from("build"),
            OsString::from("--version"),
            OsString::from("0.150.1"),
            OsString::from("--archive"),
            archive.as_os_str().to_owned(),
            OsString::from("--archive-sha256"),
            OsString::from(&archive_sha256),
            OsString::from("--generation-id"),
            OsString::from("b8-release-production"),
            OsString::from("--core"),
            core.as_os_str().to_owned(),
            OsString::from("--creation-metadata"),
            OsString::from("m2-b8-release-production"),
            OsString::from("--gzip"),
            gzip.as_os_str().to_owned(),
            OsString::from("--openssl"),
            openssl.as_os_str().to_owned(),
            OsString::from("--output"),
            generation.as_os_str().to_owned(),
        ];
        assert_eq!(codex_release_builder::run_from_args(args), 0);
        let descriptor = std::fs::read_to_string(generation.join("generation.meta")).unwrap();
        assert!(descriptor.contains(&format!("core_artifact_digest\t{core_sha256}\n")));
        assert_ne!(
            std::fs::read(generation.join("runtime")).unwrap(),
            raw_runtime
        );
        assert!(std::process::Command::new(generation.join("runtime"))
            .args(["-c", "sandbox_mode=\"danger-full-access\"", "--version"])
            .status()
            .unwrap()
            .success());
        assert!(std::process::Command::new(generation.join("runtime"))
            .args(["-c", "sandbox_mode=\"danger-full-access\"", "doctor"])
            .status()
            .unwrap()
            .success());

        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_write_signed_release(&generation, 1, &openssl, &private_key);
        let (signed_release, loaded) =
            verify_local_release_bundle(&generation, &openssl, &public_key).unwrap();
        assert_eq!(signed_release.generation_id, "b8-release-production");
        assert_eq!(loaded.manifest.core_artifact_digest, core_sha256);
        assert_eq!(loaded.manifest.source_artifact_digest, archive_sha256);

        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                core.as_os_str(),
                generation.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let installed_core = prefix.join("bin/codex");
        assert_eq!(
            openssl_sha256(&openssl, &installed_core).unwrap(),
            core_sha256
        );
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "b8-release-production");
        assert_eq!(state.previous, None);
        assert_eq!(state.previous_key, None);
        let (installed_release, installed_loaded) = verify_installed_local_release(
            &roots,
            "b8-release-production",
            state.current_key,
            "B8 integration installed generation id mismatch",
        )
        .unwrap();
        assert_eq!(installed_release, signed_release);
        assert_eq!(installed_loaded.manifest.core_artifact_digest, core_sha256);
        assert_eq!(
            openssl_sha256(
                &openssl,
                &roots
                    .generation_root
                    .join("b8-release-production/generation.meta"),
            )
            .unwrap(),
            signed_release
                .files
                .iter()
                .find(|file| file.relative_path == "generation.meta")
                .unwrap()
                .sha256
        );
        assert!(std::process::Command::new(&installed_core)
            .arg("--version")
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .status()
            .unwrap()
            .success());
        assert!(std::fs::read_dir(&tmp).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".codex-bootstrap.")
        }));
        assert!(!root.join("state").exists());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn b4_source_roots(root: &std::path::Path, openssl: &std::path::Path) -> LocalCoreRoots {
        LocalCoreRoots {
            generation_root: root.join("source-generations"),
            state_root: root.join("source-state"),
            config_dir: root.join("source-state/config"),
            resolver_path: root.join("source-resolv.conf"),
            cert_file: root.join("source-cert.pem"),
            cert_dir: root.join("source-certs"),
            openssl: openssl.to_owned(),
            curl: root.join("unused-source-curl"),
        }
    }

    #[cfg(unix)]
    fn b4_prepare_public_environment(
        root: &std::path::Path,
        live_openssl: &std::path::Path,
        install_openssl: bool,
    ) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        use std::os::unix::fs::symlink;

        let home = root.join("home");
        let prefix = root.join("prefix");
        let tmp = root.join("tmp");
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        std::fs::create_dir_all(prefix.join("etc/tls/certs")).unwrap();
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(prefix.join("etc/resolv.conf"), b"nameserver 127.0.0.1\n").unwrap();
        std::fs::write(prefix.join("etc/tls/cert.pem"), b"test-cert").unwrap();
        if install_openssl {
            symlink(live_openssl, prefix.join("bin/openssl")).unwrap();
        }
        (home, prefix, tmp)
    }

    #[cfg(unix)]
    fn b4_run_public_update(
        source_generation: &std::path::Path,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_SOURCE, source_generation)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env("CODEX_TEST_BOOTSTRAP_FIRST", "1")
            .env_remove("CODEX_TEST_REQUIRE_NO_ACQUISITION")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn b4_run_public_rollback(
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env_remove("CODEX_TEST_REQUIRE_NO_ACQUISITION")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn arh1_run_public_rollback_with_capability(
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
        hold_capability: &str,
    ) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env(rollback_guard::TEST_HOLD_CAPABILITY_ENV, hold_capability)
            .env_remove("CODEX_TEST_REQUIRE_NO_ACQUISITION")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn arh1_run_public_rollback(
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        arh1_run_public_rollback_with_capability(home, prefix, tmp, "1")
    }

    #[cfg(unix)]
    fn b5_run_public_remote_update(
        base: &str,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env_remove(UPDATE_PROBE_SOURCE)
            .env(UPDATE_PROBE_REMOTE, base)
            .env("CODEX_TEST_REQUIRE_NO_ACQUISITION", "1")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn b5_run_public_channel_update(
        index_url: &str,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_CHANNEL, "1")
            .env_remove(UPDATE_PROBE_FORCE)
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env("CODEX_TERMUX_UPDATE_INDEX_URL", index_url)
            .env("CODEX_TEST_REQUIRE_NO_ACQUISITION", "1")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR");
        command.output().unwrap()
    }

    #[cfg(unix)]
    fn b5_run_public_channel_update_tty(
        index_url: &str,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        use std::os::unix::ffi::OsStrExt as _;

        let script =
            std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/script");
        let executable = doctor_shell_quote(std::env::current_exe().unwrap().as_os_str());
        let executable = std::str::from_utf8(executable.as_bytes()).unwrap();
        let command = format!("{executable} tests::public_update_probe --exact --nocapture");
        std::process::Command::new(script)
            .args(["-qefc"])
            .arg(command)
            .arg("/dev/null")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_CHANNEL, "1")
            .env_remove(UPDATE_PROBE_FORCE)
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env("CODEX_TERMUX_UPDATE_INDEX_URL", index_url)
            .env("CODEX_TEST_REQUIRE_NO_ACQUISITION", "1")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn rald2_run_public_default_channel_update(
        force: bool,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_CHANNEL, "1")
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env_remove(UPDATE_INDEX_URL_ENV)
            .env_remove(UPDATE_VERSION_ENV)
            .env_remove(UPDATE_PRIVATE_KEY_ENV)
            .env("CODEX_TEST_REQUIRE_NO_ACQUISITION", "1")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR");
        if force {
            command.env(UPDATE_PROBE_FORCE, "1");
        } else {
            command.env_remove(UPDATE_PROBE_FORCE);
        }
        command.output().unwrap()
    }

    #[cfg(unix)]
    fn arh1_run_public_channel_force_update(
        index_url: &str,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_CHANNEL, "1")
            .env(UPDATE_PROBE_FORCE, "1")
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env("CODEX_TERMUX_UPDATE_INDEX_URL", index_url)
            .env("CODEX_TEST_REQUIRE_NO_ACQUISITION", "1")
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn b7_run_public_local_fallback(
        index_url: &str,
        version: Option<&str>,
        private_key: &std::path::Path,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_CHANNEL, "1")
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env("CODEX_TERMUX_UPDATE_INDEX_URL", index_url)
            .env(UPDATE_PRIVATE_KEY_ENV, private_key)
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR");
        if let Some(version) = version {
            command.env(UPDATE_VERSION_ENV, version);
        } else {
            command.env_remove(UPDATE_VERSION_ENV);
        }
        command.output().unwrap()
    }

    #[cfg(unix)]
    fn b7_run_public_build_local(
        version: Option<&str>,
        private_key: Option<&std::path::Path>,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
    ) -> std::process::Output {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("tests::public_update_probe")
            .arg("--exact")
            .arg("--nocapture")
            .env(UPDATE_PROBE_ROLE, "1")
            .env(UPDATE_PROBE_BUILD_LOCAL, "1")
            .env_remove(UPDATE_PROBE_CHANNEL)
            .env_remove(UPDATE_PROBE_FORCE)
            .env_remove(UPDATE_PROBE_SOURCE)
            .env_remove(UPDATE_PROBE_REMOTE)
            .env("HOME", home)
            .env("PREFIX", prefix)
            .env("TMPDIR", tmp)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR");
        if let Some(private_key) = private_key {
            command.env(UPDATE_PRIVATE_KEY_ENV, private_key);
        } else {
            command.env_remove(UPDATE_PRIVATE_KEY_ENV);
        }
        if let Some(version) = version {
            command.env(UPDATE_VERSION_ENV, version);
        } else {
            command.env_remove(UPDATE_VERSION_ENV);
        }
        command.output().unwrap()
    }

    #[cfg(unix)]
    fn b4_install_trusted_release_key(home: &std::path::Path, public_key: &std::path::Path) {
        let pinned = home.join(".local/lib/codex/core/release-public-key.pem");
        std::fs::create_dir_all(pinned.parent().unwrap()).unwrap();
        std::fs::copy(public_key, pinned).unwrap();
    }

    #[cfg(unix)]
    fn b7_public_roots(home: &std::path::Path, prefix: &std::path::Path) -> LocalCoreRoots {
        let state_root = home.join(".local/share/codex/core");
        LocalCoreRoots {
            generation_root: home.join(".local/lib/codex/core/generations"),
            config_dir: state_root.join("config"),
            state_root,
            resolver_path: prefix.join("etc/resolv.conf"),
            cert_file: prefix.join("etc/tls/cert.pem"),
            cert_dir: prefix.join("etc/tls/certs"),
            openssl: prefix.join("bin/openssl"),
            curl: prefix.join("bin/curl"),
        }
    }

    #[cfg(unix)]
    fn b7_bootstrap_initial_release(
        source_generation: &std::path::Path,
        home: &std::path::Path,
        prefix: &std::path::Path,
        bootstrap_public_key: &std::path::Path,
    ) -> Result<GenerationPointerState, LocalProductError> {
        let roots = b7_public_roots(home, prefix);
        let bootstrap_key = release_public_key_from_pem(&roots.openssl, bootstrap_public_key)?;
        let (manifest, loaded) =
            verify_local_release_bundle_with_key(source_generation, &roots.openssl, bootstrap_key)?;
        if manifest.release_public_key != bootstrap_key {
            return Err(LocalProductError::Release(
                "bootstrap release key does not match pinned key",
            ));
        }
        ensure_real_directory_tree(
            &roots.generation_root,
            "inspect immutable generation root",
            "immutable generation root is not a real directory",
        )?;
        let generation_id = stage_local_generation(source_generation, &roots.generation_root)?;
        if generation_id != loaded.generation_id {
            return Err(LocalProductError::Descriptor(
                "bootstrap generation descriptor id does not match source",
            ));
        }
        let (_, installed) = verify_installed_local_release(
            &roots,
            &generation_id,
            bootstrap_key,
            "bootstrap generation descriptor id does not match publication path",
        )?;
        if installed.generation_id != generation_id {
            return Err(LocalProductError::Descriptor(
                "bootstrap generation descriptor id does not match installed generation",
            ));
        }
        std::fs::create_dir_all(roots.state_root.parent().unwrap()).map_err(|source| {
            LocalProductError::Io {
                operation: "create bootstrap state parent",
                source,
            }
        })?;
        let paths =
            CoreStatePaths::new(&roots.state_root).map_err(LocalProductError::StateFormat)?;
        prepare_core_state_paths(&paths).map_err(LocalProductError::State)?;
        std::fs::create_dir(&roots.config_dir).map_err(|source| LocalProductError::Io {
            operation: "create bootstrap config directory",
            source,
        })?;
        let state = plan_initial_pointer_state_with_key(&generation_id, bootstrap_key)
            .map_err(LocalProductError::StateFormat)?;
        activate_pointer_state(&paths, None, &state).map_err(LocalProductError::State)?;
        Ok(state)
    }

    #[cfg(unix)]
    fn b7_seed_initial_release(
        source_generation: &std::path::Path,
        home: &std::path::Path,
        prefix: &std::path::Path,
        bootstrap_public_key: &std::path::Path,
    ) -> GenerationPointerState {
        b7_bootstrap_initial_release(source_generation, home, prefix, bootstrap_public_key).unwrap()
    }

    #[cfg(unix)]
    const B8_BOOTSTRAP_PROBE_ROLE: &str = "CODEX_B8_BOOTSTRAP_PROBE";

    #[cfg(unix)]
    #[test]
    fn b8_private_bootstrap_probe() {
        if std::env::var(B8_BOOTSTRAP_PROBE_ROLE).as_deref() != Ok("1") {
            return;
        }
        let code = run_internal_bootstrap_mode().unwrap_or(2);
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
        std::process::exit(code);
    }

    #[cfg(unix)]
    const B8_BOOTSTRAP_SWAP_ROLE: &str = "CODEX_B8_BOOTSTRAP_SWAP_PROBE";
    #[cfg(unix)]
    const B8_BOOTSTRAP_SWAP_SOURCE: &str = "CODEX_B8_BOOTSTRAP_SWAP_SOURCE";
    #[cfg(unix)]
    const B8_BOOTSTRAP_SWAP_REPLACEMENT: &str = "CODEX_B8_BOOTSTRAP_SWAP_REPLACEMENT";

    #[cfg(unix)]
    fn b8_write_core_swap_wrapper(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let test_binary = std::env::current_exe().unwrap();
        let shell = resolve_test_shell();
        let script = format!(
            "#!{}\n{}=1 exec '{}' tests::b8_private_bootstrap_swap_probe --exact --nocapture\n",
            shell.display(),
            B8_BOOTSTRAP_SWAP_ROLE,
            test_binary.display()
        );
        std::fs::write(path, script).unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn b8_private_bootstrap_swap_probe() {
        if std::env::var(B8_BOOTSTRAP_SWAP_ROLE).as_deref() != Ok("1") {
            return;
        }
        if std::env::var(INTERNAL_BOOTSTRAP_MODE_ENV).as_deref() == Ok("activate") {
            let source = std::path::PathBuf::from(
                std::env::var_os(B8_BOOTSTRAP_SWAP_SOURCE).expect("bootstrap swap source"),
            );
            let replacement = std::path::PathBuf::from(
                std::env::var_os(B8_BOOTSTRAP_SWAP_REPLACEMENT)
                    .expect("bootstrap swap replacement"),
            );
            std::fs::remove_dir_all(&source).unwrap();
            let staged = source.with_extension("bootstrap-swap");
            copy_local_directory_tree(&replacement, &staged).unwrap();
            std::fs::rename(staged, source).unwrap();
        }
        let code = run_internal_bootstrap_mode().unwrap_or(2);
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
        std::process::exit(code);
    }

    #[cfg(unix)]
    fn b8_process_env(prefix: &std::path::Path, tmp: &std::path::Path) -> TermuxProcessEnvSnapshot {
        TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(prefix.as_os_str().to_owned()),
            tmpdir: Some(tmp.as_os_str().to_owned()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        }
    }

    #[cfg(unix)]
    fn b8_bind_core_digest(
        generation: &std::path::Path,
        openssl: &std::path::Path,
        core: &std::path::Path,
    ) {
        let descriptor_path = generation.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap();
        let digest = openssl_sha256(openssl, core).unwrap();
        assert!(descriptor.contains("core_artifact_digest\tcore-digest\n"));
        std::fs::write(
            &descriptor_path,
            descriptor.replace(
                "core_artifact_digest\tcore-digest\n",
                &format!("core_artifact_digest\t{digest}\n"),
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&descriptor_path).unwrap().permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(descriptor_path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b8_write_core_probe_wrapper(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let test_binary = std::env::current_exe().unwrap();
        let shell = resolve_test_shell();
        let script = format!(
            "#!{}\n{}=1 exec '{}' tests::b8_private_bootstrap_probe --exact --nocapture\n",
            shell.display(),
            B8_BOOTSTRAP_PROBE_ROLE,
            test_binary.display()
        );
        std::fs::write(path, script).unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b2_attach_core(
        generation_dir: &std::path::Path,
        openssl: &std::path::Path,
        core: &std::path::Path,
    ) {
        std::fs::copy(core, generation_dir.join("core")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(generation_dir.join("core"))
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(generation_dir.join("core"), permissions).unwrap();
        b8_bind_core_digest(generation_dir, openssl, core);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b8_slice2_core_initial_bootstrap_is_exact_and_one_shot() {
        let root = temp_root("b8-slice2-core-initial");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "bootstrap-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &live_openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &live_openssl, &private_key);
        b4_install_trusted_release_key(&home, &public_key);

        let roots = b7_public_roots(&home, &prefix);
        let process_env = b8_process_env(&prefix, &tmp);
        let pin = home.join(".local/lib/codex/core/release-public-key.pem");
        bootstrap_self_test(&roots, &pin).unwrap();
        assert_eq!(
            bootstrap_initial_signed_local_release(&release, &roots, &pin, &core, &process_env,)
                .unwrap(),
            "bootstrap-g0"
        );
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        let key = release_public_key_from_pem(&roots.openssl, &pin).unwrap();
        assert_eq!(state.update_key, key);
        assert_eq!(state.current, "bootstrap-g0");
        assert_eq!(state.current_key, key);
        assert_eq!(state.previous, None);
        assert_eq!(state.previous_key, None);
        assert!(roots.generation_root.join("bootstrap-g0").is_dir());
        assert!(matches!(
            bootstrap_self_test(&roots, &pin),
            Err(LocalProductError::Release(
                "fresh bootstrap requires absent authoritative v3 state"
            ))
        ));
        assert!(matches!(
            bootstrap_initial_signed_local_release(&release, &roots, &pin, &core, &process_env,),
            Err(LocalProductError::Release(
                "fresh bootstrap requires absent authoritative v3 state"
            ))
        ));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_fresh_bootstrap_preflights_transaction_residue_before_publication() {
        let root = temp_root("m2-r1-fresh-bootstrap-residue");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "residue-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &live_openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &live_openssl, &private_key);

        let state_root = home.join(".local/share/codex/core");
        std::fs::create_dir_all(&state_root).unwrap();
        let state_temp = state_root.join("activation-state.tmp");
        std::fs::write(&state_temp, b"orphan").unwrap();
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                core.as_os_str(),
                release.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(output
            .stderr
            .windows(b"orphan activation-state temporary exists".len())
            .any(|window| window == b"orphan activation-state temporary exists"));
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());
        assert!(!prefix.join("bin/codex").exists());
        assert!(!state_root.join("activation-state").exists());
        assert_eq!(std::fs::read(&state_temp).unwrap(), b"orphan");
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_fresh_bootstrap_rechecks_core_binding_after_source_swap() {
        let root = temp_root("m2-r1-fresh-bootstrap-core-binding");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let first = b2_write_generation(&source, "binding-g0", false, "supported");
        b4_write_probe_runtime(&first, 0, 0);
        let first_core = root.join("prebuilt-codex-first");
        b8_write_core_swap_wrapper(&first_core);
        b8_bind_core_digest(&first, &live_openssl, &first_core);
        let second = b2_write_generation(&source, "binding-g1", false, "supported");
        b4_write_probe_runtime(&second, 0, 0);
        let second_core = root.join("prebuilt-codex-second");
        b8_write_core_probe_wrapper(&second_core);
        use std::io::Write as _;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&second_core)
            .unwrap()
            .write_all(b"\n# distinct authenticated Core\n")
            .unwrap();
        b8_bind_core_digest(&second, &live_openssl, &second_core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&first, 1, &live_openssl, &private_key);
        b4_write_signed_release(&second, 2, &live_openssl, &private_key);

        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                first_core.as_os_str(),
                first.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env(B8_BOOTSTRAP_SWAP_SOURCE, &first)
            .env(B8_BOOTSTRAP_SWAP_REPLACEMENT, &second)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(
            output
                .stderr
                .windows(b"bootstrap release does not match authenticated Core artifact".len())
                .any(|window| {
                    window == b"bootstrap release does not match authenticated Core artifact"
                }),
            "stderr={:?}",
            output.stderr
        );
        assert!(!home
            .join(".local/share/codex/core/activation-state")
            .exists());
        assert!(!home
            .join(".local/lib/codex/core/generations")
            .join("binding-g1")
            .exists());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b8_slice2_core_bootstrap_rejections_leave_no_authoritative_state() {
        for case in [
            "wrong-key",
            "rotating-key",
            "probe-failure",
            "tampered-manifest",
        ] {
            let root = temp_root(&format!("b8-slice2-{case}"));
            let live_openssl = b4_termux_openssl();
            let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
            let source = b4_source_roots(&root, &live_openssl);
            std::fs::create_dir(&source.generation_root).unwrap();
            let release = b2_write_generation(&source, case, false, "supported");
            b4_write_probe_runtime(&release, if case == "probe-failure" { 17 } else { 0 }, 0);
            let core = root.join("prebuilt-codex");
            b8_write_core_probe_wrapper(&core);
            b8_bind_core_digest(&release, &live_openssl, &core);
            let old_private = root.join("keys/old-private.pem");
            let old_public = root.join("keys/old-public.pem");
            let new_private = root.join("keys/new-private.pem");
            let new_public = root.join("keys/new-public.pem");
            b4_generate_release_keypair(&live_openssl, &old_private, &old_public);
            b4_generate_release_keypair(&live_openssl, &new_private, &new_public);
            match case {
                "wrong-key" | "probe-failure" | "tampered-manifest" => {
                    b4_write_signed_release(&release, 1, &live_openssl, &old_private)
                }
                "rotating-key" => {
                    let files = b4_exact_release_inventory(&release, &live_openssl);
                    b4_write_signed_release_inventory_with_authority(
                        &release,
                        1,
                        &live_openssl,
                        &new_private,
                        Some(&old_private),
                        &files,
                    );
                }
                _ => unreachable!(),
            }
            if case == "tampered-manifest" {
                let manifest_path = release.join("release.manifest");
                let mut manifest = std::fs::read(&manifest_path).unwrap();
                manifest.push(b'\n');
                std::fs::write(manifest_path, manifest).unwrap();
            }
            let pin_source = if case == "wrong-key" {
                &new_public
            } else {
                &old_public
            };
            b4_install_trusted_release_key(&home, pin_source);
            let roots = b7_public_roots(&home, &prefix);
            let process_env = b8_process_env(&prefix, &tmp);
            let pin = home.join(".local/lib/codex/core/release-public-key.pem");
            assert!(
                bootstrap_initial_signed_local_release(
                    &release,
                    &roots,
                    &pin,
                    &core,
                    &process_env,
                )
                .is_err(),
                "case {case} unexpectedly activated"
            );
            let paths = CoreStatePaths::new(&roots.state_root).unwrap();
            assert_eq!(read_pointer_state(&paths).unwrap(), None, "case {case}");
            assert!(!paths.activation_journal.exists(), "case {case}");
            assert!(!paths.activation_journal_temp.exists(), "case {case}");
            assert!(!paths.activation_state_temp.exists(), "case {case}");
            assert!(!roots.generation_root.join(case).exists(), "case {case}");
            remove_temp_root(root);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b8_slice2_bootstrap_script_installs_through_private_core_path() {
        let root = temp_root("b8-slice2-script");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "script-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &live_openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &live_openssl, &private_key);
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                core.as_os_str(),
                release.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env_remove(INTERNAL_BOOTSTRAP_MODE_ENV)
            .env_remove(INTERNAL_BOOTSTRAP_SOURCE_ENV)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert!(prefix.join("bin/codex").is_file());
        assert_eq!(
            openssl_sha256(&live_openssl, &prefix.join("bin/codex")).unwrap(),
            openssl_sha256(&live_openssl, &core).unwrap()
        );
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(state.current, "script-g0");
        assert_eq!(state.previous, None);
        assert!(roots.generation_root.join("script-g0").is_dir());
        assert!(std::fs::read_dir(&tmp).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".codex-bootstrap.")));

        let second = std::process::Command::new(&bootstrap)
            .args([
                core.as_os_str(),
                release.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(second.status.code(), Some(1));
        assert!(second
            .stderr
            .windows(b"existing authoritative v3 state".len())
            .any(|w| { w == b"existing authoritative v3 state" }));
        assert_eq!(read_pointer_state(&paths).unwrap().unwrap(), state);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_bootstrap_snapshots_enforce_input_bounds_before_publication() {
        for case in ["key", "manifest", "signature", "descriptor"] {
            let root = temp_root(&format!("m2-r1-bootstrap-bound-{case}"));
            let live_openssl = b4_termux_openssl();
            let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
            let source = b4_source_roots(&root, &live_openssl);
            std::fs::create_dir(&source.generation_root).unwrap();
            let release = b2_write_generation(&source, case, false, "supported");
            b4_write_probe_runtime(&release, 0, 0);
            let core = root.join("prebuilt-codex");
            b8_write_core_probe_wrapper(&core);
            b8_bind_core_digest(&release, &live_openssl, &core);
            let private_key = root.join("keys/private.pem");
            let public_key = root.join("keys/public.pem");
            b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
            b4_write_signed_release(&release, 1, &live_openssl, &private_key);

            let (bounded_path, maximum, label) = match case {
                "key" => (public_key.clone(), 16 * 1024, "bootstrap public key"),
                "manifest" => (
                    release.join("release.manifest"),
                    128 * 1024,
                    "release.manifest",
                ),
                "signature" => (release.join("release.sig"), 1024, "release.sig"),
                "descriptor" => (
                    release.join("generation.meta"),
                    64 * 1024,
                    "generation.meta",
                ),
                _ => unreachable!(),
            };
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&bounded_path)
                .unwrap();
            file.set_len(maximum + 1).unwrap();

            let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../bootstrap/codex-bootstrap");
            let output = std::process::Command::new(&bootstrap)
                .args([
                    core.as_os_str(),
                    release.as_os_str(),
                    public_key.as_os_str(),
                ])
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .output()
                .unwrap();
            let expected = format!("{label} exceeds its byte bound");
            assert_eq!(output.status.code(), Some(1), "case {case}");
            assert!(
                output
                    .stderr
                    .windows(expected.len())
                    .any(|window| window == expected.as_bytes()),
                "case {case}: stderr={:?}",
                output.stderr
            );
            assert!(!prefix.join("bin/codex").exists(), "case {case}");
            assert!(!home
                .join(".local/lib/codex/core/release-public-key.pem")
                .exists());
            assert!(!home
                .join(".local/share/codex/core/activation-state")
                .exists());
            remove_temp_root(root);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_fresh_bootstrap_retries_after_entrypoint_parent_sync_failure() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("m2-r1-fresh-entrypoint-retry");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "m2-r1-fresh-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &live_openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &live_openssl, &private_key);

        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let run_bootstrap = || {
            std::process::Command::new(&bootstrap)
                .args([
                    core.as_os_str(),
                    release.as_os_str(),
                    public_key.as_os_str(),
                ])
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .output()
                .unwrap()
        };
        let bin = prefix.join("bin");
        let mut bin_permissions = std::fs::metadata(&bin).unwrap().permissions();
        bin_permissions.set_mode(0o300);
        std::fs::set_permissions(&bin, bin_permissions).unwrap();

        let first = run_bootstrap();
        assert_eq!(first.status.code(), Some(1));
        let entrypoint = bin.join("codex");
        assert!(entrypoint.is_file());
        assert_eq!(
            openssl_sha256(&live_openssl, &entrypoint).unwrap(),
            openssl_sha256(&live_openssl, &core).unwrap()
        );
        let paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), None);
        assert!(!paths.activation_journal.exists());
        assert!(!paths.activation_journal_temp.exists());
        assert!(!paths.activation_state_temp.exists());

        let mut entrypoint_permissions = std::fs::metadata(&entrypoint).unwrap().permissions();
        entrypoint_permissions.set_mode(0o751);
        std::fs::set_permissions(&entrypoint, entrypoint_permissions).unwrap();
        let second = run_bootstrap();
        assert_eq!(second.status.code(), Some(1));
        assert_eq!(read_pointer_state(&paths).unwrap(), None);
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );

        let mut bin_permissions = std::fs::metadata(&bin).unwrap().permissions();
        bin_permissions.set_mode(0o755);
        std::fs::set_permissions(&bin, bin_permissions).unwrap();
        let completed = run_bootstrap();
        assert_eq!(
            completed.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            completed.stdout,
            completed.stderr
        );
        assert_eq!(
            read_pointer_state(&paths).unwrap().unwrap().current,
            "m2-r1-fresh-g0"
        );
        assert_eq!(
            std::fs::metadata(&entrypoint).unwrap().permissions().mode() & 0o7777,
            0o755
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_fresh_bootstrap_retries_after_trust_seed_parent_sync_failure() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("m2-r1-fresh-trust-seed-retry");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "m2-r1-pin-g0", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &live_openssl, &core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &live_openssl, &private_key);

        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let run_bootstrap = || {
            std::process::Command::new(&bootstrap)
                .args([
                    core.as_os_str(),
                    release.as_os_str(),
                    public_key.as_os_str(),
                ])
                .env("HOME", &home)
                .env("PREFIX", &prefix)
                .env("TMPDIR", &tmp)
                .output()
                .unwrap()
        };
        let pin_parent = home.join(".local/lib/codex/core");
        std::fs::create_dir_all(&pin_parent).unwrap();
        let mut pin_permissions = std::fs::metadata(&pin_parent).unwrap().permissions();
        pin_permissions.set_mode(0o300);
        std::fs::set_permissions(&pin_parent, pin_permissions).unwrap();

        let first = run_bootstrap();
        assert_eq!(first.status.code(), Some(1));
        let pin = pin_parent.join("release-public-key.pem");
        assert!(pin.is_file());
        let roots = b7_public_roots(&home, &prefix);
        assert_eq!(
            release_public_key_from_pem(&roots.openssl, &pin).unwrap(),
            release_public_key_from_pem(&live_openssl, &public_key).unwrap()
        );
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), None);
        assert!(!prefix.join("bin/codex").exists());

        let mut pin_file_permissions = std::fs::metadata(&pin).unwrap().permissions();
        pin_file_permissions.set_mode(0o600);
        std::fs::set_permissions(&pin, pin_file_permissions).unwrap();
        let second = run_bootstrap();
        assert_eq!(second.status.code(), Some(1));
        assert_eq!(read_pointer_state(&paths).unwrap(), None);
        assert!(!prefix.join("bin/codex").exists());
        assert_eq!(
            std::fs::metadata(&pin).unwrap().permissions().mode() & 0o7777,
            0o644
        );

        let mut pin_permissions = std::fs::metadata(&pin_parent).unwrap().permissions();
        pin_permissions.set_mode(0o755);
        std::fs::set_permissions(&pin_parent, pin_permissions).unwrap();
        let completed = run_bootstrap();
        assert_eq!(
            completed.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            completed.stdout,
            completed.stderr
        );
        assert_eq!(
            read_pointer_state(&paths).unwrap().unwrap().current,
            "m2-r1-pin-g0"
        );
        assert!(prefix.join("bin/codex").is_file());
        assert!(std::fs::read_dir(&pin_parent).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".release-public-key.pem.bootstrap-")
        }));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b8_slice2_bootstrap_script_core_digest_mismatch_fails_before_install() {
        let root = temp_root("b8-slice2-script-digest");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let source = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir(&source.generation_root).unwrap();
        let release = b2_write_generation(&source, "script-digest", false, "supported");
        b4_write_probe_runtime(&release, 0, 0);
        let authenticated_core = root.join("authenticated-codex");
        b8_write_core_probe_wrapper(&authenticated_core);
        b8_bind_core_digest(&release, &live_openssl, &authenticated_core);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &public_key);
        b4_write_signed_release(&release, 1, &live_openssl, &private_key);
        let wrong_core = root.join("wrong-codex");
        std::fs::copy(&authenticated_core, &wrong_core).unwrap();
        use std::io::Write as _;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&wrong_core)
            .unwrap()
            .write_all(b"\n# digest mismatch\n")
            .unwrap();
        let bootstrap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bootstrap/codex-bootstrap");
        let output = std::process::Command::new(&bootstrap)
            .args([
                wrong_core.as_os_str(),
                release.as_os_str(),
                public_key.as_os_str(),
            ])
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output
            .stderr
            .windows(b"Core artifact does not match".len())
            .any(|w| { w == b"Core artifact does not match" }));
        assert!(!prefix.join("bin/codex").exists());
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());
        assert!(!home
            .join(".local/share/codex/core/activation-state")
            .exists());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn assert_human_update_error_contains(stderr: &[u8], expected: &[u8]) {
        let actual = String::from_utf8_lossy(stderr).to_ascii_lowercase();
        let expected = String::from_utf8_lossy(expected).to_ascii_lowercase();
        assert!(
            actual.contains(expected.as_str()),
            "stderr={stderr:?} expected={expected:?}"
        );
        assert!(
            stderr.ends_with("❌\n".as_bytes()),
            "operational update failure must end with the failure marker: {stderr:?}"
        );
    }

    #[cfg(unix)]
    fn b4_assert_public_update_rejected(
        source_generation: &std::path::Path,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
        expected: &[u8],
    ) {
        let output = b4_run_public_update(source_generation, home, prefix, tmp);
        assert_eq!(
            output.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert_human_update_error_contains(&output.stderr, expected);
    }

    #[cfg(unix)]
    fn b4_assert_public_update_activated(
        source_generation: &std::path::Path,
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
        generation_id: &str,
    ) {
        let output = b4_run_public_update(source_generation, home, prefix, tmp);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let _ = generation_id;
        let expected = "Codex 9.9.9 is now active. ✅\n";
        assert!(
            output
                .stdout
                .windows(expected.len())
                .any(|window| window == expected.as_bytes()),
            "stdout={:?}",
            output.stdout
        );
    }

    #[cfg(unix)]
    fn b4_assert_public_rollback_activated(
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
        generation_id: &str,
    ) {
        let output = b4_run_public_rollback(home, prefix, tmp);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let _ = generation_id;
        let expected = "Rolled back the Termux release for Codex 9.9.9. ✅\n";
        assert!(
            output
                .stdout
                .windows(expected.len())
                .any(|window| window == expected.as_bytes()),
            "stdout={:?}",
            output.stdout
        );
    }

    #[cfg(unix)]
    fn b4_assert_public_rollback_rejected(
        home: &std::path::Path,
        prefix: &std::path::Path,
        tmp: &std::path::Path,
        expected: &[u8],
    ) {
        let output = b4_run_public_rollback(home, prefix, tmp);
        assert_eq!(
            output.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert_human_update_error_contains(&output.stderr, expected);
    }

    #[cfg(unix)]
    fn b4_assert_no_target_generation_or_state(home: &std::path::Path) {
        assert!(!home.join(".local/lib/codex/core/generations").exists());
        let state_root = home.join(".local/share/codex/core");
        assert!(!state_root.join("activation-state").exists());
        assert!(!state_root.join("activation-journal").exists());
        assert!(!state_root.join("config").exists());
    }

    #[cfg(unix)]
    fn b4_minimal_release_manifest() -> String {
        format!(
            concat!(
                "{}\n",
                "generation_id\tmanifest-only\n",
                "release_sequence\t1\n",
                "channel\t{}\n",
                "expected_platform\t{}\n",
                "expected_architecture\t{}\n",
                "core_api_identity\t{}\n",
                "persistent_schema_identity\t{}\n",
                "release_public_key\t{}\n",
                "file_count\t1\n",
                "file\tgeneration.meta\t{}\t0644\n",
            ),
            LOCAL_RELEASE_FORMAT,
            LOCAL_RELEASE_CHANNEL,
            std::env::consts::OS,
            std::env::consts::ARCH,
            CORE_API_IDENTITY,
            PERSISTENT_SCHEMA_IDENTITY,
            "0".repeat(64),
            "0".repeat(64),
        )
    }

    #[cfg(unix)]
    #[test]
    fn public_update_probe() {
        if std::env::var(UPDATE_PROBE_ROLE).as_deref() != Ok("1") {
            return;
        }
        let code = if std::env::var(UPDATE_PROBE_BUILD_LOCAL).as_deref() == Ok("1") {
            assert!(std::env::var_os(UPDATE_PROBE_SOURCE).is_none());
            assert!(std::env::var_os(UPDATE_PROBE_REMOTE).is_none());
            assert!(std::env::var_os(UPDATE_PROBE_CHANNEL).is_none());
            run_public_main([OsString::from("update"), OsString::from("--build-local")])
        } else if std::env::var(UPDATE_PROBE_CHANNEL).as_deref() == Ok("1") {
            assert!(std::env::var_os(UPDATE_PROBE_SOURCE).is_none());
            assert!(std::env::var_os(UPDATE_PROBE_REMOTE).is_none());
            if std::env::var(UPDATE_PROBE_FORCE).as_deref() == Ok("1") {
                run_public_main([OsString::from("update"), OsString::from("--force")])
            } else {
                run_public_main([OsString::from("update")])
            }
        } else {
            match (
                std::env::var_os(UPDATE_PROBE_SOURCE),
                std::env::var_os(UPDATE_PROBE_REMOTE),
            ) {
                (Some(source), None) => {
                    if std::env::var("CODEX_TEST_BOOTSTRAP_FIRST").as_deref() == Ok("1") {
                        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
                        let prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
                        let state_paths =
                            CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
                        if read_pointer_state(&state_paths).unwrap().is_none() {
                            let pinned = home.join(".local/lib/codex/core/release-public-key.pem");
                            match b7_bootstrap_initial_release(
                                std::path::Path::new(&source),
                                &home,
                                &prefix,
                                &pinned,
                            ) {
                                Ok(state) => {
                                    let _ = state;
                                    println!("Codex 9.9.9 is now active. ✅");
                                    0
                                }
                                Err(error) => {
                                    eprintln!("{}", render_update_failure(&error));
                                    1
                                }
                            }
                        } else {
                            run_public_main([
                                OsString::from("update"),
                                OsString::from("--local"),
                                source,
                            ])
                        }
                    } else {
                        run_public_main([
                            OsString::from("update"),
                            OsString::from("--local"),
                            source,
                        ])
                    }
                }
                (None, Some(remote)) => {
                    run_public_main([OsString::from("update"), OsString::from("--remote"), remote])
                }
                (None, None) => {
                    run_public_main([OsString::from("update"), OsString::from("--rollback")])
                }
                (Some(_), Some(_)) => panic!("update probe source is ambiguous"),
            }
        };
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
        std::process::exit(code);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_trust_manifest_parser_is_strict_ordered_and_bounded() {
        let valid = b4_minimal_release_manifest();
        let parsed = parse_local_release_manifest(valid.as_bytes()).unwrap();
        assert_eq!(parsed.generation_id, "manifest-only");
        assert_eq!(parsed.release_sequence, 1);
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].mode, 0o644);

        let invalid_text = [
            valid.trim_end_matches('\n').to_string(),
            valid.replacen(LOCAL_RELEASE_FORMAT, "unsupported-release", 1),
            valid.replacen(LOCAL_RELEASE_FORMAT, "codex-release-v1", 1),
            valid.replacen(
                "release_sequence\t1\nchannel\tstable\n",
                "channel\tstable\nrelease_sequence\t1\n",
                1,
            ),
            valid.replacen("release_sequence\t1\n", "release_sequence\t0\n", 1),
            valid.replacen("release_sequence\t1\n", "release_sequence\t01\n", 1),
            valid.replacen("release_sequence\t1\n", "release_sequence\t+1\n", 1),
            valid.replacen("file_count\t1\n", "file_count\t0\n", 1),
            valid.replacen("file_count\t1\n", "file_count\t01\n", 1),
            valid.replacen(
                "file_count\t1\n",
                &format!("file_count\t{}\n", LOCAL_RELEASE_MAX_FILES + 1),
                1,
            ),
            valid.replacen("\t0644\n", "\t644\n", 1),
            valid.replacen("\t0644\n", "\t0844\n", 1),
            valid.replacen("\t0644\n", "\t0044\n", 1),
            valid.replacen("file\tgeneration.meta\t", "file\truntime\t", 1),
            format!("{valid}unexpected\tfield\n"),
            valid.replace('\n', "\r\n"),
        ];
        for bytes in invalid_text.iter().map(String::as_bytes) {
            assert!(parse_local_release_manifest(bytes).is_err());
        }

        let mut invalid_utf8 = valid.into_bytes();
        invalid_utf8[0] = 0xff;
        assert!(parse_local_release_manifest(&invalid_utf8).is_err());
        assert!(parse_local_release_manifest(&vec![b'x'; LOCAL_RELEASE_MAX_BYTES + 1]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_trust_policy_accepts_only_current_contract() {
        let valid = parse_local_release_manifest(b4_minimal_release_manifest().as_bytes()).unwrap();
        validate_local_release_policy(&valid).unwrap();

        let mut coordinated = valid.clone();
        coordinated.format = LocalReleaseFormat::CoordinatedV4;
        coordinated.files.push(ReleaseFileEntry {
            relative_path: "core".to_owned(),
            sha256: "0".repeat(64),
            mode: 0o755,
        });
        validate_local_release_policy(&coordinated).unwrap();
        let mut legacy_with_core = valid.clone();
        legacy_with_core.files.push(ReleaseFileEntry {
            relative_path: "core".to_owned(),
            sha256: "0".repeat(64),
            mode: 0o755,
        });
        assert!(matches!(
            validate_local_release_policy(&legacy_with_core),
            Err(LocalProductError::ReleasePolicy(_))
        ));
        let mut coordinated_without_core = valid.clone();
        coordinated_without_core.format = LocalReleaseFormat::CoordinatedV4;
        assert!(matches!(
            validate_local_release_policy(&coordinated_without_core),
            Err(LocalProductError::ReleasePolicy(_))
        ));

        let invalid = [
            {
                let mut manifest = valid.clone();
                manifest.channel = "other".to_string();
                manifest
            },
            {
                let mut manifest = valid.clone();
                manifest.expected_platform = "other".to_string();
                manifest
            },
            {
                let mut manifest = valid.clone();
                manifest.expected_architecture = "other".to_string();
                manifest
            },
            {
                let mut manifest = valid.clone();
                manifest.core_api_identity = "other".to_string();
                manifest
            },
            {
                let mut manifest = valid.clone();
                manifest.persistent_schema_identity = "other".to_string();
                manifest
            },
        ];
        for manifest in invalid {
            assert!(matches!(
                validate_local_release_policy(&manifest),
                Err(LocalProductError::ReleasePolicy(_))
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b7_slice1_signed_transition_admission_is_bounded_and_dual_proved() {
        let root = temp_root("b7-slice1-transition");
        let openssl = b4_termux_openssl();
        let old_private = root.join("keys/old-private.pem");
        let old_public = root.join("keys/old-public.pem");
        let new_private = root.join("keys/new-private.pem");
        let new_public = root.join("keys/new-public.pem");
        let wrong_private = root.join("keys/wrong-private.pem");
        let wrong_public = root.join("keys/wrong-public.pem");
        b4_generate_release_keypair(&openssl, &old_private, &old_public);
        b4_generate_release_keypair(&openssl, &new_private, &new_public);
        b4_generate_release_keypair(&openssl, &wrong_private, &wrong_public);
        let old_key = release_public_key_from_pem(&openssl, &old_public).unwrap();
        let new_key = release_public_key_from_pem(&openssl, &new_public).unwrap();
        assert_ne!(old_key, new_key);

        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let non_rotation =
            b2_write_generation(&source_roots, "b7-non-rotation", false, "unsupported");
        b4_write_signed_release(&non_rotation, 1, &openssl, &old_private);
        let admitted =
            verify_local_release_control_with_key(&non_rotation, &openssl, old_key).unwrap();
        assert_eq!(admitted.release_public_key, old_key);
        b4_sign_release_manifest_to(
            &non_rotation,
            &openssl,
            &old_private,
            "release-authority.sig",
        );
        assert!(matches!(
            verify_local_release_control_with_key(&non_rotation, &openssl, old_key),
            Err(LocalProductError::Release(
                "release authority signature is unexpected for a non-rotation"
            ))
        ));
        std::fs::remove_file(non_rotation.join("release-authority.sig")).unwrap();

        let rotation = b2_write_generation(&source_roots, "b7-rotation", false, "unsupported");
        let files = b4_exact_release_inventory(&rotation, &openssl);
        b4_write_signed_release_inventory_with_authority(
            &rotation,
            2,
            &openssl,
            &new_private,
            Some(&old_private),
            &files,
        );
        let admitted = verify_local_release_control_with_key(&rotation, &openssl, old_key).unwrap();
        assert_eq!(admitted.release_public_key, new_key);

        std::fs::remove_file(rotation.join("release-authority.sig")).unwrap();
        assert!(matches!(
            verify_local_release_control_with_key(&rotation, &openssl, old_key),
            Err(LocalProductError::Release(
                "release authority signature is required for key rotation"
            ))
        ));
        b4_sign_release_manifest_to(&rotation, &openssl, &wrong_private, "release-authority.sig");
        assert!(matches!(
            verify_local_release_control_with_key(&rotation, &openssl, old_key),
            Err(LocalProductError::SignatureRejected)
        ));
        b4_sign_release_manifest_to(&rotation, &openssl, &old_private, "release-authority.sig");
        b4_sign_release_manifest_to(&rotation, &openssl, &old_private, "release.sig");
        assert!(matches!(
            verify_local_release_control_with_key(&rotation, &openssl, old_key),
            Err(LocalProductError::SignatureRejected)
        ));

        for invalid in [
            "",
            &"0".repeat(63),
            &"0".repeat(65),
            &"A".repeat(64),
            &"g".repeat(64),
        ] {
            assert!(ReleasePublicKey::parse_hex(invalid).is_none());
        }
        assert_eq!(
            ReleasePublicKey::parse_hex(&old_key.to_hex()),
            Some(old_key)
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_trust_signature_binds_exact_manifest_bytes() {
        let root = temp_root("b4-signature-bytes");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);

        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let source_generation =
            b2_write_generation(&source_roots, "signed-exact", false, "unsupported");
        b4_write_signed_release(&source_generation, 1, &openssl, &private_key);
        verify_local_release_bundle(&source_generation, &openssl, &public_key).unwrap();

        let manifest_path = source_generation.join("release.manifest");
        let exact = std::fs::read_to_string(&manifest_path).unwrap();
        let changed = exact.replacen("release_sequence\t1\n", "release_sequence\t2\n", 1);
        assert_ne!(changed, exact);
        std::fs::write(&manifest_path, changed).unwrap();
        assert!(matches!(
            verify_local_release_bundle(&source_generation, &openssl, &public_key),
            Err(LocalProductError::SignatureRejected)
        ));

        std::fs::write(&manifest_path, exact).unwrap();
        verify_local_release_bundle(&source_generation, &openssl, &public_key).unwrap();
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_trust_public_failures_never_stage_or_activate() {
        use std::os::unix::fs::symlink;

        let root = temp_root("b4-public-trust-failures");
        let openssl = b4_termux_openssl();
        let good_private = root.join("keys/good-private.pem");
        let good_public = root.join("keys/good-public.pem");
        let wrong_private = root.join("keys/wrong-private.pem");
        let wrong_public = root.join("keys/wrong-public.pem");
        b4_generate_release_keypair(&openssl, &good_private, &good_public);
        b4_generate_release_keypair(&openssl, &wrong_private, &wrong_public);

        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let source_generation =
            b2_write_generation(&source_roots, "trust-negative", false, "unsupported");
        b4_write_signed_release(&source_generation, 1, &openssl, &good_private);
        std::fs::copy(
            &good_public,
            source_generation.join("release-public-key.pem"),
        )
        .unwrap();
        symlink(&openssl, source_generation.join("openssl")).unwrap();

        let run_failure = |label: &str,
                           install_openssl: bool,
                           trusted_key: Option<&std::path::Path>,
                           expected: &[u8]| {
            let target = root.join(label);
            let (home, prefix, tmp) =
                b4_prepare_public_environment(&target, &openssl, install_openssl);
            if let Some(key) = trusted_key {
                let pinned = home.join(".local/lib/codex/core/release-public-key.pem");
                std::fs::create_dir_all(pinned.parent().unwrap()).unwrap();
                std::fs::copy(key, pinned).unwrap();
            }
            let output = b4_run_public_update(&source_generation, &home, &prefix, &tmp);
            assert_eq!(
                output.status.code(),
                Some(1),
                "stdout={:?} stderr={:?}",
                output.stdout,
                output.stderr
            );
            assert_human_update_error_contains(&output.stderr, expected);
            b4_assert_no_target_generation_or_state(&home);
        };

        run_failure(
            "missing-openssl",
            false,
            Some(&good_public),
            b"Termux OpenSSL is unavailable",
        );
        run_failure(
            "missing-key",
            true,
            None,
            b"trusted release public key is unavailable",
        );
        run_failure(
            "wrong-pinned-key",
            true,
            Some(&wrong_public),
            b"release authority signature is required for key rotation",
        );

        let manifest_path = source_generation.join("release.manifest");
        let unsupported = std::fs::read_to_string(&manifest_path).unwrap().replacen(
            "channel\tstable\n",
            "channel\tunsupported\n",
            1,
        );
        std::fs::write(&manifest_path, unsupported).unwrap();
        b4_sign_release_manifest(&source_generation, &openssl, &good_private);
        run_failure(
            "policy-mismatch",
            true,
            Some(&good_public),
            b"release channel is not supported",
        );

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_inventory_path_grammar_and_descriptor_helper_count_are_bounded() {
        for path in [
            "generation.meta",
            "runtime",
            "manager",
            "helpers/0",
            "helpers/42",
            "compat/asset",
            "compat/nested/asset",
        ] {
            assert!(valid_release_relative_path(path), "valid path: {path:?}");
        }
        for path in [
            "",
            "/runtime",
            "runtime/",
            "other",
            "helpers/",
            "helpers/00",
            "helpers/+1",
            "helpers/1/extra",
            "compat",
            "compat/",
            "compat//asset",
            "compat/./asset",
            "compat/../asset",
            "compat/line\nbreak",
            "compat/tab\tname",
        ] {
            assert!(!valid_release_relative_path(path), "invalid path: {path:?}");
        }

        let (root, roots) = b2_test_roots("b4-inventory-paths");
        let generation = b2_write_generation(&roots, "inventory-shape", true, "unsupported");
        let descriptor_path = generation.join("generation.meta");
        let original = std::fs::read_to_string(&descriptor_path).unwrap();
        let with_helper = original.replace(
            "helper_count\t0\n",
            "helper_count\t1\nhelper\thelper-a\thelper-digest\n",
        );
        std::fs::write(&descriptor_path, &with_helper).unwrap();
        std::fs::create_dir(generation.join("helpers")).unwrap();
        std::fs::write(generation.join("helpers/0"), b"helper").unwrap();
        std::fs::create_dir(generation.join("compat/nested")).unwrap();
        std::fs::write(generation.join("compat/nested/asset"), b"asset").unwrap();

        let loaded = load_local_generation(&generation).unwrap();
        assert_eq!(
            exact_release_file_paths(&generation, &loaded).unwrap(),
            vec![
                "compat/codex-code-mode-host".to_string(),
                "compat/nested/asset".to_string(),
                "generation.meta".to_string(),
                "helpers/0".to_string(),
                "manager".to_string(),
                "runtime".to_string(),
            ]
        );

        std::fs::write(
            &descriptor_path,
            with_helper.replacen("helper_count\t1\n", "helper_count\t01\n", 1),
        )
        .unwrap();
        assert!(matches!(
            load_local_generation(&generation),
            Err(LocalProductError::Descriptor(
                "generation helper count is invalid"
            ))
        ));

        let outside_bound = LOCAL_RELEASE_MAX_FILES.saturating_sub(2) + 1;
        std::fs::write(
            &descriptor_path,
            with_helper.replacen(
                "helper_count\t1\n",
                &format!("helper_count\t{outside_bound}\n"),
                1,
            ),
        )
        .unwrap();
        assert!(matches!(
            load_local_generation(&generation),
            Err(LocalProductError::Descriptor(
                "generation helper count is outside the supported bound"
            ))
        ));

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_generation_descriptor_read_is_bounded_before_loading() {
        let (root, roots) = b2_test_roots("m2-r1-bounded-descriptor");
        let generation = b2_write_generation(&roots, "oversized-descriptor", false, "unsupported");
        let descriptor = generation.join("generation.meta");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&descriptor)
            .unwrap();
        file.set_len((LOCAL_GENERATION_MAX_BYTES + 1) as u64)
            .unwrap();
        assert!(matches!(
            load_local_generation(&generation),
            Err(LocalProductError::Descriptor(
                "generation descriptor is too large"
            ))
        ));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_inventory_source_digest_and_file_set_fail_before_staging() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b4-inventory-source-failures");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);

        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let run_rejected = |label: &str, generation: &std::path::Path, expected: &[u8]| {
            let target = root.join(format!("target-{label}"));
            let (home, prefix, tmp) = b4_prepare_public_environment(&target, &openssl, true);
            b4_install_trusted_release_key(&home, &public_key);
            b4_assert_public_update_rejected(generation, &home, &prefix, &tmp, expected);
            b4_assert_no_target_generation_or_state(&home);
        };

        let digest_mismatch =
            b2_write_generation(&source_roots, "digest-mismatch", false, "unsupported");
        let mut files = b4_exact_release_inventory(&digest_mismatch, &openssl);
        files
            .iter_mut()
            .find(|file| file.relative_path == "runtime")
            .unwrap()
            .sha256 = "0".repeat(64);
        b4_write_signed_release_inventory(&digest_mismatch, 1, &openssl, &private_key, &files);
        assert!(matches!(
            verify_local_release_bundle(&digest_mismatch, &openssl, &public_key),
            Err(LocalProductError::ReleaseDigestMismatch)
        ));
        run_rejected(
            "digest-mismatch",
            &digest_mismatch,
            b"release file inventory digest mismatch",
        );

        let mode_mismatch =
            b2_write_generation(&source_roots, "mode-mismatch", false, "unsupported");
        b4_write_signed_release(&mode_mismatch, 2, &openssl, &private_key);
        let runtime = mode_mismatch.join("runtime");
        let mut permissions = std::fs::metadata(&runtime).unwrap().permissions();
        permissions.set_mode(permissions.mode() & !0o100);
        std::fs::set_permissions(&runtime, permissions).unwrap();
        assert!(matches!(
            verify_local_release_bundle(&mode_mismatch, &openssl, &public_key),
            Err(LocalProductError::ReleaseModeMismatch)
        ));
        run_rejected(
            "mode-mismatch",
            &mode_mismatch,
            b"release file inventory mode mismatch",
        );

        let omitted = b2_write_generation(&source_roots, "omitted-file", false, "unsupported");
        let files: Vec<_> = b4_exact_release_inventory(&omitted, &openssl)
            .into_iter()
            .filter(|file| file.relative_path != "runtime")
            .collect();
        b4_write_signed_release_inventory(&omitted, 3, &openssl, &private_key, &files);
        assert!(matches!(
            verify_local_release_bundle(&omitted, &openssl, &public_key),
            Err(LocalProductError::Release(
                "release file inventory does not exactly match generation content"
            ))
        ));
        run_rejected(
            "omitted-file",
            &omitted,
            b"release file inventory does not exactly match generation content",
        );

        let missing = b2_write_generation(&source_roots, "missing-file", false, "unsupported");
        let mut files = b4_exact_release_inventory(&missing, &openssl);
        files.push(ReleaseFileEntry {
            relative_path: "compat/missing".to_string(),
            sha256: "0".repeat(64),
            mode: 0o644,
        });
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        b4_write_signed_release_inventory(&missing, 4, &openssl, &private_key, &files);
        assert!(matches!(
            verify_local_release_bundle(&missing, &openssl, &public_key),
            Err(LocalProductError::Release(
                "release file inventory does not exactly match generation content"
            ))
        ));
        run_rejected(
            "missing-file",
            &missing,
            b"release file inventory does not exactly match generation content",
        );

        let unlisted = b2_write_generation(&source_roots, "unlisted-file", false, "unsupported");
        b4_write_signed_release(&unlisted, 5, &openssl, &private_key);
        std::fs::write(unlisted.join("compat/unlisted"), b"unlisted").unwrap();
        assert!(matches!(
            verify_local_release_bundle(&unlisted, &openssl, &public_key),
            Err(LocalProductError::Release(
                "release file inventory does not exactly match generation content"
            ))
        ));
        run_rejected(
            "unlisted-file",
            &unlisted,
            b"release file inventory does not exactly match generation content",
        );

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_inventory_staged_copy_retains_metadata_and_reverifies_digest() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b4-inventory-staged-copy");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);

        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let source = b2_write_generation(&source_roots, "staged-copy", true, "unsupported");
        let descriptor_path = source.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap().replace(
            "helper_count\t0\n",
            "helper_count\t1\nhelper\thelper-a\thelper-digest\n",
        );
        std::fs::write(&descriptor_path, descriptor).unwrap();
        std::fs::create_dir(source.join("helpers")).unwrap();
        std::fs::write(source.join("helpers/0"), b"helper-content").unwrap();
        let mut helper_mode = std::fs::metadata(source.join("helpers/0"))
            .unwrap()
            .permissions();
        helper_mode.set_mode(0o700);
        std::fs::set_permissions(source.join("helpers/0"), helper_mode).unwrap();
        std::fs::create_dir(source.join("compat/nested")).unwrap();
        std::fs::write(source.join("compat/nested/asset"), b"compat-content").unwrap();
        b4_write_signed_release(&source, 7, &openssl, &private_key);

        let (source_release, _) =
            verify_local_release_bundle(&source, &openssl, &public_key).unwrap();
        let source_manifest = std::fs::read(source.join("release.manifest")).unwrap();
        let source_signature = std::fs::read(source.join("release.sig")).unwrap();
        let (target_root, target) = b2_test_roots("b4-inventory-staged-target");
        assert_eq!(
            stage_local_generation(&source, &target.generation_root).unwrap(),
            "staged-copy"
        );
        let staged = target.generation_root.join("staged-copy");
        assert_eq!(
            std::fs::read(staged.join("release.manifest")).unwrap(),
            source_manifest
        );
        assert_eq!(
            std::fs::read(staged.join("release.sig")).unwrap(),
            source_signature
        );
        let (staged_release, _) =
            verify_local_release_bundle(&staged, &openssl, &public_key).unwrap();
        assert_eq!(staged_release, source_release);
        assert_eq!(
            std::fs::read(staged.join("compat/nested/asset")).unwrap(),
            b"compat-content"
        );
        assert_eq!(
            std::fs::read(staged.join("helpers/0")).unwrap(),
            b"helper-content"
        );

        std::fs::write(staged.join("runtime"), b"tampered-after-staging").unwrap();
        assert!(matches!(
            verify_local_release_bundle(&staged, &openssl, &public_key),
            Err(LocalProductError::ReleaseDigestMismatch)
        ));
        let state_paths = CoreStatePaths::new(&target.state_root).unwrap();
        assert!(!state_paths.activation_state.exists());
        assert!(!state_paths.activation_journal.exists());
        assert!(staged.is_dir());
        assert!(b3_candidate_entries(&target.generation_root).is_empty());

        remove_temp_root(target_root);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_inventory_unsafe_sources_and_publication_roots_cannot_escape() {
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::fs::symlink;

        let root = temp_root("b4-inventory-unsafe");
        let openssl = b4_termux_openssl();
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();

        let run_source_rejected = |label: &str, generation: &std::path::Path, expected: &[u8]| {
            let target = root.join(format!("target-{label}"));
            let (home, prefix, tmp) = b4_prepare_public_environment(&target, &openssl, true);
            b4_install_trusted_release_key(&home, &public_key);
            b4_assert_public_update_rejected(generation, &home, &prefix, &tmp, expected);
            b4_assert_no_target_generation_or_state(&home);
        };

        let runtime_link = b2_write_generation(&source_roots, "runtime-link", false, "unsupported");
        b4_write_signed_release(&runtime_link, 1, &openssl, &private_key);
        let outside_runtime = root.join("outside-runtime");
        std::fs::rename(runtime_link.join("runtime"), &outside_runtime).unwrap();
        symlink(&outside_runtime, runtime_link.join("runtime")).unwrap();
        run_source_rejected(
            "runtime-link",
            &runtime_link,
            b"activated generation runtime must be a regular file",
        );

        let compat_link = b2_write_generation(&source_roots, "compat-link", false, "unsupported");
        b4_write_signed_release(&compat_link, 2, &openssl, &private_key);
        std::fs::remove_dir_all(compat_link.join("compat")).unwrap();
        let outside_compat = root.join("outside-compat");
        std::fs::create_dir(&outside_compat).unwrap();
        std::fs::write(outside_compat.join("outside"), b"outside").unwrap();
        symlink(&outside_compat, compat_link.join("compat")).unwrap();
        run_source_rejected(
            "compat-link",
            &compat_link,
            b"activated generation compatibility directory must be a real directory",
        );

        let source_link_target =
            b2_write_generation(&source_roots, "source-link", false, "unsupported");
        b4_write_signed_release(&source_link_target, 3, &openssl, &private_key);
        let source_link = root.join("source-generation-link");
        symlink(&source_link_target, &source_link).unwrap();
        run_source_rejected(
            "source-link",
            &source_link,
            b"release generation root must be a real directory",
        );

        let non_utf8 = b2_write_generation(&source_roots, "non-utf8", false, "unsupported");
        b4_write_signed_release(&non_utf8, 4, &openssl, &private_key);
        let invalid_name = std::ffi::OsString::from_vec(vec![b'a', 0xff]);
        std::fs::write(non_utf8.join("compat").join(invalid_name), b"invalid-name").unwrap();
        run_source_rejected(
            "non-utf8",
            &non_utf8,
            b"release file path is not supported UTF-8",
        );

        let safe_source = b2_write_generation(&source_roots, "safe-source", false, "unsupported");
        b4_write_signed_release(&safe_source, 5, &openssl, &private_key);
        let target = root.join("target-generation-root-link");
        let (home, prefix, tmp) = b4_prepare_public_environment(&target, &openssl, true);
        b4_install_trusted_release_key(&home, &public_key);
        let outside_generation_root = root.join("outside-generation-root");
        std::fs::create_dir(&outside_generation_root).unwrap();
        let generation_root = home.join(".local/lib/codex/core/generations");
        std::fs::create_dir_all(generation_root.parent().unwrap()).unwrap();
        symlink(&outside_generation_root, &generation_root).unwrap();
        b4_assert_public_update_rejected(
            &safe_source,
            &home,
            &prefix,
            &tmp,
            b"immutable generation root is not a real directory",
        );
        assert_eq!(
            std::fs::read_dir(&outside_generation_root).unwrap().count(),
            0
        );
        let state_root = home.join(".local/share/codex/core");
        assert!(!state_root.join("activation-state").exists());
        assert!(!state_root.join("activation-journal").exists());
        assert!(!state_root.join("config").exists());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_fresh_bootstrap_rejects_symlink_generation_root_with_existing_destination() {
        use std::os::unix::fs::symlink;

        let root = temp_root("m2-r1-fresh-bootstrap-generation-root");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let release = b2_write_generation(&source_roots, "rooted-g0", false, "unsupported");
        let private_key = root.join("signing-private.pem");
        let public_key = root.join("signing-public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        let core = root.join("prebuilt-codex");
        b8_write_core_probe_wrapper(&core);
        b8_bind_core_digest(&release, &openssl, &core);
        b4_write_signed_release(&release, 1, &openssl, &private_key);
        b4_install_trusted_release_key(&home, &public_key);

        let outside_generation_root = root.join("outside-generation-root");
        std::fs::create_dir(&outside_generation_root).unwrap();
        let outside_generation = outside_generation_root.join("rooted-g0");
        copy_local_directory_tree(&release, &outside_generation).unwrap();
        let descriptor_before = std::fs::read(outside_generation.join("generation.meta")).unwrap();
        let generation_root = home.join(".local/lib/codex/core/generations");
        std::fs::create_dir_all(generation_root.parent().unwrap()).unwrap();
        symlink(&outside_generation_root, &generation_root).unwrap();

        let roots = b7_public_roots(&home, &prefix);
        let pin = home.join(".local/lib/codex/core/release-public-key.pem");
        let process_env = b8_process_env(&prefix, &tmp);
        assert!(matches!(
            bootstrap_initial_signed_local_release(&release, &roots, &pin, &core, &process_env),
            Err(LocalProductError::UnsafeSource(
                "immutable generation root is not a real directory"
            ))
        ));
        assert_eq!(
            std::fs::read(outside_generation.join("generation.meta")).unwrap(),
            descriptor_before
        );
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), None);
        assert!(!paths.activation_journal.exists());
        assert!(!paths.activation_state_temp.exists());
        assert!(!roots.config_dir.exists());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_r1_public_update_reuses_generation_after_root_sync_failure() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("m2-r1-generation-retry");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let generation_root = home.join(".local/lib/codex/core/generations");
        let state_root = home.join(".local/share/codex/core");
        let trusted_public_key = home.join(".local/lib/codex/core/release-public-key.pem");
        let private_key = root.join("signing-private.pem");
        b4_generate_release_keypair(&live_openssl, &private_key, &trusted_public_key);

        let source_roots = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let first = b2_write_generation(&source_roots, "m2-r1-first", false, "unsupported");
        b4_write_signed_release(&first, 1, &live_openssl, &private_key);
        b4_assert_public_update_activated(&first, &home, &prefix, &tmp, "m2-r1-first");
        let state_paths = CoreStatePaths::new(&state_root).unwrap();
        let state_before = std::fs::read(&state_paths.activation_state).unwrap();

        let next = b2_write_generation(&source_roots, "m2-r1-next", false, "supported");
        b4_write_signed_release(&next, 2, &live_openssl, &private_key);
        let mut generation_permissions = std::fs::metadata(&generation_root).unwrap().permissions();
        generation_permissions.set_mode(0o300);
        std::fs::set_permissions(&generation_root, generation_permissions).unwrap();

        let first_failure = b4_run_public_update(&next, &home, &prefix, &tmp);
        assert_eq!(first_failure.status.code(), Some(1));
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
        assert!(generation_root.join("m2-r1-next").is_dir());

        let second_failure = b4_run_public_update(&next, &home, &prefix, &tmp);
        assert_eq!(second_failure.status.code(), Some(1));
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
        assert!(generation_root.join("m2-r1-next").is_dir());

        let mut generation_permissions = std::fs::metadata(&generation_root).unwrap().permissions();
        generation_permissions.set_mode(0o755);
        std::fs::set_permissions(&generation_root, generation_permissions).unwrap();
        b4_assert_public_update_activated(&next, &home, &prefix, &tmp, "m2-r1-next");
        let state = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state.current, "m2-r1-next");
        assert_eq!(state.previous.as_deref(), Some("m2-r1-first"));
        assert!(b3_candidate_entries(&generation_root).is_empty());
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_activation_public_initial_and_update_keep_one_previous() {
        let root = temp_root("b4-public-update");
        let live_openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &live_openssl, true);
        let generation_root = home.join(".local/lib/codex/core/generations");
        let state_root = home.join(".local/share/codex/core");
        let trusted_public_key = home.join(".local/lib/codex/core/release-public-key.pem");
        let private_key = root.join("signing-private.pem");

        b4_generate_release_keypair(&live_openssl, &private_key, &trusted_public_key);

        let source_roots = b4_source_roots(&root, &live_openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let first = b2_write_generation(&source_roots, "public-first", false, "unsupported");
        b4_write_signed_release(&first, 1, &live_openssl, &private_key);
        b4_assert_public_update_activated(&first, &home, &prefix, &tmp, "public-first");

        let state_paths = CoreStatePaths::new(&state_root).unwrap();
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(GenerationPointerState {
                update_key: release_public_key_from_pem(&live_openssl, &trusted_public_key)
                    .unwrap(),
                current: "public-first".to_string(),
                current_key: release_public_key_from_pem(&live_openssl, &trusted_public_key)
                    .unwrap(),
                previous: None,
                previous_key: None,
            })
        );
        let next = b2_write_generation(&source_roots, "public-next", false, "supported");
        b4_write_signed_release(&next, 2, &live_openssl, &private_key);
        b4_assert_public_update_activated(&next, &home, &prefix, &tmp, "public-next");
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(GenerationPointerState {
                update_key: release_public_key_from_pem(&live_openssl, &trusted_public_key)
                    .unwrap(),
                current: "public-next".to_string(),
                current_key: release_public_key_from_pem(&live_openssl, &trusted_public_key)
                    .unwrap(),
                previous: Some("public-first".to_string()),
                previous_key: Some(
                    release_public_key_from_pem(&live_openssl, &trusted_public_key).unwrap()
                ),
            })
        );
        for generation_id in ["public-first", "public-next"] {
            let staged = generation_root.join(generation_id);
            assert!(staged.join("generation.meta").is_file());
            assert!(staged.join("release.manifest").is_file());
            assert!(staged.join("release.sig").is_file());
            assert!(!staged.join("signing-private.pem").exists());
        }
        assert!(!state_paths.activation_journal.exists());
        assert!(!state_paths.activation_journal_temp.exists());
        assert!(!state_paths.activation_state_temp.exists());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_activation_rejects_non_monotonic_sequence_before_candidate_execution() {
        let root = temp_root("b4-activation-sequence");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();

        let current = b2_write_generation(&source_roots, "sequence-current", false, "unsupported");
        b4_write_signed_release(&current, 10, &openssl, &private_key);
        b4_assert_public_update_activated(&current, &home, &prefix, &tmp, "sequence-current");
        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        let state_before = std::fs::read(&state_paths.activation_state).unwrap();

        let rollback =
            b2_write_generation(&source_roots, "sequence-rollback", false, "unsupported");
        b4_write_probe_runtime(&rollback, 77, 77);
        b4_write_signed_release(&rollback, 10, &openssl, &private_key);
        b4_assert_public_update_rejected(
            &rollback,
            &home,
            &prefix,
            &tmp,
            b"release sequence is not newer than the active release",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
        assert!(!home
            .join(".local/lib/codex/core/generations/sequence-rollback")
            .exists());
        assert!(!state_paths.activation_journal.exists());
        assert!(!state_paths.activation_journal_temp.exists());
        assert!(!state_paths.activation_state_temp.exists());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_activation_rejects_current_identity_mismatch_before_staging() {
        let root = temp_root("b4-activation-current-identity");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();

        let current = b2_write_generation(&source_roots, "identity-current", false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b4_assert_public_update_activated(&current, &home, &prefix, &tmp, "identity-current");

        let generation_root = home.join(".local/lib/codex/core/generations");
        let installed_current = generation_root.join("identity-current");
        let descriptor_path = installed_current.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap().replace(
            "generation_id\tidentity-current\n",
            "generation_id\tforeign-current\n",
        );
        std::fs::write(&descriptor_path, descriptor).unwrap();
        b4_write_signed_release(&installed_current, 1, &openssl, &private_key);

        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        let state_before = std::fs::read(&state_paths.activation_state).unwrap();
        let next = b2_write_generation(&source_roots, "identity-next", false, "unsupported");
        b4_write_probe_runtime(&next, 77, 77);
        b4_write_signed_release(&next, 2, &openssl, &private_key);
        b4_assert_public_update_rejected(
            &next,
            &home,
            &prefix,
            &tmp,
            b"active generation descriptor id does not match current",
        );

        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
        assert!(!generation_root.join("identity-next").exists());
        m2_b1_assert_no_transaction_files(&state_paths);

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_rald45_transition_signal_preserves_public_doctor_semantics() {
        assert_eq!(
            public_doctor_capability(
                R10_BROWSER_HELPER_BRIDGE_METADATA,
                UpstreamDoctorCapability::Unsupported,
            ),
            UpstreamDoctorCapability::Supported
        );
        assert_eq!(
            public_doctor_capability("test-fixture", UpstreamDoctorCapability::Unsupported),
            UpstreamDoctorCapability::Unsupported
        );
        assert_eq!(
            public_doctor_capability(
                R10_BROWSER_HELPER_BRIDGE_METADATA,
                UpstreamDoctorCapability::Supported,
            ),
            UpstreamDoctorCapability::Supported
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_activation_version_probe_failure_blocks_but_doctor_health_does_not() {
        let root = temp_root("b4-activation-probe-failures");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();

        let current = b2_write_generation(&source_roots, "probe-current", false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b4_assert_public_update_activated(&current, &home, &prefix, &tmp, "probe-current");
        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        let state_before = std::fs::read(&state_paths.activation_state).unwrap();

        let version_failure =
            b2_write_generation(&source_roots, "version-failure", false, "unsupported");
        b4_write_probe_runtime(&version_failure, 9, 0);
        b4_write_signed_release(&version_failure, 2, &openssl, &private_key);
        b4_assert_public_update_rejected(
            &version_failure,
            &home,
            &prefix,
            &tmp,
            b"candidate version probe was unhealthy",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );

        let doctor_failure =
            b2_write_generation(&source_roots, "doctor-failure", false, "supported");
        b4_write_probe_runtime(&doctor_failure, 0, 9);
        b4_write_signed_release(&doctor_failure, 3, &openssl, &private_key);
        b4_assert_public_update_activated(&doctor_failure, &home, &prefix, &tmp, "doctor-failure");
        let state_after_doctor_health = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state_after_doctor_health.current, "doctor-failure");
        assert_eq!(
            state_after_doctor_health.previous.as_deref(),
            Some("probe-current")
        );
        let generation_root = home.join(".local/lib/codex/core/generations");
        for generation_id in ["version-failure", "doctor-failure"] {
            let candidate = generation_root.join(generation_id);
            assert!(candidate.is_dir());
            verify_local_release_bundle(&candidate, &openssl, &public_key).unwrap();
        }
        assert!(!state_paths.activation_journal.exists());
        assert!(!state_paths.activation_journal_temp.exists());
        assert!(!state_paths.activation_state_temp.exists());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b7_slice3_local_rotation_and_rollback_preserve_forward_authority() {
        let root = temp_root("b7-slice3-local-rotation");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let old_private = root.join("keys/old-private.pem");
        let old_public = root.join("keys/old-public.pem");
        let new_private = root.join("keys/new-private.pem");
        let new_public = root.join("keys/new-public.pem");
        b4_generate_release_keypair(&openssl, &old_private, &old_public);
        b4_generate_release_keypair(&openssl, &new_private, &new_public);
        let old_key = release_public_key_from_pem(&openssl, &old_public).unwrap();
        let new_key = release_public_key_from_pem(&openssl, &new_public).unwrap();
        assert_ne!(old_key, new_key);
        b4_install_trusted_release_key(&home, &old_public);

        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let g0 = b2_write_generation(&source_roots, "rotation-g0", false, "unsupported");
        b4_write_signed_release(&g0, 1, &openssl, &old_private);
        b7_seed_initial_release(
            &g0,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();

        let g1 = b2_write_generation(&source_roots, "rotation-g1", false, "unsupported");
        b2_add_tc2_browser_helpers(&g1);
        let g1_files = b4_exact_release_inventory(&g1, &openssl);
        b4_write_signed_release_inventory_with_authority(
            &g1,
            2,
            &openssl,
            &new_private,
            Some(&old_private),
            &g1_files,
        );
        b4_assert_public_update_activated(&g1, &home, &prefix, &tmp, "rotation-g1");

        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        let forward = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(forward.update_key, new_key);
        assert_eq!(forward.current, "rotation-g1");
        assert_eq!(forward.current_key, new_key);
        assert_eq!(forward.previous.as_deref(), Some("rotation-g0"));
        assert_eq!(forward.previous_key, Some(old_key));
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());

        b4_assert_public_rollback_activated(&home, &prefix, &tmp, "rotation-g0");
        let rolled_back = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(rolled_back.update_key, new_key);
        assert_eq!(rolled_back.current, "rotation-g0");
        assert_eq!(rolled_back.current_key, old_key);
        assert_eq!(rolled_back.previous.as_deref(), Some("rotation-g1"));
        assert_eq!(rolled_back.previous_key, Some(new_key));

        let old_authority_attempt = b2_write_generation(
            &source_roots,
            "rotation-old-authority",
            false,
            "unsupported",
        );
        b4_write_signed_release(&old_authority_attempt, 3, &openssl, &old_private);
        b4_assert_public_update_rejected(
            &old_authority_attempt,
            &home,
            &prefix,
            &tmp,
            b"release authority signature is required for key rotation",
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );
        m2_b1_assert_no_transaction_files(&state_paths);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b7_slice3_core_never_falls_back_to_bootstrap_key_without_v3_state() {
        let root = temp_root("b7-slice3-no-bootstrap-fallback");
        let openssl = b4_termux_openssl();
        let (home, prefix, _) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let release = b2_write_generation(&source_roots, "no-state-release", false, "unsupported");
        b4_write_signed_release(&release, 1, &openssl, &private_key);

        let roots = b7_public_roots(&home, &prefix);
        let process_env = capture_termux_process_env();
        assert!(matches!(
            activate_signed_local_release_outcome(&release, &roots, &process_env, None),
            Err(LocalProductError::NoCurrentGeneration)
        ));
        assert!(home
            .join(".local/lib/codex/core/release-public-key.pem")
            .is_file());
        assert!(!roots.generation_root.exists());
        let state_paths = CoreStatePaths::new(&roots.state_root).unwrap();
        assert!(read_pointer_state(&state_paths).unwrap().is_none());
        m2_b1_assert_no_transaction_files(&state_paths);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b7_slice3_remote_rotation_reuses_state_authority_and_local_activation() {
        let root = temp_root("b7-slice3-remote-rotation");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let old_private = root.join("keys/old-private.pem");
        let old_public = root.join("keys/old-public.pem");
        let new_private = root.join("keys/new-private.pem");
        let new_public = root.join("keys/new-public.pem");
        b4_generate_release_keypair(&openssl, &old_private, &old_public);
        b4_generate_release_keypair(&openssl, &new_private, &new_public);
        let old_key = release_public_key_from_pem(&openssl, &old_public).unwrap();
        let new_key = release_public_key_from_pem(&openssl, &new_public).unwrap();
        b4_install_trusted_release_key(&home, &old_public);

        let source_roots = b4_source_roots(&root.join("server"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let g0 = b2_write_generation(&source_roots, "remote-rotation-g0", false, "unsupported");
        b4_write_signed_release(&g0, 1, &openssl, &old_private);
        b7_seed_initial_release(
            &g0,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();

        let g1 = b2_write_generation(&source_roots, "remote-rotation-g1", false, "unsupported");
        b2_add_tc2_browser_helpers(&g1);
        let files = b4_exact_release_inventory(&g1, &openssl);
        b4_write_signed_release_inventory_with_authority(
            &g1,
            2,
            &openssl,
            &new_private,
            Some(&old_private),
            &files,
        );
        let base = "https://releases.example.invalid/codex/remote-rotation-g1/";
        let curl_log = root.join("curl-log");
        b5_write_release_curl(&prefix.join("bin/curl"), &curl_log, base, &g1);
        let output = b5_run_public_remote_update(base, &home, &prefix, &tmp);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );

        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        let state = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state.update_key, new_key);
        assert_eq!(state.current, "remote-rotation-g1");
        assert_eq!(state.current_key, new_key);
        assert_eq!(state.previous.as_deref(), Some("remote-rotation-g0"));
        assert_eq!(state.previous_key, Some(old_key));
        assert!(!home
            .join(".local/lib/codex/core/release-public-key.pem")
            .exists());
        b5_assert_no_acquisition(&home.join(".local/lib/codex/core/generations"));
        m2_b1_assert_no_transaction_files(&state_paths);

        let log = std::fs::read_to_string(curl_log).unwrap();
        assert!(log.contains(&format!("{base}release.manifest\n")));
        assert!(log.contains(&format!("{base}release.sig\n")));
        assert!(log.contains(&format!("{base}release-authority.sig\n")));
        assert_eq!(
            log.lines().filter(|line| *line == "CALL").count(),
            files.len() + 3
        );
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_arh1_update_hold_format_is_exact_and_unsafe_files_fail_closed() {
        let record = UpdateHoldRecord {
            generation_id: "held-generation".to_string(),
            release_sequence: 42,
        };
        let rendered = render_update_hold(&record);
        assert_eq!(
            rendered,
            "codex-update-hold-v1\ngeneration_id\theld-generation\nrelease_sequence\t42\n"
        );
        assert_eq!(parse_update_hold(rendered.as_bytes()).unwrap(), record);
        for malformed in [
            b"codex-update-hold-v1\ngeneration_id\theld-generation\nrelease_sequence\t042\n"
                .as_slice(),
            b"codex-update-hold-v1\ngeneration_id\theld-generation\nrelease_sequence\t0\n"
                .as_slice(),
            b"codex-update-hold-v1\ngeneration_id\t../escape\nrelease_sequence\t42\n".as_slice(),
            b"codex-update-hold-v1\ngeneration_id\theld-generation\nrelease_sequence\t42"
                .as_slice(),
        ] {
            assert!(
                parse_update_hold(malformed).is_err(),
                "malformed={malformed:?}"
            );
        }

        use std::os::unix::fs::symlink;
        let root = temp_root("arh1-hold-symlink");
        let openssl = b4_termux_openssl();
        let (home, prefix, _) = b4_prepare_public_environment(&root, &openssl, true);
        let roots = b7_public_roots(&home, &prefix);
        std::fs::create_dir_all(&roots.state_root).unwrap();
        let target = root.join("hold-target");
        std::fs::write(&target, rendered).unwrap();
        symlink(&target, update_hold_path(&roots)).unwrap();
        assert!(matches!(
            read_update_hold(&roots),
            Err(LocalProductError::UpdateHold(
                "update hold must be a regular file"
            ))
        ));
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_recovery_public_rollback_swaps_exact_signed_previous() {
        let root = temp_root("b4-public-rollback");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();

        let first = b2_write_generation(&source_roots, "rollback-first", false, "unsupported");
        b4_write_signed_release(&first, 1, &openssl, &private_key);
        b4_assert_public_update_activated(&first, &home, &prefix, &tmp, "rollback-first");
        let next = b2_write_generation(&source_roots, "rollback-next", false, "unsupported");
        b4_write_signed_release(&next, 2, &openssl, &private_key);
        b4_assert_public_update_activated(&next, &home, &prefix, &tmp, "rollback-next");

        b4_assert_public_rollback_activated(&home, &prefix, &tmp, "rollback-first");
        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(GenerationPointerState {
                update_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                current: "rollback-first".to_string(),
                current_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                previous: Some("rollback-next".to_string()),
                previous_key: Some(release_public_key_from_pem(&openssl, &public_key).unwrap()),
            })
        );
        let generation_root = home.join(".local/lib/codex/core/generations");
        for generation_id in ["rollback-first", "rollback-next"] {
            let (_, loaded) = verify_local_release_bundle(
                &generation_root.join(generation_id),
                &openssl,
                &public_key,
            )
            .unwrap();
            assert_eq!(loaded.generation_id, generation_id);
        }
        m2_b1_assert_no_transaction_files(&state_paths);
        let roots = b7_public_roots(&home, &prefix);
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(UpdateHoldRecord {
                generation_id: "rollback-next".to_string(),
                release_sequence: 2,
            })
        );
        assert!(matches!(
            prepare_signed_local_release_with_hold_policy(&next, &roots, UpdateHoldPolicy::Enforce),
            Err(LocalProductError::UpdateHeld {
                release_sequence: 2,
                ..
            })
        ));
        assert!(matches!(
            prepare_signed_local_release_with_hold_policy(
                &next,
                &roots,
                UpdateHoldPolicy::ForceHeld,
            ),
            Ok(PreparedSignedLocalRelease::Activation(_))
        ));

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b4_recovery_public_rollback_failures_preserve_authoritative_state() {
        assert_eq!(
            run_public_main([
                OsString::from("update"),
                OsString::from("--rollback"),
                OsString::from("unexpected"),
            ]),
            2
        );

        let root = temp_root("b4-public-rollback-failures");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();

        let first = b2_write_generation(&source_roots, "failure-first", false, "unsupported");
        b4_write_signed_release(&first, 1, &openssl, &private_key);
        b4_assert_public_update_activated(&first, &home, &prefix, &tmp, "failure-first");
        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        let initial_state = std::fs::read(&state_paths.activation_state).unwrap();
        b4_assert_public_rollback_rejected(
            &home,
            &prefix,
            &tmp,
            b"activation state has no rollback generation",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            initial_state
        );
        assert!(!home.join(".local/share/codex/core/update-hold").exists());

        let next = b2_write_generation(&source_roots, "failure-next", false, "unsupported");
        b4_write_signed_release(&next, 2, &openssl, &private_key);
        b4_assert_public_update_activated(&next, &home, &prefix, &tmp, "failure-next");
        let update_state = std::fs::read(&state_paths.activation_state).unwrap();
        let installed_first = home.join(".local/lib/codex/core/generations/failure-first");
        let descriptor_path = installed_first.join("generation.meta");
        let descriptor = std::fs::read_to_string(&descriptor_path).unwrap().replace(
            "generation_id\tfailure-first\n",
            "generation_id\tforeign-identity\n",
        );
        std::fs::write(&descriptor_path, descriptor).unwrap();
        b4_write_signed_release(&installed_first, 1, &openssl, &private_key);
        b4_assert_public_rollback_rejected(
            &home,
            &prefix,
            &tmp,
            b"rollback generation descriptor id does not match previous",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            update_state
        );
        m2_b1_assert_no_transaction_files(&state_paths);

        std::fs::remove_file(installed_first.join("release.sig")).unwrap();
        b4_assert_public_rollback_rejected(
            &home,
            &prefix,
            &tmp,
            b"inspect release signature failed",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            update_state
        );
        m2_b1_assert_no_transaction_files(&state_paths);

        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn b5_shell_quote_text(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }

    #[cfg(unix)]
    fn b5_shell_quote(value: &std::path::Path) -> String {
        b5_shell_quote_text(value.to_str().expect("B5 fixture path must be UTF-8"))
    }

    #[cfg(unix)]
    fn b5_write_fake_curl(
        path: &std::path::Path,
        log: &std::path::Path,
        body: &str,
        exit_code: i32,
    ) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let log = b5_shell_quote(log);
        let body = b5_shell_quote_text(body);
        std::fs::write(
            path,
            format!(
                r#"#!{shell}
if [ "${{HOME+x}}" = x ] || [ "${{CURL_HOME+x}}" = x ] || [ "${{HTTP_PROXY+x}}" = x ] || [ "${{HTTPS_PROXY+x}}" = x ]; then
  exit 97
fi
: > {log}
for argument in "$@"; do
  printf '%s\n' "$argument" >> {log}
done
printf '%s' {body}
exit {exit_code}
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b5_write_release_curl(
        path: &std::path::Path,
        log: &std::path::Path,
        base: &str,
        release_root: &std::path::Path,
    ) {
        b5_write_release_curl_with_fault(path, log, base, release_root, None);
    }

    #[cfg(unix)]
    fn b5_write_release_curl_with_fault(
        path: &std::path::Path,
        log: &std::path::Path,
        base: &str,
        release_root: &std::path::Path,
        fault: Option<(&str, &str, i32)>,
    ) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let cat = prefix.join("bin/cat");
        assert!(
            cat.is_file(),
            "Termux cat is required for B5 fixture transport"
        );
        let log = b5_shell_quote(log);
        let base = b5_shell_quote_text(base);
        let release_root = b5_shell_quote(release_root);
        let cat = b5_shell_quote(&cat);
        let (fault_relative, fault_body, fault_exit) = fault.unwrap_or(("", "", 0));
        let fault_relative = b5_shell_quote_text(fault_relative);
        let fault_body = b5_shell_quote_text(fault_body);
        std::fs::write(
            path,
            format!(
                r#"#!{shell}
if [ "${{HOME+x}}" = x ] || [ "${{CURL_HOME+x}}" = x ] || [ "${{HTTP_PROXY+x}}" = x ] || [ "${{HTTPS_PROXY+x}}" = x ]; then
  exit 97
fi
base={base}
release_root={release_root}
cat_path={cat}
fault_relative={fault_relative}
fault_body={fault_body}
fault_exit={fault_exit}
printf 'CALL\n' >> {log}
url=
while [ "$#" -gt 0 ]; do
  printf '%s\n' "$1" >> {log}
  if [ "$1" = "--url" ]; then
    shift
    [ "$#" -gt 0 ] || exit 98
    printf '%s\n' "$1" >> {log}
    url="$1"
  fi
  shift
done
case "$url" in
  "$base"*) relative="${{url#"$base"}}" ;;
  *) exit 99 ;;
esac
case "$relative" in
  release.manifest|release.sig|release-authority.sig|generation.meta|core|runtime|manager|helpers/*|browser/open/curl|browser/manual/curl|compat/*) ;;
  *) exit 100 ;;
esac
if [ -n "$fault_relative" ] && [ "$relative" = "$fault_relative" ]; then
  printf '%s' "$fault_body"
  exit "$fault_exit"
fi
exec "$cat_path" "$release_root/$relative"
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b5_write_channel_curl(
        path: &std::path::Path,
        log: &std::path::Path,
        index_url: &str,
        index_path: &std::path::Path,
        signature_path: &std::path::Path,
        base: &str,
        release_root: &std::path::Path,
    ) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let cat = prefix.join("bin/cat");
        assert!(
            cat.is_file(),
            "Termux cat is required for channel transport"
        );
        let log = b5_shell_quote(log);
        let signature_url = format!("{index_url}.sig");
        let index_url = b5_shell_quote_text(index_url);
        let signature_url = b5_shell_quote_text(&signature_url);
        let index_path = b5_shell_quote(index_path);
        let signature_path = b5_shell_quote(signature_path);
        let base = b5_shell_quote_text(base);
        let release_root = b5_shell_quote(release_root);
        let cat = b5_shell_quote(&cat);
        std::fs::write(
            path,
            format!(
                r#"#!{shell}
if [ "${{HOME+x}}" = x ] || [ "${{CURL_HOME+x}}" = x ] || [ "${{HTTP_PROXY+x}}" = x ] || [ "${{HTTPS_PROXY+x}}" = x ]; then
  exit 97
fi
index_url={index_url}
signature_url={signature_url}
index_path={index_path}
signature_path={signature_path}
base={base}
release_root={release_root}
cat_path={cat}
printf 'CALL\n' >> {log}
url=
while [ "$#" -gt 0 ]; do
  printf '%s\n' "$1" >> {log}
  if [ "$1" = "--url" ]; then
    shift
    [ "$#" -gt 0 ] || exit 98
    printf '%s\n' "$1" >> {log}
    url="$1"
  fi
  shift
done
case "$url" in
  "$index_url") exec "$cat_path" "$index_path" ;;
  "$signature_url") exec "$cat_path" "$signature_path" ;;
  "$base"*) relative="${{url#"$base"}}" ;;
  *) exit 99 ;;
esac
case "$relative" in
  release.manifest|release.sig|release-authority.sig|generation.meta|core|runtime|manager|helpers/*|browser/open/curl|browser/manual/curl|codex-code-mode-host|compat/*) ;;
  *) exit 100 ;;
esac
exec "$cat_path" "$release_root/$relative"
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b7_write_local_update_curl(
        path: &std::path::Path,
        log: &std::path::Path,
        index_url: &str,
        metadata_url: &str,
        metadata_path: &std::path::Path,
        archive_url: &str,
        archive_path: &std::path::Path,
    ) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let cat = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/cat");
        assert!(
            cat.is_file(),
            "Termux cat is required for R7 fixture transport"
        );
        let log = b5_shell_quote(log);
        let signature_url = format!("{index_url}.sig");
        let index_url = b5_shell_quote_text(index_url);
        let signature_url = b5_shell_quote_text(&signature_url);
        let metadata_url = b5_shell_quote_text(metadata_url);
        let metadata_path = b5_shell_quote(metadata_path);
        let archive_url = b5_shell_quote_text(archive_url);
        let archive_path = b5_shell_quote(archive_path);
        let cat = b5_shell_quote(&cat);
        let script = format!(
            concat!(
                "#!{}\n",
                "index_url={}\n",
                "signature_url={}\n",
                "metadata_url={}\n",
                "metadata_path={}\n",
                "archive_url={}\n",
                "archive_path={}\n",
                "cat_path={}\n",
                "url=\n",
                "while [ \"$#\" -gt 0 ]; do\n",
                "  if [ \"$1\" = \"--url\" ]; then\n",
                "    shift\n",
                "    [ \"$#\" -gt 0 ] || exit 98\n",
                "    url=\"$1\"\n",
                "  fi\n",
                "  shift\n",
                "done\n",
                "printf '%s\\n' \"$url\" >> {}\n",
                "case \"$url\" in\n",
                "  \"$index_url\"|\"$signature_url\") exit 7 ;;\n",
                "  \"$metadata_url\") exec \"$cat_path\" \"$metadata_path\" ;;\n",
                "  \"$archive_url\") exec \"$cat_path\" \"$archive_path\" ;;\n",
                "  *) exit 99 ;;\n",
                "esac\n"
            ),
            shell,
            index_url,
            signature_url,
            metadata_url,
            metadata_path,
            archive_url,
            archive_path,
            cat,
            log,
        );
        std::fs::write(path, script).unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn b7_write_fake_github_cli(
        path: &std::path::Path,
        log: &std::path::Path,
        body: &std::path::Path,
        exit_put: i32,
    ) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let cat = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/cat");
        let log = b5_shell_quote(log);
        let body = b5_shell_quote(body);
        let cat = b5_shell_quote(&cat);
        let script = format!(
            concat!(
                "#!{}\n",
                "cat_path={}\n",
                "printf 'CALL\\n' >> {}\n",
                "for argument in \"$@\"; do printf '%s\\n' \"$argument\" >> {}; done\n",
                "if [ \"$1\" = \"auth\" ]; then exit 0; fi\n",
                "if [ \"$1\" = \"release\" ] && [ \"$2\" = \"create\" ]; then\n",
                "  printf 'RELEASE_CREATE\\n' >> {}\n",
                "  exit 0\n",
                "fi\n",
                "method=\n",
                "endpoint=\n",
                "expect_method=\n",
                "for argument in \"$@\"; do\n",
                "  if [ \"$expect_method\" = \"1\" ]; then method=\"$argument\"; expect_method=; fi\n",
                "  if [ \"$argument\" = \"--method\" ]; then expect_method=1; fi\n",
                "  case \"$argument\" in repos/*) endpoint=\"$argument\" ;; esac\n",
                "done\n",
                "if [ \"$method\" = \"PUT\" ]; then\n",
                "  printf 'PUT %s\\n' \"$endpoint\" >> {}\n",
                "  printf 'BODY\\n' >> {}\n",
                "  \"$cat_path\" >> {}\n",
                "  printf '\\nEND\\n' >> {}\n",
                "  exit {}\n",
                "fi\n",
                "printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\n'\n",
                "exit 0\n"
            ),
            shell,
            cat,
            log,
            log,
            log,
            log,
            body,
            body,
            body,
            exit_put,
        );
        std::fs::write(path, script).unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    struct B5ChannelFixture {
        root: std::path::PathBuf,
        home: std::path::PathBuf,
        prefix: std::path::PathBuf,
        tmp: std::path::PathBuf,
        openssl: std::path::PathBuf,
        private_key: std::path::PathBuf,
        current_id: String,
        base: String,
        index_url: String,
        index_path: std::path::PathBuf,
        signature_path: std::path::PathBuf,
        curl_log: std::path::PathBuf,
    }

    #[cfg(unix)]
    fn b5_channel_fixture(label: &str, generation_id: &str) -> B5ChannelFixture {
        let root = temp_root(label);
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);

        let source_roots = b4_source_roots(&root.join("release-server"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let current_id = "channel-current".to_owned();
        let current = b2_write_generation(&source_roots, &current_id, false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b7_seed_initial_release(
            &current,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();

        let release = b2_write_root_generation(&source_roots, generation_id, false, "supported");
        b4_write_signed_release(&release, 2, &openssl, &private_key);
        let base = format!("https://releases.example.invalid/codex/{generation_id}/");
        let index_url = "https://updates.example.invalid/codex/update-index-v1".to_owned();
        let index_path = root.join("update-index-v1");
        let signature_path = root.join("update-index-v1.sig");
        std::fs::write(
            &index_path,
            format!(
                "codex-update-index-v1\nchannel\tstable\ngeneration_id\t{generation_id}\nrelease_base\t{base}\n"
            ),
        )
        .unwrap();
        b4_sign_update_index(&index_path, &signature_path, &openssl, &private_key);
        let curl_log = root.join("curl-log");
        b5_write_channel_curl(
            &prefix.join("bin/curl"),
            &curl_log,
            &index_url,
            &index_path,
            &signature_path,
            &base,
            &release,
        );
        B5ChannelFixture {
            root,
            home,
            prefix,
            tmp,
            openssl,
            private_key,
            current_id,
            base,
            index_url,
            index_path,
            signature_path,
            curl_log,
        }
    }

    #[cfg(unix)]
    fn b5_point_channel_at(
        fixture: &B5ChannelFixture,
        generation_id: &str,
        release: &std::path::Path,
    ) -> String {
        let base = format!("https://releases.example.invalid/codex/{generation_id}/");
        std::fs::write(
            &fixture.index_path,
            format!(
                "codex-update-index-v1\nchannel\tstable\ngeneration_id\t{generation_id}\nrelease_base\t{base}\n"
            ),
        )
        .unwrap();
        b4_sign_update_index(
            &fixture.index_path,
            &fixture.signature_path,
            &fixture.openssl,
            &fixture.private_key,
        );
        std::fs::write(&fixture.curl_log, b"").unwrap();
        b5_write_channel_curl(
            &fixture.prefix.join("bin/curl"),
            &fixture.curl_log,
            &fixture.index_url,
            &fixture.index_path,
            &fixture.signature_path,
            &base,
            release,
        );
        base
    }

    #[cfg(unix)]
    fn b5_assert_channel_rejected(fixture: &B5ChannelFixture, expected: &[u8]) {
        let output = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert_human_update_error_contains(&output.stderr, expected);
        let state_paths =
            CoreStatePaths::new(&fixture.home.join(".local/share/codex/core")).unwrap();
        let state = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state.current, fixture.current_id);
        assert!(state.previous.is_none());
        let generation_root = fixture.home.join(".local/lib/codex/core/generations");
        let entries: Vec<_> = std::fs::read_dir(&generation_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from(&fixture.current_id)]);
        assert!(!fixture
            .home
            .join(".local/share/codex/core/.update-index")
            .exists());
        let state_root = fixture.home.join(".local/share/codex/core");
        assert!(std::fs::read_dir(&state_root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .as_encoded_bytes()
                .starts_with(b".update-index-")
        }));
        b5_assert_no_acquisition(&generation_root);
        let calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
        assert!(calls.contains(&fixture.index_url));
        assert!(calls.contains(&format!("{}.sig", fixture.index_url)));
        assert!(!calls.contains("release.manifest"));
    }

    #[cfg(unix)]
    #[test]
    fn test_arh1_public_update_rollback_hold_and_force_flow_is_exact() {
        let fixture = b5_channel_fixture("arh1-public-flow", "arh1-next");
        let first = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(first.status.code(), Some(0), "stderr={:?}", first.stderr);

        let rollback = arh1_run_public_rollback(&fixture.home, &fixture.prefix, &fixture.tmp);
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        assert!(rollback
            .stdout
            .windows(b"Rolled back the Termux release for Codex 9.9.9. \xE2\x9C\x85\n".len())
            .any(|window| window
                == b"Rolled back the Termux release for Codex 9.9.9. \xE2\x9C\x85\n"));
        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(UpdateHoldRecord {
                generation_id: "arh1-next".to_string(),
                release_sequence: 2,
            })
        );
        let state_paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let rolled_back = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(rolled_back.current, "channel-current");
        assert_eq!(rolled_back.previous.as_deref(), Some("arh1-next"));

        let held = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(held.status.code(), Some(1), "stderr={:?}", held.stderr);
        assert_human_update_error_contains(
            &held.stderr,
            b"release sequence 2 is locally held after rollback",
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );

        std::fs::write(&fixture.curl_log, b"").unwrap();
        let forced = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            forced.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            forced.stdout,
            forced.stderr
        );
        let after_force = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(after_force.current, "arh1-next");
        assert_eq!(after_force.previous.as_deref(), Some("channel-current"));
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(UpdateHoldRecord {
                generation_id: "arh1-next".to_string(),
                release_sequence: 2,
            })
        );
        let force_calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
        assert!(force_calls.contains("release.manifest"));
        assert!(force_calls.contains("generation.meta"));
        assert!(force_calls.contains("runtime"));
        b5_assert_no_acquisition(&roots.generation_root);
        m2_b1_assert_no_transaction_files(&state_paths);
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_arh1_greater_sequence_plain_update_supersedes_and_clears_hold() {
        let fixture = b5_channel_fixture("arh1-greater-sequence", "arh1-held");
        let first = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(first.status.code(), Some(0), "stderr={:?}", first.stderr);

        let rollback = arh1_run_public_rollback(&fixture.home, &fixture.prefix, &fixture.tmp);
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stderr={:?}",
            rollback.stderr
        );
        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(UpdateHoldRecord {
                generation_id: "arh1-held".to_string(),
                release_sequence: 2,
            })
        );

        let source_roots = b4_source_roots(&fixture.root.join("release-server"), &fixture.openssl);
        let newer = b2_write_root_generation(&source_roots, "arh1-newer", false, "supported");
        b4_write_signed_release(&newer, 3, &fixture.openssl, &fixture.private_key);
        let newer_base = "https://releases.example.invalid/codex/arh1-newer/";
        std::fs::write(
            &fixture.index_path,
            format!(
                "codex-update-index-v1\nchannel\tstable\ngeneration_id\tarh1-newer\nrelease_base\t{newer_base}\n"
            ),
        )
        .unwrap();
        b4_sign_update_index(
            &fixture.index_path,
            &fixture.signature_path,
            &fixture.openssl,
            &fixture.private_key,
        );
        std::fs::write(&fixture.curl_log, b"").unwrap();
        b5_write_channel_curl(
            &fixture.prefix.join("bin/curl"),
            &fixture.curl_log,
            &fixture.index_url,
            &fixture.index_path,
            &fixture.signature_path,
            newer_base,
            &newer,
        );

        let greater = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            greater.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            greater.stdout,
            greater.stderr
        );
        let state_paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let after = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(after.current, "arh1-newer");
        assert_eq!(after.previous.as_deref(), Some("channel-current"));
        assert_eq!(read_update_hold(&roots).unwrap(), None);

        let current_again = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            current_again.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            current_again.stdout,
            current_again.stderr
        );
        assert!(current_again
            .stdout
            .windows(b"Codex 9.9.9 is already up to date. \xE2\x9C\x85\n".len())
            .any(|window| window == b"Codex 9.9.9 is already up to date. \xE2\x9C\x85\n"));
        assert_eq!(read_update_hold(&roots).unwrap(), None);
        b5_assert_no_acquisition(&roots.generation_root);
        m2_b1_assert_no_transaction_files(&state_paths);
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    fn arh1_write_hold_aware_core_fixture(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt as _;

        let shell = resolve_test_shell();
        let script = format!(
            r#"#!{}
case "${{1:-}}" in
  --version) printf '%s\n' 'codex hold-aware fixture'; exit 0 ;;
  doctor) exit 0 ;;
  update)
    if [ "${{2:-}}" = "--help" ]; then
      printf '%s\n' '{}'
      exit 0
    fi
    ;;
esac
exit 2
"#,
            shell.display(),
            rollback_guard::UPDATE_HOLD_CAPABILITY_LINE,
        );
        std::fs::write(path, script).unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_arh1_legacy_core_guard_force_and_greater_sequence_process_flow() {
        use std::os::unix::fs::PermissionsExt as _;

        let fixture = b5_channel_fixture("arh1-legacy-core-guard", "arh1-legacy-held");
        let resolver_before = std::fs::read(fixture.prefix.join("etc/resolv.conf")).unwrap();
        let source_roots = b4_source_roots(&fixture.root.join("release-server"), &fixture.openssl);
        let current_source = source_roots.generation_root.join("channel-current");
        let legacy_core = fixture.root.join("legacy-core");
        b8_write_core_probe_wrapper(&legacy_core);
        b2_attach_core(&current_source, &fixture.openssl, &legacy_core);
        b4_write_signed_release(&current_source, 1, &fixture.openssl, &fixture.private_key);
        let installed_current = fixture
            .home
            .join(".local/lib/codex/core/generations/channel-current");
        for relative in ["core", "generation.meta", "release.manifest", "release.sig"] {
            std::fs::copy(
                current_source.join(relative),
                installed_current.join(relative),
            )
            .unwrap();
        }
        std::fs::copy(&legacy_core, fixture.prefix.join("bin/codex")).unwrap();
        let mut stable_permissions = std::fs::metadata(fixture.prefix.join("bin/codex"))
            .unwrap()
            .permissions();
        stable_permissions.set_mode(0o755);
        std::fs::set_permissions(fixture.prefix.join("bin/codex"), stable_permissions).unwrap();

        let held_release = source_roots.generation_root.join("arh1-legacy-held");
        let held_core = fixture.root.join("hold-aware-core");
        arh1_write_hold_aware_core_fixture(&held_core);
        b2_attach_core(&held_release, &fixture.openssl, &held_core);
        b4_write_signed_release(&held_release, 2, &fixture.openssl, &fixture.private_key);
        b5_point_channel_at(&fixture, "arh1-legacy-held", &held_release);
        let legacy_core_digest = openssl_sha256(&fixture.openssl, &legacy_core).unwrap();
        let held_core_digest = openssl_sha256(&fixture.openssl, &held_core).unwrap();
        assert_ne!(legacy_core_digest, held_core_digest);

        let first = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(first.status.code(), Some(0), "stderr={:?}", first.stderr);
        assert_eq!(
            openssl_sha256(&fixture.openssl, &fixture.prefix.join("bin/codex")).unwrap(),
            held_core_digest
        );

        let rollback = arh1_run_public_rollback_with_capability(
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
            "0",
        );
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        let state_paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let rolled_back = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(rolled_back.current, "channel-current");
        assert_eq!(rolled_back.previous.as_deref(), Some("arh1-legacy-held"));
        let expected_hold = UpdateHoldRecord {
            generation_id: "arh1-legacy-held".to_owned(),
            release_sequence: 2,
        };
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(expected_hold.clone())
        );
        let guard = rollback_guard::read_guard(&roots).unwrap().unwrap();
        assert_eq!(guard.target_generation_id, "channel-current");
        assert_eq!(guard.held_generation_id, "arh1-legacy-held");
        assert_eq!(guard.held_release_sequence, 2);
        assert_eq!(guard.held_core_sha256, held_core_digest);
        assert_eq!(
            openssl_sha256(&fixture.openssl, &fixture.prefix.join("bin/codex")).unwrap(),
            guard.held_core_sha256
        );

        let held = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(held.status.code(), Some(1), "stderr={:?}", held.stderr);
        assert!(held
            .stderr
            .windows(b"locally held after rollback".len())
            .any(|window| { window == b"locally held after rollback" }));
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );

        let mut mismatched_guard = guard.clone();
        mismatched_guard.held_release_sequence = 3;
        rollback_guard::write_guard_locked(&roots, &mismatched_guard).unwrap();
        let mismatch = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            mismatch.status.code(),
            Some(1),
            "stderr={:?}",
            mismatch.stderr
        );
        assert_human_update_error_contains(&mismatch.stderr, b"rollback Core guard");
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );
        rollback_guard::write_guard_locked(&roots, &guard).unwrap();

        let bad_signature_release = b2_write_root_generation(
            &source_roots,
            "arh1-force-bad-signature",
            false,
            "supported",
        );
        b4_write_signed_release(
            &bad_signature_release,
            2,
            &fixture.openssl,
            &fixture.private_key,
        );
        std::fs::write(
            bad_signature_release.join("release.sig"),
            b"invalid-signature",
        )
        .unwrap();
        b5_point_channel_at(&fixture, "arh1-force-bad-signature", &bad_signature_release);
        let bad_signature = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(bad_signature.status.code(), Some(1));
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(expected_hold.clone())
        );

        let bad_digest_release =
            b2_write_root_generation(&source_roots, "arh1-force-bad-digest", false, "supported");
        b4_write_signed_release(
            &bad_digest_release,
            2,
            &fixture.openssl,
            &fixture.private_key,
        );
        b4_write_probe_runtime(&bad_digest_release, 0, 1);
        b5_point_channel_at(&fixture, "arh1-force-bad-digest", &bad_digest_release);
        let bad_digest = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(bad_digest.status.code(), Some(1));
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );

        let bad_probe_release =
            b2_write_root_generation(&source_roots, "arh1-force-bad-probe", false, "supported");
        b4_write_probe_runtime(&bad_probe_release, 1, 0);
        b4_write_signed_release(
            &bad_probe_release,
            2,
            &fixture.openssl,
            &fixture.private_key,
        );
        b5_point_channel_at(&fixture, "arh1-force-bad-probe", &bad_probe_release);
        let bad_probe = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(bad_probe.status.code(), Some(1));
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(expected_hold.clone())
        );

        let force_greater_release =
            b2_write_root_generation(&source_roots, "arh1-force-greater", false, "supported");
        b4_write_signed_release(
            &force_greater_release,
            3,
            &fixture.openssl,
            &fixture.private_key,
        );
        b5_point_channel_at(&fixture, "arh1-force-greater", &force_greater_release);
        let force_greater = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(force_greater.status.code(), Some(1));
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            rolled_back
        );
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(expected_hold.clone())
        );

        b5_point_channel_at(&fixture, "arh1-legacy-held", &held_release);
        let forced = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            forced.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            forced.stdout,
            forced.stderr
        );
        let after_force = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(after_force.current, "arh1-legacy-held");
        assert_eq!(after_force.previous.as_deref(), Some("channel-current"));
        assert_eq!(
            read_update_hold(&roots).unwrap(),
            Some(expected_hold.clone())
        );
        assert_eq!(rollback_guard::read_guard(&roots).unwrap(), None);

        let current_source = source_roots.generation_root.join("channel-current");
        b5_point_channel_at(&fixture, "channel-current", &current_source);
        let anti_rollback = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(anti_rollback.status.code(), Some(1));
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            after_force
        );
        assert_eq!(read_update_hold(&roots).unwrap(), Some(expected_hold));

        let newer = b2_write_root_generation(&source_roots, "arh1-guard-newer", false, "supported");
        b2_attach_core(&newer, &fixture.openssl, &held_core);
        b4_write_signed_release(&newer, 3, &fixture.openssl, &fixture.private_key);
        b5_point_channel_at(&fixture, "arh1-guard-newer", &newer);
        let greater = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            greater.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            greater.stdout,
            greater.stderr
        );
        let final_state = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(final_state.current, "arh1-guard-newer");
        assert_eq!(read_update_hold(&roots).unwrap(), None);
        assert_eq!(rollback_guard::read_guard(&roots).unwrap(), None);

        let current_again = b5_run_public_channel_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(current_again.status.code(), Some(0));
        let force_without_hold = arh1_run_public_channel_force_update(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(force_without_hold.status.code(), Some(1));
        assert_human_update_error_contains(
            &force_without_hold.stderr,
            b"forced update requires an active rollback hold",
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap(),
            final_state
        );
        let installed_core = fixture.prefix.join("bin/codex");
        for args in [vec!["--version"], vec!["doctor"]] {
            let status = std::process::Command::new(&installed_core)
                .args(args)
                .env("HOME", &fixture.home)
                .env("PREFIX", &fixture.prefix)
                .env("TMPDIR", &fixture.tmp)
                .env_remove("SSL_CERT_FILE")
                .env_remove("SSL_CERT_DIR")
                .status()
                .unwrap();
            assert!(status.success());
        }
        assert_eq!(
            std::fs::read(fixture.prefix.join("etc/resolv.conf")).unwrap(),
            resolver_before
        );
        b5_assert_no_acquisition(&roots.generation_root);
        m2_b1_assert_no_transaction_files(&state_paths);
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_update_spinner_frames_progress_and_cycle() {
        assert_eq!(
            (0..UPDATE_SPINNER_FRAMES.len())
                .map(update_spinner_frame)
                .collect::<Vec<_>>(),
            UPDATE_SPINNER_FRAMES.to_vec()
        );
        assert_eq!(
            update_spinner_frame(UPDATE_SPINNER_FRAMES.len()),
            UPDATE_SPINNER_FRAMES[0]
        );
        assert_eq!(update_spinner_frame_index("⠹"), 2);
        assert_eq!(update_spinner_frame_index("?"), 0);
    }

    #[cfg(unix)]
    #[test]
    fn test_ux1_channel_update_uses_one_tty_transient_line_and_clears_it() {
        let fixture = b5_channel_fixture("ux1-channel-tty", "ux1-channel-next");
        let output = b5_run_public_channel_update_tty(
            &fixture.index_url,
            &fixture.home,
            &fixture.prefix,
            &fixture.tmp,
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let terminal = String::from_utf8(output.stdout).unwrap();
        let checking = terminal.find("\r\x1b[2K⠋ Checking for updates...").unwrap();
        assert!(
            terminal.matches(" Checking for updates...").count() >= 2,
            "checking spinner did not advance: {terminal:?}"
        );
        let header = terminal
            .find("Updating the Termux release for Codex 9.9.9...")
            .unwrap();
        let downloading = terminal
            .find("\r\x1b[2K⠙ Downloading signed Termux release...")
            .unwrap();
        let verifying = terminal
            .find("\r\x1b[2K⠹ Verifying release signature and contents...")
            .unwrap();
        let probing = terminal
            .find("\r\x1b[2K⠹ Checking candidate runtime...")
            .unwrap();
        let activating = terminal
            .find("\r\x1b[2K⠸ Activating Codex 9.9.9...")
            .unwrap();
        assert!(checking < header);
        assert!(header < downloading);
        assert!(downloading < verifying);
        assert!(verifying < probing);
        assert!(probing < activating);
        assert!(terminal.contains("Verified and activated the signed Termux release."));
        assert!(terminal.contains("Codex 9.9.9 is now active. ✅"));
        assert!(terminal.matches("\x1b[2K").count() >= 6);
        assert!(
            output.stderr.is_empty(),
            "script stderr={:?}",
            output.stderr
        );
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r4_bare_update_uses_only_signed_wrapper_channel() {
        {
            let fixture = b5_channel_fixture("r4-channel-happy", "channel-next");
            let output = b5_run_public_channel_update(
                &fixture.index_url,
                &fixture.home,
                &fixture.prefix,
                &fixture.tmp,
            );
            assert_eq!(
                output.status.code(),
                Some(0),
                "stdout={:?} stderr={:?}",
                output.stdout,
                output.stderr
            );
            assert!(output
                .stdout
                .windows(b"Codex 9.9.9 is now active. \xE2\x9C\x85\n".len())
                .any(|window| window == b"Codex 9.9.9 is now active. \xE2\x9C\x85\n"));
            assert!(output.stderr.is_empty(), "stderr={:?}", output.stderr);
            assert!(!output.stdout.contains(&b'\r'));
            assert!(!output.stdout.windows(2).any(|window| window == b"\x1b["));

            let state_paths =
                CoreStatePaths::new(&fixture.home.join(".local/share/codex/core")).unwrap();
            let state = read_pointer_state(&state_paths).unwrap().unwrap();
            let release_key = b4_public_key_from_private(&fixture.openssl, &fixture.private_key);
            assert_eq!(state.current, "channel-next");
            assert_eq!(state.previous.as_deref(), Some("channel-current"));
            assert_eq!(state.update_key, release_key);
            let generation_root = fixture.home.join(".local/lib/codex/core/generations");
            b5_assert_no_acquisition(&generation_root);
            let installed = generation_root.join("channel-next");
            let loaded = load_local_generation(&installed).unwrap();
            assert_eq!(loaded.generation_layout, GenerationLayout::RootCodeModeHost);
            assert!(installed.join(CODE_MODE_HOST_FILE).is_file());
            assert!(!fixture
                .home
                .join(".local/share/codex/core/.update-index")
                .exists());
            let state_root = fixture.home.join(".local/share/codex/core");
            assert!(std::fs::read_dir(&state_root).unwrap().all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .as_encoded_bytes()
                    .starts_with(b".update-index-")
            }));
            let calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
            assert!(calls.contains(&fixture.index_url));
            assert!(calls.contains(&format!("{}.sig", fixture.index_url)));
            assert!(calls.contains("release.manifest"));
            assert!(!calls.contains("codex update"));

            let state_before = std::fs::read(&state_paths.activation_state).unwrap();
            std::fs::write(&fixture.curl_log, b"").unwrap();
            let current_again = b5_run_public_channel_update(
                &fixture.index_url,
                &fixture.home,
                &fixture.prefix,
                &fixture.tmp,
            );
            assert_eq!(
                current_again.status.code(),
                Some(0),
                "stdout={:?} stderr={:?}",
                current_again.stdout,
                current_again.stderr
            );
            assert!(current_again
                .stdout
                .windows(
                    b"Codex 9.9.9 is already up to date. \xE2\x9C\x85
"
                    .len()
                )
                .any(|window| {
                    window
                        == b"Codex 9.9.9 is already up to date. \xE2\x9C\x85
"
                }));
            assert!(
                current_again.stderr.is_empty(),
                "stderr={:?}",
                current_again.stderr
            );
            assert_eq!(
                std::fs::read(&state_paths.activation_state).unwrap(),
                state_before
            );
            assert_eq!(read_pointer_state(&state_paths).unwrap().unwrap(), state);
            let exact_current_calls = std::fs::read_to_string(&fixture.curl_log).unwrap();
            assert!(exact_current_calls.contains(&fixture.index_url));
            assert!(exact_current_calls.contains(&format!("{}.sig", fixture.index_url)));
            for forbidden in [
                "release.manifest",
                "release.sig",
                "release-authority.sig",
                "generation.meta",
                "codex-code-mode-host",
                "core",
                "runtime",
                "manager",
                "helpers/",
            ] {
                assert!(
                    !exact_current_calls.contains(forbidden),
                    "exact-current path unexpectedly fetched {forbidden}: {exact_current_calls}"
                );
            }
            assert_eq!(exact_current_calls.matches("CALL\n").count(), 2);
            b5_assert_no_acquisition(&generation_root);
            m2_b1_assert_no_transaction_files(&state_paths);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_channel_fixture("r4-channel-bad-signature", "channel-bad");
            std::fs::write(&fixture.signature_path, b"not-a-signature").unwrap();
            b5_assert_channel_rejected(
                &fixture,
                b"Release signature verification failed. \xE2\x9D\x8C",
            );
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_channel_fixture("r4-channel-malformed", "channel-malformed");
            let malformed = format!(
                "codex-update-index-v1\nchannel\tstable\ngeneration_id\tchannel-malformed\nrelease_base\t{}",
                fixture.base
            );
            std::fs::write(&fixture.index_path, malformed).unwrap();
            b4_sign_update_index(
                &fixture.index_path,
                &fixture.signature_path,
                &fixture.openssl,
                &fixture.private_key,
            );
            b5_assert_channel_rejected(
                &fixture,
                b"Update index is missing its final newline. \xE2\x9D\x8C",
            );
            remove_temp_root(fixture.root);
        }
    }

    #[cfg(unix)]
    fn b5_write_cleanup_blocking_curl(path: &std::path::Path, generation_root: &std::path::Path) {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let shell = resolve_test_shell();
        let shell = std::str::from_utf8(shell.as_bytes()).expect("test shell path must be UTF-8");
        let prefix = std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap());
        let chmod = prefix.join("bin/chmod");
        assert!(
            chmod.is_file(),
            "Termux chmod is required for B5 cleanup proof"
        );
        let generation_root = b5_shell_quote(generation_root);
        let chmod = b5_shell_quote(&chmod);
        std::fs::write(
            path,
            format!(
                r#"#!{shell}
if [ "${{HOME+x}}" = x ] || [ "${{HTTP_PROXY+x}}" = x ] || [ "${{HTTPS_PROXY+x}}" = x ]; then
  exit 97
fi
printf 'manifest'
for acquisition in {generation_root}/.acquire-*; do
  [ -d "$acquisition" ] || continue
  {chmod} 000 "$acquisition" || exit 98
done
exit 0
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    struct B5RemoteFixture {
        root: std::path::PathBuf,
        home: std::path::PathBuf,
        prefix: std::path::PathBuf,
        tmp: std::path::PathBuf,
        openssl: std::path::PathBuf,
        private_key: std::path::PathBuf,
        release: std::path::PathBuf,
        base: String,
        curl_log: std::path::PathBuf,
    }

    #[cfg(unix)]
    fn b5_remote_fixture(label: &str, generation_id: &str) -> B5RemoteFixture {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root(label);
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);

        let source_roots = b4_source_roots(&root.join("release-server"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let current = b2_write_generation(&source_roots, "fixture-current", false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b7_seed_initial_release(
            &current,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();
        let release = b2_write_generation(&source_roots, generation_id, false, "unsupported");
        let core = root.join("core-entrypoint");
        b8_write_core_probe_wrapper(&core);
        b2_attach_core(&release, &openssl, &core);
        b4_write_signed_release(&release, 2, &openssl, &private_key);

        let stable_entrypoint = prefix.join("bin/codex");
        std::fs::copy(std::env::current_exe().unwrap(), &stable_entrypoint).unwrap();
        let mut stable_mode = std::fs::metadata(&stable_entrypoint).unwrap().permissions();
        stable_mode.set_mode(0o755);
        std::fs::set_permissions(&stable_entrypoint, stable_mode).unwrap();

        let base = format!("https://releases.example.invalid/codex/{generation_id}/");
        let curl_log = root.join("curl-log");
        b5_write_release_curl(&prefix.join("bin/curl"), &curl_log, &base, &release);
        B5RemoteFixture {
            root,
            home,
            prefix,
            tmp,
            openssl,
            private_key,
            release,
            base,
            curl_log,
        }
    }

    #[cfg(unix)]
    fn b5_assert_public_remote_rejected(fixture: &B5RemoteFixture, base: &str, expected: &[u8]) {
        let output =
            b5_run_public_remote_update(base, &fixture.home, &fixture.prefix, &fixture.tmp);
        assert_eq!(
            output.status.code(),
            Some(1),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        assert_human_update_error_contains(&output.stderr, expected);
    }

    #[cfg(unix)]
    fn b5_assert_no_remote_generation_or_state(home: &std::path::Path) {
        let generation_root = home.join(".local/lib/codex/core/generations");
        let entries: Vec<_> = std::fs::read_dir(&generation_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("fixture-current")]);
        let state_root = home.join(".local/share/codex/core");
        let state_paths = CoreStatePaths::new(&state_root).unwrap();
        let state = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state.current, "fixture-current");
        assert!(state.previous.is_none());
        m2_b1_assert_no_transaction_files(&state_paths);
        assert!(state_root.join("config").is_dir());
    }

    #[cfg(unix)]
    fn b5_assert_no_acquisition(generation_root: &std::path::Path) {
        assert!(std::fs::read_dir(generation_root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .as_encoded_bytes()
                .starts_with(b".acquire-")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_remote_public_happy_path_reuses_b4_and_restores_signed_modes() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b5-remote-public-happy");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);

        let source_roots = b4_source_roots(&root.join("release-server"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let current = b2_write_generation(&source_roots, "remote-bootstrap", false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b7_seed_initial_release(
            &current,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();
        let release = b2_write_generation(&source_roots, "remote-initial", true, "supported");
        let core = root.join("core-entrypoint");
        b8_write_core_probe_wrapper(&core);
        b2_attach_core(&release, &openssl, &core);
        let stable_entrypoint = prefix.join("bin/codex");
        std::fs::copy(std::env::current_exe().unwrap(), &stable_entrypoint).unwrap();
        let mut stable_mode = std::fs::metadata(&stable_entrypoint).unwrap().permissions();
        stable_mode.set_mode(0o755);
        std::fs::set_permissions(&stable_entrypoint, stable_mode).unwrap();
        let stable_before_digest = openssl_sha256(&openssl, &stable_entrypoint).unwrap();
        b2_add_tc2_browser_helpers(&release);
        b2_add_helper(&release);
        let mut helper_mode = std::fs::metadata(release.join("helpers/2"))
            .unwrap()
            .permissions();
        helper_mode.set_mode(0o710);
        std::fs::set_permissions(release.join("helpers/2"), helper_mode).unwrap();
        std::fs::create_dir(release.join("compat/nested")).unwrap();
        std::fs::write(release.join("compat/nested/data"), b"compat-data").unwrap();
        let mut data_mode = std::fs::metadata(release.join("compat/nested/data"))
            .unwrap()
            .permissions();
        data_mode.set_mode(0o640);
        std::fs::set_permissions(release.join("compat/nested/data"), data_mode).unwrap();
        let encoded_asset = release.join("compat/space name/é?%#/asset");
        std::fs::create_dir_all(encoded_asset.parent().unwrap()).unwrap();
        std::fs::write(&encoded_asset, b"encoded-compat-data").unwrap();
        let mut encoded_mode = std::fs::metadata(&encoded_asset).unwrap().permissions();
        encoded_mode.set_mode(0o600);
        std::fs::set_permissions(&encoded_asset, encoded_mode).unwrap();
        b4_write_signed_release(&release, 2, &openssl, &private_key);

        let encoded_server_asset = release.join("compat/space%20name/%C3%A9%3F%25%23/asset");
        std::fs::create_dir_all(encoded_server_asset.parent().unwrap()).unwrap();
        std::fs::copy(&encoded_asset, &encoded_server_asset).unwrap();
        std::fs::write(release.join("compat/server-only"), b"unsigned-server-file").unwrap();

        let base = "https://releases.example.invalid/codex/remote-initial/";
        let curl_log = root.join("curl-log");
        b5_write_release_curl(&prefix.join("bin/curl"), &curl_log, base, &release);
        let output = b5_run_public_remote_update(base, &home, &prefix, &tmp);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let expected = b"Codex 9.9.9 is now active. \xE2\x9C\x85\n";
        assert!(
            output
                .stdout
                .windows(expected.len())
                .any(|window| window == expected),
            "stdout={:?}",
            output.stdout
        );

        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(GenerationPointerState {
                update_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                current: "remote-initial".to_string(),
                current_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                previous: Some("remote-bootstrap".to_string()),
                previous_key: Some(release_public_key_from_pem(&openssl, &public_key).unwrap()),
            })
        );
        let generation_root = home.join(".local/lib/codex/core/generations");
        let installed = generation_root.join("remote-initial");
        let (manifest, loaded) =
            verify_local_release_bundle(&installed, &openssl, &public_key).unwrap();
        assert_eq!(loaded.generation_id, "remote-initial");
        assert_eq!(
            std::fs::read(&stable_entrypoint).unwrap(),
            std::fs::read(&core).unwrap()
        );
        let rollback_record =
            read_core_entrypoint_rollback(&b7_public_roots(&home, &prefix)).unwrap();
        assert_eq!(rollback_record.generation_id, "remote-bootstrap");
        assert_eq!(rollback_record.digest, stable_before_digest);
        for file in &manifest.files {
            assert_eq!(
                std::fs::symlink_metadata(installed.join(&file.relative_path))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                file.mode,
                "{}",
                file.relative_path
            );
        }
        b5_assert_no_acquisition(&generation_root);
        m2_b1_assert_no_transaction_files(&state_paths);
        let curl_log = std::fs::read_to_string(curl_log).unwrap();
        assert_eq!(
            curl_log.lines().filter(|line| *line == "CALL").count(),
            manifest.files.len() + 2
        );
        assert!(curl_log.contains(&format!("{base}release.manifest\n")));
        assert!(curl_log.contains(&format!("{base}compat/nested/data\n")));
        assert!(curl_log.contains(&format!(
            "{base}compat/space%20name/%C3%A9%3F%25%23/asset\n"
        )));
        assert!(!curl_log.contains(&format!("{base}compat/server-only\n")));
        assert!(curl_log.lines().any(|line| line == "--location"));
        assert!(curl_log
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|window| window == ["--proto-redir", "=https"]));
        assert_eq!(
            std::fs::read(installed.join("compat/space name/é?%#/asset")).unwrap(),
            b"encoded-compat-data"
        );
        assert!(!installed.join("compat/server-only").exists());
        assert!(!installed.join("compat/space%20name").exists());

        let rollback = b4_run_public_rollback(&home, &prefix, &tmp);
        assert_eq!(
            rollback.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            rollback.stdout,
            rollback.stderr
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap().current,
            "remote-bootstrap"
        );
        assert_eq!(
            std::fs::read(&stable_entrypoint).unwrap(),
            std::fs::read(&core).unwrap()
        );
        assert!(rollback_guard::read_guard(&b7_public_roots(&home, &prefix))
            .unwrap()
            .is_some());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r10_coordinated_activation_restores_entrypoint_on_state_boundary_failure() {
        let fixture = b5_remote_fixture("r10-coordinated-state-failure", "r10-candidate");
        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        let state_paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let before_entrypoint = std::fs::read(fixture.prefix.join("bin/codex")).unwrap();
        let prepared = match prepare_signed_local_release(&fixture.release, &roots).unwrap() {
            PreparedSignedLocalRelease::Activation(prepared) => *prepared,
            PreparedSignedLocalRelease::AlreadyCurrent(_) => {
                panic!("forward candidate unexpectedly classified as already current")
            }
        };
        std::fs::write(&state_paths.activation_state_temp, b"orphan").unwrap();

        let error = activate_prepared_local_release(
            prepared,
            &roots,
            &b8_process_env(&fixture.prefix, &fixture.tmp),
            None,
        )
        .expect_err("orphan state temporary must reject coordinated activation");
        assert!(matches!(
            error,
            LocalProductError::State(
                m2_generation_state::ActivationTransactionError::OrphanTemporaryState
            )
        ));
        assert_eq!(
            std::fs::read(fixture.prefix.join("bin/codex")).unwrap(),
            before_entrypoint
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap().unwrap().current,
            "fixture-current"
        );
        std::fs::remove_file(&state_paths.activation_state_temp).unwrap();
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_r10_new_manager_update_requires_coordinated_core_asset() {
        let fixture = b5_remote_fixture("r10-manager-without-core", "r10-unused");
        let source_roots = b4_source_roots(
            &fixture.root.join("legacy-manager-source"),
            &fixture.openssl,
        );
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let legacy_manager =
            b2_write_generation(&source_roots, "r10-legacy-manager", true, "supported");
        b4_write_signed_release(&legacy_manager, 2, &fixture.openssl, &fixture.private_key);
        let roots = b7_public_roots(&fixture.home, &fixture.prefix);
        let error = prepare_signed_local_release(&legacy_manager, &roots)
            .expect_err("new Manager-bearing v3 bundle must be rejected");
        assert!(matches!(
            error,
            LocalProductError::ReleasePolicy(
                "new Manager-bearing releases require a coordinated Core artifact"
            )
        ));
        assert!(!roots.generation_root.join("r10-legacy-manager").exists());
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_remote_public_control_and_transport_failures_preserve_state() {
        {
            let fixture = b5_remote_fixture("b5-invalid-url", "invalid-url");
            b5_assert_public_remote_rejected(
                &fixture,
                "http://releases.example.invalid/codex/invalid-url/",
                b"remote release base URL must use HTTPS",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-missing-curl", "missing-curl");
            std::fs::remove_file(fixture.prefix.join("bin/curl")).unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"Termux curl is unavailable",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-control-transport", "control-transport");
            b5_write_fake_curl(
                &fixture.prefix.join("bin/curl"),
                &fixture.curl_log,
                "partial",
                22,
            );
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote release transport failed",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-missing-signature", "missing-signature");
            std::fs::remove_file(fixture.release.join("release.sig")).unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote release transport failed",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-control-oversize", "control-oversize");
            let oversized = "x".repeat(LOCAL_RELEASE_MAX_BYTES + 1);
            b5_write_fake_curl(
                &fixture.prefix.join("bin/curl"),
                &fixture.curl_log,
                &oversized,
                0,
            );
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote release response exceeds its byte bound",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-signature-oversize", "signature-oversize");
            std::fs::write(
                fixture.release.join("release.sig"),
                vec![b'x'; LOCAL_RELEASE_SIGNATURE_MAX_BYTES as usize + 1],
            )
            .unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote release response exceeds its byte bound",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-malformed-control", "malformed-control");
            std::fs::write(fixture.release.join("release.manifest"), b"not-a-release\n").unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"release manifest format is unsupported",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-wrong-key", "wrong-key");
            let wrong_private = fixture.root.join("wrong/private.pem");
            let wrong_public = fixture.root.join("wrong/public.pem");
            b4_generate_release_keypair(&fixture.openssl, &wrong_private, &wrong_public);
            let files = b4_exact_release_inventory(&fixture.release, &fixture.openssl);
            b4_write_signed_release_inventory_with_authority(
                &fixture.release,
                2,
                &fixture.openssl,
                &wrong_private,
                Some(&wrong_private),
                &files,
            );
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"release signature verification failed",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-bad-signature", "bad-signature");
            std::fs::write(fixture.release.join("release.sig"), b"rejected-signature").unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"release signature verification failed",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-policy", "policy");
            let manifest_path = fixture.release.join("release.manifest");
            let manifest = std::fs::read_to_string(&manifest_path)
                .unwrap()
                .replace("channel\tstable\n", "channel\tunsupported\n");
            std::fs::write(&manifest_path, manifest).unwrap();
            b4_sign_release_manifest(&fixture.release, &fixture.openssl, &fixture.private_key);
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"release channel is not supported",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-base-identity", "signed-identity");
            let mismatched = "https://releases.example.invalid/codex/other-identity/";
            b5_write_release_curl(
                &fixture.prefix.join("bin/curl"),
                &fixture.curl_log,
                mismatched,
                &fixture.release,
            );
            b5_assert_public_remote_rejected(
                &fixture,
                mismatched,
                b"remote release base does not match signed generation identity",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_remote_cleanup_failure_is_terminal_and_nonactivatable() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = b5_remote_fixture("b5-cleanup-failure", "cleanup-failure");
        let generation_root = fixture.home.join(".local/lib/codex/core/generations");
        b5_write_cleanup_blocking_curl(&fixture.prefix.join("bin/curl"), &generation_root);
        b5_assert_public_remote_rejected(
            &fixture,
            &fixture.base,
            b"remove remote acquisition directory failed",
        );

        let entries: Vec<_> = std::fs::read_dir(&generation_root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|path| path.ends_with("fixture-current")));
        let acquisition = entries
            .into_iter()
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .as_encoded_bytes()
                    .starts_with(b".acquire-")
            })
            .expect("cleanup failure must leave only the private acquisition plus current");
        let state_root = fixture.home.join(".local/share/codex/core");
        let state_paths = CoreStatePaths::new(&state_root).unwrap();
        let state = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state.current, "fixture-current");
        assert!(state.previous.is_none());
        assert!(!state_root.join("activation-journal").exists());

        let mut permissions = std::fs::symlink_metadata(&acquisition)
            .unwrap()
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&acquisition, permissions).unwrap();
        std::fs::remove_dir_all(&acquisition).unwrap();
        remove_temp_root(fixture.root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_remote_public_content_failures_are_clean_and_nonactivating() {
        {
            let fixture = b5_remote_fixture("b5-missing-content", "missing-content");
            std::fs::remove_file(fixture.release.join("runtime")).unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote release transport failed",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-interrupted-content", "interrupted-content");
            b5_write_release_curl_with_fault(
                &fixture.prefix.join("bin/curl"),
                &fixture.curl_log,
                &fixture.base,
                &fixture.release,
                Some(("runtime", "partial-runtime", 22)),
            );
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote release transport failed",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-content-digest", "content-digest");
            std::fs::write(fixture.release.join("runtime"), b"tampered-runtime").unwrap();
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"release file inventory digest mismatch",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-unsafe-mode", "unsafe-mode");
            let manifest_path = fixture.release.join("release.manifest");
            let manifest = std::fs::read_to_string(&manifest_path).unwrap();
            let runtime_line = manifest
                .lines()
                .find(|line| line.starts_with("file\truntime\t"))
                .unwrap();
            let (runtime_fields, _) = runtime_line.rsplit_once('\t').unwrap();
            let manifest = manifest.replacen(runtime_line, &format!("{runtime_fields}\t0644"), 1);
            std::fs::write(&manifest_path, manifest).unwrap();
            b4_sign_release_manifest(&fixture.release, &fixture.openssl, &fixture.private_key);
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"release executable file is not owner-executable",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }

        {
            let fixture = b5_remote_fixture("b5-output-collision", "output-collision");
            let conflict = fixture.release.join("compat/conflict");
            std::fs::write(&conflict, b"conflict-file").unwrap();
            let mut files = b4_exact_release_inventory(&fixture.release, &fixture.openssl);
            files.push(ReleaseFileEntry {
                relative_path: "compat/conflict/nested".to_string(),
                sha256: "0".repeat(64),
                mode: 0o644,
            });
            files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
            b4_write_signed_release_inventory(
                &fixture.release,
                1,
                &fixture.openssl,
                &fixture.private_key,
                &files,
            );
            b5_assert_public_remote_rejected(
                &fixture,
                &fixture.base,
                b"remote acquisition path contains a non-directory",
            );
            b5_assert_no_remote_generation_or_state(&fixture.home);
            remove_temp_root(fixture.root);
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_content_response_and_aggregate_bounds_use_the_same_fetch_path() {
        let (root, mut roots) = b2_test_roots("b5-content-bounds");
        roots.curl = root.join("bin/curl");
        let log = root.join("curl-arguments");
        let base = RemoteReleaseBase::parse(OsStr::new(
            "https://releases.example.invalid/codex/content-bounds/",
        ))
        .unwrap();

        let first_acquisition = root.join(".acquire-first");
        create_remote_acquisition_root(&first_acquisition).unwrap();
        b5_write_fake_curl(&roots.curl, &log, "12345", 0);
        let mut acquired = 0;
        assert!(matches!(
            fetch_remote_resource(
                &roots,
                &base,
                "runtime",
                &first_acquisition.join("runtime"),
                4,
                &mut acquired,
            ),
            Err(LocalProductError::RemoteResponseTooLarge)
        ));
        assert_eq!(acquired, 0);
        assert_eq!(
            std::fs::metadata(first_acquisition.join("runtime"))
                .unwrap()
                .len(),
            5
        );
        std::fs::remove_dir_all(&first_acquisition).unwrap();

        let aggregate_acquisition = root.join(".acquire-aggregate");
        create_remote_acquisition_root(&aggregate_acquisition).unwrap();
        b5_write_fake_curl(&roots.curl, &log, "abc", 0);
        let mut acquired = REMOTE_RELEASE_TOTAL_MAX_BYTES - 2;
        assert!(matches!(
            fetch_remote_resource(
                &roots,
                &base,
                "runtime",
                &aggregate_acquisition.join("runtime"),
                REMOTE_RELEASE_FILE_MAX_BYTES,
                &mut acquired,
            ),
            Err(LocalProductError::RemoteResponseTooLarge)
        ));
        assert_eq!(acquired, REMOTE_RELEASE_TOTAL_MAX_BYTES - 2);
        let arguments: Vec<_> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["--max-filesize", "2"]));

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_remote_public_forward_antirollback_probe_and_rollback_integration() {
        let root = temp_root("b5-update-integration");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);

        let source_roots = b4_source_roots(&root.join("release-server"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let first = b2_write_generation(&source_roots, "local-first", false, "supported");
        b4_write_probe_runtime(&first, 0, 0);
        b4_write_signed_release(&first, 1, &openssl, &private_key);
        b4_assert_public_update_activated(&first, &home, &prefix, &tmp, "local-first");

        let state_paths = CoreStatePaths::new(&home.join(".local/share/codex/core")).unwrap();
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(GenerationPointerState {
                update_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                current: "local-first".to_string(),
                current_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                previous: None,
                previous_key: None,
            })
        );

        let remote_second = b2_write_generation(&source_roots, "remote-second", false, "supported");
        b4_write_probe_runtime(&remote_second, 0, 0);
        b4_write_signed_release(&remote_second, 2, &openssl, &private_key);
        let second_base = "https://releases.example.invalid/codex/remote-second/";
        let curl_log = root.join("curl-log");
        b5_write_release_curl(
            &prefix.join("bin/curl"),
            &curl_log,
            second_base,
            &remote_second,
        );
        let second_output = b5_run_public_remote_update(second_base, &home, &prefix, &tmp);
        assert_eq!(
            second_output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            second_output.stdout,
            second_output.stderr
        );
        assert!(second_output
            .stdout
            .windows(b"Codex 9.9.9 is now active. \xE2\x9C\x85\n".len())
            .any(|window| window == b"Codex 9.9.9 is now active. \xE2\x9C\x85\n"));
        let expected_forward = GenerationPointerState {
            update_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
            current: "remote-second".to_string(),
            current_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
            previous: Some("local-first".to_string()),
            previous_key: Some(release_public_key_from_pem(&openssl, &public_key).unwrap()),
        };
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(expected_forward.clone())
        );

        let state_before_current = std::fs::read(&state_paths.activation_state).unwrap();
        let current_again = b5_run_public_remote_update(second_base, &home, &prefix, &tmp);
        assert_eq!(
            current_again.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            current_again.stdout,
            current_again.stderr
        );
        assert!(current_again
            .stdout
            .windows(
                b"Codex 9.9.9 is already up to date. \xE2\x9C\x85
"
                .len()
            )
            .any(|window| {
                window
                    == b"Codex 9.9.9 is already up to date. \xE2\x9C\x85
"
            }));
        assert!(
            current_again.stderr.is_empty(),
            "stderr={:?}",
            current_again.stderr
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before_current
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(expected_forward.clone())
        );

        let not_newer = b2_write_generation(&source_roots, "remote-not-newer", false, "supported");
        b4_write_probe_runtime(&not_newer, 0, 0);
        b4_write_signed_release(&not_newer, 2, &openssl, &private_key);
        let not_newer_base = "https://releases.example.invalid/codex/remote-not-newer/";
        b5_write_release_curl(
            &prefix.join("bin/curl"),
            &curl_log,
            not_newer_base,
            &not_newer,
        );
        let not_newer_output = b5_run_public_remote_update(not_newer_base, &home, &prefix, &tmp);
        assert_eq!(not_newer_output.status.code(), Some(1));
        assert_human_update_error_contains(
            &not_newer_output.stderr,
            b"release sequence is not newer than the active release",
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(expected_forward.clone())
        );

        let probe_failure =
            b2_write_generation(&source_roots, "remote-probe-failure", false, "supported");
        b4_write_probe_runtime(&probe_failure, 17, 0);
        b4_write_signed_release(&probe_failure, 3, &openssl, &private_key);
        let probe_base = "https://releases.example.invalid/codex/remote-probe-failure/";
        b5_write_release_curl(
            &prefix.join("bin/curl"),
            &curl_log,
            probe_base,
            &probe_failure,
        );
        let probe_output = b5_run_public_remote_update(probe_base, &home, &prefix, &tmp);
        assert_eq!(probe_output.status.code(), Some(1));
        assert_human_update_error_contains(
            &probe_output.stderr,
            b"candidate version probe was unhealthy",
        );
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(expected_forward.clone())
        );

        let generation_root = home.join(".local/lib/codex/core/generations");
        assert!(!generation_root.join("remote-not-newer").exists());
        let inactive = generation_root.join("remote-probe-failure");
        let (inactive_release, inactive_loaded) =
            verify_local_release_bundle(&inactive, &openssl, &public_key).unwrap();
        assert_eq!(inactive_release.release_sequence, 3);
        assert_eq!(inactive_loaded.generation_id, "remote-probe-failure");
        b5_assert_no_acquisition(&generation_root);
        m2_b1_assert_no_transaction_files(&state_paths);

        b4_assert_public_rollback_activated(&home, &prefix, &tmp, "local-first");
        assert_eq!(
            read_pointer_state(&state_paths).unwrap(),
            Some(GenerationPointerState {
                update_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                current: "local-first".to_string(),
                current_key: release_public_key_from_pem(&openssl, &public_key).unwrap(),
                previous: Some("remote-second".to_string()),
                previous_key: Some(release_public_key_from_pem(&openssl, &public_key).unwrap()),
            })
        );
        b5_assert_no_acquisition(&generation_root);
        m2_b1_assert_no_transaction_files(&state_paths);

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_transport_url_contract_is_canonical_and_bounded() {
        use std::os::unix::ffi::OsStringExt;

        let base = RemoteReleaseBase::parse(OsStr::new(
            "https://releases.example.invalid/codex/generation-1/",
        ))
        .unwrap();
        assert!(base.matches_generation_identity("generation-1").unwrap());
        assert!(!base.matches_generation_identity("generation-2").unwrap());
        assert_eq!(
            base.resource_url("release.manifest").unwrap(),
            "https://releases.example.invalid/codex/generation-1/release.manifest"
        );
        assert_eq!(
            base.resource_url("compat/space name/é?%#").unwrap(),
            "https://releases.example.invalid/codex/generation-1/compat/space%20name/%C3%A9%3F%25%23"
        );
        assert!(matches!(
            base.resource_url("../escape"),
            Err(LocalProductError::Remote(
                "remote release resource path is invalid"
            ))
        ));

        let unicode =
            RemoteReleaseBase::parse(OsStr::new("https://example.invalid/releases/g%C3%A9n/"))
                .unwrap();
        assert!(unicode.matches_generation_identity("gén").unwrap());

        for invalid in [
            "",
            "http://example.invalid/releases/g/",
            "HTTPS://example.invalid/releases/g/",
            "https://example.invalid/",
            "https://user@example.invalid/releases/g/",
            "https://example.invalid/releases/g/?query",
            "https://example.invalid/releases/g/#fragment",
            "https://example.invalid/releases//g/",
            "https://example.invalid/releases/../g/",
            "https://example.invalid/releases/%67/",
            "https://example.invalid/releases/%2f/",
            "https://example.invalid/releases/%GG/",
            "https://bad_host.invalid/releases/g/",
            "https://example.invalid:0/releases/g/",
            "https://example.invalid:bad/releases/g/",
            "https://example.invalid/releases/g\\/",
            "https://example.invalid/releases/white space/",
            "https://example.invalid/releases/line\nbreak/",
            "https://example.invalid/releases/직접/",
            "https://[1:::2]/releases/g/",
            "https://[1.2.3.4]/releases/g/",
            "https://[2001:db8::1]extra/releases/g/",
        ] {
            assert!(
                RemoteReleaseBase::parse(OsStr::new(invalid)).is_err(),
                "{invalid:?}"
            );
        }
        assert!(RemoteReleaseBase::parse(OsStr::new(
            "https://[2001:db8::1]:443/releases/generation-1/"
        ))
        .is_ok());
        assert!(RemoteReleaseBase::parse(&OsString::from_vec(vec![
            b'h', b't', b't', b'p', b's', b':', b'/', b'/', 0xff, b'/',
        ]))
        .is_err());

        let oversized = format!(
            "https://example.invalid/{}/",
            "a".repeat(REMOTE_RELEASE_URL_MAX_BYTES)
        );
        assert!(RemoteReleaseBase::parse(OsStr::new(&oversized)).is_err());
        let near_limit = format!(
            "https://example.invalid/{}/",
            "a".repeat(REMOTE_RELEASE_URL_MAX_BYTES - 26)
        );
        let near_limit = RemoteReleaseBase::parse(OsStr::new(&near_limit)).unwrap();
        assert!(matches!(
            near_limit.resource_url("runtime"),
            Err(LocalProductError::Remote(
                "remote release resource URL is too large"
            ))
        ));
        assert_eq!(REMOTE_RELEASE_FILE_MAX_BYTES, 512 * 1024 * 1024);
        assert_eq!(REMOTE_RELEASE_TOTAL_MAX_BYTES, 1024 * 1024 * 1024);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_transport_private_paths_are_create_new_and_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("b5-transport-private-paths");
        let acquisition = root.join(".acquire-test");
        create_remote_acquisition_root(&acquisition).unwrap();
        let acquisition_mode = std::fs::symlink_metadata(&acquisition)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(acquisition_mode & 0o077, 0);
        assert!(create_remote_acquisition_root(&acquisition).is_err());

        let nested = acquisition.join("compat/nested");
        ensure_remote_private_directory(&acquisition.join("compat")).unwrap();
        ensure_remote_private_directory(&nested).unwrap();
        assert_eq!(
            std::fs::symlink_metadata(&nested)
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );

        let output_path = nested.join("asset");
        let output = create_remote_output(&output_path).unwrap();
        assert_eq!(
            output.metadata().unwrap().permissions().mode() & 0o7777,
            0o600
        );
        assert!(create_remote_output(&output_path).is_err());

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_transport_pinned_curl_argv_environment_and_output_are_exact() {
        let (root, mut roots) = b2_test_roots("b5-transport-exact");
        roots.curl = root.join("bin/curl");
        let log = root.join("curl-arguments");
        b5_write_fake_curl(&roots.curl, &log, "body", 0);
        let output_path = root.join("remote-output");
        let output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
            .unwrap();
        let url = "https://example.invalid/releases/generation-1/release.manifest";
        assert_eq!(fetch_remote_file(&roots, url, &output, 17).unwrap(), 4);
        assert_eq!(std::fs::read(&output_path).unwrap(), b"body");
        let arguments: Vec<_> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            arguments,
            vec![
                "--disable",
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--location",
                "--cacert",
                roots.cert_file.to_str().unwrap(),
                "--capath",
                roots.cert_dir.to_str().unwrap(),
                "--connect-timeout",
                REMOTE_CONNECT_TIMEOUT_SECONDS,
                "--max-time",
                REMOTE_TRANSFER_TIMEOUT_SECONDS,
                "--max-filesize",
                "17",
                "--url",
                url,
            ]
        );
        assert!(arguments.iter().any(|argument| argument == "--location"));
        assert!(arguments
            .windows(2)
            .any(|window| window == ["--proto-redir", "=https"]));

        let control_path = root.join("remote-control-output");
        let control_output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&control_path)
            .unwrap();
        assert_eq!(
            fetch_remote_control_file(&roots, url, &control_output, 17).unwrap(),
            4
        );
        let control_arguments: Vec<_> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            control_arguments,
            vec![
                "--disable",
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--location",
                "--cacert",
                roots.cert_file.to_str().unwrap(),
                "--capath",
                roots.cert_dir.to_str().unwrap(),
                "--connect-timeout",
                REMOTE_CONNECT_TIMEOUT_SECONDS,
                "--max-time",
                REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS,
                "--max-filesize",
                "17",
                "--url",
                url,
            ]
        );

        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b5_transport_failures_and_output_bounds_fail_closed() {
        use std::os::unix::fs::symlink;

        let (root, mut roots) = b2_test_roots("b5-transport-failures");
        let log = root.join("curl-arguments");
        roots.curl = root.join("bin/curl");
        b5_write_fake_curl(&roots.curl, &log, "partial", 22);
        let failed_path = root.join("failed-output");
        let failed = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&failed_path)
            .unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/release.manifest",
                &failed,
                32,
            ),
            Err(LocalProductError::RemoteTransportFailed)
        ));
        assert_eq!(std::fs::read(&failed_path).unwrap(), b"partial");

        b5_write_fake_curl(&roots.curl, &log, "12345", 0);
        let oversized_path = root.join("oversized-output");
        let oversized = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&oversized_path)
            .unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &oversized,
                4,
            ),
            Err(LocalProductError::RemoteResponseTooLarge)
        ));
        assert_eq!(std::fs::metadata(&oversized_path).unwrap().len(), 5);

        let prefilled_path = root.join("prefilled-output");
        std::fs::write(&prefilled_path, b"x").unwrap();
        let prefilled = std::fs::OpenOptions::new()
            .write(true)
            .open(&prefilled_path)
            .unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &prefilled,
                4,
            ),
            Err(LocalProductError::Remote(
                "remote release output must be an empty regular file"
            ))
        ));
        let directory = std::fs::File::open(&root).unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &directory,
                4,
            ),
            Err(LocalProductError::Remote(
                "remote release output must be an empty regular file"
            ))
        ));
        let empty_path = root.join("invalid-bound-output");
        let empty = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&empty_path)
            .unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &empty,
                0,
            ),
            Err(LocalProductError::Remote(
                "remote release response bound is invalid"
            ))
        ));
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &empty,
                REMOTE_RELEASE_TOTAL_MAX_BYTES + 1,
            ),
            Err(LocalProductError::Remote(
                "remote release response bound is invalid"
            ))
        ));

        let real_curl = roots.curl.clone();
        roots.curl = root.join("missing-curl");
        let missing_path = root.join("missing-output");
        let missing = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(missing_path)
            .unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &missing,
                4,
            ),
            Err(LocalProductError::CurlUnavailable)
        ));
        roots.curl = root.join("curl-link");
        symlink(&real_curl, &roots.curl).unwrap();
        let linked_path = root.join("linked-output");
        let linked = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(linked_path)
            .unwrap();
        assert!(matches!(
            fetch_remote_file(
                &roots,
                "https://example.invalid/releases/g/runtime",
                &linked,
                4,
            ),
            Err(LocalProductError::CurlUnavailable)
        ));

        remove_temp_root(root);
    }

    #[cfg(unix)]
    fn m2_b1_unique_paths(label: &str) -> CoreStatePaths {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-m2-b1-{label}-{}-{counter}",
            std::process::id()
        ));
        remove_temp_root(&root);
        let paths = CoreStatePaths::new(&root).expect("M2-B1 temp root must be valid");
        prepare_core_state_paths(&paths).expect("prepare M2-B1 temp root");
        paths
    }

    #[cfg(unix)]
    fn m2_b1_cleanup(paths: &CoreStatePaths) {
        remove_temp_root(&paths.root);
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
            update_key: ReleasePublicKey([0x11; 32]),
            current: "generation = alpha 한국어".to_string(),
            current_key: ReleasePublicKey([0x22; 32]),
            previous: Some("previous value".to_string()),
            previous_key: Some(ReleasePublicKey([0x33; 32])),
        };
        let encoded = encode_pointer_state(&state).unwrap();
        assert_eq!(
            encoded,
            format!(
                "format=codex-activation-state-v3\nupdate_key={}\ncurrent=generation = alpha 한국어\ncurrent_key={}\nprevious_present=1\nprevious=previous value\nprevious_key={}\n",
                ReleasePublicKey([0x11; 32]).to_hex(),
                ReleasePublicKey([0x22; 32]).to_hex(),
                ReleasePublicKey([0x33; 32]).to_hex(),
            )
            .as_bytes()
        );
        assert_eq!(parse_pointer_state(&encoded).unwrap(), state);

        for bad in [
            "",
            ".",
            "..",
            "nested/name",
            "line\nbreak",
            "line\rbreak",
            "tab\tbreak",
            "delete\u{7f}byte",
            "nul\0byte",
        ] {
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
        let valid = plan_initial_pointer_state("g1").unwrap();
        let valid_bytes = encode_pointer_state(&valid).unwrap();
        assert_eq!(parse_pointer_state(&valid_bytes).unwrap(), valid);
        let duplicate = GenerationPointerState {
            update_key: valid.update_key,
            current: "g1".to_string(),
            current_key: valid.current_key,
            previous: Some("g1".to_string()),
            previous_key: Some(valid.current_key),
        };
        assert_eq!(
            encode_pointer_state(&duplicate),
            Err(StateFormatError::NoChange)
        );
        let duplicate_bytes = format!(
            "format=codex-activation-state-v3\nupdate_key={}\ncurrent=g1\ncurrent_key={}\nprevious_present=1\nprevious=g1\nprevious_key={}\n",
            valid.update_key.to_hex(),
            valid.current_key.to_hex(),
            valid.current_key.to_hex(),
        );
        assert_eq!(
            parse_pointer_state(duplicate_bytes.as_bytes()),
            Err(StateFormatError::NoChange)
        );

        let key = valid.update_key.to_hex();
        let malformed_states: Vec<Vec<u8>> = vec![
            b"format=codex-activation-state-v2\ncurrent=g1\nprevious_present=0\nprevious=\n".to_vec(),
            format!("format=codex-activation-state-v3\ncurrent=g1\nupdate_key={key}\ncurrent_key={key}\nprevious_present=0\nprevious=\nprevious_key=\n").into_bytes(),
            format!("format=codex-activation-state-v3\nupdate_key={key}\ncurrent=g1\ncurrent_key={key}\nprevious_present=2\nprevious=\nprevious_key=\n").into_bytes(),
            format!("format=codex-activation-state-v3\nupdate_key={key}\ncurrent=g1\ncurrent_key={key}\nprevious_present=0\nprevious=ghost\nprevious_key=\n").into_bytes(),
            format!("format=codex-activation-state-v3\nupdate_key={key}\ncurrent=g1\ncurrent_key={key}\nprevious_present=1\nprevious=g0\nprevious_key=\n").into_bytes(),
            format!("format=codex-activation-state-v3\nupdate_key={}\ncurrent=g1\ncurrent_key={key}\nprevious_present=0\nprevious=\nprevious_key=\n", "A".repeat(64)).into_bytes(),
            vec![0xff, 0xfe, 0xfd, b'\n'],
            vec![b'x'; 20_000],
        ];
        for malformed in malformed_states {
            assert!(parse_pointer_state(&malformed).is_err());
        }

        let after = plan_activation_pointer_state(&valid, "g2").unwrap();
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
        let absent_with_data = format!(
            "format=codex-activation-journal-v3\nbefore_present=0\nbefore_update_key={}\nbefore_current=g1\nbefore_current_key={}\nbefore_previous_present=0\nbefore_previous=\nbefore_previous_key=\nafter_update_key={}\nafter_current=g2\nafter_current_key={}\nafter_previous_present=1\nafter_previous=g1\nafter_previous_key={}\n",
            valid.update_key.to_hex(),
            valid.current_key.to_hex(),
            after.update_key.to_hex(),
            after.current_key.to_hex(),
            after.previous_key.unwrap().to_hex(),
        );
        assert!(matches!(
            parse_activation_journal(absent_with_data.as_bytes()),
            Err(StateFormatError::InconsistentAbsent("journal before state"))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b1_c_initial_activation_upgrade_and_rollback_semantics_are_exact() {
        let key = ReleasePublicKey([0x11; 32]);
        let initial = plan_initial_pointer_state("g1").unwrap();
        assert_eq!(
            initial,
            GenerationPointerState {
                update_key: key,
                current: "g1".to_string(),
                current_key: key,
                previous: None,
                previous_key: None,
            }
        );
        let upgraded = plan_activation_pointer_state(&initial, "g2").unwrap();
        assert_eq!(
            upgraded,
            GenerationPointerState {
                update_key: key,
                current: "g2".to_string(),
                current_key: key,
                previous: Some("g1".to_string()),
                previous_key: Some(key),
            }
        );
        let rollback = plan_rollback_pointer_state(&upgraded).unwrap();
        assert_eq!(
            rollback,
            GenerationPointerState {
                update_key: key,
                current: "g1".to_string(),
                current_key: key,
                previous: Some("g2".to_string()),
                previous_key: Some(key),
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
    fn test_m2_b7_slice2_rotation_state_and_rollback_keep_forward_authority() {
        let k0 = ReleasePublicKey([0x21; 32]);
        let k1 = ReleasePublicKey([0x42; 32]);
        let initial = plan_initial_pointer_state_with_key("g0", k0).unwrap();
        let forward = plan_activation_pointer_state_with_key(&initial, "g1", k1).unwrap();
        assert_eq!(forward.update_key, k1);
        assert_eq!(forward.current, "g1");
        assert_eq!(forward.current_key, k1);
        assert_eq!(forward.previous.as_deref(), Some("g0"));
        assert_eq!(forward.previous_key, Some(k0));
        assert_eq!(
            parse_pointer_state(&encode_pointer_state(&forward).unwrap()).unwrap(),
            forward
        );

        let rollback = plan_rollback_pointer_state(&forward).unwrap();
        assert_eq!(rollback.update_key, k1);
        assert_eq!(rollback.current, "g0");
        assert_eq!(rollback.current_key, k0);
        assert_eq!(rollback.previous.as_deref(), Some("g1"));
        assert_eq!(rollback.previous_key, Some(k1));
        let journal = ActivationJournal {
            before: Some(forward.clone()),
            after: rollback.clone(),
        };
        assert_eq!(
            parse_activation_journal(&encode_activation_journal(&journal).unwrap()).unwrap(),
            journal
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
        let generation_root = paths.root.with_extension("generation-sentinel-dir");
        let generation_dir = generation_root.join("opaque-g1").join("nested");
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
        assert!(!generation_root.join("opaque-g2").exists());
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(upgraded));
        m2_b1_assert_no_transaction_files(&paths);

        let _ = std::fs::remove_file(&outside);
        remove_temp_root(&generation_root);
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

    #[cfg(unix)]
    #[test]
    fn test_m2_r2_activation_writers_are_serialized_before_state_mutation() {
        let paths = m2_b1_unique_paths("m2-r2-writer-lock");
        let old = plan_initial_pointer_state("g1").unwrap();
        activate_pointer_state(&paths, None, &old).unwrap();
        let new = plan_activation_pointer_state(&old, "g2").unwrap();
        let held_lock = acquire_activation_lock(&paths).unwrap();
        let contender_paths = paths.clone();
        let contender_before = old.clone();
        let contender_after = new.clone();
        let contender = std::thread::spawn(move || {
            activate_pointer_state(&contender_paths, Some(&contender_before), &contender_after)
        });
        assert!(matches!(
            contender.join().unwrap(),
            Err(ActivationTransactionError::WriterBusy)
        ));
        assert!(matches!(
            recover_activation_state(&paths),
            Err(ActivationTransactionError::WriterBusy)
        ));
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(old.clone()));
        m2_b1_assert_no_transaction_files(&paths);

        drop(held_lock);
        activate_pointer_state(&paths, Some(&old), &new).unwrap();
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(new));
        m2_b1_assert_no_transaction_files(&paths);
        m2_b1_cleanup(&paths);
    }

    #[cfg(unix)]
    struct M2B9PauseIo {
        pause_after_call: usize,
        calls: usize,
        reached: std::sync::mpsc::Sender<usize>,
        resume: std::sync::mpsc::Receiver<()>,
        inner: FsActivationIo,
    }

    #[cfg(unix)]
    impl M2B9PauseIo {
        fn new(
            pause_after_call: usize,
            reached: std::sync::mpsc::Sender<usize>,
            resume: std::sync::mpsc::Receiver<()>,
        ) -> Self {
            Self {
                pause_after_call,
                calls: 0,
                reached,
                resume,
                inner: FsActivationIo,
            }
        }

        fn around<T>(
            &mut self,
            action: impl FnOnce(&mut FsActivationIo) -> std::io::Result<T>,
        ) -> std::io::Result<T> {
            self.calls += 1;
            let current = self.calls;
            let result = action(&mut self.inner)?;
            if current == self.pause_after_call {
                self.reached.send(current).map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "M2-B9 pause observer disappeared",
                    )
                })?;
                self.resume.recv().map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "M2-B9 pause release disappeared",
                    )
                })?;
            }
            Ok(result)
        }
    }

    #[cfg(unix)]
    impl ActivationIo for M2B9PauseIo {
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
    #[test]
    fn test_m2_b9_slice0_overlap_harness_binds_real_loader_and_durable_boundary() {
        let (root, roots) = b2_test_roots("b9-slice0-harness");
        b2_write_generation(&roots, "b9-old", false, "unsupported");
        b2_write_generation(&roots, "b9-new", false, "unsupported");
        b2_activate(&roots, "b9-old");

        assert_eq!(
            load_activated_generation(&roots).unwrap().generation_id,
            "b9-old"
        );
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let old = read_pointer_state(&paths).unwrap().unwrap();
        let new = plan_activation_pointer_state_with_key(&old, "b9-new", old.update_key).unwrap();
        let (reached_tx, reached_rx) = std::sync::mpsc::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();

        let calls = std::thread::scope(|scope| {
            let activation = scope.spawn(|| {
                let mut io = M2B9PauseIo::new(3, reached_tx, resume_rx);
                activate_pointer_state_with_io(&paths, Some(&old), &new, &mut io).unwrap();
                io.calls
            });
            assert_eq!(
                reached_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap(),
                3
            );
            assert!(paths.activation_journal.is_file());
            assert!(!paths.activation_journal_temp.exists());
            assert!(!paths.activation_state_temp.exists());
            assert_eq!(read_pointer_state(&paths).unwrap(), Some(old.clone()));
            resume_tx.send(()).unwrap();
            activation.join().unwrap()
        });

        assert_eq!(calls, 8);
        assert_eq!(read_pointer_state(&paths).unwrap(), Some(new));
        assert_eq!(
            load_activated_generation(&roots).unwrap().generation_id,
            "b9-new"
        );
        m2_b1_assert_no_transaction_files(&paths);
        remove_temp_root(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b9_slice1_ordinary_launch_cannot_disrupt_successful_activation() {
        for pause_after_call in [1usize, 3, 4, 6] {
            let (root, roots) = b2_test_roots(&format!("b9-slice1-overlap-{pause_after_call}"));
            b2_write_generation(&roots, "b9-old", false, "unsupported");
            b2_write_generation(&roots, "b9-new", false, "unsupported");
            b2_activate(&roots, "b9-old");

            let paths = CoreStatePaths::new(&roots.state_root).unwrap();
            let old = read_pointer_state(&paths).unwrap().unwrap();
            let new =
                plan_activation_pointer_state_with_key(&old, "b9-new", old.update_key).unwrap();
            let (reached_tx, reached_rx) = std::sync::mpsc::channel();
            let (resume_tx, resume_rx) = std::sync::mpsc::channel();

            let (observed_generation, activation_result, calls) = std::thread::scope(|scope| {
                let activation = scope.spawn(|| {
                    let mut io = M2B9PauseIo::new(pause_after_call, reached_tx, resume_rx);
                    let result = activate_pointer_state_with_io(&paths, Some(&old), &new, &mut io);
                    (result, io.calls)
                });
                assert_eq!(
                    reached_rx
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap(),
                    pause_after_call
                );
                let observed = load_activated_generation(&roots).map(|loaded| loaded.generation_id);
                resume_tx.send(()).unwrap();
                let (result, calls) = activation.join().unwrap();
                (observed, result, calls)
            });

            let final_state = read_pointer_state(&paths).unwrap();
            let transaction_files = (
                paths.activation_journal.exists(),
                paths.activation_journal_temp.exists(),
                paths.activation_state_temp.exists(),
            );
            remove_temp_root(&root);

            let observed_generation = observed_generation.unwrap_or_else(|error| {
                panic!("pause_after_call={pause_after_call}: ordinary launch failed: {error}")
            });
            assert!(
                observed_generation == "b9-old" || observed_generation == "b9-new",
                "pause_after_call={pause_after_call}: observed {observed_generation:?}"
            );
            assert!(
                activation_result.is_ok(),
                "pause_after_call={pause_after_call}: ordinary launch disrupted activation after {calls} durable calls: {activation_result:?}; final_state={final_state:?}; transaction_files={transaction_files:?}"
            );
            assert_eq!(
                final_state.as_ref().map(|state| state.current.as_str()),
                Some("b9-new"),
                "pause_after_call={pause_after_call}"
            );
            assert_eq!(transaction_files, (false, false, false));
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b9_slice2_failed_pre_activation_update_keeps_launch_old() {
        for timing in [M2B1FaultTiming::Before, M2B1FaultTiming::After] {
            for fail_call in 1usize..=4 {
                let label = format!("b9-slice2-fault-{timing:?}-{fail_call}");
                let (root, roots) = b2_test_roots(&label);
                b2_write_generation(&roots, "b9-old", false, "unsupported");
                b2_write_generation(&roots, "b9-new", false, "unsupported");
                b2_activate(&roots, "b9-old");

                let paths = CoreStatePaths::new(&roots.state_root).unwrap();
                let old = read_pointer_state(&paths).unwrap().unwrap();
                let new =
                    plan_activation_pointer_state_with_key(&old, "b9-new", old.update_key).unwrap();
                let mut io = M2B1FaultIo::new(fail_call, timing);
                let failure = activate_pointer_state_with_io(&paths, Some(&old), &new, &mut io)
                    .expect_err("pre-activation injected fault must abort update");
                assert!(matches!(failure, ActivationTransactionError::Io { .. }));
                assert_eq!(io.calls, fail_call);
                assert_eq!(read_pointer_state(&paths).unwrap(), Some(old.clone()));

                let journal_before = std::fs::read(&paths.activation_journal).ok();
                let journal_temp_before = std::fs::read(&paths.activation_journal_temp).ok();
                let state_temp_before = std::fs::read(&paths.activation_state_temp).ok();
                assert_eq!(
                    load_activated_generation(&roots).unwrap().generation_id,
                    "b9-old",
                    "timing={timing:?} fail_call={fail_call}"
                );
                assert_eq!(read_pointer_state(&paths).unwrap(), Some(old.clone()));
                assert_eq!(
                    std::fs::read(&paths.activation_journal).ok(),
                    journal_before
                );
                assert_eq!(
                    std::fs::read(&paths.activation_journal_temp).ok(),
                    journal_temp_before
                );
                assert_eq!(
                    std::fs::read(&paths.activation_state_temp).ok(),
                    state_temp_before
                );

                assert_eq!(recover_activation_state(&paths).unwrap(), Some(old.clone()));
                assert_eq!(read_pointer_state(&paths).unwrap(), Some(old));
                assert_eq!(
                    load_activated_generation(&roots).unwrap().generation_id,
                    "b9-old"
                );
                m2_b1_assert_no_transaction_files(&paths);
                remove_temp_root(root);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_m2_b9_slice3_recovery_overlap_reads_only_complete_authoritative_generation() {
        for timing in [M2B1FaultTiming::Before, M2B1FaultTiming::After] {
            for fail_call in 1usize..=8 {
                let label = format!("b9-slice3-recovery-{timing:?}-{fail_call}");
                let (root, roots) = b2_test_roots(&label);
                b2_write_generation(&roots, "b9-old", false, "unsupported");
                b2_write_generation(&roots, "b9-new", false, "unsupported");
                b2_activate(&roots, "b9-old");

                let paths = CoreStatePaths::new(&roots.state_root).unwrap();
                let old = read_pointer_state(&paths).unwrap().unwrap();
                let new =
                    plan_activation_pointer_state_with_key(&old, "b9-new", old.update_key).unwrap();
                let mut fault_io = M2B1FaultIo::new(fail_call, timing);
                let failure =
                    activate_pointer_state_with_io(&paths, Some(&old), &new, &mut fault_io)
                        .expect_err("injected durable-boundary fault must abort activation");
                assert!(matches!(failure, ActivationTransactionError::Io { .. }));
                assert_eq!(fault_io.calls, fail_call);

                let authoritative = read_pointer_state(&paths).unwrap().unwrap();
                assert!(authoritative == old || authoritative == new);
                let expected_generation = authoritative.current.clone();
                let transaction_snapshot = || {
                    (
                        std::fs::read(&paths.activation_journal).ok(),
                        std::fs::read(&paths.activation_journal_temp).ok(),
                        std::fs::read(&paths.activation_state_temp).ok(),
                    )
                };

                let before_launch = transaction_snapshot();
                assert_eq!(
                    load_activated_generation(&roots).unwrap().generation_id,
                    expected_generation,
                    "before recovery timing={timing:?} fail_call={fail_call}"
                );
                assert_eq!(transaction_snapshot(), before_launch);

                let has_recovery_io = paths.activation_journal.exists()
                    || paths.activation_journal_temp.exists()
                    || paths.activation_state_temp.exists();
                let recovered = if has_recovery_io {
                    let (reached_tx, reached_rx) = std::sync::mpsc::channel();
                    let (resume_tx, resume_rx) = std::sync::mpsc::channel();
                    std::thread::scope(|scope| {
                        let recovery = scope.spawn(|| {
                            let mut io = M2B9PauseIo::new(1, reached_tx, resume_rx);
                            let result = recover_activation_state_with_io(&paths, &mut io);
                            (result, io.calls)
                        });
                        assert_eq!(
                            reached_rx
                                .recv_timeout(std::time::Duration::from_secs(5))
                                .unwrap(),
                            1,
                            "timing={timing:?} fail_call={fail_call}"
                        );
                        let during_recovery = transaction_snapshot();
                        assert_eq!(
                            load_activated_generation(&roots).unwrap().generation_id,
                            expected_generation,
                            "during recovery timing={timing:?} fail_call={fail_call}"
                        );
                        assert_eq!(transaction_snapshot(), during_recovery);
                        resume_tx.send(()).unwrap();
                        let (result, calls) = recovery.join().unwrap();
                        assert!(calls >= 1);
                        result.unwrap()
                    })
                } else {
                    recover_activation_state(&paths).unwrap()
                };

                assert_eq!(recovered, Some(authoritative.clone()));
                assert_eq!(read_pointer_state(&paths).unwrap(), Some(authoritative));
                assert_eq!(
                    load_activated_generation(&roots).unwrap().generation_id,
                    expected_generation,
                    "after recovery timing={timing:?} fail_call={fail_call}"
                );
                m2_b1_assert_no_transaction_files(&paths);
                remove_temp_root(root);
            }
        }
    }
}
