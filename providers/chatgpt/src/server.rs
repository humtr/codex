//! Unified localhost-only HTTP router composing:
//! - `GET /healthz`
//! - Type 1 OpenAI Responses API: `GET /v1/models`, `POST /v1/responses`
//! - Local authenticated `/control/*` control plane for Slice 6 `@TermuxChatGPT`
//! - Optional capability-scoped `/viewer/{capability}/{*path}` noVNC proxy

use crate::config::{HistoryConfig, ModelConfig};
use crate::control::{control_router, ControlState};
use crate::driver::DriverBackend;
use crate::error::Result;
use crate::responses::{handle_models, handle_responses, ResponseThreadMap};
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

#[derive(Clone)]
struct AppState {
    driver: Arc<dyn DriverBackend>,
    model_cfg: Arc<ModelConfig>,
    history_cfg: Arc<HistoryConfig>,
    threads: Arc<ResponseThreadMap>,
    catalog_path: Option<PathBuf>,
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
        control_token,
        viewer_slot,
        ResponseThreadMap::default(),
        None,
    )
}

pub fn build_router_with_threads(
    driver: Arc<dyn DriverBackend>,
    model_cfg: ModelConfig,
    history_cfg: HistoryConfig,
    control_token: String,
    viewer_slot: ViewerSlot,
    threads: ResponseThreadMap,
    catalog_path: Option<PathBuf>,
) -> Router {
    let model_cfg = Arc::new(model_cfg);
    let app_state = AppState {
        driver: Arc::clone(&driver),
        model_cfg: Arc::clone(&model_cfg),
        history_cfg: Arc::new(history_cfg),
        threads: Arc::new(threads),
        catalog_path: catalog_path.clone(),
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
    handle_responses(
        Arc::clone(&st.driver),
        &st.model_cfg,
        &st.history_cfg,
        Arc::clone(&st.threads),
        session_hint,
        body,
    )
    .await
}
