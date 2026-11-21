use crate::error::LocalProductError;
use crate::fs_util::{
    self, ensure_real_directory, ensure_real_directory_if_present, ensure_regular_file,
    read_bounded_regular_file,
};
use crate::runtime_env::{TermuxProcessEnvError, TermuxProcessEnvSnapshot};
use crate::state::generation_state;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

const LOCAL_GENERATION_FORMAT: &str = "codex-local-generation";
const WRAPPER_RUNTIME_PATH: &str = "bin/codex";
const WRAPPER_CODE_MODE_HOST_PATH: &str = "bin/codex-code-mode-host";
const WRAPPER_EXEC_SERVER_PATH: &str = "bin/codex-exec-server";
const WRAPPER_PACKAGE_JSON_PATH: &str = "codex-package.json";

pub(super) const WRAPPER_API_IDENTITY: &str = "wrapper-api";
pub(super) const PERSISTENT_SCHEMA_IDENTITY: &str = "schema";
const EXPECTED_PLATFORM: &str = "android";
const EXPECTED_ARCHITECTURE: &str = "aarch64";

const TERMUX_BROWSER_OPEN_HELPER_IDENTITY: &str = "termux-browser-open";
const TERMUX_BROWSER_MANUAL_HELPER_IDENTITY: &str = "termux-browser-manual";

// One digest row per archive entry, including full-length bin paths.
const LOCAL_GENERATION_MAX_BYTES: usize = 512 * 1024;
const LOCAL_RELEASE_MAX_FILES: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenerationLayout {
    PackageRoot,
}

