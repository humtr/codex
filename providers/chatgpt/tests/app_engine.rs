//! Actual Rust lane engine with app-wire fixtures: original-call returns,
//! warm context, durable response replay and uncertain-delivery reconciliation.
use codex_chatgpt_provider::{
    app::{AppHost, Bridge},
    config::Config,
    journal::{random_id, Journal},
    protocol::{Error, Request},
    responses::{Event, Projection},
    server::Host,
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct State {
    commands: Vec<Value>,
    stage: usize,
    round: String,
    gateway: String,
    lose_submit_reply: bool,
    lose_correction_reply: bool,
    bad_gateway_rounds: usize,
    change_account_on_rejection: bool,
    change_account_before_tools: bool,
    lose_snapshot: usize,
    hold: bool,
    host_block: Option<String>,
    host_replies: u64,
    stopped: bool,
    progress_then_loss: bool,
    public_markup: bool,
    generation: String,
    account: String,
    lanes: std::collections::BTreeSet<String>,
}
#[derive(Clone, Default)]
struct App {
    state: Arc<Mutex<State>>,
}
impl Bridge for App {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let this = self.clone();
        Box::pin(async move {
            let mut state = this.state.lock().unwrap();
            state.commands.push(command.clone());
            match command["op"].as_str().unwrap() {
                "identity" => Ok(
                    json!({"generation":if state.generation.is_empty(){"fixture-generation"}else{&state.generation},"account":if state.account.is_empty(){"fixture-account"}else{&state.account},"user":"fixture-user","structure":"personal","present":state.lanes.contains(command["lane"].as_str().unwrap())}),
                ),
                "open" => {
                    state.lanes.insert(command["lane"].as_str().unwrap().into());
                    state.gateway = command["signature"]["name"].as_str().unwrap().into();
                    Ok(json!({"localId":"owned-app-conversation"}))
                }
                "submit" => {
                    state.stage += 1;
                    state.round = command["round"].as_str().unwrap().into();
                    if state.lose_submit_reply || (state.lose_correction_reply && state.stage > 1) {
                        state.lose_submit_reply = false;
                        state.lose_correction_reply = false;
                        Err(Error("cdp_observer_disconnected"))
                    } else {
                        Ok(json!({"started":true}))
                    }
                }
                "reveal" => Ok(json!({"requested":true})),
                "release" => {
                    state.lanes.remove(command["lane"].as_str().unwrap());
                    Ok(json!({"released":true}))
                }
                "stop" => {
                    state.stopped = true;
                    Ok(json!({}))
                }
                "snapshot" => {
                    if let Some(code) = &state.host_block {
                        return Ok(
                            json!({"round":state.round,"messages":[],"error":code,"terminals":[],"dispatches":1,"userMessages":1,"toolMessages":0,"stopped":state.stopped}),
                        );
                    }
                    if state.lose_snapshot > 0 {
                        state.lose_snapshot -= 1;
                        return Err(Error("cdp_observer_disconnected"));
                    }
                    if state.hold || state.progress_then_loss {
                        if state.progress_then_loss {
                            state.progress_then_loss = false;
                            state.lose_snapshot = 3;
                        }
                        return Ok(
                            json!({"round":state.round,"messages":[{"id":"progress-z","author":{"role":"assistant"},"recipient":"all","channel":"commentary","content":{"content_type":"text","parts":["Inspecting files"]},"end_turn":false}],"terminals":[],"stopped":state.stopped}),
                        );
                    }
                    let mut messages = if state.stage <= state.bad_gateway_rounds {
                        if state.change_account_on_rejection {
                            state.account = "other-account".into();
                        }
                        json!([{ "id":format!("rejected-{}",state.stage),"author":{"role":"assistant"},"recipient":format!("local.{}",state.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"calls":[{"name":"functions.exec","input":"{}"},{"name":"invented.unadvertised","input":"{}"}]}}).to_string()},"status":"finished_successfully","end_turn":false}])
                    } else if state.stage == 1 {
                        json!([
                            {"id":"progress-z","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Inspecting files"]},"status":"finished_partial_completion","end_turn":true},
                            {"id":"host-original","author":{"role":"assistant"},"recipient":format!("local.{}",state.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"calls":json!([{"name":"functions.exec","input":"{\"cmd\":\"pwd\"}"},{"name":"functions.patch","input":"exact freeform patch"}]).to_string(),"report":"Applying the reviewed change"}}).to_string()},"status":"finished_successfully","end_turn":false}
                        ])
                    } else {
                        json!([{"id":"final-a","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Tests passed"]},"status":"finished_successfully","end_turn":true}])
                    };
                    if state.public_markup {
                        messages[0]["content"]["parts"] = json!([if state.stage == 1 {
                            "<box title=\"진행\">Inspecting files<MemoryCite source=\"fixture\" /></box>"
                        } else {
                            "<box title=\"검증\">Tests passed</box>\n`<box>literal code</box>`"
                        }]);
                    }
                    if state.change_account_before_tools {
                        state.account = "other-account".into();
                    }
                    Ok(
                        json!({"round":state.round,"messages":messages,"error":null,"terminals":["done"],"roundStarts":1,"userMessages":1,"toolMessages":state.stage-1,"hostReplyMessages":state.host_replies,"dispatches":state.stage as u64+state.host_replies}),
                    )
                }
                _ => Err(Error("unexpected_fixture_operation")),
            }
        })
    }
}
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("app-engine-{}", random_id().unwrap())))
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn body() -> Value {
    json!({"model":"chatgpt-desktop/gpt-6","stream":true,"client_metadata":{"x-codex-turn-metadata":json!({"thread_id":"native-fixture","turn_id":"turn-1"}).to_string()},"input":[{"role":"user","content":"Improve the copied project"},{"role":"user","content":"<environment_context>owned workspace</environment_context>"}],"tools":[{"type":"namespace","name":"functions","tools":[{"type":"function","name":"exec","parameters":{"type":"object"}},{"type":"custom","name":"patch","format":{"type":"text"}}]}]})
}
async fn execute(host: &AppHost, body: Value) -> Vec<Event> {
    let request = Request::parse(body, &Config::default(), None).unwrap();
    let (events, mut receive) = mpsc::channel(64);
    host.exchange(request, events, CancellationToken::new())
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(event) = receive.recv().await {
        out.push(event);
    }
    out
}

