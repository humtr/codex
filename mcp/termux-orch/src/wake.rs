//! Local wake and health client for the supervised relay.
use crate::http_client::{parse_loopback_base_url, request_json};
use crate::ledger::read_private_file;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

pub fn health_url_file() -> Option<PathBuf> {
    std::env::var_os("ORCH_RELAY_HEALTH_URL_FILE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                PathBuf::from(home).join(".local/state/codex/services/termux-orch-relay/health.url")
            })
        })
}

pub fn request(wake: bool) -> Option<Value> {
    let path = health_url_file()?;
    let url = String::from_utf8(read_private_file(&path, 256).ok()?).ok()?;
    let endpoint = parse_loopback_base_url(url.trim()).ok()?;
    let token = if wake {
        let value =
            String::from_utf8(read_private_file(&path.with_file_name("wake.token"), 256).ok()?)
                .ok()?;
        let value = value.trim();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        Some(value.to_owned())
    } else {
        None
    };
    request_json(
        &endpoint,
        if wake { "POST" } else { "GET" },
        if wake { "/wake" } else { "/healthz" },
        token.as_deref(),
        None,
        Duration::from_millis(150),
    )
    .ok()?
    .json
}
