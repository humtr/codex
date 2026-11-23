//! Optional local capability-scoped noVNC WebSocket-to-Unix-RFB proxy.

use crate::config::{constant_time_eq, random_hex};
use crate::error::{ProviderError, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        FromRequest, Path as AxumPath, Request, State,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub type ViewerSlot = Arc<RwLock<Option<Arc<Viewer>>>>;

pub struct Viewer {
    pub capability: String,
    socket: PathBuf,
    assets: PathBuf,
    port: u16,
    running: Arc<AtomicBool>,
}

impl Viewer {
    pub fn new(
        socket: PathBuf,
        assets: PathBuf,
        port: u16,
        running: Arc<AtomicBool>,
    ) -> Result<Arc<Self>> {
        if !socket.is_absolute() || !assets.is_absolute() || port == 0 {
            return Err(ProviderError::bad_request(
                "viewer_binding_invalid",
                "viewer socket and assets must be absolute paths",
            ));
        }
        let am = fs::symlink_metadata(&assets).map_err(|_| {
            ProviderError::unavailable("viewer_assets_unavailable", "assets dir unavailable")
        })?;
        if !am.is_dir() || am.uid() != unsafe { libc::geteuid() } {
            return Err(ProviderError::unavailable(
                "viewer_assets_unsafe",
                "viewer assets directory must be owned by current user",
            ));
        }
        let sm = fs::symlink_metadata(&socket).map_err(|_| {
            ProviderError::unavailable("viewer_socket_unavailable", "RFB socket unavailable")
        })?;
        if !sm.file_type().is_socket() || sm.uid() != unsafe { libc::geteuid() } {
            return Err(ProviderError::unavailable(
                "viewer_socket_unsafe",
                "viewer RFB socket must be a private user-owned Unix socket",
            ));
        }
        Ok(Arc::new(Self {
            capability: random_hex(16)?,
            socket,
            assets,
            port,
            running,
        }))
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    pub fn url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/viewer/{}/vnc.html?autoconnect=1&resize=scale&view_clip=0&path=websock",
            self.port, self.capability
        )
    }

    fn read_asset(&self, relative: &str) -> Result<(Vec<u8>, &'static str)> {
        if relative.is_empty()
            || relative.len() > 512
            || relative.contains(['\\', '\0', '%'])
            || Path::new(relative)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(ProviderError::bad_request(
                "viewer_asset_invalid",
                "invalid viewer asset path",
            ));
        }
        let mime = match Path::new(relative).extension().and_then(|e| e.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("json") => "application/json; charset=utf-8",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("ico") => "image/x-icon",
            Some("woff2") => "font/woff2",
            Some("woff") => "font/woff",
            Some("ttf") => "font/ttf",
            Some("mp3") => "audio/mpeg",
            Some("oga") => "audio/ogg",
            _ => {
                return Err(ProviderError::bad_request(
                    "viewer_asset_invalid",
                    "unsupported viewer asset extension",
                ))
            }
        };
        let mut cursor = self.assets.clone();
        for comp in Path::new(relative).components() {
            let Component::Normal(seg) = comp else {
                return Err(ProviderError::bad_request(
                    "viewer_asset_invalid",
                    "invalid viewer asset path",
                ));
            };
            cursor.push(seg);
            let meta = fs::symlink_metadata(&cursor).map_err(|_| {
                ProviderError::unavailable("viewer_asset_unavailable", "asset missing")
            })?;
            if meta.file_type().is_symlink() || meta.uid() != unsafe { libc::geteuid() } {
                return Err(ProviderError::bad_request(
                    "viewer_asset_invalid",
                    "symlinks and foreign-owned paths are rejected in viewer assets",
                ));
            }
        }
        let f = File::options()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&cursor)
            .map_err(|_| ProviderError::unavailable("viewer_asset_unavailable", "asset missing"))?;
        const LIMIT: u64 = 10 * 1024 * 1024;
        let meta = f
            .metadata()
            .map_err(|_| ProviderError::unavailable("viewer_asset_unavailable", "stat failed"))?;
        if !meta.is_file() || meta.len() > LIMIT {
            return Err(ProviderError::bad_request(
                "viewer_asset_invalid",
                "invalid asset file",
            ));
        }
        let mut out = Vec::new();
        f.take(LIMIT + 1)
            .read_to_end(&mut out)
            .map_err(|_| ProviderError::unavailable("viewer_asset_unavailable", "read failed"))?;
        Ok((out, mime))
    }
}

pub fn viewer_router(slot: ViewerSlot) -> Router {
    Router::new()
        .route("/viewer/{capability}/{*path}", get(handle_viewer))
        .with_state(slot)
}

async fn handle_viewer(
    State(slot): State<ViewerSlot>,
    AxumPath((cap, path)): AxumPath<(String, String)>,
    request: Request,
) -> Response {
    let viewer = match slot.read().ok().and_then(|g| g.clone()) {
        Some(v) if v.running() => v,
        _ => return (StatusCode::NOT_FOUND, "viewer not active").into_response(),
    };
    if !constant_time_eq(cap.as_bytes(), viewer.capability.as_bytes()) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if path == "websock" {
        if let Ok(upgrade) = WebSocketUpgrade::from_request(request, &()).await {
            let sock = viewer.socket.clone();
            return upgrade
                .protocols(["binary"])
                .on_upgrade(move |socket| proxy_rfb(socket, sock));
        }
        return (StatusCode::BAD_REQUEST, "websocket upgrade required").into_response();
    }
    match viewer.read_asset(&path) {
        Ok((bytes, mime)) => ([(header::CONTENT_TYPE, mime)], bytes).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn proxy_rfb(ws: WebSocket, sock_path: PathBuf) {
    let Ok(stream) = UnixStream::connect(&sock_path).await else {
        return;
    };
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (mut rd, mut wr) = stream.into_split();

    let to_rfb = async {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Binary(b) => {
                    if wr.write_all(&b).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };
    let from_rfb = async {
        let mut buf = vec![0u8; 32 * 1024];
        loop {
            match rd.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if ws_tx
                        .send(Message::Binary(buf[..n].to_vec().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    };
    tokio::select! {
        _ = to_rfb => {}
        _ = from_rfb => {}
    }
}
