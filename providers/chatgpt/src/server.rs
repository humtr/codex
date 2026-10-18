//! Loopback Responses endpoint. Auth, admission and disconnect cancellation are
//! independent of the app transport and exercise the actual production parser.
use crate::{
    config::Config,
    protocol::{Error, Request},
    responses::{Event, Projection},
};
use axum::{
    body::Body,
    extract::{Request as HttpRequest, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event as SseEvent, Sse},
        IntoResponse, Response,
    },
    Router,
};
use futures_util::{future::BoxFuture, StreamExt};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{
    convert::Infallible,
    io::Read,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub trait Host: Send + Sync + 'static {
    fn exchange(
        &self,
        request: Request,
        events: mpsc::Sender<Event>,
        cancel: CancellationToken,
    ) -> BoxFuture<'static, Result<(), Error>>;
}

pub struct Service {
    config: Config,
    config_path: Option<std::path::PathBuf>,
    shutdown: CancellationToken,
    token: String,
    host: Arc<dyn Host>,
    sequence: AtomicU64,
    viewer: Option<Arc<crate::viewer::Viewer>>,
    mcp: Option<Arc<crate::mcp::Broker>>,
}
impl Service {
    pub fn new(config: Config, token: String, host: Arc<dyn Host>) -> Result<Arc<Self>, Error> {
        config
            .validate()
            .map_err(|_| Error("invalid_provider_configuration"))?;
        if token.len() < 32
            || token.len() > 256
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error("invalid_server_token"));
        }
        Ok(Arc::new(Self {
            config,
            config_path: None,
            shutdown: CancellationToken::new(),
            token,
            host,
            sequence: AtomicU64::new(0),
            viewer: None,
            mcp: None,
        }))
    }
    pub fn configured(
        path: std::path::PathBuf,
        token: String,
        host: Arc<dyn Host>,
    ) -> Result<Arc<Self>, Error> {
        let config = crate::store::Snapshot::open(&path)
            .and_then(|v| v.config())
            .map_err(|_| Error("invalid_provider_configuration"))?;
        let mut service = Self::new(config, token, host)?;
        Arc::get_mut(&mut service).unwrap().config_path = Some(path);
        Ok(service)
    }
    fn effective(&self) -> Result<Config, Error> {
        match &self.config_path {
            Some(path) => crate::store::Snapshot::open(path)
                .and_then(|v| v.config())
                .map_err(|_| Error("invalid_provider_configuration"))
                .and_then(|value| {
                    if value.transport.return_path != self.config.transport.return_path
                        || value.transport.connector_name != self.config.transport.connector_name
                    {
                        Err(Error("provider_transport_restart_required"))
                    } else {
                        Ok(value)
                    }
                }),
            None => Ok(self.config.clone()),
        }
    }
    pub fn router(self: Arc<Self>) -> Router {
        let viewer = self.viewer.clone();
        let mcp = self.mcp.clone();
        let router = Router::new().fallback(handler).with_state(self);
        let router = match mcp {
            Some(mcp) => router.merge(mcp.router()),
            None => router,
        };
        match viewer {
            Some(viewer) => router.merge(viewer.router()),
            None => router,
        }
    }
    pub fn with_mcp(mut self: Arc<Self>, mcp: Arc<crate::mcp::Broker>) -> Result<Arc<Self>, Error> {
        Arc::get_mut(&mut self)
            .ok_or(Error("provider_already_shared"))?
            .mcp = Some(mcp);
        Ok(self)
    }
    pub fn with_viewer(
        mut self: Arc<Self>,
        viewer: Arc<crate::viewer::Viewer>,
    ) -> Result<Arc<Self>, Error> {
        Arc::get_mut(&mut self)
            .ok_or(Error("provider_already_shared"))?
            .viewer = Some(viewer);
        Ok(self)
    }
}