impl GenerationLayout {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            GenerationLayout::PackageRoot => "package-root",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GenerationHelperDigest {
    pub(super) identity: String,
    pub(super) digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GenerationManifest {
    pub(super) upstream_package_identity: String,
    pub(super) upstream_package_version: String,
    pub(super) source_artifact_digest: String,
    pub(super) expected_platform: String,
    pub(super) expected_architecture: String,
    pub(super) patch_policy_id: String,
    pub(super) patch_report: String,
    pub(super) runtime_digest: String,
    pub(super) helper_digests: Vec<GenerationHelperDigest>,
    pub(super) companion_digests: BTreeMap<String, String>,
    pub(super) wrapper_artifact_digest: String,
    pub(super) wrapper_api_identity: String,
    pub(super) persistent_schema_identity: String,
    pub(super) creation_metadata: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GenerationManifestError {
    EmptyRequired(&'static str),
    PlatformMismatch,
    ArchitectureMismatch,
    WrapperApiMismatch,
    PersistentSchemaMismatch,
    DuplicateHelperIdentity { first: usize, duplicate: usize },
}

impl std::fmt::Display for GenerationManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerationManifestError::EmptyRequired(name) => {
                write!(f, "required generation manifest field '{name}' is empty")
            }
            GenerationManifestError::PlatformMismatch => {
                f.write_str("generation manifest platform is not android")
            }
            GenerationManifestError::ArchitectureMismatch => {
                f.write_str("generation manifest architecture is not aarch64")
            }
            GenerationManifestError::WrapperApiMismatch => {
                f.write_str("generation manifest wrapper API identity is incompatible")
            }
            GenerationManifestError::PersistentSchemaMismatch => {
                f.write_str("generation manifest persistent schema identity is incompatible")
            }
            GenerationManifestError::DuplicateHelperIdentity { first, duplicate } => write!(
                f,
                "generation manifest helper {duplicate} duplicates helper {first}"
            ),
        }
    }
}

impl std::error::Error for GenerationManifestError {}

fn qualify_generation_manifest(
    manifest: &GenerationManifest,
) -> Result<(), GenerationManifestError> {
    for (name, value) in [
        (
            "upstream_package_identity",
            &manifest.upstream_package_identity,
        ),
        (
            "upstream_package_version",
            &manifest.upstream_package_version,
        ),
        ("source_artifact_digest", &manifest.source_artifact_digest),
        ("patch_policy_id", &manifest.patch_policy_id),
        ("patch_report", &manifest.patch_report),
        ("runtime_digest", &manifest.runtime_digest),
        ("wrapper_artifact_digest", &manifest.wrapper_artifact_digest),
        ("creation_metadata", &manifest.creation_metadata),
    ] {
        if value.is_empty() {
            return Err(GenerationManifestError::EmptyRequired(name));
        }
    }
    if manifest.expected_platform != EXPECTED_PLATFORM {
        return Err(GenerationManifestError::PlatformMismatch);
    }
    if manifest.expected_architecture != EXPECTED_ARCHITECTURE {
        return Err(GenerationManifestError::ArchitectureMismatch);
    }
    if manifest.wrapper_api_identity != WRAPPER_API_IDENTITY {
        return Err(GenerationManifestError::WrapperApiMismatch);
    }
    if manifest.persistent_schema_identity != PERSISTENT_SCHEMA_IDENTITY {
        return Err(GenerationManifestError::PersistentSchemaMismatch);
    }
    for i in 0..manifest.helper_digests.len() {
        for j in (i + 1)..manifest.helper_digests.len() {
            if manifest.helper_digests[i].identity == manifest.helper_digests[j].identity {
                return Err(GenerationManifestError::DuplicateHelperIdentity {
                    first: i,
                    duplicate: j,
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeAssetBinding<'a> {
    pub(super) program_path: &'a OsStr,
    pub(super) observed_digest: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HelperAssetBinding<'a> {
    pub(super) identity: &'a str,
    pub(super) asset_path: &'a OsStr,
    pub(super) observed_digest: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeAssetSelection<'a> {
    pub(super) runtime: RuntimeAssetBinding<'a>,
    pub(super) helpers: &'a [HelperAssetBinding<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RuntimeAssetError {
    EmptyPath(&'static str),
    RelativePath(&'static str),
    NulPath(&'static str),
    EmptyRuntimeDigest,
    DuplicateHelperIdentity { first: usize, duplicate: usize },
}

impl std::fmt::Display for RuntimeAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeAssetError::EmptyPath(name) => {
                write!(f, "runtime asset path '{name}' is empty")
            }
            RuntimeAssetError::RelativePath(name) => {
                write!(f, "runtime asset path '{name}' must be absolute")
            }
            RuntimeAssetError::NulPath(name) => {
                write!(f, "runtime asset path '{name}' must not contain NUL")
            }
            RuntimeAssetError::EmptyRuntimeDigest => f.write_str("runtime asset digest is empty"),
            RuntimeAssetError::DuplicateHelperIdentity { first, duplicate } => write!(
                f,
                "selected helper asset {duplicate} duplicates selected helper asset {first}"
            ),
        }
    }
}

impl std::error::Error for RuntimeAssetError {}

#[derive(Debug, Clone, Copy)]
pub(super) struct QualifiedRuntimeAssets<'selection, 'asset> {
    selection: &'selection RuntimeAssetSelection<'asset>,
}

impl<'selection, 'asset> QualifiedRuntimeAssets<'selection, 'asset> {
    pub(super) fn selection(self) -> &'selection RuntimeAssetSelection<'asset> {
        self.selection
    }
}

pub(super) fn termux_browser_helper_paths<'asset>(
    selection: &RuntimeAssetSelection<'asset>,
) -> Result<Option<(&'asset OsStr, &'asset OsStr)>, TermuxProcessEnvError> {
    let find = |identity: &'static str| {
        selection
            .helpers
            .iter()
            .find(|h| h.identity == identity)
            .map(|h| h.asset_path)
    };
    match (
        find(TERMUX_BROWSER_OPEN_HELPER_IDENTITY),
        find(TERMUX_BROWSER_MANUAL_HELPER_IDENTITY),
    ) {
        (None, None) => Ok(None),
        (Some(open), Some(manual)) => Ok(Some((open, manual))),
        (None, Some(_)) => Err(TermuxProcessEnvError::MissingQualifiedHelper(
            TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
        )),
        (Some(_), None) => Err(TermuxProcessEnvError::MissingQualifiedHelper(
            TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
        )),
    }
}

fn validate_absolute_runtime_asset_path(
    path: &OsStr,
    name: &'static str,
) -> Result<(), RuntimeAssetError> {
    if path.is_empty() {
        return Err(RuntimeAssetError::EmptyPath(name));
    }
    if path.as_bytes().contains(&0) {
        return Err(RuntimeAssetError::NulPath(name));
    }
    if !Path::new(path).is_absolute() {
        return Err(RuntimeAssetError::RelativePath(name));
    }
    Ok(())
}

fn qualify_runtime_assets<'selection, 'asset>(
    selection: &'selection RuntimeAssetSelection<'asset>,
) -> Result<QualifiedRuntimeAssets<'selection, 'asset>, RuntimeAssetError> {
    validate_absolute_runtime_asset_path(selection.runtime.program_path, "runtime_program")?;
    if selection.runtime.observed_digest.is_empty() {
        return Err(RuntimeAssetError::EmptyRuntimeDigest);
    }
    for i in 0..selection.helpers.len() {
        validate_absolute_runtime_asset_path(selection.helpers[i].asset_path, "helper_asset")?;
        if selection.helpers[i].observed_digest.is_empty() {
            return Err(RuntimeAssetError::EmptyRuntimeDigest);
        }
        for j in (i + 1)..selection.helpers.len() {
            if selection.helpers[i].identity == selection.helpers[j].identity {
                return Err(RuntimeAssetError::DuplicateHelperIdentity {
                    first: i,
                    duplicate: j,
                });
            }
        }
    }
    Ok(QualifiedRuntimeAssets { selection })
}

#[derive(Debug, Clone)]
pub(super) struct LocalWrapperRoots {
    pub(super) generation_root: PathBuf,
    pub(super) state_root: PathBuf,
    pub(super) config_dir: PathBuf,
    pub(super) resolver_path: PathBuf,
    pub(super) cert_file: PathBuf,
    pub(super) cert_dir: PathBuf,
    pub(super) curl: PathBuf,
}

fn required_absolute_env_path(name: &'static str) -> Result<PathBuf, LocalProductError> {
    let value = std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .ok_or(LocalProductError::MissingEnvironment(name))?;
    let path = PathBuf::from(value);
    if !fs_util::wrapper_safe_absolute_path(&path) {
        return Err(LocalProductError::InvalidEnvironmentPath(name));
    }
    Ok(path)
}

impl LocalWrapperRoots {
    pub(super) fn from_environment() -> Result<Self, LocalProductError> {
        let home = required_absolute_env_path("HOME")?;
        let prefix = required_absolute_env_path("PREFIX")?;
        let state_root = home.join(".local/share/codex/wrapper");
        Ok(Self {
            generation_root: home.join(".codex/packages/standalone/releases"),
            config_dir: state_root.join("config"),
            state_root,
            resolver_path: prefix.join("etc/resolv.conf"),
            cert_file: prefix.join("etc/tls/cert.pem"),
            cert_dir: prefix.join("etc/tls/certs"),
            curl: prefix.join("bin/curl"),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct LoadedLocalGeneration {
    pub(super) generation_id: String,
    pub(super) manifest: GenerationManifest,
    pub(super) generation_layout: GenerationLayout,
    pub(super) runtime_path: PathBuf,
    pub(super) code_mode_host_path: Option<PathBuf>,
    pub(super) generation_dir: PathBuf,
    pub(super) helper_paths: Vec<PathBuf>,
}

fn valid_nonnegative_decimal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

fn descriptor_field<'a>(
    line: Option<&'a str>,
    expected: &'static str,
) -> Result<&'a str, LocalProductError> {
    let line = line.ok_or(LocalProductError::Descriptor(
        "generation descriptor is incomplete",
    ))?;
    let (name, value) = line.split_once('\t').ok_or(LocalProductError::Descriptor(
        "generation descriptor field is malformed",
    ))?;
    if name != expected || value.is_empty() {
        return Err(LocalProductError::Descriptor(
            "generation descriptor field is invalid",
        ));
    }
    Ok(value)
}

fn parse_generation_helper_path(identity: &str) -> Option<&'static str> {
    match identity {
        TERMUX_BROWSER_OPEN_HELPER_IDENTITY => Some("browser/open/curl"),
        TERMUX_BROWSER_MANUAL_HELPER_IDENTITY => Some("browser/manual/curl"),
        _ => None,
    }
}

pub(super) fn load_local_generation(
    generation_dir: &Path,
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
        Some(LOCAL_GENERATION_FORMAT) => GenerationLayout::PackageRoot,
        _ => {
            return Err(LocalProductError::Descriptor(
                "generation descriptor format is unsupported",
            ))
        }
    };

    let generation_id = descriptor_field(lines.next(), "generation_id")?;
    generation_state::validate_generation_identity(generation_id, "generation_id")
        .map_err(LocalProductError::StateFormat)?;
    let upstream_package_identity = descriptor_field(lines.next(), "upstream_package_identity")?;
    let upstream_package_version = descriptor_field(lines.next(), "upstream_package_version")?;
    let source_artifact_digest = descriptor_field(lines.next(), "source_artifact_digest")?;
    let expected_platform = descriptor_field(lines.next(), "expected_platform")?;
    let expected_architecture = descriptor_field(lines.next(), "expected_architecture")?;
    let patch_policy_id = descriptor_field(lines.next(), "patch_policy_id")?;
    let patch_report = descriptor_field(lines.next(), "patch_report")?;
    let runtime_digest = descriptor_field(lines.next(), "runtime_digest")?;
    let wrapper_artifact_digest = descriptor_field(lines.next(), "wrapper_artifact_digest")?;
    let wrapper_api_identity = descriptor_field(lines.next(), "wrapper_api_identity")?;
    let persistent_schema_identity = descriptor_field(lines.next(), "persistent_schema_identity")?;
    if descriptor_field(lines.next(), "qualification")? != "qualified" {
        return Err(LocalProductError::Descriptor(
            "activated generation is not qualified",
        ));
    }
    let creation_metadata = descriptor_field(lines.next(), "creation_metadata")?;
    let helper_count_str = descriptor_field(lines.next(), "helper_count")?;
    if !valid_nonnegative_decimal(helper_count_str) {
        return Err(LocalProductError::Descriptor(
            "generation helper count is invalid",
        ));
    }
    let helper_count: usize = helper_count_str
        .parse()
        .map_err(|_| LocalProductError::Descriptor("generation helper count is invalid"))?;
    if helper_count > LOCAL_RELEASE_MAX_FILES.saturating_sub(2) {
        return Err(LocalProductError::Descriptor(
            "generation helper count is outside the supported bound",
        ));
    }

    let mut helper_digests = Vec::with_capacity(helper_count);
    let mut helper_paths = Vec::with_capacity(helper_count);
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
                .filter(|v| !v.is_empty())
                .ok_or(LocalProductError::Descriptor(
                    "generation helper identity is invalid",
                ))?;
        let digest =
            parts
                .next()
                .filter(|v| !v.is_empty())
                .ok_or(LocalProductError::Descriptor(
                    "generation helper digest is invalid",
                ))?;
        if parts.next().is_some() {
            return Err(LocalProductError::Descriptor(
                "generation helper entry is invalid",
            ));
        }
        if let Some(rel) = parse_generation_helper_path(identity) {
            helper_digests.push(GenerationHelperDigest {
                identity: identity.to_owned(),
                digest: digest.to_owned(),
            });
            helper_paths.push(generation_dir.join(rel));
        }
    }
    let mut companion_digests = BTreeMap::new();
    for line in lines {
        let row = descriptor_field(Some(line), "companion")?;
        let (path, digest) = row.split_once('\t').ok_or(LocalProductError::Descriptor(
            "generation companion entry is malformed",
        ))?;
        let valid_path = path.strip_prefix("bin/").is_some_and(|name| {
            !matches!(name, "" | "." | ".." | "codex")
                && !name.contains(['/', '\\'])
                && !name.bytes().any(|byte| byte.is_ascii_control())
        });
        if !valid_path || !valid_sha256_hex(digest) {
            return Err(LocalProductError::Descriptor(
                "generation companion path or digest is invalid",
            ));
        }
        if companion_digests
            .insert(path.to_owned(), digest.to_owned())
            .is_some()
        {
            return Err(LocalProductError::Descriptor(
                "generation companion path is duplicated",
            ));
        }
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
        companion_digests,
        wrapper_artifact_digest: wrapper_artifact_digest.to_owned(),
        wrapper_api_identity: wrapper_api_identity.to_owned(),
        persistent_schema_identity: persistent_schema_identity.to_owned(),
        creation_metadata: creation_metadata.to_owned(),
    };

    ensure_real_directory(
        &generation_dir.join("bin"),
        "inspect activated generation bin directory",
        "activated generation bin directory must be a real directory",
    )?;
    let runtime_path = generation_dir.join(WRAPPER_RUNTIME_PATH);
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

    let code_mode_host = generation_dir.join(WRAPPER_CODE_MODE_HOST_PATH);
    let code_mode_host_path = match fs::symlink_metadata(&code_mode_host) {
        Ok(_) => {
            ensure_regular_file(
                &code_mode_host,
                "inspect activated generation code-mode host",
                "activated generation code-mode host must be a regular file",
            )?;
            Some(code_mode_host)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect activated generation code-mode host",
                source,
            })
        }
    };
    match fs::symlink_metadata(generation_dir.join(WRAPPER_EXEC_SERVER_PATH)) {
        Ok(_) => ensure_regular_file(
            &generation_dir.join(WRAPPER_EXEC_SERVER_PATH),
            "inspect activated generation exec server",
            "activated generation exec server must be a regular file",
        )?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect activated generation exec server",
                source,
            })
        }
    }
    ensure_regular_file(
        &generation_dir.join(WRAPPER_PACKAGE_JSON_PATH),
        "inspect activated generation package metadata",
        "activated generation package metadata must be a regular file",
    )?;
    for forbidden in ["compat", "runtime", "codex-code-mode-host"] {
        match fs::symlink_metadata(generation_dir.join(forbidden)) {
            Ok(_) => {
                return Err(LocalProductError::UnsafeSource(
                    "generation contains unexpected legacy path",
                ))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "inspect activated generation layout",
                    source,
                })
            }
        }
    }
    for subdir in ["codex-path", "codex-resources"] {
        ensure_real_directory_if_present(
            &generation_dir.join(subdir),
            "inspect activated generation package directory",
            "activated generation package directory must be a real directory",
        )?;
    }

    for (index, helper_path) in helper_paths.iter().enumerate() {
        if !helper_path.is_file() {
            return Err(LocalProductError::Descriptor(
                "activated generation helper is missing",
            ));
        }
        let identity = manifest.helper_digests[index].identity.as_str();
        if matches!(
            identity,
            TERMUX_BROWSER_OPEN_HELPER_IDENTITY | TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
        ) {
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
        ensure_regular_file(
            helper_path,
            "inspect activated generation helper",
            "activated generation helper must be a regular file",
        )?;
    }

    Ok(LoadedLocalGeneration {
        generation_id: generation_id.to_owned(),
        manifest,
        generation_layout,
        runtime_path,
        code_mode_host_path,
        generation_dir: generation_dir.to_path_buf(),
        helper_paths,
    })
}

