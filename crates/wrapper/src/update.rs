use crate::daemon_sync;
use crate::error::LocalProductError;
use crate::fs_util::{
    copy_local_directory_tree, ensure_real_directory_tree, read_bounded_regular, rename_noreplace,
    sync_generation_directory, LOCAL_STAGING_COUNTER,
};
use crate::generation::{
    load_local_generation, probe_release_candidate, sha256_regular_file, valid_sha256_hex,
    verify_generation_asset_digests, GenerationManifest, LoadedLocalGeneration, LocalWrapperRoots,
    PERSISTENT_SCHEMA_IDENTITY, WRAPPER_API_IDENTITY,
};
use crate::maintenance;
use crate::progress::{format_bytes, prompt_update_confirmation, UpdatePresentation};
use crate::remote_download::{self, ReleaseFileEntry, REMOTE_TRANSFER_TIMEOUT_SECONDS};
use crate::runtime_env::{
    capture_termux_process_env, prepare_wrapper_config, TermuxProcessEnvSnapshot,
};
use crate::state::generation_state::{self, ActivationOperation};
use crate::upstream_metadata::{
    reject_changed_archive, resolve_official_release_metadata,
    resolve_official_release_metadata_quick, valid_update_version, validate_https_url,
    version_order, OfficialReleaseMetadata, UPDATE_VERSION_ENV, UPSTREAM_PACKAGE_ASSET,
    UPSTREAM_RELEASE_METADATA_BASE,
};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

const WRAPPER_CREATION_METADATA: &str = "codex-wrapper";
pub(super) const UPDATE_CHECK_CACHE_FILE: &str = "update-check.cache";
const UPDATE_CHECK_CACHE_HEADER: &str = "codex-update-check-v1";
pub(super) const UPDATE_CHECK_CACHE_TTL_SECS: u64 = 15 * 60;
const UPDATE_CHECK_CACHE_MAX_BYTES: u64 = 4096;
const DISABLE_UPDATE_PROMPT_ENV: &str = "CODEX_DISABLE_UPDATE_PROMPT";
const UPDATE_USAGE: &str = "usage: codex update [--upstream] [--version <MAJOR.MINOR.PATCH>] [--force-rebuild] | --check [--version <MAJOR.MINOR.PATCH>] [--json] | --path <FILE_OR_DIR> | --rollback | --help";

pub(super) fn print_update_usage() {
    println!("{UPDATE_USAGE}");
    println!("  default / --upstream      download official Codex package, verify SHA-256, adapt locally for Termux, and activate");
    println!("  --version <M.m.p>         target a specific official release version (with default/--upstream)");
    println!("  --force-rebuild           download and rebuild even when the active upstream release matches");
    println!("  --path <FILE_OR_DIR>      adapt an offline official package .tar.gz or stage a prepared generation directory");
    println!("  --rollback                atomically switch back to the previous generation");
    println!("  --check [--json]          inspect exact official release inputs without installing or changing state");
    println!("  --help                    show this help");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateAction {
    Help,
    Upstream {
        requested_version: Option<String>,
        force_rebuild: bool,
    },
    Check {
        version: Option<String>,
        json: bool,
    },
    Path(PathBuf),
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateCliError {
    Usage(&'static str),
    InvalidVersion,
}

impl std::fmt::Display for UpdateCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateCliError::Usage(msg) => write!(f, "{msg}\n{UPDATE_USAGE}"),
            UpdateCliError::InvalidVersion => {
                f.write_str("codex update: --version must be MAJOR.MINOR.PATCH")
            }
        }
    }
}

pub(super) fn parse_update_args(args: &[OsString]) -> Result<UpdateAction, UpdateCliError> {
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        return Ok(UpdateAction::Help);
    }
    if args.len() == 1 && args[0] == OsStr::new("--rollback") {
        return Ok(UpdateAction::Rollback);
    }

    let mut upstream_flag = false;
    let mut force_rebuild = false;
    let mut check = false;
    let mut json = false;
    let mut version: Option<String> = None;
    let mut local_path: Option<PathBuf> = None;
    let mut idx = 0;
    while idx < args.len() {
        let Some(flag) = args[idx].to_str() else {
            return Err(UpdateCliError::Usage(
                "codex update: unsupported non-UTF-8 argument",
            ));
        };
        if flag == "--check" && !check {
            check = true;
            idx += 1;
        } else if flag == "--json" && !json {
            json = true;
            idx += 1;
        } else if flag == "--upstream" {
            if upstream_flag {
                return Err(UpdateCliError::Usage(
                    "codex update: duplicate --upstream flag",
                ));
            }
            upstream_flag = true;
            idx += 1;
        } else if flag == "--force-rebuild" && !force_rebuild {
            force_rebuild = true;
            idx += 1;
        } else if flag == "--version" {
            if version.is_some() {
                return Err(UpdateCliError::Usage(
                    "codex update: duplicate --version flag",
                ));
            }
            let Some(next) = args.get(idx + 1).and_then(|v| v.to_str()) else {
                return Err(UpdateCliError::Usage(
                    "codex update: --version requires a value",
                ));
            };
            if !valid_update_version(next) {
                return Err(UpdateCliError::InvalidVersion);
            }
            version = Some(next.to_owned());
            idx += 2;
        } else if let Some(v) = flag.strip_prefix("--version=") {
            if version.is_some() {
                return Err(UpdateCliError::Usage(
                    "codex update: duplicate --version flag",
                ));
            }
            if !valid_update_version(v) {
                return Err(UpdateCliError::InvalidVersion);
            }
            version = Some(v.to_owned());
            idx += 1;
        } else if flag == "--path" {
            if local_path.is_some() {
                return Err(UpdateCliError::Usage("codex update: duplicate --path flag"));
            }
            let Some(next) = args
                .get(idx + 1)
                .filter(|v| !v.is_empty() && *v != "--force-rebuild")
            else {
                return Err(UpdateCliError::Usage(
                    "codex update: --path requires a non-empty file or directory",
                ));
            };
            local_path = Some(PathBuf::from(next));
            idx += 2;
        } else if let Some(p) = args[idx]
            .to_str()
            .and_then(|s| s.strip_prefix("--path="))
            .filter(|s| !s.is_empty())
        {
            if local_path.is_some() {
                return Err(UpdateCliError::Usage("codex update: duplicate --path flag"));
            }
            local_path = Some(PathBuf::from(p));
            idx += 1;
        } else {
            return Err(UpdateCliError::Usage("codex update: unsupported option"));
        }
    }

    if force_rebuild && (local_path.is_some() || check) {
        return Err(UpdateCliError::Usage(
            "codex update: --force-rebuild requires an upstream update",
        ));
    }
    if let Some(path) = local_path {
        if upstream_flag || version.is_some() || check || json {
            return Err(UpdateCliError::Usage(
                "codex update: --path cannot be combined with --upstream or --version",
            ));
        }
        return Ok(UpdateAction::Path(path));
    }

    if json && !check {
        return Err(UpdateCliError::Usage(
            "codex update: --json requires --check",
        ));
    }
    if check {
        return Ok(UpdateAction::Check { version, json });
    }

    Ok(UpdateAction::Upstream {
        requested_version: version,
        force_rebuild,
    })
}

fn generation_compatible_with_wrapper(manifest: &GenerationManifest) -> bool {
    manifest.wrapper_api_identity == WRAPPER_API_IDENTITY
        && manifest.persistent_schema_identity == PERSISTENT_SCHEMA_IDENTITY
        && manifest.patch_policy_id == codex_release_builder::PATCH_POLICY_ID
}