fn failure(status: StatusCode, code: &'static str) -> Response {
    (
        status,
        axum::Json(json!({"error":{"message":code,"type":"invalid_request_error","code":code}})),
    )
        .into_response()
}
fn one_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(value)
}
fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(given) = one_header(headers, "authorization") else {
        return false;
    };
    let expected = format!("Bearer {token}");
    // HMAC verification uses constant-time tag equality, including unequal input lengths.
    let mut mac = Hmac::<Sha256>::new_from_slice(token.as_bytes()).unwrap();
    mac.update(expected.as_bytes());
    let tag = mac.finalize().into_bytes();
    let mut check = Hmac::<Sha256>::new_from_slice(token.as_bytes()).unwrap();
    check.update(given.as_bytes());
    check.verify_slice(&tag).is_ok()
}
async fn read_body(
    headers: &HeaderMap,
    body: Body,
    config: &Config,
) -> Result<Value, (StatusCode, &'static str)> {
    let wire_limit = config.transport.max_wire_bytes;
    if headers.contains_key("content-length") {
        let size = one_header(headers, "content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or((StatusCode::BAD_REQUEST, "invalid_body_length"))?;
        if size > wire_limit {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, "wire_body_too_large"));
        }
    }
    let encoding = if headers.contains_key("content-encoding") {
        one_header(headers, "content-encoding")
            .ok_or((StatusCode::BAD_REQUEST, "invalid_body_encoding"))?
    } else {
        "identity"
    };
    if !matches!(encoding, "identity" | "zstd") {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_body_encoding",
        ));
    }
    let mut bytes = Vec::new();
    let mut stream = body.into_data_stream();
    while let Some(part) = stream.next().await {
        let part = part.map_err(|_| (StatusCode::BAD_REQUEST, "body_read_failed"))?;
        if part.len() as u64 > wire_limit.saturating_sub(bytes.len() as u64) {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, "wire_body_too_large"));
        }
        bytes.extend_from_slice(&part);
    }
    let decoded_limit = config.transport.max_decoded_bytes;
    if encoding == "zstd" {
        bytes = tokio::task::spawn_blocking(move || {
            let reader = zstd::stream::read::Decoder::new(bytes.as_slice())
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_zstd_body"))?;
            let mut decoded = Vec::new();
            reader
                .take(decoded_limit + 1)
                .read_to_end(&mut decoded)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_zstd_body"))?;
            if decoded.len() as u64 > decoded_limit {
                return Err((StatusCode::PAYLOAD_TOO_LARGE, "decoded_body_too_large"));
            }
            Ok(decoded)
        })
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "body_decode_failed"))??;
    } else if bytes.len() as u64 > decoded_limit {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "decoded_body_too_large"));
    }
    serde_json::from_slice(&bytes).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_json_body"))
}

