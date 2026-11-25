//! Authenticated local `/control/*` HTTP API (consumed by `@TermuxChatGPT`).
//!
//! Every `/control/*` endpoint requires `Authorization: Bearer <token>` matching
//! the `0600` token at `<state_dir>/control.token` in constant time.

use crate::config::{constant_time_eq, ModelConfig};
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use crate::responses::{sync_catalog_from_status, LiveModelSet};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct ControlState {
    pub driver: Arc<dyn DriverBackend>,
    pub token: Arc<str>,
    pub catalog_path: Option<PathBuf>,
    pub model_cfg: Arc<ModelConfig>,
    pub live_models: LiveModelSet,
    pub parallel_tool_calls: bool,
}

fn require_bearer(headers: &HeaderMap, expected: &str) -> Result<()> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ProviderError::unauthorized(
                "control_auth_required",
                "missing Authorization: Bearer <token> header",
            )
        })?;
    let provided = raw.strip_prefix("Bearer ").ok_or_else(|| {
        ProviderError::unauthorized(
            "control_auth_invalid",
            "Authorization header must use Bearer scheme",
        )
    })?;
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(ProviderError::unauthorized(
            "control_auth_rejected",
            "invalid control bearer token",
        ));
    }
    Ok(())
}

pub fn control_router(state: ControlState) -> Router {
    Router::new()
        .route("/control/status", get(get_status))
        .route("/control/chats", get(list_chats))
        .route("/control/chats/{id}", get(read_chat))
        .route("/control/chats/{id}/rename", post(rename_chat))
        .route("/control/projects", get(list_projects).post(create_project))
        .route("/control/projects/{id}", get(read_project))
        .route("/control/send", post(send_message))
        .route("/control/observe", post(observe_chat))
        .route("/control/stop", post(stop_generation))
        .route("/control/viewer", get(get_viewer))
        .route("/control/viewer/open", post(open_viewer))
        .route("/control/host/restart", post(restart_host))
        .route(
            "/control/connectors",
            get(list_connectors).post(register_connector),
        )
        .route("/control/connectors/{id}/refresh", post(refresh_connector))
        .with_state(state)
}

#[derive(Deserialize)]
struct ListChatsQuery {
    limit: Option<u64>,
    cursor: Option<String>,
    archived: Option<bool>,
}

#[derive(Deserialize)]
struct ReadChatQuery {
    limit: Option<u64>,
}

#[derive(Deserialize)]
struct RenameBody {
    title: String,
}

#[derive(Deserialize)]
struct ListProjectsQuery {
    limit: Option<u64>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct CreateProjectBody {
    name: String,
    instructions: Option<String>,
    #[serde(default)]
    memory_scope: Option<String>,
}

async fn get_status(State(st): State<ControlState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let status = st.driver.status().await?;
    sync_catalog_from_status(
        st.catalog_path.as_deref(),
        &status,
        &st.model_cfg,
        Some(&st.live_models),
        st.parallel_tool_calls,
    );
    Ok(Json(status))
}

async fn list_chats(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Query(q): Query<ListChatsQuery>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    Ok(Json(
        st.driver
            .list_chats(json!({
                "limit": limit,
                "cursor": q.cursor,
                "archived": q.archived.unwrap_or(false)
            }))
            .await?,
    ))
}

fn is_safe_control_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && !id
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\' | '?' | '#'))
}

async fn read_chat(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<ReadChatQuery>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if !is_safe_control_id(&id) {
        return Err(ProviderError::bad_request(
            "invalid_chat_id",
            "invalid chat id",
        ));
    }
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    Ok(Json(
        st.driver
            .read_chat(json!({ "chat_id": id, "limit": limit }))
            .await?,
    ))
}

async fn rename_chat(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RenameBody>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let title = body.title.trim();
    if !is_safe_control_id(&id) || title.is_empty() || title.len() > 200 {
        return Err(ProviderError::bad_request(
            "invalid_rename_request",
            "chat_id and non-empty title <= 200 bytes required",
        ));
    }
    Ok(Json(
        st.driver
            .rename_chat(json!({ "chat_id": id, "title": title }))
            .await?,
    ))
}

async fn list_projects(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Query(q): Query<ListProjectsQuery>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    Ok(Json(
        st.driver
            .list_projects(json!({ "limit": limit, "cursor": q.cursor }))
            .await?,
    ))
}