fn check_release(
    roots: &LocalWrapperRoots,
    version: Option<&str>,
    json: bool,
) -> Result<String, LocalProductError> {
    let staging = create_staging_root(&std::env::temp_dir(), ".codex-check-")?;
    let result = (|| {
        let (metadata, url) = resolve_official_release_metadata(roots, &staging, version)?;
        let descriptor_sha = sha256_regular_file(&staging.join("upstream-release.json"))?;
        let wrapper_sha = sha256_regular_file(&running_wrapper_artifact()?)?;
        // Inspection never recovers a journal or takes an update/pruning lease.
        let paths = generation_state::WrapperStatePaths::new(&roots.state_root)
            .map_err(LocalProductError::StateFormat)?;
        let state =
            generation_state::read_pointer_state(&paths).map_err(LocalProductError::State)?;
        let installed = state
            .map(|state| load_local_generation(&roots.generation_root.join(state.current)))
            .transpose()?;
        if let Some(old) = installed.as_ref() {
            reject_changed_archive(
                &old.manifest.upstream_package_version,
                &old.manifest.source_artifact_digest,
                &metadata,
            )?;
        }
        let order = installed
            .as_ref()
            .map(|old| version_order(&metadata.version, &old.manifest.upstream_package_version))
            .transpose()?;
        let status = match installed.as_ref() {
            Some(_) if order.is_some_and(|order| order.is_lt()) => "older",
            Some(old)
                if old.manifest.upstream_package_version == metadata.version
                    && old.manifest.source_artifact_digest == metadata.package_sha256
                    && generation_compatible_with_wrapper(&old.manifest) =>
            {
                "current"
            }
            _ => "available",
        };
        let archive_url = format!(
            "{UPSTREAM_RELEASE_METADATA_BASE}/{}/{UPSTREAM_PACKAGE_ASSET}",
            metadata.version
        );
        if json {
            let installed_version = installed.as_ref().map_or("null".to_owned(), |old| {
                format!("\"{}\"", old.manifest.upstream_package_version)
            });
            Ok(format!("{{\"schema_version\":1,\"status\":\"{status}\",\"upstream_version\":\"{}\",\"archive_url\":\"{archive_url}\",\"archive_sha256\":\"{}\",\"metadata_url\":\"{url}\",\"metadata_sha256\":\"{descriptor_sha}\",\"wrapper_sha256\":\"{wrapper_sha}\",\"installed_version\":{installed_version}}}\n", metadata.version, metadata.package_sha256))
        } else {
            Ok(format!(
                "Codex {}: {status}\narchive: {archive_url}\nsha256: {}\n",
                metadata.version, metadata.package_sha256
            ))
        }
    })();
    std::fs::remove_dir_all(staging).map_err(|source| LocalProductError::Io {
        operation: "remove release inspection temporary directory",
        source,
    })?;
    result
}

fn local_update_generation_id(
    roots: &LocalWrapperRoots,
    version: &str,
    archive_sha256: &str,
) -> String {
    let short: String = archive_sha256.chars().take(6).collect();
    let short = if short.len() == 6 {
        short
    } else {
        "000000".to_owned()
    };
    let base = format!("{version}-{short}");
    if std::fs::symlink_metadata(roots.generation_root.join(&base)).is_err() {
        return base;
    }
    for seq in 2u64.. {
        let candidate = format!("{base}-{seq}");
        if std::fs::symlink_metadata(roots.generation_root.join(&candidate)).is_err() {
            return candidate;
        }
    }
    base
}

fn running_wrapper_artifact() -> Result<PathBuf, LocalProductError> {
    #[cfg(test)]
    if let Some(input) = std::env::var_os("CODEX_SOURCE_ACCEPTANCE_CORE") {
        return std::fs::canonicalize(input).map_err(|source| LocalProductError::Io {
            operation: "resolve test wrapper artifact",
            source,
        });
    }
    let input = std::env::current_exe().map_err(|source| LocalProductError::Io {
        operation: "resolve running wrapper binary",
        source,
    })?;
    std::fs::canonicalize(input).map_err(|source| LocalProductError::Io {
        operation: "resolve running wrapper binary",
        source,
    })
}

fn create_staging_root(parent: &Path, prefix: &str) -> Result<PathBuf, LocalProductError> {
    use std::os::unix::fs::DirBuilderExt as _;
    ensure_real_directory_tree(
        parent,
        "prepare wrapper staging parent",
        "wrapper staging parent must be a real directory",
    )?;
    for _ in 0..32 {
        let sequence = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        let path = parent.join(format!("{prefix}{}-{sequence}", std::process::id()));
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "create private update staging directory",
                    source,
                })
            }
        }
    }
    Err(LocalProductError::LocalUpdate(
        "could not allocate a unique update staging directory",
    ))
}

fn sync_tree_recursive(root: &Path) -> Result<(), LocalProductError> {
    let entries = std::fs::read_dir(root).map_err(|source| LocalProductError::Io {
        operation: "read generation directory for sync",
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "read generation entry for sync",
            source,
        })?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).map_err(|source| LocalProductError::Io {
            operation: "inspect generation entry for sync",
            source,
        })?;
        if meta.file_type().is_dir() {
            sync_tree_recursive(&path)?;
        } else if meta.file_type().is_file() {
            std::fs::File::open(&path)
                .and_then(|file| file.sync_all())
                .map_err(|source| LocalProductError::Io {
                    operation: "sync generation file",
                    source,
                })?;
        }
    }
    sync_generation_directory(root)
}

#[derive(Debug)]
struct ActivatedOutcome {
    generation_id: String,
    previous_version: Option<String>,
    current_version: String,
}

fn ensure_writable_roots(roots: &LocalWrapperRoots) -> Result<(), LocalProductError> {
    ensure_real_directory_tree(
        &roots.state_root,
        "prepare wrapper state directory",
        "wrapper state directory must be a real directory",
    )?;
    ensure_real_directory_tree(
        &roots.config_dir,
        "prepare wrapper config directory",
        "wrapper config directory must be a real directory",
    )?;
    ensure_real_directory_tree(
        &roots.generation_root,
        "prepare wrapper generation root",
        "wrapper generation root must be a real directory",
    )?;
    Ok(())
}

fn activation_paths(
    roots: &LocalWrapperRoots,
) -> Result<generation_state::WrapperStatePaths, LocalProductError> {
    generation_state::WrapperStatePaths::new(&roots.state_root)
        .map(|paths| paths.with_generation_root(&roots.generation_root))
        .map_err(LocalProductError::StateFormat)
}

fn activate_generation(
    paths: &generation_state::WrapperStatePaths,
    operation: ActivationOperation,
    before: Option<&generation_state::GenerationPointerState>,
    after: &generation_state::GenerationPointerState,
    lock: &generation_state::ActivationLockGuard,
    presentation: &mut UpdatePresentation,
) -> Result<(), LocalProductError> {
    use generation_state::ActivationTransactionError::{CommitSync, CommittedCleanup, Io};

    let mut io = generation_state::FsActivationIo;
    match generation_state::activate_pointer_state_locked(
        paths, operation, before, after, &mut io, lock,
    ) {
        Ok(()) => Ok(()),
        Err(err @ CommittedCleanup { .. }) => {
            presentation.warn(&format!("codex update: warning: {err}"));
            Ok(())
        }
        Err(err @ Io { .. }) => {
            // A link operation may have changed the filesystem before failing.
            // Repair it from the journal while still holding the writer lock.
            match generation_state::recover_activation_locked(paths, &mut io, lock) {
                Ok((state, _)) if state.as_ref() == Some(after) => {
                    presentation.warn(&format!(
                        "codex update: warning: activation recovered after: {err}"
                    ));
                    Ok(())
                }
                Ok(_) => Err(LocalProductError::State(err)),
                Err(recovery @ CommitSync(_)) => Err(LocalProductError::State(recovery)),
                Err(recovery) => {
                    presentation.warn(&format!(
                        "codex update: activation recovery is pending: {recovery}"
                    ));
                    Err(LocalProductError::State(err))
                }
            }
        }
        Err(err) => Err(LocalProductError::State(err)),
    }
}

fn current_loaded_generation(
    roots: &LocalWrapperRoots,
) -> Result<
    (
        Option<generation_state::GenerationPointerState>,
        Option<LoadedLocalGeneration>,
    ),
    LocalProductError,
> {
    let state_paths = activation_paths(roots)?;
    let lock = generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let (before, _) = generation_state::recover_activation_locked(
        &state_paths,
        &mut generation_state::FsActivationIo,
        &lock,
    )
    .map_err(LocalProductError::State)?;
    state_paths
        .read_standalone_current()
        .map_err(LocalProductError::State)?;
    drop(lock);
    // A forward update can replace missing or damaged generation files. The
    // independently recovered pointer remains the activation transaction's base.
    let loaded = before.as_ref().and_then(|state| {
        load_local_generation(&roots.generation_root.join(&state.current))
            .ok()
            .filter(|loaded| loaded.generation_id == state.current)
    });
    Ok((before, loaded))
}

