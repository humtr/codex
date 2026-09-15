from pathlib import Path

p = Path("crates/core/src/local_derived.rs")
s = p.read_text()

def replace_between(text, start, end, replacement):
    a = text.index(start)
    b = text.index(end, a)
    return text[:a] + replacement + text[b:]

s = replace_between(
    s,
    "#[derive(Debug, Clone, PartialEq, Eq)]\npub(super) struct PublicBaseline {",
    "fn valid_sha256",
    r'''#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PublicBaseline {
    pub(super) generation_id: String,
    pub(super) release_sequence: u64,
    pub(super) update_key: ReleasePublicKey,
    pub(super) release_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LocalDerivedProvenance {
    pub(super) baseline: PublicBaseline,
    pub(super) upstream_version: String,
    pub(super) upstream_archive_sha256: String,
    pub(super) builder_core_sha256: String,
}

fn valid_provenance_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+')
        })
}

''',
)

s = replace_between(
    s,
    "pub(super) fn render_provenance",
    "fn validate_local_provenance_state",
    r'''fn validate_provenance(provenance: &LocalDerivedProvenance) -> Result<(), LocalProductError> {
    m2_generation_state::validate_generation_identity(
        &provenance.baseline.generation_id,
        "local-derived baseline generation",
    )
    .map_err(LocalProductError::StateFormat)?;
    if !valid_provenance_token(&provenance.baseline.generation_id) {
        return Err(LocalProductError::Release(
            "local-derived provenance baseline generation is malformed",
        ));
    }
    if provenance.baseline.release_sequence == 0 {
        return Err(LocalProductError::Release(
            "local-derived provenance sequence is malformed",
        ));
    }
    if !valid_sha256(&provenance.baseline.release_manifest_sha256) {
        return Err(LocalProductError::Release(
            "local-derived provenance manifest digest is malformed",
        ));
    }
    if !valid_provenance_token(&provenance.upstream_version) {
        return Err(LocalProductError::Release(
            "local-derived provenance upstream version is malformed",
        ));
    }
    if !valid_sha256(&provenance.upstream_archive_sha256) {
        return Err(LocalProductError::Release(
            "local-derived provenance upstream archive digest is malformed",
        ));
    }
    if !valid_sha256(&provenance.builder_core_sha256) {
        return Err(LocalProductError::Release(
            "local-derived provenance builder Core digest is malformed",
        ));
    }
    Ok(())
}

pub(super) fn render_provenance(
    provenance: &LocalDerivedProvenance,
) -> Result<String, LocalProductError> {
    validate_provenance(provenance)?;
    Ok(format!(
        "{PROVENANCE_FORMAT};baseline_generation={};baseline_sequence={};baseline_update_key={};baseline_manifest_sha256={};upstream_version={};upstream_archive_sha256={};builder_core_sha256={}",
        provenance.baseline.generation_id,
        provenance.baseline.release_sequence,
        provenance.baseline.update_key.to_hex(),
        provenance.baseline.release_manifest_sha256,
        provenance.upstream_version,
        provenance.upstream_archive_sha256,
        provenance.builder_core_sha256,
    ))
}

pub(super) fn parse_provenance(
    value: &str,
) -> Result<Option<LocalDerivedProvenance>, LocalProductError> {
    if !value.starts_with(PROVENANCE_FORMAT) {
        return Ok(None);
    }
    let fields: Vec<&str> = value.split(';').collect();
    if fields.len() != 8 || fields[0] != PROVENANCE_FORMAT {
        return Err(LocalProductError::Release(
            "local-derived provenance is malformed",
        ));
    }
    let generation_id = fields[1]
        .strip_prefix("baseline_generation=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance baseline generation is malformed",
        ))?;
    let sequence_text = fields[2]
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
    let update_key_text = fields[3]
        .strip_prefix("baseline_update_key=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance update key is malformed",
        ))?;
    let update_key = ReleasePublicKey::parse_hex(update_key_text).ok_or(
        LocalProductError::Release("local-derived provenance update key is malformed"),
    )?;
    let release_manifest_sha256 = fields[4]
        .strip_prefix("baseline_manifest_sha256=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance manifest digest is malformed",
        ))?;
    let upstream_version = fields[5]
        .strip_prefix("upstream_version=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance upstream version is malformed",
        ))?;
    let upstream_archive_sha256 = fields[6]
        .strip_prefix("upstream_archive_sha256=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance upstream archive digest is malformed",
        ))?;
    let builder_core_sha256 = fields[7]
        .strip_prefix("builder_core_sha256=")
        .ok_or(LocalProductError::Release(
            "local-derived provenance builder Core digest is malformed",
        ))?;
    let provenance = LocalDerivedProvenance {
        baseline: PublicBaseline {
            generation_id: generation_id.to_owned(),
            release_sequence,
            update_key,
            release_manifest_sha256: release_manifest_sha256.to_owned(),
        },
        upstream_version: upstream_version.to_owned(),
        upstream_archive_sha256: upstream_archive_sha256.to_owned(),
        builder_core_sha256: builder_core_sha256.to_owned(),
    };
    validate_provenance(&provenance)?;
    Ok(Some(provenance))
}

''',
)

