//! Authenticated local `/control/*` HTTP API (consumed by Slice 6 `@TermuxChatGPT`).
//!
//! Every `/control/*` endpoint requires `Authorization: Bearer <token>` matching
//! the `0600` token at `<state_dir>/control.token` in constant time.

use crate::config::constant_time_eq;
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct ControlState {
    pub driver: Arc<dyn DriverBackend>,
    pub token: Arc<str>,
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
}

async fn get_status(State(st): State<ControlState>, headers: HeaderMap) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    Ok(Json(st.driver.status().await?))
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

async fn read_chat(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<ReadChatQuery>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    if id.is_empty() || id.len() > 128 || id.contains('/') {
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
    if id.is_empty() || id.len() > 128 || id.contains('/') || title.is_empty() || title.len() > 200
    {
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
    if id.is_empty() || id.len() > 128 || id.contains('/') {
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
    Ok(Json(
        st.driver
            .create_project(json!({ "name": name, "instructions": body.instructions }))
            .await?,
    ))
}

async fn send_message(
    State(st): State<ControlState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
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
                "chat_id": body.get("chat_id"),
                "project_id": body.get("project_id"),
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
    Ok(Json(
        st.driver
            .observe_chat(json!({ "chat_id": body.get("chat_id") }))
            .await?,
    ))
}

async fn stop_generation(
    State(st): State<ControlState>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    require_bearer(&headers, &st.token)?;
    Ok(Json(st.driver.stop_generation().await?))
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
