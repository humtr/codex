use super::*;
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

pub(super) const UPDATE_HOLD_CAPABILITY_LINE: &str = "update hold capability: codex-update-hold-v1";
const ROLLBACK_CORE_GUARD_FORMAT: &str = "codex-rollback-core-guard-v1";
const ROLLBACK_CORE_GUARD_FILE: &str = "rollback-core-guard";
const ROLLBACK_CORE_GUARD_MAX_BYTES: usize = 8192;
const CAPABILITY_OUTPUT_MAX_BYTES: usize = 8192;
const CAPABILITY_TIMEOUT_SECONDS: u64 = 5;
#[cfg(test)]
pub(super) const TEST_HOLD_CAPABILITY_ENV: &str = "CODEX_TEST_ROLLBACK_HOLD_CAPABILITY";
static ROLLBACK_CORE_GUARD_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RollbackCoreGuardRecord {
    pub(super) target_generation_id: String,
    pub(super) held_generation_id: String,
    pub(super) held_release_sequence: u64,
    pub(super) held_core_sha256: String,
}

fn guard_path(roots: &LocalCoreRoots) -> std::path::PathBuf {
    roots.state_root.join(ROLLBACK_CORE_GUARD_FILE)
}

fn render_guard(record: &RollbackCoreGuardRecord) -> String {
    format!(
        "{ROLLBACK_CORE_GUARD_FORMAT}\ntarget_generation_id\t{}\nheld_generation_id\t{}\nheld_release_sequence\t{}\nheld_core_sha256\t{}\n",
        record.target_generation_id,
        record.held_generation_id,
        record.held_release_sequence,
        record.held_core_sha256,
    )
}

fn parse_guard(bytes: &[u8]) -> Result<RollbackCoreGuardRecord, LocalProductError> {
    if bytes.len() > ROLLBACK_CORE_GUARD_MAX_BYTES {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard exceeds its byte bound",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LocalProductError::UpdateHold("rollback Core guard is not UTF-8"))?;
    if text.contains('\r') || !text.ends_with('\n') {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard format is invalid",
        ));
    }
    let lines: Vec<&str> = text
        .strip_suffix('\n')
        .unwrap_or(text)
        .split('\n')
        .collect();
    if lines.len() != 5 || lines[0] != ROLLBACK_CORE_GUARD_FORMAT {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard format is invalid",
        ));
    }
    let target_generation_id =
        lines[1]
            .strip_prefix("target_generation_id\t")
            .ok_or(LocalProductError::UpdateHold(
                "rollback Core guard target generation is invalid",
            ))?;
    let held_generation_id =
        lines[2]
            .strip_prefix("held_generation_id\t")
            .ok_or(LocalProductError::UpdateHold(
                "rollback Core guard held generation is invalid",
            ))?;
    m2_generation_state::validate_generation_identity(
        target_generation_id,
        "rollback Core guard target_generation_id",
    )
    .map_err(LocalProductError::StateFormat)?;
    m2_generation_state::validate_generation_identity(
        held_generation_id,
        "rollback Core guard held_generation_id",
    )
    .map_err(LocalProductError::StateFormat)?;
    if target_generation_id == held_generation_id {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard generations must differ",
        ));
    }
    let sequence_text =
        lines[3]
            .strip_prefix("held_release_sequence\t")
            .ok_or(LocalProductError::UpdateHold(
                "rollback Core guard release sequence is invalid",
            ))?;
    let held_release_sequence = sequence_text.parse::<u64>().map_err(|_| {
        LocalProductError::UpdateHold("rollback Core guard release sequence is invalid")
    })?;
    if held_release_sequence == 0 || held_release_sequence.to_string() != sequence_text {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard release sequence is invalid",
        ));
    }
    let held_core_sha256 =
        lines[4]
            .strip_prefix("held_core_sha256\t")
            .ok_or(LocalProductError::UpdateHold(
                "rollback Core guard Core digest is invalid",
            ))?;
    if !valid_sha256_hex(held_core_sha256) {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard Core digest is invalid",
        ));
    }
    Ok(RollbackCoreGuardRecord {
        target_generation_id: target_generation_id.to_owned(),
        held_generation_id: held_generation_id.to_owned(),
        held_release_sequence,
        held_core_sha256: held_core_sha256.to_owned(),
    })
}

