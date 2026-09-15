use super::*;

const PROVENANCE_FORMAT: &str = "codex-local-derived-v1";
const LOCAL_RELEASE_BASE_HOST: &str = "local.invalid";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PublicBaseline {
    pub(super) release_sequence: u64,
    pub(super) update_key: ReleasePublicKey,
    pub(super) release_manifest_sha256: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn render_provenance(baseline: &PublicBaseline) -> String {
    format!(
        "{PROVENANCE_FORMAT};baseline_sequence={};baseline_update_key={};baseline_manifest_sha256={}",
        baseline.release_sequence,
        baseline.update_key.to_hex(),
        baseline.release_manifest_sha256
    )
}

pub(super) fn parse_provenance(
    value: &str,
) -> Result<Option<PublicBaseline>, LocalProductError> {
    if !value.starts_with(PROVENANCE_FORMAT) {
        return Ok(None);
    }
    let fields: Vec<&str> = value.split(';').collect();
    if fields.len() != 4 || fields[0] != PROVENANCE_FORMAT {
        return Err(LocalProductError::Release(
            "local-derived provenance is malformed",
        ));
    }
    let sequence_text = fields[1]
        .strip_prefix("baseline_sequence=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance sequence is malformed",
        ))?;
    let release_sequence = sequence_text.parse::<u64>().map_err(|_| {
        LocalProductError::Release("local-derived provenance sequence is malformed")
    })?;
    if release_sequence == 0 || release_sequence.to_string() != sequence_text {
        return Err(LocalProductError::Release(
            "local-derived provenance sequence is malformed",
        ));
    }
    let update_key_text = fields[2]
        .strip_prefix("baseline_update_key=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance update key is malformed",
        ))?;
    let update_key = ReleasePublicKey::parse_hex(update_key_text).ok_or(
        LocalProductError::Release("local-derived provenance update key is malformed"),
    )?;
    let digest = fields[3]
        .strip_prefix("baseline_manifest_sha256=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance manifest digest is malformed",
        ))?;
    if !valid_sha256(digest) {
        return Err(LocalProductError::Release(
            "local-derived provenance manifest digest is malformed",
        ));
    }
    Ok(Some(PublicBaseline {
        release_sequence,
        update_key,
        release_manifest_sha256: digest.to_owned(),
    }))
}

fn validate_local_provenance_state(
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    provenance: &PublicBaseline,
) -> Result<(), LocalProductError> {
    if before.current_key == before.update_key {
        return Err(LocalProductError::Release(
            "local-derived provenance cannot use the official update authority as its verifier",
        ));
    }
    if provenance.release_sequence != current_release.release_sequence
        || provenance.update_key != before.update_key
    {
        return Err(LocalProductError::Release(
            "local-derived provenance does not match authenticated activation state",
        ));
    }
    Ok(())
}

pub(super) fn current_is_local_derived(
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
) -> Result<bool, LocalProductError> {
    let Some(provenance) = parse_provenance(&current_loaded.manifest.creation_metadata)? else {
        return Ok(false);
    };
    validate_local_provenance_state(before, current_release, &provenance)?;
    Ok(true)
}

