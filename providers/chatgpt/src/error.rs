//! Structured errors for `chatgpt-provider`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ProviderError {
    pub code: &'static str,
    pub detail: String,
    pub status: StatusCode,
}

impl ProviderError {
    pub fn bad_request(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn unauthorized(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn unavailable(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub fn internal(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ProviderError {}

impl IntoResponse for ProviderError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "state": "error",
                "error": self.code,
                "detail": self.detail
            })),
        )
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, ProviderError>;
