from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def replace_region(text: str, start: str, end: str, replacement: str, label: str) -> str:
    start_index = text.find(start)
    if start_index < 0:
        raise SystemExit(f"{label}: start marker missing")
    end_index = text.find(end, start_index)
    if end_index < 0:
        raise SystemExit(f"{label}: end marker missing")
    return text[:start_index] + replacement + text[end_index:]


core = Path("crates/core/src/local_derived.rs")
s = core.read_text()

front = r'''#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PublicBaseline {
    pub(super) generation_id: String,
    pub(super) release_sequence: u64,
    pub(super) update_key: ReleasePublicKey,
    pub(super) release_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LocalDerivedProvenance {
    pub(super) upstream_version: String,
    pub(super) archive_sha256: String,
    pub(super) baseline: PublicBaseline,
    pub(super) core_sha256: String,
}

const BUILDER_PROVENANCE: &str = "core-linked-release-builder-v1";

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn provenance_field<'a>(
    field: &'a str,
    prefix: &str,
    message: &'static str,
) -> Result<&'a str, LocalProductError> {
    field
        .strip_prefix(prefix)
        .ok_or(LocalProductError::Release(message))
}

pub(super) fn render_provenance(provenance: &LocalDerivedProvenance) -> String {
    format!(
        "{PROVENANCE_FORMAT};upstream_version={};archive_sha256={};baseline_generation={};baseline_sequence={};baseline_update_key={};baseline_manifest_sha256={};builder={BUILDER_PROVENANCE};core_sha256={}",
        provenance.upstream_version,
        provenance.archive_sha256,
        provenance.baseline.generation_id,
        provenance.baseline.release_sequence,
        provenance.baseline.update_key.to_hex(),
        provenance.baseline.release_manifest_sha256,
        provenance.core_sha256,
    )
}

pub(super) fn parse_provenance(
    value: &str,
) -> Result<Option<LocalDerivedProvenance>, LocalProductError> {
    if !value.starts_with(PROVENANCE_FORMAT) {
        return Ok(None);
    }
    let fields: Vec<&str> = value.split(';').collect();
    if fields.len() != 9 || fields[0] != PROVENANCE_FORMAT {
        return Err(LocalProductError::Release(
            "local-derived provenance is malformed",
        ));
    }

    let upstream_version = provenance_field(
        fields[1],
        "upstream_version=",
        "local-derived provenance upstream version is malformed",
    )?;
    if !valid_update_version(upstream_version) {
        return Err(LocalProductError::Release(
            "local-derived provenance upstream version is malformed",
        ));
    }

    let archive_sha256 = provenance_field(
        fields[2],
        "archive_sha256=",
        "local-derived provenance archive digest is malformed",
    )?;
    if !valid_sha256(archive_sha256) {
        return Err(LocalProductError::Release(
            "local-derived provenance archive digest is malformed",
        ));
    }

    let baseline_generation = provenance_field(
        fields[3],
        "baseline_generation=",
        "local-derived provenance baseline generation is malformed",
    )?;
    m2_generation_state::validate_generation_identity(
        baseline_generation,
        "local-derived provenance baseline_generation",
    )
    .map_err(LocalProductError::StateFormat)?;

    let sequence_text = provenance_field(
        fields[4],
        "baseline_sequence=",
        "local-derived provenance sequence is malformed",
    )?;
    let release_sequence = sequence_text.parse::<u64>().map_err(|_| {
        LocalProductError::Release("local-derived provenance sequence is malformed")
    })?;
    if release_sequence == 0 || release_sequence.to_string() != sequence_text {
        return Err(LocalProductError::Release(
            "local-derived provenance sequence is malformed",
        ));
    }

    let update_key_text = provenance_field(
        fields[5],
        "baseline_update_key=",
        "local-derived provenance baseline verifier is malformed",
    )?;
    let update_key = ReleasePublicKey::parse_hex(update_key_text).ok_or(
        LocalProductError::Release("local-derived provenance baseline verifier is malformed"),
    )?;

    let release_manifest_sha256 = provenance_field(
        fields[6],
        "baseline_manifest_sha256=",
        "local-derived provenance baseline manifest digest is malformed",
    )?;
    if !valid_sha256(release_manifest_sha256) {
        return Err(LocalProductError::Release(
            "local-derived provenance baseline manifest digest is malformed",
        ));
    }

    let builder = provenance_field(
        fields[7],
        "builder=",
        "local-derived provenance builder is malformed",
    )?;
    if builder != BUILDER_PROVENANCE {
        return Err(LocalProductError::Release(
            "local-derived provenance builder is unsupported",
        ));
    }

    let core_sha256 = provenance_field(
        fields[8],
        "core_sha256=",
        "local-derived provenance Core digest is malformed",
    )?;
    if !valid_sha256(core_sha256) {
        return Err(LocalProductError::Release(
            "local-derived provenance Core digest is malformed",
        ));
    }

    Ok(Some(LocalDerivedProvenance {
        upstream_version: upstream_version.to_owned(),
        archive_sha256: archive_sha256.to_owned(),
        baseline: PublicBaseline {
            generation_id: baseline_generation.to_owned(),
            release_sequence,
            update_key,
            release_manifest_sha256: release_manifest_sha256.to_owned(),
        },
        core_sha256: core_sha256.to_owned(),
    }))
}

fn validate_generation_provenance(
    loaded: &LoadedLocalGeneration,
    provenance: &LocalDerivedProvenance,
) -> Result<(), LocalProductError> {
    if loaded.manifest.upstream_package_version != provenance.upstream_version
        || loaded.manifest.source_artifact_digest != provenance.archive_sha256
        || loaded.manifest.core_artifact_digest != provenance.core_sha256
    {
        return Err(LocalProductError::Release(
            "local-derived provenance does not match the signed generation descriptor",
        ));
    }
    Ok(())
}

fn validate_public_baseline_bundle(
    roots: &LocalCoreRoots,
    baseline: &PublicBaseline,
) -> Result<(), LocalProductError> {
    let (release, loaded) = verify_installed_local_release(
        roots,
        &baseline.generation_id,
        baseline.update_key,
        "local-derived public baseline descriptor id does not match provenance",
    )?;
    let manifest_digest = openssl_sha256(
        &roots.openssl,
        &roots
            .generation_root
            .join(&baseline.generation_id)
            .join("release.manifest"),
    )?;
    if loaded.generation_id != baseline.generation_id
        || release.release_sequence != baseline.release_sequence
        || manifest_digest != baseline.release_manifest_sha256
    {
        return Err(LocalProductError::Release(
            "local-derived public baseline failed authenticated provenance binding",
        ));
    }
    Ok(())
}

fn validate_local_provenance_state(
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
    provenance: &LocalDerivedProvenance,
) -> Result<(), LocalProductError> {
    if before.current_key == before.update_key {
        return Err(LocalProductError::Release(
            "local-derived generation cannot use the official forward authority as its verifier",
        ));
    }
    if current_loaded.generation_id != before.current
        || current_release.release_sequence != provenance.baseline.release_sequence
        || current_release.release_public_key != before.current_key
        || provenance.baseline.generation_id == before.current
    {
        return Err(LocalProductError::Release(
            "local-derived provenance does not match authenticated activation state",
        ));
    }
    validate_generation_provenance(current_loaded, provenance)?;
    validate_public_baseline_bundle(roots, &provenance.baseline)
}

pub(super) fn current_is_local_derived(
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
) -> Result<bool, LocalProductError> {
    let Some(provenance) = parse_provenance(&current_loaded.manifest.creation_metadata)? else {
        return Ok(false);
    };
    validate_local_provenance_state(
        roots,
        before,
        current_release,
        current_loaded,
        &provenance,
    )?;
    Ok(true)
}

fn authenticated_public_baseline(
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
) -> Result<PublicBaseline, LocalProductError> {
    if let Some(provenance) = parse_provenance(&current_loaded.manifest.creation_metadata)? {
        validate_local_provenance_state(
            roots,
            before,
            current_release,
            current_loaded,
            &provenance,
        )?;
        return Ok(provenance.baseline);
    }
    let manifest = roots
        .generation_root
        .join(&before.current)
        .join("release.manifest");
    Ok(PublicBaseline {
        generation_id: before.current.clone(),
        release_sequence: current_release.release_sequence,
        update_key: before.current_key,
        release_manifest_sha256: openssl_sha256(&roots.openssl, &manifest)?,
    })
}

fn ensure_local_derived_activation_allowed(
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
) -> Result<(), LocalProductError> {
    if rollback_guard::effective_update_hold(roots)?.is_some() {
        return Err(LocalProductError::UpdateHold(
            "local-derived activation is blocked while an authenticated public rollback hold is active",
        ));
    }
    if current_is_local_derived(roots, before, current_release, current_loaded)? {
        return Err(LocalProductError::LocalUpdate(
            "a local-derived generation is already current; use signed update or rollback before rebuilding",
        ));
    }
    if before.current_key != before.update_key {
        return Err(LocalProductError::UpdateHold(
            "local-derived activation requires the current official generation to match the forward update authority",
        ));
    }
    Ok(())
}

pub(super) fn admit_signed_candidate(
    source_dir: &std::path::Path,
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
    source_release: &LocalReleaseManifest,
    source_loaded: &LoadedLocalGeneration,
) -> Result<bool, LocalProductError> {
    let baseline =
        authenticated_public_baseline(roots, before, current_release, current_loaded)?;
    if source_release.release_sequence < baseline.release_sequence {
        return Err(LocalProductError::ReleaseSequenceRollback);
    }
    if source_release.release_sequence > baseline.release_sequence {
        return Ok(false);
    }
    if current_is_local_derived(roots, before, current_release, current_loaded)? {
        let source_digest = openssl_sha256(&roots.openssl, &source_dir.join("release.manifest"))?;
        if source_loaded.generation_id == baseline.generation_id
            && source_digest == baseline.release_manifest_sha256
        {
            return Ok(false);
        }
        return Err(LocalProductError::ReleaseSequenceRollback);
    }
    if source_loaded.generation_id == before.current && source_release == current_release {
        return Ok(true);
    }
    Err(LocalProductError::ReleaseSequenceRollback)
}

pub(super) fn plan_pointer_state(
    before: &m2_generation_state::GenerationPointerState,
    generation_id: &str,
    verifier_key: ReleasePublicKey,
) -> m2_generation_state::GenerationPointerState {
    m2_generation_state::GenerationPointerState {
        update_key: before.update_key,
        current: generation_id.to_owned(),
        current_key: verifier_key,
        previous: Some(before.current.clone()),
        previous_key: Some(before.current_key),
    }
}

fn verify_official_archive_digest(
    observed: &str,
    expected: &str,
) -> Result<(), LocalProductError> {
    if observed == expected && valid_sha256(observed) {
        Ok(())
    } else {
        Err(LocalProductError::LocalUpdate(
            "official upstream archive digest does not match release metadata",
        ))
    }
}

'''

