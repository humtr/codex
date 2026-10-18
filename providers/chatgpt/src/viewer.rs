//! Optional loopback noVNC assets and a capability-bound bridge to our private
//! RFB socket. No app/auth/CDP API and no separately supervised runtime.
use crate::{journal::random_id, protocol::Error};
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        FromRequestParts, Path, Request, State,
    },
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{
    fs::File,
    io::Read,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path as FsPath, PathBuf},
    sync::Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Viewer {
    pub capability: String,
    socket: PathBuf,
    assets: PathBuf,
    port: u16,
    socket_identity: (u64, u64),
    shutdown: tokio_util::sync::CancellationToken,
}
fn owned_directory(path: &FsPath) -> Result<(), Error> {
    let m = std::fs::symlink_metadata(path).map_err(|_| Error("viewer_assets_unavailable"))?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err(Error("owned_viewer_directory_required"));
    }
    Ok(())
}
impl Viewer {
    pub fn new(socket: PathBuf, assets: PathBuf, port: u16) -> Result<Arc<Self>, Error> {
        if !socket.is_absolute() || !assets.is_absolute() || port == 0 {
            return Err(Error("viewer_binding_invalid"));
        }
        owned_directory(&assets)?;
        let m =
            std::fs::symlink_metadata(&socket).map_err(|_| Error("viewer_socket_unavailable"))?;
        if !m.file_type().is_socket()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != 0o600
        {
            return Err(Error("private_viewer_socket_required"));
        }
        let viewer = Arc::new(Self {
            capability: random_id()?,
            socket,
            assets,
            port,
            socket_identity: (m.dev(), m.ino()),
            shutdown: tokio_util::sync::CancellationToken::new(),
        });
        viewer.asset("vnc.html")?;
        viewer.asset("core/rfb.js")?;
        Ok(viewer)
    }
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/viewer/{capability}/{*path}", get(handle).post(handle))
            .with_state(self)
    }
    pub(crate) fn close(&self) {
        self.shutdown.cancel();
    }
    fn asset(&self, relative: &str) -> Result<Vec<u8>, Error> {
        let path = FsPath::new(relative);
        if relative.is_empty()
            || relative.len() > 512
            || relative.contains(['\\', '\0', '%'])
            || path
                .components()
                .any(|v| !matches!(v, Component::Normal(_)))
        {
            return Err(Error("viewer_asset_invalid"));
        }
        mime(path).ok_or(Error("viewer_asset_invalid"))?;
        let mut full = self.assets.clone();
        for part in path.components() {
            full.push(part);
            if full != self.assets.join(path) {
                owned_directory(&full)?;
            }
        }
        let mut file = File::options()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&full)
            .map_err(|_| Error("viewer_asset_unavailable"))?;
        let m = file
            .metadata()
            .map_err(|_| Error("viewer_asset_unavailable"))?;
        const LIMIT: u64 = 10 * 1024 * 1024;
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o022 != 0
            || m.len() > LIMIT
        {
            return Err(Error("viewer_asset_invalid"));
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error("viewer_asset_unavailable"))?;
        if bytes.len() as u64 > LIMIT {
            return Err(Error("viewer_asset_invalid"));
        }
        Ok(bytes)
    }
}
fn mime(path: &FsPath) -> Option<&'static str> {
    Some(match path.extension()?.to_str()? {
        "html" => "text/html;charset=utf-8",
        "js" => "text/javascript;charset=utf-8",
        "css" => "text/css;charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => return None,
    })
}
fn response(status: StatusCode, mime: &'static str, bytes: impl Into<Body>) -> Response {
    let mut response = (status, bytes.into()).into_response();
    let h = response.headers_mut();
    for (key, value) in [
        (header::CONTENT_TYPE, mime),
        (header::CACHE_CONTROL, "no-store"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ] {
        h.insert(key, value.parse().unwrap());
    }
    response
}
async fn handle(
    State(viewer): State<Arc<Viewer>>,
    Path((capability, path)): Path<(String, String)>,
    request: Request,
) -> Response {
    let mut expected = Hmac::<Sha256>::new_from_slice(viewer.capability.as_bytes()).unwrap();
    expected.update(viewer.capability.as_bytes());
    let tag = expected.finalize().into_bytes();
    let mut received = Hmac::<Sha256>::new_from_slice(viewer.capability.as_bytes()).unwrap();
    received.update(capability.as_bytes());
    if capability.len() != 48
        || !capability.bytes().all(|b| b.is_ascii_hexdigit())
        || received.verify_slice(&tag).is_err()
    {
        return response(StatusCode::NOT_FOUND, "text/plain", "Not found");
    }
    let (mut parts, body) = request.into_parts();
    let origin = format!("http://127.0.0.1:{}", viewer.port);
    let same_origin = parts.headers.get_all(header::ORIGIN).iter().count() == 1
        && parts
            .headers
            .get(header::ORIGIN)
            .is_some_and(|v| v.as_bytes() == origin.as_bytes());
    if path == "websock" {
        if parts.method != Method::GET || !same_origin {
            return response(StatusCode::FORBIDDEN, "text/plain", "Forbidden");
        }
        return match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
            Ok(ws) => ws
                .max_message_size(4 * 1024 * 1024)
                .max_frame_size(4 * 1024 * 1024)
                .on_upgrade(move |socket| rfb(socket, viewer))
                .into_response(),
            Err(_) => response(StatusCode::BAD_REQUEST, "text/plain", "Upgrade required"),
        };
    }
    if path == "display-proof" {
        if parts.method != Method::POST
            || !same_origin
            || !matches!(
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    axum::body::to_bytes(body, 2048)
                )
                .await,
                Ok(Ok(_))
            )
        {
            return response(StatusCode::FORBIDDEN, "text/plain", "Forbidden");
        }
        // UI aggregate reports are optional. Do not persist keystrokes, pixels,
        // URLs or unvalidated browser-provided state in the Provider journal.
        return response(StatusCode::NO_CONTENT, "text/plain", "");
    }
    if !matches!(parts.method, Method::GET | Method::HEAD) {
        return response(StatusCode::METHOD_NOT_ALLOWED, "text/plain", "");
    }
    let (mime, bytes) = match path.as_str() {
        "mobile_controls.js" => (
            "text/javascript;charset=utf-8",
            include_bytes!("../mobile_controls.js").to_vec(),
        ),
        "mobile_controls.css" => (
            "text/css;charset=utf-8",
            include_bytes!("../mobile_controls.css").to_vec(),
        ),
        _ => {
            let Some(mime) = mime(FsPath::new(&path)) else {
                return response(StatusCode::NOT_FOUND, "text/plain", "");
            };
            let Ok(mut bytes) = viewer.asset(&path) else {
                return response(StatusCode::NOT_FOUND, "text/plain", "");
            };
            if path == "vnc.html" {
                let Ok(html) = String::from_utf8(bytes) else {
                    return response(StatusCode::NOT_FOUND, "text/plain", "");
                };
                bytes = html
                    .replace(
                        "</body>",
                        &format!("{}</body>", include_str!("../mobile_toolbar.html")),
                    )
                    .into_bytes();
            }
            (mime, bytes)
        }
    };
    response(StatusCode::OK, mime, bytes)
}
async fn rfb(socket: WebSocket, viewer: Arc<Viewer>) {
    if !std::fs::symlink_metadata(&viewer.socket)
        .is_ok_and(|m| m.file_type().is_socket() && (m.dev(), m.ino()) == viewer.socket_identity)
    {
        return;
    }
    let Ok(rfb) = tokio::net::UnixStream::connect(&viewer.socket).await else {
        return;
    };
    let (mut sink, mut messages) = socket.split();
    let (mut read, mut write) = rfb.into_split();
    tokio::select! {
        _=viewer.shutdown.cancelled()=>(),
        _=async {
            let mut buffer=[0;65536];
            while let Ok(size)=read.read(&mut buffer).await {
                if size==0 || sink.send(Message::Binary(buffer[..size].to_vec().into())).await.is_err(){break;}
            }
        }=>(),
        _=async {
            while let Some(Ok(message))=messages.next().await {
                let bytes=match message {Message::Binary(v)=>v,Message::Text(v)=>v.as_bytes().to_vec().into(),Message::Close(_)=>break,_=>continue};
                if write.write_all(&bytes).await.is_err(){break;}
            }
        }=>(),
    }
}