fn install_built_generation_dir(
    built_dir: &Path,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    before: Option<&generation_state::GenerationPointerState>,
    before_loaded: Option<&LoadedLocalGeneration>,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    let loaded = load_local_generation(built_dir)?;
    let generation_id = loaded.generation_id.clone();
    let version = loaded.manifest.upstream_package_version.clone();

    ensure_writable_roots(roots)?;
    let final_dir = roots.generation_root.join(&generation_id);
    let reuse_existing = match std::fs::symlink_metadata(&final_dir) {
        Ok(meta) if meta.is_dir() => true,
        Ok(_) => return Err(LocalProductError::GenerationCollision),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect destination generation path",
                source,
            })
        }
    };

    let candidate_parent = roots.generation_root.clone();
    let candidate_seq = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let candidate_dir =
        candidate_parent.join(format!(".candidate-{}-{candidate_seq}", std::process::id()));

    let result = (|| -> Result<ActivatedOutcome, LocalProductError> {
        let staged_loaded = if reuse_existing {
            // An interrupted update can leave its complete generation in the
            // catalog. Reuse an exact, verified admission without rewriting it.
            let existing = load_local_generation(&final_dir)?;
            if existing.generation_id != generation_id
                || existing.generation_layout != loaded.generation_layout
                || existing.manifest != loaded.manifest
            {
                return Err(LocalProductError::GenerationCollision);
            }
            sync_tree_recursive(&final_dir)?;
            verify_generation_asset_digests(&loaded)?;
            existing
        } else {
            copy_local_directory_tree(built_dir, &candidate_dir)?;
            sync_tree_recursive(&candidate_dir)?;
            load_local_generation(&candidate_dir)?
        };

        presentation.transient(&format!(
            "Verifying Codex {version} assets & candidate runtime..."
        ));
        verify_generation_asset_digests(&staged_loaded)?;
        prepare_wrapper_config(&roots.config_dir).map_err(|source| LocalProductError::Io {
            operation: "prepare wrapper config.toml",
            source,
        })?;
        probe_release_candidate(&staged_loaded, roots, process_env)?;

        presentation.transient(&format!("Activating Codex {version}..."));
        let state_paths = activation_paths(roots)?;
        generation_state::prepare_wrapper_state_paths(&state_paths)
            .map_err(LocalProductError::State)?;
        let lock = generation_state::acquire_activation_lock(&state_paths)
            .map_err(LocalProductError::State)?;
        let authoritative =
            generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
        if authoritative.as_ref() != before {
            return Err(LocalProductError::State(
                generation_state::ActivationTransactionError::StaleAuthoritativeState,
            ));
        }
        if generation_state::has_pending_activation(&state_paths)
            .map_err(LocalProductError::State)?
        {
            return Err(LocalProductError::State(
                generation_state::ActivationTransactionError::PendingJournal,
            ));
        }
        state_paths
            .read_standalone_current()
            .map_err(LocalProductError::State)?;
        if !reuse_existing {
            rename_noreplace(&candidate_dir, &final_dir).map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    LocalProductError::GenerationCollision
                } else {
                    LocalProductError::Io {
                        operation: "commit generation directory",
                        source,
                    }
                }
            })?;
        }
        sync_generation_directory(&roots.generation_root)?;

        if authoritative
            .as_ref()
            .is_some_and(|state| state.current == generation_id)
        {
            let mut io = generation_state::FsActivationIo;
            generation_state::sync_standalone_current(
                &state_paths,
                Some(&generation_id),
                None,
                &mut io,
                &lock,
            )
            .map_err(LocalProductError::State)?;
            return Ok(ActivatedOutcome {
                generation_id,
                previous_version: None,
                current_version: version,
            });
        }
        let after = match authoritative.as_ref() {
            Some(prev) => generation_state::plan_activation_pointer_state(prev, &generation_id),
            None => generation_state::plan_initial_pointer_state(&generation_id),
        }
        .map_err(LocalProductError::StateFormat)?;
        activate_generation(
            &state_paths,
            ActivationOperation::Update,
            authoritative.as_ref(),
            &after,
            &lock,
            presentation,
        )?;

        Ok(ActivatedOutcome {
            generation_id,
            previous_version: before_loaded.map(|l| l.manifest.upstream_package_version.clone()),
            current_version: version,
        })
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&candidate_dir);
    }
    result
}

fn gzip_tool_path(roots: &LocalWrapperRoots) -> Result<PathBuf, LocalProductError> {
    roots
        .curl
        .parent()
        .ok_or(LocalProductError::LocalUpdate(
            "Termux tool directory is unavailable",
        ))
        .map(|bin| bin.join("gzip"))
}

struct ArchiveBuildInputs<'a> {
    archive_path: &'a Path,
    version: &'a str,
    archive_sha256: &'a str,
    staging_root: &'a Path,
}

fn build_from_archive_and_activate(
    inputs: ArchiveBuildInputs<'_>,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    before: Option<&generation_state::GenerationPointerState>,
    before_loaded: Option<&LoadedLocalGeneration>,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    if !codex_release_builder::binary_adaptation_available(inputs.version) {
        return Err(LocalProductError::LocalUpdate(
            "upstream Codex version format is invalid",
        ));
    }
    let wrapper = running_wrapper_artifact()?;
    let generation_id = local_update_generation_id(roots, inputs.version, inputs.archive_sha256);
    let built_dir = inputs.staging_root.join("built-generation");
    let gzip = gzip_tool_path(roots)?;
    presentation.transient(&format!(
        "Building Termux generation for Codex {}...",
        inputs.version
    ));
    let version_label = inputs.version;
    let mut patch_progress = |stage: &str| {
        presentation.transient(&format!("Codex {version_label}: {stage}"));
    };
    codex_release_builder::build_generation_with_progress(
        &codex_release_builder::GenerationRequest {
            version: inputs.version.to_owned(),
            archive: inputs.archive_path.to_path_buf(),
            archive_sha256: inputs.archive_sha256.to_owned(),
            generation_id,
            wrapper,
            creation_metadata: WRAPPER_CREATION_METADATA.to_owned(),
            gzip,
            openssl: None,
            output: built_dir.clone(),
        },
        &mut patch_progress,
    )
    .map_err(LocalProductError::Builder)?;
    install_built_generation_dir(
        &built_dir,
        roots,
        process_env,
        before,
        before_loaded,
        presentation,
    )
}

enum UpstreamOutcome {
    AlreadyCurrent {
        generation_id: String,
        version: String,
    },
    Activated(ActivatedOutcome),
}

fn run_upstream_update(
    requested_version: Option<&str>,
    force_rebuild: bool,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
    confirmed: Option<&OfficialReleaseMetadata>,
) -> Result<UpstreamOutcome, LocalProductError> {
    ensure_writable_roots(roots)?;
    let (before, installed) = current_loaded_generation(roots)?;
    let staging_root = create_staging_root(&roots.state_root, ".local-update-")?;
    let result = (|| -> Result<UpstreamOutcome, LocalProductError> {
        presentation.transient("Checking official Codex release...");
        let exact_version = confirmed
            .map(|release| release.version.as_str())
            .or(requested_version);
        let (metadata, _) = resolve_official_release_metadata(roots, &staging_root, exact_version)?;
        if confirmed.is_some_and(|release| *release != metadata) {
            return Err(LocalProductError::LocalUpdate(
                "official descriptor changed since the startup prompt; run codex update to check again",
            ));
        }
        let unpinned =
            requested_version.is_none() && std::env::var_os(UPDATE_VERSION_ENV).is_none();
        if unpinned {
            write_update_check_cache(
                &roots.state_root,
                &UpdateCheckCacheEntry {
                    checked_at: current_unix_secs(),
                    release: metadata.clone(),
                    deferred_for: None,
                },
            );
        }

        if let Some(current) = installed.as_ref() {
            reject_changed_archive(
                &current.manifest.upstream_package_version,
                &current.manifest.source_artifact_digest,
                &metadata,
            )?;
            if (unpinned || confirmed.is_some())
                && version_order(
                    &metadata.version,
                    &current.manifest.upstream_package_version,
                )?
                .is_lt()
            {
                return Err(LocalProductError::LocalUpdate("official discovery would downgrade the installed version; use an explicit --version if intended"));
            }
        }
        let before_loaded = installed
            .as_ref()
            .filter(|current| verify_generation_asset_digests(current).is_ok());
        if !force_rebuild {
            if let Some(current) = before_loaded.filter(|current| {
                current.manifest.upstream_package_version == metadata.version
                    && current.manifest.source_artifact_digest == metadata.package_sha256
                    && generation_compatible_with_wrapper(&current.manifest)
            }) {
                return Ok(UpstreamOutcome::AlreadyCurrent {
                    generation_id: current.generation_id.clone(),
                    version: metadata.version,
                });
            }
        }

        let archive_path = staging_root.join(UPSTREAM_PACKAGE_ASSET);
        let download_url = format!(
            "{UPSTREAM_RELEASE_METADATA_BASE}/{}/{UPSTREAM_PACKAGE_ASSET}",
            metadata.version
        );
        validate_https_url(&download_url)?;
        presentation.transient(&format!(
            "Downloading official Codex {} package...",
            metadata.version
        ));
        let archive_entry = ReleaseFileEntry {
            relative_path: UPSTREAM_PACKAGE_ASSET.to_owned(),
            sha256: metadata.package_sha256.clone(),
            mode: 0o600,
        };
        let mut last_bytes = u64::MAX;
        let mut last_total: Option<u64> = None;
        let version_label = metadata.version.as_str();
        let mut progress = |bytes: u64, total: Option<u64>| {
            if bytes != last_bytes || total != last_total {
                last_bytes = bytes;
                last_total = total;
                let msg = match total {
                    Some(t) if t > 0 => {
                        let pct = (bytes.saturating_mul(100) / t).min(100);
                        format!(
                            "Downloading Codex {version_label} ({} / {}, {pct}%)...",
                            format_bytes(bytes),
                            format_bytes(t),
                        )
                    }
                    _ => format!(
                        "Downloading Codex {version_label} ({})...",
                        format_bytes(bytes)
                    ),
                };
                presentation.transient(&msg);
            }
        };
        remote_download::payload(
            roots,
            &download_url,
            &archive_path,
            &archive_entry,
            remote_download::PayloadOptions {
                limit: codex_release_builder::ARCHIVE_MAX_BYTES,
                force_download: force_rebuild,
                expected_size: None,
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: Some(&mut progress),
            },
        )?;
        let activated = build_from_archive_and_activate(
            ArchiveBuildInputs {
                archive_path: &archive_path,
                version: &metadata.version,
                archive_sha256: &metadata.package_sha256,
                staging_root: &staging_root,
            },
            roots,
            process_env,
            before.as_ref(),
            before_loaded,
            presentation,
        )?;
        Ok(UpstreamOutcome::Activated(activated))
    })();
    let _ = std::fs::remove_dir_all(&staging_root);
    result
}

