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
    let port = base_url
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
    let metadata = projection::catalog(config).map_err(|_| Error("native_catalog_invalid"))?;
    let bytes = serde_json::to_vec(&metadata).unwrap();
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
    let routing = json!({"model":config.model.default,"model_provider":PROVIDER,"model_reasoning_effort":config.model.reasoning_effort,"model_catalog_json":catalog,"model_providers":{(PROVIDER):{"name":"ChatGPT · Termux Provider","base_url":format!("http://127.0.0.1:{port}/v1"),"wire_api":"responses","requires_openai_auth":false,"supports_websockets":false,"request_max_retries":0,"stream_max_retries":0,"auth":{"command":binary,"args":["token","--state",state],"timeout_ms":10000,"refresh_interval_ms":1000}}}});
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
