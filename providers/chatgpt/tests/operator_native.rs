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
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;
struct Synthetic {
    path: PathBuf,
    commands: Mutex<Vec<Value>>,
    stopped: Mutex<BTreeMap<String, bool>>,
}
impl Bridge for Synthetic {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        self.commands.lock().unwrap().push(command.clone());
        std::fs::write(
            &self.path,
            serde_json::to_vec(&*self.commands.lock().unwrap()).unwrap(),
        )
        .unwrap();
        let lane = command["lane"].as_str().unwrap_or("").to_owned();
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
                    "present":self.commands.lock().unwrap().iter().any(|row|row["op"]=="open" && row["lane"]==lane)})
            }
            "open" => json!({"localId":lane}),
            "submit" => {
                if self.path.parent().unwrap().join("reject-submit").exists() {
                    return Box::pin(async { Err(Error("synthetic_submit_unknown")) });
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
                let submitted = commands
                    .iter()
                    .rev()
                    .find(|row| row["op"] == "submit" && row["lane"] == lane);
                let reviewer = commands
                    .iter()
                    .find(|row| row["op"] == "open" && row["lane"] == lane)
                    .is_some_and(|row| row["signature"]["name"] != "operator_no_local_tools");
                let assessment = json!({"risk_level":"low","user_authorization":"high","outcome":if self.path.parent().unwrap().join("deny-review").exists(){"deny"}else{"allow"},"rationale":"Only this owned disposable fixture program is in scope."});
                let messages = if reviewer {
                    json!([{"id":"owned-review-answer","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":[assessment.to_string()]},"status":"finished_successfully","end_turn":true}])
                } else {
                    json!([])
                };
                json!({"round":submitted.map(|row|&row["round"]),"stopped":self.stopped.lock().unwrap().get(&lane).copied().unwrap_or(false),"messages":messages,"terminals":["done"],"roundStarts":1,"dispatches":1,"userMessages":1,"toolMessages":0})
            }
            "release" => json!({"released":true}),
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
    config.execution.mode = ExecutionMode::Operator;
    config.execution.core = Some(root.join("p/bin/codex"));
    config.execution.home = Some(root.join("h/.codex"));
    config.execution.workspace = Some(root.join("w"));
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
        format!("http://127.0.0.1:{port}/v1"),
        root.join("fixture-auth"),
    )
    .unwrap();
    let bridge = Arc::new(Synthetic {
        path: root.join("bridge-commands.json"),
        commands: Mutex::new(vec![]),
        stopped: Mutex::new(BTreeMap::new()),
    });
    let operator = Operator::new(
        pool,
        Broker::new(journal.clone()),
        journal,
        config.clone(),
        bridge,
        "owned-fixture-bearer-000000000000000000000000".into(),
    )
    .unwrap();
    let service = Service::new(
        config,
        "owned-fixture-bearer-000000000000000000000000".into(),
        operator.clone(),
    )
    .unwrap()
    .with_operator(operator)
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