async fn settled_history(host: &AppHost) -> Value {
    let mut body = body();
    let first = execute(host, body.clone()).await;
    let items = body["input"].as_array_mut().unwrap();
    items.extend(output(&first));
    for event in first {
        if let Event::Tool(call) = event {
            items.push(json!({"type":if call.tool.kind==codex_chatgpt_provider::protocol::ToolKind::Custom{"custom_tool_call_output"}else{"function_call_output"},"call_id":call.id,"output":"actual fixture outcome"}));
        }
    }
    let final_events = execute(host, body.clone()).await;
    body["input"]
        .as_array_mut()
        .unwrap()
        .extend(output(&final_events));
    body
}

#[tokio::test]
async fn v2_checkpoint_waits_for_native_history_replacement_then_retires_once() {
    let root = Root::new();
    let app = Arc::new(App::default());
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let host = AppHost::new(app.clone(), journal.clone());
    let history = settled_history(&host).await;
    let mut compact = history.clone();
    compact["input"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"compaction_trigger"}));
    let checkpoint = execute(&host, compact.clone()).await;
    assert!(!checkpoint
        .iter()
        .any(|v| matches!(v, Event::Text { .. } | Event::Tool(_))));
    let result = output(&checkpoint);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["type"], "compaction");
    assert!(result[0]["encrypted_content"]
        .as_str()
        .unwrap()
        .starts_with("ocx1:"));
    assert_eq!(
        output(&execute(&host, compact).await),
        result,
        "repeated checkpoint request is cached without submission"
    );
    {
        let s = app.state.lock().unwrap();
        assert_eq!(s.commands.iter().filter(|v| v["op"] == "open").count(), 1);
        assert_eq!(s.commands.iter().filter(|v| v["op"] == "submit").count(), 3);
        let last = s
            .commands
            .iter()
            .rev()
            .find(|v| v["op"] == "submit")
            .unwrap();
        assert!(last["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["hidden"] == true));
    }
    // Upstream commits replacement, and a new observer loads the private journal.
    drop(host);
    let host = AppHost::new(app.clone(), journal);
    let mut next = history;
    next["input"] = json!([result[0],{"role":"user","content":"Continue the accepted work"}]);
    let events = execute(&host, next.clone()).await;
    next["input"]
        .as_array_mut()
        .unwrap()
        .extend(output(&events));
    next["input"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role":"user","content":"Continue again"}));
    execute(&host, next).await;
    let s = app.state.lock().unwrap();
    assert_eq!(
        s.commands.iter().filter(|v| v["op"] == "release").count(),
        1
    );
    assert_eq!(s.commands.iter().filter(|v| v["op"] == "open").count(), 2);
    let submissions = s
        .commands
        .iter()
        .filter(|v| v["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submissions[4]["messages"].as_array().unwrap().len(), 1);
    assert_eq!(submissions[4]["messages"][0]["text"], "Continue again");
}

#[tokio::test]
async fn uncommitted_summary_remains_transient_and_normal_continuation_stays_warm() {
    for checkpoint in [false, true] {
        let root = Root::new();
        let app = Arc::new(App::default());
        let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
        let mut history = settled_history(&host).await;
        let mut compact = history.clone();
        if checkpoint {
            compact["input"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type":"compaction_trigger"}));
        } else {
            compact["client_metadata"]["x-codex-turn-metadata"]=json!({"thread_id":"native-fixture","turn_id":"turn-1","request_kind":"compaction"}).to_string().into();
            compact["input"].as_array_mut().unwrap().push(
                json!({"role":"user","content":"Transient native summarization instruction"}),
            );
        }
        execute(&host, compact).await;
        history["input"]
            .as_array_mut()
            .unwrap()
            .push(json!({"role":"user","content":"Real next instruction"}));
        execute(&host, history).await;
        let s = app.state.lock().unwrap();
        assert_eq!(s.commands.iter().filter(|v| v["op"] == "open").count(), 1);
        let last = s
            .commands
            .iter()
            .rev()
            .find(|v| v["op"] == "submit")
            .unwrap();
        assert_eq!(last["messages"].as_array().unwrap().len(), 1);
        assert_eq!(last["messages"][0]["text"], "Real next instruction");
        assert!(last["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == codex_chatgpt_provider::context::CONTINUE_INSTRUCTION));
    }
}
fn output(events: &[Event]) -> Vec<Value> {
    let mut projection = Projection::new("response".into(), "configured".into());
    for event in events {
        let frames = projection.push(event.clone()).unwrap();
        if let Some(response) = frames.iter().find(|v| v["type"] == "response.completed") {
            return response["response"]["output"].as_array().unwrap().clone();
        }
    }
    panic!("response did not complete")
}

#[tokio::test]
async fn tool_batch_returns_once_to_original_host_call_without_setup_or_assistant_repetition() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let app = Arc::new(App::default());
    let host = AppHost::new(app.clone(), journal);
    let initial = body();
    let first = execute(&host, initial.clone()).await;
    let calls = first
        .iter()
        .filter_map(|e| {
            if let Event::Tool(call) = e {
                Some(call)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|c| c.host_id == "host-original"));
    let mut second = initial.clone();
    let input = second["input"].as_array_mut().unwrap();
    input.extend(output(&first));
    for call in calls {
        input.push(json!({"type":if call.tool.name=="patch"{"custom_tool_call_output"}else{"function_call_output"},"call_id":call.id,"output":"owned result"}));
    }
    second["reasoning"] = json!({"effort":"xhigh"});
    let last = execute(&host, second.clone()).await;
    assert_eq!(output(&last)[0]["content"][0]["text"], "Tests passed");
    let repeated = execute(&host, second).await;
    assert_eq!(output(&repeated), output(&last));
    let state = app.state.lock().unwrap();
    assert_eq!(
        state.commands.iter().filter(|c| c["op"] == "open").count(),
        1
    );
    let submissions = state
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submissions.len(), 2);
    assert_eq!(submissions[0]["messages"][0]["hidden"], false);
    assert_eq!(submissions[0]["messages"][1]["role"], "developer");
    assert_eq!(submissions[0]["messages"][1]["hidden"], true);
    assert!(submissions[1]["messages"].as_array().unwrap().is_empty());
    assert!(submissions[1]["instructions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(submissions[1]["results"].as_array().unwrap().len(), 1);
    assert_eq!(submissions[1]["results"][0]["id"], "host-original");
    assert_eq!(
        submissions[1]["results"][0]["result"]["results"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(submissions[1]["model"], "gpt-6-thinking");
    assert_eq!(submissions[1]["thinkingEffort"], "max");
}

#[tokio::test]
async fn observer_loss_after_submit_reconciles_same_round_without_resubmission() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let app = Arc::new(App::default());
    {
        let mut state = app.state.lock().unwrap();
        state.lose_submit_reply = true;
        state.lose_snapshot = 1;
    }
    let host = AppHost::new(app.clone(), journal);
    let first = execute(&host, body()).await;
    assert_eq!(
        first.iter().filter(|e| matches!(e, Event::Tool(_))).count(),
        2
    );
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| c["op"] == "submit")
            .count(),
        1
    );
    drop(host);
    let reloaded = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let cached = execute(&reloaded, body()).await;
    assert_eq!(output(&cached), output(&first));
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| c["op"] == "submit")
            .count(),
        1
    );
}

async fn failure(host: &AppHost, body: Value, cancel: CancellationToken) -> (Error, Vec<Event>) {
    let request = Request::parse(body, &Config::default(), None).unwrap();
    let (events, mut receive) = mpsc::channel(64);
    let error = host.exchange(request, events, cancel).await.unwrap_err();
    let mut out = Vec::new();
    while let Some(event) = receive.recv().await {
        out.push(event);
    }
    (error, out)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_confirmation_wait_is_visible_and_resolves_or_cancels_without_resubmission() {
    for cancel_wait in [false, true] {
        let root = Root::new();
        let app = Arc::new(App::default());
        app.state.lock().unwrap().host_block = Some("mcp_host_confirmation_required".into());
        let host = Arc::new(AppHost::new(
            app.clone(),
            Arc::new(Journal::open(&root.0).unwrap()),
        ));
        let cancel = CancellationToken::new();
        let task = {
            let host = host.clone();
            let cancel = cancel.clone();
            tokio::spawn(async move {
                let request = Request::parse(body(), &Config::default(), None).unwrap();
                let (events, mut receive) = mpsc::channel(64);
                let result = host.exchange(request, events, cancel).await;
                let mut collected = Vec::new();
                while let Some(event) = receive.recv().await {
                    collected.push(event);
                }
                (result, collected)
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture")
                    .ok()
                    .is_some_and(|v| {
                        v["host_block"] == "mcp_host_confirmation_required"
                            && v["public_event_count"] == 1
                    })
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let observed =
            codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap();
        assert_eq!(observed["phase"], "running");
        assert_eq!(observed["app_dispatches"], 1);
        assert_eq!(observed["user_role_objects"], 1);
        assert!(!task.is_finished());
        host.reveal("native-fixture".into()).await.unwrap();
        assert!(
            !task.is_finished(),
            "navigation must not lock the running exchange"
        );
        if cancel_wait {
            cancel.cancel();
        } else {
            let mut state = app.state.lock().unwrap();
            state.host_block = None;
            state.host_replies = 1;
        }
        let (result, events) = tokio::time::timeout(std::time::Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        if cancel_wait {
            assert_eq!(result, Err(Error("app_stop_acknowledged")));
        } else {
            assert!(result.is_ok());
            assert!(events.iter().any(|e| matches!(e, Event::Tool(_))));
        }
        assert_eq!(events.iter().filter(|e|matches!(e,Event::Text{delta,..} if delta.contains("waiting for your confirmation"))).count(),1);
        assert_eq!(
            app.state
                .lock()
                .unwrap()
                .commands
                .iter()
                .filter(|c| c["op"] == "submit")
                .count(),
            1
        );
        let observed =
            codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap();
        assert!(observed["host_block"].is_null());
        assert_eq!(observed["app_dispatches"], if cancel_wait { 1 } else { 2 });
        assert_eq!(
            observed["host_action_reply_objects"],
            if cancel_wait { 0 } else { 1 }
        );
        assert_eq!(observed["user_role_objects"], 1);
        assert_eq!(observed["tool_return_objects"], 0);
    }
}

#[tokio::test(start_paused = true)]
async fn confirmed_host_wait_survives_work_deadline_and_still_acknowledges_cancel() {
    let root = Root::new();
    let app = Arc::new(App::default());
    app.state.lock().unwrap().host_block = Some("mcp_host_confirmation_required".into());
    let host = Arc::new(AppHost::new(
        app.clone(),
        Arc::new(Journal::open(&root.0).unwrap()),
    ));
    let cancel = CancellationToken::new();
    let task = {
        let host = host.clone();
        let cancel = cancel.clone();
        tokio::spawn(async move { failure(&host, body(), cancel).await })
    };
    loop {
        if codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture")
            .ok()
            .is_some_and(|v| v["host_block"] == "mcp_host_confirmation_required")
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    tokio::time::advance(std::time::Duration::from_secs(601)).await;
    tokio::task::yield_now().await;
    assert!(
        !task.is_finished(),
        "human wait must not expire the work budget"
    );
    let observed = codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap();
    assert_eq!(observed["phase"], "running");
    assert_eq!(observed["app_dispatches"], 1);
    cancel.cancel();
    let (result, _) = tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, Error("app_stop_acknowledged"));
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .count(),
        1
    );
}

#[tokio::test]
async fn reveal_uses_only_the_bound_native_lane_and_refuses_a_changed_account() {
    let root = Root::new();
    let app = Arc::new(App::default());
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    assert_eq!(
        host.reveal("missing".into()).await,
        Err(Error("native_lane_not_found"))
    );
    let _ = execute(&host, body()).await;
    let submits = app
        .state
        .lock()
        .unwrap()
        .commands
        .iter()
        .filter(|v| v["op"] == "submit")
        .count();
    host.reveal("native-fixture".into()).await.unwrap();
    {
        let mut state = app.state.lock().unwrap();
        let command = state.commands.iter().find(|v| v["op"] == "reveal").unwrap();
        assert_eq!(command["identity"]["account"], "fixture-account");
        assert!(command["lane"].as_str().unwrap().starts_with("lane_"));
        assert_eq!(
            state
                .commands
                .iter()
                .filter(|v| v["op"] == "submit")
                .count(),
            submits
        );
        state.account = "changed-account".into();
    }
    assert_eq!(
        host.reveal("native-fixture".into()).await,
        Err(Error("app_account_changed"))
    );
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|v| v["op"] == "reveal")
            .count(),
        1
    );
}

#[tokio::test]
async fn unknown_host_block_is_a_last_observation_not_a_live_attached_wait() {
    let root = Root::new();
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let app = Arc::new(App::default());
    app.state.lock().unwrap().host_block = Some("fixture_failure".into());
    let host = AppHost::new(app, journal.clone());
    failure(&host, body(), CancellationToken::new()).await;
    let key = codex_chatgpt_provider::context::digest(&"native-fixture");
    let mut record: Value = journal.read(&key).unwrap().unwrap();
    record["host_block"] = json!("mcp_host_confirmation_required");
    journal.write(&key, &record).unwrap();
    let status = codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap();
    assert_eq!(status["phase"], "unknown");
    assert_eq!(status["host_block"], "mcp_host_confirmation_required");
    assert_eq!(
        status["meaning"],
        "remote work state unknown; reconcile before continuation"
    );
}

#[tokio::test]
async fn failed_app_snapshot_preserves_observed_submission_counts() {
    let root = Root::new();
    let app = Arc::new(App::default());
    app.state.lock().unwrap().host_block = Some("fixture_failure".into());
    let host = AppHost::new(app, Arc::new(Journal::open(&root.0).unwrap()));
    assert_eq!(
        failure(&host, body(), CancellationToken::new()).await.0,
        Error("app_round_failed")
    );
    let state = codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap();
    assert_eq!(state["app_dispatches"], 1);
    assert_eq!(state["user_role_objects"], 1);
    assert_eq!(state["phase"], "unknown");
}

#[tokio::test]
async fn stopped_round_requires_a_new_native_turn_and_never_leaks_tools_or_replays_work() {
    let root = Root::new();
    let app = Arc::new(App::default());
    app.state.lock().unwrap().hold = true;
    let host = Arc::new(AppHost::new(
        app.clone(),
        Arc::new(Journal::open(&root.0).unwrap()),
    ));
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let task = {
        let host = host.clone();
        tokio::spawn(async move { failure(&host, body(), token).await })
    };
    loop {
        if app
            .state
            .lock()
            .unwrap()
            .commands
            .iter()
            .any(|c| c["op"] == "snapshot")
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    cancel.cancel();
    let (error, events) = task.await.unwrap();
    assert_eq!(error, Error("app_stop_acknowledged"));
    let status = codex_chatgpt_provider::runtime::thread_status(&root.0, "native-fixture")
        .await
        .unwrap();
    assert_eq!(status["lane"]["phase"], "stopped");
    assert_eq!(status["endpoint_live"], false);
    assert_eq!(status["remote_current_state"], "not_queried");
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Tool(_) | Event::Completed { .. })));
    assert_eq!(
        failure(&host, body(), CancellationToken::new()).await.0,
        Error("previous_round_stopped")
    );
    let mut changed = body();
    changed["reasoning"] = json!({"effort":"high"});
    assert_eq!(
        failure(&host, changed, CancellationToken::new()).await.0,
        Error("new_native_turn_required")
    );
    let mut next = body();
    next["client_metadata"]["x-codex-turn-metadata"] =
        json!({"thread_id":"native-fixture","turn_id":"turn-2"})
            .to_string()
            .into();
    next["input"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role":"user","content":"Continue from the latest native result"}));
    app.state.lock().unwrap().hold = false;
    let result = execute(&host, next).await;
    assert_eq!(output(&result)[0]["content"][0]["text"], "Tests passed");
    let state = app.state.lock().unwrap();
    let submit = state
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submit.len(), 2);
    assert_ne!(submit[0]["lane"], submit[1]["lane"]);
    assert!(submit[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["text"] == "Continue from the latest native result"));
    assert_eq!(
        state
            .commands
            .iter()
            .filter(|c| c["op"] == "release")
            .count(),
        1
    );
}

#[tokio::test]
async fn lost_observer_reloads_partial_transcript_then_reconciles_without_a_second_submission() {
    let root = Root::new();
    let app = Arc::new(App::default());
    app.state.lock().unwrap().progress_then_loss = true;
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let (error, partial) = failure(&host, body(), CancellationToken::new()).await;
    assert_eq!(error, Error("app_observation_lost"));
    assert_eq!(partial.len(), 1);
    drop(host);
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let mut other = body();
    other["reasoning"] = json!({"effort":"high"});
    assert_eq!(
        failure(&host, other, CancellationToken::new()).await.0,
        Error("lane_reconciliation_required")
    );
    let result = execute(&host, body()).await;
    assert_eq!(
        result
            .iter()
            .filter(|e| matches!(e,Event::Text{ id,.. } if id=="progress-z"))
            .count(),
        1
    );
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| c["op"] == "submit")
            .count(),
        1
    );
}

async fn settled_input(host: &AppHost) -> Value {
    let mut second = body();
    let first = execute(host, second.clone()).await;
    second["input"]
        .as_array_mut()
        .unwrap()
        .extend(output(&first));
    for event in &first {
        if let Event::Tool(call) = event {
            second["input"].as_array_mut().unwrap().push(json!({"type":if call.tool.name=="patch"{"custom_tool_call_output"}else{"function_call_output"},"call_id":call.id,"output":"settled native result"}));
        }
    }
    let final_response = execute(host, second.clone()).await;
    second["input"]
        .as_array_mut()
        .unwrap()
        .extend(output(&final_response));
    second
}
fn next_turn(mut input: Value) -> Value {
    input["client_metadata"]["x-codex-turn-metadata"] =
        json!({"thread_id":"native-fixture","turn_id":"turn-2"})
            .to_string()
            .into();
    input["input"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role":"user","content":"Continue from the checked latest native state"}));
    input
}

#[tokio::test]
async fn settled_native_turn_restores_after_host_restart_without_replaying_tools_or_crossing_accounts(
) {
    let root = Root::new();
    let app = Arc::new(App::default());
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let settled = settled_input(&host).await;
    drop(host);
    {
        let mut s = app.state.lock().unwrap();
        s.generation = "replacement-host".into();
        s.lanes.clear();
    }
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let mut setting_only = settled.clone();
    setting_only["reasoning"] = json!({"effort":"high"});
    assert_eq!(
        failure(&host, setting_only, CancellationToken::new())
            .await
            .0,
        Error("owned_app_round_unavailable")
    );
    // A changed app account is never an implicit destination for native history.
    app.state.lock().unwrap().account = "other-account".into();
    assert_eq!(
        failure(&host, next_turn(settled.clone()), CancellationToken::new())
            .await
            .0,
        Error("app_account_changed")
    );
    app.state.lock().unwrap().account.clear();
    let result = execute(&host, next_turn(settled)).await;
    assert_eq!(output(&result)[0]["content"][0]["text"], "Tests passed");
    let s = app.state.lock().unwrap();
    let submits = s
        .commands
        .iter()
        .filter(|v| v["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submits.len(), 3);
    assert_ne!(submits[1]["lane"], submits[2]["lane"]);
    assert!(submits[2]["results"].as_array().unwrap().is_empty());
    assert!(submits[2]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["text"] == "Continue from the checked latest native state"));
}

#[tokio::test]
async fn host_replacement_never_replays_cached_executable_calls_or_pending_results() {
    let root = Root::new();
    let app = Arc::new(App::default());
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    execute(&host, body()).await;
    {
        let mut s = app.state.lock().unwrap();
        s.generation = "replacement-host".into();
        s.lanes.clear();
    }
    assert_eq!(
        failure(&host, body(), CancellationToken::new()).await.0,
        Error("owned_app_round_unavailable")
    );
    assert_eq!(
        failure(&host, next_turn(body()), CancellationToken::new())
            .await
            .0,
        Error("owned_app_round_unavailable")
    );
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .count(),
        1
    );
}

#[tokio::test]
async fn rejected_gateway_batch_returns_once_on_original_id_without_user_messages_or_effects() {
    for (bad, lost_reply) in [(1, false), (1, true), (4, false)] {
        let root = Root::new();
        let app = Arc::new(App::default());
        {
            let mut state = app.state.lock().unwrap();
            state.bad_gateway_rounds = bad;
            state.lose_correction_reply = lost_reply;
        }
        let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
        let (events, mut receive) = mpsc::channel(64);
        let result = host
            .exchange(
                Request::parse(body(), &Config::default(), None).unwrap(),
                events,
                CancellationToken::new(),
            )
            .await;
        if bad == 4 {
            assert_eq!(result.unwrap_err(), Error("gateway_correction_limit"));
        } else {
            result.unwrap();
        }
        let mut emitted = Vec::new();
        while let Some(event) = receive.recv().await {
            emitted.push(event)
        }
        assert!(
            !emitted.iter().any(|v| matches!(v, Event::Tool(_))),
            "a rejected batch must never partly execute its valid first member"
        );
        assert_eq!(
            emitted
                .iter()
                .filter(|v| matches!(v,Event::Text{id,..} if id.starts_with("rejected_")))
                .count(),
            if bad == 4 { 3 } else { 1 }
        );
        if bad == 1 {
            execute(&host, body()).await;
        }
        let commands = app.state.lock().unwrap().commands.clone();
        let submits = commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .collect::<Vec<_>>();
        assert_eq!(
            submits.len(),
            if bad == 4 { 4 } else { 2 },
            "unknown submit delivery is observed, not resent"
        );
        for (index, submit) in submits.iter().enumerate().skip(1) {
            assert!(submit["messages"].as_array().unwrap().is_empty());
            assert!(submit["instructions"].as_array().unwrap().is_empty());
            assert_eq!(submit["results"].as_array().unwrap().len(), 1);
            assert_eq!(submit["results"][0]["id"], format!("rejected-{index}"));
            assert_eq!(
                submit["results"][0]["result"]["error"]["code"],
                "unadvertised_tool"
            );
        }
    }
}

#[tokio::test]
async fn rejected_gateway_does_not_continue_into_a_changed_account() {
    let root = Root::new();
    let app = Arc::new(App::default());
    {
        let mut state = app.state.lock().unwrap();
        state.bad_gateway_rounds = 1;
        state.change_account_on_rejection = true;
    }
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let (events, _) = mpsc::channel(64);
    assert_eq!(
        host.exchange(
            Request::parse(body(), &Config::default(), None).unwrap(),
            events,
            CancellationToken::new()
        )
        .await
        .unwrap_err(),
        Error("app_account_changed")
    );
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .count(),
        1
    );
}

/// Each synthetic app conversation has independent remote work, like the real
/// bridge's lane map. Keep one running while another finishes a native tool batch.
#[derive(Default)]
struct IndependentApps(Mutex<std::collections::BTreeMap<String, Arc<App>>>);
impl Bridge for IndependentApps {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let app = {
            let mut apps = self.0.lock().unwrap();
            let first = apps.is_empty();
            apps.entry(command["lane"].as_str().unwrap().to_owned())
                .or_insert_with(|| {
                    let app = Arc::new(App::default());
                    app.state.lock().unwrap().hold = first;
                    app
                })
                .clone()
        };
        app.operation(command)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_lanes_finish_independently_and_stopping_one_preserves_its_sibling() {
    let root = Root::new();
    let bridge = Arc::new(IndependentApps::default());
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let host = Arc::new(AppHost::new(bridge.clone(), journal));
    let cancel = CancellationToken::new();
    let held = {
        let host = host.clone();
        let token = cancel.clone();
        tokio::spawn(async move { failure(&host, body(), token).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if bridge.0.lock().unwrap().values().any(|app| {
                app.state
                    .lock()
                    .unwrap()
                    .commands
                    .iter()
                    .any(|c| c["op"] == "snapshot")
            }) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        failure(&host, body(), CancellationToken::new()).await.0,
        Error("native_lane_busy")
    );
    let observed = codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap();
    assert_eq!(observed["phase"], "running");
    assert_eq!(observed["app_queried"], false);
    let mut sibling = body();
    sibling["client_metadata"]["x-codex-turn-metadata"] =
        json!({"thread_id":"independent-sibling","turn_id":"sibling-1"})
            .to_string()
            .into();
    let first = execute(&host, sibling.clone()).await;
    sibling["input"]
        .as_array_mut()
        .unwrap()
        .extend(output(&first));
    for event in &first {
        if let Event::Tool(call) = event {
            sibling["input"].as_array_mut().unwrap().push(json!({
                "type":if call.tool.kind==codex_chatgpt_provider::protocol::ToolKind::Custom{"custom_tool_call_output"}else{"function_call_output"},
                "call_id":call.id,"output":"sibling-only-result"}));
        }
    }
    let done = execute(&host, sibling.clone()).await;
    assert_eq!(output(&done)[0]["content"][0]["text"], "Tests passed");
    assert!(
        !held.is_finished(),
        "sibling must complete while parent is still active"
    );
    cancel.cancel();
    let (error, partial) = held.await.unwrap();
    assert_eq!(error, Error("app_stop_acknowledged"));
    assert!(partial
        .iter()
        .all(|v| !matches!(v, Event::Tool(_) | Event::Completed { .. })));
    // Exact retry reads the sibling's accepted response even after parent stop.
    assert_eq!(output(&execute(&host, sibling).await), output(&done));
    let apps = bridge.0.lock().unwrap();
    assert_eq!(apps.len(), 2);
    let states = apps
        .values()
        .map(|app| app.state.lock().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(states.iter().filter(|s| s.stopped).count(), 1);
    let sibling = states.iter().find(|s| !s.hold).unwrap();
    let submissions = sibling
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submissions.len(), 2, "no concurrent/retry work may replay");
    assert_eq!(submissions[1]["results"][0]["id"], "host-original");
    assert!(submissions[1]["results"]
        .to_string()
        .contains("sibling-only-result"));
    assert!(states
        .iter()
        .find(|s| s.hold)
        .unwrap()
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .all(|c| c["results"].as_array().unwrap().is_empty()));
}

#[tokio::test]
async fn thread_status_inspects_pending_work_without_exporting_content_or_starting_a_service() {
    let root = Root::new();
    let app = Arc::new(App::default());
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    execute(&host, body()).await;
    let before = app.state.lock().unwrap().commands.len();
    let status = codex_chatgpt_provider::runtime::thread_status(&root.0, "native-fixture")
        .await
        .unwrap();
    assert_eq!(
        app.state.lock().unwrap().commands.len(),
        before,
        "inspection must not query the app"
    );
    assert_eq!(status["endpoint_live"], false);
    assert_eq!(status["service_started"], false);
    let lane = &status["lane"];
    assert_eq!(lane["phase"], "tools_ready");
    assert_eq!(lane["pending_native_calls"], 2);
    assert_eq!(lane["app_dispatches"], 1);
    assert_eq!(lane["chat_allowance_debits"], "unknown");
    let binary = std::process::Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
        .args(["status", "--state"])
        .arg(&root.0)
        .args(["--thread", "native-fixture"])
        .output()
        .unwrap();
    assert!(
        binary.status.success(),
        "{}",
        String::from_utf8_lossy(&binary.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&binary.stdout).unwrap(),
        status
    );
    assert!(!root.0.join("startup.log").exists());
    let serialized = status.to_string();
    for forbidden in [
        "Improve the copied project",
        "owned workspace",
        "functions.exec",
        "host-original",
        "fixture-account",
        "fixture-user",
        "termux_codex_",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "status leaked private transport context"
        );
    }
    assert_eq!(
        codex_chatgpt_provider::app::inspect_lane(&root.0, "absent"),
        Err(Error("native_lane_not_found"))
    );
    assert_eq!(
        codex_chatgpt_provider::app::inspect_lane(&root.0, "invalid\nthread"),
        Err(Error("invalid_thread_identity"))
    );
    // The on-disk diagnostic must also remain content-free on a tampered error.
    let journal = std::fs::read_dir(&root.0)
        .unwrap()
        .map(|v| v.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .unwrap();
    let mut value: Value = serde_json::from_slice(&std::fs::read(&journal).unwrap()).unwrap();
    value["payload"]["last_error"] = json!("private credential text");
    std::fs::write(&journal, value.to_string()).unwrap();
    assert_eq!(
        codex_chatgpt_provider::app::inspect_lane(&root.0, "native-fixture").unwrap()["last_error"],
        "provider_error"
    );
}

#[tokio::test]
async fn native_guardian_ignores_project_destination_and_refuses_a_project_bound_lane() {
    let root = Root::new();
    let app = Arc::new(App::default());
    let journal = Arc::new(Journal::open(&root.0).unwrap());
    let host = AppHost::new(app.clone(), journal.clone());
    let mut settings = Config::default();
    settings.history.mode = codex_chatgpt_provider::config::HistoryMode::Project;
    settings.history.project_name = Some("Unrelated project instructions".into());
    let mut body = body();
    body["client_metadata"]["x-codex-turn-metadata"] =
        json!({"thread_id":"native-review", "thread_source":"guardian_review"})
            .to_string()
            .into();
    let request = Request::parse(body.clone(), &settings, None).unwrap();
    assert!(request.guardian);
    let (events, _receive) = mpsc::channel(64);
    host.exchange(request, events, CancellationToken::new())
        .await
        .unwrap();
    let before = {
        let state = app.state.lock().unwrap();
        assert!(!state
            .commands
            .iter()
            .any(|c| c["op"].as_str().unwrap().starts_with("project_")));
        assert!(state
            .commands
            .iter()
            .any(|c| c["op"] == "open" && c["project"].is_null()));
        state.commands.len()
    };
    drop(host);
    let key = codex_chatgpt_provider::context::digest(&"native-review");
    let mut record: Value = journal.read(&key).unwrap().unwrap();
    record["project"] = json!("g-p-existing-project");
    journal.write(&key, &record).unwrap();
    let host = AppHost::new(app.clone(), journal);
    assert_eq!(
        failure(&host, body, CancellationToken::new()).await.0,
        Error("reviewer_project_isolation_required")
    );
    assert_eq!(
        app.state.lock().unwrap().commands.len(),
        before,
        "a project-bound reviewer must fail before any app operation"
    );
}

#[tokio::test]
async fn projected_components_keep_native_prefix_and_cached_replies_without_extra_submissions() {
    let root = Root::new();
    let app = Arc::new(App::default());
    app.state.lock().unwrap().public_markup = true;
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let mut next = body();
    let first = execute(&host, next.clone()).await;
    assert_eq!(
        output(&first)[0]["content"][0]["text"],
        "**진행**\nInspecting files\n"
    );
    next["input"].as_array_mut().unwrap().extend(output(&first));
    for event in &first {
        if let Event::Tool(call) = event {
            next["input"].as_array_mut().unwrap().push(json!({
                "type":if call.tool.kind==codex_chatgpt_provider::protocol::ToolKind::Custom{"custom_tool_call_output"}else{"function_call_output"},
                "call_id":call.id,"output":"exact <box>native result</box>"}));
        }
    }
    let final_events = execute(&host, next.clone()).await;
    assert_eq!(
        output(&final_events)[0]["content"][0]["text"],
        "**검증**\nTests passed\n\n`<box>literal code</box>`"
    );
    assert_eq!(output(&execute(&host, next).await), output(&final_events));
    let state = app.state.lock().unwrap();
    let submissions = state
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submissions.len(), 2);
    assert!(submissions[1]["messages"].as_array().unwrap().is_empty());
    assert!(submissions[1]["results"]
        .to_string()
        .contains("exact <box>native result</box>"));
}

#[tokio::test]
async fn account_change_during_model_work_refuses_before_publishing_any_native_call() {
    let root = Root::new();
    let app = Arc::new(App::default());
    app.state.lock().unwrap().change_account_before_tools = true;
    let host = AppHost::new(app.clone(), Arc::new(Journal::open(&root.0).unwrap()));
    let (error, events) = failure(&host, body(), CancellationToken::new()).await;
    assert_eq!(error, Error("app_account_changed"));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Tool(_) | Event::Completed { .. })),
        "an unconfirmed account must not publish executable effects or completion"
    );
    let before = app
        .state
        .lock()
        .unwrap()
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .count();
    assert_eq!(before, 1);
    app.state.lock().unwrap().account = "fixture-account".into();
    app.state.lock().unwrap().change_account_before_tools = false;
    let recovered = execute(&host, body()).await;
    assert_eq!(
        recovered
            .iter()
            .filter(|e| matches!(e, Event::Tool(_)))
            .count(),
        2
    );
    assert_eq!(
        app.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| c["op"] == "submit")
            .count(),
        1,
        "restoring the owned account reconciles the same round, not a new inference"
    );
}