fn validate_and_load_generation(
    roots: &LocalWrapperRoots,
    generation_id: &str,
) -> Result<LoadedLocalGeneration, LocalProductError> {
    let loaded = load_local_generation(&roots.generation_root.join(generation_id))?;
    if loaded.generation_id != generation_id {
        return Err(LocalProductError::Descriptor(
            "activated generation descriptor does not match pointer",
        ));
    }
    Ok(loaded)
}

pub(super) fn reconcile_standalone_current_and_load(
    roots: &LocalWrapperRoots,
    state_paths: &generation_state::WrapperStatePaths,
    pointer_state: generation_state::GenerationPointerState,
) -> Result<
    (
        generation_state::GenerationPointerState,
        LoadedLocalGeneration,
    ),
    LocalProductError,
> {
    let expected = state_paths.standalone_current_target(&pointer_state.current);
    if state_paths
        .read_standalone_current()
        .map_err(LocalProductError::State)?
        != expected
    {
        let lock = generation_state::wait_for_activation_lock(state_paths)
            .map_err(LocalProductError::State)?;
        let mut io = generation_state::FsActivationIo;
        let (authoritative, _) =
            generation_state::recover_activation_locked(state_paths, &mut io, &lock)
                .map_err(LocalProductError::State)?;
        let authoritative = authoritative.ok_or(LocalProductError::NoCurrentGeneration)?;
        let loaded = validate_and_load_generation(roots, &authoritative.current)?;
        generation_state::sync_standalone_current(
            state_paths,
            Some(&authoritative.current),
            None,
            &mut io,
            &lock,
        )
        .map_err(LocalProductError::State)?;
        return Ok((authoritative, loaded));
    }

    let loaded = validate_and_load_generation(roots, &pointer_state.current)?;
    Ok((pointer_state, loaded))
}

