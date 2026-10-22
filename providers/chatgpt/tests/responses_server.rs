//! Real TCP/SSE/tool-return proof with a deterministic host; no paid submissions.
use codex_chatgpt_provider::{
    config::Config,
    protocol::{gateway_calls, Error, Request},
    responses::Event,
    server::{self, Host, Service},
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

const TOKEN: &str = "owned_fixture_token_012345678901234567890123456789";
#[derive(Default, Clone)]
struct Fixture {
    calls: Arc<AtomicUsize>,
    cancelled: Arc<AtomicUsize>,
    reveals: Arc<AtomicUsize>,
    approvals: Arc<AtomicUsize>,
}
impl Host for Fixture {
    fn approve(&self, thread: String, ui: bool) -> BoxFuture<'static, Result<Value, Error>> {
        let this = self.clone();
        Box::pin(async move {
            if thread != "owned-navigation" {
                return Err(Error("native_lane_not_found"));
            }
            this.approvals.fetch_add(1, Ordering::SeqCst);
            Ok(
                json!({"app_approval_requested":true,"scope":"conversation","path":if ui {"ui"} else {"action"}}),
            )
        })
    }
    fn reveal(&self, thread: String) -> BoxFuture<'static, Result<(), Error>> {
        let this = self.clone();
        Box::pin(async move {
            if thread != "owned-navigation" {
                return Err(Error("native_lane_not_found"));
            }
            this.reveals.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn exchange(
        &self,
        request: Request,
        events: mpsc::Sender<Event>,
        cancel: CancellationToken,
    ) -> BoxFuture<'static, Result<(), Error>> {
        let fixture = self.clone();
        Box::pin(async move {
            fixture.calls.fetch_add(1, Ordering::SeqCst);
            if request.thread == "cancellation" {
                events
                    .send(Event::Text {
                        id: "progress".into(),
                        phase: "commentary".into(),
                        delta: "Running".into(),
                    })
                    .await
                    .unwrap();
                cancel.cancelled().await;
                fixture.cancelled.fetch_add(1, Ordering::SeqCst);
                return Err(Error("cancelled"));
            }
            if request.results().next().is_none() {
                let (calls, _) = gateway_calls(
                    "original-host-call",
                    r#"{"name":"functions.exec","input":"{\"cmd\":\"pwd\"}"}"#,
                    &request.tools,
                )?;
                events
                    .send(Event::Text {
                        id: "progress".into(),
                        phase: "commentary".into(),
                        delta: "Checking files".into(),
                    })
                    .await
                    .unwrap();
                events.send(Event::Tool(calls[0].clone())).await.unwrap();
            } else {
                let results = request.results().collect::<Vec<_>>();
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].0, "original-host-call");
                assert_eq!(results[0].1["output"], "/owned/project");
                events
                    .send(Event::Text {
                        id: "final".into(),
                        phase: "final_answer".into(),
                        delta: "Verified /owned/project".into(),
                    })
                    .await
                    .unwrap();
            }
            events
                .send(Event::Completed {
                    input_tokens: 10,
                    output_tokens: 5,
                })
                .await
                .unwrap();
            Ok(())
        })
    }
}
async fn start(config: Config) -> (std::net::SocketAddr, Arc<Fixture>, CancellationToken) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let fixture = Arc::new(Fixture::default());
    let service = Service::new(config, TOKEN.into(), fixture.clone()).unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    tokio::spawn(async move {
        server::serve(listener, service, shutdown).await.unwrap();
    });
    (address, fixture, stop)
}
async fn send(address: std::net::SocketAddr, body: &[u8], headers: &str) -> TcpStream {
    let mut stream = TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("POST /v1/responses HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\n{headers}\r\n",body.len()).as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
    stream
}
async fn collect(mut stream: TcpStream) -> String {
    let mut out = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.read_to_end(&mut out),
    )
    .await
    .unwrap()
    .unwrap();
    String::from_utf8(out).unwrap()
}
fn body() -> Value {
    json!({"model":"chatgpt-desktop/gpt-6","stream":true,"client_metadata":{"x-codex-turn-metadata":{"thread_id":"fixture"}},"input":"Inspect files","tools":[{"type":"namespace","name":"functions","tools":[{"type":"function","name":"exec","parameters":{"type":"object"}}]}]})
}
fn frames(raw: &str) -> Vec<Value> {
    raw.lines()
        .filter_map(|s| {
            s.strip_prefix("data: ")
                .map(|s| serde_json::from_str(s).unwrap())
        })
        .collect()
}

