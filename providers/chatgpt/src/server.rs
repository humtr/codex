//! Unified localhost-only HTTP router composing:
//! - `GET /healthz`
//! - Type 1 OpenAI Responses API: `GET /v1/models`, `POST /v1/responses`
//! - Local authenticated `/control/*` control plane for `@TermuxChatGPT`
//! - Optional capability-scoped `/viewer/{capability}/{*path}` noVNC proxy

use crate::config::{HistoryConfig, McpConfig, ModelConfig};
use crate::control::{control_router, ControlState};
use crate::driver::DriverBackend;
use crate::error::Result;
use crate::responses::{
    handle_models, handle_responses_with_metadata, new_live_model_set, LiveModelSet,
    ResponseThreadMap,
};
use crate::viewer::{viewer_router, ViewerSlot};
use axum::{
    extract::State,
    http::HeaderMap,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
struct AppState {
    driver: Arc<dyn DriverBackend>,
    model_cfg: Arc<ModelConfig>,
    history_cfg: Arc<HistoryConfig>,
    mcp_cfg: Arc<McpConfig>,
    threads: Arc<ResponseThreadMap>,
    catalog_path: Option<PathBuf>,
    live_models: LiveModelSet,
    sse_idle: Duration,
}

#[cfg(test)]
pub fn build_router(
    driver: Arc<dyn DriverBackend>,
    model_cfg: ModelConfig,
    history_cfg: HistoryConfig,
    control_token: String,
    viewer_slot: ViewerSlot,
) -> Router {
    build_router_with_threads(
        driver,
        model_cfg,
        history_cfg,
        McpConfig::default(),
        control_token,
        viewer_slot,
        ResponseThreadMap::default(),
        None,
        crate::stream::SSE_IDLE,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_router_with_threads(
    driver: Arc<dyn DriverBackend>,
    model_cfg: ModelConfig,
    history_cfg: HistoryConfig,
    mcp_cfg: McpConfig,
    control_token: String,
    viewer_slot: ViewerSlot,
    threads: ResponseThreadMap,
    catalog_path: Option<PathBuf>,
    sse_idle: Duration,
) -> Router {
    let live_models =
        new_live_model_set(model_cfg.available.iter().cloned(), catalog_path.as_deref());
    let model_cfg = Arc::new(model_cfg);
    let parallel_tool_calls = false;
    let app_state = AppState {
        driver: Arc::clone(&driver),
        model_cfg: Arc::clone(&model_cfg),
        history_cfg: Arc::new(history_cfg),
        mcp_cfg: Arc::new(mcp_cfg),
        threads: Arc::new(threads),
        catalog_path: catalog_path.clone(),
        live_models: Arc::clone(&live_models),
        sse_idle,
    };
    let v1_router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(v1_models))
        .route("/v1/responses", post(v1_responses))
        .with_state(app_state);

    let ctrl_router = control_router(ControlState {
        driver,
        token: control_token.into(),
        catalog_path,
        model_cfg,
        live_models,
        parallel_tool_calls,
    });

    v1_router
        .merge(ctrl_router)
        .merge(viewer_router(viewer_slot))
}

async fn healthz(State(st): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "chatgpt-provider",
        "version": env!("CARGO_PKG_VERSION"),
        "host_running": st.driver.host_running().await
    }))
}

async fn v1_models(State(st): State<AppState>) -> Json<Value> {
    handle_models(
        Arc::clone(&st.driver),
        &st.model_cfg,
        st.catalog_path.as_deref(),
        Some(&st.live_models),
        false,
    )
    .await
}

async fn v1_responses(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    let session_hint = headers
        .get("thread-id")
        .or_else(|| headers.get("session-id"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);
    // Upstream carries the authoritative metadata in the request body; the
    // bounded HTTP header is a compatibility projection (and can be absent).
    let parse_metadata = |s: &str| {
        serde_json::from_str::<Value>(s)
            .ok()
            .filter(Value::is_object)
    };
    let turn_metadata = body["client_metadata"]["x-codex-turn-metadata"]
        .as_str()
        .and_then(parse_metadata)
        .or_else(|| {
            headers
                .get("x-codex-turn-metadata")
                .and_then(|v| v.to_str().ok())
                .and_then(parse_metadata)
        });
    handle_responses_with_metadata(
        Arc::clone(&st.driver),
        &st.model_cfg,
        &st.history_cfg,
        &st.mcp_cfg,
        Arc::clone(&st.threads),
        session_hint,
        turn_metadata,
        Some(&st.live_models),
        st.sse_idle,
        body,
    )
    .await
}
