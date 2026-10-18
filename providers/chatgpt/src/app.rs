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
#[derive(Clone)]
enum Source {
    Relay(PathBuf),
    Pipe(Client),
}
pub struct Remote {
    source: Source,
    owner: String,
    connection: Arc<Mutex<Option<Connection>>>,
}
impl Remote {
    pub fn new(descriptor: PathBuf, owner: String) -> Self {
        Self {
            source: Source::Relay(descriptor),
            owner,
            connection: Arc::new(Mutex::new(None)),
        }
    }
    pub fn pipe(client: Client, owner: String) -> Self {
        Self {
            source: Source::Pipe(client),
            owner,
            connection: Arc::new(Mutex::new(None)),
        }
    }
}
impl Bridge for Remote {
    fn operation(&self, mut command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let source = self.source.clone();
        let owner = self.owner.clone();
        let connection = self.connection.clone();
        Box::pin(async move {
            let selected = match &source {
                Source::Relay(path) => Some(Descriptor::read(path)?),
                Source::Pipe(_) => None,
            };
            let generation = match (&source, &selected) {
                (_, Some(d)) => d.generation(),
                (Source::Pipe(c), _) => c.generation.to_string(),
                _ => unreachable!(),
            };
            let mut guard = connection.lock().await;
            if guard.as_ref().is_some_and(|c| c.generation != generation) {
                // Reattach only to the newly verified owned host. Lane-level
                // identity admission below decides whether a settled turn may
                // import context; pending work remains non-replayable.
                *guard = None;
            }
            if guard.is_none() {
                let client = match (&source, &selected) {
                    (_, Some(d)) => Client::connect(d).await?,
                    (Source::Pipe(c), _) => c.clone(),
                    _ => unreachable!(),
                };
                let until = tokio::time::Instant::now() + Duration::from_secs(30);
                let app = loop {
                    match client.app().await {
                        Ok(app) => break app,
                        Err(Error("app_target_unavailable"))
                            if tokio::time::Instant::now() < until =>
                        {
                            tokio::time::sleep(Duration::from_millis(200)).await
                        }
                        Err(error) => return Err(error),
                    }
                };
                let init = json!({"op":"init","owner":owner,"bindings":{"initial":"app://-/assets/app-initial-ed5807835d97.js","shared":"app://-/assets/app-shared-2d77298f0100.js"}});
                let until = tokio::time::Instant::now() + Duration::from_secs(30);
                loop {
                    let value = evaluate_bridge(&app, &init).await?;
                    if value["ready"] == true {
                        break;
                    }
                    if tokio::time::Instant::now() >= until {
                        return Err(Error("app_scope_unavailable"));
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                *guard = Some(Connection {
                    generation: generation.clone(),
                    app,
                });
            }
            let app = guard.as_ref().unwrap().app.clone();
            drop(guard);
            command["owner"] = json!(owner);
            let mut result = evaluate_bridge(&app, &command).await;
            if command["op"] == "identity" {
                let until = tokio::time::Instant::now() + Duration::from_secs(30);
                while result == Err(Error("app_auth_loading"))
                    && tokio::time::Instant::now() < until
                {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    result = evaluate_bridge(&app, &command).await;
                }
                if let Ok(value) = &mut result {
                    let object = value
                        .as_object_mut()
                        .ok_or(Error("app_identity_unavailable"))?;
                    object.insert("generation".into(), json!(generation));
                }
            }
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
async fn evaluate_bridge(app: &App, command: &Value) -> Result<Value, Error> {
    // Export only our own static bridge codes. Host exception text may include
    // account details, request bodies or credentials and never crosses here.
    let known = [
        "app_account_changed",
        "project_destination_incompatible",
        "app_project_operations_unavailable",
        "invalid_project_identity",
        "project_creation_unconfirmed",
        "project_creation_rejected",
        "invalid_project_list",
        "project_list_bound_exceeded",
        "project_membership_unsettled",
        "project_membership_unconfirmed",
        "app_auth_context_unavailable",
        "app_auth_loading",
        "app_lane_busy",
        "app_bridge_owned",
        "app_local_functions_unavailable",
    ];
    let expression = format!("(async()=>{{const command={command};\ntry{{return {{value:await ({})(command)}};}}catch(e){{return {{error:{}.includes(e.message)?e.message:'app_bridge_operation_failed'}};}}}})()", include_str!("app_bridge.js"), serde_json::to_string(&known).unwrap());
    let result = app.evaluate(&expression).await?;
    if let Some(code) = result["error"].as_str() {
        return Err(Error(
            known
                .into_iter()
                .find(|v| *v == code)
                .unwrap_or("app_bridge_operation_failed"),
        ));
    }
    result
        .get("value")
        .cloned()
        .ok_or(Error("invalid_app_bridge_result"))
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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Identity {
    generation: String,
    account: String,
    user: String,
    structure: String,
}
impl Identity {
    fn parse(value: &Value) -> Result<Self, Error> {
        let identity: Self =
            serde_json::from_value(value.clone()).map_err(|_| Error("app_identity_unavailable"))?;
        if [&identity.generation, &identity.account, &identity.user]
            .iter()
            .any(|v| v.is_empty() || v.len() > 512 || v.chars().any(char::is_control))
            || !matches!(identity.structure.as_str(), "personal" | "workspace")
        {
            return Err(Error("app_identity_unavailable"));
        }
        Ok(identity)
    }
    fn account_matches(&self, other: &Self) -> bool {
        self.account == other.account
            && self.user == other.user
            && self.structure == other.structure
    }
}

fn user_input_identity(request: &Request) -> String {
    context::digest(
        &request
            .input
            .iter()
            .filter(|v| v["role"] == "user")
            .filter_map(|v| context::text(v).ok())
            .filter(|s| !context::internal_visibility_hint(s))
            .collect::<Vec<_>>(),
    )
}
fn new_turn(lane: &Lane, request: &Request) -> bool {
    match (&lane.turn, &request.turn) {
        (Some(old), Some(new)) => old != new,
        _ => lane.user_input != user_input_identity(request),
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Lane {
    native_thread: String,
    marker: String,
    gateway: String,
    phase: Phase,
    prefix: Prefix,
    request: String,
    turn: Option<String>,
    identity: Option<Identity>,
    user_input: String,
    round: String,
    pending: Vec<Call>,
    confirmed: BTreeMap<String, String>,
    public: BTreeMap<String, (String, String)>,
    cached: Vec<Event>,
    user_messages: u64,
    tool_messages: u64,
    dispatches: u64,
    checkpoint: Option<context::Checkpoint>,
    project: Option<String>,
    last_error: Option<String>,
    #[serde(default)]
    rejected: u8,
}

/// Read-only, content-free last observation; not a fresh app progress query.
pub fn inspect_lane(root: &std::path::Path, thread: &str) -> Result<Value, Error> {
    if thread.is_empty() || thread.len() > 256 || thread.chars().any(char::is_control) {
        return Err(Error("invalid_thread_identity"));
    }
    let key = context::digest(&thread);
    let value = Journal::inspect(root, &key)?.ok_or(Error("native_lane_not_found"))?;
    let lane: Lane = serde_json::from_value(value).map_err(|_| Error("invalid_lane_record"))?;
    if lane.native_thread != thread {
        return Err(Error("native_thread_identity_conflict"));
    }
    let meaning = match lane.phase {
        Phase::New | Phase::Opening => "preparing; remote acceptance may be unknown",
        Phase::Idle => "last response settled; native work may continue separately",
        Phase::Dispatching | Phase::Running => "app work was active at last observation",
        Phase::ToolsReady => "native tool calls emitted; awaiting native execution or results",
        Phase::CancelRequested => "stop requested; acknowledgment not confirmed",
        Phase::Stopped => "app acknowledged stop; native effects require native history review",
        Phase::Unknown => "remote work state unknown; reconcile before continuation",
    };
    Ok(
        json!({"schema":"termux-codex-provider-lane-status-v1","thread":thread,
        "phase":lane.phase,"meaning":meaning,"observation":"last_private_journal",
        "pending_native_calls":lane.pending.len(),"public_event_count":lane.cached.len(),
        "app_dispatches":lane.dispatches,"user_role_objects":lane.user_messages,
        "tool_return_objects":lane.tool_messages,"last_error":lane.last_error.as_deref()
            .map(|code| if code.len()<=128 && code.bytes().all(|c|c.is_ascii_lowercase() || c.is_ascii_digit() || c==b'_') {code}else{"provider_error"}),
        "chat_allowance_debits":"unknown","app_queried":false}),
    )
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
            turn: None,
            identity: None,
            user_input: String::new(),
            round: String::new(),
            pending: Vec::new(),
            confirmed: BTreeMap::new(),
            public: BTreeMap::new(),
            cached: Vec::new(),
            user_messages: 0,
            tool_messages: 0,
            dispatches: 0,
            checkpoint: None,
            project: None,
            last_error: None,
            rejected: 0,
        })
    }
}

pub struct AppHost {
    bridge: Arc<dyn Bridge>,
    journal: Arc<Journal>,
    lanes: Arc<Mutex<BTreeMap<String, Arc<Mutex<Lane>>>>>,
    project_lock: Arc<Mutex<()>>,
}
impl AppHost {
    pub fn new(bridge: Arc<dyn Bridge>, journal: Arc<Journal>) -> Self {
        Self {
            bridge,
            journal,
            lanes: Arc::new(Mutex::new(BTreeMap::new())),
            project_lock: Arc::new(Mutex::new(())),
        }
    }
    pub async fn shutdown(&self) {
        let lanes = self
            .lanes
            .lock()
            .await
            .iter()
            .map(|(key, lane)| (key.clone(), lane.clone()))
            .collect::<Vec<_>>();
        let mut stops = tokio::task::JoinSet::new();
        for (key, mutex) in lanes {
            let bridge = self.bridge.clone();
            let journal = self.journal.clone();
            stops.spawn(async move {
                let mut lane = mutex.lock().await;
                if matches!(
                    lane.phase,
                    Phase::Dispatching | Phase::Running | Phase::Unknown | Phase::CancelRequested
                ) {
                    let _ = stop_lane(bridge.as_ref(), &journal, &key, &mut lane).await;
                }
            });
        }
        while stops.join_next().await.is_some() {}
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
        let project_lock = self.project_lock.clone();
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
                LaneServices {
                    bridge: bridge.as_ref(),
                    journal: &journal,
                    project_lock: &project_lock,
                },
                &key,
                &mut lane,
                &request,
                &events,
                &cancel,
            )
            .await;
            if let Err(error) = &result {
                lane.last_error = Some(error.0.into());
                journal.write(&key, &*lane)?;
            }
            if result.is_err()
                && cancel.is_cancelled()
                && matches!(lane.phase, Phase::Dispatching | Phase::Running)
            {
                return stop_lane(bridge.as_ref(), &journal, &key, &mut lane).await;
            }
            if result.is_err()
                && !matches!(
                    lane.phase,
                    Phase::Idle
                        | Phase::ToolsReady
                        | Phase::Stopped
                        | Phase::Unknown
                        | Phase::Opening
                        | Phase::New
                        | Phase::CancelRequested
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
                    previous["phase"] = json!(phase);
                } else {
                    output.push(public_item(id, phase, delta));
                }
            }
            Event::Tool(call) => output.push(call.output_item()?),
            Event::Checkpoint {
                id,
                encrypted_content,
            } => output
                .push(json!({"id":id,"type":"compaction","encrypted_content":encrypted_content})),
            Event::Completed { .. } => (),
        }
    }
    Ok(output)
}

struct LaneServices<'a> {
    bridge: &'a dyn Bridge,
    journal: &'a Journal,
    project_lock: &'a Mutex<()>,
}
async fn exchange(
    services: LaneServices<'_>,
    key: &str,
    lane: &mut Lane,
    request: &Request,
    events: &mpsc::Sender<Event>,
    cancel: &CancellationToken,
) -> Result<(), Error> {
    let LaneServices {
        bridge,
        journal,
        project_lock,
    } = services;
    if cancel.is_cancelled() {
        return Err(Error("cancelled_before_dispatch"));
    }
    let request_digest = context::digest(request);
    if request_digest == lane.request && lane.phase == Phase::Idle {
        for event in &lane.cached {
            events
                .send(event.clone())
                .await
                .map_err(|_| Error("native_observer_closed"))?;
        }
        return Ok(());
    }
    if lane.phase == Phase::CancelRequested {
        return stop_lane(bridge, journal, key, lane).await;
    }
    let observed = bridge
        .operation(json!({"op":"identity","lane":lane.marker}))
        .await?;
    let current = Identity::parse(&observed)?;
    if let Some(old) = &lane.identity {
        if !old.account_matches(&current) {
            return Err(Error("app_account_changed"));
        }
        if old.generation != current.generation || observed["present"] != true {
            if matches!(lane.phase, Phase::Idle | Phase::Stopped) && new_turn(lane, request) {
                // Re-import only at a new native user turn after a settled round.
                // Old remote chats remain intact. No pending call is reissued.
                *lane = Lane::new(&request.thread)?;
            } else if !matches!(lane.phase, Phase::New | Phase::Opening) {
                return Err(Error("owned_app_round_unavailable"));
            }
        }
    } else if lane.phase != Phase::New && lane.phase != Phase::Opening {
        return Err(Error("app_identity_unavailable"));
    }
    lane.identity = Some(current);
    journal.write(key, lane)?;
    if request_digest == lane.request && lane.phase == Phase::ToolsReady {
        for event in &lane.cached {
            events
                .send(event.clone())
                .await
                .map_err(|_| Error("native_observer_closed"))?;
        }
        return Ok(());
    }

    let readable_handoff = lane.phase == Phase::Idle
        && new_turn(lane, request)
        && crate::handoff::latest(&request.input)?.is_some()
        && lane.prefix.delta(request).map_or(true, |delta| {
            crate::handoff::latest(delta).is_ok_and(|v| v.is_some())
        });
    if lane.phase == Phase::Idle
        && (readable_handoff
            || lane
                .checkpoint
                .as_ref()
                .is_some_and(|v| v.acknowledged(request)))
    {
        bridge
            .operation(json!({"op":"release","lane":lane.marker}))
            .await?;
        let identity = lane.identity.clone();
        *lane = Lane::new(&request.thread)?;
        lane.identity = identity;
        journal.write(key, lane)?;
    }

    if lane.phase == Phase::Stopped {
        if request_digest == lane.request {
            return Err(Error("previous_round_stopped"));
        }
        if !new_turn(lane, request) {
            return Err(Error("new_native_turn_required"));
        }
        // A new native turn owns its next submission. Preserve the old remote
        // conversation, but rebuild from latest canonical native input after an
        // acknowledged stop rather than guessing its partly streamed context.
        bridge
            .operation(json!({"op":"release","lane":lane.marker}))
            .await?;
        let identity = lane.identity.clone();
        *lane = Lane::new(&request.thread)?;
        lane.identity = identity;
        journal.write(key, lane)?;
    }
    if lane.phase == Phase::Opening {
        // Opening allocates no inference. Reconcile the same marker and exact
        // declaration; never create a successor because its reply was lost.
        bridge
            .operation(json!({"op":"open","lane":lane.marker,"signature":signature(&lane.gateway),"project":lane.project}))
            .await?;
        lane.phase = Phase::Idle;
        journal.write(key, lane)?;
    }
    if lane.phase == Phase::Unknown && request_digest != lane.request {
        return Err(Error("lane_reconciliation_required"));
    }
    let recovering = matches!(
        lane.phase,
        Phase::Dispatching | Phase::Running | Phase::Unknown
    );
    if recovering && request_digest != lane.request {
        return Err(Error("previous_round_unsettled"));
    }
    if lane.phase == Phase::New {
        let _guard = project_lock.lock().await;
        lane.project =
            crate::project::resolve(bridge, journal, &request.history, &observed).await?;
        lane.phase = Phase::Opening;
        journal.write(key, lane)?;
        if bridge
            .operation(json!({"op":"open","lane":lane.marker,"signature":signature(&lane.gateway),"project":lane.project}))
            .await
            .is_err()
        {
            journal.write(key, lane)?;
            return Err(Error("app_lane_open_uncertain"));
        }
        lane.phase = Phase::Idle;
        journal.write(key, lane)?;
    }
    if recovering {
        // The response observer is new; restore already observed public deltas
        // before app snapshots supply only their unseen suffixes.
        for event in &lane.cached {
            events
                .send(event.clone())
                .await
                .map_err(|_| Error("native_observer_closed"))?;
        }
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
        let mut messages = context::messages(&import)?;
        let mut instructions = Vec::<String>::new();
        if lane.prefix.instructions != context::digest(&request.instructions) {
            for value in &request.instructions {
                instructions.push(context::text(value)?);
            }
        }
        if lane.prefix.tools != context::digest(&(&request.tools, &request.host_tools)) {
            instructions.push(format!("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from Core; the app must not execute tools itself.\n{}",serde_json::to_string(&request.tools).unwrap()));
        }
        if !request.host_tools.is_empty()
            && lane.prefix.tools != context::digest(&(&request.tools, &request.host_tools))
        {
            instructions.push(format!("Separate host-side capabilities: {}. These are app-owned search capabilities, not Core function names; do not send them through the local gateway. Keep public findings distinct from native command execution.",serde_json::to_string(&request.host_tools).unwrap()));
        }
        if let Some(mode) = request.compaction {
            if mode == context::Compaction::Checkpoint {
                messages.push(context::Message {
                    role: "user".into(),
                    text: context::SUMMARY_INSTRUCTION.into(),
                    hidden: true,
                });
            }
            for message in &mut messages {
                message.hidden = true;
            }
            instructions.push(context::SUMMARY_INSTRUCTION.into());
        } else if lane.checkpoint.is_some() && lane.prefix.instructions.is_empty() {
            instructions.push(context::CONTINUE_INSTRUCTION.into());
        }
        let results=results.into_iter().map(|(id,entries)| {
            let outputs=entries.iter().map(|(call,value)|json!({"id":call.id,"name":call.tool.key(),"output":value.get("output").cloned().unwrap_or_else(||value.clone())})).collect::<Vec<_>>();
            json!({"id":id,"result":if outputs.len()==1 {outputs[0].clone()} else {json!({"results":outputs})}})
        }).collect::<Vec<_>>();
        if messages.is_empty() && results.is_empty() {
            return Err(Error("native_request_has_no_new_input"));
        }
        lane.request = request_digest;
        lane.turn = request.turn.clone();
        lane.user_input = user_input_identity(request);
        lane.round = random_id()?;
        lane.cached.clear();
        lane.public.clear();
        lane.rejected = 0;
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
            return stop_lane(bridge, journal, key, lane).await;
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
            // The app also labels interim reports `final`, sometimes ending
            // them with end_turn=true/finished_partial_completion before a tool.
            // Only a successfully committed end-of-turn is a native final answer.
            let phase = if message["channel"] != "commentary"
                && message["end_turn"] == true
                && message["status"] == "finished_successfully"
            {
                "final_answer"
            } else {
                "commentary"
            };
            let previous = lane
                .public
                .get(id)
                .map(|(phase, text)| (phase.clone(), text.clone()));
            if previous.as_ref().is_some_and(|(p, old)| {
                (p != phase && !(p == "commentary" && phase == "final_answer"))
                    || !text.starts_with(old)
            }) {
                return Err(Error("public_message_changed"));
            }
            let offset = previous.as_ref().map_or(0, |(_, text)| text.len());
            lane.public.insert(id.into(), (phase.into(), text.clone()));
            let promoted = previous.as_ref().is_some_and(|(p, _)| p != phase);
            if (text.len() > offset || promoted)
                && request.compaction != Some(context::Compaction::Checkpoint)
            {
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
            lane.user_messages = state["userMessages"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?;
            lane.tool_messages = state["toolMessages"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?;
            lane.dispatches = state["dispatches"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?;
            let mut calls = Vec::new();
            let mut reports = Vec::new();
            let mut host_calls = Vec::new();
            let mut rejected = None;
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
                host_calls.push(id);
                let parsed = gateway_calls(
                    id,
                    message["content"]["text"]
                        .as_str()
                        .ok_or(Error("invalid_host_call"))?,
                    &request.tools,
                );
                let (batch, report) = match parsed {
                    Ok(value) => value,
                    Err(error)
                        if matches!(
                            error.0,
                            "unadvertised_tool"
                                | "invalid_gateway_arguments"
                                | "invalid_gateway_batch"
                                | "invalid_gateway_report"
                                | "invalid_tool_search_arguments"
                        ) =>
                    {
                        rejected = Some(error);
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                calls.extend(batch);
                if let Some(report) = report.filter(|r| !r.trim().is_empty()) {
                    reports.push((format!("report_{id}"), report));
                }
            }
            if let Some(error) = rejected {
                if request.compaction.is_some() || final_seen || host_calls.len() != 1 {
                    return Err(error);
                }
                if lane.rejected >= 3 {
                    return Err(Error("gateway_correction_limit"));
                }
                let observed = bridge
                    .operation(json!({"op":"identity","lane":lane.marker}))
                    .await?;
                let current = Identity::parse(&observed)?;
                let same_account = lane
                    .identity
                    .as_ref()
                    .is_some_and(|old| old.account_matches(&current));
                if !same_account {
                    return Err(Error("app_account_changed"));
                }
                if lane.identity.as_ref() != Some(&current) || observed["present"] != true {
                    return Err(Error("owned_app_round_unavailable"));
                }
                let id = host_calls[0];
                if lane.confirmed.contains_key(id) || lane.pending.iter().any(|v| v.id == id) {
                    return Err(Error("host_call_replayed"));
                }
                let result = json!({"error":{"code":error.0,"message":"The gateway request was rejected before execution. Use exact keys from the Native tool catalog; use tool_search to discover deferred tools. No part of this rejected batch executed."}});
                lane.confirmed.insert(id.into(), context::digest(&result));
                lane.rejected += 1;
                let report=Event::Text{id:format!("rejected_{id}"),phase:"commentary".into(),delta:format!("Provider rejected a tool request before execution: {}. Returning the error for correction.\n",error.0)};
                lane.cached.push(report.clone());
                lane.round = random_id()?;
                lane.phase = Phase::Dispatching;
                journal.write(key, lane)?;
                events
                    .send(report)
                    .await
                    .map_err(|_| Error("native_observer_closed"))?;
                let submitted=bridge.operation(json!({"op":"submit","lane":lane.marker,"round":lane.round,"model":request.host_model,"thinkingEffort":request.thinking_effort,"messages":[],"instructions":[],"results":[{"id":id,"result":result}]})).await;
                if submitted.is_ok() {
                    lane.phase = Phase::Running
                }
                journal.write(key, lane)?;
                continue;
            }
            if final_seen && !calls.is_empty() {
                return Err(Error("final_with_pending_calls"));
            }
            if request.compaction.is_some() && !calls.is_empty() {
                return Err(Error("compaction_must_not_call_tools"));
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
            let report_events = reports
                .into_iter()
                .map(|(id, report)| {
                    lane.public
                        .insert(id.clone(), ("commentary".into(), report.clone()));
                    let event = Event::Text {
                        id,
                        phase: "commentary".into(),
                        delta: report,
                    };
                    lane.cached.push(event.clone());
                    event
                })
                .collect::<Vec<_>>();
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
            let checkpoint_event = if request.compaction.is_some() {
                let summary = messages
                    .iter()
                    .rev()
                    .find_map(|v| {
                        let id = v["id"].as_str()?;
                        lane.public
                            .get(id)
                            .filter(|(phase, _)| phase == "final_answer")
                            .map(|(_, text)| text.clone())
                    })
                    .ok_or(Error("empty_compaction_summary"))?;
                let checkpoint = context::Checkpoint::offer(request, &summary)?;
                let event =
                    (request.compaction == Some(context::Compaction::Checkpoint)).then(|| {
                        Event::Checkpoint {
                            id: format!("checkpoint_{}", lane.round),
                            encrypted_content: checkpoint.encoded().into(),
                        }
                    });
                if let Some(event) = &event {
                    lane.cached.push(event.clone());
                }
                lane.checkpoint = Some(checkpoint);
                lane.prefix.accept_compaction(request);
                event
            } else {
                let output = native_output(&lane.cached)?;
                lane.prefix.accept(request, &output);
                None
            };
            let output = native_output(&lane.cached)?;
            lane.phase = if lane.pending.is_empty() {
                Phase::Idle
            } else {
                Phase::ToolsReady
            };
            let completed = Event::Completed {
                input_tokens: (serde_json::to_vec(request).unwrap().len() / 4) as u64,
                output_tokens: (serde_json::to_vec(&output).unwrap().len() / 4) as u64,
            };
            lane.cached.push(completed.clone());
            journal.write(key, lane)?;
            // Commit terminal reports and pending IDs/results before publishing.
            // A disconnected observer must not duplicate cached report text on retry.
            for event in report_events {
                events
                    .send(event)
                    .await
                    .map_err(|_| Error("native_observer_closed"))?;
            }
            // Commit pending IDs/results before Core can observe an executable call.
            for call in &lane.pending {
                events
                    .send(Event::Tool(call.clone()))
                    .await
                    .map_err(|_| Error("native_observer_closed"))?;
            }
            if let Some(event) = checkpoint_event {
                events
                    .send(event)
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

async fn stop_lane(
    bridge: &dyn Bridge,
    journal: &Journal,
    key: &str,
    lane: &mut Lane,
) -> Result<(), Error> {
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
            if state["round"] == lane.round && state["stopped"] == true {
                lane.phase = Phase::Stopped;
                journal.write(key, lane)?;
                return Err(Error("app_stop_acknowledged"));
            }
        }
        if tokio::time::Instant::now() >= until {
            // Retain stop intent even when observation failed; a later client
            // must reconcile cancellation, never publish or replay this round.
            return Err(Error("app_stop_unconfirmed"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
