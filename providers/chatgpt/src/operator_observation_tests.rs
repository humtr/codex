//! Observation regressions with a valid owned process binding. The absent-peer
//! case has no socket; the protocol peer serves only these tests' Native replies.
//! Neither peer performs Native tools or model inference.
use super::*;
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command},
};

fn private_file(path: &Path, bytes: &[u8]) {
    File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

#[test]
#[ignore = "bounded owned subprocess for Native observation regressions"]
fn observation_peer() {
    let root = PathBuf::from(std::env::var_os("CODEX_OBSERVATION_FIXTURE").unwrap());
    assert!(root.join("owned").is_file());
    if root.join("endpoint.json").exists() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(observation_endpoint(&root));
        return;
    }
    // A valid retained process binding is not evidence that the process
    // currently exposes a connectable Native endpoint.
    private_file(&root.join("ready"), b"ready");
    std::thread::sleep(Duration::from_secs(30));
}

async fn observation_endpoint(root: &Path) {
    use futures_util::{SinkExt, StreamExt};
    let endpoint: Value =
        serde_json::from_slice(&fs::read(root.join("endpoint.json")).unwrap()).unwrap();
    let socket = PathBuf::from(endpoint["socket"].as_str().unwrap());
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    private_file(&root.join("ready"), b"ready");
    let _ = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let root = root.to_owned();
            tokio::spawn(async move {
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(message)) = socket.next().await {
                    if message.is_close() { break; }
                    if !message.is_text() { continue; }
                    let row: Value = serde_json::from_str(&message.to_string()).unwrap();
                    let Some(id) = row.get("id") else { continue; };
                    let method = row["method"].as_str().unwrap();
                    let mut log = File::options().append(true).open(root.join("requests.jsonl")).unwrap();
                    writeln!(log, "{row}").unwrap();
                    let mut state: Value = serde_json::from_slice(&fs::read(root.join("state.json")).unwrap()).unwrap();
                    if method == "turn/start" && state["reject_start"] == true {
                        socket.send(json!({"id":id,"error":{"code":-32603,"message":"fixture start rejected before a receipt"}}).to_string().into()).await.unwrap();
                        continue;
                    }
                    let result = match method {
                        "initialize" => json!({}),
                        "thread/loaded/list" => json!({"data":["original-thread"],"nextCursor":null}),
                        "thread/read" => {
                            let mut thread = state["thread"].clone();
                            if row["params"]["includeTurns"] != true { thread["turns"] = json!([]); }
                            json!({"thread":thread})
                        }
                        "thread/backgroundTerminals/list" => json!({"data":state["jobs"],"nextCursor":null}),
                        "thread/items/list" => json!({"data":state["items"],"nextCursor":null}),
                        "turn/start" => {
                            assert_eq!(state["thread"]["status"]["type"], "idle");
                            let input = row["params"]["input"].clone();
                            let turn = json!({"id":"original-turn","status":"inProgress","items":[]});
                            state["thread"]["status"] = json!({"type":"active"});
                            state["thread"]["turns"] = json!([turn]);
                            state["items"] = json!([{"turnId":"original-turn","item":{"id":"original-input","type":"userMessage","content":input}}]);
                            let path = PathBuf::from(state["thread"]["path"].as_str().unwrap());
                            let mut rollout = File::options().append(true).open(path).unwrap();
                            writeln!(rollout, "{}", json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":input[0]["text"]}]}})).unwrap();
                            fs::write(root.join("state.json"), state.to_string()).unwrap();
                            json!({"turn":turn})
                        }
                        _ => panic!("unexpected Native effect or API: {method}"),
                    };
                    socket.send(json!({"id":id,"result":result}).to_string().into()).await.unwrap();
                }
            });
        }
    }).await;
}

