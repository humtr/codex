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
    /// Read-only transport admission before common journal state is advanced.
    /// A known rejection is distinct from an uncertain dispatched operation.
    fn admit_submission(&self, _command: Value) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }
}

/// Transport is an immutable lane declaration, never a renderer-wide switch.
pub(crate) struct Transport {
    pub remote: Arc<dyn Bridge>,
    pub path: crate::config::ReturnPath,
}
impl Bridge for Transport {
    fn operation(&self, mut command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        if command["op"] == "open" {
            command["returnPath"] = json!(match self.path {
                crate::config::ReturnPath::Mcp => "mcp",
                crate::config::ReturnPath::LocalFunction => "local_function",
            });
        }
        self.remote.operation(command)
    }
    fn admit_submission(&self, command: Value) -> BoxFuture<'static, Result<(), Error>> {
        self.remote.admit_submission(command)
    }
}

struct Connection {
    generation: String,
    app: App,
    initialized: bool,
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
    mcp: bool,
    viewer: Option<Arc<crate::viewer::Viewer>>,
}
impl Remote {
    pub fn new(descriptor: PathBuf, owner: String) -> Self {
        Self {
            source: Source::Relay(descriptor),
            owner,
            connection: Arc::new(Mutex::new(None)),
            mcp: false,
            viewer: None,
        }
    }
    pub fn pipe(client: Client, owner: String) -> Self {
        Self {
            source: Source::Pipe(client),
            owner,
            connection: Arc::new(Mutex::new(None)),
            mcp: false,
            viewer: None,
        }
    }
    pub fn with_viewer(mut self, viewer: Option<Arc<crate::viewer::Viewer>>) -> Self {
        self.viewer = viewer;
        self
    }
    pub fn with_mcp(mut self) -> Self {
        self.mcp = true;
        self
    }
}
impl Bridge for Remote {
    fn operation(&self, mut command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let source = self.source.clone();
        let owner = self.owner.clone();
        let connection = self.connection.clone();
        let mcp = self.mcp;
        let viewer = self.viewer.clone();
        Box::pin(async move {
            let selected = match &source {
                Source::Relay(path) => Some(Descriptor::read(path)?),
                Source::Pipe(_) => None,
            };
            if command["op"] == "host_stop" {
                return Err(Error("host_shared_owner"));
            }
            if command["op"] == "host_open" {
                viewer
                    .as_ref()
                    .ok_or(Error("provider_viewer_unavailable"))?
                    .open()
                    .await?;
                return Ok(
                    json!({"viewer_open_requested":true,"inference_submitted":false,"host_started":false}),
                );
            }
            if command["op"] == "host_ensure" {
                command["op"] = json!("host_status");
            }
            let observation = matches!(
                command["op"].as_str(),
                Some(
                    "identity"
                        | "host_status"
                        | "chat_list"
                        | "chat_read"
                        | "chat_page"
                        | "project_list"
                        | "project_read"
                )
            );
            let bindings = json!({"initial":"app://-/assets/app-initial-ed5807835d97.js","shared":"app://-/assets/app-shared-2d77298f0100.js"});
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
                *guard = Some(Connection {
                    generation: generation.clone(),
                    app,
                    initialized: false,
                });
            }
            if !observation
                && !matches!(
                    command["op"].as_str(),
                    Some("snapshot" | "stop" | "release" | "dispose")
                )
                && (!guard.as_ref().unwrap().initialized || command["op"] == "open")
            {
                let app = guard.as_ref().unwrap().app.clone();
                let init = json!({"op":"init","owner":owner,"bridgeRevision":context::digest(&include_str!("app_bridge.js")),"returnPath":"mcp","bindings":bindings});
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
                guard.as_mut().unwrap().initialized = true;
            }
            let app = guard.as_ref().unwrap().app.clone();
            drop(guard);
            command["owner"] = json!(owner);
            command["bridgeRevision"] = json!(context::digest(&include_str!("app_bridge.js")));
            if observation {
                command["readOnlyContext"] = json!(true);
                command["bindings"] = bindings;
                command["bridgeRevision"] = json!(context::digest(&include_str!("app_bridge.js")));
            }
            if command["op"] == "open" && command.get("returnPath").is_none() {
                command["returnPath"] = json!(if mcp { "mcp" } else { "local_function" });
            }
            if command["op"] == "approve_ui" {
                let mut check = command.clone();
                check["op"] = json!("approval_prepare");
                evaluate_bridge(&app, &check).await?;
                viewer
                    .as_ref()
                    .ok_or(Error("app_approval_viewer_unavailable"))?
                    .open()
                    .await?;
            }
            let mut result = evaluate_bridge(&app, &command).await;
            if command["op"] == "approve_ui" {
                let until = tokio::time::Instant::now() + Duration::from_secs(20);
                while result.as_ref().is_ok_and(|value| value["pending"] == true) {
                    if tokio::time::Instant::now() >= until {
                        return Err(Error("app_approval_ui_unavailable"));
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    result = evaluate_bridge(&app, &command).await;
                }
            }
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
            if command["op"] == "dispose" && result.as_ref().is_ok_and(|v| v["disposed"] == true) {
                let mut current = connection.lock().await;
                if let Some(current) = current.as_mut().filter(|c| c.generation == generation) {
                    current.initialized = false;
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
pub(crate) fn valid_dispatches(snapshot: &Value) -> bool {
    let replies = match snapshot.get("roundHostReplies") {
        None => 0,
        Some(value) => match value.as_u64() {
            Some(value) => value,
            None => return false,
        },
    };
    matches!(replies.checked_add(1), Some(expected) if snapshot["roundStarts"].as_u64() == Some(expected))
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
        "app_stop_unconfirmed",
        "app_lane_declaration_changed",
        "app_hooks_changed",
        "app_messages_required",
        "app_bridge_owned",
        "app_bridge_owner_mismatch",
        "app_bridge_revision_required",
        "app_bridge_revision_conflict",
        "app_reveal_unavailable",
        "app_approval_unavailable",
        "app_approval_ambiguous",
        "app_approval_ui_required",
        "app_approval_ui_unavailable",
        "app_approval_manual_required",
        "app_approval_unconfirmed",
        "app_local_functions_unavailable",
        "app_return_path_invalid",
        "app_chat_operations_unavailable",
        "app_chat_response_invalid",
        "app_chat_unavailable",
        "app_chat_auth_expired",
        "app_chat_forbidden",
        "app_chat_not_found",
        "app_chat_temporarily_unavailable",
        "app_chat_rate_limited",
        "app_chat_read_pending",
        "app_chat_request_failed",
        "app_scope_unavailable",
        "app_operation_not_read_only",
        "app_bindings_unavailable",
        "app_chat_input_unavailable",
        "app_chat_owned_work",
        "app_chat_active_unowned",
        "assignment_budget_invalid",
        "assignment_budget_changed",
        "assignment_app_budget_exhausted",
        "assignment_time_budget_exhausted",
    ];
    let expression = format!("(async()=>{{const command={command};\ntry{{return {{value:await ({})(command)}};}}catch(e){{return {{error:{}.includes(e.message)?e.message:'app_bridge_operation_failed'}};}}}})()", include_str!("app_bridge.js"), serde_json::to_string(&known[..]).unwrap());
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
pub(crate) struct Identity {
    generation: String,
    account: String,
    user: String,
    structure: String,
}
impl Identity {
    pub(crate) fn parse(value: &Value) -> Result<Self, Error> {
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
    host_block: Option<String>,
    #[serde(default)]
    host_reply_messages: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    previous_chats: Vec<String>,
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
    let meaning = if lane.host_block.is_some()
        && matches!(lane.phase, Phase::Running | Phase::Dispatching)
    {
        "waiting for user authentication or confirmation in ChatGPT; original request retained"
    } else {
        match lane.phase {
            Phase::New | Phase::Opening => "preparing; remote acceptance may be unknown",
            Phase::Idle => "last response settled; native work may continue separately",
            Phase::Dispatching | Phase::Running => "app work was active at last observation",
            Phase::ToolsReady => "native tool calls emitted; awaiting native execution or results",
            Phase::CancelRequested => "stop requested; acknowledgment not confirmed",
            Phase::Stopped => "app acknowledged stop; native effects require native history review",
            Phase::Unknown => "remote work state unknown; reconcile before continuation",
        }
    };
    Ok(
        json!({"schema":"termux-codex-provider-lane-status-v1","thread":thread,
        "phase":lane.phase,"meaning":meaning,"observation":"last_private_journal",
        "pending_native_calls":lane.pending.len(),"public_event_count":lane.cached.len(),"conversation_id":lane.conversation_id,"previous_chats":lane.previous_chats,
        "host_block":lane.host_block,"host_action_reply_objects":lane.host_reply_messages,"app_dispatches":lane.dispatches,"user_role_objects":lane.user_messages,
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
            host_block: None,
            host_reply_messages: 0,
            rejected: 0,
            conversation_id: None,
            previous_chats: Vec::new(),
        })
    }
    fn successor(&self, thread: &str) -> Result<Self, Error> {
        let mut next = Self::new(thread)?;
        next.previous_chats = self.previous_chats.clone();
        if let Some(id) = &self.conversation_id {
            if !next.previous_chats.contains(id) {
                next.previous_chats.push(id.clone());
            }
        }
        if next.previous_chats.len() > 256 {
            return Err(Error("app_chat_lineage_limit"));
        }
        Ok(next)
    }
}

fn observe_counts(
    journal: &Journal,
    key: &str,
    lane: &mut Lane,
    state: &Value,
) -> Result<(), Error> {
    if let Some(id) = state["serverId"].as_str() {
        if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
            return Err(Error("app_conversation_identity_invalid"));
        }
        if lane.conversation_id.as_ref().is_some_and(|old| old != id) {
            return Err(Error("app_conversation_identity_changed"));
        }
        if lane.conversation_id.is_none() {
            lane.conversation_id = Some(id.to_owned());
            journal.write(key, lane)?;
        }
    }
    if state.get("dispatches").is_some()
        || state.get("userMessages").is_some()
        || state.get("toolMessages").is_some()
    {
        let counts = (
            state["dispatches"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?,
            state["userMessages"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?,
            state["toolMessages"]
                .as_u64()
                .ok_or(Error("invalid_submission_counters"))?,
        );
        let host_replies = state
            .get("hostReplyMessages")
            .map(|v| v.as_u64().ok_or(Error("invalid_submission_counters")))
            .transpose()?
            .unwrap_or(lane.host_reply_messages);
        if counts != (lane.dispatches, lane.user_messages, lane.tool_messages)
            || host_replies != lane.host_reply_messages
        {
            (lane.dispatches, lane.user_messages, lane.tool_messages) = counts;
            lane.host_reply_messages = host_replies;
            journal.write(key, lane)?;
        }
    }
    Ok(())
}

pub struct AppHost {
    bridge: Arc<dyn Bridge>,
    journal: Arc<Journal>,
    lanes: Arc<Mutex<BTreeMap<String, Arc<Mutex<Lane>>>>>,
    project_lock: Arc<Mutex<()>>,
    connector: String,
}
impl AppHost {
    pub fn new(bridge: Arc<dyn Bridge>, journal: Arc<Journal>) -> Self {
        Self {
            bridge,
            journal,
            lanes: Arc::new(Mutex::new(BTreeMap::new())),
            project_lock: Arc::new(Mutex::new(())),
            connector: crate::config::TransportConfig::default().connector_name,
        }
    }
    pub fn with_connector(mut self, connector: String) -> Self {
        self.connector = connector;
        self
    }
    async fn existing_lane(&self, thread: &str) -> Result<Option<Arc<Mutex<Lane>>>, Error> {
        let key = context::digest(&thread);
        let mutex = {
            let mut lanes = self.lanes.lock().await;
            if let Some(mutex) = lanes.get(&key) {
                Some(mutex.clone())
            } else if let Some(lane) = self.journal.read::<Lane>(&key)? {
                if lane.native_thread != thread {
                    return Err(Error("native_thread_identity_conflict"));
                }
                let mutex = Arc::new(Mutex::new(lane));
                lanes.insert(key.clone(), mutex.clone());
                Some(mutex)
            } else {
                None
            }
        };
        Ok(mutex)
    }
    pub async fn detach_settled(&self, thread: &str) -> Result<(), Error> {
        let key = context::digest(&thread);
        let mutex = self
            .existing_lane(thread)
            .await?
            .ok_or(Error("native_lane_not_found"))?;
        let mut lane = mutex.lock().await;
        if lane.phase != Phase::Idle
            || !lane.pending.is_empty()
            || !matches!(lane.cached.last(), Some(Event::Completed { .. }))
        {
            return Err(Error("app_lane_busy"));
        }
        let released = self
            .bridge
            .operation(json!({"op":"release","lane":lane.marker}))
            .await?;
        if released["released"] != true {
            return Err(Error("app_lane_release_unconfirmed"));
        }
        lane.phase = Phase::Stopped;
        self.journal.write(&key, &*lane)?;
        Ok(())
    }
    pub async fn stop(&self, thread: &str) -> Result<bool, Error> {
        let key = context::digest(&thread);
        let mutex = self.existing_lane(thread).await?;
        let Some(mutex) = mutex else {
            return Ok(false);
        };
        let mut lane = tokio::time::timeout(Duration::from_secs(15), mutex.lock())
            .await
            .map_err(|_| Error("app_stop_unconfirmed"))?;
        if lane.phase == Phase::Idle
            && lane.pending.is_empty()
            && matches!(lane.cached.last(), Some(Event::Completed { .. }))
        {
            // Durable response settlement remains proof after a clean app
            // context replacement. Do not recreate a completed conversation.
            lane.phase = Phase::Stopped;
            self.journal.write(&key, &*lane)?;
            return Ok(true);
        }
        if lane.phase != Phase::Stopped {
            match stop_lane(self.bridge.as_ref(), &self.journal, &key, &mut lane).await {
                Err(Error("app_stop_acknowledged")) => {}
                other => other?,
            }
        }
        Ok(lane.phase == Phase::Stopped)
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
                if matches!(lane.phase, Phase::Idle | Phase::Stopped) {
                    if let Ok(state) = bridge
                        .operation(json!({"op":"snapshot","lane":lane.marker}))
                        .await
                    {
                        if state["round"] == lane.round
                            && state["pending"] == false
                            && state["streamActive"] == false
                            && state["approvalPending"] != true
                            && (lane.phase == Phase::Stopped
                                && (state["stopped"] == true
                                    || lane.pending.is_empty()
                                        && matches!(
                                            lane.cached.last(),
                                            Some(Event::Completed { .. })
                                        )
                                        && state["error"].is_null())
                                || lane.phase == Phase::Idle && state["error"].is_null())
                            && bridge
                                .operation(json!({"op":"release","lane":lane.marker}))
                                .await
                                .is_ok_and(|v| v["released"] == true)
                        {
                            return Some(key);
                        }
                    }
                }
                None
            });
        }
        while let Some(result) = stops.join_next().await {
            if let Ok(Some(key)) = result {
                self.lanes.lock().await.remove(&key);
            }
        }
        // Passive disposal cannot acquire a missing host or initialize hooks.
        // Another live owned lane makes disposal refuse; no owner is replaced.
        let _ = self.bridge.operation(json!({"op":"dispose"})).await;
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

pub(crate) async fn approve_lane(
    bridge: Arc<dyn Bridge>,
    lane: &str,
    identity: Value,
    connector: &str,
    ui: bool,
) -> Result<Value, Error> {
    let command = json!({"op":if ui {"approve_ui"} else {"approve"},"lane":lane,"connector":connector,
                "identity":{"account":identity["account"],"user":identity["user"],"structure":identity["structure"]}});
    let result = match bridge.operation(command.clone()).await {
        Err(Error("app_approval_ui_required")) if !ui => {
            let mut fallback = command;
            fallback["op"] = json!("approve_ui");
            bridge.operation(fallback).await?
        }
        value => value?,
    };
    if result["path"] == "ui" {
        let message = result["messageId"]
            .as_str()
            .ok_or(Error("app_approval_unconfirmed"))?;
        let mut observed = false;
        for _ in 0..40 {
            let snapshot = bridge
                .operation(json!({"op":"snapshot","lane":lane}))
                .await?;
            if snapshot["approvalResponses"]
                .as_array()
                .is_some_and(|ids| ids.iter().any(|id| id == message))
            {
                observed = true;
                break;
            }
            if snapshot["stopped"] == true {
                return Err(Error("app_approval_unconfirmed"));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        if !observed {
            return Err(Error("app_approval_unconfirmed"));
        }
    }
    if result["requested"] != true
        || !matches!(result["scope"].as_str(), Some("conversation" | "once"))
        || !matches!(result["path"].as_str(), Some("action" | "ui"))
    {
        return Err(Error("app_approval_unconfirmed"));
    }
    Ok(
        json!({"app_approval_requested":true,"scope":result["scope"],"path":result["path"],"window_open_requested":result["path"]=="ui","native_completion_confirmed":false}),
    )
}

impl Host for AppHost {
    fn approve(&self, thread: String, ui: bool) -> BoxFuture<'static, Result<Value, Error>> {
        let bridge = self.bridge.clone();
        let journal = self.journal.clone();
        let connector = self.connector.clone();
        Box::pin(async move {
            let lane: Lane = journal
                .read(&context::digest(&thread))?
                .ok_or(Error("native_lane_not_found"))?;
            if lane.native_thread != thread {
                return Err(Error("native_thread_identity_conflict"));
            }
            if !matches!(lane.phase, Phase::Running | Phase::Dispatching) {
                return Err(Error("app_approval_unavailable"));
            }
            let expected = lane.identity.ok_or(Error("app_identity_unavailable"))?;
            let current = bridge
                .operation(json!({"op":"identity","lane":lane.marker}))
                .await?;
            if Identity::parse(&current)? != expected {
                return Err(Error("app_account_changed"));
            }
            approve_lane(bridge, &lane.marker, current, &connector, ui).await
        })
    }
    fn reveal(&self, thread: String) -> BoxFuture<'static, Result<(), Error>> {
        let bridge = self.bridge.clone();
        let journal = self.journal.clone();
        Box::pin(async move {
            if thread.is_empty() || thread.len() > 256 || thread.chars().any(char::is_control) {
                return Err(Error("invalid_thread_identity"));
            }
            let lane: Lane = journal
                .read(&context::digest(&thread))?
                .ok_or(Error("native_lane_not_found"))?;
            if lane.native_thread != thread {
                return Err(Error("native_thread_identity_conflict"));
            }
            let expected = lane.identity.ok_or(Error("app_identity_unavailable"))?;
            let current = Identity::parse(
                &bridge
                    .operation(json!({"op":"identity","lane":lane.marker}))
                    .await?,
            )?;
            if current != expected {
                return Err(Error("app_account_changed"));
            }
            bridge.operation(json!({"op":"reveal","lane":lane.marker,"identity":{"account":expected.account,"user":expected.user,"structure":expected.structure}})).await?;
            Ok(())
        })
    }
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

fn public_delta(lane: &Lane, source: &str, phase: &str, delta: String, full: &str) -> Event {
    let prefix = format!("{source}:continuation:");
    let previous = lane.cached.iter().rev().find_map(|event| match event {
        Event::Text { id, .. } if id == source || id.starts_with(&prefix) => Some(id),
        _ => None,
    });
    let current = previous.is_some_and(
        |previous| matches!(lane.cached.last(), Some(Event::Text { id, .. }) if id == previous),
    );
    let id = if current {
        previous.unwrap().clone()
    } else if previous.is_none() {
        source.to_owned()
    } else {
        format!("{prefix}{}", lane.cached.len())
    };
    let delta = if previous.is_some() && !current && delta.is_empty() && phase == "final_answer" {
        full.to_owned()
    } else {
        delta
    };
    Event::Text {
        id,
        phase: phase.into(),
        delta,
    }
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
            Event::Completed { .. } | Event::Flush => (),
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
    if request.guardian && lane.project.is_some() {
        return Err(Error("reviewer_project_isolation_required"));
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
    if let Some(old) = lane.identity.clone() {
        if !old.account_matches(&current) {
            if request.history.account_change == crate::config::AccountChange::NewChat
                && matches!(lane.phase, Phase::Idle | Phase::Stopped)
                && lane.pending.is_empty()
                && new_turn(lane, request)
                && lane.user_input != user_input_identity(request)
            {
                // A new native input uses the currently selected app account.
                // Keep the previous account's chat; never continue its remote
                // conversation, replay its calls or send its results elsewhere.
                *lane = lane.successor(&request.thread)?;
            } else {
                return Err(Error("app_account_changed"));
            }
        } else if old.generation != current.generation || observed["present"] != true {
            if matches!(lane.phase, Phase::Idle | Phase::Stopped) && new_turn(lane, request) {
                // Re-import only at a new native user turn after a settled round.
                // Old remote chats remain intact. No pending call is reissued.
                *lane = lane.successor(&request.thread)?;
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
        *lane = lane.successor(&request.thread)?;
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
        *lane = lane.successor(&request.thread)?;
        lane.identity = identity;
        journal.write(key, lane)?;
    }
    if lane.phase == Phase::Opening {
        // Opening allocates no inference. Reconcile the same marker and exact
        // declaration; never create a successor because its reply was lost.
        bridge
            .operation(json!({"op":"open","lane":lane.marker,"nativeThread":lane.native_thread,"nativeParent":request.parent,"signature":signature(&lane.gateway),"project":lane.project,"identity":lane.identity}))
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
        lane.project = if request.guardian {
            None
        } else {
            crate::project::resolve(bridge, journal, &request.history, &observed).await?
        };
        lane.phase = Phase::Opening;
        journal.write(key, lane)?;
        if bridge
            .operation(json!({"op":"open","lane":lane.marker,"nativeThread":lane.native_thread,"nativeParent":request.parent,"signature":signature(&lane.gateway),"project":lane.project,"identity":lane.identity}))
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
            instructions.push(format!("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from native Codex; the app must not execute tools itself.\n{}",serde_json::to_string(&request.tools).unwrap()));
        }
        if !request.host_tools.is_empty()
            && lane.prefix.tools != context::digest(&(&request.tools, &request.host_tools))
        {
            instructions.push(format!("Separate host-side capabilities: {}. These are app-owned search capabilities, not native Codex function names; do not send them through the local gateway. Keep public findings distinct from native command execution.",serde_json::to_string(&request.host_tools).unwrap()));
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
        if cancel.is_cancelled() {
            return stop_lane(bridge, journal, key, lane).await;
        }
        bridge.admit_submission(json!({"lane":lane.marker,"nativeThread":request.thread,"nativeParent":request.parent,"policyDigest":context::digest(&request.instructions),"hostToolsDigest":context::digest(&request.host_tools),"model":request.host_model,"thinkingEffort":request.thinking_effort,"messages":messages,"instructions":instructions,"results":results})).await?;
        if cancel.is_cancelled() {
            return stop_lane(bridge, journal, key, lane).await;
        }
        lane.request = request_digest;
        lane.turn = request.turn.clone();
        lane.user_input = user_input_identity(request);
        lane.round = random_id()?;
        lane.cached.clear();
        lane.public.clear();
        lane.rejected = 0;
        lane.phase = Phase::Dispatching;
        lane.host_block = None;
        lane.last_error = None;
        journal.write(key, lane)?;
        let result=bridge.operation(json!({"op":"submit","lane":lane.marker,"nativeThread":request.thread,"nativeParent":request.parent,"round":lane.round,"policyDigest":context::digest(&request.instructions),"hostToolsDigest":context::digest(&request.host_tools),"model":request.host_model,"thinkingEffort":request.thinking_effort,"messages":messages,"instructions":instructions,"results":results})).await;
        if result.is_err() {
            // Snapshot the same app round below. Do not replay submit after a
            // lost evaluate reply; it may already have sent the model request.
            lane.phase = Phase::Dispatching;
        } else {
            lane.phase = Phase::Running;
        }
        journal.write(key, lane)?;
    }
    let mut until = tokio::time::Instant::now() + Duration::from_secs(600);
    let mut read_failures = 0;
    loop {
        let poll_started = tokio::time::Instant::now();
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
            Err(error) => {
                read_failures += 1;
                if read_failures >= 3 {
                    // Error carries only a static Provider code, never host
                    // exception text, account data or request contents.
                    eprintln!("app_observation_failed: {}", error.0);
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
        observe_counts(journal, key, lane, &state)?;
        if let Some(
            code @ ("mcp_host_authentication_required" | "mcp_host_confirmation_required"),
        ) = state["error"].as_str()
        {
            if lane.host_block.as_deref() != Some(code) {
                lane.host_block = Some(code.into());
                lane.phase = Phase::Running;
                let event = Event::Text {
                    id: format!("host-wait-{}-{}", lane.round, code), phase:"commentary".into(),
                    delta: if code == "mcp_host_confirmation_required" {
                        "ChatGPT is waiting for your confirmation to use the connected app. Open ChatGPT to approve or reject; this request remains attached."
                    } else {
                        "ChatGPT is waiting for you to authenticate the connected app. Complete its sign-in in ChatGPT; this request remains attached."
                    }.into(),
                };
                lane.cached.push(event.clone());
                journal.write(key, lane)?;
                events
                    .send(event)
                    .await
                    .map_err(|_| Error("native_observer_closed"))?;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            // A human permission/sign-in wait is not model execution time.
            // Transport cancellation is still checked on every iteration.
            until += poll_started.elapsed();
            continue;
        }
        if lane.host_block.take().is_some() {
            journal.write(key, lane)?;
        }
        if state["error"].is_string() {
            lane.phase = Phase::Unknown;
            journal.write(key, lane)?;
            return Err(Error(match state["error"].as_str() {
                Some("mcp_host_authentication_required") => "mcp_host_authentication_required",
                Some("mcp_host_confirmation_required") => "mcp_host_confirmation_required",
                Some("assignment_app_budget_exhausted") => "assignment_app_budget_exhausted",
                Some("assignment_time_budget_exhausted") => "assignment_time_budget_exhausted",
                _ => "app_round_failed",
            }));
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
            let rendered = crate::presentation::text(&text, phase == "final_answer");
            let previous_rendered = previous.as_ref().map_or_else(String::new, |(phase, text)| {
                crate::presentation::text(text, phase == "final_answer")
            });
            if !rendered.starts_with(&previous_rendered) {
                return Err(Error("public_presentation_changed"));
            }
            let offset = previous_rendered.len();
            lane.public.insert(id.into(), (phase.into(), text.clone()));
            let promoted = previous.as_ref().is_some_and(|(p, _)| p != phase);
            if (rendered.len() > offset || promoted)
                && request.compaction != Some(context::Compaction::Checkpoint)
            {
                let event = public_delta(lane, id, phase, rendered[offset..].into(), &rendered);
                publish(lane, events, event).await?;
            }
            if phase == "final_answer"
                && message["end_turn"] == true
                && message["status"] == "finished_successfully"
            {
                final_seen = true;
            }
        }
        if !state["terminals"].as_array().is_some_and(|v| v.is_empty()) {
            if state["terminals"] != json!(["done"]) || !valid_dispatches(&state) {
                lane.phase = Phase::Unknown;
                journal.write(key, lane)?;
                return Err(Error("app_round_incomplete"));
            }
            // A user can change the app account while inference is running.
            // Recheck before publishing executable calls or committing a summary;
            // startup identity alone cannot authorize those later effects.
            let observed = bridge
                .operation(json!({"op":"identity","lane":lane.marker}))
                .await?;
            let current = Identity::parse(&observed)?;
            let previous = lane
                .identity
                .as_ref()
                .ok_or(Error("app_identity_unavailable"))?;
            if !previous.account_matches(&current) {
                return Err(Error("app_account_changed"));
            }
            if previous.generation != current.generation || observed["present"] != true {
                return Err(Error("owned_app_round_unavailable"));
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
                let submitted=bridge.operation(json!({"op":"submit","lane":lane.marker,"nativeThread":request.thread,"nativeParent":request.parent,"round":lane.round,"model":request.host_model,"thinkingEffort":request.thinking_effort,"messages":[],"instructions":[],"results":[{"id":id,"result":result}]})).await;
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
                        delta: crate::presentation::text(&report, true),
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
    lane.host_block = None;
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
            if state["round"] == lane.round {
                observe_counts(journal, key, lane, &state)?;
            }
            if (state["round"] == lane.round || lane.round.is_empty() && state["round"].is_null())
                && state["stopped"] == true
            {
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

#[cfg(test)]
mod approval_tests {
    use super::*;

    #[test]
    fn replay_owned_public_failure_trace_when_supplied() {
        let Ok(path) = std::env::var("CODEX_PROVIDER_PUBLIC_REPLAY") else {
            return;
        };
        let events: Vec<Event> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(!events.is_empty());
        let mut old = crate::responses::Projection::new("old".into(), "model".into());
        assert!(events.iter().cloned().any(|event| old.push(event).is_err()));
        let mut lane = Lane::new("recorded-public-replay").unwrap();
        let mut projection = crate::responses::Projection::new("fixed".into(), "model".into());
        let mut full = BTreeMap::<String, String>::new();
        let mut original_text = String::new();
        for event in events {
            let Event::Text { id, phase, delta } = event else {
                panic!("expected captured public text");
            };
            original_text.push_str(&delta);
            full.entry(id.clone()).or_default().push_str(&delta);
            let event = public_delta(&lane, &id, &phase, delta, &full[&id]);
            lane.cached.push(event.clone());
            projection.push(event).unwrap();
        }
        let frames = projection
            .push(Event::Completed {
                input_tokens: 0,
                output_tokens: 0,
            })
            .unwrap();
        let actual: String = frames.last().unwrap()["response"]["output"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["content"][0]["text"].as_str().unwrap())
            .collect();
        assert_eq!(actual, original_text);
    }

    #[test]
    fn interleaved_public_suffixes_append_without_reopening_completed_items() {
        let mut lane = Lane::new("interleaved-public").unwrap();
        let mut projection = crate::responses::Projection::new("response".into(), "model".into());
        for (source, phase, delta, full) in [
            ("a", "commentary", "Initial report", "Initial report"),
            ("b", "commentary", "Progress", "Progress"),
            (
                "a",
                "commentary",
                "\nCorrection",
                "Initial report\nCorrection",
            ),
            ("a", "final_answer", "", "Initial report\nCorrection"),
        ] {
            let event = public_delta(&lane, source, phase, delta.into(), full);
            lane.cached.push(event.clone());
            projection.push(event).unwrap();
        }
        let frames = projection
            .push(Event::Completed {
                input_tokens: 0,
                output_tokens: 0,
            })
            .unwrap();
        let output = frames.last().unwrap()["response"]["output"]
            .as_array()
            .unwrap();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0]["content"][0]["text"], "Initial report");
        assert_eq!(output[1]["content"][0]["text"], "Progress");
        assert_eq!(output[2]["content"][0]["text"], "\nCorrection");
        assert_eq!(output[2]["phase"], "final_answer");
        assert_ne!(output[0]["id"], output[2]["id"]);
        assert_eq!(
            native_output(&lane.cached).unwrap(),
            output
                .iter()
                .map(|v| public_item(
                    v["id"].as_str().unwrap(),
                    v["phase"].as_str().unwrap(),
                    v["content"][0]["text"].as_str().unwrap()
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn phase_only_final_promotion_of_an_older_item_preserves_the_full_final() {
        let mut lane = Lane::new("older-final").unwrap();
        let initial = public_delta(
            &lane,
            "a",
            "commentary",
            "Whole answer".into(),
            "Whole answer",
        );
        lane.cached.push(initial);
        let second = public_delta(
            &lane,
            "b",
            "commentary",
            "Other progress".into(),
            "Other progress",
        );
        lane.cached.push(second);
        let final_event = public_delta(&lane, "a", "final_answer", String::new(), "Whole answer");
        match &final_event {
            Event::Text { id, phase, delta } => {
                assert_ne!(id, "a");
                assert_eq!(phase, "final_answer");
                assert_eq!(delta, "Whole answer");
            }
            _ => panic!("expected public final"),
        }
        lane.cached.push(final_event);
        let mut live = crate::responses::Projection::new("live".into(), "model".into());
        let mut replay = crate::responses::Projection::new("replay".into(), "model".into());
        for event in lane.cached {
            live.push(event.clone()).unwrap();
            replay.push(event).unwrap();
        }
        let end = Event::Completed {
            input_tokens: 0,
            output_tokens: 0,
        };
        let left = live.push(end.clone()).unwrap();
        let right = replay.push(end).unwrap();
        assert_eq!(
            left.last().unwrap()["response"]["output"],
            right.last().unwrap()["response"]["output"]
        );
    }
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex as SyncMutex,
    };
    #[test]
    fn saved_chat_identity_survives_legacy_decode_and_successor() {
        let mut old = serde_json::to_value(Lane::new("thread").unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("conversation_id");
        old.as_object_mut().unwrap().remove("previous_chats");
        let mut lane: Lane = serde_json::from_value(old).unwrap();
        assert!(lane.previous_chats.is_empty());
        lane.conversation_id = Some("saved-source".into());
        let next = lane.successor("thread").unwrap();
        assert_eq!(next.previous_chats, ["saved-source"]);
        assert_eq!(next.native_thread, "thread");
        assert!(next.conversation_id.is_none());
    }
    struct Cleanup {
        calls: SyncMutex<Vec<Value>>,
    }
    #[tokio::test]
    async fn stop_reuses_committed_settlement_after_app_context_disappears() {
        let root = std::env::temp_dir().join(format!("settled-stop-{}", random_id().unwrap()));
        let journal = Arc::new(Journal::open(&root).unwrap());
        let bridge = Arc::new(Cleanup {
            calls: SyncMutex::new(Vec::new()),
        });
        let host = AppHost::new(bridge.clone(), journal.clone());
        let mut lane = Lane::new("completed-native-thread").unwrap();
        lane.phase = Phase::Idle;
        lane.cached.push(Event::Completed {
            input_tokens: 1,
            output_tokens: 1,
        });
        let key = context::digest(&"completed-native-thread");
        journal.write(&key, &lane).unwrap();
        assert!(host.stop("completed-native-thread").await.unwrap());
        assert!(host.stop("completed-native-thread").await.unwrap());
        assert!(bridge.calls.lock().unwrap().is_empty());
        assert!(journal.read::<Lane>(&key).unwrap().unwrap().phase == Phase::Stopped);
        std::fs::remove_dir_all(root).unwrap();
    }
    impl Bridge for Cleanup {
        fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
            self.calls.lock().unwrap().push(command.clone());
            let result = match command["op"].as_str().unwrap() {
                "snapshot" => {
                    Ok(json!({"round":"settled","pending":false,"streamActive":false,"error":null}))
                }
                "release" => Ok(json!({"released":true})),
                "dispose" => Ok(json!({"disposed":true})),
                _ => panic!("shutdown must not submit or initialize app hooks"),
            };
            Box::pin(async move { result })
        }
    }
    #[tokio::test]
    async fn shutdown_after_receipt_only_stop_requires_matching_completed_round() {
        for completed in [false, true] {
            let root =
                std::env::temp_dir().join(format!("receipt-cleanup-{}", random_id().unwrap()));
            let journal = Arc::new(Journal::open(&root).unwrap());
            let bridge = Arc::new(Cleanup {
                calls: SyncMutex::new(Vec::new()),
            });
            let host = AppHost::new(bridge.clone(), journal.clone());
            let mut lane = Lane::new("completed-thread").unwrap();
            lane.phase = if completed {
                Phase::Idle
            } else {
                Phase::Stopped
            };
            lane.round = "settled".into();
            lane.conversation_id = Some("saved-source".into());
            if completed {
                lane.cached.push(Event::Completed {
                    input_tokens: 1,
                    output_tokens: 1,
                });
            }
            let key = context::digest(&"completed-thread");
            journal.write(&key, &lane).unwrap();
            host.lanes
                .lock()
                .await
                .insert(key.clone(), Arc::new(Mutex::new(lane)));
            if completed {
                assert!(host.stop("completed-thread").await.unwrap());
                assert!(bridge.calls.lock().unwrap().is_empty());
            }
            host.shutdown().await;
            assert_eq!(host.lanes.lock().await.is_empty(), completed);
            assert_eq!(
                bridge
                    .calls
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|v| v["op"] == "release"),
                completed
            );
            assert_eq!(
                journal
                    .read::<Lane>(&key)
                    .unwrap()
                    .unwrap()
                    .conversation_id
                    .as_deref(),
                Some("saved-source")
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[tokio::test]
    async fn shutdown_releases_confirmed_idle_lane_and_preserves_saved_identity() {
        let root = std::env::temp_dir().join(format!("cleanup-{}", random_id().unwrap()));
        let journal = Arc::new(Journal::open(&root).unwrap());
        let bridge = Arc::new(Cleanup {
            calls: SyncMutex::new(Vec::new()),
        });
        let host = AppHost::new(bridge.clone(), journal.clone());
        let mut lane = Lane::new("native-thread").unwrap();
        lane.phase = Phase::Idle;
        lane.round = "settled".into();
        lane.conversation_id = Some("saved-source".into());
        let key = context::digest(&"native-thread");
        journal.write(&key, &lane).unwrap();
        host.lanes
            .lock()
            .await
            .insert(key.clone(), Arc::new(Mutex::new(lane)));
        host.shutdown().await;
        assert!(host.lanes.lock().await.is_empty());
        assert_eq!(
            journal
                .read::<Lane>(&key)
                .unwrap()
                .unwrap()
                .conversation_id
                .as_deref(),
            Some("saved-source")
        );
        assert_eq!(
            bridge
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|v| v["op"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["snapshot", "release", "dispose"]
        );
        drop(host);
        drop(journal);
        std::fs::remove_dir_all(root).unwrap();
    }
    struct Approval {
        calls: SyncMutex<Vec<Value>>,
        changed: AtomicBool,
    }
    impl Bridge for Approval {
        fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
            self.calls.lock().unwrap().push(command.clone());
            let result = match command["op"].as_str().unwrap() {
                "identity" => Ok(
                    json!({"generation":"generation","account":if self.changed.load(Ordering::SeqCst){"other"}else{"account"},"user":"user","structure":"personal"}),
                ),
                "approve" => {
                    assert_eq!(command["connector"], "Selected connector");
                    Err(Error("app_approval_ui_required"))
                }
                "approve_ui" => Ok(
                    json!({"requested":true,"path":"ui","scope":"conversation","messageId":"original-confirmation"}),
                ),
                "snapshot" => {
                    Ok(json!({"approvalResponses":["original-confirmation"],"stopped":false}))
                }
                _ => panic!("unexpected app control {}", command["op"]),
            };
            Box::pin(async move { result })
        }
    }
    #[tokio::test]
    async fn active_managed_lane_approval_uses_original_host_reply_and_rejects_changed_account() {
        struct Root(std::path::PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Root(std::env::temp_dir().join(format!("ap-{}", random_id().unwrap())));
        let journal = Arc::new(Journal::open(&root.0).unwrap());
        let bridge = Arc::new(Approval {
            calls: SyncMutex::new(Vec::new()),
            changed: AtomicBool::new(false),
        });
        let host = AppHost::new(bridge.clone(), journal.clone())
            .with_connector("Selected connector".into());
        let mut lane = Lane::new("native-thread").unwrap();
        lane.phase = Phase::Running;
        lane.identity=Some(Identity::parse(&json!({"generation":"generation","account":"account","user":"user","structure":"personal"})).unwrap());
        let key = context::digest(&"native-thread");
        journal.write(&key, &lane).unwrap();
        let mutex = Arc::new(Mutex::new(lane));
        host.lanes.lock().await.insert(key, mutex.clone());
        // A running response owns the lane mutex; consent must still observe
        // its journal and the same original app confirmation concurrently.
        let _active = mutex.lock().await;
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            host.approve("native-thread".into(), false),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result["scope"], "conversation");
        assert_eq!(result["window_open_requested"], true);
        assert_eq!(result["native_completion_confirmed"], false);
        let before = bridge.calls.lock().unwrap().len();
        bridge.changed.store(true, Ordering::SeqCst);
        assert_eq!(
            host.approve("native-thread".into(), false).await,
            Err(Error("app_account_changed"))
        );
        let calls = bridge.calls.lock().unwrap();
        assert_eq!(calls.len(), before + 1);
        assert_eq!(calls.last().unwrap()["op"], "identity");
        assert!(calls.iter().all(|v| v["op"] != "submit"));
    }
}
