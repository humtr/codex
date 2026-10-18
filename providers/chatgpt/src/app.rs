//! Signed-app boundary. App JS stays a capability/wire shim; Rust controls lane
//! identity, native tool correlation, public progress and uncertain delivery.
use crate::{
    cdp::{App, Client, Descriptor},
    context::{self, Prefix},
    journal::{random_id, Journal},
    protocol::{gateway_calls, Call, Error, Request},
    responses::Event,
    server::Host,
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

pub trait Bridge: Send + Sync + 'static {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>>;
}

struct Connection {
    generation: String,
    app: App,
}
pub struct Remote {
    descriptor: PathBuf,
    owner: String,
    connection: Arc<Mutex<Option<Connection>>>,
}
impl Remote {
    pub fn new(descriptor: PathBuf, owner: String) -> Self {
        Self {
            descriptor,
            owner,
            connection: Arc::new(Mutex::new(None)),
        }
    }
}
impl Bridge for Remote {
    fn operation(&self, mut command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let descriptor = self.descriptor.clone();
        let owner = self.owner.clone();
        let connection = self.connection.clone();
        Box::pin(async move {
            let selected = Descriptor::read(&descriptor)?;
            let generation = selected.generation();
            let mut guard = connection.lock().await;
            if guard.as_ref().is_some_and(|c| c.generation != generation) {
                return Err(Error("owned_host_changed"));
            }
            if guard.is_none() {
                let client = Client::connect(&selected).await?;
                let app = client.app().await?;
                let init = json!({"op":"init","owner":owner,"bindings":{"initial":"app://-/assets/app-initial-ed5807835d97.js","shared":"app://-/assets/app-shared-2d77298f0100.js"}});
                let until = tokio::time::Instant::now() + Duration::from_secs(30);
                loop {
                    let value = app.evaluate(&expression(&init)).await?;
                    if value["ready"] == true {
                        break;
                    }
                    if tokio::time::Instant::now() >= until {
                        return Err(Error("app_scope_unavailable"));
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                *guard = Some(Connection { generation, app });
            }
            let app = guard.as_ref().unwrap().app.clone();
            drop(guard);
            command["owner"] = json!(owner);
            let result = app.evaluate(&expression(&command)).await;
            if result
                .as_ref()
                .is_err_and(|error| error.0.starts_with("cdp_"))
            {
                // Reconnection is read-only until Rust reconciles this round.
                // Never replay an operation whose app-side delivery is unknown.
                connection.lock().await.take();
            }
            result
        })
    }
}
fn expression(command: &Value) -> String {
    format!("({})({})", include_str!("app_bridge.js"), command)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    New,
    Opening,
    Idle,
    Dispatching,
    Running,
    ToolsReady,
    CancelRequested,
    Stopped,
    Unknown,
}
#[derive(Clone, Serialize, Deserialize)]
struct Lane {
    native_thread: String,
    marker: String,
    gateway: String,
    phase: Phase,
    prefix: Prefix,
    request: String,
    round: String,
    pending: Vec<Call>,
    confirmed: BTreeMap<String, String>,
    public: BTreeMap<String, (String, String)>,
    cached: Vec<Event>,
    user_messages: u64,
    tool_messages: u64,
    dispatches: u64,
}
impl Lane {
    fn new(thread: &str) -> Result<Self, Error> {
        Ok(Self {
            native_thread: thread.into(),
            marker: format!("lane_{}", random_id()?),
            gateway: format!("termux_codex_{}", random_id()?),
            phase: Phase::New,
            prefix: Prefix::default(),
            request: String::new(),
            round: String::new(),
            pending: Vec::new(),
            confirmed: BTreeMap::new(),
            public: BTreeMap::new(),
            cached: Vec::new(),
            user_messages: 0,
            tool_messages: 0,
            dispatches: 0,
        })
    }
}

pub struct AppHost {
    bridge: Arc<dyn Bridge>,
    journal: Arc<Journal>,
    lanes: Arc<Mutex<BTreeMap<String, Arc<Mutex<Lane>>>>>,
}
impl AppHost {
    pub fn new(bridge: Arc<dyn Bridge>, journal: Arc<Journal>) -> Self {
        Self {
            bridge,
            journal,
            lanes: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

fn signature(name: &str) -> Value {
    json!({"name":name,"type":"kwargs","description":"Use the advertised native Codex tools. Core approves and executes them. Return exact arguments; never fabricate execution. Independent calls may be batched.","params":[
        {"name":"name","required":false,"type":{"type":"string","description":"Exact catalog key for a single call; omit with calls."}},
        {"name":"input","required":false,"type":{"type":"string","description":"Single-call JSON arguments or exact custom tool input."}},
        {"name":"calls","required":false,"type":{"type":"string","description":"JSON array of 1–32 independent {name,input} objects. Every input is a string. Omit name/input; never batch dependent steps."}},
        {"name":"report","required":false,"type":{"type":"string","description":"Optional useful public initial/intermediate report in the user's language. Never private reasoning."}}
    ]})
}

impl Host for AppHost {
    fn exchange(
        &self,
        request: Request,
        events: mpsc::Sender<Event>,
        cancel: CancellationToken,
    ) -> BoxFuture<'static, Result<(), Error>> {
        let bridge = self.bridge.clone();
        let journal = self.journal.clone();
        let lanes = self.lanes.clone();
        Box::pin(async move {
            let key = context::digest(&request.thread);
            let mutex = {
                let mut lanes = lanes.lock().await;
                if let Some(lane) = lanes.get(&key) {
                    lane.clone()
                } else {
                    let state = journal
                        .read::<Lane>(&key)?
                        .unwrap_or(Lane::new(&request.thread)?);
                    if state.native_thread != request.thread {
                        return Err(Error("native_thread_identity_conflict"));
                    }
                    let lane = Arc::new(Mutex::new(state));
                    lanes.insert(key.clone(), lane.clone());
                    lane
                }
            };
            let mut lane = mutex.try_lock().map_err(|_| Error("native_lane_busy"))?;
            let result = exchange(
                bridge.as_ref(),
                &journal,
                &key,
                &mut lane,
                &request,
                &events,
                &cancel,
            )
            .await;
            if result.is_err()
                && !matches!(
                    lane.phase,
                    Phase::Idle | Phase::ToolsReady | Phase::Stopped | Phase::Unknown
                )
            {
                lane.phase = Phase::Unknown;
                journal.write(&key, &*lane)?;
            }
            result
        })
    }
}

async fn publish(lane: &mut Lane, events: &mpsc::Sender<Event>, event: Event) -> Result<(), Error> {
    lane.cached.push(event.clone());
    events
        .send(event)
        .await
        .map_err(|_| Error("native_observer_closed"))
}
fn public_item(id: &str, phase: &str, text: &str) -> Value {
    json!({"id":id,"type":"message","role":"assistant","phase":phase,"content":[{"type":"output_text","text":text}]})
}

fn native_output(events: &[Event]) -> Result<Vec<Value>, Error> {
    let mut output = Vec::<Value>::new();
    for event in events {
        match event {
            Event::Text { id, phase, delta } => {
                if let Some(previous) = output.iter_mut().find(|v| v["id"] == *id) {
                    let text = previous["content"][0]["text"].as_str().unwrap().to_owned() + delta;
                    previous["content"][0]["text"] = json!(text);
                } else {
                    output.push(public_item(id, phase, delta));
                }
            }
            Event::Tool(call) => output.push(call.output_item()?),
            Event::Completed { .. } => (),
        }
    }
    Ok(output)
}

async fn exchange(
    bridge: &dyn Bridge,
    journal: &Journal,
    key: &str,
    lane: &mut Lane,
    request: &Request,
    events: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<(), Error> {
    if cancel.is_cancelled() {
        return Err(Error("cancelled_before_dispatch"));
    }
    let request_digest = context::digest(request);
    if request_digest == lane.request && matches!(lane.phase, Phase::Idle | Phase::ToolsReady) {
        for event in &lane.cached {
            events
                .send(event.clone())
                .await
                .map_err(|_| Error("native_observer_closed"))?;
        }
        return Ok(());
    }
    if matches!(
        lane.phase,
        Phase::Unknown | Phase::Opening | Phase::CancelRequested | Phase::Stopped
    ) {
        return Err(Error("lane_reconciliation_required"));
    }
    let recovering = matches!(lane.phase, Phase::Dispatching | Phase::Running);
    if recovering && request_digest != lane.request {
        return Err(Error("previous_round_unsettled"));
    }
    if lane.phase == Phase::New {
        lane.phase = Phase::Opening;
        journal.write(key, lane)?;
        if bridge
            .operation(json!({"op":"open","lane":lane.marker,"signature":signature(&lane.gateway)}))
            .await
            .is_err()
        {
            lane.phase = Phase::Unknown;
            journal.write(key, lane)?;
            return Err(Error("app_lane_open_uncertain"));
        }
        lane.phase = Phase::Idle;
        journal.write(key, lane)?;
    }
    if !recovering {
        let delta = lane.prefix.delta(request)?;
        let received = request.results().collect::<BTreeMap<_, _>>();
        let mut results = BTreeMap::<String, Vec<(Call, Value)>>::new();
        for call in &lane.pending {
            let output = received
                .get(call.id.as_str())
                .ok_or(Error("native_tool_result_missing"))?;
            results
                .entry(call.host_id.clone())
                .or_default()
                .push((call.clone(), (*output).clone()));
        }
        for (id, old) in &lane.confirmed {
            if received
                .get(id.as_str())
                .is_some_and(|v| context::digest(*v) != *old)
            {
                return Err(Error("native_tool_result_changed"));
            }
        }
        let pending_ids = lane
            .pending
            .iter()
            .map(|c| c.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let import = delta
            .iter()
            .filter(|v| {
                !v["call_id"]
                    .as_str()
                    .is_some_and(|id| pending_ids.contains(id))
            })
            .cloned()
            .collect::<Vec<_>>();
        let messages = context::messages(&import)?;
        let mut instructions = Vec::<String>::new();
        if lane.prefix.instructions != context::digest(&request.instructions) {
            for value in &request.instructions {
                instructions.push(context::text(value)?);
            }
        }
        if lane.prefix.tools != context::digest(&request.tools) {
            instructions.push(format!("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from Core; the app must not execute tools itself.\n{}",serde_json::to_string(&request.tools).unwrap()));
        }
        let results=results.into_iter().map(|(id,entries)| {
            let outputs=entries.iter().map(|(call,value)|json!({"id":call.id,"name":call.tool.key(),"output":value.get("output").cloned().unwrap_or_else(||value.clone())})).collect::<Vec<_>>();
            json!({"id":id,"result":if outputs.len()==1 {outputs[0].clone()} else {json!({"results":outputs})}})
        }).collect::<Vec<_>>();
        if messages.is_empty() && results.is_empty() {
            return Err(Error("native_request_has_no_new_input"));
        }
        lane.request = request_digest;
        lane.round = random_id()?;
        lane.cached.clear();
        lane.public.clear();
        lane.phase = Phase::Dispatching;
        journal.write(key, lane)?;
        let result=bridge.operation(json!({"op":"submit","lane":lane.marker,"round":lane.round,"model":request.host_model,"thinkingEffort":request.thinking_effort,"messages":messages,"instructions":instructions,"results":results})).await;
        if result.is_err() {
            // Snapshot the same app round below. Do not replay submit after a
            // lost evaluate reply; it may already have sent the model request.
            lane.phase = Phase::Dispatching;
        } else {
            lane.phase = Phase::Running;
        }
        journal.write(key, lane)?;
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(600);
    let mut read_failures = 0;
    loop {
        if cancel.is_cancelled() {
            lane.phase = Phase::CancelRequested;
            journal.write(key, lane)?;
            let _ = bridge
                .operation(json!({"op":"stop","lane":lane.marker}))
                .await;
            let until = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                if let Ok(state) = bridge
                    .operation(json!({"op":"snapshot","lane":lane.marker}))
                    .await
                {
                    if state["stopped"] == true {
                        lane.phase = Phase::Stopped;
                        journal.write(key, lane)?;
                        return Err(Error("app_stop_acknowledged"));
                    }
                }
                if tokio::time::Instant::now() >= until {
                    lane.phase = Phase::Unknown;
                    journal.write(key, lane)?;
                    return Err(Error("app_stop_unconfirmed"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        if tokio::time::Instant::now() >= until {
            lane.phase = Phase::Unknown;
            journal.write(key, lane)?;
            return Err(Error("app_round_timeout_unconfirmed"));
        }
        let state = match bridge
            .operation(json!({"op":"snapshot","lane":lane.marker}))
            .await
        {
            Ok(state) => {
                read_failures = 0;
                state
            }
            Err(_) => {
                read_failures += 1;
                if read_failures >= 3 {
                    lane.phase = Phase::Unknown;
                    journal.write(key, lane)?;
                    return Err(Error("app_observation_lost"));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };
        if state["round"] != lane.round {
            lane.phase = Phase::Unknown;
            journal.write(key, lane)?;
            return Err(Error("app_round_identity_unconfirmed"));
        }
        if state["error"].is_string() {
            lane.phase = Phase::Unknown;
            journal.write(key, lane)?;
            return Err(Error("app_round_failed"));
        }
        let messages = state["messages"]
            .as_array()
            .ok_or(Error("invalid_app_snapshot"))?;
        let mut final_seen = false;
        for message in messages {
            if message["author"]["role"] != "assistant"
                || message["recipient"].as_str().is_some_and(|v| v != "all")
                || message["metadata"]["is_visually_hidden_from_conversation"] == true
            {
                continue;
            }
            if !matches!(
                message["channel"].as_str(),
                None | Some("commentary" | "final")
            ) || message["content"]["content_type"] != "text"
            {
                continue;
            }
            let id = message["id"]
                .as_str()
                .ok_or(Error("invalid_public_message"))?;
            let text = message["content"]["parts"]
                .as_array()
                .ok_or(Error("invalid_public_message"))?
                .iter()
                .filter_map(Value::as_str)
                .collect::<String>();
            let phase = if message["channel"] == "commentary" || message["end_turn"] == false {
                "commentary"
            } else {
                "final_answer"
            };
            let previous = lane
                .public
                .get(id)
                .map(|(phase, text)| (phase.clone(), text.clone()));
            if previous
                .as_ref()
                .is_some_and(|(p, old)| p != phase || !text.starts_with(old))
            {
                return Err(Error("public_message_changed"));
            }
            let offset = previous.as_ref().map_or(0, |(_, text)| text.len());
            if text.len() > offset {
                publish(
                    lane,
                    events,
                    Event::Text {
                        id: id.into(),
                        phase: phase.into(),
                        delta: text[offset..].into(),
                    },
                )
                .await?;
            }
            lane.public.insert(id.into(), (phase.into(), text));
            if phase == "final_answer"
                && message["end_turn"] == true
                && message["status"] == "finished_successfully"
            {
                final_seen = true;
            }
        }
        if !state["terminals"].as_array().is_some_and(|v| v.is_empty()) {
            if state["terminals"] != json!(["done"]) || state["roundStarts"] != 1 {
                lane.phase = Phase::Unknown;
                journal.write(key, lane)?;
                return Err(Error("app_round_incomplete"));
            }
            let mut calls = Vec::new();
            let mut reports = Vec::new();
            for message in messages {
                if message["author"]["role"] != "assistant"
                    || message["recipient"] != format!("local.{}", lane.gateway)
                {
                    continue;
                }
                if message["status"] != "finished_successfully" {
                    return Err(Error("incomplete_host_call"));
                }
                let id = message["id"].as_str().ok_or(Error("invalid_host_call"))?;
                let (batch, report) = gateway_calls(
                    id,
                    message["content"]["text"]
                        .as_str()
                        .ok_or(Error("invalid_host_call"))?,
                    &request.tools,
                )?;
                calls.extend(batch);
                if let Some(report) = report.filter(|r| !r.trim().is_empty()) {
                    reports.push((format!("report_{id}"), report));
                }
            }
            if final_seen && !calls.is_empty() {
                return Err(Error("final_with_pending_calls"));
            }
            if !final_seen && calls.is_empty() {
                return Err(Error("empty_app_round"));
            }
            let ids = calls
                .iter()
                .map(|c| &c.id)
                .collect::<std::collections::BTreeSet<_>>();
            if ids.len() != calls.len() {
                return Err(Error("duplicate_host_call"));
            }
            if calls.iter().any(|call| {
                lane.confirmed.contains_key(&call.id)
                    || lane.pending.iter().any(|old| old.id == call.id)
            }) {
                return Err(Error("host_call_replayed"));
            }
            for (id, report) in reports {
                publish(
                    lane,
                    events,
                    Event::Text {
                        id: id.clone(),
                        phase: "commentary".into(),
                        delta: report.clone(),
                    },
                )
                .await?;
                lane.public.insert(id, ("commentary".into(), report));
            }
            for call in &calls {
                lane.cached.push(Event::Tool(call.clone()));
            }
            for call in &lane.pending {
                let value = request
                    .results()
                    .find(|(id, _)| *id == call.id)
                    .ok_or(Error("native_tool_result_missing"))?
                    .1;
                lane.confirmed
                    .insert(call.id.clone(), context::digest(value));
            }
            lane.pending = calls;
            let output = native_output(&lane.cached)?;
            lane.prefix.accept(request, &output);
            lane.phase = if lane.pending.is_empty() {
                Phase::Idle
            } else {
                Phase::ToolsReady
            };
            lane.user_messages = state["userMessages"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?;
            lane.tool_messages = state["toolMessages"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?;
            lane.dispatches = state["dispatches"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?;
            let completed = Event::Completed {
                input_tokens: (serde_json::to_vec(request).unwrap().len() / 4) as u64,
                output_tokens: (serde_json::to_vec(&output).unwrap().len() / 4) as u64,
            };
            lane.cached.push(completed.clone());
            journal.write(key, lane)?;
            // Commit pending IDs/results before Core can observe an executable call.
            for call in &lane.pending {
                events
                    .send(Event::Tool(call.clone()))
                    .await
                    .map_err(|_| Error("native_observer_closed"))?;
            }
            events
                .send(completed)
                .await
                .map_err(|_| Error("native_observer_closed"))?;
            return Ok(());
        }
        if lane.phase != Phase::Running {
            lane.phase = Phase::Running;
            journal.write(key, lane)?;
        }
        tokio::select! {_=cancel.cancelled()=>(),_=tokio::time::sleep(Duration::from_millis(100))=>()}
    }
}
