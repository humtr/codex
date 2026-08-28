use std::ffi::{OsStr, OsString};

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
    match original.first().map(OsString::as_os_str) {
        Some(value) if value == OsStr::new("update") => {
            PublicDispatchRoute::Update(original.into_iter().skip(1).collect())
        }
        Some(value) if value == OsStr::new("doctor") => {
            PublicDispatchRoute::Doctor(original.into_iter().skip(1).collect())
        }
        Some(value) if value == OsStr::new("termux") => {
            PublicDispatchRoute::Termux(original.into_iter().skip(1).collect())
        }
        _ => PublicDispatchRoute::Upstream(original),
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

#[cfg(unix)]
pub const RESOLVER_FD: std::os::raw::c_int = 33;
#[cfg(unix)]
pub const CONFIG_DIR_FD: std::os::raw::c_int = 34;
#[cfg(unix)]
const SAFE_MIN_FD: std::os::raw::c_int = 35;
#[cfg(unix)]
const F_DUPFD_CLOEXEC: std::os::raw::c_int = 1030;

#[cfg(unix)]
extern "C" {
    fn dup2(oldfd: std::os::raw::c_int, newfd: std::os::raw::c_int) -> std::os::raw::c_int;
    fn fcntl(fd: std::os::raw::c_int, cmd: std::os::raw::c_int, ...) -> std::os::raw::c_int;
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
        }
    }
}

#[cfg(unix)]
impl std::error::Error for TermuxProcessEnvError {}

#[cfg(unix)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
struct TermuxBaseEnvPlan {
    assignments: Vec<(OsString, OsString)>,
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

#[cfg(unix)]
fn plan_termux_env(
    snapshot: &TermuxProcessEnvSnapshot,
    compat_dir: &OsStr,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
) -> Result<TermuxBaseEnvPlan, TermuxProcessEnvError> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let prefix = required_process_env(&snapshot.prefix, "PREFIX")?;
    let temp_dir = required_process_env(&snapshot.tmpdir, "TMPDIR")?;
    let prefix_bin = std::path::PathBuf::from(prefix).join("bin");
    if !valid_path_component(compat_dir) {
        return Err(TermuxProcessEnvError::InvalidPathComponent("compat_dir"));
    }
    if !valid_path_component(prefix_bin.as_os_str()) {
        return Err(TermuxProcessEnvError::InvalidPathComponent("prefix_bin"));
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

    let mut assignments = Vec::with_capacity(7);
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

    Ok(TermuxBaseEnvPlan { assignments })
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
            let mut command = std::process::Command::new(selection.program_path);
            command.args(args);
            Err(command.exec())
        }
    }
}

#[cfg(unix)]
#[derive(Debug)]
enum RuntimeLaunchError {
    Environment(TermuxProcessEnvError),
    Policy(PassthroughError),
    Exec(std::io::Error),
}

#[cfg(unix)]
impl std::fmt::Display for RuntimeLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeLaunchError::Environment(err) => err.fmt(f),
            RuntimeLaunchError::Policy(err) => err.fmt(f),
            RuntimeLaunchError::Exec(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for RuntimeLaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuntimeLaunchError::Environment(err) => Some(err),
            RuntimeLaunchError::Policy(err) => Some(err),
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
#[allow(dead_code)]
fn launch_qualified_runtime<'selection, 'asset, R, C, I, S>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
    process_env: &TermuxProcessEnvSnapshot,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
    resolver_path: R,
    config_dir: C,
    args: I,
) -> RuntimeLaunchError
where
    R: AsRef<std::path::Path>,
    C: AsRef<std::path::Path>,
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let selection = assets.selection();
    let env_plan = match plan_termux_env(
        process_env,
        selection.compatibility_dir,
        cert_file,
        cert_dir,
    ) {
        Ok(plan) => plan,
        Err(err) => return RuntimeLaunchError::Environment(err),
    };

    let planned_args = match plan_passthrough_args(args) {
        Ok(args) => args,
        Err(err) => return RuntimeLaunchError::Policy(err),
    };
    RuntimeLaunchError::Exec(exec_runtime(
        selection.runtime.program_path,
        &planned_args,
        resolver_path,
        config_dir,
        Some(&env_plan),
    ))
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug)]
enum QualifiedUpstreamDoctorProbeError {
    Environment(TermuxProcessEnvError),
    Io(std::io::Error),
}