fn resolve_cli_path(input: &Path) -> Result<PathBuf, LocalProductError> {
    let abs = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| LocalProductError::Io {
                operation: "resolve current directory",
                source,
            })?
            .join(input)
    };
    Ok(abs)
}

fn run_path_update(
    input_path: &Path,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    ensure_writable_roots(roots)?;
    let source_path = resolve_cli_path(input_path)?;
    let source_meta =
        std::fs::symlink_metadata(&source_path).map_err(|source| LocalProductError::Io {
            operation: "inspect local package path",
            source,
        })?;
    let (before, loaded) = current_loaded_generation(roots)?;
    let before_loaded = loaded
        .as_ref()
        .filter(|current| verify_generation_asset_digests(current).is_ok());

    if source_meta.file_type().is_dir() {
        presentation.transient("Inspecting local generation directory...");
        let descriptor = source_path.join("generation.meta");
        if !descriptor.exists() {
            return Err(LocalProductError::LocalUpdate(
                "local directory must be a prepared generation directory containing generation.meta (or pass a .tar.gz upstream package archive)",
            ));
        }
        return install_built_generation_dir(
            &source_path,
            roots,
            process_env,
            before.as_ref(),
            before_loaded,
            presentation,
        );
    }

    if !source_meta.file_type().is_file() {
        return Err(LocalProductError::UnsafeSource(
            "local package path must be a regular non-symlink file or generation directory",
        ));
    }
    if source_meta.len() == 0 || source_meta.len() > codex_release_builder::ARCHIVE_MAX_BYTES {
        return Err(LocalProductError::LocalUpdate(
            "local package archive exceeds its byte bound",
        ));
    }

    presentation.transient("Inspecting local Codex archive...");
    let gzip = gzip_tool_path(roots)?;
    let version = codex_release_builder::inspect_package_archive_version(&source_path, &gzip)
        .map_err(LocalProductError::Builder)?;
    if !codex_release_builder::binary_adaptation_available(&version) {
        return Err(LocalProductError::LocalUpdate(
            "local package archive version format is invalid",
        ));
    }

    let staging_root = create_staging_root(&roots.state_root, ".local-update-")?;
    let result = (|| {
        let staged_archive = staging_root.join(UPSTREAM_PACKAGE_ASSET);
        std::fs::copy(&source_path, &staged_archive).map_err(|source| LocalProductError::Io {
            operation: "stage local package archive",
            source,
        })?;
        let sha256 = sha256_regular_file(&staged_archive)?;
        build_from_archive_and_activate(
            ArchiveBuildInputs {
                archive_path: &staged_archive,
                version: &version,
                archive_sha256: &sha256,
                staging_root: &staging_root,
            },
            roots,
            process_env,
            before.as_ref(),
            before_loaded,
            presentation,
        )
    })();
    let _ = std::fs::remove_dir_all(&staging_root);
    result
}

