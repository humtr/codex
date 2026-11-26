//! `chatgpt-provider` — ChatGPT Desktop Provider.
//!
//! Single CDP pipe owner + clean-room `page_driver.js` + `/v1/responses` (for
//! `codex -p chatgpt`) + authenticated local `/control/*` (for `@TermuxChatGPT`).
//! Zero DDPE, zero MCP server, zero `shared_server`, zero Wrapper coupling.

mod cdp;
mod config;
mod control;
mod driver;
mod error;
mod host;
mod presentation;
mod responses;
mod server;
mod supervisor;
mod viewer;

use crate::config::{load_or_create_control_token, user_home, ProviderConfig};
use crate::driver::{CdpDriver, DriverBackend};
use crate::error::Result;
#[cfg(test)]
use crate::server::build_router;
use crate::server::build_router_with_threads;
use crate::viewer::ViewerSlot;
use serde_json::json;
use std::env;
use std::future::IntoFuture;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::signal::unix::{signal, SignalKind};

fn print_help() {
    println!(
        "chatgpt-provider {} — ChatGPT Desktop Provider\n\n\
         Usage:\n\
         \x20 chatgpt-provider serve [--config <path>] [--bind 127.0.0.1:<port>] [--state <dir>] [--rootfs <dir>] [--eager-host]\n\
         \x20 chatgpt-provider status [--config <path>]\n\
         \x20 chatgpt-provider print-codex-profile [--config <path>] [--state <dir>] [--bind 127.0.0.1:4319] [--model auto] [--standalone]\n\
         \x20 chatgpt-provider --version | --help",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let result = if args.first().is_some_and(|arg| arg == "host-supervisor") {
        supervisor::run(&args[1..])
    } else {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                crate::error::ProviderError::internal("runtime_start_failed", error.to_string())
            })
            .and_then(|runtime| runtime.block_on(run()))
    };
    if let Err(err) = result {
        eprintln!("chatgpt-provider: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            print_help();
            Ok(())
        }
        Some("-V" | "--version") => {
            println!("chatgpt-provider {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("print-codex-profile") => {
            let home = user_home()?;
            let mut cfg_path: Option<PathBuf> = None;
            let mut state_override: Option<PathBuf> = None;
            let mut bind = "127.0.0.1:4319".to_owned();
            let mut model = "auto".to_owned();
            let mut standalone = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" if i + 1 < args.len() => {
                        cfg_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--state" if i + 1 < args.len() => {
                        state_override = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--bind" if i + 1 < args.len() => {
                        bind = args[i + 1].clone();
                        i += 2;
                    }
                    "--model" if i + 1 < args.len() => {
                        model = args[i + 1].clone();
                        i += 2;
                    }
                    "--standalone" => {
                        standalone = true;
                        i += 1;
                    }
                    other => {
                        eprintln!("chatgpt-provider: unknown flag '{other}'");
                        std::process::exit(2);
                    }
                }
            }
            let state_dir = match state_override {
                Some(dir) => dir,
                None => {
                    ProviderConfig::load_or_default(cfg_path.as_deref(), &home)?.state_dir(&home)
                }
            };
            let catalog_path = state_dir.join("model-catalog.json");
            print!(
                "{}",
                format_codex_profile(&bind, &model, &catalog_path, standalone)
            );
            Ok(())
        }
        Some("status") => {
            let home = user_home()?;
            let cfg_path = parse_opt_path(&args[1..], "--config");
            let cfg = ProviderConfig::load_or_default(cfg_path.as_deref(), &home)?;
            let rootfs = cfg.resolved_rootfs(&home);
            let profile = cfg.profile_dir(&home);
            let state_dir = cfg.state_dir(&home);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "bind": cfg.server.bind,
                    "rootfs": rootfs.display().to_string(),
                    "rootfs_ready": rootfs.join("usr/bin/chatgpt").is_file() && rootfs.join("usr/lib/chatgpt/resources/app.asar").is_file(),
                    "profile_dir": profile.display().to_string(),
                    "profile_exists": profile.is_dir(),
                    "state_dir": state_dir.display().to_string(),
                    "control_token_exists": state_dir.join("control.token").is_file(),
                    "default_model": cfg.model.default,
                }))
                .unwrap()
            );
            Ok(())
        }
        Some("serve") => {
            let home = user_home()?;
            let mut cfg_path: Option<PathBuf> = None;
            let mut bind_override: Option<String> = None;
            let mut state_override: Option<PathBuf> = None;
            let mut rootfs_override: Option<PathBuf> = None;
            let mut eager_override = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" if i + 1 < args.len() => {
                        cfg_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--bind" if i + 1 < args.len() => {
                        bind_override = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--state" if i + 1 < args.len() => {
                        state_override = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--rootfs" if i + 1 < args.len() => {
                        rootfs_override = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--eager-host" => {
                        eager_override = true;
                        i += 1;
                    }
                    other => {
                        eprintln!("chatgpt-provider: unknown serve argument '{other}'");
                        std::process::exit(2);
                    }
                }
            }
            let mut cfg = ProviderConfig::load_or_default(cfg_path.as_deref(), &home)?;
            if let Some(b) = bind_override {
                cfg.server.bind = b;
            }
            if let Some(s) = state_override {
                cfg.server.state_dir = Some(s);
            }
            if let Some(r) = rootfs_override {
                cfg.host.rootfs = Some(r);
            }
            if eager_override {
                cfg.host.eager_start = true;
            }
            cfg.validate()?;

            let state_dir = cfg.state_dir(&home);
            let catalog_path = state_dir.join("model-catalog.json");
            let parallel_tool_calls = true;
            if !catalog_path.exists() {
                let fallback = crate::responses::build_codex_model_catalog(
                    &crate::responses::fallback_catalog_models(&cfg.model),
                    &cfg.model,
                    parallel_tool_calls,
                );
                if let Err(err) =
                    crate::responses::sync_model_catalog_file(&catalog_path, &fallback)
                {
                    eprintln!(
                        "chatgpt-provider: warning: failed to seed {}: {err}",
                        catalog_path.display()
                    );
                }
            } else if let Ok(mut catalog) = std::fs::read(&catalog_path).and_then(|bytes| {
                serde_json::from_slice::<serde_json::Value>(&bytes).map_err(std::io::Error::other)
            }) {
                crate::responses::set_catalog_parallel_tool_calls(
                    &mut catalog,
                    parallel_tool_calls,
                );
                if let Err(err) = crate::responses::sync_model_catalog_file(&catalog_path, &catalog)
                {
                    eprintln!(
                        "chatgpt-provider: warning: failed to update {}: {err}",
                        catalog_path.display()
                    );
                }
            }
            let (token_path, token) = load_or_create_control_token(&state_dir)?;
            let response_state = cfg.response_state_dir(&home);
            config::ensure_private_dir(&response_state)?;
            let threads = crate::responses::ResponseThreadMap::open(&response_state)?;
            let listener = TcpListener::bind(&cfg.server.bind).await.map_err(|e| {
                crate::error::ProviderError::unavailable(
                    "bind_failed",
                    format!("failed to bind {}: {e}", cfg.server.bind),
                )
            })?;
            let local_addr = listener.local_addr().map_err(|e| {
                crate::error::ProviderError::internal("local_addr_failed", e.to_string())
            })?;
            let viewer_slot: ViewerSlot = Arc::new(RwLock::new(None));
            let driver = CdpDriver::new(
                cfg.clone(),
                home,
                local_addr.port(),
                Arc::clone(&viewer_slot),
            );

            let register = |kind| {
                signal(kind).map_err(|error| {
                    crate::error::ProviderError::internal(
                        "shutdown_signal_failed",
                        error.to_string(),
                    )
                })
            };
            let (mut terminate, mut interrupt) = (
                register(SignalKind::terminate())?,
                register(SignalKind::interrupt())?,
            );
            let stopping = async {
                tokio::select! {
                    _ = terminate.recv() => {}
                    _ = interrupt.recv() => {}
                }
            };
            tokio::pin!(stopping);

            if cfg.host.eager_start {
                tokio::select! {
                    biased;
                    _ = &mut stopping => return driver.shutdown().await,
                    result = driver.restart_host() => { result?; }
                }
                if let Ok(st) = driver.status().await {
                    crate::responses::sync_catalog_from_status(
                        Some(&catalog_path),
                        &st,
                        &cfg.model,
                        None,
                        parallel_tool_calls,
                    );
                }
            }

            let driver_dyn: Arc<dyn DriverBackend> = driver.clone();
            let app = build_router_with_threads(
                driver_dyn,
                cfg.model,
                cfg.history,
                token,
                viewer_slot,
                threads,
                Some(catalog_path),
            );
            eprintln!(
                "chatgpt-provider {} listening on http://{} (control token: {})",
                env!("CARGO_PKG_VERSION"),
                local_addr,
                token_path.display()
            );
            let (close, closed) = tokio::sync::oneshot::channel();
            let server = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = closed.await;
                })
                .into_future();
            tokio::pin!(server);
            let serve_res = tokio::select! {
                result = &mut server => {
                    driver.shutdown().await?;
                    result
                }
                _ = &mut stopping => {
                    driver.begin_shutdown();
                    let _ = close.send(());
                    let (http, host) = tokio::join!(
                        tokio::time::timeout(Duration::from_secs(2), &mut server),
                        driver.shutdown(),
                    );
                    host?;
                    http.unwrap_or(Ok(()))
                }
            };
            serve_res.map_err(|e| {
                crate::error::ProviderError::internal("server_error", e.to_string())
            })?;
            Ok(())
        }
        Some(other) => {
            eprintln!("chatgpt-provider: unknown subcommand '{other}' (try --help)");
            std::process::exit(2);
        }
    }
}