async fn read_project(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<ListProjectsQuery>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if !is_safe_control_id(&id) {
        return Err(ProviderError::bad_request(
            "invalid_project_id",
            "invalid project id",
        ));
    }
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    Ok(Json(
        st.driver
            .read_project(json!({ "project_id": id, "limit": limit, "cursor": q.cursor }))
            .await?,
    ))
}

async fn create_project(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(ProviderError::bad_request(
            "invalid_project_name",
            "project name must be 1..=120 bytes",
        ));
    }
    if let Some(ref scope) = body.memory_scope {
        if !matches!(scope.as_str(), "unset" | "project_v2") {
            return Err(ProviderError::bad_request(
                "invalid_memory_scope",
                "memory_scope must be \"unset\" or \"project_v2\"",
            ));
        }
    }
    let mut params = json!({ "name": name, "instructions": body.instructions });
    if let Some(ref scope) = body.memory_scope {
        params["memory_scope"] = json!(scope);
    }
    Ok(Json(st.driver.create_project(params).await?))
}

async fn send_message(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if let Some(cid) = body.get("chat_id").and_then(Value::as_str) {
        if !is_safe_control_id(cid) {
            return Err(ProviderError::bad_request(
                "invalid_chat_id",
                "invalid chat_id",
            ));
        }
    }
    if let Some(pid) = body.get("project_id").and_then(Value::as_str) {
        if !is_safe_control_id(pid) {
            return Err(ProviderError::bad_request(
                "invalid_project_id",
                "invalid project_id",
            ));
        }
    }
    if let Some(lid) = body.get("lane_id").and_then(Value::as_str) {
        if !is_safe_control_id(lid) {
            return Err(ProviderError::bad_request(
                "invalid_lane_id",
                "invalid lane_id",
            ));
        }
    }
    if let Some(account) = body
        .get("expected_account_id")
        .filter(|value| !value.is_null())
    {
        if !account.as_str().is_some_and(is_safe_control_id) {
            return Err(ProviderError::bad_request(
                "invalid_expected_account_id",
                "expected_account_id must be a valid account id",
            ));
        }
    }
    let prompt = body
        .get("prompt")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty() && s.len() <= 64 * 1024)
        .ok_or_else(|| {
            ProviderError::bad_request("prompt_invalid", "prompt must be non-empty <= 64 KiB")
        })?;
    let wait_ms = body
        .get("wait_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000)
        .min(300_000);
    Ok(Json(
        st.driver
            .send_message(json!({
                "lane_id": body.get("lane_id"),
                "chat_id": body.get("chat_id"),
                "project_id": body.get("project_id"),
                "expected_account_id": body.get("expected_account_id"),
                "prompt": prompt,
                "model": body.get("model"),
                "reasoning_effort": body.get("reasoning_effort"),
                "wait_ms": wait_ms
            }))
            .await?,
    ))
}

async fn observe_chat(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if let Some(cid) = body.get("chat_id").and_then(Value::as_str) {
        if !is_safe_control_id(cid) {
            return Err(ProviderError::bad_request(
                "invalid_chat_id",
                "invalid chat_id",
            ));
        }
    }
    if let Some(lid) = body.get("lane_id").and_then(Value::as_str) {
        if !is_safe_control_id(lid) {
            return Err(ProviderError::bad_request(
                "invalid_lane_id",
                "invalid lane_id",
            ));
        }
    }
    Ok(Json(
        st.driver
            .observe_chat(json!({
                "chat_id": body.get("chat_id"),
                "lane_id": body.get("lane_id")
            }))
            .await?,
    ))
}

#[derive(Deserialize, Default)]
struct StopGenerationBody {
    chat_id: Option<String>,
    lane_id: Option<String>,
    all: Option<bool>,
}

async fn stop_generation(
    State(st): State<ControlState>,
    headers: HeaderMap,
    body: Option<Json<StopGenerationBody>>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let body = body.map(|Json(b)| b).unwrap_or_default();
    if let Some(cid) = body.chat_id.as_deref() {
        if !is_safe_control_id(cid) {
            return Err(ProviderError::bad_request(
                "invalid_chat_id",
                "invalid chat_id",
            ));
        }
    }
    if let Some(lid) = body.lane_id.as_deref() {
        if !is_safe_control_id(lid) {
            return Err(ProviderError::bad_request(
                "invalid_lane_id",
                "invalid lane_id",
            ));
        }
    }
    Ok(Json(
        st.driver
            .stop_generation(json!({
                "chat_id": body.chat_id,
                "lane_id": body.lane_id,
                "all": body.all.unwrap_or(false),
            }))
            .await?,
    ))
}