s = replace_region(
    s,
    "#[derive(Debug, Clone, PartialEq, Eq)]\npub(super) struct PublicBaseline",
    "pub(super) struct EphemeralSigningKey",
    front,
    "replace local-derived provenance/admission front",
)

generate = r'''pub(super) fn generate_ephemeral_signing_key(
    openssl: &std::path::Path,
    staging_root: &std::path::Path,
) -> Result<EphemeralSigningKey, LocalProductError> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    ensure_openssl_available(openssl)?;
    let path = staging_root.join("ephemeral-local-derived-signing-key.pem");
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|source| LocalProductError::Io {
            operation: "create ephemeral local-derived signing key",
            source,
        })?;
    let mut key = EphemeralSigningKey {
        path,
        public_key: ReleasePublicKey([0u8; 32]),
    };
    let preparation = (|| {
        let status = std::process::Command::new(openssl)
            .args(["genpkey", "-algorithm", "ED25519"])
            .env_clear()
            .stdout(std::process::Stdio::from(output))
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|source| LocalProductError::Io {
                operation: "generate ephemeral local-derived signing key",
                source,
            })?;
        if !status.success() {
            return Err(LocalProductError::LocalUpdate(
                "ephemeral local-derived signing key generation failed",
            ));
        }
        let persisted = std::fs::OpenOptions::new()
            .read(true)
            .open(&key.path)
            .map_err(|source| LocalProductError::Io {
                operation: "reopen ephemeral local-derived signing key",
                source,
            })?;
        persisted.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync ephemeral local-derived signing key",
            source,
        })?;
        let metadata =
            std::fs::symlink_metadata(&key.path).map_err(|source| LocalProductError::Io {
                operation: "inspect ephemeral local-derived signing key",
                source,
            })?;
        if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o7777 != 0o600 {
            return Err(LocalProductError::LocalUpdate(
                "ephemeral local-derived signing key is not a private 0600 regular file",
            ));
        }
        release_public_key_from_private_pem(openssl, &key.path)
    })();
    match preparation {
        Ok(public_key) => {
            key.public_key = public_key;
            Ok(key)
        }
        Err(error) => match key.destroy() {
            Ok(_) => Err(error),
            Err(cleanup) => Err(cleanup),
        },
    }
}

'''
s = replace_region(
    s,
    "pub(super) fn generate_ephemeral_signing_key(",
    "fn activate_built_source(",
    generate,
    "replace ephemeral key generation",
)