fn run_rollback(
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> Result<ActivatedOutcome, LocalProductError> {
    let state_paths = activation_paths(roots)?;
    let lock = generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let (before, committed) = generation_state::recover_activation_locked(
        &state_paths,
        &mut generation_state::FsActivationIo,
        &lock,
    )
    .map_err(LocalProductError::State)?;
    let before = before.ok_or(LocalProductError::NoCurrentGeneration)?;
    state_paths
        .read_standalone_current()
        .map_err(LocalProductError::State)?;
    drop(lock);
    if let Some(journal) = committed {
        if journal.operation == ActivationOperation::Legacy && journal.is_rollback_transition() {
            return Err(LocalProductError::LocalUpdate(
                "recovered an older activation journal with ambiguous operation; inspect codex wrapper status before retrying",
            ));
        }
        if journal.operation == ActivationOperation::Rollback {
            // The previous rollback published its state but stopped before removing
            // the journal. Recovery has finished that operation, so do not swap again.
            let version = |id: &str| {
                load_local_generation(&roots.generation_root.join(id))
                    .map(|loaded| loaded.manifest.upstream_package_version)
                    .unwrap_or_else(|_| id.to_owned())
            };
            return Ok(ActivatedOutcome {
                current_version: version(&before.current),
                previous_version: before.previous.as_deref().map(version),
                generation_id: before.current,
            });
        }
    }
    let after = generation_state::plan_rollback_pointer_state(&before)
        .map_err(LocalProductError::StateFormat)?;

    // Do not let a damaged current generation prevent rolling back to a healthy previous one.
    let previous_version = load_local_generation(&roots.generation_root.join(&before.current))
        .ok()
        .map(|current| current.manifest.upstream_package_version)
        .unwrap_or_else(|| before.current.clone());

    let target_loaded = load_local_generation(&roots.generation_root.join(&after.current))?;
    if target_loaded.generation_id != after.current {
        return Err(LocalProductError::Descriptor(
            "rollback generation descriptor id does not match previous",
        ));
    }
    presentation.transient(&format!(
        "Verifying rollback target {}...",
        target_loaded.generation_id
    ));
    verify_generation_asset_digests(&target_loaded)?;
    prepare_wrapper_config(&roots.config_dir).map_err(|source| LocalProductError::Io {
        operation: "prepare wrapper config.toml",
        source,
    })?;
    probe_release_candidate(&target_loaded, roots, process_env)?;

    let lock = generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let authoritative =
        generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
    if authoritative.as_ref() != Some(&before) {
        return Err(LocalProductError::State(
            generation_state::ActivationTransactionError::StaleAuthoritativeState,
        ));
    }
    activate_generation(
        &state_paths,
        ActivationOperation::Rollback,
        Some(&before),
        &after,
        &lock,
        presentation,
    )?;
    Ok(ActivatedOutcome {
        generation_id: after.current,
        previous_version: Some(previous_version),
        current_version: target_loaded.manifest.upstream_package_version,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateCheckCacheEntry {
    checked_at: u64,
    release: OfficialReleaseMetadata,
    deferred_for: Option<String>,
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn read_update_check_cache(state_root: &Path, now_secs: u64) -> Option<UpdateCheckCacheEntry> {
    let bytes = read_bounded_regular(
        &state_root.join(UPDATE_CHECK_CACHE_FILE),
        UPDATE_CHECK_CACHE_MAX_BYTES,
        true,
        "invalid update-check.cache",
    )
    .ok()?;
    let mut lines = std::str::from_utf8(&bytes).ok()?.lines();
    if lines.next()? != UPDATE_CHECK_CACHE_HEADER {
        return None;
    }
    let checked_at: u64 = lines.next()?.strip_prefix("checked_at\t")?.parse().ok()?;
    let latest_version = lines.next()?.strip_prefix("latest_version\t")?;
    let archive_sha256 = lines.next()?.strip_prefix("archive_sha256\t")?;
    let deferred_for = match lines.next() {
        Some(line) => {
            let version = line.strip_prefix("deferred_for\t")?;
            if !valid_update_version(version) {
                return None;
            }
            Some(version.to_owned())
        }
        None => None,
    };
    if lines.next().is_some()
        || !valid_update_version(latest_version)
        || !valid_sha256_hex(archive_sha256)
        || now_secs.checked_sub(checked_at)? >= UPDATE_CHECK_CACHE_TTL_SECS
    {
        return None;
    }
    Some(UpdateCheckCacheEntry {
        checked_at,
        release: OfficialReleaseMetadata {
            version: latest_version.to_owned(),
            package_sha256: archive_sha256.to_owned(),
        },
        deferred_for,
    })
}

fn write_update_check_cache(state_root: &Path, entry: &UpdateCheckCacheEntry) {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;

    if ensure_real_directory_tree(
        state_root,
        "prepare wrapper state directory for update cache",
        "wrapper state directory must be a real directory",
    )
    .is_err()
    {
        return;
    }
    let seq = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let tmp_path = state_root.join(format!(
        ".{UPDATE_CHECK_CACHE_FILE}.{}.{seq}.tmp",
        std::process::id()
    ));
    let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp_path)
    else {
        return;
    };
    let mut content = format!(
        "{UPDATE_CHECK_CACHE_HEADER}\nchecked_at\t{}\nlatest_version\t{}\narchive_sha256\t{}\n",
        entry.checked_at, entry.release.version, entry.release.package_sha256,
    );
    if let Some(version) = &entry.deferred_for {
        content.push_str(&format!("deferred_for\t{version}\n"));
    }
    let write_result = file
        .write_all(content.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if write_result.is_err()
        || std::fs::rename(&tmp_path, state_root.join(UPDATE_CHECK_CACHE_FILE)).is_err()
    {
        let _ = std::fs::remove_file(&tmp_path);
        return;
    }
    let _ = crate::fs_util::sync_directory(state_root);
}

fn read_active_installed_generation(roots: &LocalWrapperRoots) -> Option<LoadedLocalGeneration> {
    let paths = generation_state::WrapperStatePaths::new(&roots.state_root).ok()?;
    let pointer = generation_state::read_pointer_state(&paths).ok()??;
    let loaded = load_local_generation(&roots.generation_root.join(&pointer.current)).ok()?;
    (loaded.generation_id == pointer.current).then_some(loaded)
}

fn check_and_prompt_bare_launch<W: std::io::Write>(
    roots: &LocalWrapperRoots,
    now_secs: u64,
    err_out: &mut W,
    confirm: impl FnOnce(&mut W, &str) -> bool,
    fetch_latest: impl FnOnce(&LocalWrapperRoots) -> Option<OfficialReleaseMetadata>,
    run_update: impl FnOnce(&OfficialReleaseMetadata) -> i32,
) -> bool {
    let Some(installed) = read_active_installed_generation(roots) else {
        return false;
    };
    let current_version = installed.manifest.upstream_package_version;
    let mut entry = if let Some(cached) = read_update_check_cache(&roots.state_root, now_secs) {
        cached
    } else {
        let Some(release) = fetch_latest(roots) else {
            return false;
        };
        let entry = UpdateCheckCacheEntry {
            checked_at: now_secs,
            release,
            deferred_for: None,
        };
        write_update_check_cache(&roots.state_root, &entry);
        entry
    };
    let latest = &entry.release.version;
    if entry.deferred_for.as_deref() == Some(&current_version)
        || !matches!(
            version_order(latest, &current_version),
            Ok(std::cmp::Ordering::Greater)
        )
    {
        return false;
    }

    let prompt =
        format!("Codex {latest} is available (current: {current_version}). Update now? [y/N]: ");

    if confirm(err_out, &prompt) {
        let rc = run_update(&entry.release);
        if rc == 0 {
            return true;
        }
        let _ = writeln!(
            err_out,
            "codex: update failed (exit {rc}); continuing with the active generation."
        );
    }
    entry.deferred_for = Some(current_version);
    write_update_check_cache(&roots.state_root, &entry);
    false
}

pub(super) fn maybe_prompt_and_update_on_bare_launch(roots: &LocalWrapperRoots) {
    use std::io::IsTerminal as _;
    use std::os::fd::AsFd as _;

    if std::env::var_os(DISABLE_UPDATE_PROMPT_ENV).is_some()
        || std::env::var_os(UPDATE_VERSION_ENV).is_some()
        || !std::io::stdin().is_terminal()
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return;
    }
    // A buffered stdin reader can consume input intended for the exec'd TUI.
    let Ok(stdin) = std::io::stdin().as_fd().try_clone_to_owned() else {
        return;
    };
    let mut stdin = std::fs::File::from(stdin);
    let _ = check_and_prompt_bare_launch(
        roots,
        current_unix_secs(),
        &mut std::io::stderr(),
        |out, prompt| prompt_update_confirmation(&mut stdin, out, prompt),
        |roots| {
            let staging = create_staging_root(&std::env::temp_dir(), ".codex-quick-check-").ok()?;
            let resolved = resolve_official_release_metadata_quick(roots, &staging).ok();
            let _ = std::fs::remove_dir_all(&staging);
            resolved
        },
        |release| {
            run_update_action_confirmed(
                UpdateAction::Upstream {
                    requested_version: None,
                    force_rebuild: false,
                },
                Some(release),
            )
        },
    );
}

enum UpdateSuccess {
    Activated(ActivatedOutcome),
    AlreadyCurrent {
        generation_id: String,
        version: String,
    },
    RolledBack(ActivatedOutcome),
}

impl UpdateSuccess {
    fn generation_id(&self) -> &str {
        match self {
            UpdateSuccess::Activated(outcome) | UpdateSuccess::RolledBack(outcome) => {
                &outcome.generation_id
            }
            UpdateSuccess::AlreadyCurrent { generation_id, .. } => generation_id,
        }
    }

    fn finish_presentation(&self, presentation: &mut UpdatePresentation) {
        match self {
            UpdateSuccess::Activated(outcome) => presentation.finish_activated(
                outcome.previous_version.as_deref(),
                &outcome.current_version,
                &outcome.generation_id,
            ),
            UpdateSuccess::AlreadyCurrent {
                generation_id,
                version,
            } => presentation.finish_current(version, generation_id),
            UpdateSuccess::RolledBack(outcome) => presentation.finish_rollback(
                outcome
                    .previous_version
                    .as_deref()
                    .unwrap_or(&outcome.current_version),
                &outcome.current_version,
                &outcome.generation_id,
            ),
        }
    }
}

fn execute_mutating_update(
    action: UpdateAction,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
    confirmed: Option<&OfficialReleaseMetadata>,
) -> Result<UpdateSuccess, LocalProductError> {
    match action {
        UpdateAction::Rollback => {
            run_rollback(roots, process_env, presentation).map(UpdateSuccess::RolledBack)
        }
        UpdateAction::Path(path) => {
            run_path_update(&path, roots, process_env, presentation).map(UpdateSuccess::Activated)
        }
        UpdateAction::Upstream {
            requested_version,
            force_rebuild,
        } => match run_upstream_update(
            requested_version.as_deref(),
            force_rebuild,
            roots,
            process_env,
            presentation,
            confirmed,
        )? {
            UpstreamOutcome::AlreadyCurrent {
                generation_id,
                version,
            } => Ok(UpdateSuccess::AlreadyCurrent {
                generation_id,
                version,
            }),
            UpstreamOutcome::Activated(outcome) => Ok(UpdateSuccess::Activated(outcome)),
        },
        UpdateAction::Help | UpdateAction::Check { .. } => {
            unreachable!("non-mutating update actions handled before update lease")
        }
    }
}

fn print_daemon_sync_report(report: &daemon_sync::DaemonSyncReport) {
    for line in report.summary_lines() {
        println!("{line}");
    }
    for warning in report.warnings() {
        eprintln!("{warning}");
    }
}

pub(super) fn run_update_action(action: UpdateAction) -> i32 {
    run_update_action_confirmed(action, None)
}

fn run_update_action_confirmed(
    action: UpdateAction,
    confirmed: Option<&OfficialReleaseMetadata>,
) -> i32 {
    if matches!(action, UpdateAction::Help) {
        print_update_usage();
        return 0;
    }

    let mut presentation = UpdatePresentation::new();
    let roots = match LocalWrapperRoots::from_environment() {
        Ok(roots) => roots,
        Err(err) => {
            return presentation.fail(&err);
        }
    };
    if let UpdateAction::Check { version, json } = action {
        return match check_release(&roots, version.as_deref(), json) {
            Ok(output) => {
                print!("{output}");
                0
            }
            Err(err) => presentation.fail(&err),
        };
    }
    let _lease = match maintenance::UpdateLease::acquire(&roots) {
        Ok(lease) => lease,
        Err(err) => {
            return presentation.fail(&err);
        }
    };
    let process_env = capture_termux_process_env();

    match execute_mutating_update(action, &roots, &process_env, &mut presentation, confirmed) {
        Ok(success) => {
            let daemon_report = daemon_sync::synchronize(
                success.generation_id(),
                &roots,
                &process_env,
                &mut presentation,
            );
            success.finish_presentation(&mut presentation);
            print_daemon_sync_report(&daemon_report);
            0
        }
        Err(err) => presentation.fail(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::JsonParser as JsonCursor;
    use crate::tests::{remove_temp_root, temp_root, test_process_env, write_sample_generation};
    use crate::upstream_metadata::{parse_official_release_metadata, OfficialReleaseMetadata};
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    #[test]
    fn inspection_cli_is_read_only_and_conflicting_flags_are_rejected() {
        let args = |items: &[&str]| items.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            parse_update_args(&args(&["--check", "--json", "--version", "1.2.3"])).unwrap(),
            UpdateAction::Check {
                version: Some("1.2.3".into()),
                json: true
            }
        );
        assert_eq!(
            parse_update_args(&args(&["--force-rebuild"])).unwrap(),
            UpdateAction::Upstream {
                requested_version: None,
                force_rebuild: true,
            }
        );
        assert_eq!(
            parse_update_args(&args(&[
                "--upstream",
                "--version",
                "1.2.3",
                "--force-rebuild"
            ]))
            .unwrap(),
            UpdateAction::Upstream {
                requested_version: Some("1.2.3".into()),
                force_rebuild: true,
            }
        );
        for flags in [
            &["--force-rebuild", "--rollback"][..],
            &["--check", "--force-rebuild"],
            &["--path", "/offline", "--force-rebuild"],
            &["--path", "--force-rebuild"],
            &["--force-rebuild", "--force-rebuild"],
        ] {
            let mut words = vec![OsString::from("update")];
            words.extend(flags.iter().map(OsString::from));
            assert_eq!(crate::run_public_main(words), 2, "{flags:?}");
        }
        for words in [
            vec!["--json"],
            vec!["--check", "--check"],
            vec!["--check", "--rollback"],
            vec!["--check", "--path", "/tmp/archive"],
            vec!["--check", "--json", "--json"],
        ] {
            assert!(parse_update_args(&args(&words)).is_err(), "{words:?}");
        }
        assert!(version_order("1.10.0", "1.9.99").unwrap().is_gt());
        assert!(version_order("1.2", "1.2.3").is_err());
        assert!(version_order("1.0.0", "invalid").is_err());
        let discovered = OfficialReleaseMetadata {
            version: "1.2.3".into(),
            package_sha256: "a".repeat(64),
        };
        assert!(reject_changed_archive("1.2.3", &"b".repeat(64), &discovered).is_err());
        assert!(reject_changed_archive("1.2.3", &"a".repeat(64), &discovered).is_ok());
        assert!(reject_changed_archive("1.2.2", &"b".repeat(64), &discovered).is_ok());
    }

    fn metadata(digest: char) -> String {
        format!("{{\"tag_name\":\"rust-v1.2.3\",\"assets\":[{{\"name\":\"{UPSTREAM_PACKAGE_ASSET}\",\"digest\":\"sha256:{}\"}}]}}", digest.to_string().repeat(64))
    }

    #[test]
    fn official_metadata_rejects_prerelease_draft_and_duplicate_flags() {
        for extra in [
            "\"draft\":true,",
            "\"prerelease\":true,",
            "\"draft\":false,\"draft\":false,",
        ] {
            let raw = metadata('a').replacen('{', &format!("{{{extra}"), 1);
            assert!(
                parse_official_release_metadata(raw.as_bytes(), None).is_err(),
                "{extra}"
            );
        }
    }

    #[test]
    fn metadata_accepts_paired_surrogates_and_only_unrelated_null_digests() {
        let raw = metadata('a').replace(
            r#""assets":["#,
            r#""body":"Ready \uD83D\uDE80","assets":[{"digest":null,"name":"other"},"#,
        );
        let expected = parse_official_release_metadata(metadata('a').as_bytes(), None).unwrap();
        assert_eq!(
            parse_official_release_metadata(raw.as_bytes(), None).unwrap(),
            expected
        );
        assert_eq!(
            JsonCursor::new(br#""\uD800\uDC00\uDBFF\uDFFF""#)
                .string()
                .unwrap(),
            "\u{10000}\u{10ffff}"
        );
        for escape in [
            r"\uD83D",
            r"\uDE80",
            r"\uD83D\u0041",
            r"\uD83D\uD83D",
            r"\uD83D \uDE80",
        ] {
            let invalid = raw.replace(r"\uD83D\uDE80", escape);
            assert!(
                parse_official_release_metadata(invalid.as_bytes(), None).is_err(),
                "{escape}"
            );
        }
        for asset in [
            format!(r#"{{"name":"{UPSTREAM_PACKAGE_ASSET}","digest":null}}"#),
            format!(r#"{{"name":"{UPSTREAM_PACKAGE_ASSET}"}}"#),
        ] {
            let invalid = format!(r#"{{"tag_name":"rust-v1.2.3","assets":[{asset}]}}"#);
            assert!(parse_official_release_metadata(invalid.as_bytes(), None).is_err());
        }
        let duplicate = raw.replace(r#""digest":null"#, r#""digest":null,"digest":null"#);
        assert!(parse_official_release_metadata(duplicate.as_bytes(), None).is_err());
    }

    #[test]
    fn inspection_cross_checks_descriptors_without_installed_state_or_artifact_downloads() {
        let root = temp_root("release-check");
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let cat = if Path::new("/system/bin/cat").is_file() {
            "/system/bin/cat"
        } else {
            "/bin/cat"
        };
        let latest = root.join("latest.json");
        let exact = root.join("exact.json");
        fs::write(&latest, metadata('a')).unwrap();
        fs::write(&exact, metadata('a')).unwrap();
        let curl = root.join("curl");
        fs::write(&curl, format!("#!{shell}\nurl=\nmax_time=\nwhile [ $# -gt 0 ]; do\n case \"$1\" in\n  --url) shift; url=$1 ;;\n  --max-time) shift; max_time=$1 ;;\n esac\n shift\ndone\n[ \"$max_time\" = 30 ] || exit 58\ncase \"$url\" in\n *releases/latest) exec {cat} '{}' ;;\n */release.json) exec {cat} '{}' ;;\n '') exit 0 ;;\n *) exit 57 ;;\nesac\n", latest.display(), exact.display())).unwrap();
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o700)).unwrap();
        let state = root.join("state");
        let roots = LocalWrapperRoots {
            generation_root: root.join("generations"),
            state_root: state.clone(),
            config_dir: state.join("config"),
            resolver_path: state.join("resolv.conf"),
            curl,
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
        };
        let output = check_release(&roots, None, true).unwrap();
        assert!(output.contains("\"upstream_version\":\"1.2.3\""));
        assert!(output.contains("\"installed_version\":null"));
        assert!(!state.exists());
        assert!(!roots.generation_root.exists());
        fs::write(&exact, metadata('b')).unwrap();
        assert!(check_release(&roots, None, true).is_err());
        assert!(!state.exists());
        assert!(!roots.generation_root.exists());

        // When GitHub API returns 403 rate limit, discovery falls back to the
        // public latest release redirect and verifies OpenAI's exact descriptor.
        fs::write(&exact, metadata('a')).unwrap();
        fs::write(
            &roots.curl,
            format!(
                r#"#!{shell}
url=
while [ $# -gt 0 ]; do
 case "$1" in
  --url) shift; url=$1 ;;
 esac
 shift
done
case "$url" in
 https://api.github.com/*releases/latest)
  printf 'CODEX_HTTP_STATUS=403' >&2
  exit 22 ;;
 https://github.com/openai/codex/releases/latest)
  printf 'HTTP/2 302\r\nlocation: https://github.com/openai/codex/releases/tag/rust-v1.2.3\r\n\r\n' ;;
 */1.2.3/release.json)
  exec {cat} '{}' ;;
 *) exit 57 ;;
esac
"#,
                exact.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&roots.curl, fs::Permissions::from_mode(0o700)).unwrap();
        let fallback_out = check_release(&roots, None, true).unwrap();
        assert!(fallback_out.contains("\"upstream_version\":\"1.2.3\""));
        assert!(!state.exists());
        remove_temp_root(root);
    }

    fn setup_update_test_roots(root: &Path) -> LocalWrapperRoots {
        let state_root = root.join("state");
        let config_dir = state_root.join("config");
        let generation_root = root.join(".codex/packages/standalone/releases");
        crate::tests::create_private_test_dirs(&config_dir);
        fs::create_dir_all(&generation_root).unwrap();
        let resolver_path = root.join("resolv.conf");
        fs::write(&resolver_path, b"nameserver 1.1.1.1\n").unwrap();
        let cert_file = root.join("cert.pem");
        let cert_dir = root.join("certs");
        fs::write(&cert_file, b"").unwrap();
        fs::create_dir_all(&cert_dir).unwrap();
        let curl = root.join("prefix/bin/curl");
        LocalWrapperRoots {
            generation_root,
            state_root,
            config_dir,
            resolver_path,
            cert_file,
            cert_dir,
            curl,
        }
    }

    #[test]
    fn upstream_checks_compatibility_and_reaches_download_for_force_or_damage() {
        let root = temp_root("release-selection");
        let roots = setup_update_test_roots(&root);
        let env = crate::tests::test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();
        let source = root.join("prepared");
        let id = "1.2.3-aaaaaa";
        crate::tests::write_sample_generation(&source, id, false);
        run_path_update(&source, &roots, &env, &mut presentation).unwrap();

        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        fs::create_dir_all(roots.curl.parent().unwrap()).unwrap();
        fs::write(
            &roots.curl,
            format!(
                r#"#!{shell}
for url do :; done
case "$url" in
    */release.json) printf '%s' '{}' ;;
    */{UPSTREAM_PACKAGE_ASSET}) exit 57 ;;
    *) exit 58 ;;
esac
"#,
                metadata('a')
            ),
        )
        .unwrap();
        fs::set_permissions(&roots.curl, fs::Permissions::from_mode(0o700)).unwrap();
        let paths = activation_paths(&roots).unwrap();
        let pointer = fs::read(&paths.activation_state).unwrap();
        let output = check_release(&roots, Some("1.2.3"), true).unwrap();
        assert!(output.contains("\"status\":\"current\""));
        assert!(!output.contains(&format!("\"wrapper_sha256\":\"{}\"", "a".repeat(64))));
        let update = |force, presentation: &mut UpdatePresentation| {
            run_upstream_update(Some("1.2.3"), force, &roots, &env, presentation, None)
        };
        assert!(matches!(
            update(false, &mut presentation),
            Ok(UpstreamOutcome::AlreadyCurrent { .. })
        ));
        let assert_download = |force, presentation: &mut UpdatePresentation| {
            assert!(matches!(
                update(force, presentation),
                Err(LocalProductError::RemoteTransportFailed { code: Some(57), .. })
            ));
            assert_eq!(fs::read(&paths.activation_state).unwrap(), pointer);
        };
        assert_download(true, &mut presentation);

        let generation = roots.generation_root.join(id);
        let descriptor = generation.join("generation.meta");
        let compatible = fs::read_to_string(&descriptor).unwrap();
        for (field, identity) in [
            ("wrapper_api_identity", WRAPPER_API_IDENTITY),
            ("persistent_schema_identity", PERSISTENT_SCHEMA_IDENTITY),
            ("patch_policy_id", codex_release_builder::PATCH_POLICY_ID),
        ] {
            let incompatible = compatible.replace(
                &format!("{field}\t{identity}\n"),
                &format!("{field}\told-identity\n"),
            );
            fs::write(&descriptor, incompatible).unwrap();
            assert!(check_release(&roots, Some("1.2.3"), true)
                .unwrap()
                .contains("\"status\":\"available\""));
            assert_download(false, &mut presentation);
        }
        fs::write(descriptor, compatible).unwrap();
        fs::write(generation.join("bin/codex"), b"corrupted bytes").unwrap();
        assert_download(false, &mut presentation);
        fs::remove_dir_all(generation).unwrap();
        assert_download(false, &mut presentation);
        remove_temp_root(root);
    }

    #[test]
    fn path_update_replaces_missing_current_and_rejects_corrupt_pointer_or_journal() {
        let root = temp_root("path-recovery");
        let roots = setup_update_test_roots(&root);
        let env = crate::tests::test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();
        let source = root.join("prepared");
        let id = "1.2.3-aaaaaa";
        let paths = activation_paths(&roots).unwrap();
        let before = generation_state::plan_initial_pointer_state("missing-current").unwrap();
        let lock = generation_state::acquire_activation_lock(&paths).unwrap();
        activate_generation(
            &paths,
            ActivationOperation::Update,
            None,
            &before,
            &lock,
            &mut presentation,
        )
        .unwrap();
        drop(lock);
        crate::tests::write_sample_generation(&source, id, false);

        let pointer = fs::read(&paths.activation_state).unwrap();
        fs::write(&paths.activation_state, b"corrupt state\n").unwrap();
        assert!(matches!(
            run_path_update(&source, &roots, &env, &mut presentation),
            Err(LocalProductError::State(_))
        ));
        assert_eq!(
            fs::read(&paths.activation_state).unwrap(),
            b"corrupt state\n"
        );
        fs::write(&paths.activation_state, &pointer).unwrap();
        let unrelated = generation_state::ActivationJournal {
            operation: ActivationOperation::Update,
            before: None,
            after: generation_state::plan_initial_pointer_state("unrelated").unwrap(),
        };
        let journal = generation_state::encode_activation_journal(&unrelated).unwrap();
        fs::write(&paths.activation_journal, &journal).unwrap();
        assert!(matches!(
            run_path_update(&source, &roots, &env, &mut presentation),
            Err(LocalProductError::State(
                generation_state::ActivationTransactionError::RecoveryConflict
            ))
        ));
        assert_eq!(fs::read(&paths.activation_journal).unwrap(), journal);
        assert_eq!(fs::read(&paths.activation_state).unwrap(), pointer);
        assert!(!roots.generation_root.join(id).exists());

        fs::remove_file(&paths.activation_journal).unwrap();
        let selected = run_path_update(&source, &roots, &env, &mut presentation).unwrap();
        let (state, loaded) = current_loaded_generation(&roots).unwrap();
        let state = state.unwrap();
        assert_eq!(selected.generation_id, id);
        assert_eq!(state.current, id);
        assert_eq!(state.previous, Some(before.current));
        verify_generation_asset_digests(&loaded.unwrap()).unwrap();
        remove_temp_root(root);
    }

    #[test]
    fn install_built_generation_dir_rejects_tampered_runtime_and_helpers() {
        let root = temp_root("p1-01-digest");
        let roots = setup_update_test_roots(&root);
        let env = crate::tests::test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();

        // 1. Tampered bin/codex
        let built_bad_rt = root.join("built-bad-rt");
        crate::tests::write_sample_generation(&built_bad_rt, "1.2.3-aaaaaa", false);
        fs::write(
            built_bad_rt.join("bin/codex"),
            b"#!/bin/sh\necho tampered\n",
        )
        .unwrap();
        fs::set_permissions(
            built_bad_rt.join("bin/codex"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(matches!(
            install_built_generation_dir(
                &built_bad_rt,
                &roots,
                &env,
                None,
                None,
                &mut presentation
            ),
            Err(LocalProductError::Descriptor(_))
        ));
        assert!(!roots.generation_root.join("1.2.3-aaaaaa").exists());

        // 2. Tampered browser helper
        let built_bad_helper = root.join("built-bad-helper");
        crate::tests::write_sample_generation(&built_bad_helper, "1.2.3-bbbbbb", false);
        fs::write(
            built_bad_helper.join("browser/open/curl"),
            b"#!/bin/sh\necho tampered\n",
        )
        .unwrap();
        fs::set_permissions(
            built_bad_helper.join("browser/open/curl"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(matches!(
            install_built_generation_dir(
                &built_bad_helper,
                &roots,
                &env,
                None,
                None,
                &mut presentation
            ),
            Err(LocalProductError::Descriptor(_))
        ));
        assert!(!roots.generation_root.join("1.2.3-bbbbbb").exists());

        // 3. Missing browser helper
        let built_missing_helper = root.join("built-missing-helper");
        crate::tests::write_sample_generation(&built_missing_helper, "1.2.3-cccccc", false);
        fs::remove_file(built_missing_helper.join("browser/manual/curl")).unwrap();
        assert!(install_built_generation_dir(
            &built_missing_helper,
            &roots,
            &env,
            None,
            None,
            &mut presentation
        )
        .is_err());
        assert!(!roots.generation_root.join("1.2.3-cccccc").exists());

        remove_temp_root(root);
    }

    #[test]
    fn rollback_rejects_corrupt_previous_and_recovers_from_damaged_current() {
        let root = temp_root("p1-02-03-rollback");
        let roots = setup_update_test_roots(&root);
        let env = crate::tests::test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();

        let src1 = root.join("src-g1");
        let src2 = root.join("src-g2");
        crate::tests::write_sample_generation(&src1, "1.2.3-111111", false);
        crate::tests::write_sample_generation(&src2, "1.2.4-222222", false);
        install_built_generation_dir(&src1, &roots, &env, None, None, &mut presentation).unwrap();
        let (ptr1, loaded1) = current_loaded_generation(&roots).unwrap();
        install_built_generation_dir(
            &src2,
            &roots,
            &env,
            ptr1.as_ref(),
            loaded1.as_ref(),
            &mut presentation,
        )
        .unwrap();

        let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        let before_bytes = fs::read(&state_paths.activation_state).unwrap();
        let current_link = roots.generation_root.parent().unwrap().join("current");
        assert_eq!(
            fs::read_link(&current_link).unwrap(),
            PathBuf::from("releases/1.2.4-222222")
        );

        // P1-02: Tampered previous target fails before lock/pointer mutation.
        let prev_rt = roots.generation_root.join("1.2.3-111111/bin/codex");
        let orig_prev_rt = fs::read(&prev_rt).unwrap();
        fs::write(&prev_rt, b"#!/bin/sh\necho corrupt\n").unwrap();
        fs::set_permissions(&prev_rt, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(run_rollback(&roots, &env, &mut presentation).is_err());
        assert_eq!(
            fs::read(&state_paths.activation_state).unwrap(),
            before_bytes
        );
        assert_eq!(
            fs::read_link(&current_link).unwrap(),
            PathBuf::from("releases/1.2.4-222222")
        );

        // P1-03: Restore previous and completely damage active `current` -> rollback still recovers.
        fs::write(&prev_rt, orig_prev_rt).unwrap();
        fs::set_permissions(&prev_rt, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_file(roots.generation_root.join("1.2.4-222222/bin/codex")).unwrap();
        fs::write(
            roots.generation_root.join("1.2.4-222222/generation.meta"),
            b"corrupt\n",
        )
        .unwrap();

        let rolled = run_rollback(&roots, &env, &mut presentation).unwrap();
        assert_eq!(rolled.generation_id, "1.2.3-111111");
        assert_eq!(
            fs::read_link(&current_link).unwrap(),
            PathBuf::from("releases/1.2.3-111111")
        );

        remove_temp_root(root);
    }

    #[test]
    fn rollback_distinguishes_reselecting_previous_from_retrying_a_committed_rollback() {
        for operation in [
            ActivationOperation::Update,
            ActivationOperation::Rollback,
            ActivationOperation::Legacy,
        ] {
            let root = temp_root("rollback-journal-operation");
            let roots = setup_update_test_roots(&root);
            let env = crate::tests::test_process_env(&roots);
            let mut presentation = UpdatePresentation::new();
            let first = root.join("first");
            let second = root.join("second");
            let first_id = "1.2.3-111111";
            let second_id = "1.2.4-222222";
            crate::tests::write_sample_generation(&first, first_id, false);
            crate::tests::write_sample_generation(&second, second_id, false);
            for dir in [&first, &second, &first] {
                let (state, loaded) = current_loaded_generation(&roots).unwrap();
                install_built_generation_dir(
                    dir,
                    &roots,
                    &env,
                    state.as_ref(),
                    loaded.as_ref(),
                    &mut presentation,
                )
                .unwrap();
            }

            let paths = activation_paths(&roots).unwrap();
            let after = generation_state::read_pointer_state(&paths)
                .unwrap()
                .unwrap();
            let before = generation_state::plan_rollback_pointer_state(&after).unwrap();
            let journal = generation_state::ActivationJournal {
                operation,
                before: Some(before.clone()),
                after: after.clone(),
            };
            // Both commands can leave this same committed pair after a cleanup failure.
            fs::write(
                &paths.activation_journal,
                generation_state::encode_activation_journal(&journal).unwrap(),
            )
            .unwrap();

            let result = run_rollback(&roots, &env, &mut presentation);
            let expected = match operation {
                ActivationOperation::Update => {
                    assert_eq!(result.unwrap().generation_id, second_id);
                    before
                }
                ActivationOperation::Rollback => {
                    assert_eq!(result.unwrap().generation_id, first_id);
                    after
                }
                ActivationOperation::Legacy => {
                    assert!(result
                        .unwrap_err()
                        .to_string()
                        .contains("ambiguous operation"));
                    after
                }
            };
            assert_eq!(
                paths.read_standalone_current().unwrap(),
                paths.standalone_current_target(&expected.current)
            );
            assert_eq!(
                generation_state::read_pointer_state(&paths).unwrap(),
                Some(expected)
            );
            assert!(!paths.activation_journal.exists());
            remove_temp_root(root);
        }
    }

    #[test]
    fn local_generation_id_uses_version_prefix_and_collision_suffix() {
        let root = temp_root("gen-id-naming");
        let roots = setup_update_test_roots(&root);
        let digest = "abcdef0123456789".repeat(4);

        assert_eq!(
            local_update_generation_id(&roots, "0.169.1", &digest),
            "0.169.1-abcdef"
        );
        fs::create_dir_all(roots.generation_root.join("0.169.1-abcdef")).unwrap();
        assert_eq!(
            local_update_generation_id(&roots, "0.169.1", &digest),
            "0.169.1-abcdef-2"
        );
        fs::create_dir_all(roots.generation_root.join("0.169.1-abcdef-2")).unwrap();
        assert_eq!(
            local_update_generation_id(&roots, "0.169.1", &digest),
            "0.169.1-abcdef-3"
        );

        remove_temp_root(root);
    }

    #[test]
    fn startup_update_cache_defers_one_offer_until_expiry() {
        let root = temp_root("startup-prompt-cache");
        let roots = setup_update_test_roots(&root);
        let gen1 = root.join("src-1.2.3");
        write_sample_generation(&gen1, "1.2.3-aaaaaa", false);
        let env = test_process_env(&roots);
        let mut presentation = UpdatePresentation::new();
        install_built_generation_dir(&gen1, &roots, &env, None, None, &mut presentation).unwrap();
        let release = OfficialReleaseMetadata {
            version: "1.3.0".into(),
            package_sha256: "b".repeat(64),
        };
        let path = roots.state_root.join(UPDATE_CHECK_CACHE_FILE);
        let mut output = Vec::new();
        let start = 1000;
        let expiry = start + UPDATE_CHECK_CACHE_TTL_SECS;
        let never_fetch: fn(&LocalWrapperRoots) -> Option<OfficialReleaseMetadata> =
            |_| panic!("fresh cache should avoid network discovery");
        let never_update: fn(&OfficialReleaseMetadata) -> i32 =
            |_| panic!("unconfirmed input should avoid update");
        assert!(!check_and_prompt_bare_launch(
            &roots,
            start,
            &mut output,
            |out, prompt| {
                out.extend_from_slice(prompt.as_bytes());
                false
            },
            |_| Some(release.clone()),
            never_update,
        ));
        assert!(String::from_utf8_lossy(&output)
            .contains("Codex 1.3.0 is available (current: 1.2.3). Update now? [y/N]: "));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(read_update_check_cache(&roots.state_root, start - 1).is_none());
        assert!(read_update_check_cache(&roots.state_root, expiry).is_none());
        assert_eq!(
            read_update_check_cache(&roots.state_root, expiry - 1)
                .unwrap()
                .deferred_for
                .as_deref(),
            Some("1.2.3")
        );
        output.clear();
        assert!(!check_and_prompt_bare_launch(
            &roots,
            expiry - 1,
            &mut output,
            |_, _| panic!("deferred offer should not prompt"),
            never_fetch,
            never_update,
        ));
        assert!(output.is_empty());
        assert!(check_and_prompt_bare_launch(
            &roots,
            expiry,
            &mut output,
            |_, _| true,
            |_| Some(release.clone()),
            |confirmed| {
                assert_eq!(confirmed, &release);
                0
            },
        ));

        fs::write(&path, b"corrupt").unwrap();
        assert!(read_update_check_cache(&roots.state_root, expiry).is_none());
        assert!(!check_and_prompt_bare_launch(
            &roots,
            expiry,
            &mut output,
            |_, _| true,
            |_| Some(release),
            |_| 1,
        ));
        assert!(String::from_utf8_lossy(&output).contains("continuing with the active generation"));
        remove_temp_root(root);
    }
}
