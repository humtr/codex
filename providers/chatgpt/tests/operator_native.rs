//! An explicitly armed Android synthetic app; never authenticates or submits Chat.
//! The Python acceptance driver owns the disposable signed Core/TUI fixture.
use codex_chatgpt_provider::{
    app::Bridge,
    config::{Config, ExecutionMode, ReturnPath},
    journal::Journal,
    mcp::Broker,
    operator::Operator,
    operator_native::Pool,
    protocol::Error,
    server::{self, Service},
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;
struct Synthetic {
    path: PathBuf,
    commands: Mutex<Vec<Value>>,
    stopped: Mutex<BTreeMap<String, bool>>,
    released: Mutex<BTreeSet<String>>,
    ui_approval_at: Mutex<Option<std::time::Instant>>,
}
fn fixture_base_url(port: u16, requested: Option<String>) -> String {
    let base = requested.unwrap_or_else(|| format!("http://127.0.0.1:{port}/v1"));
    let selected = base
        .strip_prefix("http://127.0.0.1:")
        .and_then(|value| value.strip_suffix("/v1"))
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|value| *value != 0)
        .expect("explicit fixture HTTP proxy must use a loopback port");
    assert_eq!(base, format!("http://127.0.0.1:{selected}/v1"));
    base
}
#[test]
fn fixture_http_proxy_keeps_the_default_or_explicit_loopback_endpoint() {
    assert_eq!(fixture_base_url(3210, None), "http://127.0.0.1:3210/v1");
    assert_eq!(
        fixture_base_url(3210, Some("http://127.0.0.1:4321/v1".into())),
        "http://127.0.0.1:4321/v1"
    );
}
#[test]
fn fixture_http_proxy_rejects_other_destinations() {
    for value in [
        "http://example.com:4321/v1",
        "http://127.0.0.1:0/v1",
        "http://127.0.0.1:4321/v1/other",
        "http://127.0.0.1:04321/v1",
        "http://127.0.0.1:4321@other/v1",
    ] {
        assert!(std::panic::catch_unwind(|| fixture_base_url(3210, Some(value.into()))).is_err());
    }
}
impl Bridge for Synthetic {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        {
            let mut commands = self.commands.lock().unwrap();
            commands.push(command.clone());
            let temporary = self.path.with_extension("tmp");
            std::fs::write(&temporary, serde_json::to_vec(&*commands).unwrap()).unwrap();
            std::fs::rename(temporary, &self.path).unwrap();
        }
        let lane = command["lane"].as_str().unwrap_or("").to_owned();
        if matches!(command["op"].as_str(), Some("stop" | "snapshot"))
            && (self.released.lock().unwrap().contains(&lane)
                || !self
                    .commands
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|row| row["op"] == "open" && row["lane"] == lane))
        {
            return Box::pin(async { Err(Error("app_lane_missing")) });
        }
        if command["op"] == "snapshot"
            && self
                .path
                .parent()
                .unwrap()
                .join("unobservable-host")
                .exists()
        {
            return Box::pin(async { Err(Error("synthetic_host_unobservable")) });
        }

        let value = match command["op"].as_str().unwrap() {
            "identity" => {
                json!({"generation":"fixture","account":if self.path.parent().unwrap().join("changed-account").exists(){"changed-synthetic-account"}else{"owned-synthetic-account"},"user":"owned-synthetic-user","structure":"personal",
                    "present":self.commands.lock().unwrap().iter().any(|row|matches!(row["op"].as_str(),Some("open"|"chat_adopt")) && row["lane"]==lane)})
            }
            "open" | "chat_adopt" => {
                if command["signature"]["name"]
                    .as_str()
                    .is_some_and(|v| v.starts_with("operator_local_"))
                    && (command["signature"]["type"] != "kwargs"
                        || command["signature"]["params"]
                            .as_array()
                            .is_none_or(|params| {
                                params.len() != 2
                                    || params.iter().any(|v| v["type"]["type"] != "string")
                            }))
                {
                    return Box::pin(async { Err(Error("synthetic_app_declaration_invalid")) });
                }
                self.released.lock().unwrap().remove(&lane);
                json!({"localId":lane})
            }
            "host_status" => {
                json!({"saved_read_backoff":if self.path.parent().unwrap().join("source-read-rate-limit").exists(){json!({"cooling_down":true,"retry_after_seconds":3})}else{json!({"cooling_down":false})}})
            }
            "project_read" if command["project"] == "g-p-owned-source-project" => {
                json!({"id":"g-p-owned-source-project","name":"Owned source project","compatible":true})
            }
            "chat_read" => {
                if self
                    .path
                    .parent()
                    .unwrap()
                    .join("source-read-rate-limit")
                    .exists()
                {
                    return Box::pin(async { Err(Error("app_chat_rate_limited")) });
                }
                let fault = self.path.parent().unwrap().join("source-read-timeout-once");
                if fault.exists() {
                    std::fs::rename(&fault, fault.with_extension("observed")).unwrap();
                    return Box::pin(async { Err(Error("cdp_operation_timeout")) });
                }
                let branch_fault = self
                    .path
                    .parent()
                    .unwrap()
                    .join("source-read-branch-conflict");
                if branch_fault.exists() {
                    std::fs::write(
                        branch_fault.with_extension("observed"),
                        b"Original anchor absent from host-selected branch",
                    )
                    .unwrap();
                    return Box::pin(async {
                        Ok(
                            json!({"conversation_id":"owned-saved-chat","current_node":"different-branch", "mapping":{
                                "different-branch":{"parent":null,"message":{"id":"different-branch","author":{"role":"assistant"},"channel":"final","status":"finished_successfully","end_turn":true,"content":{"content_type":"text","parts":["Unrelated original branch"]}}}
                            }}),
                        )
                    });
                }
                let saved = self.path.parent().unwrap().join("saved-source.json");
                if saved.exists() {
                    serde_json::from_slice::<Value>(&std::fs::read(saved).unwrap()).unwrap()
                } else {
                    let commands = self.commands.lock().unwrap();
                    let submitted = commands
                        .iter()
                        .rev()
                        .find(|row| row["op"] == "submit" && row["lane"] == lane)
                        .or_else(|| commands.iter().rev().find(|row| row["op"] == "submit"))
                        .unwrap();
                    let id = format!("owned-adapter-{}", submitted["round"].as_str().unwrap());
                    json!({"conversation_id":"owned-saved-chat","current_node":id,"mapping":{id.clone():{"parent":null,"message":{"id":id,"role":"assistant","channel":"final","status":"finished_successfully","end_turn":true,"content":{"content_type":"text","parts":["OWNED_MANAGED_ROUND_COMPLETE"]}}}}})
                }
            }
            "follow" | "follow_settle" => json!({"observing":true}),
            "submit" => {
                if self.path.parent().unwrap().join("reject-submit").exists() {
                    return Box::pin(async { Err(Error("synthetic_submit_unknown")) });
                }
                if self
                    .path
                    .parent()
                    .unwrap()
                    .join("bound-source-fixture")
                    .exists()
                {
                    // Normal native-origin requests also become saved Chat
                    // history. This producer must advance before a later web
                    // follow-up is published by the external test actor.
                    let saved = self.path.parent().unwrap().join("saved-source.json");
                    let mut source: Value = if saved.exists() {
                        serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap()
                    } else {
                        json!({"conversation_id":"owned-saved-chat","current_node":null,"mapping":{}})
                    };
                    let round = command["round"].as_str().unwrap();
                    let user = format!("owned-native-user-{round}");
                    let answer = format!("owned-adapter-{round}");
                    let text = command["messages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .rev()
                        .find(|message| message["role"] == "user" && message["hidden"] != true)
                        .unwrap()["text"]
                        .clone();
                    let parent = source["current_node"].clone();
                    source["mapping"][&user] = json!({"parent":parent,"message":{"id":user,"role":"user","content":{"content_type":"text","parts":[text]}}});
                    source["mapping"][&answer] = json!({"parent":user,"message":{"id":answer,"role":"assistant","channel":"final","status":"finished_successfully","end_turn":true,"content":{"content_type":"text","parts":["OWNED_MANAGED_ROUND_COMPLETE"]}}});
                    source["current_node"] = json!(answer);
                    let temporary = saved.with_extension("native.tmp");
                    std::fs::write(&temporary, serde_json::to_vec(&source).unwrap()).unwrap();
                    std::fs::rename(temporary, saved).unwrap();
                }
                self.stopped.lock().unwrap().insert(lane, false);
                let hold = self.path.parent().unwrap().join("hold-submit");
                if hold.exists() {
                    return Box::pin(async move {
                        while hold.exists() {
                            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        }
                        Ok(json!({"started":true}))
                    });
                }
                json!({"started":true})
            }
            "stop" => {
                self.stopped.lock().unwrap().insert(lane, true);
                json!({})
            }
            "snapshot" => {
                let commands = self.commands.lock().unwrap();
                let submitted = commands.iter().rev().find(|row| {
                    matches!(row["op"].as_str(), Some("submit" | "follow")) && row["lane"] == lane
                });
                let reviewer = commands
                    .iter()
                    .find(|row| row["op"] == "open" && row["lane"] == lane)
                    .is_some_and(|row| {
                        row["signature"]["name"].as_str().is_some_and(|name| {
                            name != "operator_no_local_tools"
                                && !name.starts_with("operator_local_")
                        })
                    });
                let assessment = json!({"risk_level":"low","user_authorization":"high","outcome":if self.path.parent().unwrap().join("deny-review").exists(){"deny"}else{"allow"},"rationale":"Only this owned disposable fixture program is in scope."});
                let ended = submitted.is_some_and(|row| {
                    row["messages"].as_array().is_some_and(|messages| {
                        messages
                            .iter()
                            .any(|message| message["text"] == "OWNED_ENDED_HOST")
                    })
                });
                let partial = self.path.parent().unwrap().join("partial-host").exists();
                let messages = if self
                    .path
                    .parent()
                    .unwrap()
                    .join("model-adapter-fixture")
                    .exists()
                {
                    submitted.map(|row| json!([{"id":format!("owned-adapter-{}",row["round"].as_str().unwrap()),"author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["OWNED_MANAGED_ROUND_COMPLETE"]},"status":"finished_successfully","end_turn":true}])).unwrap_or_else(|| json!([]))
                } else if reviewer {
                    json!([{"id":"owned-review-answer","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":[assessment.to_string()]},"status":"finished_successfully","end_turn":true}])
                } else if self
                    .path
                    .parent()
                    .unwrap()
                    .join("local-function-fixture")
                    .exists()
                {
                    let offered =
                        std::fs::read(self.path.parent().unwrap().join("local-request.json"))
                            .ok()
                            .and_then(|v| serde_json::from_slice::<Value>(&v).ok());
                    match offered
                        .filter(|v| submitted.is_some_and(|row| row["round"] == v["round"]))
                    {
                        Some(v) => {
                            let gateway = commands
                                .iter()
                                .find(|row| row["op"] == "open" && row["lane"] == lane)
                                .unwrap()["signature"]["name"]
                                .as_str()
                                .unwrap();
                            let input = v["input_override"]
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| v["arguments"].to_string());
                            json!([{"id":v["id"],"author":{"role":"assistant"},"recipient":format!("local.{gateway}"),"channel":"commentary","content":{"content_type":"code","text":json!({"name":v["name"],"input":input}).to_string()},"status":"finished_successfully","end_turn":false}])
                        }
                        None => json!([]),
                    }
                } else if ended {
                    json!([{"id":"owned-incomplete-answer","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Required operator connector is unavailable."]},"status":"finished_successfully","end_turn":true}])
                } else if partial {
                    json!([{"id":"owned-partial-answer","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Tool use follows this partial response."]},"status":"finished_partial_completion","end_turn":true}])
                } else if self
                    .path
                    .parent()
                    .unwrap()
                    .join("host-confirmation")
                    .exists()
                {
                    json!([{"id":"owned-app-permission","author":{"role":"tool"},"metadata":{"jit_plugin_data":{"from_server":{"type":"confirm_action"}}}}])
                } else if self
                    .path
                    .parent()
                    .unwrap()
                    .join("host-authentication")
                    .exists()
                {
                    json!([{"id":"owned-app-auth","author":{"role":"tool"},"metadata":{"jit_plugin_data":{"from_server":{"type":"plugin_auth_required"}}}}])
                } else {
                    json!([])
                };
                json!({"serverId":if self.path.parent().unwrap().join("bound-source-fixture").exists(){json!("owned-saved-chat")}else{Value::Null},"roundStarts":if submitted.is_some_and(|row|row["op"]=="follow"){0}else{1},"round":submitted.map(|row|&row["round"]),"accepted":submitted.is_some(),"pending":self.path.parent().unwrap().join("hold-host-completion").exists(),"streamActive":false,"error":if self.path.parent().unwrap().join("failed-host").exists() && submitted.is_some_and(|row| row["messages"].as_array().is_some_and(|messages| messages.iter().any(|message| message["text"]=="OWNED_FAILED_HOST"))) {json!("app_submit_failed")} else {Value::Null},"stopped":self.stopped.lock().unwrap().get(&lane).copied().unwrap_or(false),"approvalResponses":if self.ui_approval_at.lock().unwrap().is_some_and(|at|at.elapsed()>=std::time::Duration::from_millis(350)){json!(["fixture-confirmation"])}else{json!([])},"messages":messages,"terminals":["done"],"dispatches":1,"userMessages":1,"toolMessages":0})
            }
            "release" => {
                self.released.lock().unwrap().insert(lane);
                json!({"released":true})
            }
            "reveal" => json!({"requested":true}),
            "dispose" => json!({"disposed":true}),
            "approve" => {
                if self
                    .path
                    .parent()
                    .unwrap()
                    .join("approval-ui-required")
                    .exists()
                {
                    return Box::pin(async { Err(Error("app_approval_ui_required")) });
                }
                if self
                    .path
                    .parent()
                    .unwrap()
                    .join("approval-unknown")
                    .exists()
                {
                    return Box::pin(async { Err(Error("app_approval_unconfirmed")) });
                }
                json!({"requested":true,"scope":"conversation","path":"action"})
            }
            "approve_ui" => {
                *self.ui_approval_at.lock().unwrap() = Some(std::time::Instant::now());
                json!({"requested":true,"scope":"conversation","path":"ui","messageId":"fixture-confirmation"})
            }
            _ => return Box::pin(async { Err(Error("unexpected_synthetic_operation")) }),
        };
        Box::pin(async move { Ok(value) })
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires explicit disposable Android signed Core/TUI fixture; zero Chat"]
async fn synthetic_operator_service() {
    let root = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_OPERATOR_FIXTURE").expect("explicit fixture root"),
    );
    assert!(root.is_absolute() && root.join("owned-fixture").is_file());
    let mut config = Config::default();
    config.transport.return_path = ReturnPath::Mcp;
    if root.join("local-function-fixture").exists() {
        config.transport.return_path = ReturnPath::LocalFunction;
    }
    config.execution.mode = if root.join("model-adapter-fixture").exists() {
        ExecutionMode::ModelAdapter
    } else {
        ExecutionMode::Operator
    };
    let mode = config.execution.mode;
    config.execution.core = Some(root.join("p/bin/codex"));
    config.execution.home = Some(root.join("h/.codex"));
    config.execution.workspace = Some(root.join("w"));
    if let Some(path) = std::env::var_os("CODEX_PROVIDER_OPERATOR_CONFIG") {
        config = Config::parse(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            config.execution.home.as_deref(),
            Some(root.join("h/.codex").as_path())
        );
        assert_eq!(
            config.execution.workspace.as_deref(),
            Some(root.join("w").as_path())
        );
        assert_eq!(config.execution.mode, mode);
    }
    if root.join("source-project-fixture").exists() {
        config.history.mode = codex_chatgpt_provider::config::HistoryMode::Project;
        config.history.project_name = Some("Owned source project".into());
        config.history.project_id = Some("g-p-owned-source-project".into());
    }
    config.validate().unwrap();
    let journal = Arc::new(Journal::open(&root.join("provider-state")).unwrap());
    let selected_port = std::env::var("CODEX_PROVIDER_OPERATOR_PORT")
        .ok()
        .map(|v| v.parse::<u16>().unwrap())
        .unwrap_or(0);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", selected_port))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let pool = Pool::new(
        config.clone(),
        root.join("provider-state"),
        fixture_base_url(port, std::env::var("CODEX_PROVIDER_OPERATOR_BASE_URL").ok()),
        root.join("fixture-auth"),
    )
    .unwrap();
    // This explicit fixture also launches a TUI client below. Managed routing
    // itself must not publish user profiles; prepare this disposable client's
    // profile explicitly, just as the installed public prepare command does.
    codex_chatgpt_provider::registration::prepare(
        &config,
        &root.join("provider-state"),
        &root.join("h/.local/share/codex/core/config/profiles"),
        &root.join("h/.codex"),
        &root.join("fixture-auth"),
        &format!("http://127.0.0.1:{port}/v1"),
    )
    .unwrap();
    let bridge = Arc::new(Synthetic {
        path: root.join("bridge-commands.json"),
        commands: Mutex::new(if root.join("bridge-commands.json").exists() {
            serde_json::from_slice(&std::fs::read(root.join("bridge-commands.json")).unwrap())
                .unwrap()
        } else {
            vec![]
        }),
        stopped: Mutex::new(BTreeMap::new()),
        released: Mutex::new(BTreeSet::new()),
        ui_approval_at: Mutex::new(None),
    });
    let operator = Operator::new(
        pool,
        Broker::new(journal.clone()),
        journal.clone(),
        config.clone(),
        bridge,
        "88c34e80b76d5b9289cc2719f6d43fb2d7d33f8c1f680a42".into(),
    )
    .unwrap();
    let service = Service::new(
        config,
        "88c34e80b76d5b9289cc2719f6d43fb2d7d33f8c1f680a42".into(),
        operator.clone(),
    )
    .unwrap()
    .with_operator(operator)
    .unwrap();
    codex_chatgpt_provider::runtime::publish_listener(
        &journal,
        port,
        "88c34e80b76d5b9289cc2719f6d43fb2d7d33f8c1f680a42".into(),
        None,
    )
    .unwrap();
    std::fs::write(
        root.join("provider-ready.json"),
        json!({"port":port}).to_string(),
    )
    .unwrap();
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let marker = root.join("stop-provider");
    tokio::spawn(async move {
        loop {
            if marker.exists() {
                stop.cancel();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });
    server::serve(listener, service, shutdown).await.unwrap();
}