activate_built = r'''fn activate_built_source(
    source_dir: &std::path::Path,
    expected_before: &m2_generation_state::GenerationPointerState,
    expected_provenance: &LocalDerivedProvenance,
    verifier_key: ReleasePublicKey,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<String, LocalProductError> {
    let (source_release, source_loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, verifier_key)?;
    if source_release.release_sequence != expected_provenance.baseline.release_sequence
        || source_release.release_public_key != verifier_key
    {
        return Err(LocalProductError::Release(
            "local-derived signed bundle does not match its authenticated public baseline",
        ));
    }
    let provenance = parse_provenance(&source_loaded.manifest.creation_metadata)?.ok_or(
        LocalProductError::Release("local-derived generation is missing signed provenance"),
    )?;
    if &provenance != expected_provenance {
        return Err(LocalProductError::Release(
            "local-derived signed provenance changed during construction",
        ));
    }
    validate_generation_provenance(&source_loaded, &provenance)?;
    if source_loaded.manager_path.is_some() && source_loaded.core_path.is_none() {
        return Err(LocalProductError::ReleasePolicy(
            "new Manager-bearing releases require a coordinated Core artifact",
        ));
    }
    let generation_id = stage_local_generation(source_dir, &roots.generation_root)?;
    let (_, staged_loaded) = verify_installed_local_release(
        roots,
        &generation_id,
        verifier_key,
        "local-derived staged generation descriptor id does not match state",
    )?;
    let staged_provenance = parse_provenance(&staged_loaded.manifest.creation_metadata)?.ok_or(
        LocalProductError::Release("local-derived staged generation is missing signed provenance"),
    )?;
    if staged_provenance != provenance {
        return Err(LocalProductError::Release(
            "local-derived staged provenance changed after immutable publication",
        ));
    }
    validate_generation_provenance(&staged_loaded, &staged_provenance)?;
    std::fs::create_dir_all(&roots.config_dir).map_err(|source| LocalProductError::Io {
        operation: "create Core config directory",
        source,
    })?;
    probe_release_candidate(&staged_loaded, roots, process_env)?;

    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    m2_generation_state::prepare_core_state_paths(&state_paths)
        .map_err(LocalProductError::State)?;
    let after = plan_pointer_state(expected_before, &generation_id, verifier_key);
    let lock = m2_generation_state::acquire_activation_lock(&state_paths)
        .map_err(LocalProductError::State)?;
    let authoritative =
        m2_generation_state::read_pointer_state(&state_paths).map_err(LocalProductError::State)?;
    if authoritative.as_ref() != Some(expected_before) {
        return Err(LocalProductError::State(
            m2_generation_state::ActivationTransactionError::StaleAuthoritativeState,
        ));
    }
    let (_, active_loaded) = verify_installed_local_release(
        roots,
        &expected_before.current,
        expected_before.current_key,
        "active generation descriptor id does not match current",
    )?;
    let active_core_binding =
        active_core_entrypoint_binding_with_state(roots, &active_loaded, expected_before)?;
    let rollback = if let Some(core_path) = staged_loaded.core_path.as_ref() {
        let candidate_digest = &staged_loaded.manifest.core_artifact_digest;
        if active_core_binding
            .as_ref()
            .is_some_and(|binding| binding.digest == *candidate_digest)
        {
            None
        } else {
            let snapshot_generation = active_core_binding
                .as_ref()
                .map(|binding| binding.generation_id.as_str())
                .unwrap_or(expected_before.current.as_str());
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
        Some(expected_before),
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
    drop(lock);
    Ok(generation_id)
}

'''
s = replace_region(
    s,
    "fn activate_built_source(",
    "pub(super) fn activate(",
    activate_built,
    "replace local-derived activation transaction",
)