fn authenticated_public_baseline(
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
) -> Result<PublicBaseline, LocalProductError> {
    if let Some(provenance) = parse_provenance(&current_loaded.manifest.creation_metadata)? {
        validate_local_provenance_state(before, current_release, &provenance)?;
        return Ok(provenance);
    }
    let manifest = roots
        .generation_root
        .join(&before.current)
        .join("release.manifest");
    Ok(PublicBaseline {
        release_sequence: current_release.release_sequence,
        update_key: before.update_key,
        release_manifest_sha256: openssl_sha256(&roots.openssl, &manifest)?,
    })
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
    if current_is_local_derived(before, current_release, current_loaded)? {
        let source_digest = openssl_sha256(&roots.openssl, &source_dir.join("release.manifest"))?;
        if source_digest == baseline.release_manifest_sha256 {
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

pub(super) struct EphemeralSigningKey {
    path: std::path::PathBuf,
    public_key: ReleasePublicKey,
}

impl EphemeralSigningKey {
    pub(super) fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub(super) fn public_key(&self) -> ReleasePublicKey {
        self.public_key
    }

    fn destroy(mut self) -> Result<ReleasePublicKey, LocalProductError> {
        let public_key = self.public_key;
        let parent = self
            .path
            .parent()
            .ok_or(LocalProductError::LocalUpdate(
                "ephemeral local-derived signing key has no parent",
            ))?
            .to_owned();
        std::fs::remove_file(&self.path).map_err(|source| LocalProductError::Io {
            operation: "remove ephemeral local-derived signing key",
            source,
        })?;
        self.path = std::path::PathBuf::new();
        sync_directory(&parent)?;
        Ok(public_key)
    }
}

impl Drop for EphemeralSigningKey {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub(super) fn generate_ephemeral_signing_key(
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
    let metadata = std::fs::symlink_metadata(&key.path).map_err(|source| LocalProductError::Io {
        operation: "inspect ephemeral local-derived signing key",
        source,
    })?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o7777 != 0o600 {
        return Err(LocalProductError::LocalUpdate(
            "ephemeral local-derived signing key is not a private 0600 regular file",
        ));
    }
    key.public_key = release_public_key_from_private_pem(openssl, &key.path)?;
    Ok(key)
}

fn activate_built_source(
    source_dir: &std::path::Path,
    expected_before: &m2_generation_state::GenerationPointerState,
    baseline: &PublicBaseline,
    verifier_key: ReleasePublicKey,
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<String, LocalProductError> {
    let (source_release, source_loaded) =
        verify_local_release_bundle_with_key(source_dir, &roots.openssl, verifier_key)?;
    if source_release.release_sequence != baseline.release_sequence
        || source_release.release_public_key != verifier_key
    {
        return Err(LocalProductError::Release(
            "local-derived signed bundle does not match its authenticated baseline",
        ));
    }
    let provenance = parse_provenance(&source_loaded.manifest.creation_metadata)?.ok_or(
        LocalProductError::Release("local-derived generation is missing signed provenance"),
    )?;
    if &provenance != baseline {
        return Err(LocalProductError::Release(
            "local-derived signed provenance changed during construction",
        ));
    }
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

pub(super) fn activate(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<String, LocalProductError> {
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
        let baseline = authenticated_public_baseline(
            roots,
            &before,
            &current_release,
            &current_loaded,
        )?;
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
        let gzip = roots
            .curl
            .parent()
            .ok_or(LocalProductError::LocalUpdate(
                "Termux tool directory is unavailable",
            ))?
            .join("gzip");
        let generation_id = local_update_generation_id();
        let unsigned_generation = staging_root.join("unsigned-generation");
        let creation_metadata = render_provenance(&baseline);
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
        codex_release_builder::publish_generation(
            &unsigned_generation,
            &baseline.release_sequence.to_string(),
            &release_base,
            signing_key.path(),
            &roots.openssl,
            &publication,
        )
        .map_err(|_| LocalProductError::LocalUpdate("local-derived signing failed"))?;
        let verifier_key_after_delete = signing_key.destroy()?;
        if verifier_key_after_delete != verifier_key {
            return Err(LocalProductError::LocalUpdate(
                "ephemeral local-derived verifier changed during cleanup",
            ));
        }
        let signed_source = publication.join("releases").join(&generation_id);
        activate_built_source(
            &signed_source,
            &before,
            &baseline,
            verifier_key,
            roots,
            process_env,
        )
    })();
    let _ = std::fs::remove_dir_all(&staging_root);
    result
}

#[cfg(test)]
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

    #[test]
    fn rald1_provenance_is_canonical_and_fail_closed() {
        let baseline = PublicBaseline {
            release_sequence: 7,
            update_key: ReleasePublicKey([0x44; 32]),
            release_manifest_sha256: "ab".repeat(32),
        };
        let rendered = render_provenance(&baseline);
        assert_eq!(parse_provenance(&rendered).unwrap(), Some(baseline));
        assert!(parse_provenance("ordinary-official-metadata")
            .unwrap()
            .is_none());
        assert!(parse_provenance("codex-local-derived-v1;baseline_sequence=7").is_err());
        assert!(parse_provenance(
            "codex-local-derived-v1;baseline_sequence=7;baseline_update_key=zz;baseline_manifest_sha256=00"
        )
        .is_err());
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
    fn rald1_build_local_selector_is_exact_and_combined_flags_are_usage_errors() {
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
    fn rald1_local_derived_source_has_no_official_private_key_or_github_publication_surface() {
        let source = include_str!("local_derived.rs");
        for forbidden in [
            "configured_update_private_key",
            "UPDATE_PRIVATE_KEY_ENV",
            "publish_local_update_to_github",
            "github_cli_path",
            "github_command",
            "GITHUB_TOKEN",
        ] {
            assert!(
                !source.contains(forbidden),
                "forbidden local-derived dependency: {forbidden}"
            );
        }
        assert!(source.contains("codex_release_builder::fetch_archive"));
        assert!(source.contains("metadata.package_sha256"));
        assert!(source.contains("signing_key.destroy()?"));
    }

    #[test]
    fn rald1_plain_transport_fallback_uses_local_derived_and_never_publication() {
        let source = include_str!("main.rs");
        let marker = "Err(LocalProductError::RemoteTransportFailed)";
        let start = source.find(marker).unwrap();
        let tail = &source[start..source.len().min(start + 1000)];
        assert!(tail.contains("local_derived::activate"));
        assert!(tail.contains("GithubPublicationOutcome::Skipped"));
        assert!(!tail.contains("publish_local_update_to_github"));
    }
}
