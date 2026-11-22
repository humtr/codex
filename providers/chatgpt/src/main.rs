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
mod viewer;

use crate::config::{load_or_create_control_token, user_home, ProviderConfig};
use crate::driver::{CdpDriver, DriverBackend};
use crate::error::Result;
use crate::server::build_router;
use crate::viewer::ViewerSlot;
use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

fn print_help() {
    println!(
        "chatgpt-provider {} — slim Stage 2 ChatGPT Desktop Provider\n\n\
         Usage:\n\
         \x20 chatgpt-provider serve [--config <path>] [--bind 127.0.0.1:<port>] [--state <dir>] [--rootfs <dir>] [--eager-host]\n\
         \x20 chatgpt-provider status [--config <path>]\n\
         \x20 chatgpt-provider print-codex-profile [--bind 127.0.0.1:4319] [--model auto]\n\
         \x20 chatgpt-provider --version | --help",
        env!("CARGO_PKG_VERSION")
    );
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
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
                    other => {
                        eprintln!("chatgpt-provider: unknown flag '{other}'");
                        std::process::exit(2);
                    }
                }
            }
            println!(
                "# Opt-in Codex CLI profile (`codex -p chatgpt`) — zero Wrapper coupling\n\
                 [profiles.chatgpt]\n\
                 model = \"{model}\"\n\
                 model_provider = \"chatgpt_desktop\"\n\n\
                 [model_providers.chatgpt_desktop]\n\
                 name = \"ChatGPT Desktop (Stage 2)\"\n\
                 base_url = \"http://{bind}/v1\"\n\
                 wire_api = \"responses\"\n\
                 requires_openai_auth = false"
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
            let (token_path, token) = load_or_create_control_token(&state_dir)?;
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

            if cfg.host.eager_start {
                let _ = driver.restart_host().await?;
            }

            let driver_dyn: Arc<dyn DriverBackend> = driver.clone();
            let app = build_router(driver_dyn, cfg.model, cfg.history, token, viewer_slot);
            eprintln!(
                "chatgpt-provider {} listening on http://{} (control token: {})",
                env!("CARGO_PKG_VERSION"),
                local_addr,
                token_path.display()
            );
            let serve_res = axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await;
            driver.shutdown().await;
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

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
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
        fn observe_chat(&self, p: Value) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async move {
                Ok(json!({"state":"settled","chat_id":p["chat_id"],"generating":false}))
            })
        }
        fn stop_generation(&self) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async { Ok(json!({"state":"stopped","stopped":true})) })
        }
        fn viewer_info(&self) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async { Ok(json!({"state":"ok","available":false,"url":null})) })
        }
        fn restart_host(&self) -> BoxFuture<'_, Result<Value>> {
            Box::pin(async { Ok(json!({"state":"ok","restarted":true})) })
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
            let body = resp_bytes[split + 4..].to_vec();
            let mut builder = Response::builder().status(status_code);
            for line in head.lines().skip(1) {
                if let Some((k, v)) = line.split_once(':') {
                    builder = builder.header(k.trim(), v.trim());
                }
            }
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
            router,
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
    }
}