fn parse_opt_path(args: &[String], flag: &str) -> Option<PathBuf> {
    let mut i = 0;
    while i + 1 < args.len() {
        if args[i] == flag {
            return Some(PathBuf::from(&args[i + 1]));
        }
        i += 1;
    }
    None
}

fn format_codex_profile(bind: &str, model: &str, catalog_path: &Path, standalone: bool) -> String {
    let cat = serde_json::to_string(&catalog_path.display().to_string())
        .unwrap_or_else(|_| format!("\"{}\"", catalog_path.display()));
    if standalone {
        format!(
            "# Standalone Codex provider config (`~/.codex/chatgpt.config.toml`) for `!cdx provider chatgpt`\n\
             model = \"{model}\"\n\
             model_provider = \"chatgpt_desktop\"\n\
             model_catalog_json = {cat}\n\
             model_reasoning_summary = \"auto\"\n\n\
             [model_providers.chatgpt_desktop]\n\
             name = \"ChatGPT Desktop\"\n\
             base_url = \"http://{bind}/v1\"\n\
             wire_api = \"responses\"\n\
             requires_openai_auth = false\n\
             supports_websockets = false\n\
             request_max_retries = 0\n\
             stream_max_retries = 0\n\n\
             [model_providers.chatgpt_desktop.capabilities]\n\
             remote_compaction = \"unsupported\"\n"
        )
    } else {
        format!(
            "# Opt-in Codex CLI profile (`codex -p chatgpt` in `~/.codex/config.toml`) — zero Wrapper coupling\n\
             # (Or pass `--standalone` to generate `~/.codex/chatgpt.config.toml`)\n\
             [profiles.chatgpt]\n\
             model = \"{model}\"\n\
             model_provider = \"chatgpt_desktop\"\n\
             model_catalog_json = {cat}\n\
             model_reasoning_summary = \"auto\"\n\n\
             [model_providers.chatgpt_desktop]\n\
             name = \"ChatGPT Desktop\"\n\
             base_url = \"http://{bind}/v1\"\n\
             wire_api = \"responses\"\n\
             requires_openai_auth = false\n\
             supports_websockets = false\n\
             request_max_retries = 0\n\
             stream_max_retries = 0\n\n\
             [model_providers.chatgpt_desktop.capabilities]\n\
             remote_compaction = \"unsupported\"\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::BoxFuture;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use std::sync::Mutex;
    use std::time::Duration;
    use tower_like::oneshot;

    struct MockDriver {
        last_send: Mutex<Option<Value>>,
        reply: Value,
    }

    impl MockDriver {
        fn new(reply: Value) -> Arc<Self> {
            Arc::new(Self {
                last_send: Mutex::new(None),
                reply,
            })
        }
    }

    impl DriverBackend for MockDriver {
        fn host_running(&self) -> BoxFuture<'_, bool> {
            Box::pin(async { true })
        }
        fn status(&self) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async {
                Ok(json!({"state":"ok","auth_state":"authenticated","host_running":true}))
            })
        }
        fn list_chats(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async {
                Ok(json!({"state":"ok","chats":[{"id":"c1","title":"Demo"}],"next_cursor":null}))
            })
        }
        fn read_chat(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move { Ok(json!({"state":"ok","chat_id":p["chat_id"],"messages":[]})) })
        }
        fn rename_chat(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(
                async move { Ok(json!({"state":"ok","chat_id":p["chat_id"],"title":p["title"]})) },
            )
        }
        fn list_projects(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async { Ok(json!({"state":"ok","projects":[{"id":"p1","name":"Work"}]})) })
        }
        fn read_project(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(
                async move { Ok(json!({"state":"ok","project_id":p["project_id"],"chats":[]})) },
            )
        }
        fn create_project(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move { Ok(json!({"state":"ok","project_id":"p2","name":p["name"]})) })
        }
        fn send_message(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            *self.last_send.lock().unwrap() = Some(p);
            let r = self.reply.clone();
            Box::pin(async move { Ok(r) })
        }
        fn stream_message(
            &self,
            p: Value,
            tx: crate::cdp::LaneSender,
        ) -> BoxFuture<'_, Result<Value>> {
            use crate::cdp::{StreamEvent, StreamItemKind};
            let result = self.send_message(p);
            Box::pin(async move {
                let reply = result.await?;
                if let Some(snapshots) = reply["test_snapshots"].as_array() {
                    for (idx, snapshot) in snapshots.iter().enumerate() {
                        let event = if let Some(raw_ev) = snapshot.get("event") {
                            serde_json::from_value::<StreamEvent>(raw_ev.clone()).unwrap()
                        } else {
                            let kind_str = snapshot["kind"].as_str().unwrap_or("answer");
                            let text = snapshot["text"].as_str().unwrap_or("").to_owned();
                            let committed = snapshot
                                .get("committed")
                                .and_then(Value::as_bool)
                                .unwrap_or(true);
                            let item_id = snapshot
                                .get("item_id")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                                .unwrap_or_else(|| format!("{kind_str}_{idx}"));
                            if kind_str == "tool_call" {
                                StreamEvent::ToolCall {
                                    id: snapshot["call_id"]
                                        .as_str()
                                        .unwrap_or("call_mock")
                                        .to_owned(),
                                    name: snapshot["name"].as_str().unwrap_or("tool").to_owned(),
                                    namespace: snapshot
                                        .get("namespace")
                                        .and_then(Value::as_str)
                                        .map(ToOwned::to_owned),
                                    arguments: snapshot["arguments"]
                                        .as_str()
                                        .unwrap_or("{}")
                                        .to_owned(),
                                    native_message_id: snapshot["native_message_id"]
                                        .as_str()
                                        .map(str::to_owned),
                                    native_recipient: snapshot["native_recipient"]
                                        .as_str()
                                        .map(str::to_owned),
                                }
                            } else {
                                let item_kind = match kind_str {
                                    "reasoning" | "reasoning_summary" => {
                                        StreamItemKind::ReasoningSummary
                                    }
                                    "commentary" => StreamItemKind::Commentary,
                                    _ => StreamItemKind::Answer,
                                };
                                StreamEvent::ItemUpsert {
                                    item_id,
                                    item_kind,
                                    text,
                                    committed,
                                    streamable: snapshot["streamable"].as_bool().unwrap_or(false),
                                }
                            }
                        };
                        tx.send_nonblocking(event);
                        let delay_ms = snapshot
                            .get("delay_ms")
                            .and_then(Value::as_u64)
                            .unwrap_or_else(|| reply["test_delay_ms"].as_u64().unwrap_or(125_000));
                        if delay_ms > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                }
                Ok(reply)
            })
        }

        fn observe_chat(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move {
                Ok(json!({"state":"settled","chat_id":p["chat_id"],"generating":false}))
            })
        }
        fn stop_generation(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move {
                Ok(json!({
                    "state":"stopped",
                    "stopped":true,
                    "chat_id":p["chat_id"],
                    "lane_id":p["lane_id"]
                }))
            })
        }
        fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async { Ok(json!({"state":"ok","available":false,"url":null})) })
        }
        fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async { Ok(json!({"state":"ok","restarted":true})) })
        }
        fn list_connectors(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move {
                Ok(json!({"state":"ok","account_id":p["account_id"],"tunnels":[],"connectors":[]}))
            })
        }
        fn register_connector(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move {
                Ok(json!({
                    "state":"ok",
                    "connector_id":"connector_mock1",
                    "name":p["name"],
                    "tunnel_id":p["tunnel_id"],
                    "created":true,
                    "linked":true,
                    "schema_agreement":true
                }))
            })
        }
        fn refresh_connector(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move {
                Ok(json!({
                    "state":"ok",
                    "connector_id":p["connector_id"],
                    "schema_agreement":true
                }))
            })
        }
    }

    mod tower_like {
        use axum::{body::Body, http::Request, response::Response, Router};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        /// Drive the real axum Router over a real ephemeral loopback TCP socket (zero extra dev-deps).
        pub async fn oneshot(router: Router, req: Request<Body>) -> Response {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let srv = tokio::spawn(async move {
                let _ = axum::serve(listener, router).await;
            });

            let method = req.method().as_str().to_owned();
            let uri = req.uri().to_string();
            let mut headers = String::new();
            for (k, v) in req.headers() {
                headers.push_str(&format!("{}: {}\r\n", k, v.to_str().unwrap()));
            }
            let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let raw_req = format!(
                "{method} {uri} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
                body_bytes.len()
            );

            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            stream.write_all(raw_req.as_bytes()).await.unwrap();
            stream.write_all(&body_bytes).await.unwrap();
            let mut resp_bytes = Vec::new();
            stream.read_to_end(&mut resp_bytes).await.unwrap();
            srv.abort();

            let split = resp_bytes
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .unwrap();
            let head = String::from_utf8_lossy(&resp_bytes[..split]);
            let status_code: u16 = head
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap();
            let raw_body = resp_bytes[split + 4..].to_vec();
            let mut is_chunked = false;
            let mut builder = Response::builder().status(status_code);
            for line in head.lines().skip(1) {
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim().eq_ignore_ascii_case("transfer-encoding")
                        && v.to_ascii_lowercase().contains("chunked")
                    {
                        is_chunked = true;
                    }
                    builder = builder.header(k.trim(), v.trim());
                }
            }
            let body = if is_chunked {
                let mut out = Vec::new();
                let mut pos = 0;
                while pos < raw_body.len() {
                    let Some(rel) = raw_body[pos..].windows(2).position(|w| w == b"\r\n") else {
                        break;
                    };
                    let sz_line = std::str::from_utf8(&raw_body[pos..pos + rel]).unwrap();
                    let sz = usize::from_str_radix(sz_line.split(';').next().unwrap().trim(), 16)
                        .unwrap();
                    pos += rel + 2;
                    if sz == 0 {
                        break;
                    }
                    out.extend_from_slice(&raw_body[pos..pos + sz]);
                    pos += sz + 2;
                }
                out
            } else {
                raw_body
            };
            builder.body(Body::from(body)).unwrap()
        }
    }

    #[tokio::test]
    async fn control_plane_requires_bearer_token_and_routes_cleanly() {
        let driver =
            MockDriver::new(json!({"state":"settled","assistant_text":"hi","tool_calls":[]}));
        let router = build_router(
            driver,
            config::ModelConfig::default(),
            config::HistoryConfig::default(),
            "secret-token-0123456789abcdef0123456789".to_owned(),
            Arc::new(RwLock::new(None)),
        );

        // Missing token -> 401
        let r_unauth = oneshot(
            router.clone(),
            Request::builder()
                .uri("/control/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(r_unauth.status(), StatusCode::UNAUTHORIZED);

        // Wrong token -> 401
        let r_wrong = oneshot(
            router.clone(),
            Request::builder()
                .uri("/control/status")
                .header("authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(r_wrong.status(), StatusCode::UNAUTHORIZED);

        // Valid token -> 200
        let r_ok = oneshot(
            router.clone(),
            Request::builder()
                .uri("/control/status")
                .header(
                    "authorization",
                    "Bearer secret-token-0123456789abcdef0123456789",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(r_ok.status(), StatusCode::OK);
        let body = to_bytes(r_ok.into_body(), 65536).await.unwrap();
        let val: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["auth_state"], "authenticated");
    }

    #[tokio::test]
    async fn responses_sse_emits_monotonic_events_and_in_app_tool_callbacks() {
        let driver = MockDriver::new(json!({
            "state": "settled",
            "chat_id": "conv-sse-tool",
            "remote_head": "head-sse-tool",
            "assistant_text": "Checking files",
            "tool_calls": [{
                "id": "call_shell_1",
                "name": "exec_command",
                "arguments": "{\"cmd\":\"pwd\"}"
            }]
        }));
        let router = build_router(
            driver.clone(),
            config::ModelConfig::default(),
            config::HistoryConfig::default(),
            "tok-0123456789abcdef0123456789abcdef".to_owned(),
            Arc::new(RwLock::new(None)),
        );

        let req_body = json!({
            "model": "gpt-5.4",
            "stream": true,
            "instructions": "Be concise",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "Where am I?"}]}],
            "tools": [{"type": "function", "name": "exec_command", "parameters": {"type": "object"}}]
        });
        let resp = oneshot(
            router,
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(req_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let sse = String::from_utf8(
            to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(sse.contains("event: response.created\n"));
        assert!(sse.contains("event: response.output_text.delta\n"));
        assert!(sse.contains("event: response.function_call_arguments.done\n"));
        assert!(sse.contains("event: response.completed\n"));

        // Verify local_function_signatures passed to page driver via structured native_candidate without prompt injection!
        let sent = driver.last_send.lock().unwrap().clone().unwrap();
        assert_eq!(sent["local_function_signatures"][0]["name"], "exec_command");
        let msgs = sent["native_candidate"]["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "user");
        assert!(!msgs[0]["parts"][0]
            .as_str()
            .unwrap()
            .contains("[Codex Local Tool Boundary]"));
    }

    #[test]
    fn config_rejects_non_loopback_bind_address() {
        let mut cfg = ProviderConfig::default();
        cfg.server.bind = "0.0.0.0:4319".to_owned();
        assert!(cfg.validate().is_err());
    }

    #[tokio::test]
    async fn responses_extracts_codex_additional_tools_emits_custom_and_namespaced_calls_and_tracks_delta_sessions(
    ) {
        let driver = MockDriver::new(json!({
            "state": "settled",
            "chat_id": "conv-orch-1",
            "account_id": "acct-orch-1",
            "remote_head": "head-orch-1",
            "assistant_text": "",
            "tool_calls": [
                {
                    "id": "call_exec_1",
                    "native_message_id": "exec_1",
                    "native_recipient": "local.exec",
                    "name": "exec",
                    "arguments": "{\"input\":\"echo hi\"}"
                },
                {
                    "id": "call_spawn_1",
                    "native_message_id": "spawn_1",
                    "native_recipient": "local.spawn_agent",
                    "namespace": "collaboration",
                    "name": "spawn_agent",
                    "arguments": "{\"task_name\":\"worker\",\"message\":\"compute\"}"
                }
            ]
        }));
        let history = config::HistoryConfig {
            project_id: Some("g-p-test-proj".to_owned()),
            ..config::HistoryConfig::default()
        };
        let router = build_router(
            driver.clone(),
            config::ModelConfig::default(),
            history,
            "tok-0123456789abcdef0123456789abcdef".to_owned(),
            Arc::new(RwLock::new(None)),
        );

        let r1_body = json!({
            "model": "gpt-6-sol-wm",
            "stream": true,
            "prompt_cache_key": "sess-orch-1",
            "reasoning": {"effort": "xhigh"},
            "input": [
                {
                    "type": "additional_tools",
                    "role": "developer",
                    "tools": [
                        {
                            "type": "namespace",
                            "name": "functions",
                            "tools": [
                                {"type": "custom", "name": "exec", "description": "Run shell command"}
                            ]
                        },
                        {
                            "type": "namespace",
                            "name": "collaboration",
                            "tools": [
                                {"type": "function", "name": "spawn_agent", "description": "Spawn worker", "parameters": {"type": "object"}}
                            ]
                        }
                    ]
                },
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Delegate work"}]}
            ]
        });
        let resp1 = oneshot(
            router.clone(),
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .header("thread-id", "thread-orch-1")
                .body(Body::from(r1_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp1.status(), StatusCode::OK);
        let sse1 = String::from_utf8(
            to_bytes(resp1.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(sse1.contains("event: response.custom_tool_call_input.done\n"));
        assert!(sse1.contains("\"type\":\"custom_tool_call\""));
        assert!(sse1.contains("\"input\":\"echo hi\""));
        assert!(sse1.contains("\"name\":\"spawn_agent\""));
        assert!(sse1.contains("\"namespace\":\"collaboration\""));

        let sent1 = driver.last_send.lock().unwrap().clone().unwrap();
        assert_eq!(sent1["project_id"], "g-p-test-proj");
        assert_eq!(sent1["model"], "gpt-6-sol-wm");
        assert_eq!(sent1["reasoning_effort"], "xhigh");
        assert_eq!(sent1["native_candidate"]["messages"][0]["role"], "user");
        assert_eq!(
            sent1["local_function_signatures"].as_array().unwrap().len(),
            2
        );

        // Round 2 with same thread-id must reuse conv-orch-1 and send structured tool outputs + agent_message delta!
        let mut r2_body = r1_body.clone();
        r2_body["input"].as_array_mut().unwrap().push(json!({
            "type": "custom_tool_call",
            "call_id": "call_exec_1",
            "name": "exec",
            "input": "echo hi"
        }));
        r2_body["input"].as_array_mut().unwrap().push(json!({
            "type": "function_call",
            "call_id": "call_spawn_1",
            "name": "spawn_agent",
            "namespace": "collaboration",
            "arguments": "{\"task_name\":\"worker\",\"message\":\"compute\"}"
        }));
        r2_body["input"].as_array_mut().unwrap().push(json!({
            "type": "custom_tool_call_output",
            "call_id": "call_exec_1",
            "output": "hi"
        }));
        r2_body["input"].as_array_mut().unwrap().push(json!({
            "type": "function_call_output",
            "call_id": "call_spawn_1",
            "output": "WORKER_DONE=42"
        }));
        r2_body["input"].as_array_mut().unwrap().push(json!({
            "type": "agent_message",
            "author": "/root",
            "recipient": "/root/worker",
            "content": [
                {"type": "input_text", "text": "Message Type: NEW_TASK\nPayload:\n"},
                {"type": "encrypted_content", "encrypted_content": "READ_SECRET_TASK_99"}
            ]
        }));
        let resp2 = oneshot(
            router.clone(),
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .header("thread-id", "thread-orch-1")
                .body(Body::from(r2_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp2.status(), StatusCode::OK);
        let sent2 = driver.last_send.lock().unwrap().clone().unwrap();
        assert_eq!(sent2["chat_id"], "conv-orch-1");
        let msgs2 = sent2["native_candidate"]["messages"].as_array().unwrap();
        assert_eq!(msgs2.len(), 3);
        assert_eq!(msgs2[0]["role"], "tool");
        assert_eq!(msgs2[1]["role"], "tool");
        assert_eq!(msgs2[1]["parts"][0], "WORKER_DONE=42");
        assert!(msgs2[2]["parts"][0]
            .as_str()
            .unwrap()
            .contains("READ_SECRET_TASK_99"));
        assert!(!msgs2.iter().any(|m| m["parts"][0]
            .as_str()
            .unwrap_or("")
            .contains("Delegate work")));
        assert_eq!(
            sent2["local_function_signatures"].as_array().unwrap().len(),
            2
        );

        // Keep round 2's cached chat and compact input so changed prefix selects a fresh chat.
        let r3_body = json!({
            "model": "gpt-6-sol-wm",
            "stream": true,
            "prompt_cache_key": "sess-orch-1",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Compacted summary of prior work"}]},
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Continue after compaction"}]}
            ]
        });
        let resp3 = oneshot(
            router,
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .header("thread-id", "thread-orch-1")
                .body(Body::from(r3_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp3.status(), StatusCode::OK);
        let sent3 = driver.last_send.lock().unwrap().clone().unwrap();
        assert!(sent3["chat_id"].is_null());

        // Background Transcript Title Generation MUST use temporary chat and omit project_id!
        let title_body = json!({
            "model": "gpt-6-sol-wm",
            "stream": false,
            "instructions": "Transcript Title Generation: generate a concise title for this conversation.",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "Hello"}]}]
        });
        let resp_title = oneshot(
            build_router(
                driver.clone(),
                config::ModelConfig::default(),
                config::HistoryConfig {
                    project_id: Some("g-p-test-proj".to_owned()),
                    ..config::HistoryConfig::default()
                },
                "tok-0123456789abcdef0123456789abcdef".to_owned(),
                Arc::new(RwLock::new(None)),
            ),
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(title_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp_title.status(), StatusCode::OK);
        let sent_title = driver.last_send.lock().unwrap().clone().unwrap();
        assert!(sent_title["project_id"].is_null());
        assert_eq!(sent_title["temporary"], true);
    }

    #[tokio::test(start_paused = true)]
    async fn responses_flush_live_items_before_long_turn_settles() {
        use futures_util::StreamExt;
        use std::time::Duration;
        let driver = MockDriver::new(json!({
            "state": "settled",
            "reasoning_summary": "Visible summary",
            "assistant_text": "Final answer",
            "items": [{"id":"r","kind":"reasoning_summary","text":"Visible summary"}, {"id":"a","kind":"answer","text":"Final answer"}],
            "test_snapshots": [
                {"item_id":"r","kind":"reasoning","text":"Visible","committed":false,"streamable":true,"delay_ms":1000},
                {"item_id":"r","kind":"reasoning","text":"Visible summary","committed":false,"streamable":true,"delay_ms":1000},
                {"item_id":"r","kind":"reasoning","text":"Visible summary","committed":true,"delay_ms":1000},
                {"item_id":"a","kind":"answer","text":"Final","committed":false,"streamable":true,"delay_ms":1000},
                {"item_id":"a","kind":"answer","text":"Final answer","committed":false,"streamable":true,"delay_ms":1000},
                {"item_id":"a","kind":"answer","text":"Final answer","committed":true,"delay_ms":250000}
            ]
        }));
        let start = tokio::time::Instant::now();
        let response = crate::responses::handle_responses(
            driver,
            &Default::default(),
            &Default::default(),
            Arc::new(Default::default()),
            Some("test".to_owned()),
            None,
            json!({"input":"test","stream":true}),
        )
        .await
        .unwrap();
        let mut stream = response.into_body().into_data_stream();
        let mut observed = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            let text = std::str::from_utf8(&chunk).unwrap();
            if let Some(data) = text.lines().find_map(|line| line.strip_prefix("data: ")) {
                let event: Value = serde_json::from_str(data).unwrap();
                observed.push((start.elapsed(), event));
            }
        }
        let time = |kind| {
            observed
                .iter()
                .find(|(_, event)| event["type"] == kind)
                .unwrap()
                .0
        };
        assert!(time("response.reasoning_summary_text.delta") < time("response.output_text.delta"));
        assert!(time("response.output_text.delta") < time("response.completed"));
        assert!(time("response.completed") >= Duration::from_secs(250));
        for kind in [
            "response.reasoning_summary_text.delta",
            "response.output_text.delta",
        ] {
            let deltas: Vec<_> = observed.iter().filter(|(_, e)| e["type"] == kind).collect();
            assert_eq!(deltas.len(), 2);
            assert!(deltas[1].0 < time("response.completed"));
        }
        let response = &observed.last().unwrap().1["response"];
        assert!(response["usage"]["input_tokens"].as_u64().unwrap() > 0);
        assert!(response["usage"]["output_tokens"].as_u64().unwrap() > 0);
        assert_eq!(response["usage"]["estimated"], true);
        for (index, (_, event)) in observed.iter().enumerate() {
            assert_eq!(event["sequence_number"], index);
        }
    }

    #[tokio::test]
    async fn responses_restart_reuses_pinned_chat_and_summary_only_survives() {
        use axum::body::to_bytes;
        let state =
            std::env::temp_dir().join(format!("provider-map-{}", config::random_hex(8).unwrap()));
        std::fs::create_dir(&state).unwrap();
        let driver = MockDriver::new(json!({
            "state":"settled","chat_id":"conv-one","account_id":"acct-one","remote_head":"head-one",
            "reasoning_summary":"Only summary", "assistant_text":""
        }));
        let first = json!({"input":[{"role":"user","content":"First"}],"stream":true});
        let mut second = first.clone();
        second["input"]
            .as_array_mut()
            .unwrap()
            .push(json!({"role":"user","content":"Second"}));
        let mut compacted = second.clone();
        compacted["input"][0]["content"] = json!("Compressed history");
        for (round, body) in [first, second.clone(), compacted].into_iter().enumerate() {
            let map = Arc::new(crate::responses::ResponseThreadMap::open(&state).unwrap());
            let response = crate::responses::handle_responses(
                driver.clone(),
                &Default::default(),
                &Default::default(),
                Arc::clone(&map),
                Some("test-session".to_owned()),
                None,
                body,
            )
            .await
            .unwrap();
            let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            let sse = std::str::from_utf8(&bytes).unwrap();
            assert!(sse.contains("response.completed"));
            assert!(sse.contains("Only summary"));
            assert!(sse.contains("response.reasoning_summary_text.delta"));
            assert!(sse.contains("response.output_text.delta"));
            let sent = driver.last_send.lock().unwrap().clone();
            if round == 0 {
                assert!(sent.as_ref().unwrap()["chat_id"].is_null());
            } else if round == 1 {
                assert_eq!(sent.as_ref().unwrap()["chat_id"], "conv-one");
                assert_eq!(sent.as_ref().unwrap()["expected_account_id"], "acct-one");
                let msgs = sent.as_ref().unwrap()["native_candidate"]["messages"]
                    .as_array()
                    .unwrap();
                assert!(!msgs
                    .iter()
                    .any(|m| m["parts"][0].as_str().unwrap_or("").contains("First")));
            } else {
                assert!(sent.as_ref().unwrap()["chat_id"].is_null());
                assert_eq!(sent.as_ref().unwrap()["expected_account_id"], "acct-one");
            }
            if round == 1 {
                let duplicate = crate::responses::handle_responses(
                    driver.clone(),
                    &Default::default(),
                    &Default::default(),
                    Arc::clone(&map),
                    Some("test-session".to_owned()),
                    None,
                    second.clone(),
                )
                .await
                .unwrap_err();
                assert_eq!(duplicate.code, "response_input_already_sent");
            }
            // The HTTP producer holds the map (and its lease) until EOF.
            drop(map);
        }
        std::fs::remove_dir_all(state).unwrap();
    }

    #[tokio::test]
    #[ignore = "requires an installed official Termux Codex 0.171.0"]
    async fn native_codex_reads_provider_summary_and_answer_items() {
        use std::time::Duration;
        let root = std::env::temp_dir().join(format!(
            "provider-native-{}",
            config::random_hex(8).unwrap()
        ));
        let home = root.join("codex");
        let cwd = root.join("work");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = build_router(
            MockDriver::new(json!({
                "state":"settled", "reasoning_summary":"VISIBLE_FIXTURE_SUMMARY",
                "assistant_text":"VISIBLE_FIXTURE_ANSWER", "test_delay_ms":450,
                "test_snapshots":[{"kind":"reasoning","text":"VISIBLE_FIXTURE_SUMMARY"},
                                  {"kind":"answer","text":"VISIBLE_FIXTURE_ANSWER"}]
            })),
            Default::default(),
            Default::default(),
            "fixture-control-token".to_owned(),
            Arc::new(RwLock::new(None)),
        );
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        std::fs::write(
            home.join("config.toml"),
            format!(
                r#"
model = "fixture-model"
model_provider = "chatgpt_desktop"
approval_policy = "never"
[model_providers.chatgpt_desktop]
name = "Isolated Provider fixture"
base_url = "http://{addr}/v1"
requires_openai_auth = false
wire_api = "responses"
"#
            ),
        )
        .unwrap();
        let output = tokio::time::timeout(
            Duration::from_secs(24),
            tokio::process::Command::new("codex")
                .args([
                    "--no-daemon",
                    "exec",
                    "--json",
                    "--skip-git-repo-check",
                    "Run the streaming fixture",
                ])
                .current_dir(&cwd)
                .env("CODEX_HOME", &home)
                .env_remove("CODEX_SHARED_HOME")
                .env_remove("CODEX_SQLITE_HOME")
                .env_remove("CODEX_THREAD_ID")
                .env_remove("CODEX_SHARED_EXTRA_FILES")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        server.abort();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "{stderr}\n{stdout}");
        let items: Vec<Value> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        assert!(
            items.iter().any(|v| v["type"] == "item.completed"
                && v["item"]["type"] == "reasoning"
                && v["item"]["text"] == "VISIBLE_FIXTURE_SUMMARY"),
            "{stdout}"
        );
        assert!(
            items.iter().any(|v| v["type"] == "item.completed"
                && v["item"]["type"] == "agent_message"
                && v["item"]["text"] == "VISIBLE_FIXTURE_ANSWER"),
            "{stdout}"
        );
        let sessions = std::process::Command::new("rg")
            .args(["-l", "VISIBLE_FIXTURE_SUMMARY"])
            .arg(home.join("sessions"))
            .output()
            .unwrap();
        let logs = String::from_utf8(sessions.stdout).unwrap();
        assert!(logs
            .lines()
            .any(|path| std::fs::read_to_string(path).is_ok_and(|log| {
                log.contains("VISIBLE_FIXTURE_ANSWER") && log.contains("VISIBLE_FIXTURE_SUMMARY")
            })));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn control_plane_rejects_dotdot_and_control_char_ids() {
        let driver =
            MockDriver::new(json!({"state":"settled","assistant_text":"hi","tool_calls":[]}));
        let tok = "secret-token-0123456789abcdef0123456789";
        let router = build_router(
            driver,
            config::ModelConfig::default(),
            config::HistoryConfig::default(),
            tok.to_owned(),
            Arc::new(RwLock::new(None)),
        );
        for uri in [
            "/control/projects/..",
            "/control/chats/..",
            "/control/chats/.",
        ] {
            let r = oneshot(
                router.clone(),
                Request::builder()
                    .uri(uri)
                    .header("authorization", format!("Bearer {tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(r.status(), StatusCode::BAD_REQUEST, "uri={uri}");
        }
    }

    #[tokio::test]
    async fn responses_stream_client_disconnect_triggers_stop_generation() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        struct SlowDriver {
            stopped: Arc<AtomicBool>,
            started: Arc<tokio::sync::Notify>,
        }
        impl DriverBackend for SlowDriver {
            fn host_running(&self) -> BoxFuture<'_, bool> {
                Box::pin(async { true })
            }
            fn status(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn list_chats(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn read_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn rename_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn list_projects(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn read_project(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn create_project(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn send_message(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                let started = Arc::clone(&self.started);
                Box::pin(async move {
                    started.notify_one();
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    Ok(json!({"state":"settled","assistant_text":"late","tool_calls":[]}))
                })
            }
            fn observe_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"in_progress"})) })
            }
            fn stop_generation(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
                let stopped = Arc::clone(&self.stopped);
                Box::pin(async move {
                    if p.get("lane_id")
                        .and_then(Value::as_str)
                        .is_some_and(|s| s.starts_with("resp_"))
                    {
                        stopped.store(true, Ordering::Release);
                    }
                    Ok(json!({"state":"stopped","stopped":true}))
                })
            }
            fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok","available":false})) })
            }
            fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn list_connectors(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok","tunnels":[],"connectors":[]})) })
            }
            fn register_connector(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn refresh_connector(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
        }

        let stopped = Arc::new(AtomicBool::new(false));
        let started = Arc::new(tokio::sync::Notify::new());
        let driver: Arc<dyn DriverBackend> = Arc::new(SlowDriver {
            stopped: Arc::clone(&stopped),
            started: Arc::clone(&started),
        });
        let router = build_router(
            driver,
            config::ModelConfig::default(),
            config::HistoryConfig::default(),
            "tok-0123456789abcdef0123456789abcdef".to_owned(),
            Arc::new(RwLock::new(None)),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let srv = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let body = json!({"model":"auto","stream":true,"input":"long task"}).to_string();
        let req = format!(
            "POST /v1/responses HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        started.notified().await;
        // Drop the client TCP stream mid-turn!
        drop(stream);

        // Wait briefly for Axum stream drop -> stop_generation() cancellation
        for _ in 0..40 {
            if stopped.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        srv.abort();
        assert!(
            stopped.load(Ordering::Acquire),
            "disconnecting mid-stream must invoke stop_generation()"
        );
    }

    #[tokio::test]
    async fn control_connectors_auth_and_validation() {
        let token = "tok-0123456789abcdef0123456789abcdef";
        let driver: Arc<dyn DriverBackend> = Arc::new(MockDriver {
            reply: json!({"state":"settled","assistant_text":"ok","tool_calls":[]}),
            last_send: std::sync::Mutex::new(None),
        });
        let router = build_router(
            driver,
            config::ModelConfig::default(),
            config::HistoryConfig::default(),
            token.to_owned(),
            Arc::new(RwLock::new(None)),
        );

        // Unauthorized GET /control/connectors -> 401
        let unauth = tower_like::oneshot(
            router.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/control/connectors")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(unauth.status(), 401);

        // Authorized GET /control/connectors -> 200
        let listed = tower_like::oneshot(
            router.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/control/connectors")
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(listed.status(), 200);

        // Invalid tunnel_id -> 400
        let bad_tid = tower_like::oneshot(
            router.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/control/connectors")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({"name":"TermuxShell","tunnel_id":"not-a-tunnel"}).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(bad_tid.status(), 400);

        // Valid register -> 200
        let ok_reg = tower_like::oneshot(
            router.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/control/connectors")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "name":"TermuxShell",
                        "tunnel_id":"tunnel_6b03b77cf8bc8191b405430734cc50f7",
                        "description":"Termux shell MCP"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(ok_reg.status(), 200);

        // Valid refresh -> 200
        let ok_ref = tower_like::oneshot(
            router,
            axum::http::Request::builder()
                .method("POST")
                .uri("/control/connectors/connector_mock1/refresh")
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(ok_ref.status(), 200);
    }

    #[tokio::test]
    async fn model_catalog_json_syncs_from_v1_models_and_control_status() {
        use std::os::unix::fs::MetadataExt;
        struct ModelsMockDriver(Value);
        impl DriverBackend for ModelsMockDriver {
            fn host_running(&self) -> BoxFuture<'_, bool> {
                Box::pin(async { true })
            }
            fn status(&self) -> BoxFuture<'_, Result<Value>> {
                let v = self.0.clone();
                Box::pin(async move { Ok(v) })
            }
            fn list_chats(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn read_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn rename_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn list_projects(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn read_project(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn create_project(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn send_message(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn stream_message(
                &self,
                _: Value,
                _: crate::cdp::LaneSender,
            ) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn observe_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn stop_generation(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok","available":false,"url":null})) })
            }
            fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok","restarted":true})) })
            }
            fn list_connectors(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn register_connector(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
            fn refresh_connector(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({})) })
            }
        }

        let dir = env::temp_dir().join(format!(
            "cgp-catalog-{}-{}",
            std::process::id(),
            config::random_hex(4).unwrap()
        ));
        let catalog_path = dir.join("model-catalog.json");
        let driver = Arc::new(ModelsMockDriver(json!({
            "state": "ok",
            "auth_state": "authenticated",
            "models": [
                { "slug": "gpt-5-6", "title": "GPT-5.6 Sol", "description": "Older", "max_tokens": 52815 },
                { "slug": "gpt-5-6-instant", "title": "GPT-5.6 Sol", "max_tokens": 52815 },
                { "slug": "gpt-5-6-thinking", "title": "GPT-5.6 Sol", "max_tokens": 262144 },
                { "slug": "gpt-5-6-pro", "title": "GPT-5.6 Pro", "max_tokens": 410000 },
                { "slug": "gpt-5-6-mini", "title": "GPT-5.6 Luna", "max_tokens": 137000 },
                { "slug": "gpt-5-6-t-mini", "title": "GPT-5.6 Luna", "max_tokens": 262144 },
                { "slug": "gpt-6-sol-wm", "title": "GPT-6 Sol", "max_tokens": 262144 },
                { "slug": "research", "title": "Deep Research", "max_tokens": 34815 },
                { "slug": "gpt-6", "title": "GPT-6", "max_tokens": 52815 },
                { "slug": "gpt-6-instant", "title": "GPT-6", "max_tokens": 52815 },
                { "slug": "gpt-6-thinking", "title": "GPT-6", "max_tokens": 262144 },
                { "slug": "gpt-6-pro", "title": "GPT-6 Pro", "max_tokens": 410000 },
                { "slug": "gpt-6-mini", "title": "GPT-6", "max_tokens": 137000 },
                { "slug": "gpt-6-t-mini", "title": "GPT-6", "max_tokens": 262144 },
                { "slug": "gpt-6-2", "title": "GPT-6.2", "max_tokens": 52815 },
                { "slug": "gpt-6-2-instant", "title": "GPT-6.2", "max_tokens": 52815 },
                { "slug": "gpt-6-2-thinking", "title": "GPT-6.2", "description": "Latest flagship", "max_tokens": 262144 },
                { "slug": "chatgpt_alpha_model_external_access_reserved_gate_13", "title": "Gate", "tags": ["hidden"] }
            ]
        })));
        let token = "tok-cat-1234567890abcdef";
        let model_cfg = config::ModelConfig {
            reasoning_effort: "xhigh".to_owned(),
            ..Default::default()
        };
        let router = build_router_with_threads(
            driver,
            model_cfg,
            config::HistoryConfig::default(),
            token.to_owned(),
            Arc::new(RwLock::new(None)),
            crate::responses::ResponseThreadMap::default(),
            Some(catalog_path.clone()),
        );

        let res = oneshot(
            router.clone(),
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        let v1_body: Value =
            serde_json::from_slice(&to_bytes(res.into_body(), 1024 * 1024).await.unwrap()).unwrap();
        let v1_ids: Vec<&str> = v1_body["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|x| x["id"].as_str())
            .collect();
        assert!(!v1_ids.contains(&"gpt-6-sol-wm"));
        assert!(!v1_ids.contains(&"chatgpt_alpha_model_external_access_reserved_gate_13"));

        let raw = std::fs::read_to_string(&catalog_path).unwrap();
        let cat: Value = serde_json::from_str(&raw).unwrap();
        let models = cat["models"].as_array().unwrap();
        let visible: Vec<&Value> = models
            .iter()
            .filter(|m| m["visibility"] == "list")
            .collect();
        let visible_pairs: Vec<(&str, &str, i64)> = visible
            .iter()
            .map(|m| {
                (
                    m["slug"].as_str().unwrap(),
                    m["display_name"].as_str().unwrap(),
                    m["priority"].as_i64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            visible_pairs,
            vec![
                ("gpt-6-2", "GPT-6.2", 1),
                ("gpt-6", "GPT-6", 2),
                ("gpt-6-pro", "GPT-6 Pro", 3),
                ("gpt-6-mini", "GPT-6 Mini", 4),
                ("gpt-5-6", "GPT-5.6", 5),
                ("gpt-5-6-pro", "GPT-5.6 Pro", 6),
                ("gpt-5-6-mini", "GPT-5.6 Mini", 7),
                ("research", "Deep Research", 8),
            ]
        );
        assert_eq!(visible[0]["default_reasoning_level"], "xhigh");
        assert_eq!(visible[0]["context_window"], 1_000_000);
        assert_eq!(visible[0]["auto_compact_token_limit"], 800_000);
        assert_eq!(visible[0]["effective_context_window_percent"], 95);
        assert_eq!(visible[0]["supports_reasoning_summaries"], json!(true));
        assert_eq!(
            visible[0]["supports_reasoning_summary_parameter"],
            json!(true)
        );
        assert_eq!(
            visible[0]["supported_reasoning_levels"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        // Pro has no "none" option
        assert_eq!(
            visible[2]["supported_reasoning_levels"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        // Research has empty efforts and default none
        assert_eq!(visible[7]["default_reasoning_level"], "none");
        assert_eq!(visible[7]["supported_reasoning_levels"], json!([]));
        // Hidden entries preserve variants and exclude *-wm and reserved gates from picker
        assert!(models
            .iter()
            .filter(|m| m["slug"] == "gpt-6-sol-wm" || m["slug"] == "gpt-6-2-thinking")
            .all(|m| m["visibility"] == "hide"));

        // Second sync via /control/status leaves identical inode untouched (no rewrite).
        let ino1 = std::fs::metadata(&catalog_path).unwrap().ino();
        let st_res = oneshot(
            router,
            Request::builder()
                .method("GET")
                .uri("/control/status")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(st_res.status(), StatusCode::OK);
        assert_eq!(std::fs::metadata(&catalog_path).unwrap().ino(), ino1);

        // host_running: false fallback status must never clobber the persisted live catalog or live_models.
        let before_raw = std::fs::read_to_string(&catalog_path).unwrap();
        let live_set = crate::responses::new_live_model_set(["gpt-6-2-thinking".to_owned()], None);
        crate::responses::sync_catalog_from_status(
            Some(&catalog_path),
            &json!({
                "state": "ok",
                "host_running": false,
                "auth_state": "host_stopped",
                "models": ["gpt-5-6"]
            }),
            &config::ModelConfig::default(),
            Some(&live_set),
            true,
        );
        assert_eq!(std::fs::metadata(&catalog_path).unwrap().ino(), ino1);
        assert_eq!(std::fs::read_to_string(&catalog_path).unwrap(), before_raw);
        assert!(live_set.read().unwrap().contains("gpt-6-2-thinking"));

        // print-codex-profile emits model_catalog_json and model_reasoning_summary in both modes.
        let standalone = format_codex_profile("127.0.0.1:4319", "gpt-6-2", &catalog_path, true);
        assert!(standalone.contains("model_catalog_json = "));
        assert!(standalone.contains("model-catalog.json"));
        assert!(standalone.contains("model_reasoning_summary = \"auto\""));
        let profile = format_codex_profile("127.0.0.1:4319", "gpt-6-2", &catalog_path, false);
        assert!(profile.contains("[profiles.chatgpt]"));
        assert!(profile.contains("model_catalog_json = "));
        assert!(profile.contains("model_reasoning_summary = \"auto\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn effort_routing_maps_canonical_models_to_variant_slugs() {
        let known: std::collections::HashSet<String> = [
            "gpt-6-2",
            "gpt-6-2-instant",
            "gpt-6-2-thinking",
            "gpt-6-mini",
            "gpt-6-t-mini",
            "gpt-6-pro",
            "gpt-6-sol-wm",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        assert_eq!(
            crate::responses::resolve_dispatch_model_and_effort("gpt-6-2", Some("none"), &known),
            ("gpt-6-2-instant".to_owned(), None)
        );
        assert_eq!(
            crate::responses::resolve_dispatch_model_and_effort("gpt-6-2", Some("xhigh"), &known),
            ("gpt-6-2-thinking".to_owned(), Some("xhigh".to_owned()))
        );
        assert_eq!(
            crate::responses::resolve_dispatch_model_and_effort("gpt-6-mini", Some("high"), &known),
            ("gpt-6-t-mini".to_owned(), Some("high".to_owned()))
        );
        assert_eq!(
            crate::responses::resolve_dispatch_model_and_effort("gpt-6-mini", Some("none"), &known),
            ("gpt-6-mini".to_owned(), None)
        );
        assert_eq!(
            crate::responses::resolve_dispatch_model_and_effort("gpt-6-pro", Some("max"), &known),
            ("gpt-6-pro".to_owned(), Some("max".to_owned()))
        );
        assert_eq!(
            crate::responses::resolve_dispatch_model_and_effort(
                "gpt-6-sol-wm",
                Some("high"),
                &known
            ),
            ("gpt-6-sol-wm".to_owned(), Some("high".to_owned()))
        );
    }

    #[tokio::test]
    async fn responses_mutable_revisions_and_stream_nonstream_equivalence() {
        use axum::body::to_bytes;
        let settled_items = json!([
            {"id": "thought:m1:1", "kind": "reasoning_summary", "text": "Checking files"},
            {"id": "thought:m1:2", "kind": "reasoning_summary", "text": "Inspecting 한국어 🚀 diff"},
            {"id": "commentary:m2", "kind": "commentary", "text": "Looking at src/main.rs now."},
            {"id": "answer:m3", "kind": "answer", "text": "  Preserved spaces\n다음 줄 OK  "},
            {"id": "tool:call_1", "kind": "tool_call", "call_id": "call_1", "name": "exec_command", "arguments": "{\"cmd\":\"pwd\"}"}
        ]);
        let reply = json!({
            "state": "settled",
            "chat_id": "conv-norm-1",
            "remote_head": "head-norm-1",
            "account_id": "acct-norm",
            "items": settled_items,
            "assistant_text": "  Preserved spaces\n다음 줄 OK  ",
            "reasoning_summary": "Checking files\n\nInspecting 한국어 🚀 diff",
            "tool_calls": [{"id": "call_1", "name": "exec_command", "arguments": "{\"cmd\":\"pwd\"}"}],
            "test_delay_ms": 0,
            "test_snapshots": [
                {"item_id": "thought:m1:1", "kind": "reasoning_summary", "text": "Wrong draft heading", "committed": false},
                {"item_id": "thought:m1:1", "kind": "reasoning_summary", "text": "Checking files", "committed": true},
                {"item_id": "answer:m3", "kind": "answer", "text": "Initial draft that gets completely replaced", "committed": false},
                {"item_id": "thought:m1:2", "kind": "reasoning_summary", "text": "Inspecting 한국어 🚀 diff", "committed": true},
                {"item_id": "commentary:m2", "kind": "commentary", "text": "Looking at src/main.rs now.", "committed": true},
                {"item_id": "answer:m3", "kind": "answer", "text": "", "committed": false},
                {"item_id": "answer:m3", "kind": "answer", "text": "  Preserved spaces\n다음 줄 OK  ", "committed": true},
                {"kind": "tool_call", "call_id": "call_1", "name": "exec_command", "arguments": "{\"cmd\":\"pwd\"}"}
            ]
        });
        let driver = MockDriver::new(reply);
        let sigs_input =
            json!([{"type": "function", "name": "exec_command", "parameters": {"type": "object"}}]);

        let stream_resp = crate::responses::handle_responses(
            driver.clone(),
            &Default::default(),
            &Default::default(),
            Arc::new(Default::default()),
            None,
            None,
            json!({"model": "gpt-6-2", "input": "hello", "stream": true, "tools": sigs_input}),
        )
        .await
        .unwrap();
        let sse = String::from_utf8(
            to_bytes(stream_resp.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(!sse.contains("chatgpt_stream_revised"));
        assert!(!sse.contains("Wrong draft heading"));
        assert!(!sse.contains("Initial draft"));

        let events: Vec<Value> = sse
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(|raw| serde_json::from_str::<Value>(raw).unwrap())
            .collect();
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev["sequence_number"], i);
        }
        let completed = events
            .iter()
            .find(|ev| ev["type"] == "response.completed")
            .unwrap()["response"]
            .clone();

        // Check SSE delta concatenation matches final output text/summary & tool deduplication (exactly once).
        let rs_deltas: Vec<String> = events
            .iter()
            .filter(|ev| ev["type"] == "response.reasoning_summary_text.delta")
            .map(|ev| ev["delta"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            rs_deltas,
            vec![
                "Checking files".to_owned(),
                "Inspecting 한국어 🚀 diff".to_owned()
            ]
        );
        let msg_deltas: Vec<String> = events
            .iter()
            .filter(|ev| ev["type"] == "response.output_text.delta")
            .map(|ev| ev["delta"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            msg_deltas,
            vec![
                "Looking at src/main.rs now.".to_owned(),
                "  Preserved spaces\n다음 줄 OK  ".to_owned()
            ]
        );
        let fc_count = events
            .iter()
            .filter(|ev| ev["type"] == "response.function_call_arguments.done")
            .count();
        assert_eq!(fc_count, 1, "tool call must be emitted exactly once");

        // Compare stream=true output vs stream=false output structurally (modulo unique item IDs).
        let json_resp = crate::responses::handle_responses(
            driver,
            &Default::default(),
            &Default::default(),
            Arc::new(Default::default()),
            None,
            None,
            json!({"model": "gpt-6-2", "input": "hello", "stream": false, "tools": sigs_input}),
        )
        .await
        .unwrap();
        let non_stream: Value =
            serde_json::from_slice(&to_bytes(json_resp.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        let norm_output = |out: &Value| -> Vec<Value> {
            out.as_array()
                .unwrap()
                .iter()
                .map(|item| {
                    let mut c = item.clone();
                    c.as_object_mut().unwrap().remove("id");
                    c
                })
                .collect()
        };
        assert_eq!(
            norm_output(&completed["output"]),
            norm_output(&non_stream["output"])
        );
        assert_eq!(completed["usage"], non_stream["usage"]);
    }

    #[tokio::test]
    async fn responses_blocks_string_replay_and_disarms_cancel_after_settlement() {
        use axum::body::to_bytes;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct SettledDriver {
            stopped: Arc<AtomicBool>,
        }
        impl DriverBackend for SettledDriver {
            fn host_running(&self) -> BoxFuture<'_, bool> {
                Box::pin(async { true })
            }
            fn status(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn list_chats(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn read_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn rename_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn list_projects(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn read_project(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn create_project(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn send_message(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async {
                    Ok(json!({
                        "state": "settled",
                        "chat_id": "conv-settled-1",
                        "account_id": "acct-1",
                        "remote_head": "head-settled-1",
                        "assistant_text": "Done",
                        "tool_calls": []
                    }))
                })
            }
            fn stream_message(
                &self,
                p: Value,
                _tx: crate::cdp::LaneSender,
            ) -> BoxFuture<'_, Result<Value>> {
                self.send_message(p)
            }
            fn observe_chat(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"settled"})) })
            }
            fn stop_generation(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                self.stopped.store(true, Ordering::Release);
                Box::pin(async { Ok(json!({"state":"stopped","stopped":true})) })
            }
            fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok","available":false})) })
            }
            fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn list_connectors(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok","tunnels":[],"connectors":[]})) })
            }
            fn register_connector(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
            fn refresh_connector(&self, _: Value) -> BoxFuture<'_, Result<Value>> {
                Box::pin(async { Ok(json!({"state":"ok"})) })
            }
        }

        let stopped = Arc::new(AtomicBool::new(false));
        let driver: Arc<dyn DriverBackend> = Arc::new(SettledDriver {
            stopped: Arc::clone(&stopped),
        });
        let map = Arc::new(crate::responses::ResponseThreadMap::default());
        let resp = crate::responses::handle_responses(
            driver.clone(),
            &Default::default(),
            &Default::default(),
            Arc::clone(&map),
            Some("str-sess".to_owned()),
            None,
            json!({"input": "Same string prompt", "stream": true}),
        )
        .await
        .unwrap();
        let _ = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !stopped.load(Ordering::Acquire),
            "settled response must disarm CancelTurnGuard"
        );
        let replay_err = crate::responses::handle_responses(
            driver,
            &Default::default(),
            &Default::default(),
            map,
            Some("str-sess".to_owned()),
            None,
            json!({"input": "Same string prompt", "stream": true}),
        )
        .await
        .unwrap_err();
        assert_eq!(replay_err.code, "response_input_already_sent");
    }

    #[tokio::test]
    async fn memory_utility_call_bypasses_session_lease_project_and_unknown_slug() {
        let driver = MockDriver::new(json!({
            "state": "settled",
            "chat_id": "conv-mem-1",
            "account_id": "acct-1",
            "assistant_text": "Memory saved"
        }));
        let router = build_router(
            driver.clone(),
            config::ModelConfig {
                default: "gpt-6-2".to_owned(),
                available: vec![
                    "gpt-6-2".to_owned(),
                    "gpt-6-2-instant".to_owned(),
                    "gpt-6-2-thinking".to_owned(),
                ],
                ..config::ModelConfig::default()
            },
            config::HistoryConfig {
                project_id: Some("g-p-user-proj".to_owned()),
                ..config::HistoryConfig::default()
            },
            "tok-0123456789abcdef0123456789abcdef".to_owned(),
            Arc::new(RwLock::new(None)),
        );

        let mem_body = json!({
            "model": "gpt-6-luna",
            "stream": true,
            "prompt_cache_key": "sess-shared-1",
            "input": [
                {"role": "developer", "content": [{"type": "input_text", "text": "## Memory Writing Agent: Phase 1"}]}
            ]
        });
        for metadata in [
            r#"{"request_kind":"memory","thread_source":"memory_consolidation","turn_trigger":"memory_consolidation"}"#,
            r#"{"request_kind":"memories","thread_source":"memories"}"#,
            r#"{"request_kind":"title_gen","thread_source":"title_gen"}"#,
        ] {
            let resp_mem = oneshot(
                router.clone(),
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .header("x-codex-turn-metadata", metadata)
                    .body(Body::from(mem_body.to_string()))
                    .unwrap(),
            )
            .await;
            assert_eq!(resp_mem.status(), StatusCode::OK);
            let sent = driver.last_send.lock().unwrap().clone().unwrap();
            assert!(sent["project_id"].is_null());
            assert!(sent["chat_id"].is_null());
            assert_eq!(sent["temporary"], true);
            assert_eq!(sent["model"], "gpt-6-2-instant");
            assert!(sent["reasoning_effort"].is_null());
            assert_eq!(sent["local_function_signatures"], json!([]));
            let _ = to_bytes(resp_mem.into_body(), 65536).await.unwrap();
        }

        // Real user turn with the exact same prompt_cache_key must NOT see any stale thread
        // entry from the utility memory call and must still pin the configured project.
        let user_body = json!({
            "model": "gpt-6-2",
            "stream": false,
            "prompt_cache_key": "sess-shared-1",
            "reasoning": {"effort": "high"},
            "tools": [{"type": "function", "name": "exec_command", "parameters": {"type": "object"}}],
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "Hello"}]}]
        });
        let resp_user = oneshot(
            router,
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .header(
                    "x-codex-turn-metadata",
                    r#"{"request_kind":"turn","thread_source":"user","turn_trigger":"exec"}"#,
                )
                .body(Body::from(user_body.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(resp_user.status(), StatusCode::OK);
        let sent_user = driver.last_send.lock().unwrap().clone().unwrap();
        assert_eq!(sent_user["project_id"], "g-p-user-proj");
        assert!(sent_user["chat_id"].is_null());
        assert_eq!(sent_user["model"], "gpt-6-2-thinking");
    }

    #[tokio::test]
    async fn responses_dedup_commentary_and_render_dil_equivalently_across_stream_and_json() {
        let dil_commentary = "## 저장소 상태\n\n<row align=\"center\" gap={2}>\n  <badge color=\"success\">정상</badge>\n  <text color=\"secondary\" size=\"sm\">실제 조회 기준</text>\n</row>";
        let dup_answer = format!(
            "{dil_commentary}\n\nGitHub: <Link url=\"https://github.com/humtr/codex\" title=\"humtr/codex\"/>"
        );
        let reply = json!({
            "state": "settled",
            "chat_id": "conv-dil-1",
            "account_id": "acct-1",
            "test_delay_ms": 0,
            "items": [
                {"id": "commentary:m1", "kind": "commentary", "text": dil_commentary},
                {"id": "commentary:m2", "kind": "commentary", "text": dil_commentary},
                {"id": "answer:m3", "kind": "answer", "text": dup_answer}
            ],
            "test_snapshots": [
                {"item_id": "commentary:m1", "kind": "commentary", "text": dil_commentary, "committed": true},
                {"item_id": "commentary:m2", "kind": "commentary", "text": dil_commentary, "committed": true},
                {"item_id": "answer:m3", "kind": "answer", "text": dup_answer, "committed": true}
            ]
        });
        let driver = MockDriver::new(reply);
        let stream_resp = crate::responses::handle_responses(
            driver.clone(),
            &Default::default(),
            &Default::default(),
            Arc::new(Default::default()),
            None,
            None,
            json!({"input": "Check", "stream": true}),
        )
        .await
        .unwrap();
        let sse = String::from_utf8(
            to_bytes(stream_resp.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        let completed_data = sse
            .split("\n\n")
            .find_map(|block| {
                let mut ev = "";
                let mut data = "";
                for line in block.lines() {
                    if let Some(v) = line.strip_prefix("event: ") {
                        ev = v;
                    } else if let Some(v) = line.strip_prefix("data: ") {
                        data = v;
                    }
                }
                (ev == "response.completed").then(|| serde_json::from_str::<Value>(data).unwrap())
            })
            .expect("completed event");

        let json_resp = crate::responses::handle_responses(
            driver,
            &Default::default(),
            &Default::default(),
            Arc::new(Default::default()),
            None,
            None,
            json!({"input": "Check", "stream": false}),
        )
        .await
        .unwrap();
        let json_val: Value =
            serde_json::from_slice(&to_bytes(json_resp.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();

        let stream_out = completed_data["response"]["output"].as_array().unwrap();
        let json_out = json_val["output"].as_array().unwrap();
        assert_eq!(stream_out.len(), 2);
        assert_eq!(json_out.len(), 2);
        assert_eq!(stream_out[0]["phase"], "commentary");
        assert_eq!(stream_out[1]["phase"], "final_answer");
        let c_text = stream_out[0]["content"][0]["text"].as_str().unwrap();
        let a_text = stream_out[1]["content"][0]["text"].as_str().unwrap();
        assert!(c_text.contains("**[✓ 정상]** · *실제 조회 기준*"));
        assert!(!c_text.contains("<row"));
        assert_eq!(
            a_text,
            "GitHub: [humtr/codex](https://github.com/humtr/codex)"
        );
        assert_eq!(json_out[0]["content"][0]["text"].as_str().unwrap(), c_text);
        assert_eq!(json_out[1]["content"][0]["text"].as_str().unwrap(), a_text);
    }
}