activate = r'''pub(super) fn activate(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<String, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    let before = m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)?;
    let (current_release, current_loaded) = verify_installed_local_release(
        roots,
        &before.current,
        before.current_key,
        "active generation descriptor id does not match current",
    )?;
    ensure_local_derived_activation_allowed(roots, &before, &current_release, &current_loaded)?;
    let baseline = authenticated_public_baseline(roots, &before, &current_release, &current_loaded)?;

    let metadata_root = create_local_update_staging_root(&roots.state_root)?;
    let metadata = resolve_official_release_metadata(roots, &metadata_root);
    let metadata_cleanup = std::fs::remove_dir_all(&metadata_root).map_err(|source| {
        LocalProductError::Io {
            operation: "remove local-derived metadata staging",
            source,
        }
    });
    let metadata = match (metadata, metadata_cleanup) {
        (_, Err(error)) => return Err(error),
        (Err(error), Ok(())) => return Err(error),
        (Ok(metadata), Ok(())) => metadata,
    };

    let staging_root = create_local_update_staging_root(&roots.state_root)?;
    let result = (|| {
        let archive = staging_root.join(UPSTREAM_PACKAGE_ASSET);
        let archive_digest = codex_release_builder::fetch_archive(
            &metadata.version,
            &roots.curl,
            &roots.openssl,
            &archive,
        )
        .map_err(|_| {
            LocalProductError::LocalUpdate("official upstream archive acquisition failed")
        })?;
        verify_official_archive_digest(&archive_digest, &metadata.package_sha256)?;
        let core = std::fs::canonicalize(std::env::current_exe().map_err(|source| {
            LocalProductError::Io {
                operation: "resolve running Core artifact",
                source,
            }
        })?)
        .map_err(|source| LocalProductError::Io {
            operation: "resolve running Core artifact",
            source,
        })?;
        let core_sha256 = openssl_sha256(&roots.openssl, &core)?;
        let gzip = roots
            .curl
            .parent()
            .ok_or(LocalProductError::LocalUpdate(
                "Termux tool directory is unavailable",
            ))?
            .join("gzip");
        let generation_id = local_update_generation_id();
        let unsigned_generation = staging_root.join("unsigned-generation");
        let provenance = LocalDerivedProvenance {
            upstream_version: metadata.version.clone(),
            archive_sha256: archive_digest.clone(),
            baseline: baseline.clone(),
            core_sha256,
        };
        let creation_metadata = render_provenance(&provenance);
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
        .map_err(|_| LocalProductError::LocalUpdate("local-derived upstream adaptation failed"))?;
        let signing_key = generate_ephemeral_signing_key(&roots.openssl, &staging_root)?;
        let verifier_key = signing_key.public_key();
        let release_base = format!(
            "https://{LOCAL_RELEASE_BASE_HOST}/releases/{generation_id}/"
        );
        let publication = staging_root.join("local-derived-publication");
        let signing = codex_release_builder::publish_generation(
            &unsigned_generation,
            &baseline.release_sequence.to_string(),
            &release_base,
            signing_key.path(),
            &roots.openssl,
            &publication,
        )
        .map_err(|_| LocalProductError::LocalUpdate("local-derived signing failed"));
        let key_cleanup = signing_key.destroy();
        let verifier_key_after_delete = match (signing, key_cleanup) {
            (_, Err(error)) => return Err(error),
            (Err(error), Ok(_)) => return Err(error),
            (Ok(()), Ok(public_key)) => public_key,
        };
        if verifier_key_after_delete != verifier_key {
            return Err(LocalProductError::LocalUpdate(
                "ephemeral local-derived verifier changed during cleanup",
            ));
        }
        let signed_source = publication.join("releases").join(&generation_id);
        activate_built_source(
            &signed_source,
            &before,
            &provenance,
            verifier_key,
            roots,
            process_env,
        )
    })();
    let cleanup = std::fs::remove_dir_all(&staging_root).map_err(|source| LocalProductError::Io {
        operation: "remove local-derived private staging",
        source,
    });
    match (result, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(generation_id), Ok(())) => Ok(generation_id),
    }
}

'''
s = replace_region(
    s,
    "pub(super) fn activate(",
    "#[cfg(test)]\nmod tests {",
    activate,
    "replace local-derived builder",
)