s = replace_between(
    s,
    "fn validate_local_provenance_state",
    "pub(super) fn current_is_local_derived",
    r'''fn validate_local_provenance_state(
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    provenance: &LocalDerivedProvenance,
) -> Result<(), LocalProductError> {
    if before.current_key == before.update_key {
        return Err(LocalProductError::Release(
            "local-derived provenance cannot use the official update authority as its verifier",
        ));
    }
    if provenance.baseline.release_sequence != current_release.release_sequence
        || provenance.baseline.update_key != before.update_key
    {
        return Err(LocalProductError::Release(
            "local-derived provenance does not match authenticated activation state",
        ));
    }
    Ok(())
}

''',
)

s = replace_between(
    s,
    "fn authenticated_public_baseline",
    "pub(super) fn admit_signed_candidate",
    r'''fn authenticated_public_baseline(
    roots: &LocalCoreRoots,
    before: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
) -> Result<PublicBaseline, LocalProductError> {
    if let Some(provenance) = parse_provenance(&current_loaded.manifest.creation_metadata)? {
        validate_local_provenance_state(before, current_release, &provenance)?;
        return Ok(provenance.baseline);
    }
    let manifest = roots
        .generation_root
        .join(&before.current)
        .join("release.manifest");
    Ok(PublicBaseline {
        generation_id: before.current.clone(),
        release_sequence: current_release.release_sequence,
        update_key: before.update_key,
        release_manifest_sha256: openssl_sha256(&roots.openssl, &manifest)?,
    })
}

''',
)

s = s.replace(
    "    baseline: &PublicBaseline,\n    verifier_key: ReleasePublicKey,",
    "    provenance: &LocalDerivedProvenance,\n    verifier_key: ReleasePublicKey,",
    1,
)
s = s.replace(
    "    if source_release.release_sequence != baseline.release_sequence\n",
    "    if source_release.release_sequence != provenance.baseline.release_sequence\n",
    1,
)
s = s.replace(
    "    let provenance = parse_provenance(&source_loaded.manifest.creation_metadata)?.ok_or(\n",
    "    let signed_provenance = parse_provenance(&source_loaded.manifest.creation_metadata)?.ok_or(\n",
    1,
)
s = s.replace(
    "    if &provenance != baseline {\n",
    "    if &signed_provenance != provenance {\n",
    1,
)

old = r'''        let generation_id = local_update_generation_id();
        let unsigned_generation = staging_root.join("unsigned-generation");
        let creation_metadata = render_provenance(&baseline);
'''
new = r'''        let builder_core_sha256 = openssl_sha256(&roots.openssl, &core)?;
        let provenance = LocalDerivedProvenance {
            baseline,
            upstream_version: metadata.version.clone(),
            upstream_archive_sha256: archive_digest.clone(),
            builder_core_sha256,
        };
        let generation_id = local_update_generation_id();
        let unsigned_generation = staging_root.join("unsigned-generation");
        let creation_metadata = render_provenance(&provenance)?;
'''
if old not in s:
    raise SystemExit("activation provenance marker missing")
s = s.replace(old, new, 1)
s = s.replace(
    "            &baseline.release_sequence.to_string(),\n",
    "            &provenance.baseline.release_sequence.to_string(),\n",
    1,
)
s = s.replace(
    "            &baseline,\n            verifier_key,\n",
    "            &provenance,\n            verifier_key,\n",
    1,
)

# Fold the two v5 test fixes into source before replacing the test module.
s = s.replace(
    '        let source = include_str!("local_derived.rs");\n        for forbidden in [',
    '        let source = include_str!("local_derived.rs")\n            .split("\\n#[cfg(test)]")\n            .next()\n            .unwrap();\n        for forbidden in [',
    1,
)