struct Events {
    receiver: mpsc::Receiver<Result<SseEvent, Infallible>>,
    cancel: CancellationToken,
}
impl futures_util::Stream for Events {
    type Item = Result<SseEvent, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
impl Drop for Events {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
fn sse(value: Value) -> Result<SseEvent, Infallible> {
    Ok(SseEvent::default()
        .event(value["type"].as_str().unwrap_or("error"))
        .data(value.to_string()))
}

async fn handler(State(service): State<Arc<Service>>, request: HttpRequest) -> Response {
    let (parts, body) = request.into_parts();
    if parts.headers.contains_key("origin") {
        return failure(StatusCode::UNAUTHORIZED, "origin_not_allowed");
    }
    if parts.uri.path() == "/__ready" && parts.method == "GET" {
        let Some(challenge) = one_header(&parts.headers, "x-termux-challenge")
            .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        else {
            return failure(StatusCode::BAD_REQUEST, "invalid_readiness_challenge");
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(service.token.as_bytes()).unwrap();
        mac.update(format!("termux-codex-ready-v1:{challenge}").as_bytes());
        let proof = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        return (
            [("cache-control", "no-store")],
            axum::Json(json!({"schema":"termux-codex-provider-v1","proof":proof})),
        )
            .into_response();
    }
    if !authorized(&parts.headers, &service.token) {
        return failure(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let config = match service.effective() {
        Ok(config) => config,
        Err(error) => return failure(StatusCode::SERVICE_UNAVAILABLE, error.0),
    };
    if parts.uri.path() == "/v1/models" && parts.method == "GET" {
        return axum::Json(json!({"object":"list","data":config.catalog.models.iter().map(|m| json!({"id":m.id,"object":"model","owned_by":"termux-codex"})).collect::<Vec<_>>()})).into_response();
    }
    if parts.uri.path() != "/v1/responses" || parts.method != "POST" {
        return failure(StatusCode::NOT_FOUND, "not_found");
    }
    let body = match tokio::time::timeout(
        Duration::from_secs(30),
        read_body(&parts.headers, body, &config),
    )
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err((status, code))) => return failure(status, code),
        Err(_) => return failure(StatusCode::REQUEST_TIMEOUT, "body_read_timeout"),
    };
    let thread = one_header(&parts.headers, "session_id")
        .or_else(|| one_header(&parts.headers, "x-codex-thread-id"));
    let parsed = match Request::parse(body, &config, thread) {
        Ok(value) => value,
        Err(error) => return failure(StatusCode::BAD_REQUEST, error.0),
    };
    let cancel = service.shutdown.child_token();
    let (events, mut input) = mpsc::channel(64);
    let (output, receiver) = mpsc::channel(64);
    let host = service.host.clone();
    let selected = parsed.clone();
    let stopped = cancel.clone();
    let mut worker = tokio::spawn(async move { host.exchange(selected, events, stopped).await });
    let mut projection = Projection::new(
        format!(
            "resp_termux_{}_{}",
            std::process::id(),
            service.sequence.fetch_add(1, Ordering::Relaxed)
        ),
        parsed.model,
    );
    let stopped = cancel.clone();
    tokio::spawn(async move {
        for frame in projection.start() {
            if output.send(sse(frame)).await.is_err() {
                stopped.cancel();
                return;
            }
        }
        loop {
            let event = tokio::select! {
                _ = stopped.cancelled() => break,
                event = input.recv() => event,
            };
            match event {
                Some(event) => match projection.push(event) {
                    Ok(frames) => {
                        for frame in frames {
                            if output.send(sse(frame)).await.is_err() {
                                stopped.cancel();
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        let _ = output.send(sse(projection.fail(error))).await;
                        stopped.cancel();
                        break;
                    }
                },
                None => break,
            }
            if projection.completed() {
                break;
            }
        }
        if stopped.is_cancelled() {
            // The host owns acknowledged cancellation. Do not abort its cleanup
            // simply because the HTTP observer vanished.
            if tokio::time::timeout(Duration::from_secs(15), &mut worker)
                .await
                .is_err()
            {
                worker.abort();
            }
        } else {
            let result = worker.await;
            if !projection.completed() {
                let error = match result {
                    Ok(Err(error)) => error,
                    _ => Error("host_stream_incomplete"),
                };
                let _ = output.send(sse(projection.fail(error))).await;
            }
        }
    });
    Sse::new(Events { receiver, cancel })
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(2))
                // Codex times out waiting for parsed data events; SSE comments
                // alone do not reset that deadline. This is transport liveness,
                // never model progress or a public transcript item.
                .event(SseEvent::default().event("ping").data(r#"{"type":"ping"}"#)),
        )
        .into_response()
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    service: Arc<Service>,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    if !listener.local_addr()?.ip().is_loopback() {
        return Err(std::io::Error::other("provider must bind loopback"));
    }
    let stop = service.shutdown.clone();
    let viewer = service.viewer.clone();
    let mcp = service.mcp.clone();
    axum::serve(listener, service.router())
        .with_graceful_shutdown(async move {
            shutdown.cancelled().await;
            stop.cancel();
            if let Some(mcp) = mcp {
                mcp.close().await;
            }
            if let Some(viewer) = viewer {
                viewer.close();
            }
        })
        .await
}