#[cfg(unix)]
impl std::fmt::Display for QualifiedUpstreamDoctorProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedUpstreamDoctorProbeError::Environment(err) => err.fmt(f),
            QualifiedUpstreamDoctorProbeError::Io(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for QualifiedUpstreamDoctorProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QualifiedUpstreamDoctorProbeError::Environment(err) => Some(err),
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
fn probe_qualified_upstream_doctor<'selection, 'asset, R, C>(
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
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
    let env_plan = plan_termux_env(
        process_env,
        selection.compatibility_dir,
        cert_file,
        cert_dir,
    )
    .map_err(QualifiedUpstreamDoctorProbeError::Environment)?;
    let runtime_fds = RuntimeFdSources::open(resolver_path, config_dir)
        .map_err(QualifiedUpstreamDoctorProbeError::Io)?;
    let mut cmd = std::process::Command::new(selection.runtime.program_path);
    cmd.args(["-c", "sandbox_mode=\"danger-full-access\"", "doctor"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
    runtime_fds.configure(&mut cmd);
    let status = cmd
        .status()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateArtifactSource<'a> {
    Remote { immutable_locator: &'a str },
    LocalArtifact { path: &'a OsStr },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UpdateCandidateEvidence<'a> {
    signed_release_manifest_identity: &'a str,
    expected_source_artifact_digest: &'a str,
    release_signature_ok: bool,
    architecture_ok: bool,
    core_api_ok: bool,
    channel_ok: bool,
    anti_rollback_ok: bool,
    artifact_digest_ok: bool,
    archive_safe: bool,
    compatibility_ok: bool,
    candidate_probe_ok: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UpdateRequest<'a> {
    source: UpdateArtifactSource<'a>,
    evidence: UpdateCandidateEvidence<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UpdateInterfaceError {
    Empty(&'static str),
    Rejected(&'static str),
    SourceArtifactDigestMismatch,
}

impl std::fmt::Display for UpdateInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateInterfaceError::Empty(name) => {
                write!(f, "required update value '{name}' is empty")
            }
            UpdateInterfaceError::Rejected(name) => {
                write!(f, "update evidence '{name}' was rejected")
            }
            UpdateInterfaceError::SourceArtifactDigestMismatch => {
                f.write_str("qualified generation source digest does not match admitted release")
            }
        }
    }
}

impl std::error::Error for UpdateInterfaceError {}

fn require_update_evidence(value: bool, name: &'static str) -> Result<(), UpdateInterfaceError> {
    if value {
        Ok(())
    } else {
        Err(UpdateInterfaceError::Rejected(name))
    }
}

fn qualify_update_candidate<'request, 'value, 'generation>(
    request: &'request UpdateRequest<'value>,
    generation: QualifiedGenerationManifest<'generation>,
) -> Result<(), UpdateInterfaceError> {
    if request.evidence.signed_release_manifest_identity.is_empty() {
        return Err(UpdateInterfaceError::Empty(
            "signed_release_manifest_identity",
        ));
    }
    if request.evidence.expected_source_artifact_digest.is_empty() {
        return Err(UpdateInterfaceError::Empty(
            "expected_source_artifact_digest",
        ));
    }
    match request.source {
        UpdateArtifactSource::Remote { immutable_locator } if immutable_locator.is_empty() => {
            return Err(UpdateInterfaceError::Empty("immutable_locator"));
        }
        UpdateArtifactSource::LocalArtifact { path } if path.is_empty() => {
            return Err(UpdateInterfaceError::Empty("local_artifact_path"));
        }
        _ => {}
    }
    for (value, name) in [
        (request.evidence.release_signature_ok, "release_signature"),
        (request.evidence.architecture_ok, "architecture"),
        (request.evidence.core_api_ok, "core_api"),
        (request.evidence.channel_ok, "channel"),
        (request.evidence.anti_rollback_ok, "anti_rollback"),
        (request.evidence.artifact_digest_ok, "artifact_digest"),
        (request.evidence.archive_safe, "archive_safety"),
        (request.evidence.compatibility_ok, "compatibility"),
        (request.evidence.candidate_probe_ok, "candidate_probe"),
    ] {
        require_update_evidence(value, name)?;
    }

    if generation.manifest().source_artifact_digest
        != request.evidence.expected_source_artifact_digest
    {
        return Err(UpdateInterfaceError::SourceArtifactDigestMismatch);
    }

    Ok(())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorOutputMode {
    Human,
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
    Probe(QualifiedUpstreamDoctorProbeError),
}

#[cfg(unix)]
impl std::fmt::Display for LocalDoctorCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalDoctorCommandError::Usage => f.write_str("usage: codex doctor [--json]"),
            LocalDoctorCommandError::Probe(err) => err.fmt(f),
        }
    }
}

#[cfg(unix)]
impl std::error::Error for LocalDoctorCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocalDoctorCommandError::Usage => None,
            LocalDoctorCommandError::Probe(err) => Some(err),
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
        (Some(arg), None) if arg.as_os_str() == OsStr::new("--json") => Ok(DoctorOutputMode::Json),
        _ => Err(LocalDoctorCommandError::Usage),
    }
}

#[cfg(unix)]
fn run_local_doctor_command<'selection, 'asset, I, S, R, C>(
    args: I,
    capability: UpstreamDoctorCapability,
    assets: QualifiedRuntimeAssets<'selection, 'asset>,
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
    let mode = doctor_output_mode(args)?;
    let upstream = match capability {
        UpstreamDoctorCapability::Supported => probe_qualified_upstream_doctor(
            assets,
            process_env,
            cert_file,
            cert_dir,
            resolver_path,
            config_dir,
        )
        .map_err(LocalDoctorCommandError::Probe)?,
        UpstreamDoctorCapability::Unsupported => UpstreamDoctorStatus::Unsupported,
    };
    let report = compose_doctor_report(upstream, termux_core, manager);
    let output = match mode {
        DoctorOutputMode::Human => render_doctor_human(&report),
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
    manager_doctor_status: ManagerDoctorStatus,
}

#[cfg(unix)]
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicDispatchCompletion {
    Update(Vec<OsString>),
    Doctor(DoctorCommandOutcome),
    TermuxUnavailable(&'static str),
}

#[cfg(unix)]
#[derive(Debug)]
enum PublicDispatchExecutionError {
    Upstream(RuntimeLaunchError),
    Doctor(LocalDoctorCommandError),
    Manager(std::io::Error),
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

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let _ = plan_public_dispatch(args);
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
            qualification: GenerationQualification::Qualified,
            creation_metadata: "test-fixture".to_string(),
        }
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
    fn test_public_dispatch_exact_routes_and_upstream_preservation() {
        assert_eq!(
            plan_public_dispatch(["update", "--channel", "stable"]),
            PublicDispatchRoute::Update(vec!["--channel".into(), "stable".into()])
        );
        assert_eq!(
            plan_public_dispatch(["doctor", "--json"]),
            PublicDispatchRoute::Doctor(vec!["--json".into()])
        );
        assert_eq!(
            plan_public_dispatch(["termux", "status"]),
            PublicDispatchRoute::Termux(vec!["status".into()])
        );
        for argv in [
            vec![],
            vec![OsString::from("--version")],
            vec![OsString::from("-V")],
            vec![OsString::from("--"), OsString::from("doctor")],
            vec![OsString::from("Doctor")],
            vec![OsString::from("doctorx")],
            vec![OsString::from("exec"), OsString::from("termux")],
        ] {
            assert_eq!(
                plan_public_dispatch(argv.clone()),
                PublicDispatchRoute::Upstream(argv)
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_public_dispatch_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let first = OsString::from_vec(vec![b'u', b'p', 0xff]);
        let tail = OsString::from_vec(vec![0x80, b'x', 0xfe]);
        match plan_public_dispatch(vec![first.clone(), tail.clone()]) {
            PublicDispatchRoute::Upstream(argv) => {
                assert_eq!(argv[0].as_bytes(), first.as_bytes());
                assert_eq!(argv[1].as_bytes(), tail.as_bytes());
            }
            other => panic!("unexpected route: {other:?}"),
        }
        match plan_public_dispatch(vec![OsString::from("doctor"), tail.clone()]) {
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
            assert!(plan_passthrough_args(argv.clone()).is_err(), "{argv:?}");
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
            prefix: Some(OsString::from("/test/prefix")),
            tmpdir: Some(OsString::from("/test/tmp")),
            inherited_path: Some(inherited.clone()),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: Some(OsString::from("/test/certs")),
        };
        let plan = plan_termux_env(
            &snapshot,
            OsStr::new("/test/compat"),
            OsStr::new("/fallback/cert.pem"),
            None,
        )
        .unwrap();
        assert_eq!(plan.assignments.len(), 7);
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
    }

    #[cfg(unix)]
    #[test]
    fn test_termux_environment_errors_and_capture_are_direct() {
        let mut snapshot = TermuxProcessEnvSnapshot {
            prefix: None,
            tmpdir: Some("/tmp".into()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        assert_eq!(
            plan_termux_env(&snapshot, OsStr::new("/compat"), OsStr::new("/cert"), None),
            Err(TermuxProcessEnvError::MissingRequired("PREFIX"))
        );
        snapshot.prefix = Some("/prefix".into());
        snapshot.tmpdir = Some(OsString::new());
        assert_eq!(
            plan_termux_env(&snapshot, OsStr::new("/compat"), OsStr::new("/cert"), None),
            Err(TermuxProcessEnvError::EmptyRequired("TMPDIR"))
        );
        let captured = capture_termux_process_env();
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
        let mut bad = manifest.clone();
        bad.qualification = GenerationQualification::Rejected;
        assert_eq!(
            qualify_generation_manifest(&bad, &requirements()).unwrap_err(),
            GenerationManifestError::RejectedQualification
        );
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

    fn valid_update_request() -> UpdateRequest<'static> {
        UpdateRequest {
            source: UpdateArtifactSource::Remote {
                immutable_locator: "https://example.invalid/release.tar.zst",
            },
            evidence: UpdateCandidateEvidence {
                signed_release_manifest_identity: "release-manifest-1",
                expected_source_artifact_digest: "source-digest",
                release_signature_ok: true,
                architecture_ok: true,
                core_api_ok: true,
                channel_ok: true,
                anti_rollback_ok: true,
                artifact_digest_ok: true,
                archive_safe: true,
                compatibility_ok: true,
                candidate_probe_ok: true,
            },
        }
    }

    #[test]
    fn test_update_qualification_is_one_direct_gate() {
        let manifest = valid_manifest(false, false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let request = valid_update_request();
        assert_eq!(qualify_update_candidate(&request, generation), Ok(()));
        let mut bad = request;
        bad.evidence.channel_ok = false;
        assert_eq!(
            qualify_update_candidate(&bad, generation),
            Err(UpdateInterfaceError::Rejected("channel"))
        );
        let mut bad = request;
        bad.evidence.expected_source_artifact_digest = "wrong";
        assert_eq!(
            qualify_update_candidate(&bad, generation),
            Err(UpdateInterfaceError::SourceArtifactDigestMismatch)
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_update_qualification_preserves_local_non_utf8_path_without_extra_wrapper() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let manifest = valid_manifest(false, false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let raw = OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]);
        let mut request = valid_update_request();
        request.source = UpdateArtifactSource::LocalArtifact { path: &raw };
        assert_eq!(qualify_update_candidate(&request, generation), Ok(()));
        match request.source {
            UpdateArtifactSource::LocalArtifact { path } => {
                assert_eq!(path.as_bytes(), raw.as_bytes())
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_doctor_report_and_usage_keep_bounded_public_contract() {
        let report = compose_doctor_report(
            UpstreamDoctorStatus::Unsupported,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        );
        assert_eq!(report.summary, DoctorSummaryStatus::Degraded);
        assert_eq!(doctor_exit_class(&report), DoctorExitClass::HealthFailure);
        assert_eq!(
            render_doctor_json(&report),
            "{\"schema_version\":1,\"upstream\":{\"status\":\"unsupported\"},\"termux_core\":{\"status\":\"healthy\"},\"manager\":{\"status\":\"unavailable\"},\"summary\":{\"status\":\"degraded\"}}\n"
        );
        assert_eq!(
            doctor_output_mode(Vec::<OsString>::new()).unwrap(),
            DoctorOutputMode::Human
        );
        assert_eq!(
            doctor_output_mode([OsString::from("--json")]).unwrap(),
            DoctorOutputMode::Json
        );
        let err = doctor_output_mode([OsString::from("secret-value")]).unwrap_err();
        assert_eq!(err.to_string(), "usage: codex doctor [--json]");
        assert!(!err.to_string().contains("secret-value"));
    }

    #[cfg(unix)]
    fn temp_root(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("codex-r2-{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
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
if [ "$1" = "doctor" ]; then
  printf 'SECRET-UPSTREAM-STDOUT\n'
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
    fn probe_context<'a>(
        manifest: &'a GenerationManifest,
        selection: &'a RuntimeAssetSelection<'a>,
        manager_selection: Option<&'a ManagerArtifactSelection<'a>>,
        snapshot: &'a TermuxProcessEnvSnapshot,
        cert_file: &'a OsStr,
        cert_dir: &'a OsStr,
        resolver: &'a std::path::Path,
        config: &'a std::path::Path,
    ) -> LocalPublicDispatchContext<'a, 'a, 'a, 'a, 'a> {
        let generation = qualify_generation_manifest(manifest, &requirements()).unwrap();
        let assets = qualify_runtime_assets(generation, selection).unwrap();
        let manager = qualify_manager_artifact(generation, manager_selection).unwrap();
        LocalPublicDispatchContext {
            runtime_assets: assets,
            manager_artifact: manager,
            process_env: snapshot,
            cert_file,
            cert_dir: Some(cert_dir),
            resolver_path: resolver,
            config_dir: config,
            doctor_capability: UpstreamDoctorCapability::Supported,
            core_doctor_status: CoreDoctorStatus::Healthy,
            manager_doctor_status: ManagerDoctorStatus::Unavailable,
        }
    }

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
        let manifest = valid_manifest(scenario == "manager", false);
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime.as_os_str(),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: compat.as_os_str(),
            helpers: &[],
        };
        let snapshot = TermuxProcessEnvSnapshot {
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
        let route = match scenario.as_str() {
            "version" => PublicDispatchRoute::Upstream(vec![OsString::from("--version")]),
            "signal" => PublicDispatchRoute::Upstream(vec![OsString::from("signal")]),
            "tty" => PublicDispatchRoute::Upstream(vec![OsString::from("tty")]),
            "exec" => PublicDispatchRoute::Upstream(vec![
                OsString::from("exec"),
                OsString::from("arg with spaces"),
                OsString::from_vec(vec![0xff, 0x80, b'z']),
            ]),
            "manager" => PublicDispatchRoute::Termux(vec![
                OsString::from("status"),
                OsString::from_vec(vec![0xff, b'm']),
            ]),
            other => panic!("unknown probe scenario {other}"),
        };
        let context = probe_context(
            &manifest,
            &selection,
            (scenario == "manager").then_some(&manager_selection),
            &snapshot,
            cert.as_os_str(),
            cert_dir.as_os_str(),
            &resolver,
            &config,
        );
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
        let status = std::process::Command::new(std::env::current_exe().unwrap())
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
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
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
        let _ = std::fs::remove_dir_all(root);
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
        let _ = std::fs::remove_dir_all(root);
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
        for _ in 0..500 {
            if pid_file.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let runtime_pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(runtime_pid, child.id());
        assert_eq!(unsafe { kill(child.id() as i32, 15) }, 0);
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(143));
        let _ = std::fs::remove_dir_all(root);
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
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn test_policy_and_environment_fail_before_runtime_io() {
        let manifest = valid_manifest(false, false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: OsStr::new("/missing/runtime"),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: OsStr::new("/compat"),
            helpers: &[],
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some("/prefix".into()),
            tmpdir: Some("/tmp".into()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        assert!(matches!(
            launch_qualified_runtime(
                assets,
                &snapshot,
                OsStr::new("/cert"),
                None,
                "/missing/resolver",
                "/missing/config",
                ["--sandbox=read-only"]
            ),
            RuntimeLaunchError::Policy(_)
        ));
        let missing_env = TermuxProcessEnvSnapshot {
            prefix: None,
            ..snapshot
        };
        assert!(matches!(
            launch_qualified_runtime(
                assets,
                &missing_env,
                OsStr::new("/cert"),
                None,
                "/missing/resolver",
                "/missing/config",
                ["--version"]
            ),
            RuntimeLaunchError::Environment(TermuxProcessEnvError::MissingRequired("PREFIX"))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_doctor_is_bounded_read_only_and_maps_upstream_status_only() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("doctor");
        let manifest = valid_manifest(false, false);
        let generation = qualify_generation_manifest(&manifest, &requirements()).unwrap();
        let compat = root.join("compat");
        let selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: runtime.as_os_str(),
                observed_digest: "runtime-digest",
            },
            compatibility_dir: compat.as_os_str(),
            helpers: &[],
        };
        let assets = qualify_runtime_assets(generation, &selection).unwrap();
        let snapshot = TermuxProcessEnvSnapshot {
            prefix: Some(root.join("prefix").into_os_string()),
            tmpdir: Some(root.join("tmp").into_os_string()),
            inherited_path: None,
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let before = std::fs::read(&resolver).unwrap();
        let outcome = run_local_doctor_command(
            [OsString::from("--json")],
            UpstreamDoctorCapability::Supported,
            assets,
            &snapshot,
            root.join("cert.pem").as_os_str(),
            Some(root.join("certs").as_os_str()),
            &resolver,
            &config,
            CoreDoctorStatus::Healthy,
            ManagerDoctorStatus::Unavailable,
        )
        .unwrap();
        assert_eq!(outcome.exit_class, DoctorExitClass::HealthFailure);
        assert!(outcome
            .output
            .contains("\"upstream\":{\"status\":\"healthy\"}"));
        assert!(!outcome.output.contains("SECRET-UPSTREAM"));
        assert_eq!(std::fs::read(&resolver).unwrap(), before);
        let _ = std::fs::remove_dir_all(root);
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
    fn test_manager_available_exec_uses_qualified_path_and_preserves_raw_argv() {
        let (root, runtime, resolver, config) = prepare_exec_fixture("manager");
        let result = run_product_probe("manager", &root, &runtime, &resolver, &config);
        assert_eq!(result.status.code(), Some(73));
        assert!(result
            .stdout
            .windows(b"ARGS:<status><".len())
            .any(|w| w == b"ARGS:<status><"));
        assert!(result.stdout.windows(2).any(|w| w == [0xff, b'm']));
        let _ = std::fs::remove_dir_all(root);
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
        let _ = std::fs::remove_dir_all(root);
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
}