tests = r'''#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_root(label: &str) -> std::path::PathBuf {
        let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-rald1-{}-{sequence}-{label}",
            std::process::id()
        ));
        let mut builder = std::fs::DirBuilder::new();
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
        builder.create(&root).unwrap();
        root
    }

    fn find_tool(name: &str) -> std::path::PathBuf {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .map(|directory| directory.join(name))
            .find(|path| {
                std::fs::metadata(path).is_ok_and(|metadata| {
                    metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0
                })
            })
            .unwrap_or_else(|| panic!("required test tool is unavailable: {name}"))
    }

    fn sample_provenance() -> LocalDerivedProvenance {
        LocalDerivedProvenance {
            upstream_version: "9.9.9".to_owned(),
            archive_sha256: "ab".repeat(32),
            baseline: PublicBaseline {
                generation_id: "official-baseline-7".to_owned(),
                release_sequence: 7,
                update_key: ReleasePublicKey([0x44; 32]),
                release_manifest_sha256: "cd".repeat(32),
            },
            core_sha256: "ef".repeat(32),
        }
    }

    #[test]
    fn rald1_provenance_is_canonical_complete_and_fail_closed() {
        let provenance = sample_provenance();
        let rendered = render_provenance(&provenance);
        assert_eq!(parse_provenance(&rendered).unwrap(), Some(provenance));
        for required in [
            "codex-local-derived-v1",
            "upstream_version=9.9.9",
            "archive_sha256=",
            "baseline_generation=official-baseline-7",
            "baseline_sequence=7",
            "baseline_update_key=",
            "baseline_manifest_sha256=",
            "builder=core-linked-release-builder-v1",
            "core_sha256=",
        ] {
            assert!(rendered.contains(required));
        }
        assert!(parse_provenance("ordinary-official-metadata")
            .unwrap()
            .is_none());
        assert!(parse_provenance("codex-local-derived-v1;baseline_sequence=7").is_err());
        assert!(parse_provenance(&rendered.replace("upstream_version=9.9.9", "upstream_version=09.9.9"))
            .is_err());
        assert!(parse_provenance(&rendered.replace("builder=core-linked-release-builder-v1", "builder=other"))
            .is_err());
        assert!(parse_provenance(&rendered.replace(&"ab".repeat(32), "zz")).is_err());
    }

    #[test]
    fn rald1_provenance_supports_maximum_generation_identity_with_bounded_metadata() {
        let mut provenance = sample_provenance();
        provenance.baseline.generation_id = "g".repeat(512);
        let rendered = render_provenance(&provenance);
        assert!(rendered.len() < 2048);
        assert_eq!(parse_provenance(&rendered).unwrap(), Some(provenance));
    }

    #[test]
    fn rald1_local_pointer_preserves_official_forward_authority_and_public_sequence() {
        let official = ReleasePublicKey([0x11; 32]);
        let local = ReleasePublicKey([0x22; 32]);
        let before = m2_generation_state::GenerationPointerState {
            update_key: official,
            current: "official-7".to_owned(),
            current_key: official,
            previous: Some("official-6".to_owned()),
            previous_key: Some(official),
        };
        let after = plan_pointer_state(&before, "local-derived-7", local);
        assert_eq!(after.update_key, official);
        assert_eq!(after.current_key, local);
        assert_eq!(after.previous.as_deref(), Some("official-7"));
        assert_eq!(after.previous_key, Some(official));
    }

    #[test]
    fn rald1_official_archive_digest_requires_exact_canonical_match() {
        let digest = "ab".repeat(32);
        assert!(verify_official_archive_digest(&digest, &digest).is_ok());
        assert!(verify_official_archive_digest(&digest, &"cd".repeat(32)).is_err());
        assert!(verify_official_archive_digest("AB", "AB").is_err());
    }

    #[test]
    fn rald1_ephemeral_private_key_is_0600_and_drop_cleans_it() {
        let root = temp_root("ephemeral-key");
        let openssl = find_tool("openssl");
        let key = generate_ephemeral_signing_key(&openssl, &root).unwrap();
        let path = key.path().to_owned();
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
        drop(key);
        assert!(!path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rald1_ephemeral_private_key_is_removed_when_generation_fails() {
        let root = temp_root("ephemeral-key-failure");
        let fake = root.join("openssl-fail");
        std::fs::write(&fake, b"#!/bin/sh\nexit 1\n").unwrap();
        let mut permissions = std::fs::metadata(&fake).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake, permissions).unwrap();
        assert!(generate_ephemeral_signing_key(&fake, &root).is_err());
        assert!(!root.join("ephemeral-local-derived-signing-key.pem").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rald1_build_local_selector_is_exact_and_combined_flags_are_usage_errors() {
        assert_eq!(
            super::super::plan_public_dispatch(["update", "--build-local"]).unwrap(),
            super::super::PublicDispatchRoute::Update(vec!["--build-local".into()])
        );
        assert_eq!(
            super::super::run_core_update(vec!["--build-local".into(), "x".into()]),
            2
        );
        assert_eq!(
            super::super::run_core_update(vec!["--build-local".into(), "--force".into()]),
            2
        );
        assert_eq!(
            super::super::run_core_update(vec!["--build-local".into(), "--rollback".into()]),
            2
        );
        assert!(super::super::UPDATE_USAGE.contains("--build-local"));
    }

    #[test]
    fn rald1_local_derived_production_has_no_official_private_key_or_github_surface() {
        let source = include_str!("local_derived.rs")
            .split("\n#[cfg(test)]")
            .next()
            .unwrap();
        for forbidden in [
            "configured_update_private_key",
            "UPDATE_PRIVATE_KEY_ENV",
            "CODEX_RELEASE_SIGNING_KEY",
            "CODEX_TERMUX_UPDATE_PRIVATE_KEY",
            "publish_local_update_to_github",
            "github_cli_path",
            "github_command",
            "GITHUB_TOKEN",
            "Command::new(\"gh\")",
            "required_absolute_env_path(\"HOME\")",
        ] {
            assert!(
                !source.contains(forbidden),
                "forbidden local-derived dependency: {forbidden}"
            );
        }
        assert!(source.contains("resolve_official_release_metadata"));
        assert!(source.contains("codex_release_builder::fetch_archive"));
        assert!(source.contains("metadata.package_sha256"));
        assert!(source.contains("signing_key.destroy()"));
        assert!(source.contains("rollback_guard::effective_update_hold"));
    }

    #[test]
    fn rald1_plain_transport_fallback_uses_local_derived_and_never_publication() {
        let source = include_str!("main.rs");
        let unified = source
            .find("fn activate_unified_update_with_hold_policy(")
            .unwrap();
        let marker = "Err(LocalProductError::RemoteTransportFailed)";
        let relative = source[unified..].find(marker).unwrap();
        let start = unified + relative;
        let tail = &source[start..source.len().min(start + 1200)];
        assert!(tail.contains("hold_policy == UpdateHoldPolicy::Enforce"));
        assert!(tail.contains("local_derived::activate"));
        assert!(tail.contains("GithubPublicationOutcome::Skipped"));
        assert!(!tail.contains("publish_local_update_to_github"));
    }

    #[test]
    fn rald1_force_route_never_enters_local_derived_fallback() {
        let source = include_str!("main.rs");
        let run = source.find("fn run_core_update(args: Vec<OsString>)").unwrap();
        let tail = &source[run..source.len().min(run + 5000)];
        assert!(tail.contains("UpdateHoldPolicy::ForceHeld"));
        assert!(tail.contains("if build_local"));
        assert!(!tail.contains("--force\") => local_derived"));
    }
}
'''
s = replace_region(
    s,
    "#[cfg(test)]\nmod tests {",
    "__RALD1_EOF_MARKER_THAT_DOES_NOT_EXIST__",
    tests,
    "replace local-derived tests",
) if "__RALD1_EOF_MARKER_THAT_DOES_NOT_EXIST__" in s else s[:s.index("#[cfg(test)]\nmod tests {")] + tests