pub(super) fn load_activated_generation(
    roots: &LocalWrapperRoots,
) -> Result<
    (
        generation_state::GenerationPointerState,
        LoadedLocalGeneration,
    ),
    LocalProductError,
> {
    let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?
        .with_generation_root(&roots.generation_root);
    // The very first activation may have a durable journal while its
    // authoritative pointer is still absent. Recovery must inspect that
    // journal, too; otherwise reads report a misleading clean uninstalled
    // state and the next updater is blocked by a dangling transaction.
    let pointer_state = generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    reconcile_standalone_current_and_load(roots, &state_paths, pointer_state)
}

pub(super) fn inspect_activated_generation(
    roots: &LocalWrapperRoots,
) -> Result<LoadedLocalGeneration, LocalProductError> {
    let paths = generation_state::WrapperStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?
        .with_generation_root(&roots.generation_root);
    let _lock =
        generation_state::acquire_activation_read_lock(&paths).map_err(LocalProductError::State)?;
    if generation_state::has_pending_activation(&paths).map_err(LocalProductError::State)? {
        return Err(LocalProductError::Descriptor(
            "wrapper activation has a pending transaction",
        ));
    }
    let pointer = generation_state::read_pointer_state(&paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    // Validate the compatibility link's type without repairing it.
    paths
        .read_standalone_current()
        .map_err(LocalProductError::State)?;
    validate_and_load_generation(roots, &pointer.current)
}

pub(super) fn with_qualified_loaded_runtime<'loaded, T, F>(
    loaded: &'loaded LoadedLocalGeneration,
    operation: F,
) -> Result<T, LocalProductError>
where
    F: for<'selection, 'asset> FnOnce(
        QualifiedRuntimeAssets<'selection, 'asset>,
    ) -> Result<T, LocalProductError>,
{
    qualify_generation_manifest(&loaded.manifest).map_err(LocalProductError::Manifest)?;
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
        helpers: &helper_bindings,
    };
    let runtime_assets =
        qualify_runtime_assets(&runtime_selection).map_err(LocalProductError::Runtime)?;
    operation(runtime_assets)
}

