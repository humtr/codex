//! Rust MCP HTTP -> common lane -> native call/result -> original HTTP reply.
//! Synthetic app only; no Chat submissions, remote connector or credentials.
use codex_chatgpt_provider::{
    app::{AppHost, Bridge},
    config::Config,
    context::digest,
    journal::{random_id, Journal},
    mcp::{Arguments, Broker},
    mcp_bridge::McpBridge,
    protocol::{Error, Request},
    responses::{Event, Projection},
    server::Host,
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pm-{}", &random_id().unwrap()[..8]));
        std::fs::create_dir(&path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("failed fixture retained at {}", self.0.display());
            return;
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn request() -> Value {
    json!({"stream":true,"model":"chatgpt-desktop/gpt-6","instructions":"Keep native authority","input":[{"role":"user","content":"Implement and test the owned task"}],"tools":[{"type":"function","name":"exec","parameters":{"type":"object"}},{"type":"custom","name":"patch","format":{"type":"text"}}]})
}
fn parsed() -> Request {
    Request::parse(
        request(),
        &Config::default(),
        Some("owned-mcp-native-thread"),
    )
    .unwrap()
}
async fn rpc(address: std::net::SocketAddr, id: Value, method: &str, params: Value) -> Value {
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let body = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string();
    stream.write_all(format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.read_to_end(&mut bytes),
    )
    .await
    .unwrap()
    .unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.starts_with("HTTP/1.1 200"));
    serde_json::from_str(text.split_once("\r\n\r\n").unwrap().1).unwrap()
}
async fn effect(
    address: std::net::SocketAddr,
    token: &str,
    logical: &str,
    name: &str,
    input: &str,
) -> Value {
    let row=rpc(address,json!(format!("rpc-{logical}")),"tools/call",json!({"name":"native_tool","arguments":{"turn_token":token,"call_id":logical,"name":name,"input":input,"report":format!("Run {logical}")}})).await;
    assert_eq!(
        row["result"]["isError"], false,
        "MCP invocation failed: {row}"
    );
    serde_json::from_str(row["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
#[tokio::test]
async fn detached_peer_and_rpc_id_changes_reconcile_one_durable_logical_call() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let token = broker
        .register("native-lane", parsed().tools)
        .await
        .unwrap();
    let args = Arguments {
        call_id: "same-action".into(),
        name: "exec".into(),
        input: "{}".into(),
        report: "Run once".into(),
    };
    let b = broker.clone();
    let t = token.clone();
    let a = args.clone();
    let abandoned = tokio::spawn(async move { b.invoke(&t, a).await });
    let calls = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let v = broker.offer(&token).await.unwrap();
            if !v.is_empty() {
                break v;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    abandoned.abort();
    let _ = abandoned.await;
    assert_eq!(calls.len(), 1);
    let id = calls[0]["id"].as_str().unwrap();
    assert!(broker.offer(&token).await.unwrap().is_empty());
    let mut changed = args.clone();
    changed.input = "{\"changed\":true}".into();
    assert_eq!(
        broker.invoke(&token, changed).await,
        Err(Error("mcp_call_changed"))
    );
    assert_eq!(
        broker
            .settle(
                &token,
                &[
                    (id.into(), json!({"output":"must not publish"})),
                    ("unowned".into(), json!({}))
                ]
            )
            .await,
        Err(Error("mcp_result_invalid"))
    );
    let b = broker.clone();
    let t = token.clone();
    let a = args.clone();
    let retry = tokio::spawn(async move { b.invoke(&t, a).await });
    assert!(!retry.is_finished());
    let output = json!({"output":"actual native result"});
    broker
        .settle(&token, &[(id.into(), output.clone())])
        .await
        .unwrap();
    assert_eq!(retry.await.unwrap().unwrap(), output);
    assert_eq!(
        broker
            .settle(&token, &[(id.into(), json!({"output":"changed"}))])
            .await,
        Err(Error("mcp_result_changed"))
    );
    broker.retire(&token, false).await.unwrap();
    assert_eq!(
        broker.invoke(&token, args).await.unwrap(),
        output,
        "lost last HTTP delivery remains readable"
    );
    assert!(broker
        .invoke(
            &token,
            Arguments {
                call_id: "new-action".into(),
                name: "exec".into(),
                input: "{}".into(),
                report: String::new()
            }
        )
        .await
        .is_err());
    assert_eq!(
        Broker::new(journal).catalog(&token).await,
        Err(Error("mcp_turn_unavailable")),
        "another owner never reactivates old intents"
    );
}

#[tokio::test]
async fn recorded_token_can_be_retired_after_restart_without_reactivation() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let token = broker
        .register("retained-owner", parsed().tools)
        .await
        .unwrap();
    let call = Arguments {
        call_id: "completed-once".into(),
        name: "exec".into(),
        input: "{}".into(),
        report: "One original action".into(),
    };
    broker.enqueue(&token, &call).await.unwrap();
    let offered = broker.offer(&token).await.unwrap();
    broker
        .settle(
            &token,
            &[(
                offered[0]["id"].as_str().unwrap().into(),
                json!({"output":"original receipt"}),
            )],
        )
        .await
        .unwrap();
    let before = broker.recorded_calls(&token).unwrap();
    drop(broker);
    let restarted = Broker::new(journal.clone());
    assert_eq!(
        restarted.catalog(&token).await,
        Err(Error("mcp_turn_unavailable"))
    );
    restarted.retire(&token, false).await.unwrap();
    restarted.retire(&token, false).await.unwrap();
    assert_eq!(restarted.recorded_calls(&token).unwrap(), before);
    assert_eq!(
        restarted.catalog(&token).await,
        Err(Error("mcp_turn_unavailable"))
    );
    assert_eq!(
        restarted.enqueue(&token, &call).await,
        Err(Error("mcp_turn_unavailable"))
    );
    let retired: Value = journal
        .read(&digest(&("mcp-turn-v1", &token)))
        .unwrap()
        .unwrap();
    assert_eq!(retired["retired"], true);
    let fresh = restarted
        .register("retained-owner", parsed().tools)
        .await
        .unwrap();
    assert_ne!(fresh, token);
    assert!(!restarted.catalog(&fresh).await.unwrap().is_empty());
}

#[tokio::test]
async fn restarted_retirement_preserves_unreturned_calls_and_completion_guard() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let token = broker
        .register("pending-owner", parsed().tools)
        .await
        .unwrap();
    broker
        .enqueue(
            &token,
            &Arguments {
                call_id: "pending-once".into(),
                name: "exec".into(),
                input: "{}".into(),
                report: "Original pending action".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(broker.offer(&token).await.unwrap().len(), 1);
    let before = broker.recorded_calls(&token).unwrap();
    drop(broker);
    let restarted = Broker::new(journal.clone());
    assert_eq!(
        restarted.retire(&token, false).await,
        Err(Error("mcp_pending_completion"))
    );
    assert_eq!(restarted.recorded_calls(&token).unwrap(), before);
    restarted.retire(&token, true).await.unwrap();
    assert_eq!(restarted.recorded_calls(&token).unwrap(), before);
    let retired: Value = journal
        .read(&digest(&("mcp-turn-v1", &token)))
        .unwrap()
        .unwrap();
    assert_eq!(retired["retired"], true);
    assert_eq!(
        restarted.catalog(&token).await,
        Err(Error("mcp_turn_unavailable"))
    );
}

#[tokio::test]
async fn restarted_retirement_rejects_unknown_or_invalid_records() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let token = random_id().unwrap();
    assert_eq!(
        broker.retire(&token, true).await,
        Err(Error("mcp_turn_unavailable"))
    );
    let key = digest(&("mcp-turn-v1", &token));
    journal
        .write(&key, &json!({"invalid":"not a turn"}))
        .unwrap();
    assert_eq!(
        broker.retire(&token, true).await,
        Err(Error("invalid_lane_record"))
    );
    assert_eq!(
        journal.read::<Value>(&key).unwrap().unwrap(),
        json!({"invalid":"not a turn"})
    );
}

#[derive(Default)]
struct AppState {
    commands: Vec<Value>,
    lanes: BTreeMap<String, Value>,
    results: Vec<Value>,
    return_only: bool,
}
struct App {
    state: Arc<Mutex<AppState>>,
    address: std::net::SocketAddr,
    native: bool,
    hold: bool,
}
impl Bridge for App {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let state = self.state.clone();
        let address = self.address;
        let native = self.native;
        let hold = self.hold;
        Box::pin(async move {
            if command["op"] == "dispose" {
                assert!(state.lock().unwrap().lanes.is_empty());
                return Ok(json!({"disposed":true}));
            }
            let marker = command["lane"].as_str().unwrap().to_owned();
            let mut data = state.lock().unwrap();
            data.commands.push(command.clone());
            match command["op"].as_str().unwrap() {
                "identity" => Ok(
                    json!({"account":"owned-account","user":"owned-user","structure":"personal","generation":"owned-generation","present":data.lanes.contains_key(&marker)}),
                ),
                "open" => {
                    data.lanes.insert(marker, json!({}));
                    Ok(json!({"localId":"owned-app-conversation"}))
                }
                "submit" => {
                    assert!(command["results"].as_array().unwrap().is_empty());
                    let instruction = command["instructions"]
                        .as_array()
                        .unwrap()
                        .last()
                        .unwrap()
                        .as_str()
                        .unwrap();
                    assert!(!instruction.contains("Codex Native2"));
                    assert!(instruction.contains("My Owned Connector"));
                    let token = instruction
                        .split("Exact turn_token: ")
                        .nth(1)
                        .unwrap()
                        .split('\n')
                        .next()
                        .unwrap()
                        .to_owned();
                    data.lanes.insert(marker.clone(),json!({"round":command["round"],"messages":[],"terminals":[],"roundStarts":1,"userMessages":1,"toolMessages":0,"dispatches":1,"pending":false,"streamActive":false,"approvalPending":false}));
                    if data.return_only {
                        assert!(command.to_string().contains("Native continuation verified"));
                        let lane = data.lanes.get_mut(&marker).unwrap();
                        lane["messages"] = json!([{"id":"returned-final","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Chat continuation verified"]},"status":"finished_successfully","end_turn":true}]);
                        lane["terminals"] = json!(["done"]);
                        return Ok(json!({"started":true}));
                    }
                    if hold {
                        return Ok(json!({"started":true}));
                    }
                    let app = state.clone();
                    tokio::spawn(async move {
                        let catalog = rpc(
                            address,
                            json!(1),
                            "tools/call",
                            json!({"name":"native_catalog","arguments":{"turn_token":token}}),
                        )
                        .await;
                        let tools: Value = serde_json::from_str(
                            catalog["result"]["content"][0]["text"].as_str().unwrap(),
                        )
                        .unwrap();
                        let (exec, patch) = if native {
                            let keys = tools.as_object().unwrap().keys().collect::<Vec<_>>();
                            (
                                keys.iter()
                                    .find(|key| {
                                        matches!(
                                            key.rsplit('.').next(),
                                            Some("exec_command" | "shell_command" | "shell")
                                        )
                                    })
                                    .unwrap()
                                    .as_str(),
                                keys.iter()
                                    .find(|key| key.rsplit('.').next() == Some("apply_patch"))
                                    .unwrap()
                                    .as_str(),
                            )
                        } else {
                            assert!(tools["exec"].is_object() && tools["patch"].is_object());
                            ("exec", "patch")
                        };
                        let command = |script: &str| {
                            if exec.ends_with("exec_command") {
                                json!({"cmd":script,"max_output_tokens":1000})
                            } else if exec.ends_with("shell_command") {
                                json!({"command":script,"timeout_ms":10000,"max_output_chars":1000})
                            } else {
                                json!({"command":["bash","-c",script],"timeout_ms":10000})
                            }
                            .to_string()
                        };
                        let input = if native {
                            command("printf 'one-native-effect\\n' >> effect-count")
                        } else {
                            "{}".into()
                        };
                        let first = effect(address, &token, "first", exec, &input).await;
                        app.lock().unwrap().results.push(first);
                        app.lock().unwrap().lanes.get_mut(&marker).unwrap()["messages"] = json!([{"id":"report-1","author":{"role":"assistant"},"recipient":"all","channel":"commentary","content":{"content_type":"text","parts":["First action completed"]},"status":"finished_successfully","end_turn":false}]);
                        let second = effect(
                            address,
                            &token,
                            "second",
                            patch,
                            if native{"*** Begin Patch\n*** Add File: patched.txt\n+native MCP return\n*** End Patch"}else{"*** exact custom patch ***"},
                        )
                        .await;
                        app.lock().unwrap().results.push(second);
                        if native {
                            let third=effect(address,&token,"verify",exec,&command("test \"$(wc -l < effect-count)\" -eq 1 && grep -qx 'native MCP return' patched.txt && printf verified")).await;
                            app.lock().unwrap().results.push(third);
                        }
                        let mut data = app.lock().unwrap();
                        let lane = data.lanes.get_mut(&marker).unwrap();
                        lane["messages"].as_array_mut().unwrap().push(json!({"id":"final-1","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Implemented and tested"]},"status":"finished_successfully","end_turn":true}));
                        lane["terminals"] = json!(["done"]);
                    });
                    Ok(json!({"started":true}))
                }
                "snapshot" => Ok(data.lanes[&marker].clone()),
                "release" => {
                    data.lanes.remove(&marker);
                    Ok(json!({"released":true}))
                }
                "stop" => {
                    let lane = data.lanes.get_mut(&marker).unwrap();
                    lane["stopped"] = json!(true);
                    Ok(lane.clone())
                }
                _ => Err(Error("unexpected_fixture_operation")),
            }
        })
    }
}
async fn run(host: &AppHost, body: Value) -> (Result<(), Error>, Vec<Event>) {
    let request =
        Request::parse(body, &Config::default(), Some("owned-mcp-native-thread")).unwrap();
    let (sender, mut receiver) = mpsc::channel(64);
    let result = host
        .exchange(request, sender, CancellationToken::new())
        .await;
    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    (result, events)
}
fn native_output(events: &[Event]) -> Vec<Value> {
    let mut projection = Projection::new("owned-response".into(), "chatgpt-desktop/gpt-6".into());
    events
        .iter()
        .flat_map(|event| projection.push(event.clone()).unwrap())
        .find(|v| v["type"] == "response.completed")
        .unwrap()["response"]["output"]
        .as_array()
        .unwrap()
        .clone()
}
#[tokio::test]
async fn actual_host_confirmation_stream_end_keeps_catalog_capability_until_real_completion() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let state = Arc::new(Mutex::new(AppState::default()));
    let bridge = McpBridge::new(
        Arc::new(App {
            state: state.clone(),
            address: "127.0.0.1:9".parse().unwrap(),
            native: false,
            hold: true,
        }),
        broker.clone(),
        journal,
        "My Owned Connector".into(),
    )
    .unwrap();
    bridge
        .operation(json!({"op":"open","lane":"wait","signature":{"name":"gateway"}}))
        .await
        .unwrap();
    bridge.operation(json!({"op":"submit","lane":"wait","round":"round","model":"host","thinkingEffort":null,"messages":[{"role":"user","text":"task"}],"instructions":[],"results":[]})).await.unwrap();
    let token = {
        let mut app = state.lock().unwrap();
        let instruction = app.commands.iter().find(|v| v["op"] == "submit").unwrap()
            ["instructions"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .as_str()
            .unwrap();
        let token = instruction
            .split("Exact turn_token: ")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap()
            .to_owned();
        let lane = app.lanes.get_mut("wait").unwrap();
        lane["messages"] = json!([{"id":"confirmation","author":{"role":"tool"},"metadata":{"jit_plugin_data":{"from_server":{"type":"confirm_action"}}}}]);
        lane["terminals"] = json!(["done"]);
        token
    };
    let blocked = bridge
        .operation(json!({"op":"snapshot","lane":"wait"}))
        .await
        .unwrap();
    assert_eq!(blocked["error"], "mcp_host_confirmation_required");
    broker
        .catalog(&token)
        .await
        .expect("confirmation pause must not retire the actual MCP return path");
    {
        let mut app = state.lock().unwrap();
        let lane = app.lanes.get_mut("wait").unwrap();
        lane["hostResponses"] = json!(["confirmation"]);
        lane["terminals"] = json!([]);
        lane["roundStarts"] = json!(2);
        lane["roundHostReplies"] = json!(1);
    }
    let continued = bridge
        .operation(json!({"op":"snapshot","lane":"wait"}))
        .await
        .unwrap();
    assert!(!continued["error"].is_string());
    broker.catalog(&token).await.unwrap();
    {
        let mut app = state.lock().unwrap();
        let lane = app.lanes.get_mut("wait").unwrap();
        lane["messages"] = json!([{"id":"final","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Finished"]},"status":"finished_successfully","end_turn":true}]);
        lane["terminals"] = json!(["done"]);
    }
    let done = bridge
        .operation(json!({"op":"snapshot","lane":"wait"}))
        .await
        .unwrap();
    assert_eq!(done["roundStarts"], 2);
    assert_eq!(done["roundHostReplies"], 1);
    assert_eq!(
        broker.catalog(&token).await,
        Err(Error("mcp_turn_unavailable"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn two_http_tool_round_trips_share_one_app_submission_and_the_native_transcript() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = broker.clone().router();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    // Discovery remains available without a native capability and cannot grant
    // execution. Honest side-effect hints let the host apply its own policy.
    let discovery = rpc(address, json!("discovery"), "tools/list", json!({})).await;
    let tools = discovery["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["name"], "native_catalog");
    assert_eq!(
        tools[0]["annotations"],
        json!({"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false})
    );
    assert_eq!(tools[1]["name"], "native_tool");
    assert_eq!(
        tools[1]["annotations"],
        json!({"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false})
    );
    for tool in tools {
        let description = tool["description"].as_str().unwrap();
        assert!(!["MUST", "Core owns", "retry"]
            .iter()
            .any(|v| description.contains(v)));
    }
    let refused = rpc(address, json!("unbound"), "tools/call", json!({"name":"native_tool","arguments":{"call_id":"unbound","name":"exec","input":"{}","report":""}})).await;
    assert_eq!(refused["result"]["isError"], true);
    assert!(refused.to_string().contains("mcp_turn_unavailable"));
    let state = Arc::new(Mutex::new(AppState::default()));
    let remote = Arc::new(App {
        state: state.clone(),
        address,
        native: false,
        hold: false,
    });
    let wrapper = Arc::new(
        McpBridge::new(
            remote.clone(),
            broker.clone(),
            journal.clone(),
            "My Owned Connector".into(),
        )
        .unwrap(),
    );
    let mut host = AppHost::new(wrapper, journal.clone());
    let mut body = request();
    let mut observed = Vec::new();
    for index in 0..6 {
        let (result, events) =
            tokio::time::timeout(std::time::Duration::from_secs(10), run(&host, body.clone()))
                .await
                .unwrap();
        result.unwrap();
        let output = native_output(&events);
        observed.extend(output.clone());
        let calls = events
            .iter()
            .filter_map(|v| {
                if let Event::Tool(call) = v {
                    Some(call)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if index % 3 == 2 {
            assert!(calls.is_empty());
            assert!(serde_json::to_string(&output)
                .unwrap()
                .contains("Implemented and tested"));
            if index == 5 {
                break;
            }
            // A replacement transport generation can serve a settled cached
            // answer locally, then imports latest native history at the next
            // user turn. It never revives an old pending MCP capability.
            host = AppHost::new(
                Arc::new(
                    McpBridge::new(
                        remote.clone(),
                        broker.clone(),
                        journal.clone(),
                        "My Owned Connector".into(),
                    )
                    .unwrap(),
                ),
                journal.clone(),
            );
            let cached = run(&host, body.clone()).await;
            cached.0.unwrap();
            assert_eq!(native_output(&cached.1), output);
            body["input"].as_array_mut().unwrap().extend(output);
            body["input"]
                .as_array_mut()
                .unwrap()
                .push(json!({"role":"user","content":"Continue with the next owned task"}));
            continue;
        }
        assert_eq!(calls.len(), 1);
        if index == 0 {
            let replacement = AppHost::new(
                Arc::new(
                    McpBridge::new(
                        remote.clone(),
                        broker.clone(),
                        journal.clone(),
                        "My Owned Connector".into(),
                    )
                    .unwrap(),
                ),
                journal.clone(),
            );
            let refused = run(&replacement, body.clone()).await;
            assert_eq!(refused.0, Err(Error("owned_app_round_unavailable")));
            assert!(!refused.1.iter().any(|v| matches!(v, Event::Tool(_))));
        }
        assert_eq!(
            calls[0].tool.name,
            if index % 3 == 0 { "exec" } else { "patch" }
        );
        if index % 3 == 1 {
            assert_eq!(calls[0].input, "*** exact custom patch ***");
        }
        let repeated = run(&host, body.clone()).await;
        repeated.0.unwrap();
        assert_eq!(
            native_output(&repeated.1),
            output,
            "same Responses retry replays no app/native effect"
        );
        body["input"].as_array_mut().unwrap().extend(output);
        body["input"].as_array_mut().unwrap().push(json!({"type":if index%3==0{"function_call_output"}else{"custom_tool_call_output"},"call_id":calls[0].id,"output":format!("actual result {}",index%3)}));
    }
    let data = state.lock().unwrap();
    assert_eq!(
        data.commands.iter().filter(|v| v["op"] == "submit").count(),
        2
    );
    assert_eq!(data.results.len(), 4);
    for (index, result) in data.results.iter().enumerate() {
        assert_eq!(result["output"], format!("actual result {}", index % 2));
    }
    let text = serde_json::to_string(&observed).unwrap();
    assert_eq!(text.matches("First action completed").count(), 2);
    let lane: Value = journal
        .read(&digest(&"owned-mcp-native-thread"))
        .unwrap()
        .unwrap();
    assert_eq!(lane["phase"], "idle");
    assert_eq!(lane["dispatches"], 1);
    assert_eq!(lane["user_messages"], 1);
    assert_eq!(lane["tool_messages"], 0);
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_mcp_calls_refuse_new_input_policy_and_route_changes_and_stop_only_their_owner() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let state = Arc::new(Mutex::new(AppState::default()));
    let app = Arc::new(App {
        state: state.clone(),
        address: "127.0.0.1:1".parse().unwrap(),
        native: false,
        hold: true,
    });
    let bridge = McpBridge::new(app, broker.clone(), journal, "My Owned Connector".into()).unwrap();
    let catalog=format!("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from native Codex; the app must not execute tools itself.\n{}",serde_json::to_string(&parsed().tools).unwrap());
    let mut tokens = Vec::new();
    let mut waiting = Vec::new();
    let mut offered = Vec::new();
    for lane in ["first-lane", "sibling-lane"] {
        bridge
            .operation(json!({"op":"open","lane":lane,"signature":{"name":"virtual_gateway"}}))
            .await
            .unwrap();
        bridge.operation(json!({"op":"submit","lane":lane,"round":"initial","policyDigest":"native-policy","hostToolsDigest":"same-host-capabilities","model":"owned-model","thinkingEffort":"none","messages":[{"role":"user","text":"Owned task"}],"instructions":[catalog],"results":[]})).await.unwrap();
        let token = state
            .lock()
            .unwrap()
            .commands
            .iter()
            .rev()
            .find(|v| v["op"] == "submit")
            .unwrap()["instructions"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .as_str()
            .unwrap()
            .split("Exact turn_token: ")
            .nth(1)
            .unwrap()
            .split('\n')
            .next()
            .unwrap()
            .to_owned();
        let b = broker.clone();
        let t = token.clone();
        waiting.push(tokio::spawn(async move {
            b.invoke(
                &t,
                Arguments {
                    call_id: "action".into(),
                    name: "exec".into(),
                    input: "{}".into(),
                    report: "Run once".into(),
                },
            )
            .await
        }));
        let snapshot = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let v = bridge
                    .operation(json!({"op":"snapshot","lane":lane}))
                    .await
                    .unwrap();
                if v["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["recipient"] == "local.virtual_gateway")
                {
                    break v;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        offered.push(
            snapshot["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["recipient"] == "local.virtual_gateway")
                .unwrap()["id"]
                .clone(),
        );
        tokens.push(token);
    }
    let returned = json!({"op":"submit","lane":"first-lane","round":"return","policyDigest":"native-policy","hostToolsDigest":"same-host-capabilities","model":"owned-model","thinkingEffort":"none","messages":[],"instructions":[],"results":[{"id":offered[0],"result":{"output":"done"}}]});
    let mut changed = returned.clone();
    changed["messages"] = json!([{"role":"user","text":"new request must wait"}]);
    assert_eq!(
        bridge.admit_submission(changed.clone()).await,
        Err(Error("mcp_pending_new_input"))
    );
    assert_eq!(
        bridge.operation(changed).await,
        Err(Error("mcp_pending_new_input"))
    );
    let mut changed = returned.clone();
    changed["instructions"] = json!(["changed developer policy"]);
    changed["policyDigest"] = json!("changed-native-policy");
    assert_eq!(
        bridge.admit_submission(changed.clone()).await,
        Err(Error("mcp_active_policy_change"))
    );
    assert_eq!(
        bridge.operation(changed).await,
        Err(Error("mcp_active_policy_change"))
    );
    let mut changed = returned.clone();
    changed["model"] = json!("another-model");
    assert_eq!(
        bridge.admit_submission(changed.clone()).await,
        Err(Error("mcp_active_route_change"))
    );
    assert_eq!(
        bridge.operation(changed).await,
        Err(Error("mcp_active_route_change"))
    );
    assert!(waiting.iter().all(|v| !v.is_finished()));
    bridge
        .operation(json!({"op":"stop","lane":"first-lane"}))
        .await
        .unwrap();
    assert_eq!(
        waiting.remove(0).await.unwrap(),
        Err(Error("mcp_turn_unavailable"))
    );
    assert!(!waiting[0].is_finished());
    let mut sibling = returned;
    sibling["instructions"] = json!([
        catalog,
        "Separate host-side capabilities: unchanged allowed search"
    ]);
    bridge.admit_submission(sibling.clone()).await.unwrap();
    sibling["lane"] = json!("sibling-lane");
    sibling["results"][0]["id"] = offered[1].clone();
    bridge.operation(sibling.clone()).await.unwrap();
    bridge.operation(sibling).await.unwrap();
    assert_eq!(
        waiting.remove(0).await.unwrap().unwrap(),
        json!({"output":"done"})
    );
    assert_eq!(
        state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .count(),
        2
    );
    assert_ne!(tokens[0], tokens[1]);
}

#[tokio::test]
async fn catalog_updates_commit_with_results_before_the_model_can_discover_the_next_tool() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal);
    let mut catalog = parsed().tools;
    let token = broker.register("owned", catalog.clone()).await.unwrap();
    let b = broker.clone();
    let t = token.clone();
    let caller = tokio::spawn(async move {
        b.invoke(
            &t,
            Arguments {
                call_id: "search".into(),
                name: "exec".into(),
                input: "{}".into(),
                report: String::new(),
            },
        )
        .await
        .unwrap();
        b.catalog(&t).await.unwrap()
    });
    let call = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let v = broker.offer(&token).await.unwrap();
            if !v.is_empty() {
                break v[0].clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut tool = catalog["exec"].clone();
    tool.name = "discovered".into();
    tool.declaration["name"] = json!("discovered");
    catalog.insert("discovered".into(), tool);
    broker
        .settle_with_catalog(
            &token,
            &[(
                call["id"].as_str().unwrap().into(),
                json!({"output":"discovered"}),
            )],
            Some(catalog.clone()),
        )
        .await
        .unwrap();
    assert_eq!(caller.await.unwrap(), catalog);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_service_shutdown_releases_pending_mcp_http_without_waiting_for_a_model() {
    use codex_chatgpt_provider::server::{self, Service};
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let broker = Broker::new(journal.clone());
    let token = broker
        .register("shutdown-lane", parsed().tools)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Arc::new(App {
        state: Arc::new(Mutex::new(AppState::default())),
        address,
        native: false,
        hold: true,
    });
    let host = Arc::new(AppHost::new(app, journal));
    let service = Service::new(
        Config::default(),
        "owned_server_token_012345678901234567890123456789".into(),
        host,
    )
    .unwrap()
    .with_mcp(broker.clone())
    .unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let server =
        tokio::spawn(async move { server::serve(listener, service, shutdown).await.unwrap() });
    let t = token.clone();
    let caller = tokio::spawn(async move {
        rpc(address,json!(1),"tools/call",json!({"name":"native_tool","arguments":{"turn_token":t,"call_id":"pending","name":"exec","input":"{}","report":"Owned action"}})).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if !broker.offer(&token).await.unwrap().is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    stop.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    let reply = caller.await.unwrap();
    assert_eq!(reply["result"]["isError"], true);
    assert!(reply.to_string().contains("mcp_turn_unavailable"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn actual_native_commands_patch_and_test_return_on_three_original_mcp_http_calls() {
    use codex_chatgpt_provider::{cdp::Process, server::Service};
    use std::os::fd::{AsRawFd, FromRawFd};
    let runtime = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_NATIVE").expect("explicit native runtime required"),
    );
    let prefix = PathBuf::from(std::env::var_os("PREFIX").unwrap());
    let root = Root::new();
    for name in ["home", "codex", "system", "tmp", "work"] {
        std::fs::create_dir(root.0.join(name)).unwrap();
    }
    let journal = Arc::new(Journal::open(&root.0.join("journal")).unwrap());
    let broker = Broker::new(journal.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = Arc::new(Mutex::new(AppState::default()));
    let app = Arc::new(App {
        state: state.clone(),
        address,
        native: true,
        hold: false,
    });
    let bridge = Arc::new(
        McpBridge::new(
            app,
            broker.clone(),
            journal.clone(),
            "My Owned Connector".into(),
        )
        .unwrap(),
    );
    let host = Arc::new(AppHost::new(bridge, journal.clone()));
    let token = random_id().unwrap();
    let config = Config::default();
    journal.write(&digest(&"termux-codex-responses-service-v1"),&json!({"schema":"termux-codex-responses-service-v1","process":Process::read(std::process::id()).unwrap(),"port":address.port(),"token":token})).unwrap();
    codex_chatgpt_provider::registration::prepare(
        &config,
        &root.0.join("journal"),
        &root.0.join("registry"),
        &root.0.join("codex"),
        std::path::Path::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider")),
        &format!("http://{address}/v1"),
    )
    .unwrap();
    let service = Service::new(config, token, host.clone())
        .unwrap()
        .with_mcp(broker)
        .unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let native_requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let received = native_requests.clone();
    let router = service.router().route("/native/responses", axum::routing::post(move |request: axum::extract::Request| {
        let received = received.clone();
        async move {
            use axum::response::IntoResponse;
            let compressed = request.headers().get("content-encoding").is_some_and(|v| v == "zstd");
            let bytes = axum::body::to_bytes(request.into_body(), 32 * 1024 * 1024).await.unwrap();
            let bytes = if compressed { zstd::stream::decode_all(&bytes[..]).unwrap() } else { bytes.to_vec() };
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            received.lock().unwrap().push(body.clone());
            // Independently enforce the native server's observed input contract.
            if body["input"].as_array().unwrap().iter().any(|item| item["call_id"].as_str().is_some_and(|id| id.len() > 64)) {
                return (axum::http::StatusCode::BAD_REQUEST, "call_id exceeds 64 bytes").into_response();
            }
            let item = json!({"id":"native-final","type":"message","role":"assistant","content":[{"type":"output_text","text":"Native continuation verified","annotations":[]}]});
            let response = json!({"id":"native-response","object":"response","model":body["model"],"status":"completed","output":[item],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}});
            let frames = [json!({"type":"response.created","response":{"id":"native-response","status":"in_progress","output":[]}}), json!({"type":"response.output_item.done","output_index":0,"item":item}), json!({"type":"response.completed","response":response})];
            let text = frames.iter().map(|event| format!("event: {}\ndata: {}\n\n", event["type"].as_str().unwrap(), event)).collect::<String>();
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], text).into_response()
        }
    }));
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move { shutdown.cancelled().await })
            .await
            .unwrap()
    });
    let files = [
        std::fs::File::open(prefix.join("etc/resolv.conf")).unwrap(),
        std::fs::File::open(root.0.join("system")).unwrap(),
        std::fs::File::open(root.0.join("tmp")).unwrap(),
    ];
    let files = files.map(|file| {
        let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
        assert!(fd >= 64);
        unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) }
    });
    let fds = files.each_ref().map(AsRawFd::as_raw_fd);
    let native_command = || {
        let mut command = tokio::process::Command::new(&runtime);
        command
            .env_clear()
            .env("PATH", prefix.join("bin"))
            .env("SHELL", prefix.join("bin/bash"))
            .env("PREFIX", &prefix)
            .env("HOME", root.0.join("home"))
            .env("CODEX_HOME", root.0.join("codex"))
            .env("TMPDIR", root.0.join("tmp"))
            .env("TERM", "dumb")
            .current_dir(root.0.join("work"))
            .kill_on_drop(true);
        unsafe {
            command.pre_exec(move || {
                for (source, destination) in fds.into_iter().zip(33..=35) {
                    if libc::dup2(source, destination) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        command
    };
    let mut command = native_command();
    command.args([
        "exec",
        "--json",
        "--skip-git-repo-check",
        "--profile",
        "chatgpt",
        "--dangerously-bypass-approvals-and-sandbox",
        "Perform the owned MCP return qualification",
    ]);
    let output = tokio::time::timeout(std::time::Duration::from_secs(60), command.output())
        .await
        .unwrap()
        .unwrap();
    std::fs::write(root.0.join("chat.stdout"), &output.stdout).unwrap();
    std::fs::write(root.0.join("chat.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "native execution failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Implemented and tested"));
    assert_eq!(
        std::fs::read_to_string(root.0.join("work/effect-count")).unwrap(),
        "one-native-effect\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.0.join("work/patched.txt")).unwrap(),
        "native MCP return\n"
    );
    {
        let data = state.lock().unwrap();
        assert_eq!(
            data.commands.iter().filter(|v| v["op"] == "submit").count(),
            1
        );
        assert_eq!(data.results.len(), 3);
        assert!(data.results[2].to_string().contains("verified"));
    }
    let thread = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|event| event["type"] == "thread.started")
        .unwrap()["thread_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let native = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        native_command()
            .env("OPENAI_API_KEY", "owned-inert-native-fixture")
            .args([
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--dangerously-bypass-approvals-and-sandbox",
                "-m",
                "gpt-6.1-sol",
                "-c",
                "model_provider=\"openai\"",
                "-c",
                &format!("openai_base_url=\"http://{address}/native\""),
                "resume",
                &thread,
                "Continue after the actual Chat tool results",
            ])
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    std::fs::write(root.0.join("native.stdout"), &native.stdout).unwrap();
    std::fs::write(root.0.join("native.stderr"), &native.stderr).unwrap();
    std::fs::write(
        root.0.join("native-request.json"),
        serde_json::to_vec(&*native_requests.lock().unwrap()).unwrap(),
    )
    .unwrap();
    assert!(
        native.status.success(),
        "native return failed: {}\n{}",
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr)
    );
    assert!(String::from_utf8_lossy(&native.stdout).contains("Native continuation verified"));
    {
        let received = native_requests.lock().unwrap();
        let body = received.last().unwrap();
        let input = body["input"].as_array().unwrap();
        let calls = input
            .iter()
            .filter(|v| {
                matches!(
                    v["type"].as_str(),
                    Some("function_call" | "custom_tool_call")
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 3);
        for call in calls {
            assert!(input.iter().any(|result| matches!(
                result["type"].as_str(),
                Some("function_call_output" | "custom_tool_call_output")
            ) && result["call_id"] == call["call_id"]));
        }
    }
    state.lock().unwrap().return_only = true;
    let returned = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        native_command()
            .args([
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--profile",
                "chatgpt",
                "--dangerously-bypass-approvals-and-sandbox",
                "resume",
                &thread,
                "Continue after the native answer without repeating tools",
            ])
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    std::fs::write(root.0.join("chat-return.stdout"), &returned.stdout).unwrap();
    std::fs::write(root.0.join("chat-return.stderr"), &returned.stderr).unwrap();
    assert!(
        returned.status.success(),
        "Chat re-entry failed: {}\n{}",
        String::from_utf8_lossy(&returned.stdout),
        String::from_utf8_lossy(&returned.stderr)
    );
    assert!(String::from_utf8_lossy(&returned.stdout).contains("Chat continuation verified"));
    assert_eq!(
        std::fs::read_to_string(root.0.join("work/effect-count")).unwrap(),
        "one-native-effect\n"
    );
    stop.cancel();
    server.await.unwrap();
    host.shutdown().await;
    // Retain the actual canonical records and wire observation for admission.
    let fixture_root = root.0.clone();
    std::mem::forget(root);
    println!(
        "{}",
        json!({"accepted":true,"fixtureRoot":fixture_root,"actualNativeExecutor":true,"actualMcpHttpReturns":3,"appFixtureSubmissions":2,"chatSubmissions":0,"nativeEffects":3,"sameThreadNativeFollowup":true,"sameThreadChatReturn":true,"publicNativeTranscript":true,"liveConnectorAdmitted":false})
    );
}