async fn get_viewer(State(st): State<ControlState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    Ok(Json(st.driver.viewer_info().await?))
}

async fn open_viewer(State(st): State<ControlState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let info = st.driver.viewer_info().await?;
    let url = info
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::unavailable(
                "viewer_unavailable",
                "viewer is not active (start host and ensure viewer assets are installed)",
            )
        })?
        .to_owned();
    let prefix = std::path::PathBuf::from(
        std::env::var_os("PREFIX")
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "/data/data/com.termux/files/usr".into()),
    );
    crate::host::open_android_url(&prefix, &url).await?;
    Ok(Json(json!({
        "state": "ok",
        "viewer_open_requested": true,
        "url": url
    })))
}

async fn restart_host(State(st): State<ControlState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    Ok(Json(st.driver.restart_host().await?))
}

#[derive(Deserialize)]
struct ListConnectorsQuery {
    account_id: Option<String>,
    sku: Option<String>,
    include_details: Option<bool>,
}

#[derive(Deserialize)]
struct RegisterConnectorBody {
    name: String,
    tunnel_id: String,
    description: Option<String>,
    account_id: Option<String>,
    sku: Option<String>,
    refresh: Option<bool>,
}

#[derive(Deserialize, Default)]
struct RefreshConnectorBody {
    account_id: Option<String>,
    sku: Option<String>,
    link_if_missing: Option<bool>,
}

fn is_valid_tunnel_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("tunnel_") else {
        return false;
    };
    let is_lower_alnum = |s: &str| {
        s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    };
    if rest.len() == 32 && is_lower_alnum(rest) {
        return true;
    }
    if let Some((prefix, suffix)) = rest.split_once('_') {
        return prefix.len() == 4
            && suffix.len() == 32
            && is_lower_alnum(prefix)
            && is_lower_alnum(suffix);
    }
    false
}

async fn list_connectors(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Query(q): Query<ListConnectorsQuery>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if let Some(aid) = q.account_id.as_deref() {
        if !is_safe_control_id(aid) {
            return Err(ProviderError::bad_request(
                "invalid_account_id",
                "invalid account_id",
            ));
        }
    }
    Ok(Json(
        st.driver
            .list_connectors(json!({
                "account_id": q.account_id,
                "sku": q.sku,
                "include_details": q.include_details.unwrap_or(true),
            }))
            .await?,
    ))
}

async fn register_connector(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Json(body): Json<RegisterConnectorBody>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(ProviderError::bad_request(
            "invalid_connector_name",
            "connector name must be 1..=120 bytes",
        ));
    }
    let tunnel_id = body.tunnel_id.trim();
    if !is_valid_tunnel_id(tunnel_id) {
        return Err(ProviderError::bad_request(
            "invalid_tunnel_id",
            "tunnel_id must match tunnel_[xxxx_]<32-lowercase-alnum>",
        ));
    }
    if let Some(aid) = body.account_id.as_deref() {
        if !is_safe_control_id(aid) {
            return Err(ProviderError::bad_request(
                "invalid_account_id",
                "invalid account_id",
            ));
        }
    }
    let description = body.description.unwrap_or_default();
    if description.len() > 500 {
        return Err(ProviderError::bad_request(
            "invalid_connector_description",
            "description must be <= 500 bytes",
        ));
    }
    Ok(Json(
        st.driver
            .register_connector(json!({
                "name": name,
                "tunnel_id": tunnel_id,
                "description": description,
                "account_id": body.account_id,
                "sku": body.sku,
                "refresh": body.refresh.unwrap_or(true),
            }))
            .await?,
    ))
}

async fn refresh_connector(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Option<Json<RefreshConnectorBody>>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if !is_safe_control_id(&id) {
        return Err(ProviderError::bad_request(
            "invalid_connector_id",
            "invalid connector id",
        ));
    }
    let body = body.map(|Json(b)| b).unwrap_or_default();
    if let Some(aid) = body.account_id.as_deref() {
        if !is_safe_control_id(aid) {
            return Err(ProviderError::bad_request(
                "invalid_account_id",
                "invalid account_id",
            ));
        }
    }
    Ok(Json(
        st.driver
            .refresh_connector(json!({
                "connector_id": id,
                "account_id": body.account_id,
                "sku": body.sku,
                "link_if_missing": body.link_if_missing.unwrap_or(false),
            }))
            .await?,
    ))
}