pub(super) fn verify_generation_asset_digests(
    loaded: &LoadedLocalGeneration,
) -> Result<(), LocalProductError> {
    qualify_generation_manifest(&loaded.manifest).map_err(LocalProductError::Manifest)?;
    let runtime_meta =
        fs::symlink_metadata(&loaded.runtime_path).map_err(|source| LocalProductError::Io {
            operation: "inspect generation runtime permissions",
            source,
        })?;
    if !runtime_meta.file_type().is_file() || runtime_meta.permissions().mode() & 0o111 == 0 {
        return Err(LocalProductError::Descriptor(
            "generation runtime is not an executable regular file",
        ));
    }
    let observed_runtime = sha256_regular_file(&loaded.runtime_path)?;
    if observed_runtime != loaded.manifest.runtime_digest {
        return Err(LocalProductError::Descriptor(
            "generation runtime SHA-256 digest does not match generation.meta",
        ));
    }
    for (helper_meta, helper_path) in loaded
        .manifest
        .helper_digests
        .iter()
        .zip(&loaded.helper_paths)
    {
        let helper_fs_meta =
            fs::symlink_metadata(helper_path).map_err(|source| LocalProductError::Io {
                operation: "inspect generation helper permissions",
                source,
            })?;
        if !helper_fs_meta.file_type().is_file() || helper_fs_meta.permissions().mode() & 0o111 == 0
        {
            return Err(LocalProductError::Descriptor(
                "generation helper is not an executable regular file",
            ));
        }
        let observed_helper = sha256_regular_file(helper_path)?;
        if observed_helper != helper_meta.digest {
            return Err(LocalProductError::Descriptor(
                "generation helper SHA-256 digest does not match generation.meta",
            ));
        }
    }
    for (relative_path, digest) in &loaded.manifest.companion_digests {
        let path = loaded.generation_dir.join(relative_path);
        let metadata = fs::symlink_metadata(&path).map_err(|source| LocalProductError::Io {
            operation: "inspect generation companion permissions",
            source,
        })?;
        if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err(LocalProductError::Descriptor(
                "generation companion is not an executable regular file",
            ));
        }
        if sha256_regular_file(&path)? != *digest {
            return Err(LocalProductError::Descriptor(
                "generation companion SHA-256 digest does not match generation.meta",
            ));
        }
    }
    Ok(())
}