core.write_text(s)

main = Path("crates/core/src/main.rs")
m = main.read_text()
m = replace_once(
    m,
    "    let local_derived_current = local_derived::current_is_local_derived(\n        &before,",
    "    let local_derived_current = local_derived::current_is_local_derived(\n        roots,\n        &before,",
    "pass roots to rollback local-derived verifier",
)

integration = r'''

    #[cfg(unix)]
    fn rald1_set_descriptor_field(
        generation: &std::path::Path,
        field: &str,
        value: &str,
    ) {
        assert!(!value.contains(['\n', '\r', '\t']));
        let path = generation.join("generation.meta");
        let input = std::fs::read_to_string(&path).unwrap();
        let prefix = format!("{field}\t");
        let mut matches = 0;
        let mut output = String::with_capacity(input.len() + value.len());
        for line in input.lines() {
            if line.starts_with(&prefix) {
                matches += 1;
                output.push_str(&prefix);
                output.push_str(value);
            } else {
                output.push_str(line);
            }
            output.push('\n');
        }
        assert_eq!(matches, 1, "descriptor field {field} must be unique");
        std::fs::write(path, output).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_rald1_state_roundtrip_official_advance_rollback_and_probe_failure() {
        let root = temp_root("rald1-state-flow");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let source = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source.generation_root).unwrap();
        let official_private = root.join("official-key/private.pem");
        let official_public = root.join("official-key/public.pem");
        b4_generate_release_keypair(&openssl, &official_private, &official_public);

        let official7 = b2_write_generation(&source, "rald1-official-7", false, "supported");
        b4_write_probe_runtime(&official7, 0, 0);
        b4_write_signed_release(&official7, 7, &openssl, &official_private);
        let initial = b7_seed_initial_release(&official7, &home, &prefix, &official_public);
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let baseline_manifest_sha256 = openssl_sha256(
            &openssl,
            &roots
                .generation_root
                .join("rald1-official-7/release.manifest"),
        )
        .unwrap();

        let local = b2_write_generation(&source, "rald1-local-7", false, "supported");
        b4_write_probe_runtime(&local, 0, 0);
        let archive_sha256 = "ab".repeat(32);
        let core_sha256 = "cd".repeat(32);
        rald1_set_descriptor_field(&local, "source_artifact_digest", &archive_sha256);
        rald1_set_descriptor_field(&local, "core_artifact_digest", &core_sha256);
        let local_loaded = load_local_generation(&local).unwrap();
        let provenance = local_derived::LocalDerivedProvenance {
            upstream_version: local_loaded.manifest.upstream_package_version.clone(),
            archive_sha256: archive_sha256.clone(),
            baseline: local_derived::PublicBaseline {
                generation_id: initial.current.clone(),
                release_sequence: 7,
                update_key: initial.update_key,
                release_manifest_sha256: baseline_manifest_sha256,
            },
            core_sha256,
        };
        rald1_set_descriptor_field(
            &local,
            "creation_metadata",
            &local_derived::render_provenance(&provenance),
        );
        let local_private = root.join("local-key/private.pem");
        let local_public = root.join("local-key/public.pem");
        b4_generate_release_keypair(&openssl, &local_private, &local_public);
        b4_write_signed_release(&local, 7, &openssl, &local_private);
        let local_key = release_public_key_from_pem(&openssl, &local_public).unwrap();
        assert_ne!(local_key, initial.update_key);
        assert_eq!(
            stage_local_generation(&local, &roots.generation_root).unwrap(),
            "rald1-local-7"
        );
        let local_state =
            local_derived::plan_pointer_state(&initial, "rald1-local-7", local_key);
        activate_pointer_state(&paths, Some(&initial), &local_state).unwrap();
        std::fs::remove_file(&local_private).unwrap();
        std::fs::remove_file(&local_public).unwrap();

        let reread = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(reread.update_key, initial.update_key);
        assert_eq!(reread.current_key, local_key);
        let (local_release, local_loaded) = verify_installed_local_release(
            &roots,
            &reread.current,
            reread.current_key,
            "RALD-1 new-process local-derived verification",
        )
        .unwrap();
        assert_eq!(local_release.release_sequence, 7);
        assert!(local_derived::current_is_local_derived(
            &roots,
            &reread,
            &local_release,
            &local_loaded,
        )
        .unwrap());

        let rolled = rollback_signed_local_release(&roots).unwrap();
        assert_eq!(rolled, "rald1-official-7");
        assert!(read_update_hold(&roots).unwrap().is_none());
        assert!(rollback_guard::read_guard(&roots).unwrap().is_none());
        let official_again = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(official_again.current, "rald1-official-7");
        assert_eq!(official_again.update_key, initial.update_key);

        let local_again =
            local_derived::plan_pointer_state(&official_again, "rald1-local-7", local_key);
        activate_pointer_state(&paths, Some(&official_again), &local_again).unwrap();

        let attacker = b2_write_generation(&source, "rald1-attacker-8", false, "supported");
        b4_write_probe_runtime(&attacker, 0, 0);
        let attacker_private = root.join("attacker-key/private.pem");
        let attacker_public = root.join("attacker-key/public.pem");
        b4_generate_release_keypair(&openssl, &attacker_private, &attacker_public);
        b4_write_signed_release(&attacker, 8, &openssl, &attacker_private);
        let before_attack = std::fs::read(&paths.activation_state).unwrap();
        assert!(prepare_signed_local_release(&attacker, &roots).is_err());
        assert_eq!(std::fs::read(&paths.activation_state).unwrap(), before_attack);

        let official8 = b2_write_generation(&source, "rald1-official-8", false, "supported");
        b4_write_probe_runtime(&official8, 0, 0);
        b4_write_signed_release(&official8, 8, &openssl, &official_private);
        let prepared = match prepare_signed_local_release(&official8, &roots).unwrap() {
            PreparedSignedLocalRelease::Activation(prepared) => *prepared,
            PreparedSignedLocalRelease::AlreadyCurrent(_) => {
                panic!("newer official release must require activation")
            }
        };
        assert_eq!(
            activate_prepared_local_release(
                prepared,
                &roots,
                &b8_process_env(&prefix, &tmp),
            )
            .unwrap(),
            "rald1-official-8"
        );
        let advanced = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(advanced.current, "rald1-official-8");
        assert_eq!(advanced.previous.as_deref(), Some("rald1-local-7"));
        assert_eq!(advanced.update_key, initial.update_key);
        assert_eq!(advanced.current_key, initial.update_key);

        assert_eq!(rollback_signed_local_release(&roots).unwrap(), "rald1-local-7");
        let held = read_update_hold(&roots).unwrap().unwrap();
        assert_eq!(held.generation_id, "rald1-official-8");
        assert_eq!(held.release_sequence, 8);
        let rolled_to_local = read_pointer_state(&paths).unwrap().unwrap();
        let (rolled_release, rolled_loaded) = verify_installed_local_release(
            &roots,
            &rolled_to_local.current,
            rolled_to_local.current_key,
            "RALD-1 rolled-back local-derived verification",
        )
        .unwrap();
        assert!(local_derived::current_is_local_derived(
            &roots,
            &rolled_to_local,
            &rolled_release,
            &rolled_loaded,
        )
        .unwrap());

        let official9 = b2_write_generation(&source, "rald1-official-9", false, "supported");
        b4_write_probe_runtime(&official9, 1, 0);
        b4_write_signed_release(&official9, 9, &openssl, &official_private);
        let state_before_probe = std::fs::read(&paths.activation_state).unwrap();
        let authority_before_probe = rolled_to_local.update_key;
        let hold_before_probe = read_update_hold(&roots).unwrap();
        let prepared = match prepare_signed_local_release(&official9, &roots).unwrap() {
            PreparedSignedLocalRelease::Activation(prepared) => *prepared,
            PreparedSignedLocalRelease::AlreadyCurrent(_) => {
                panic!("greater official release must require activation")
            }
        };
        assert!(activate_prepared_local_release(
            prepared,
            &roots,
            &b8_process_env(&prefix, &tmp),
        )
        .is_err());
        assert_eq!(std::fs::read(&paths.activation_state).unwrap(), state_before_probe);
        assert_eq!(
            read_pointer_state(&paths).unwrap().unwrap().update_key,
            authority_before_probe
        );
        assert_eq!(read_update_hold(&roots).unwrap(), hold_before_probe);

        remove_temp_root(root);
    }
'''
last = m.rfind("\n}")
if last < 0:
    raise SystemExit("main tests module closing brace missing")
