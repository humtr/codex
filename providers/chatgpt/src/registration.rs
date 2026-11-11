//! Explicit generated upstream profile/catalog output, with no account policy.
use crate::{
    cdp::private_read,
    config::Config,
    context::digest,
    journal::{private_directory, random_id},
    projection,
    protocol::Error,
};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::fs::{symlink, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

pub const PROVIDER: &str = "chatgpt_desktop";
pub const MARKER: &str = "# codex-chatgpt-provider generated v1\n";
fn read(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        _ => private_read(path, 4 * 1024 * 1024).map(Some),
    }
}
fn publish(path: &Path, bytes: &[u8], before: Option<&[u8]>) -> Result<(), Error> {
    if before == Some(bytes) {
        return Ok(());
    }
    let parent = path.parent().ok_or(Error("projection_path_invalid"))?;
    let temporary = parent.join(format!(".projection-{}", random_id()?));
    let result = (|| {
        let mut f = File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| Error("projection_write_failed"))?;
        f.write_all(bytes)
            .and_then(|()| f.sync_all())
            .map_err(|_| Error("projection_write_failed"))?;
        if read(path)?.as_deref() != before {
            return Err(Error("projection_changed"));
        }
        if before.is_none() {
            crate::store::publish_new(&temporary, path)
                .map_err(|_| Error("projection_publish_failed"))?;
        } else {
            fs::rename(&temporary, path).map_err(|_| Error("projection_publish_failed"))?;
        }
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error("projection_sync_failed"))
    })();
    let _ = fs::remove_file(&temporary);
    result
}
fn local_link(home: &Path, profile: &Path) -> Result<PathBuf, Error> {
    let m = fs::symlink_metadata(home).map_err(|_| Error("existing_codex_home_required"))?;
    if !home.is_absolute()
        || !m.is_dir()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o022 != 0
    {
        return Err(Error("owned_codex_home_required"));
    }
    let path = home.join("chatgpt.config.toml");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Ok(m)
            if m.file_type().is_symlink()
                && m.uid() == unsafe { libc::geteuid() }
                && fs::read_link(&path).ok().as_deref() == Some(profile) => {}
        _ => return Err(Error("existing_user_profile_preserved")),
    }
    Ok(path)
}
pub fn prepare(
    config: &Config,
    state: &Path,
    registry: &Path,
    home: &Path,
    binary: &Path,
    base_url: &str,
) -> Result<Value, Error> {
    config
        .validate()
        .map_err(|_| Error("invalid_provider_configuration"))?;
    base_url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|s| s.strip_suffix("/v1"))
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|p| *p > 0)
        .ok_or(Error("owned_provider_endpoint_required"))?;
    if !state.is_absolute() || !registry.is_absolute() || !binary.is_absolute() || !binary.is_file()
    {
        return Err(Error("projection_path_invalid"));
    }
    let profile = registry.join("chatgpt.config.toml");
    let local = local_link(home, &profile)?;
    private_directory(registry)?;
    let lock = File::open(registry).map_err(|_| Error("projection_registry_unavailable"))?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(Error("projection_busy"));
    }
    let before = read(&profile)?;
    if before
        .as_ref()
        .is_some_and(|v| !v.starts_with(MARKER.as_bytes()))
    {
        return Err(Error("existing_user_profile_preserved"));
    }
    let mut routing = native_routing(config, state, binary, base_url)?;
    routing["model"] = json!(config.model.default);
    routing["model_provider"] = json!(PROVIDER);
    routing["model_reasoning_effort"] = json!(config.model.reasoning_effort);
    let catalog = routing["model_catalog_json"].clone();
    let text = MARKER.to_owned()
        + &toml::to_string_pretty(&routing).map_err(|_| Error("profile_serialization_failed"))?;
    // Recheck the local target before changing the shared output. Existing user
    // config/auth files are never opened for writing or materialized over.
    local_link(home, &profile)?;
    publish(&profile, text.as_bytes(), before.as_deref())?;
    match symlink(&profile, &local) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            local_link(home, &profile)?;
        }
        Err(_) => return Err(Error("profile_registration_failed")),
    }
    Ok(
        json!({"prepared":true,"profile":profile,"catalog":catalog,"codex_home":home,"provider":PROVIDER,"native_defaults_changed":false,"app_inference_submitted":false}),
    )
}

/// Native thread routing needs a catalog, not publication into user profiles.
pub fn native_routing(
    config: &Config,
    state: &Path,
    binary: &Path,
    base_url: &str,
) -> Result<Value, Error> {
    config
        .validate()
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let port = base_url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|s| s.strip_suffix("/v1"))
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|p| *p > 0)
        .ok_or(Error("owned_provider_endpoint_required"))?;
    if !state.is_absolute() || !binary.is_absolute() || !binary.is_file() {
        return Err(Error("projection_path_invalid"));
    }
    let metadata = projection::catalog(config).map_err(|_| Error("native_catalog_invalid"))?;
    let catalog = catalog_snapshot(state, &metadata)?;
    Ok(
        json!({"model_catalog_json":catalog,"model_providers":{(PROVIDER):{"name":"ChatGPT Linux app","base_url":format!("http://127.0.0.1:{port}/v1"),"wire_api":"responses","requires_openai_auth":false,"supports_websockets":false,"request_max_retries":0,"stream_max_retries":0,"auth":{"command":binary,"args":["token","--state",state],"timeout_ms":60000,"refresh_interval_ms":1000}}}}),
    )
}