pub(super) fn sha256_regular_file(path: &Path) -> Result<String, LocalProductError> {
    ensure_regular_file(
        path,
        "inspect file for SHA-256 verification",
        "SHA-256 verification target must be a regular file",
    )?;
    codex_release_builder::sha256_file(path).map_err(|source| LocalProductError::Io {
        operation: "hash file for SHA-256 verification",
        source,
    })
}

pub(super) fn probe_release_candidate(
    loaded: &LoadedLocalGeneration,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<(), LocalProductError> {
    with_qualified_loaded_runtime(loaded, |assets| {
        crate::qualification::probe(
            assets,
            process_env,
            roots,
            &loaded.manifest.upstream_package_version,
        )
    })
}

pub(super) fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        make_test_roots, remove_temp_root, temp_root, write_sample_generation, EnvRestoreGuard,
    };

    #[test]
    fn load_generation_validates_package_root_layout_and_rejects_legacy_or_bad_metadata() {
        let root = temp_root("gen-load");
        let g1 = root.join("gen-with-cmh");
        let g2 = root.join("gen-without-cmh");
        write_sample_generation(&g1, "gen-with-cmh", true);
        write_sample_generation(&g2, "gen-without-cmh", false);

        let l1 = load_local_generation(&g1).unwrap();
        assert!(l1.code_mode_host_path.is_some());
        assert_eq!(l1.generation_layout.as_str(), "package-root");
        with_qualified_loaded_runtime(&l1, |_| Ok(())).unwrap();

        let l2 = load_local_generation(&g2).unwrap();
        assert!(l2.code_mode_host_path.is_none());

        // Unexpected legacy files inside a package-root generation are rejected.
        fs::create_dir_all(g2.join("compat")).unwrap();
        assert!(load_local_generation(&g2).is_err());

        // Incompatible wrapper API identity is rejected at qualification time.
        let bad_api = root.join("gen-bad-api");
        write_sample_generation(&bad_api, "gen-bad-api", false);
        let original = fs::read_to_string(bad_api.join("generation.meta")).unwrap();
        fs::write(
            bad_api.join("generation.meta"),
            original.replace("wrapper-api\n", "wrapper-api-old\n"),
        )
        .unwrap();
        let loaded_bad = load_local_generation(&bad_api).unwrap();
        assert!(with_qualified_loaded_runtime(&loaded_bad, |_| Ok(())).is_err());

        remove_temp_root(root);
    }

    #[test]
    fn companion_digests_are_optional_and_verify_every_declared_asset() {
        let root = temp_root("generation-companions");
        let id = "0.170.0-3f7e1a";
        let generation = root.join(id);
        write_sample_generation(&generation, id, true);
        let descriptor = generation.join("generation.meta");
        let legacy = fs::read_to_string(&descriptor).unwrap().replace(
            "upstream_package_version\t1.2.3",
            "upstream_package_version\t0.170.0",
        );
        fs::write(&descriptor, &legacy).unwrap();
        let loaded_legacy = load_local_generation(&generation).unwrap();
        assert_eq!(loaded_legacy.manifest.upstream_package_version, "0.170.0");
        assert!(loaded_legacy.manifest.companion_digests.is_empty());
        verify_generation_asset_digests(&loaded_legacy).unwrap();

        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let names = [
            "bin/codex-code-mode-host",
            "bin/codex-exec-server",
            "bin/extra",
        ];
        let mut with_companions = legacy;
        for name in names {
            let path = generation.join(name);
            fs::write(&path, b"abc").unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            use std::fmt::Write as _;
            writeln!(with_companions, "companion\t{name}\t{expected}").unwrap();
        }
        fs::write(&descriptor, &with_companions).unwrap();
        let loaded = load_local_generation(&generation).unwrap();
        assert_eq!(loaded.manifest.companion_digests.len(), names.len());
        verify_generation_asset_digests(&loaded).unwrap();

        for name in names {
            let path = generation.join(name);
            fs::write(&path, b"modified").unwrap();
            assert!(
                matches!(
                    verify_generation_asset_digests(&loaded),
                    Err(LocalProductError::Descriptor(
                        "generation companion SHA-256 digest does not match generation.meta"
                    ))
                ),
                "{name}"
            );
            fs::write(path, b"abc").unwrap();
        }
        let extra = generation.join("bin/extra");
        fs::set_permissions(&extra, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            verify_generation_asset_digests(&loaded),
            Err(LocalProductError::Descriptor(
                "generation companion is not an executable regular file"
            ))
        ));
        fs::remove_file(&extra).unwrap();
        assert!(matches!(
            verify_generation_asset_digests(&loaded),
            Err(LocalProductError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound
        ));
        std::os::unix::fs::symlink("codex-exec-server", &extra).unwrap();
        assert!(matches!(
            verify_generation_asset_digests(&loaded),
            Err(LocalProductError::Descriptor(
                "generation companion is not an executable regular file"
            ))
        ));
        remove_temp_root(root);
    }

    #[test]
    fn companion_manifest_rejects_duplicate_or_malformed_rows() {
        let root = temp_root("generation-companion-rows");
        let generation = root.join("gen-1");
        write_sample_generation(&generation, "gen-1", false);
        let descriptor = generation.join("generation.meta");
        let legacy = fs::read_to_string(&descriptor).unwrap();
        let valid_digest = "a".repeat(64);
        let row = format!("companion\tbin/extra\t{valid_digest}\n");
        for name in [
            "bin/codex",
            "bin/.",
            "bin/..",
            "bin/",
            "bin/sub/program",
            "bin//program",
            "/bin/program",
            "./bin/program",
            "other/program",
            "bin/back\\slash",
            "bin/bad\0name",
        ] {
            let invalid = format!("{legacy}companion\t{name}\t{valid_digest}\n");
            fs::write(&descriptor, invalid).unwrap();
            assert!(load_local_generation(&generation).is_err(), "{name:?}");
        }
        for trailing in [
            row.repeat(2),
            format!("{row}junk\n"),
            format!("{row}\n"),
            format!("companion\tbin/extra\t{valid_digest}\tjunk\n"),
            format!("companion\tbin/extra\t{}\n", valid_digest.to_uppercase()),
            format!("companion\tbin/extra\t{}\n", "z".repeat(64)),
            "companion\tbin/extra\t1234\n".to_owned(),
            "companion\tbin/extra\n".to_owned(),
        ] {
            fs::write(&descriptor, format!("{legacy}{trailing}")).unwrap();
            assert!(load_local_generation(&generation).is_err(), "{trailing:?}");
        }
        remove_temp_root(root);
    }

    #[test]
    fn absent_initial_pointer_still_recovers_a_published_journal() {
        let root = temp_root("initial-journal");
        let roots = make_test_roots(&root);
        let paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        let after = generation_state::plan_initial_pointer_state("first").unwrap();
        let journal = generation_state::ActivationJournal {
            operation: generation_state::ActivationOperation::Legacy,
            before: None,
            after,
        };
        fs::write(
            &paths.activation_journal,
            generation_state::encode_activation_journal(&journal).unwrap(),
        )
        .unwrap();
        fs::remove_dir(&roots.generation_root).unwrap();
        let first = load_activated_generation(&roots).unwrap_err();
        assert!(matches!(first, LocalProductError::NoCurrentGeneration));
        assert!(!paths.activation_journal.exists());
        assert!(!paths.activation_state.exists());
        let second = load_activated_generation(&roots).unwrap_err();
        assert!(matches!(second, LocalProductError::NoCurrentGeneration));
        remove_temp_root(root);
    }

    #[test]
    fn load_activated_generation_reconciles_standalone_current_and_rejects_non_symlink() {
        let _guard = EnvRestoreGuard::new(&[]);
        let root = temp_root("reconcile-standalone-current");
        let roots = make_test_roots(&root);
        fs::create_dir_all(&roots.generation_root).unwrap();
        let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();

        write_sample_generation(&roots.generation_root.join("gen-a"), "gen-a", false);
        write_sample_generation(&roots.generation_root.join("gen-b"), "gen-b", false);

        let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
        let mut io = generation_state::FsActivationIo;
        let sa = generation_state::plan_initial_pointer_state("gen-a").unwrap();
        generation_state::activate_pointer_state_locked(
            &state_paths,
            generation_state::ActivationOperation::Update,
            None,
            &sa,
            &mut io,
            &lock,
        )
        .unwrap();
        let sb = generation_state::plan_activation_pointer_state(&sa, "gen-b").unwrap();
        generation_state::activate_pointer_state_locked(
            &state_paths,
            generation_state::ActivationOperation::Update,
            Some(&sa),
            &sb,
            &mut io,
            &lock,
        )
        .unwrap();
        drop(lock);

        // Case 1: activation-state points to `gen-b`, while `standalone/current` still points to `gen-a`.
        let standalone_current = roots.generation_root.parent().unwrap().join("current");
        let target_a = Path::new("generations/gen-a");
        let target_b = Path::new("generations/gen-b");
        std::os::unix::fs::symlink(target_a, &standalone_current).unwrap();
        let (ptr, loaded) = load_activated_generation(&roots).unwrap();
        assert_eq!(ptr.current, "gen-b");
        assert_eq!(loaded.generation_id, "gen-b");
        assert_eq!(fs::read_link(&standalone_current).unwrap(), target_b);

        // Another writer may acquire the lock after the pointer read and
        // before reconciliation of the now-missing compatibility link.
        fs::remove_file(&standalone_current).unwrap();
        let state_paths = state_paths.with_generation_root(&roots.generation_root);
        let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            drop(lock);
        });
        let (_, loaded2) =
            reconcile_standalone_current_and_load(&roots, &state_paths, ptr).unwrap();
        release.join().unwrap();
        assert_eq!(loaded2.generation_id, "gen-b");
        assert_eq!(fs::read_link(&standalone_current).unwrap(), target_b);

        // Case 3: regular file or directory at `standalone/current` fails closed without overwriting.
        fs::remove_file(&standalone_current).unwrap();
        fs::write(&standalone_current, b"corrupt-non-symlink").unwrap();
        let err = load_activated_generation(&roots).unwrap_err();
        assert!(matches!(
            err,
            LocalProductError::State(
                generation_state::ActivationTransactionError::UnsafeFileType(_)
            )
        ));
        assert_eq!(
            fs::read(&standalone_current).unwrap(),
            b"corrupt-non-symlink"
        );

        fs::remove_file(&standalone_current).unwrap();
        fs::create_dir(&standalone_current).unwrap();
        let err_dir = load_activated_generation(&roots).unwrap_err();
        assert!(matches!(
            err_dir,
            LocalProductError::State(
                generation_state::ActivationTransactionError::UnsafeFileType(_)
            )
        ));
        assert!(standalone_current.is_dir());

        remove_temp_root(root);
    }
}
