//! `chatgpt-provider` — slim Stage 2 Type 1 ChatGPT Desktop Provider.
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
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::signal::unix::{signal, SignalKind};

fn print_help() {
    println!(
        "chatgpt-provider {} — slim Stage 2 ChatGPT Desktop Provider\n\n\
         Usage:\n\
         \x20 chatgpt-provider serve [--config <path>] [--bind 127.0.0.1:<port>] [--state <dir>] [--rootfs <dir>] [--eager-host]\n\
         \x20 chatgpt-provider status [--config <path>]\n\
         \x20 chatgpt-provider print-codex-profile [--bind 127.0.0.1:4319] [--model auto] [--standalone]\n\
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
            let mut bind = "127.0.0.1:4319".to_owned();
            let mut model = "auto".to_owned();
            let mut standalone = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
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
            if standalone {
                println!(
                    "# Standalone Codex provider config (`~/.codex/chatgpt.config.toml`) for `!cdx provider chatgpt`\n\
                     model = \"{model}\"\n\
                     model_provider = \"chatgpt_desktop\"\n\n\
                     [model_providers.chatgpt_desktop]\n\
                     name = \"ChatGPT Desktop (Stage 2)\"\n\
                     base_url = \"http://{bind}/v1\"\n\
                     wire_api = \"responses\"\n\
                     requires_openai_auth = false\n\
                     supports_websockets = false\n\
                     request_max_retries = 0\n\
                     stream_max_retries = 0"
                );
            } else {
                println!(
                    "# Opt-in Codex CLI profile (`codex -p chatgpt` in `~/.codex/config.toml`) — zero Wrapper coupling\n\
                     # (Or pass `--standalone` to generate `~/.codex/chatgpt.config.toml`)\n\
                     [profiles.chatgpt]\n\
                     model = \"{model}\"\n\
                     model_provider = \"chatgpt_desktop\"\n\n\
                     [model_providers.chatgpt_desktop]\n\
                     name = \"ChatGPT Desktop (Stage 2)\"\n\
                     base_url = \"http://{bind}/v1\"\n\
                     wire_api = \"responses\"\n\
                     requires_openai_auth = false\n\
                     supports_websockets = false\n\
                     request_max_retries = 0\n\
                     stream_max_retries = 0"
                );
            }
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
            }

            let driver_dyn: Arc<dyn DriverBackend> = driver.clone();
            let app = build_router_with_threads(
                driver_dyn,
                cfg.model,
                cfg.history,
                token,
                viewer_slot,
                threads,
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
            tx: tokio::sync::mpsc::Sender<crate::cdp::StreamUpdate>,
        ) -> BoxFuture<'_, Result<Value>> {
            use crate::cdp::StreamUpdate;
            let result = self.send_message(p);
            Box::pin(async move {
                let reply = result.await?;
                if let Some(snapshots) = reply["test_snapshots"].as_array() {
                    for snapshot in snapshots {
                        let text = snapshot["text"].as_str().unwrap().to_owned();
                        let event = if snapshot["kind"] == "reasoning" {
                            StreamUpdate::Reasoning(text)
                        } else {
                            StreamUpdate::Answer(text)
                        };
                        tx.send(event).await.unwrap();
                        tokio::time::sleep(std::time::Duration::from_millis(
                            reply["test_delay_ms"].as_u64().unwrap_or(125_000),
                        ))
                        .await;
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

        // Verify local_function_signatures passed to page driver!
        let sent = driver.last_send.lock().unwrap().clone().unwrap();
        assert_eq!(sent["local_function_signatures"][0]["name"], "exec_command");
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
            "assistant_text": "",
            "tool_calls": [
                {
                    "id": "call_exec_1",
                    "name": "exec",
                    "arguments": "echo hi"
                },
                {
                    "id": "call_spawn_1",
                    "name": "spawn_agent",
                    "arguments": "{\"task_name\":\"worker\",\"message\":\"compute\"}"
                }
            ]
        }));
        let history = config::HistoryConfig {
            project_id: Some("g-p-test-proj".to_owned()),
            project_name: None,
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
        assert_eq!(
            sent1["local_function_signatures"].as_array().unwrap().len(),
            2
        );

        // Round 2 with same thread-id must reuse conv-orch-1 and send only delta tail!
        let mut r2_body = r1_body.clone();
        r2_body["input"].as_array_mut().unwrap().push(json!({
            "type": "function_call",
            "call_id": "call_spawn_1",
            "name": "spawn_agent",
            "arguments": "{}"
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
        assert!(sent2["prompt"].as_str().unwrap().contains("WORKER_DONE=42"));
        assert!(sent2["prompt"]
            .as_str()
            .unwrap()
            .contains("READ_SECRET_TASK_99"));
        assert!(!sent2["prompt"].as_str().unwrap().contains("Delegate work"));
        assert_eq!(sent2["is_continuation"], true);
        assert_eq!(
            sent2["local_function_signatures"].as_array().unwrap().len(),
            2
        );

        // Keep round 2's cached chat and grow the input so the changed prefix
        // itself selects a fresh chat.
        let mut r3_body = r2_body;
        r3_body["input"][1]["content"][0]["text"] = json!("Compacted summary of prior work");
        r3_body["input"].as_array_mut().unwrap().push(json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "Continue after compaction"}]
        }));
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
        assert_eq!(sent3["is_continuation"], false);

        // Background Transcript Title Generation MUST omit project_id to avoid polluting projects!
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
                    project_name: None,
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
    }

    #[tokio::test(start_paused = true)]
    async fn responses_flush_live_items_before_long_turn_settles() {
        use futures_util::StreamExt;
        use std::time::Duration;
        let driver = MockDriver::new(json!({
            "state": "settled",
            "reasoning_summary": "Visible summary",
            "assistant_text": "Final answer",
            "test_snapshots": [
                {"kind":"reasoning","text":"Visible summary"},
                {"kind":"answer","text":"Final answer"}
            ]
        }));
        let start = tokio::time::Instant::now();
        let response = crate::responses::handle_responses(
            driver,
            &Default::default(),
            &Default::default(),
            Arc::new(Default::default()),
            Some("test".to_owned()),
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
            "state":"settled","chat_id":"conv-one","account_id":"acct-one",
            "reasoning_summary":"Only summary", "assistant_text":""
        }));
        let first = json!({"input":[{"role":"user","content":"First"}],"stream":true});
        let mut second = first.clone();
        second["input"]
            .as_array_mut()
            .unwrap()
            .push(json!({"role":"user","content":"Second"}));
        for (round, body) in [first, second].into_iter().enumerate() {
            let map = Arc::new(crate::responses::ResponseThreadMap::open(&state).unwrap());
            let response = crate::responses::handle_responses(
                driver.clone(),
                &Default::default(),
                &Default::default(),
                Arc::clone(&map),
                Some("test-session".to_owned()),
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
            let sent = driver.last_send.lock().unwrap();
            if round == 0 {
                assert!(sent.as_ref().unwrap()["chat_id"].is_null());
            } else {
                assert_eq!(sent.as_ref().unwrap()["chat_id"], "conv-one");
                assert_eq!(sent.as_ref().unwrap()["expected_account_id"], "acct-one");
                assert!(!sent.as_ref().unwrap()["prompt"]
                    .as_str()
                    .unwrap()
                    .contains("First"));
            }
            drop(sent);
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
                        "description":"Stage 2 shell MCP"
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
}