struct Fixture {
    root: PathBuf,
    installation: PathBuf,
    home: PathBuf,
    workspace: PathBuf,
    program: PathBuf,
    children: Vec<Child>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("oo.{}", &random_id().unwrap()[..8]));
        let installation = root.join("i");
        let home = root.join("h");
        let workspace = root.join("w");
        let program = installation.join(".local/lib/codex/core/generations/f/runtime");
        for path in [&home, &workspace, program.parent().unwrap()] {
            crate::journal::private_directory(path).unwrap();
        }
        // Reuse this test executable's bytes, but exercise the actual installed
        // runtime pathname and kernel process identity checks.
        let executable = std::env::current_exe().unwrap();
        if fs::hard_link(&executable, &program).is_err() {
            fs::copy(&executable, &program).unwrap();
        }
        Self {
            root,
            installation,
            home,
            workspace,
            program,
            children: vec![],
        }
    }
    async fn peer(&mut self) -> Owner {
        self.peer_in(false).await.0
    }
    async fn peer_in(&mut self, endpoint: bool) -> (Owner, PathBuf) {
        let number = self.children.len();
        let root = self.root.join(format!("peer-{number}"));
        crate::journal::private_directory(&root).unwrap();
        let invocation = codex_runtime_binding::encode_invocation(&[
            "--config".into(),
            "model_provider=\"chatgpt_desktop\"".into(),
        ])
        .unwrap()
        .unwrap();
        let config = format!("# peer {number}\n").into_bytes();
        let name = codex_runtime_binding::configuration_namespace(
            &self.home,
            self.program.as_os_str(),
            None,
            Some(&invocation),
            &config,
            None,
        );
        let directory = self
            .installation
            .join(".local/share/codex/core")
            .join("catalogs")
            .join(name);
        crate::journal::private_directory(&directory.join("config")).unwrap();
        private_file(&root.join("owned"), b"owned absent-endpoint fixture");
        if endpoint {
            let rollout = root.join("rollout-original-thread.jsonl");
            private_file(
                &rollout,
                b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"original-thread\"}}\n",
            );
            private_file(
                &root.join("endpoint.json"),
                json!({"socket":directory.join("s")}).to_string().as_bytes(),
            );
            private_file(&root.join("state.json"), json!({"thread":{"id":"original-thread","cwd":self.workspace,"modelProvider":"chatgpt_desktop","status":{"type":"idle"},"turns":[],"path":rollout},"items":[],"jobs":[]}).to_string().as_bytes());
            private_file(&root.join("requests.jsonl"), b"");
        }
        let child = Command::new(&self.program)
            .args([
                "--ignored",
                "--exact",
                "operator::observation_tests::observation_peer",
            ])
            .env("CODEX_OBSERVATION_FIXTURE", &root)
            .spawn()
            .unwrap();
        let pid = child.id();
        self.children.push(child);
        private_file(
            &directory.join("owner"),
            &codex_runtime_binding::configuration_owner_bytes(
                &self.home,
                self.program.as_os_str(),
                false,
                true,
            ),
        );
        private_file(&directory.join("config/config.toml"), &config);
        private_file(&directory.join("pid"), pid.to_string().as_bytes());
        private_file(&directory.join("invocation"), &invocation);
        tokio::time::timeout(Duration::from_secs(5), async {
            while !root.join("ready").exists() {
                assert!(
                    self.children[number].try_wait().unwrap().is_none(),
                    "owned peer exited before readiness"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        (
            Owner::load(&directory, &self.home, &self.installation).unwrap(),
            root,
        )
    }
    async fn operator(&self, owner: Owner) -> (Arc<Operator>, Arc<Broker>, Record) {
        struct NoopBridge;
        impl crate::app::Bridge for NoopBridge {
            fn operation(&self, _: Value) -> BoxFuture<'static, Result<Value, Error>> {
                Box::pin(async { Err(Error("unexpected_app_bridge_call")) })
            }
        }
        let mut config = Config::default();
        config.execution.core = Some(self.program.clone());
        config.execution.home = Some(self.home.clone());
        config.execution.workspace = Some(self.workspace.clone());
        let state = self.root.join("state");
        let journal = Arc::new(Journal::open(&state).unwrap());
        let broker = Broker::new(journal.clone());
        let tools = BTreeMap::from([(
            "exec_command".into(),
            crate::protocol::Tool {
                name: "exec_command".into(),
                namespace: None,
                kind: crate::protocol::ToolKind::Function,
                declaration: json!({"type":"object"}),
            },
        )]);
        let token = broker
            .register_scoped("observation-work", tools, false, Some(&self.workspace))
            .await
            .unwrap();
        let record = Record {
            schema: WORK_SCHEMA.into(),
            id: "observation-work".into(),
            thread: "original-thread".into(),
            execution: owner,
            origin: "chatgpt".into(),
            native_turn: Some("original-turn".into()),
            generation: 1,
            capability: "a".repeat(48),
            token,
            phase: "waiting_native".into(),
            pending: vec![],
            receipts: BTreeMap::new(),
            summary: None,
            source: None,
            source_context: None,
            accepted: false,
            driver: None,
            driver_phase: "not_started".into(),
            identity: None,
            host_observed_at: None,
            host_last_state: "unknown".into(),
            observed: None,
            observed_context: None,
            policy: None,
            selection: Some(Selection {
                inference: Inference::Attached,
                return_path: ReturnPath::Mcp,
            }),
            workspace: Some(self.workspace.clone()),
            assignment: None,
        };
        journal
            .write(&digest(&("operator-work-v1", &record.id)), &record)
            .unwrap();
        journal
            .write(
                &work_index_key(),
                &WorkIndex {
                    ids: vec![record.id.clone()],
                    pending: None,
                },
            )
            .unwrap();
        let pool = Pool::for_test(
            config.clone(),
            state,
            self.installation.clone(),
            self.home.clone(),
            self.workspace.clone(),
            self.program.clone(),
        );
        let operator = Operator::new(
            pool,
            broker.clone(),
            journal,
            config,
            Arc::new(NoopBridge),
            "fixture-token".into(),
        )
        .unwrap();
        let record = operator.works.lock().await["original-thread"]
            .record
            .clone();
        (operator, broker, record)
    }
}

#[tokio::test]
async fn failed_initial_native_turn_is_observed_before_attach_returns_pending() {
    let mut fixture = Fixture::new();
    let (owner, peer) = fixture.peer_in(true).await;
    let (operator, broker, mut record) = fixture.operator(owner).await;
    {
        let mut works = operator.works.lock().await;
        let work = works.get_mut(&record.thread).unwrap();
        work.record.phase = "waiting_native".into();
        work.record.native_turn = None;
        operator.persist(&work.record).unwrap();
        record = work.record.clone();
    }
    let start = json!({"action":"start","work_id":record.id,"controller":record.capability,
        "task":"Preserve this original task after the initial transport fails.","call_id":"initial-start"});
    let started = operator
        .call("codex_sessions", start.clone())
        .await
        .unwrap();
    assert_eq!(started["native_turn"], "original-turn");
    let args = json!({"work_id":record.id,"controller":record.capability});
    let pending = operator.call("codex_attach", args.clone()).await.unwrap();
    assert_eq!(pending["delivery"], "pending_native_context");
    assert_eq!(pending["accepted"], false);
    assert!(pending["receipt"].is_null());

    let state_path = peer.join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let update = |value: &Value| {
        let next = peer.join("next-state.json");
        private_file(&next, value.to_string().as_bytes());
        fs::rename(next, &state_path).unwrap();
    };
    state["thread"]["status"] = json!({"type":"systemError"});
    state["thread"]["turns"][0]["status"] = json!("failed");
    state["thread"]["turns"][0]["error"] = json!({"message":"original transport failure"});
    state["thread"]["turns"][0]["id"] = json!("different-turn");
    update(&state);
    assert_eq!(
        operator.call("codex_attach", args.clone()).await.unwrap()["delivery"],
        "pending_native_context",
        "another turn is not the admitted start's terminal proof"
    );
    assert_eq!(
        operator.works.lock().await[&record.thread].record.phase,
        "starting_native"
    );

    state["thread"]["turns"][0]["id"] = json!("original-turn");
    update(&state);
    let original_history = fs::read(&state_path).unwrap();
    let rollout = PathBuf::from(state["thread"]["path"].as_str().unwrap());
    let original_rollout = fs::read(&rollout).unwrap();
    let observed = operator.call("codex_attach", args.clone()).await;
    assert_eq!(
        observed,
        Err(Error("operator_reconciliation_required")),
        "an already failed admitted turn must not remain pending forever"
    );
    let current = operator.works.lock().await[&record.thread].record.clone();
    assert_eq!(current.phase, "reconciliation_required");
    assert!(!current.accepted);
    assert_eq!(current.id, record.id);
    assert_eq!(current.thread, record.thread);
    assert_eq!(current.execution, record.execution);
    assert_eq!(current.generation, record.generation);
    assert_eq!(current.capability, record.capability);
    assert_eq!(current.token, record.token);
    assert_eq!(current.native_turn.as_deref(), Some("original-turn"));
    assert!(broker.recorded_calls(&record.token).unwrap().is_empty());
    assert!(broker.unsettled(&record.token).await.unwrap().is_empty());
    assert_eq!(
        broker.catalog(&record.token).await,
        Err(Error("mcp_turn_unavailable")),
        "only the confirmed failed generation is retired"
    );
    assert_eq!(
        operator.call("codex_attach", args.clone()).await,
        Err(Error("operator_reconciliation_required"))
    );
    let mut stale = args.clone();
    stale["controller"] = json!("stale-controller");
    assert_eq!(
        operator.call("codex_attach", stale).await,
        Err(Error("operator_controller_fenced"))
    );
    let status = operator
        .call("codex_status", json!({"work_id":record.id}))
        .await
        .unwrap();
    assert_eq!(status["phase"], "reconciliation_required");
    assert_eq!(status["native_status"]["type"], "systemError");
    assert_eq!(
        operator.call("codex_sessions", start).await.unwrap(),
        started,
        "the exact original start retry returns its receipt without a new turn"
    );
    assert_eq!(fs::read(&state_path).unwrap(), original_history);
    assert_eq!(fs::read(rollout).unwrap(), original_rollout);
    let requests = fs::read_to_string(peer.join("requests.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let starts = requests
        .iter()
        .filter(|row| row["method"] == "turn/start")
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 1);
    assert_eq!(
        starts[0]["params"]["input"][0]["text"],
        "Preserve this original task after the initial transport fails."
    );
    assert!(
        requests.iter().all(|row| matches!(
            row["method"].as_str(),
            Some(
                "initialize"
                    | "thread/loaded/list"
                    | "thread/read"
                    | "thread/items/list"
                    | "thread/backgroundTerminals/list"
                    | "turn/start"
            )
        )),
        "observation must not create, resume, interrupt, or execute another Native operation"
    );
}

#[tokio::test]
async fn unknown_initial_start_cannot_borrow_a_previous_turns_failure() {
    let mut fixture = Fixture::new();
    let (owner, peer) = fixture.peer_in(true).await;
    let (operator, broker, record) = fixture.operator(owner).await;
    {
        let mut works = operator.works.lock().await;
        let work = works.get_mut(&record.thread).unwrap();
        work.record.phase = "waiting_native".into();
        work.record.native_turn = Some("previous-turn".into());
        operator.persist(&work.record).unwrap();
    }
    let state_path = peer.join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["reject_start"] = json!(true);
    state["thread"]["turns"] = json!([{"id":"previous-turn","status":"failed"}]);
    let next = peer.join("next-state.json");
    private_file(&next, state.to_string().as_bytes());
    fs::rename(next, &state_path).unwrap();
    let original_history = fs::read(&state_path).unwrap();
    let result = operator
        .call(
            "codex_sessions",
            json!({"action":"start",
        "work_id":record.id,"controller":record.capability,"task":"New explicit task.",
        "call_id":"unknown-start"}),
        )
        .await;
    assert_eq!(result, Err(Error("native_request_rejected")));
    let current = operator.works.lock().await[&record.thread].record.clone();
    assert_eq!(current.phase, "starting_native");
    assert_eq!(
        current.native_turn, None,
        "without the new receipt the previous Native failure cannot prove this start failed"
    );
    let pending = operator
        .call(
            "codex_attach",
            json!({"work_id":record.id,"controller":record.capability}),
        )
        .await
        .unwrap();
    assert_eq!(pending["delivery"], "pending_native_context");
    assert_eq!(pending["accepted"], false);
    assert!(pending["receipt"].is_null());
    assert_eq!(
        operator.works.lock().await[&record.thread].record.phase,
        "starting_native"
    );
    assert!(broker.catalog(&record.token).await.is_ok());
    assert!(broker.recorded_calls(&record.token).unwrap().is_empty());
    assert_eq!(fs::read(&state_path).unwrap(), original_history);
    assert_eq!(
        fs::read_to_string(peer.join("requests.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|row| row["method"] == "turn/start")
            .count(),
        1
    );
}

#[tokio::test]
async fn retained_offered_result_remains_readable_when_native_socket_is_unavailable() {
    let mut fixture = Fixture::new();
    let owner = fixture.peer().await;
    let (operator, broker, record) = fixture.operator(owner).await;
    let call = crate::mcp::Arguments {
        call_id: "original-call".into(),
        name: "exec_command".into(),
        input: json!({"cmd":"printf original-result","login":false}).to_string(),
        report: "original operation".into(),
    };
    broker.enqueue(&record.token, &call).await.unwrap();
    let offered = broker.offer(&record.token).await.unwrap();
    assert_eq!(offered.len(), 1);
    let native_result = json!({"output":"Process exited with code 0\nOutput:\noriginal-result"});
    let native_input = vec![json!({"type":"message","role":"user",
        "content":[{"type":"input_text","text":"original later input"}]})];
    broker
        .settle_prepared(
            &record.token,
            &[(
                offered[0]["id"].as_str().unwrap().to_owned(),
                native_result.clone(),
            )],
            None,
            &native_input,
            || Ok(()),
        )
        .await
        .unwrap();
    assert!(broker.unsettled(&record.token).await.unwrap().is_empty());
    let result = json!({"result":native_result,"native_input":native_input});
    let call_path = operator
        .journal
        .root
        .join(format!("{}.json", digest(&("mcp-turn-v1", &record.token))));
    let work_path = operator.journal.root.join(format!(
        "{}.json",
        digest(&("operator-work-v1", &record.id))
    ));
    let call_before = fs::read(&call_path).unwrap();
    let work_before = fs::read(&work_path).unwrap();
    let retained = broker
        .invoke(
            &record.token,
            crate::mcp::Arguments {
                call_id: call.call_id.clone(),
                name: call.name.clone(),
                input: call.input.clone(),
                report: call.report.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(retained, result);
    let observer = Broker::new(operator.journal.clone());
    assert_eq!(
        observer.retained_result(&record.token, &call).unwrap(),
        Some(result.clone()),
        "durable result retrieval must not depend on the old in-memory payload cache"
    );
    let records_before = broker.recorded_calls(&record.token).unwrap();
    let args = json!({"work_id":record.id,"controller":record.capability,
        "call_id":call.call_id,"name":call.name,"input":call.input,"report":call.report});
    let retried = operator.call("codex_execute", args.clone()).await;
    assert_eq!(
        broker.recorded_calls(&record.token).unwrap(),
        records_before
    );
    assert!(
        retried.is_ok(),
        "live discovery hid the exact offered and durably settled receipt: {retried:?}"
    );
    let retried = retried.unwrap();
    assert_eq!(retried["result"], result);
    assert_eq!(retried["delivery"], "retrieved_by_original_call");
    assert_eq!(retried["history_observation_available"], false);
    assert_eq!(retried["context_observation_available"], false);
    assert_eq!(fs::read(&call_path).unwrap(), call_before);
    assert_eq!(fs::read(&work_path).unwrap(), work_before);
    for field in [
        "cursor",
        "context_cursor",
        "native_updates",
        "context_updates",
    ] {
        assert!(
            retried.get(field).is_none(),
            "retained result fabricated {field}"
        );
    }

    // The read path must use the original Broker fingerprint, not merely the
    // public call ID, and it must retain current-controller authorization.
    for (field, changed, error) in [
        ("name", "different-tool", "mcp_call_changed"),
        (
            "input",
            "{\"cmd\":\"different-command\"}",
            "mcp_call_changed",
        ),
        ("report", "changed report", "mcp_call_changed"),
        (
            "controller",
            "stale-controller",
            "operator_controller_fenced",
        ),
        ("work_id", "different-work", "operator_work_unavailable"),
        ("call_id", "unknown-call", "native_socket_unavailable"),
    ] {
        let mut changed_args = args.clone();
        changed_args[field] = json!(changed);
        assert_eq!(
            operator.call("codex_execute", changed_args).await,
            Err(Error(error)),
            "changed {field}"
        );
        assert_eq!(
            broker.recorded_calls(&record.token).unwrap(),
            records_before
        );
        assert_eq!(fs::read(&call_path).unwrap(), call_before);
        assert_eq!(fs::read(&work_path).unwrap(), work_before);
    }
    let pending = crate::mcp::Arguments {
        call_id: "pending-call".into(),
        name: call.name.clone(),
        input: call.input.clone(),
        report: call.report.clone(),
    };
    broker.enqueue(&record.token, &pending).await.unwrap();
    let records_pending = broker.recorded_calls(&record.token).unwrap();
    let mut pending_args = args.clone();
    pending_args["call_id"] = json!(pending.call_id);
    assert_eq!(
        operator.call("codex_execute", pending_args).await,
        Err(Error("native_socket_unavailable"))
    );
    assert_eq!(
        broker.recorded_calls(&record.token).unwrap(),
        records_pending
    );

    // A retained result cannot be used to acquire App-owned execution or skip
    // an existing App identity check.
    {
        let mut works = operator.works.lock().await;
        works
            .get_mut(&record.thread)
            .unwrap()
            .record
            .selection
            .as_mut()
            .unwrap()
            .inference = Inference::Submitted;
    }
    assert_eq!(
        operator.call("codex_execute", args.clone()).await,
        Err(Error("app_response_owns_execution"))
    );
    {
        let mut works = operator.works.lock().await;
        let current = &mut works.get_mut(&record.thread).unwrap().record;
        current.selection.as_mut().unwrap().inference = Inference::Attached;
        current.driver = Some("owned-app-lane".into());
    }
    assert_eq!(
        operator.call("codex_execute", args.clone()).await,
        Err(Error("unexpected_app_bridge_call"))
    );
    assert_eq!(
        broker.recorded_calls(&record.token).unwrap(),
        records_pending
    );

    // Actual owner transitions rotate the controller and Broker token. Neither
    // an old capability nor the new generation can revive the old invocation.
    let next_owner = fixture.peer().await;
    broker.retire(&record.token, true).await.unwrap();
    let records_retired = broker.recorded_calls(&record.token).unwrap();
    assert_eq!(
        broker.retained_result(&record.token, &call).unwrap(),
        Some(result)
    );
    let next_token = broker
        .register_scoped(&record.id, BTreeMap::new(), false, Some(&fixture.workspace))
        .await
        .unwrap();
    let next_controller = random_id().unwrap();
    {
        let mut works = operator.works.lock().await;
        let current = &mut works.get_mut(&record.thread).unwrap().record;
        current.driver = None;
        current.execution = next_owner;
        current.generation += 1;
        current.capability = next_controller.clone();
        current.token = next_token.clone();
    }
    assert_eq!(
        operator.call("codex_execute", args.clone()).await,
        Err(Error("operator_controller_fenced"))
    );
    let mut next_args = args;
    next_args["controller"] = json!(next_controller);
    assert_eq!(
        operator.call("codex_execute", next_args).await,
        Err(Error("native_socket_unavailable"))
    );
    assert_eq!(
        broker.recorded_calls(&record.token).unwrap(),
        records_retired
    );
    assert!(broker.recorded_calls(&next_token).unwrap().is_empty());
}