m = m[:last] + integration + m[last:]
main.write_text(m)

builder = Path("crates/release-builder/src/lib.rs")
b = builder.read_text()
b = replace_once(
    b,
    "const TEXT_VALUE_MAX_BYTES: usize = 512;\n",
    "const TEXT_VALUE_MAX_BYTES: usize = 512;\nconst CREATION_METADATA_MAX_BYTES: usize = 2048;\n",
    "add bounded creation-metadata extension",
)
b = replace_once(
    b,
    "    if !valid_line_value(&request.creation_metadata, TEXT_VALUE_MAX_BYTES) {\n",
    "    if !valid_line_value(&request.creation_metadata, CREATION_METADATA_MAX_BYTES) {\n",
    "use creation-metadata-specific bound",
)
builder.write_text(b)

spec = Path("SPEC.md")
text = spec.read_text()
old = '''`activation-state-v3.update_key` remains the official forward-update authority and MUST
remain byte-identical across local-derived activation. The local-derived signed
`generation.meta` MUST carry canonical `codex-local-derived-v1` provenance binding the
authenticated public baseline sequence, the official `update_key`, and the SHA-256 of
the authenticated baseline `release.manifest`. The local-derived release manifest uses
that baseline public sequence unchanged; it does not allocate or consume a synthetic
public sequence. Malformed provenance or provenance inconsistent with authenticated
state fails closed.
'''
new = '''`activation-state-v3.update_key` remains the official forward-update authority and MUST
remain byte-identical across local-derived activation. The local-derived signed
`generation.meta` MUST carry canonical `codex-local-derived-v1` provenance binding the
exact official upstream version and archive SHA-256, authenticated public baseline
generation and release sequence, the historical verifier and SHA-256 of that baseline
`release.manifest`, a fixed Core-linked builder contract, and the exact running Core
SHA-256 used by the build. The historical baseline verifier is provenance and does not
replace the current `activation-state-v3.update_key`; after a later official update and
rollback those keys may legitimately differ. The local-derived release manifest uses
its baseline public sequence unchanged; it does not allocate or consume a synthetic
public sequence. The creation-metadata field has a dedicated 2048-byte bound so the
full legal generation identity plus these fixed bindings fit without widening other
release text fields. Malformed, unsupported, or descriptor-inconsistent provenance,
or a missing/corrupt baseline bundle, fails closed.
'''
text = replace_once(text, old, new, "strengthen RALD-1 SPEC provenance")
old = '''Local-derived activation uses the existing probe plus atomic current/previous generation
transaction while preserving `update_key`. Rolling back from a valid local-derived
current to its retained previous generation MUST NOT create a public update hold or the
legacy rollback-Core hold guard. Rolling back from an official current retains the
existing authenticated hold/guard semantics unchanged. `codex update --force` remains
limited to retrying the exact authenticated held public sequence and never targets a
local-derived generation.
'''
new = '''Local-derived activation uses the existing probe plus atomic current/previous generation
transaction while preserving `update_key`. A new local-derived activation MUST fail
closed while an authenticated public rollback hold is active, and it MUST also refuse
to stack a second local-derived current; this prevents local construction from changing
the retained generation that authenticates hold/rollback semantics. Rolling back from
a valid local-derived current to its retained previous generation MUST NOT create a new
public update hold or legacy rollback-Core hold guard. Rolling back from an official
current retains the existing authenticated hold/guard semantics unchanged, including
when the retained previous generation is local-derived. `codex update --force` remains
limited to retrying the exact authenticated held public sequence and never targets a
local-derived generation.
'''
text = replace_once(text, old, new, "strengthen RALD-1 SPEC hold interaction")
spec.write_text(text)
