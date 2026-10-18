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
    lose_snapshot: usize,
    hold: bool,
    stopped: bool,
    progress_then_loss: bool,
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
                    if state.lose_submit_reply {
                        state.lose_submit_reply = false;
                        Err(Error("cdp_observer_disconnected"))
                    } else {
                        Ok(json!({"started":true}))
                    }
                }
                "release" => {
                    state.lanes.remove(command["lane"].as_str().unwrap());
                    Ok(json!({"released":true}))
                }
                "stop" => {
                    state.stopped = true;
                    Ok(json!({}))
                }
                "snapshot" => {
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
                    let messages = if state.stage == 1 {
                        json!([
                            {"id":"progress-z","author":{"role":"assistant"},"recipient":"all","channel":"commentary","content":{"content_type":"text","parts":["Inspecting files"]},"status":"finished_successfully","end_turn":false},
                            {"id":"host-original","author":{"role":"assistant"},"recipient":format!("local.{}",state.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"calls":json!([{"name":"functions.exec","input":"{\"cmd\":\"pwd\"}"},{"name":"functions.patch","input":"exact freeform patch"}]).to_string(),"report":"Applying the reviewed change"}}).to_string()},"status":"finished_successfully","end_turn":false}
                        ])
                    } else {
                        json!([{"id":"final-a","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Tests passed"]},"status":"finished_successfully","end_turn":true}])
                    };
                    Ok(
                        json!({"round":state.round,"messages":messages,"error":null,"terminals":["done"],"roundStarts":1,"userMessages":1,"toolMessages":state.stage-1,"dispatches":state.stage}),
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
    assert_eq!(submissions[0]["messages"][1]["role"], "user");
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