module = r'''#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    const RESTART_VERIFY_ROLE: &str = "CODEX_RALD1_RESTART_VERIFY_ROLE";
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
            baseline: PublicBaseline {
                generation_id: "official-7".to_owned(),
                release_sequence: 7,
                update_key: ReleasePublicKey([0x44; 32]),
                release_manifest_sha256: "ab".repeat(32),
            },
            upstream_version: "0.154.0".to_owned(),
            upstream_archive_sha256: "cd".repeat(32),
            builder_core_sha256: "ef".repeat(32),
        }
    }

    #[test]
    fn rald1_provenance_is_complete_canonical_and_fail_closed() {
        let provenance = sample_provenance();
        let rendered = render_provenance(&provenance).unwrap();
        assert_eq!(parse_provenance(&rendered).unwrap(), Some(provenance));
        assert!(rendered.contains("baseline_generation=official-7"));
        assert!(rendered.contains("upstream_version=0.154.0"));
        assert!(rendered.contains("upstream_archive_sha256="));
        assert!(rendered.contains("builder_core_sha256="));
        assert!(parse_provenance("ordinary-official-metadata")
            .unwrap()
            .is_none());
        assert!(parse_provenance("codex-local-derived-v1;baseline_sequence=7").is_err());
        let mut malformed = render_provenance(&sample_provenance()).unwrap();
        malformed = malformed.replace("baseline_generation=official-7", "baseline_generation=../bad");
        assert!(parse_provenance(&malformed).is_err());
        let mut malformed = render_provenance(&sample_provenance()).unwrap();
        malformed = malformed.replace(&"cd".repeat(32), "00");
        assert!(parse_provenance(&malformed).is_err());
    }

    #[test]
    fn rald1_official_archive_digest_match_is_exact() {
        let digest = "12".repeat(32);
        verify_official_archive_digest(&digest, &digest).unwrap();
        assert!(verify_official_archive_digest(&digest, &"34".repeat(32)).is_err());
        assert!(verify_official_archive_digest(&digest.to_uppercase(), &digest.to_uppercase()).is_err());
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
    fn rald1_ephemeral_private_key_is_0600_and_cleaned_on_success_and_failure() {
        let openssl = find_tool("openssl");
        let root = temp_root("ephemeral-key-success");
        let key = generate_ephemeral_signing_key(&openssl, &root).unwrap();
        let path = key.path().to_owned();
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
        let verifier = key.public_key();
        assert_eq!(key.destroy().unwrap(), verifier);
        assert!(!path.exists());
        std::fs::remove_dir_all(root).unwrap();

        let root = temp_root("ephemeral-key-failure");
        let fake = root.join("failing-openssl");
        std::fs::write(&fake, b"#!/bin/sh\nexit 1\n").unwrap();
        let mut mode = std::fs::metadata(&fake).unwrap().permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(&fake, mode).unwrap();
        assert!(generate_ephemeral_signing_key(&fake, &root).is_err());
        assert!(!root.join("ephemeral-local-derived-signing-key.pem").exists());
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
        let source = include_str!("local_derived.rs")
            .split("\n#[cfg(test)]")
            .next()
            .unwrap();
        for forbidden in [
            "configured_update_private_key",
            "UPDATE_PRIVATE_KEY_ENV",
            "CODEX_RELEASE_SIGNING_KEY",
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
        assert!(source.contains("builder_core_sha256"));
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
        let tail = &source[start..source.len().min(start + 1000)];
        assert!(tail.contains("local_derived::activate"));
        assert!(tail.contains("GithubPublicationOutcome::Skipped"));
        assert!(!tail.contains("publish_local_update_to_github"));
    }

    #[test]
    fn rald1_activation_orders_probe_before_state_commit() {
        let source = include_str!("local_derived.rs")
            .split("\n#[cfg(test)]")
            .next()
            .unwrap();
        let start = source.find("fn activate_built_source(").unwrap();
        let body = &source[start..];
        let probe = body.find("probe_release_candidate(").unwrap();
        let commit = body.find("activate_pointer_state_locked(").unwrap();
        assert!(probe < commit);
    }

    #[test]
    fn rald1_restart_verifier_probe() {
        if std::env::var(RESTART_VERIFY_ROLE).as_deref() != Ok("1") {
            return;
        }
        let roots = LocalCoreRoots::from_environment().unwrap();
        let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root).unwrap();
        let state = m2_generation_state::read_pointer_state(&state_paths)
            .unwrap()
            .unwrap();
        let (release, loaded) = verify_installed_local_release(
            &roots,
            &state.current,
            state.current_key,
            "restart local-derived generation id mismatch",
        )
        .unwrap();
        assert!(current_is_local_derived(&state, &release, &loaded).unwrap());
    }
}
'''
start = s.index("#[cfg(test)]\nmod tests {")
s = s[:start] + module
p.write_text(s)