pub(super) fn read_guard(
    roots: &LocalCoreRoots,
) -> Result<Option<RollbackCoreGuardRecord>, LocalProductError> {
    let path = guard_path(roots);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect rollback Core guard",
                source,
            })
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(LocalProductError::UpdateHold(
                "rollback Core guard must be a regular file",
            ))
        }
        Ok(_) => {}
    }
    let bytes = read_bounded_regular_file(
        &path,
        ROLLBACK_CORE_GUARD_MAX_BYTES,
        "read rollback Core guard",
        LocalProductError::UpdateHold("rollback Core guard exceeds its byte bound"),
        LocalProductError::UpdateHold("rollback Core guard must be a regular file"),
    )?;
    parse_guard(&bytes).map(Some)
}

pub(super) fn write_guard_locked(
    roots: &LocalCoreRoots,
    record: &RollbackCoreGuardRecord,
) -> Result<(), LocalProductError> {
    ensure_real_directory(
        &roots.state_root,
        "inspect Core state root for rollback Core guard",
        "Core state root for rollback Core guard is not a real directory",
    )?;
    let destination = guard_path(roots);
    match std::fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => {
            return Err(LocalProductError::UpdateHold(
                "rollback Core guard destination has an unsafe file type",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect rollback Core guard destination",
                source,
            })
        }
    }
    let sequence = ROLLBACK_CORE_GUARD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = roots.state_root.join(format!(
        ".rollback-core-guard.{}.{}.tmp",
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
                operation: "create rollback Core guard temporary",
                source,
            })?;
        output
            .write_all(render_guard(record).as_bytes())
            .map_err(|source| LocalProductError::Io {
                operation: "write rollback Core guard temporary",
                source,
            })?;
        output.sync_all().map_err(|source| LocalProductError::Io {
            operation: "sync rollback Core guard temporary",
            source,
        })?;
        let metadata =
            std::fs::symlink_metadata(&temporary).map_err(|source| LocalProductError::Io {
                operation: "inspect rollback Core guard temporary",
                source,
            })?;
        if !metadata.file_type().is_file() {
            return Err(LocalProductError::UpdateHold(
                "rollback Core guard temporary is not a regular file",
            ));
        }
        if metadata.permissions().mode() & 0o7777 != 0o600 {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&temporary, permissions).map_err(|source| {
                LocalProductError::Io {
                    operation: "set rollback Core guard temporary mode",
                    source,
                }
            })?;
        }
        drop(output);
        std::fs::rename(&temporary, &destination).map_err(|source| LocalProductError::Io {
            operation: "replace rollback Core guard",
            source,
        })?;
        sync_directory(&roots.state_root)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub(super) fn remove_guard_if_present(roots: &LocalCoreRoots) -> Result<(), LocalProductError> {
    let path = guard_path(roots);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect rollback Core guard for removal",
                source,
            })
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(LocalProductError::UpdateHold(
                "rollback Core guard to remove has an unsafe file type",
            ))
        }
        Ok(_) => {}
    }
    std::fs::remove_file(&path).map_err(|source| LocalProductError::Io {
        operation: "remove rollback Core guard",
        source,
    })?;
    sync_directory(&roots.state_root)
}

fn guard_matches_state(
    guard: &RollbackCoreGuardRecord,
    state: &m2_generation_state::GenerationPointerState,
) -> bool {
    state.current == guard.target_generation_id
        && state.previous.as_deref() == Some(guard.held_generation_id.as_str())
}

fn authoritative_state(
    roots: &LocalCoreRoots,
) -> Result<m2_generation_state::GenerationPointerState, LocalProductError> {
    let state_paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    m2_generation_state::recover_activation_state(&state_paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)
}

fn verify_guard_binding(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
    guard: &RollbackCoreGuardRecord,
) -> Result<
    (
        LocalReleaseManifest,
        LoadedLocalGeneration,
        LocalReleaseManifest,
        LoadedLocalGeneration,
    ),
    LocalProductError,
> {
    if !guard_matches_state(guard, state) {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard does not match authoritative activation state",
        ));
    }
    let current_key = state.current_key;
    let previous_key = state.previous_key.ok_or(LocalProductError::UpdateHold(
        "rollback Core guard has no retained held verifier authority",
    ))?;
    let (target_release, target_loaded) = verify_installed_local_release(
        roots,
        &state.current,
        current_key,
        "rollback Core guard target generation descriptor does not match current",
    )?;
    let (held_release, held_loaded) = verify_installed_local_release(
        roots,
        &guard.held_generation_id,
        previous_key,
        "rollback Core guard held generation descriptor does not match previous",
    )?;
    if target_loaded.generation_id != guard.target_generation_id
        || held_loaded.generation_id != guard.held_generation_id
        || held_release.release_sequence != guard.held_release_sequence
        || held_release.release_sequence <= target_release.release_sequence
        || held_loaded.core_path.is_none()
        || target_release.core_api_identity != held_release.core_api_identity
        || target_release.persistent_schema_identity != held_release.persistent_schema_identity
        || held_loaded.manifest.core_artifact_digest != guard.held_core_sha256
    {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard failed signed generation binding",
        ));
    }
    Ok((target_release, target_loaded, held_release, held_loaded))
}