/// One immutable snapshot producer is shared by preparation and live discovery.
pub(crate) fn catalog_snapshot(state: &Path, metadata: &Value) -> Result<PathBuf, Error> {
    let bytes = serde_json::to_vec(metadata).map_err(|_| Error("native_catalog_invalid"))?;
    let snapshots = state.join("catalogs");
    private_directory(&snapshots)?;
    let catalog = snapshots.join(format!("{}.json", digest(&metadata)));
    if let Some(old) = read(&catalog)? {
        if old != bytes {
            return Err(Error("catalog_snapshot_conflict"));
        }
    } else {
        publish(&catalog, &bytes, None)?;
    }
    Ok(catalog)
}

/// Upgrade the fresh reply of an older local service through the same catalog
/// producer. The exact account-bound App observation must still be present;
/// retained/configuration-only lists are never promoted to current entitlement.
pub fn current_selection_reply(
    config_path: &Path,
    state: &Path,
    reply: &Value,
) -> Result<Value, Error> {
    if reply["source"] != "app_and_config"
        || reply["inference_submitted"] != false
        || reply.get("schema").is_some()
    {
        return Err(Error("current_app_catalog_required"));
    }
    let identity = crate::app::Identity::parse(&reply["identity"])?;
    let observed = reply["observed_at_unix_ms"]
        .as_u64()
        .ok_or(Error("current_app_catalog_required"))?;
    let cached: crate::models::AppSnapshot = serde_json::from_value(
        crate::journal::Journal::inspect(state, &crate::models::snapshot_key())?
            .ok_or(Error("current_app_catalog_required"))?,
    )
    .map_err(|_| Error("app_model_catalog_invalid"))?;
    cached.identity.validate()?;
    if !identity.account_matches(&cached.identity) || cached.observed_at_unix_ms != observed {
        return Err(Error("app_catalog_observation_changed"));
    }
    let snapshot = crate::store::Snapshot::open(config_path)
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let mut config = snapshot
        .config()
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let configured = snapshot
        .configured_models()
        .map_err(|_| Error("invalid_provider_configuration"))?;
    config.catalog = config
        .catalog
        .with_app_options(&cached.options, &configured)?;
    if !snapshot
        .unchanged()
        .map_err(|_| Error("invalid_provider_configuration"))?
    {
        return Err(Error("provider_configuration_changed"));
    }
    selection_reply(&config, state, &reply["identity"], observed)
}

pub(crate) fn selection_reply(
    config: &Config,
    state: &Path,
    identity: &Value,
    observed_at_unix_ms: u64,
) -> Result<Value, Error> {
    crate::app::Identity::parse(identity)?;
    let native_catalog =
        projection::selection_catalog(config).map_err(|_| Error("native_catalog_invalid"))?;
    let native_catalog_path = catalog_snapshot(state, &native_catalog)?;
    Ok(
        json!({"schema":"codex-chatgpt-app-models-v1", "catalog":config.catalog,
        "native_catalog":native_catalog, "native_catalog_path":native_catalog_path,
        "identity":identity, "observed_at_unix_ms":observed_at_unix_ms,
        "source":"app_and_config", "account_verified_now":true, "inference_submitted":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_service_projection_requires_the_exact_fresh_account_observation() {
        let root = std::env::temp_dir().join(format!("model-upgrade-{}", random_id().unwrap()));
        private_directory(&root).unwrap();
        let state = root.join("state");
        let journal = crate::journal::Journal::open(&state).unwrap();
        let identity = json!({"generation":"fixture-generation", "account":"fixture-account", "user":"fixture-user", "structure":"personal"});
        let options = json!([{"label":"Server family", "options":[
            {"slug":"offered-thinking", "lane":"thinking", "modelLabel":"Thinking", "thinkingEffort":"extended", "isAvailable":true},
            {"slug":"revoked", "lane":"pro", "modelLabel":"Pro", "isAvailable":false}
        ]}]);
        let observation = crate::models::AppSnapshot {
            identity: crate::app::Identity::parse(&identity).unwrap(),
            options,
            observed_at_unix_ms: 71,
        };
        journal
            .write(&crate::models::snapshot_key(), &observation)
            .unwrap();
        let reply = json!({"source":"app_and_config", "inference_submitted":false,
            "identity":identity, "observed_at_unix_ms":71,
            "catalog":{"models":[{"id":"untrusted-configured-model"}]}});
        let config = root.join("missing-config.toml");
        let projected = current_selection_reply(&config, &state, &reply).unwrap();
        assert_eq!(projected["schema"], "codex-chatgpt-app-models-v1");
        assert_eq!(projected["account_verified_now"], true);
        let models = projected["native_catalog"]["models"].as_array().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["slug"], "chatgpt-desktop/app/offered-thinking");
        assert_eq!(models[0]["supported_reasoning_levels"][0]["effort"], "high");
        assert_eq!(
            serde_json::from_slice::<Value>(
                &fs::read(projected["native_catalog_path"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
            projected["native_catalog"]
        );
        for changed in [
            json!({"source":"configured_only"}),
            json!({"source":"retained_app_and_config"}),
            json!({"observed_at_unix_ms":70}),
            json!({"identity":{"generation":"fixture-generation", "account":"other-account", "user":"fixture-user", "structure":"personal"}}),
            json!({"inference_submitted":true}),
        ] {
            let mut invalid = reply.clone();
            invalid
                .as_object_mut()
                .unwrap()
                .extend(changed.as_object().unwrap().clone());
            assert!(current_selection_reply(&config, &state, &invalid).is_err());
        }
        assert!(!config.exists());
        drop(journal);
        fs::remove_dir_all(root).unwrap();
    }
}
