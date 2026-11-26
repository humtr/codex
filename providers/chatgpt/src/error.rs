//! Structured errors for `chatgpt-provider`.

use axum::{
    http::{header, HeaderValue, StatusCode},
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
    pub retry_after: Option<String>,
}

impl ProviderError {
    pub fn bad_request(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::BAD_REQUEST,
            retry_after: None,
        }
    }

    pub fn unauthorized(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::UNAUTHORIZED,
            retry_after: None,
        }
    }

    pub fn rate_limited(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::TOO_MANY_REQUESTS,
            retry_after: None,
        }
    }

    pub fn unavailable(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
            retry_after: None,
        }
    }

    pub fn internal(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            retry_after: None,
        }
    }

    pub fn with_retry_after(mut self, retry_after: Option<String>) -> Self {
        self.retry_after = retry_after.and_then(|s| {
            let trimmed = s.trim();
            if !trimmed.is_empty() && trimmed.len() <= 64 && trimmed.is_ascii() {
                Some(trimmed.to_owned())
            } else {
                None
            }
        });
        self
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
        let mut resp = (
            self.status,
            Json(json!({
                "state": "error",
                "error": self.code,
                "detail": self.detail
            })),
        )
            .into_response();
        if let Some(ra) = self.retry_after.as_deref() {
            if let Ok(hv) = HeaderValue::from_str(ra) {
                resp.headers_mut().insert(header::RETRY_AFTER, hv);
            }
        }
        resp
    }
}

pub type Result<T> = std::result::Result<T, ProviderError>;