fn verify_explicit_hold_binding(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
    hold: &UpdateHoldRecord,
) -> Result<(), LocalProductError> {
    if hold.generation_id == state.current {
        let (release, loaded) = verify_installed_local_release(
            roots,
            &state.current,
            state.current_key,
            "update hold current generation descriptor does not match current",
        )?;
        if loaded.generation_id != hold.generation_id
            || release.release_sequence != hold.release_sequence
        {
            return Err(LocalProductError::UpdateHold(
                "update hold does not match the authenticated current generation",
            ));
        }
        return Ok(());
    }
    if state.previous.as_deref() == Some(hold.generation_id.as_str()) {
        let previous_key = state.previous_key.ok_or(LocalProductError::UpdateHold(
            "update hold retained previous generation has no verifier authority",
        ))?;
        let (current_release, _) = verify_installed_local_release(
            roots,
            &state.current,
            state.current_key,
            "update hold active generation descriptor does not match current",
        )?;
        let (held_release, held_loaded) = verify_installed_local_release(
            roots,
            &hold.generation_id,
            previous_key,
            "update hold held generation descriptor does not match previous",
        )?;
        if held_loaded.generation_id != hold.generation_id
            || held_release.release_sequence != hold.release_sequence
            || held_release.release_sequence <= current_release.release_sequence
        {
            return Err(LocalProductError::UpdateHold(
                "update hold does not match the authenticated retained previous generation",
            ));
        }
        return Ok(());
    }
    Err(LocalProductError::UpdateHold(
        "update hold generation is neither current nor retained previous",
    ))
}

pub(super) fn effective_update_hold(
    roots: &LocalCoreRoots,
) -> Result<Option<UpdateHoldRecord>, LocalProductError> {
    let explicit = read_update_hold(roots)?;
    let guard = read_guard(roots)?;
    if explicit.is_none() && guard.is_none() {
        return Ok(None);
    }
    let state = authoritative_state(roots)?;
    if let Some(hold) = explicit.as_ref() {
        verify_explicit_hold_binding(roots, &state, hold)?;
    }
    if let Some(guard) = guard.as_ref() {
        let _ = verify_guard_binding(roots, &state, guard)?;
        if let Some(hold) = explicit.as_ref() {
            if hold.generation_id != guard.held_generation_id
                || hold.release_sequence != guard.held_release_sequence
            {
                return Err(LocalProductError::UpdateHold(
                    "update hold does not match rollback Core guard",
                ));
            }
        }
    }
    if let Some(hold) = explicit {
        return Ok(Some(hold));
    }
    let guard = guard.expect("guard is present when explicit hold is absent");
    Ok(Some(UpdateHoldRecord {
        generation_id: guard.held_generation_id,
        release_sequence: guard.held_release_sequence,
    }))
}

pub(super) fn target_core_supports_hold(
    target_core: Option<&std::path::Path>,
) -> Result<bool, LocalProductError> {
    #[cfg(test)]
    if let Some(value) = std::env::var_os(TEST_HOLD_CAPABILITY_ENV) {
        return match value.as_encoded_bytes() {
            b"1" => Ok(true),
            b"0" => Ok(false),
            _ => Err(LocalProductError::UpdateHold(
                "test rollback hold capability override is invalid",
            )),
        };
    }
    let Some(target_core) = target_core else {
        return Ok(false);
    };
    inspect_core_entrypoint_source(target_core, "inspect rollback target Core capability")?;
    let mut child = std::process::Command::new(target_core)
        .args(["update", "--help"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|source| LocalProductError::Io {
            operation: "query rollback target Core capability",
            source,
        })?;
    let stdout = child.stdout.take().ok_or(LocalProductError::UpdateHold(
        "rollback target Core capability stdout is unavailable",
    ))?;
    let reader = std::thread::spawn(move || -> std::io::Result<(Vec<u8>, bool)> {
        let mut input = stdout;
        let mut captured = Vec::with_capacity(CAPABILITY_OUTPUT_MAX_BYTES + 1);
        let mut buffer = [0u8; 4096];
        let mut oversized = false;
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let remaining = (CAPABILITY_OUTPUT_MAX_BYTES + 1).saturating_sub(captured.len());
            let take = remaining.min(read);
            captured.extend_from_slice(&buffer[..take]);
            if captured.len() > CAPABILITY_OUTPUT_MAX_BYTES || take < read {
                oversized = true;
            }
        }
        Ok((captured, oversized))
    });
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(CAPABILITY_TIMEOUT_SECONDS);
    let status = loop {
        match child.try_wait().map_err(|source| LocalProductError::Io {
            operation: "poll rollback target Core capability",
            source,
        })? {
            Some(status) => break status,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(LocalProductError::UpdateHold(
                    "rollback target Core capability probe timed out",
                ));
            }
            None => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    };
    let (stdout, oversized) = reader
        .join()
        .map_err(|_| {
            LocalProductError::UpdateHold("rollback target Core capability reader failed")
        })?
        .map_err(|source| LocalProductError::Io {
            operation: "read rollback target Core capability",
            source,
        })?;
    if !status.success() {
        return Ok(false);
    }
    if oversized {
        return Err(LocalProductError::UpdateHold(
            "rollback target Core capability output exceeds its byte bound",
        ));
    }
    let text = std::str::from_utf8(&stdout).map_err(|_| {
        LocalProductError::UpdateHold("rollback target Core capability output is not UTF-8")
    })?;
    Ok(text.lines().any(|line| line == UPDATE_HOLD_CAPABILITY_LINE))
}