#[tokio::test]
async fn reveal_requires_local_auth_and_an_exact_native_thread_without_inference() {
    let (address, fixture, stop) = start(Config::default()).await;
    for endpoint in ["reveal", "approve", "approve_ui"] {
        for (extra, status) in [
            ("X-Codex-Thread-Id: owned-navigation\r\n", 200),
            ("X-Codex-Thread-Id: missing\r\n", 409),
            ("", 400),
            (
                "X-Codex-Thread-Id: owned-navigation\r\nOrigin: https://unowned.invalid\r\n",
                401,
            ),
            (
                "X-Codex-Thread-Id: owned-navigation\r\nAuthorization: Bearer wrong\r\n",
                401,
            ),
        ] {
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream.write_all(format!("POST /__{endpoint} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 0\r\n{extra}\r\n").as_bytes()).await.unwrap();
            let result = collect(stream).await;
            assert!(
                result.starts_with(&format!("HTTP/1.1 {status}")),
                "{result}"
            );
        }
    }
    assert_eq!(fixture.approvals.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.reveals.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    stop.cancel();
}

#[tokio::test]
async fn responses_sse_returns_native_tools_and_public_transcript_in_order() {
    let (address, fixture, stop) = start(Config::default()).await;
    let raw = collect(send(address, body().to_string().as_bytes(), "").await).await;
    assert!(raw.starts_with("HTTP/1.1 200"), "{raw}");
    let first = frames(&raw);
    let result = &first.last().unwrap()["response"];
    assert_eq!(result["output"][0]["phase"], "commentary");
    assert_eq!(result["output"][1]["namespace"], "functions");
    assert_eq!(result["output"][1]["call_id"], "original-host-call");
    let mut next = body();
    next["input"] = json!([{"type":"function_call_output","call_id":"original-host-call","output":"/owned/project"}]);
    let compressed = zstd::stream::encode_all(next.to_string().as_bytes(), 1).unwrap();
    let raw = collect(send(address, &compressed, "Content-Encoding: zstd\r\n").await).await;
    let second = frames(&raw);
    assert_eq!(
        second.last().unwrap()["response"]["output"][0]["content"][0]["text"],
        "Verified /owned/project"
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 2);
    stop.cancel();
}

#[tokio::test]
async fn admission_bounds_and_origin_fail_before_any_host_work() {
    let mut config = Config::default();
    config.transport.max_wire_bytes = 512;
    config.transport.max_decoded_bytes = 1024;
    let (address, fixture, stop) = start(config).await;
    for (bytes, headers, status, code) in [
        (
            body().to_string().into_bytes(),
            "Origin: https://unowned.invalid\r\n",
            401,
            "origin_not_allowed",
        ),
        (vec![b'x'; 513], "", 413, "wire_body_too_large"),
        (
            zstd::stream::encode_all(vec![b'x'; 1025].as_slice(), 1).unwrap(),
            "Content-Encoding: zstd\r\n",
            413,
            "decoded_body_too_large",
        ),
        (
            b"broken".to_vec(),
            "Content-Encoding: zstd\r\n",
            400,
            "invalid_zstd_body",
        ),
        (
            b"{private malformed text".to_vec(),
            "",
            400,
            "invalid_json_body",
        ),
    ] {
        let raw = collect(send(address, &bytes, headers).await).await;
        assert!(raw.starts_with(&format!("HTTP/1.1 {status}")), "{raw}");
        assert!(raw.contains(code), "{raw}");
        assert!(!raw.contains("private malformed"));
    }
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    stop.cancel();
}

#[tokio::test]
async fn silent_host_keeps_data_events_alive_without_another_exchange_or_transcript_item() {
    let (address, fixture, stop) = start(Config::default()).await;
    let mut value = body();
    value["client_metadata"]["x-codex-turn-metadata"]["thread_id"] = json!("cancellation");
    let mut stream = send(address, value.to_string().as_bytes(), "").await;
    let mut raw = Vec::new();
    tokio::time::timeout(std::time::Duration::from_secs(7), async {
        let mut buffer = [0; 4096];
        loop {
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0, "retained stream closed during host wait");
            raw.extend_from_slice(&buffer[..count]);
            if String::from_utf8_lossy(&raw)
                .matches(r#"data: {"type":"ping"}"#)
                .count()
                >= 2
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    let observed = frames(&String::from_utf8(raw).unwrap());
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.cancelled.load(Ordering::SeqCst), 0);
    assert_eq!(
        observed
            .iter()
            .filter(|v| v["type"] == "response.output_item.added")
            .count(),
        1
    );
    assert!(observed
        .iter()
        .filter(|v| v["type"] == "ping")
        .all(|v| v.as_object().unwrap().len() == 1));
    assert!(!observed.iter().any(|v| v["type"] == "response.completed"));
    drop(stream);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while fixture.cancelled.load(Ordering::SeqCst) != 1 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    stop.cancel();
}

#[tokio::test]
async fn closing_the_native_observer_requests_owned_host_cancellation() {
    let (address, fixture, stop) = start(Config::default()).await;
    let mut value = body();
    value["client_metadata"]["x-codex-turn-metadata"]["thread_id"] = json!("cancellation");
    let mut stream = send(address, value.to_string().as_bytes(), "").await;
    let mut bytes = [0; 4096];
    assert!(stream.read(&mut bytes).await.unwrap() > 0);
    drop(stream);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while fixture.cancelled.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    stop.cancel();
}

#[tokio::test]
async fn config_file_changes_routes_at_next_request_without_rebuild_or_restart() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "provider-reload-{}",
        codex_chatgpt_provider::journal::random_id().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("config.toml");
    let fixture = Arc::new(Fixture::default());
    let service = Service::configured(path.clone(), TOKEN.into(), fixture.clone()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let worker = tokio::spawn(async move {
        server::serve(listener, service, shutdown).await.unwrap();
    });
    let mut value = body();
    value["model"] = json!("chatgpt-desktop/configured");
    let rejected = collect(send(address, value.to_string().as_bytes(), "").await).await;
    assert!(rejected.contains("unsupported_desktop_route"));
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    let mut config = Config::default();
    let mut model = config.catalog.models[0].clone();
    model.id = "chatgpt-desktop/configured".into();
    model.routes[0].host_model = "configured-host".into();
    config.catalog.models.push(model);
    codex_chatgpt_provider::store::Snapshot::open(&path)
        .unwrap()
        .commit_bytes(config.render().unwrap().as_bytes())
        .unwrap();
    let accepted = collect(send(address, value.to_string().as_bytes(), "").await).await;
    assert_eq!(
        frames(&accepted).last().unwrap()["response"]["model"],
        "chatgpt-desktop/configured"
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    config.transport.return_path = codex_chatgpt_provider::config::ReturnPath::Mcp;
    codex_chatgpt_provider::store::Snapshot::open(&path)
        .unwrap()
        .commit_bytes(config.render().unwrap().as_bytes())
        .unwrap();
    let rejected = collect(send(address, value.to_string().as_bytes(), "").await).await;
    assert!(rejected.contains("provider_transport_restart_required"));
    assert_eq!(
        fixture.calls.load(Ordering::SeqCst),
        1,
        "transport changes never switch an active service"
    );
    std::fs::write(&path, "invalid = [").unwrap();
    let rejected = collect(send(address, value.to_string().as_bytes(), "").await).await;
    assert!(rejected.starts_with("HTTP/1.1 503"));
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    stop.cancel();
    worker.await.unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