pub(super) fn guard_record_for_rollback(
    before: &m2_generation_state::GenerationPointerState,
    after: &m2_generation_state::GenerationPointerState,
    current_release: &LocalReleaseManifest,
    current_loaded: &LoadedLocalGeneration,
    target_release: &LocalReleaseManifest,
    target_loaded: &LoadedLocalGeneration,
) -> Result<RollbackCoreGuardRecord, LocalProductError> {
    if before.current == after.current
        || after.previous.as_deref() != Some(before.current.as_str())
        || current_loaded.generation_id != before.current
        || target_loaded.generation_id != after.current
        || current_loaded.core_path.is_none()
        || current_release.release_sequence <= target_release.release_sequence
        || current_release.core_api_identity != target_release.core_api_identity
        || current_release.persistent_schema_identity != target_release.persistent_schema_identity
        || !valid_sha256_hex(&current_loaded.manifest.core_artifact_digest)
    {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard cannot bind the authenticated rollback pair",
        ));
    }
    Ok(RollbackCoreGuardRecord {
        target_generation_id: after.current.clone(),
        held_generation_id: before.current.clone(),
        held_release_sequence: current_release.release_sequence,
        held_core_sha256: current_loaded.manifest.core_artifact_digest.clone(),
    })
}

fn guarded_core_binding_for_state(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
    active_loaded: &LoadedLocalGeneration,
    observed_core_digest: &str,
) -> Result<Option<(String, String)>, LocalProductError> {
    let Some(guard) = read_guard(roots)? else {
        return Ok(None);
    };
    let (_, target_loaded, _, held_loaded) = verify_guard_binding(roots, state, &guard)?;
    if active_loaded.generation_id != target_loaded.generation_id
        || held_loaded.manifest.core_artifact_digest != observed_core_digest
    {
        return Err(LocalProductError::UpdateHold(
            "rollback Core guard failed stable Core provenance binding",
        ));
    }
    Ok(Some((guard.held_generation_id, guard.held_core_sha256)))
}

pub(super) fn guarded_core_binding_with_state(
    roots: &LocalCoreRoots,
    state: &m2_generation_state::GenerationPointerState,
    active_loaded: &LoadedLocalGeneration,
    observed_core_digest: &str,
) -> Result<Option<(String, String)>, LocalProductError> {
    guarded_core_binding_for_state(roots, state, active_loaded, observed_core_digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_core_guard_format_is_exact_and_fail_closed() {
        let record = RollbackCoreGuardRecord {
            target_generation_id: "target-1".to_owned(),
            held_generation_id: "held-2".to_owned(),
            held_release_sequence: 11,
            held_core_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
        };
        let rendered = render_guard(&record);
        assert_eq!(
            rendered,
            "codex-rollback-core-guard-v1\ntarget_generation_id\ttarget-1\nheld_generation_id\theld-2\nheld_release_sequence\t11\nheld_core_sha256\taaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n"
        );
        assert_eq!(parse_guard(rendered.as_bytes()).unwrap(), record);
        assert!(parse_guard(rendered.trim_end().as_bytes()).is_err());
        assert!(parse_guard(rendered.replace("11", "011").as_bytes()).is_err());
        assert!(parse_guard(rendered.replace("target-1", "held-2").as_bytes()).is_err());
    }
}
