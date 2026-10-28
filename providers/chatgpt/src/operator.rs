//! MCP operates one native work binding. Responses rounds carry native tool
//! dispatch only; Chat lifetime is independent. Upstream is the transcript owner.
use crate::{
    app::{AppHost, Bridge},
    config::{Config, ReturnPath},
    context::{self, digest},
    journal::{random_id, Journal},
    mcp::{Arguments, Broker},
    operation::{self, Operations},
    operator_native::{Native, Owner, Pool},
    protocol::{gateway_calls, Error, Request},
    responses::Event,
    server::Host,
};
use axum::{
    extract::{Request as HttpRequest, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_util::sync::CancellationToken;

const HANDOFF: &str = "Codex Chat context handoff v1\n";
const DIRECT_PROGRESS: &str = "Record and display every complete public progress message for this task verbatim in Native before publishing the same message in ChatGPT. On joining, also record public progress already shown for this task. Do not replace full public text with a shorter status, reduce reasoning or tool use, or expose hidden reasoning/private controls. If a native tool call's report already records that exact message, do not record it again with codex_report. Retry the same call_id and text after uncertain delivery. Submitted reports are not independently observed host messages.";
const DIRECT_COMPLETION: &str = "Record the complete public final answer and confirm settled Native completion. Prepare the full answer before codex_finish; after it succeeds, return exactly that same text in ChatGPT, without a second summary, additions or omissions. Preserve all requested detail and public progress; do not reduce reasoning or tool use. Never include private controller, return or service values in public text. private_control_in_public_text means nothing was admitted: remove only those private values and correct the same call_id. Exact admitted retries use the same call_id and text to recover settlement without emitting again. Submitted completion is not independently observed host provenance.";
const LIMIT: usize = 16 * 1024 * 1024;
pub(crate) const WORK_SCHEMA: &str = "termux-codex-operator-work-v2";
const SOURCE_ADMISSION_SCHEMA: &str = "codex-chatgpt-source-admission-v1";
const RECONCILE_INTERVAL: Duration = Duration::from_secs(2);
fn source_observation_pending(error: &Error) -> bool {
    matches!(
        error.0,
        "app_chat_read_pending"
            | "cdp_operation_timeout"
            | "app_chat_rate_limited"
            | "app_chat_temporarily_unavailable"
            | "app_chat_request_failed"
            | "bound_chat_turn_active"
            | "bound_chat_observation_busy"
            | "bound_chat_observation_superseded"
            | "native_thread_not_loaded"
    )
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceAdmission {
    schema: String,
    work_id: String,
    thread_id: String,
    turn_id: String,
    account_ref: String,
    source: crate::bound_chat::Input,
    source_anchor: String,
    client_user_message_id: String,
    execution_home: std::path::PathBuf,
    workspace: std::path::PathBuf,
}
const DEVELOPMENT_BINDING_SCHEMA: &str = "codex-chatgpt-development-binding-v1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum SourceObservation {
    Observing,
    RetryWait {
        reason: String,
        retry_at_unix_ms: u64,
    },
    Blocked {
        reason: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DevelopmentBinding {
    schema: String,
    work_id: String,
    thread_id: String,
    account_ref: String,
    conversation_id: String,
    enabled: bool,
    last_observation: SourceObservation,
    #[serde(default)]
    source_selection_required: bool,
}
fn development_binding_key(work: &str) -> String {
    digest(&("development-binding-v1", work))
}
fn source_admission_key(work: &str, turn: &str) -> String {
    digest(&("source-admission-v1", work, turn))
}
fn queue_key(thread: &str, call: &str) -> String {
    digest(&("native-queue-v1", thread, call))
}
fn queue_client_id(thread: &str, call: &str) -> String {
    format!("termux-queue-{}", queue_key(thread, call))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    reasoning: Reasoning,
    return_path: ReturnPath,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Reasoning {
    Direct,
    Provider,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    max_parallel: usize,
    max_app_starts: usize,
    max_native_calls: usize,
    max_elapsed_seconds: u64,
}
impl Budget {
    fn extend_budget(&self, proposed: Self) -> Result<Self, Error> {
        if proposed.max_parallel != self.max_parallel
            || proposed.max_app_starts != self.max_app_starts
            || proposed.max_native_calls < self.max_native_calls
            || proposed.max_elapsed_seconds < self.max_elapsed_seconds
            || proposed.max_elapsed_seconds == 0
        {
            return Err(Error("assignment_budget_extension_invalid"));
        }
        Ok(proposed)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assignment {
    role: String,
    parent: Option<String>,
    dependencies: Vec<String>,
    budget: Budget,
    #[serde(default)]
    admitted_calls: std::collections::BTreeSet<String>,
    #[serde(default)]
    evaluation: Option<Value>,
    #[serde(default)]
    started_at_unix_ms: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    id: String,
    thread: String,
    execution: Owner,
    origin: String,
    native_turn: Option<String>,
    generation: u64,
    capability: String,
    token: String,
    phase: String,
    pending: Vec<String>,
    receipts: BTreeMap<String, (String, String)>,
    summary: Option<String>,
    source: Option<Cursor>,
    source_context: Option<Cursor>,
    accepted: bool,
    driver: Option<String>,
    driver_phase: String,
    identity: Option<String>,
    host_observed_at: Option<u64>,
    host_last_state: String,
    observed: Option<Cursor>,
    observed_context: Option<Cursor>,
    policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selection: Option<Selection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workspace: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assignment: Option<Assignment>,
}
// This offline upgrade/rollback boundary is the only reader of old selection
// names. Normal API and journal decoding accept only Direct/Provider.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReasoningMigration {
    schema: String,
    owner: String,
    root: std::path::PathBuf,
    target: String,
    records: BTreeMap<String, Value>,
}
fn migrate_selection(value: &mut Value, target: &str) -> Result<(), Error> {
    let Some(selection) = value.get_mut("selection").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let name = match selection["reasoning"].as_str() {
        Some("current" | "direct") => {
            if target == "direct-provider" {
                "direct"
            } else {
                "current"
            }
        }
        Some("managed" | "provider") => {
            if target == "direct-provider" {
                "provider"
            } else {
                "managed"
            }
        }
        _ => return Err(Error("reasoning_migration_selection_invalid")),
    };
    selection["reasoning"] = json!(name);
    Ok(())
}
fn migration_record(mut value: Value, target: &str) -> Result<Value, Error> {
    if !matches!(
        value["schema"].as_str(),
        Some("termux-codex-operator-work-v1" | WORK_SCHEMA)
    ) {
        return Err(Error("reasoning_migration_record_invalid"));
    }
    value["schema"] = json!(WORK_SCHEMA);
    migrate_selection(&mut value, "direct-provider")?;
    serde_json::from_value::<Record>(value.clone())
        .map_err(|_| Error("reasoning_migration_record_invalid"))?;
    migrate_selection(&mut value, target)?;
    if target == "provider14" {
        value["schema"] = json!("termux-codex-operator-work-v1");
    }
    Ok(value)
}
/// Explicit offline format conversion, including the inverse needed before an
/// old binary rollback. Original Chat/native history and call receipts stay put.
pub fn migrate_reasoning(
    root: &std::path::Path,
    backup: &std::path::Path,
    target: &str,
) -> Result<Value, Error> {
    use std::{
        fs::{self, File, OpenOptions},
        io::Write,
        os::unix::fs::OpenOptionsExt,
    };
    if !matches!(target, "direct-provider" | "provider14") {
        return Err(Error("reasoning_migration_target_invalid"));
    }
    let parent = backup
        .parent()
        .ok_or(Error("reasoning_migration_backup_invalid"))?;
    if !root.is_absolute()
        || fs::canonicalize(root).ok().as_deref() != Some(root)
        || !backup.is_absolute()
        || fs::canonicalize(parent).ok().as_deref() != Some(parent)
        || backup.starts_with(root)
        || !root.join("owner-v1").is_file()
    {
        return Err(Error("reasoning_migration_backup_invalid"));
    }
    crate::journal::private_directory(parent)?;
    let journal = Journal::open(root)?;
    let snapshot = if backup.exists() {
        let size = fs::symlink_metadata(backup)
            .map_err(|_| Error("reasoning_migration_backup_unavailable"))?
            .len();
        let raw = crate::cdp::private_read(backup, size)?;
        let snapshot: ReasoningMigration = serde_json::from_slice(&raw)
            .map_err(|_| Error("reasoning_migration_backup_invalid"))?;
        if snapshot.schema != "termux-codex-reasoning-migration-v1"
            || snapshot.owner != journal.owner
            || snapshot.root != root
            || snapshot.target != target
        {
            return Err(Error("reasoning_migration_backup_invalid"));
        }
        snapshot
    } else {
        let mut records = BTreeMap::new();
        let mut source = journal.records("termux-codex-operator-work-v1")?;
        source.extend(journal.records(WORK_SCHEMA)?);
        for (key, value) in source {
            if key != digest(&("operator-work-v1", &value["id"]))
                && key
                    != digest(&(
                        "operator-previous-controller-v1",
                        &value["id"],
                        &value["generation"],
                    ))
            {
                return Err(Error("reasoning_migration_record_key_invalid"));
            }
            let converted = migration_record(value.clone(), target)?;
            if converted != value {
                records.insert(key, value.clone());
            }
            let lane_key = digest(&value["thread"]);
            if let Some(lane) = journal.read::<Value>(&lane_key)? {
                if lane["native_thread"] != value["thread"] {
                    return Err(Error("reasoning_migration_record_key_invalid"));
                }
                if crate::app::migrate_source_queue(lane.clone(), target)? != lane {
                    records.insert(lane_key, lane);
                }
            }
            let key = digest(&("work-route-transfer-v1", &value["id"]));
            if let Some(mut offer) = journal.read::<Value>(&key)? {
                let before = offer.clone();
                if offer["work_id"] != value["id"] {
                    return Err(Error("reasoning_migration_record_key_invalid"));
                }
                migrate_selection(&mut offer, target)?;
                if before != offer {
                    records.insert(key, before);
                }
            }
        }
        let key = work_index_key();
        if let Some(mut index) = journal.read::<Value>(&key)? {
            let before = index.clone();
            if !index["pending"].is_null() {
                index["pending"] = migration_record(index["pending"].clone(), target)?;
            }
            if before != index {
                records.insert(key, before);
            }
        }
        let snapshot = ReasoningMigration {
            schema: "termux-codex-reasoning-migration-v1".into(),
            owner: journal.owner.clone(),
            root: root.into(),
            target: target.into(),
            records,
        };
        let bytes = serde_json::to_vec(&snapshot)
            .map_err(|_| Error("reasoning_migration_backup_invalid"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(backup)
            .map_err(|_| Error("reasoning_migration_backup_unavailable"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| Error("reasoning_migration_backup_unavailable"))?;
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|_| Error("reasoning_migration_backup_unavailable"))?;
        snapshot
    };
    // Validate every current value before changing any: an interrupted conversion
    // may resume only from its original or exact converted values.
    let mut updates = Vec::new();
    for (key, before) in &snapshot.records {
        let mut after = before.clone();
        if matches!(
            after["schema"].as_str(),
            Some("termux-codex-operator-work-v1" | WORK_SCHEMA)
        ) {
            after = migration_record(after, target)?;
        } else if let Some(thread) = after["native_thread"].as_str() {
            if key != &digest(&thread) {
                return Err(Error("reasoning_migration_record_key_invalid"));
            }
            after = crate::app::migrate_source_queue(after, target)?;
        } else if key == &work_index_key() {
            if !after["pending"].is_null() {
                after["pending"] = migration_record(after["pending"].clone(), target)?;
            }
        } else {
            migrate_selection(&mut after, target)?;
        }
        let current = journal
            .read::<Value>(key)?
            .ok_or(Error("reasoning_migration_state_changed"))?;
        if current != *before && current != after {
            return Err(Error("reasoning_migration_state_changed"));
        }
        updates.push((key, after));
    }
    for (key, after) in &updates {
        journal.write(key, after)?;
    }
    Ok(
        json!({"converted_records":updates.len(),"target":target,"backup":backup,
        "original_transcripts_unchanged":true,"inference_submitted":false}),
    )
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkIndex {
    ids: Vec<String>,
    pending: Option<Record>,
}
fn work_index_key() -> String {
    digest(&"operator-index-v2")
}
fn commit_work_index(journal: &Journal, index: &WorkIndex) -> Result<(), Error> {
    // Keep the legacy lookup usable at settled downgrade boundaries. A pending
    // v2 registration must be reconciled before admitting an older writer.
    if index.pending.is_none() {
        journal.write(&digest(&"operator-index-v1"), &index.ids)?;
    }
    journal.write(&work_index_key(), index)
}
fn recover_work_index(journal: &Journal) -> Result<WorkIndex, Error> {
    let legacy = journal
        .read::<Vec<String>>(&digest(&"operator-index-v1"))?
        .unwrap_or_default();
    let mut index = if let Some(index) = journal.read::<WorkIndex>(&work_index_key())? {
        index
    } else {
        if !journal.records("termux-codex-operator-work-v1")?.is_empty() {
            return Err(Error("reasoning_migration_required"));
        }
        let mut ids = legacy.clone();
        // Include pre-migration orphan work records; do not mistake an old
        // incomplete index for authority that those native works do not exist.
        for (key, value) in journal.records(WORK_SCHEMA)? {
            let record: Record =
                serde_json::from_value(value).map_err(|_| Error("operator_record_invalid"))?;
            if key != digest(&("operator-work-v1", &record.id)) {
                return Err(Error("operator_record_invalid"));
            }
            if !ids.contains(&record.id) {
                ids.push(record.id);
            }
        }
        WorkIndex { ids, pending: None }
    };
    // An explicitly admitted old writer may have registered new settled work.
    // Its complete records remain authoritative when returning to this source.
    for id in legacy {
        if !index.ids.contains(&id) {
            index.ids.push(id);
        }
    }
    if let Some(record) = &index.pending {
        if record.schema != WORK_SCHEMA {
            return Err(Error("reasoning_migration_required"));
        }
        let key = digest(&("operator-work-v1", &record.id));
        if journal.read::<Record>(&key)?.is_none() {
            journal.write(&key, record)?;
        }
        if !index.ids.contains(&record.id) {
            index.ids.push(record.id.clone());
        }
        index.pending = None;
    }
    if index.ids.len() > 256
        || index
            .ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != index.ids.len()
    {
        return Err(Error("operator_record_invalid"));
    }
    commit_work_index(journal, &index)?;
    Ok(index)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Cursor {
    count: usize,
    digest: String,
}
struct Work {
    record: Record,
    active: Option<mpsc::Sender<Command>>,
    request: Option<Request>,
    round: String,
    checkpoint: Option<context::Checkpoint>,
    driver_lock: Arc<Mutex<()>>,
    native_admission: Arc<Mutex<()>>,
    stops_in_progress: usize,
    app_cancel: Option<CancellationToken>,
    completion_observer: Option<CancellationToken>,
}
enum Command {
    Text {
        capability: String,
        id: String,
        text: String,
        final_answer: bool,
        ack: oneshot::Sender<Result<(), Error>>,
    },
    Summary {
        capability: String,
        round: String,
        text: String,
        ack: oneshot::Sender<Result<(), Error>>,
    },
}
type Works = Arc<Mutex<BTreeMap<String, Work>>>;
struct LimitedBridge {
    remote: Arc<dyn Bridge>,
    works: Works,
}
impl Bridge for LimitedBridge {
    fn operation(&self, mut command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let remote = self.remote.clone();
        let works = self.works.clone();
        Box::pin(async move {
            if matches!(command["op"].as_str(), Some("open" | "chat_adopt")) {
                let works = works.lock().await;
                if let Some(work) = command["nativeThread"]
                    .as_str()
                    .and_then(|thread| works.get(thread))
                    .or_else(|| {
                        command["nativeParent"]
                            .as_str()
                            .and_then(|thread| works.get(thread))
                    })
                {
                    if work.stops_in_progress != 0
                        || matches!(
                            work.record.phase.as_str(),
                            "stop_requested" | "stopped" | "reconciliation_required"
                        )
                    {
                        return Err(Error("operator_controller_fenced"));
                    }
                    if let Some(assignment) = &work.record.assignment {
                        command["limits"] = json!({"maxDispatches":assignment.budget.max_app_starts,"deadlineUnixMs":assignment.started_at_unix_ms.map(|start|start.saturating_add(assignment.budget.max_elapsed_seconds.saturating_mul(1000)))});
                        command["nativeThread"] = json!(work.record.thread);
                    }
                }
            }
            remote.operation(command).await
        })
    }
    fn admit_submission(&self, command: Value) -> BoxFuture<'static, Result<(), Error>> {
        let remote = self.remote.clone();
        let works = self.works.clone();
        Box::pin(async move {
            let works = works.lock().await;
            if let Some(work) = command["nativeThread"]
                .as_str()
                .and_then(|id| works.get(id))
                .or_else(|| {
                    command["nativeParent"]
                        .as_str()
                        .and_then(|id| works.get(id))
                })
            {
                if work.stops_in_progress != 0
                    || matches!(
                        work.record.phase.as_str(),
                        "stop_requested" | "stopped" | "reconciliation_required"
                    )
                {
                    return Err(Error("operator_controller_fenced"));
                }
            }
            drop(works);
            remote.admit_submission(command).await
        })
    }
}
#[derive(Clone)]
pub struct Operator {
    pool: Arc<Pool>,
    broker: Arc<Broker>,
    journal: Arc<Journal>,
    works: Works,
    config: Config,
    bridge: Arc<dyn Bridge>,
    reviewer: Arc<AppHost>,
    mcp_host: Arc<AppHost>,
    local_host: Arc<AppHost>,
    token: String,
    session_mutations: Arc<Mutex<()>>,
    operations: Arc<Operations>,
    pools: Arc<Mutex<BTreeMap<String, Arc<Pool>>>>,
}
fn recovered_phase(phase: &str) -> &str {
    if matches!(phase, "completed" | "interrupted" | "stopped") {
        phase
    } else {
        "reconciliation_required"
    }
}
fn observed_owner_requires_reconciliation(
    snapshot: &Record,
    current: &Record,
    observed: &Owner,
) -> Result<bool, Error> {
    if current.execution == *observed {
        return Ok(false);
    }
    if current.execution != snapshot.execution || current.generation != snapshot.generation {
        return Err(Error("native_observation_superseded"));
    }
    Ok(true)
}
fn new_native_owner_allowed(record: &Record, turn: Option<&str>) -> bool {
    turn.is_some() && turn != record.native_turn.as_deref() && record.pending.is_empty()
}
fn same_controller(snapshot: &Record, current: &Record) -> bool {
    snapshot.execution == current.execution
        && snapshot.generation == current.generation
        && snapshot.capability == current.capability
}
fn same_native_round(snapshot: &Record, current: &Record) -> bool {
    same_controller(snapshot, current) && snapshot.native_turn == current.native_turn
}
fn driver_round_error(current: &Record) -> Option<Error> {
    // A deliberate Chat handoff rotates the controller within this native
    // response. Its new capability alone must not cancel that response.
    if matches!(current.phase.as_str(), "stop_requested" | "stopped") {
        Some(Error("operator_controller_fenced"))
    } else if current.driver_phase == "start_unknown" {
        Some(Error("operator_app_start_unconfirmed"))
    } else if current.phase == "reconciliation_required" {
        Some(Error("operator_reconciliation_required"))
    } else {
        None
    }
}
fn admit_app_round(work: &Work, snapshot: &Record, turn: Option<&str>) -> Result<(), Error> {
    // turn/start can publish its reserved turn ID while the model request is
    // entering. That receipt is the same controller's request, not a new writer.
    let start_returned = snapshot.phase == "starting_native"
        && work.record.phase == "starting_native"
        && turn.is_some()
        && work.record.native_turn.as_deref() == turn
        && same_controller(snapshot, &work.record);
    if !same_native_round(snapshot, &work.record) && !start_returned
        || work.stops_in_progress != 0
        || work.record.phase == "stop_requested"
        || work.record.phase == "stopped"
            && (turn.is_none() || turn == work.record.native_turn.as_deref())
    {
        return Err(Error("operator_controller_fenced"));
    }
    if work.app_cancel.is_some() {
        return Err(Error("native_lane_busy"));
    }
    Ok(())
}

fn app_lane_requires_host_stop(lane: &Value) -> bool {
    if lane["pending_native_calls"].as_u64() != Some(0) {
        return true;
    }
    if lane["phase"] == "stopped" {
        return false;
    }
    lane["phase"] != "idle"
        || lane["app_dispatches"].as_u64() != Some(0)
        || lane["user_role_objects"].as_u64() != Some(0)
        || lane["tool_return_objects"].as_u64() != Some(0)
        || lane["public_event_count"].as_u64() != Some(0)
        || !lane["conversation_id"].is_null()
}
fn controller_matches(record: &Record, args: &Value) -> bool {
    args["controller"].as_str() == Some(&record.capability)
        && !record
            .selection
            .as_ref()
            .is_some_and(|s| s.reasoning == Reasoning::Provider)
        && record.accepted
        && matches!(
            record.phase.as_str(),
            "running" | "waiting_native" | "native_summary"
        )
}
fn same_assignment_parent(left: Option<&str>, right: Option<&str>) -> bool {
    left.is_some() && left == right
}

impl Operator {
    pub fn new(
        pool: Arc<Pool>,
        broker: Arc<Broker>,
        journal: Arc<Journal>,
        config: Config,
        bridge: Arc<dyn Bridge>,
        token: String,
    ) -> Result<Arc<Self>, Error> {
        let mut works = BTreeMap::new();
        for id in recover_work_index(&journal)?.ids {
            let key = digest(&("operator-work-v1", &id));
            let value: Value = journal
                .read(&key)?
                .ok_or(Error("operator_record_missing"))?;
            if value["schema"] != WORK_SCHEMA {
                return Err(Error("reasoning_migration_required"));
            }
            let mut record: Record =
                serde_json::from_value(value).map_err(|_| Error("operator_record_invalid"))?;
            if record.id != id || works.len() >= 256 {
                return Err(Error("operator_record_invalid"));
            }
            // A new service never inherits an old controller or retransmits a call.
            record.capability = random_id()?;
            record.accepted = false;
            record.phase = recovered_phase(&record.phase).to_owned();
            if works.contains_key(&record.thread) {
                return Err(Error("operator_record_invalid"));
            }
            // The local CLI resolves the current controller from this record.
            // Publish the same rotation held in memory before accepting calls.
            journal.write(&key, &record)?;
            works.insert(
                record.thread.clone(),
                Work {
                    record,
                    active: None,
                    request: None,
                    round: String::new(),
                    checkpoint: None,
                    driver_lock: Arc::new(Mutex::new(())),
                    native_admission: Arc::new(Mutex::new(())),
                    stops_in_progress: 0,
                    app_cancel: None,
                    completion_observer: None,
                },
            );
        }
        let works = Arc::new(Mutex::new(works));
        let bridge: Arc<dyn Bridge> = Arc::new(LimitedBridge {
            remote: bridge,
            works: works.clone(),
        });
        let local_bridge: Arc<dyn Bridge> = Arc::new(crate::app::Transport {
            remote: bridge.clone(),
            path: ReturnPath::LocalFunction,
        });
        let mcp_bridge: Arc<dyn Bridge> = Arc::new(crate::mcp_bridge::McpBridge::new(
            Arc::new(crate::app::Transport {
                remote: bridge.clone(),
                path: ReturnPath::Mcp,
            }),
            broker.clone(),
            journal.clone(),
            config.transport.connector_name.clone(),
        )?);
        let mcp_host = Arc::new(
            AppHost::new(mcp_bridge, journal.clone())
                .with_connector(config.transport.connector_name.clone()),
        );
        let local_host = Arc::new(
            AppHost::new(local_bridge, journal.clone())
                .with_connector(config.transport.connector_name.clone()),
        );
        let reviewer = if config.transport.return_path == ReturnPath::Mcp {
            mcp_host.clone()
        } else {
            local_host.clone()
        };
        Ok(Arc::new(Self {
            pool,
            broker: broker.clone(),
            reviewer,
            mcp_host,
            local_host,
            journal,
            works,
            config,
            bridge,
            token,
            session_mutations: Arc::new(Mutex::new(())),
            operations: Arc::new(Operations::default()),
            pools: Arc::new(Mutex::new(BTreeMap::new())),
        }))
    }
    async fn scoped_pool(
        &self,
        home: &std::path::Path,
        workspace: &std::path::Path,
    ) -> Result<Arc<Pool>, Error> {
        if home == self.pool.home && workspace == self.pool.workspace {
            return Ok(self.pool.clone());
        }
        for path in [home, workspace] {
            if !path.is_absolute()
                || std::fs::canonicalize(path).ok().as_deref() != Some(path)
                || !path.is_dir()
            {
                return Err(Error("native_workspace_identity_invalid"));
            }
        }
        let key = digest(&(home, workspace, &self.config));
        let mut pools = self.pools.lock().await;
        if let Some(pool) = pools.get(&key) {
            return Ok(pool.clone());
        }
        let mut config = self.config.clone();
        config.execution.home = Some(home.to_owned());
        config.execution.workspace = Some(workspace.to_owned());
        let pool = self.pool.scoped(config)?;
        pools.insert(key, pool.clone());
        Ok(pool)
    }
    async fn work_pool(&self, record: &Record) -> Result<Arc<Pool>, Error> {
        self.scoped_pool(
            &record.execution.home,
            record.workspace.as_deref().unwrap_or(&self.pool.workspace),
        )
        .await
    }
    fn app_host(&self, path: ReturnPath) -> Arc<AppHost> {
        if path == ReturnPath::Mcp {
            self.mcp_host.clone()
        } else {
            self.local_host.clone()
        }
    }
    fn app_route(&self, record: &Record, turn: Option<&str>) -> Result<Option<ReturnPath>, Error> {
        if self.mcp_host.source_turn_pending(&record.thread, turn)? {
            return Ok(Some(ReturnPath::Mcp));
        }
        Ok(record.selection.as_ref().and_then(|selection| {
            (selection.reasoning == Reasoning::Provider).then_some(selection.return_path)
        }))
    }
    async fn admit_native_call(&self, thread: &str, call_id: &str) -> Result<(), Error> {
        let mut works = self.works.lock().await;
        let work = works
            .get_mut(thread)
            .ok_or(Error("operator_work_unavailable"))?;
        if matches!(work.record.phase.as_str(), "stop_requested" | "stopped")
            || work.stops_in_progress != 0
        {
            return Err(Error("operator_controller_fenced"));
        }
        let generation = work.record.generation;
        if let Some(assignment) = &mut work.record.assignment {
            let id = digest(&(generation, call_id));
            if assignment.admitted_calls.contains(&id) {
                return Ok(());
            }
            if assignment.admitted_calls.len() >= assignment.budget.max_native_calls {
                return Err(Error("assignment_native_budget_exhausted"));
            }
            if assignment.started_at_unix_ms.is_some_and(|start| {
                unix_ms().saturating_sub(start)
                    >= assignment.budget.max_elapsed_seconds.saturating_mul(1000)
            }) {
                return Err(Error("assignment_time_budget_exhausted"));
            }
            assignment.admitted_calls.insert(id);
            self.persist(&work.record)?;
        }
        Ok(())
    }
    async fn wait_assignment(&self, record: &Record) -> Result<(), Error> {
        let Some(assignment) = &record.assignment else {
            return Ok(());
        };
        loop {
            let mut works = self.works.lock().await;
            let current = works
                .get(&record.thread)
                .ok_or(Error("operator_work_unavailable"))?;
            if current.record.capability != record.capability
                || current.stops_in_progress != 0
                || matches!(
                    current.record.phase.as_str(),
                    "stop_requested" | "stopped" | "reconciliation_required"
                )
            {
                return Err(Error("operator_controller_fenced"));
            }
            let mut dependencies_ready = true;
            for id in &assignment.dependencies {
                let dependency = works
                    .values()
                    .find(|work| &work.record.id == id)
                    .ok_or(Error("assignment_dependency_unavailable"))?;
                let evaluation = dependency
                    .record
                    .assignment
                    .as_ref()
                    .and_then(|a| a.evaluation.as_ref());
                if evaluation.is_some_and(|v| {
                    matches!(
                        v["decision"].as_str(),
                        Some("rejected" | "changes_requested")
                    )
                }) {
                    return Err(Error("assignment_dependency_not_accepted"));
                }
                dependencies_ready &= dependency.record.phase == "completed"
                    && evaluation.is_some_and(|v| v["decision"] == "accepted");
            }
            let active = works
                .values()
                .filter(|work| {
                    work.record.id != record.id
                        && work.record.assignment.as_ref().is_some_and(|a| {
                            same_assignment_parent(
                                a.parent.as_deref(),
                                assignment.parent.as_deref(),
                            )
                        })
                        && (work.record.native_turn.is_some() || work.record.phase == "admitting")
                        && !matches!(
                            work.record.phase.as_str(),
                            "completed" | "stopped" | "queued"
                        )
                })
                .count();
            if dependencies_ready && active < assignment.budget.max_parallel {
                let work = works.get_mut(&record.thread).unwrap();
                work.record.phase = "admitting".into();
                work.record
                    .assignment
                    .as_mut()
                    .unwrap()
                    .started_at_unix_ms
                    .get_or_insert_with(unix_ms);
                self.persist(&work.record)?;
                return Ok(());
            }
            let work = works.get_mut(&record.thread).unwrap();
            let current = work.record.assignment.as_mut().unwrap();
            let start = *current.started_at_unix_ms.get_or_insert_with(unix_ms);
            if unix_ms().saturating_sub(start)
                >= current.budget.max_elapsed_seconds.saturating_mul(1000)
            {
                return Err(Error("assignment_time_budget_exhausted"));
            }
            if work.record.phase != "queued" {
                work.record.phase = "queued".into();
                self.persist(&work.record)?;
            }
            drop(works);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    fn persist(&self, record: &Record) -> Result<(), Error> {
        self.journal
            .write(&digest(&("operator-work-v1", &record.id)), record)
    }
    async fn register_in(
        &self,
        thread: &str,
        origin: &str,
        request: Option<&Request>,
        pool: Arc<Pool>,
        selection: Option<Selection>,
        assignment: Option<Assignment>,
    ) -> Result<Record, Error> {
        if let Some(work) = self.works.lock().await.get(thread) {
            return Ok(work.record.clone());
        }
        let native = pool.for_thread(thread).await?;
        if selection.is_some() {
            native.verify_scope(thread, &pool.workspace).await?;
        }
        let execution = native.owner.clone();
        let mut works = self.works.lock().await;
        if let Some(work) = works.get(thread) {
            return Ok(work.record.clone());
        }
        if works.len() >= 256 {
            return Err(Error("operator_work_limit"));
        }
        let id = random_id()?;
        let token = self
            .broker
            .register(
                &id,
                request.map_or_else(BTreeMap::new, |v| v.tools.clone()),
                false,
            )
            .await?;
        let record = Record {
            schema: WORK_SCHEMA.into(),
            id,
            thread: thread.into(),
            execution,
            origin: origin.into(),
            native_turn: request.and_then(|request| request.turn.clone()),
            generation: 1,
            capability: random_id()?,
            token,
            phase: "waiting_native".into(),
            pending: vec![],
            receipts: BTreeMap::new(),
            summary: None,
            source: None,
            source_context: None,
            accepted: false,
            driver: None,
            driver_phase: "none".into(),
            identity: None,
            host_observed_at: None,
            host_last_state: "unknown".into(),
            observed: None,
            observed_context: None,
            policy: None,
            selection,
            workspace: Some(pool.workspace.clone()),
            assignment,
        };
        let mut index = self
            .journal
            .read::<WorkIndex>(&work_index_key())?
            .ok_or(Error("operator_record_missing"))?;
        if index.pending.is_some() {
            return Err(Error("operator_registration_reconciliation_required"));
        }
        index.pending = Some(record.clone());
        commit_work_index(&self.journal, &index)?;
        self.persist(&record)?;
        works.insert(
            thread.into(),
            Work {
                record: record.clone(),
                active: None,
                request: None,
                round: String::new(),
                checkpoint: None,
                driver_lock: Arc::new(Mutex::new(())),
                native_admission: Arc::new(Mutex::new(())),
                stops_in_progress: 0,
                app_cancel: None,
                completion_observer: None,
            },
        );
        index.ids.push(record.id.clone());
        index.pending = None;
        commit_work_index(&self.journal, &index)?;
        Ok(record)
    }
    async fn binding(
        &self,
        args: &Value,
        require_controller: bool,
    ) -> Result<(Record, Arc<Native>), Error> {
        let works = self.works.lock().await;
        let work = works
            .values()
            .find(|v| Some(v.record.id.as_str()) == args["work_id"].as_str())
            .ok_or(Error("operator_work_unavailable"))?;
        let mut record = work.record.clone();
        if args.get("controller").is_some()
            && args["controller"].as_str() != Some(&record.capability)
        {
            return Err(Error("operator_controller_fenced"));
        }
        if require_controller && !controller_matches(&record, args) {
            return Err(Error("operator_controller_fenced"));
        }
        if require_controller
            && self
                .app_route(&record, record.native_turn.as_deref())?
                .is_some()
        {
            return Err(Error("app_response_owns_execution"));
        }
        drop(works);
        if require_controller {
            self.verify_identity(&record).await?;
        }
        let pool = self.work_pool(&record).await?;
        let selected = if record.origin == "codex" {
            self.pool.discover_thread(&record.thread).await
        } else {
            pool.for_thread(&record.thread).await
        };
        let native = match selected {
            Ok(native) => native,
            Err(Error("native_thread_owner_unavailable"))
                if matches!(
                    record.phase.as_str(),
                    "reconciliation_required" | "completed" | "stop_requested" | "stopped"
                ) =>
            {
                pool.prepare().await?
            }
            Err(error) => return Err(error),
        };
        if record.selection.is_some() {
            if native.owner == record.execution {
                native.verify_scope(&record.thread, &pool.workspace).await?;
            } else {
                // Read-only observers may see a native account switch before
                // its first new turn. Keep control fenced until reconciliation.
                native.workspace(&record.thread).await?;
            }
        }
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            // Discovery awaits native reads. A new native round may already
            // have reconciled this owner while this observer held its snapshot.
            // Never fence that newer round using an obsolete execution binding.
            if (args.get("controller").is_some()
                && args["controller"].as_str() != Some(&work.record.capability))
                || require_controller && !controller_matches(&work.record, args)
            {
                return Err(Error("operator_controller_fenced"));
            }
            if observed_owner_requires_reconciliation(&record, &work.record, &native.owner)? {
                work.record.accepted = false;
                work.record.phase = recovered_phase(&work.record.phase).to_owned();
                self.persist(&work.record)?;
                if require_controller {
                    return Err(Error("native_owner_reconciliation_required"));
                }
            }
            record = work.record.clone();
        }
        Ok((record, native))
    }

    async fn admit_native_round(
        &self,
        record: &Record,
        request: &Request,
        native: &Native,
        pool: &Pool,
    ) -> Result<(), Error> {
        native.verify_scope(&record.thread, &pool.workspace).await?;
        let owner_changed = native.owner != record.execution
            || record.workspace.as_deref() != Some(pool.workspace.as_path());
        let source = self.mcp_host.pending_external(&record.thread)?;
        let new_input = request.compaction.is_none()
            && request.turn.is_some()
            && request.turn != record.native_turn;
        let select_origin =
            new_input && record.phase != "starting_native" && record.selection.is_some();
        if !owner_changed && !select_origin && source.is_none() {
            return Ok(());
        }
        if owner_changed && !new_native_owner_allowed(record, request.turn.as_deref()) {
            return Err(Error("native_owner_reconciliation_required"));
        }
        // The request's claimed turn must be the actual canonical in-progress
        // turn. Historical work origin is not authority for a later CLI input.
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":true}),
            )
            .await?;
        let turn = state["thread"]["turns"]
            .as_array()
            .and_then(|turns| turns.last())
            .ok_or(Error("native_round_not_ready"))?;
        if turn["id"].as_str() != request.turn.as_deref() || turn["status"] != "inProgress" {
            return Err(Error("native_round_not_ready"));
        }
        if let Some((source, call_id)) = &source {
            let client_id = queue_client_id(&record.thread, call_id);
            let consumed = Self::history(native, &record.thread)
                .await?
                .into_iter()
                .any(|row| {
                    row["turn_id"].as_str() == request.turn.as_deref()
                        && row["item"]["type"] == "userMessage"
                        && row["item"]["clientId"] == client_id
                        && context::text(&row["item"])
                            .is_ok_and(|text| digest(&text) == source.fingerprint)
                });
            if !consumed {
                return Err(Error("bound_chat_input_not_consumed"));
            }
            let (account_ref, source_anchor) =
                self.mcp_host.external_origin(&record.thread, source)?;
            let admission = SourceAdmission {
                schema: SOURCE_ADMISSION_SCHEMA.into(),
                work_id: record.id.clone(),
                thread_id: record.thread.clone(),
                turn_id: request
                    .turn
                    .clone()
                    .ok_or(Error("bound_chat_native_turn_missing"))?,
                account_ref,
                source: source.clone(),
                source_anchor,
                client_user_message_id: client_id,
                execution_home: native.owner.home.clone(),
                workspace: pool.workspace.clone(),
            };
            let key = source_admission_key(&record.id, &admission.turn_id);
            if self
                .journal
                .read::<SourceAdmission>(&key)?
                .is_some_and(|previous| previous != admission)
            {
                return Err(Error("source_admission_conflict"));
            }
            // This records consumed canonical input, not model participation or
            // tool execution. Persist before binding so a crash cannot lose the
            // original IDs after the active AppHost slot advances.
            self.journal.write(&key, &admission)?;
            self.mcp_host
                .bind_external(
                    &record.thread,
                    source,
                    request
                        .turn
                        .as_deref()
                        .ok_or(Error("bound_chat_native_turn_missing"))?,
                )
                .await?;
        }
        if owner_changed {
            self.recovery_receipts(native, record).await?;
        }
        let source_bound = self
            .mcp_host
            .source_turn_pending(&record.thread, request.turn.as_deref())?;
        let mut works = self.works.lock().await;
        let work = works
            .get_mut(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?;
        if work.record.execution != record.execution
            || work.record.generation != record.generation
            || work.record.capability != record.capability
            || work.stops_in_progress != 0
            || work.record.phase == "stop_requested"
            || work.active.is_some()
            || work.app_cancel.is_some()
            || !work.record.pending.is_empty()
        {
            return Err(Error("native_owner_reconciliation_required"));
        }
        let selection = work
            .record
            .selection
            .as_ref()
            .filter(|_| select_origin && work.record.phase != "starting_native")
            .map(|previous| Selection {
                reasoning: if source_bound {
                    Reasoning::Direct
                } else {
                    Reasoning::Provider
                },
                return_path: if source_bound {
                    ReturnPath::Mcp
                } else {
                    previous.return_path
                },
            });
        let route_changed = selection.as_ref().is_some_and(|selected| {
            work.record.selection.as_ref().is_none_or(|old| {
                old.reasoning != selected.reasoning || old.return_path != selected.return_path
            })
        });
        if !owner_changed && !route_changed {
            return Ok(());
        }
        self.preserve_controller(&work.record)?;
        self.broker.retire(&work.record.token, true).await?;
        work.record.token = self
            .broker
            .register(&record.id, request.tools.clone(), false)
            .await?;
        work.record.capability = random_id()?;
        work.record.generation += 1;
        work.record.execution = native.owner.clone();
        work.record.workspace = Some(pool.workspace.clone());
        work.record.accepted = false;
        if let Some(selection) = selection {
            work.record.selection = Some(selection);
        }
        if work
            .record
            .selection
            .as_ref()
            .is_some_and(|selection| selection.reasoning == Reasoning::Provider)
        {
            work.record.phase = "waiting_native".into();
        }
        self.persist(&work.record)
    }
    async fn verify_identity(&self, record: &Record) -> Result<Value, Error> {
        let Some(lane) = &record.driver else {
            return Ok(Value::Null);
        };
        let current = tokio::time::timeout(
            Duration::from_secs(10),
            self.bridge.operation(json!({"op":"identity","lane":lane})),
        )
        .await
        .map_err(|_| Error("app_identity_unavailable"))??;
        crate::app::Identity::parse(&current)?;
        let fingerprint = digest(&(
            &current["generation"],
            &current["account"],
            &current["user"],
            &current["structure"],
        ));
        if record.identity.as_ref() != Some(&fingerprint) {
            return Err(Error("app_account_changed"));
        }
        Ok(current)
    }
    async fn recovery_receipts(
        &self,
        native: &Native,
        record: &Record,
    ) -> Result<Vec<Value>, Error> {
        let history = Self::history(native, &record.thread).await?;
        let calls = self.broker.recorded_calls(&record.token)?;
        let mut receipts = vec![];
        for call in calls
            .iter()
            .filter(|call| call["offered"] == true && call["settled"] == false)
        {
            let item = history
                .iter()
                .find(|row| row["item"]["id"] == call["id"])
                .ok_or(Error("operator_unreturned_calls_require_reconciliation"))?;
            receipts.push(item.clone());
        }
        if !self.owned_jobs(native, record).await?.is_empty() {
            return Err(Error("operator_background_jobs_unsettled"));
        }
        Ok(receipts)
    }
    fn preserve_controller(&self, record: &Record) -> Result<(), Error> {
        self.journal.write(
            &digest(&(
                "operator-previous-controller-v1",
                &record.id,
                record.generation,
            )),
            record,
        )
    }
    async fn owned_jobs(&self, native: &Native, record: &Record) -> Result<Vec<Value>, Error> {
        let owned = self.broker.recorded_calls(&record.token)?;
        let app_owned = self
            .app_route(record, record.native_turn.as_deref())?
            .is_some();
        let mut cursor = Value::Null;
        let mut jobs = vec![];
        for page in 0..32 {
            let response = native
                .call(
                    "thread/backgroundTerminals/list",
                    json!({"threadId":record.thread,"limit":100,"cursor":cursor}),
                )
                .await?;
            let data = response["data"]
                .as_array()
                .ok_or(Error("native_jobs_unobservable"))?;
            jobs.extend(
                data.iter()
                    .filter(|job| app_owned || owned.iter().any(|call| call["id"] == job["itemId"]))
                    .cloned(),
            );
            cursor = response["nextCursor"].clone();
            if cursor.is_null() {
                return Ok(jobs);
            }
            if page == 31 {
                return Err(Error("native_jobs_unobservable"));
            }
        }
        Err(Error("native_jobs_unobservable"))
    }
    async fn stop_driver(&self, record: &Record) -> Result<bool, Error> {
        if let Some(path) = self.app_route(record, record.native_turn.as_deref())? {
            return self.app_host(path).stop(&record.thread).await;
        }
        let driver_lock = self
            .works
            .lock()
            .await
            .get(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?
            .driver_lock
            .clone();
        let _guard = tokio::time::timeout(Duration::from_secs(15), driver_lock.lock())
            .await
            .map_err(|_| Error("app_stop_unconfirmed"))?;
        let Some(lane) = &record.driver else {
            return Ok(record.driver_phase == "released_after_stop");
        };
        self.bridge
            .operation(json!({"op":"stop","lane":lane}))
            .await?;
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let snapshot = self
                    .bridge
                    .operation(json!({"op":"snapshot","lane":lane}))
                    .await?;
                if snapshot["stopped"] == true {
                    return Ok::<_, Error>(true);
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|_| Error("app_stop_unconfirmed"))?
    }
    async fn history(native: &Native, thread: &str) -> Result<Vec<Value>, Error> {
        let mut cursor = Value::Null;
        let mut result = vec![];
        let mut bytes = 0;
        for _ in 0..4096 {
            let response = native
                .call(
                    "thread/items/list",
                    json!({"threadId":thread,"limit":100,"cursor":cursor,"sortDirection":"asc"}),
                )
                .await?;
            let items = response["data"]
                .as_array()
                .ok_or(Error("native_history_unavailable"))?;
            for row in items {
                let item = &row["item"];
                if !row["turnId"].is_string() || !item["type"].is_string() {
                    return Err(Error("native_history_unavailable"));
                }
                if item["type"] == "reasoning"
                    || item["status"]
                        .as_str()
                        .is_some_and(|v| matches!(v, "inProgress" | "in_progress" | "running"))
                {
                    continue;
                }
                let value = json!({"turn_id":row["turnId"],"item":item});
                bytes += value.to_string().len();
                if bytes > 64 * 1024 * 1024 {
                    return Err(Error("native_history_read_bound"));
                }
                result.push(value);
            }
            let next = response["nextCursor"].clone();
            if next.is_null() {
                return Ok(result);
            }
            if next == cursor {
                return Err(Error("native_history_cursor_repeated"));
            }
            cursor = next;
        }
        Err(Error("native_history_read_bound"))
    }
    fn cursor(history: &[Value]) -> Cursor {
        Cursor {
            count: history.len(),
            digest: digest(&history),
        }
    }
    fn covered(history: &[Value], cursor: &Cursor) -> bool {
        history
            .get(..cursor.count)
            .is_some_and(|prefix| digest(&prefix) == cursor.digest)
    }
    fn development_binding(&self, record: &Record) -> Result<Option<DevelopmentBinding>, Error> {
        let binding: Option<DevelopmentBinding> =
            self.journal.read(&development_binding_key(&record.id))?;
        if binding.as_ref().is_some_and(|binding| {
            binding.schema != DEVELOPMENT_BINDING_SCHEMA
                || binding.work_id != record.id
                || binding.thread_id != record.thread
        }) {
            return Err(Error("development_binding_conflict"));
        }
        Ok(binding)
    }
    /// An owned AppHost source establishes the connection. Explicit detachment
    /// wins over later observation; only an owned successor advances its source.
    async fn ensure_development_binding(
        &self,
        record: &Record,
    ) -> Result<Option<DevelopmentBinding>, Error> {
        let works = self.works.lock().await;
        let work = works
            .get(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?;
        if !same_native_round(record, &work.record)
            || work.stops_in_progress != 0
            || matches!(work.record.phase.as_str(), "stop_requested" | "stopped")
        {
            return Err(Error("bound_chat_observation_superseded"));
        }
        let old = self.development_binding(record)?;
        if old.as_ref().is_some_and(|binding| !binding.enabled) {
            return Ok(old);
        }
        if !record
            .selection
            .as_ref()
            .is_some_and(|selection| selection.return_path == ReturnPath::Mcp)
        {
            if let Some(mut binding) = old {
                let blocked = SourceObservation::Blocked {
                    reason: "source_observation_transport_unavailable".into(),
                };
                if binding.last_observation != blocked {
                    binding.last_observation = blocked;
                    self.journal
                        .write(&development_binding_key(&record.id), &binding)?;
                }
                return Ok(Some(binding));
            }
            return Ok(None);
        }
        let Some(source) = self.mcp_host.source_connection(&record.thread)? else {
            return Ok(old);
        };
        if let Some(binding) = &old {
            if binding.account_ref == source.account_ref
                && binding.conversation_id == source.conversation
            {
                return Ok(old);
            }
            if !source.previous_chats.contains(&binding.conversation_id) {
                return Err(Error("development_source_changed"));
            }
        }
        let binding = DevelopmentBinding {
            schema: DEVELOPMENT_BINDING_SCHEMA.into(),
            work_id: record.id.clone(),
            thread_id: record.thread.clone(),
            account_ref: source.account_ref,
            conversation_id: source.conversation,
            enabled: true,
            last_observation: SourceObservation::Observing,
            source_selection_required: false,
        };
        self.journal
            .write(&development_binding_key(&record.id), &binding)?;
        Ok(Some(binding))
    }
    async fn source_retry_seconds(&self) -> Result<u64, Error> {
        let status = self.bridge.operation(json!({"op":"host_status"})).await?;
        if status["saved_read_backoff"]["cooling_down"] == true {
            status["saved_read_backoff"]["retry_after_seconds"]
                .as_u64()
                .ok_or(Error("app_chat_response_invalid"))
        } else {
            Ok(RECONCILE_INTERVAL.as_secs())
        }
    }
    async fn save_source_observation(
        &self,
        record: &Record,
        observation: SourceObservation,
    ) -> Result<(), Error> {
        let works = self.works.lock().await;
        if works
            .get(&record.thread)
            .is_none_or(|work| !same_native_round(record, &work.record))
        {
            return Ok(());
        }
        let Some(mut binding) = self.development_binding(record)? else {
            return Ok(());
        };
        let selection_required = match &observation {
            SourceObservation::Observing => false,
            SourceObservation::Blocked { reason }
                if matches!(
                    reason.as_str(),
                    "bound_chat_branch_changed" | "source_selection_required"
                ) =>
            {
                true
            }
            _ => binding.source_selection_required,
        };
        if binding.enabled
            && (binding.last_observation != observation
                || binding.source_selection_required != selection_required)
        {
            binding.source_selection_required = selection_required;
            binding.last_observation = observation;
            self.journal
                .write(&development_binding_key(&record.id), &binding)?;
        }
        Ok(())
    }
    async fn source_observation_failed(&self, record: &Record, error: &Error) {
        let observation = if source_observation_pending(error) {
            match self.source_retry_seconds().await {
                Ok(seconds) => SourceObservation::RetryWait {
                    reason: error.0.into(),
                    retry_at_unix_ms: unix_ms().saturating_add(seconds.saturating_mul(1000)),
                },
                Err(readiness) => {
                    eprintln!("source_readiness_failed_after_{}: {}", error.0, readiness.0);
                    SourceObservation::Blocked {
                        reason: readiness.0.into(),
                    }
                }
            }
        } else {
            SourceObservation::Blocked {
                reason: error.0.into(),
            }
        };
        if let Err(error) = self.save_source_observation(record, observation).await {
            eprintln!("source_observation_save_failed: {}", error.0);
        }
    }
    async fn set_development_binding(
        &self,
        args: Value,
        receipt: Option<&(String, String)>,
    ) -> Result<Value, Error> {
        let (record, native) = self.binding(&args, false).await?;
        if args["controller"].as_str() != Some(&record.capability) || record.phase != "completed" {
            return Err(Error("native_writer_unsettled"));
        }
        let pool = self.work_pool(&record).await?;
        native.verify_scope(&record.thread, &pool.workspace).await?;
        let history = Self::history(&native, &record.thread).await?;
        let context = native.public_context(&record.thread).await?;
        if args["cursor"] != json!(Self::cursor(&history))
            || args["context_cursor"] != json!(Self::cursor(&context))
        {
            return Err(Error("native_history_boundary_changed"));
        }
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":true}),
            )
            .await?;
        if !source_native_settled(&record, &state)
            || !state["thread"]["turns"]
                .as_array()
                .and_then(|turns| turns.last())
                .is_some_and(|turn| settled_source_turn(&record, turn))
            || !self.owned_jobs(&native, &record).await?.is_empty()
        {
            return Err(Error("native_writer_unsettled"));
        }
        let enabled = args["action"] == "bind";
        let mut binding = self
            .ensure_development_binding(&record)
            .await?
            .ok_or(Error("source_observation_unavailable"))?;
        if enabled {
            if !self.mcp_host.has_retained_source(&record.thread).await? {
                return Err(Error("source_observation_unavailable"));
            }
            let source = self
                .mcp_host
                .source_connection(&record.thread)?
                .ok_or(Error("source_observation_unavailable"))?;
            if args["conversation_id"].as_str() != Some(source.conversation.as_str()) {
                return Err(Error("development_source_changed"));
            }
            let page = crate::chats::Chats::new(self.bridge.clone())
                .read_source(json!({"action":"read",
                "conversation_id":source.conversation,"after":args["source_cursor"],"limit":1}))
                .await?;
            if page["cursor"] != args["source_cursor"]
                || page["complete"] != true
                || page["account"] != source.account_ref
            {
                return Err(Error("chat_source_boundary_changed"));
            }
            binding.account_ref = source.account_ref;
            binding.conversation_id = source.conversation;
        }
        let mut works = self.works.lock().await;
        let work = works
            .get_mut(&record.thread)
            .ok_or(Error("operator_controller_fenced"))?;
        if !same_native_round(&record, &work.record)
            || work.record.phase != "completed"
            || work.stops_in_progress != 0
            || work.app_cancel.is_some()
            || work.active.is_some()
        {
            return Err(Error("operator_controller_fenced"));
        }
        self.mutation_intent(receipt)?;
        binding.enabled = enabled;
        self.journal
            .write(&development_binding_key(&record.id), &binding)?;
        if let Some(previous) = work.completion_observer.take() {
            previous.cancel();
        }
        Ok(json!({"binding":binding,"inference_submitted":false}))
    }
    fn source_participation(&self, record: &Record, turn: &str) -> Result<Value, Error> {
        let Some(lease) = crate::mcp_bridge::source_lease(&self.journal, &record.thread, turn)?
        else {
            return Ok(Value::Null);
        };
        if self
            .source_admission(record, turn)?
            .is_none_or(|admission| admission.source != lease.input)
        {
            return Err(Error("source_lease_conflict"));
        }
        self.broker.source_participation(&lease.token)
    }
    async fn join_verified_source(&self, args: Value) -> Result<Value, Error> {
        let pending = |reason: &str| {
            json!({"status":"waiting","reason":reason,
            "retry_after_seconds":RECONCILE_INTERVAL.as_secs(),"inference_submitted":false})
        };
        let conversation = text(&args, "conversation_id", 256)?;
        text(&args, "call_id", 160)?;
        let (mut record, native) = self.binding(&args, false).await?;
        let binding = self
            .ensure_development_binding(&record)
            .await?
            .filter(|binding| binding.enabled)
            .ok_or(Error("development_source_detached"))?;
        if binding.conversation_id != conversation {
            return Err(Error("development_source_changed"));
        }
        if matches!(record.phase.as_str(), "completed" | "interrupted") {
            if let Some(selected) = args.get("source_message_id").and_then(Value::as_str) {
                if record
                    .native_turn
                    .as_deref()
                    .map(|turn| self.source_admission(&record, turn))
                    .transpose()?
                    .flatten()
                    .is_some_and(|admission| admission.source.message == selected)
                {
                    return Err(Error("source_response_ended"));
                }
            }
        }
        if self
            .source_admission(&record, record.native_turn.as_deref().unwrap_or(""))?
            .is_none()
            || matches!(record.phase.as_str(), "completed" | "interrupted")
        {
            let ready = self.synchronize_source(json!({"thread_id":record.thread,"source_message_id":args.get("source_message_id")})).await?;
            record = self
                .works
                .lock()
                .await
                .get(&record.thread)
                .ok_or(Error("operator_work_unavailable"))?
                .record
                .clone();
            if record.phase != "running" {
                if ready["status"] == "waiting" {
                    return Ok(json!({"status":"waiting","reason":ready["reason"],
                        "retry_after_seconds":ready["retry_after_seconds"],"inference_submitted":false}));
                }
                if record
                    .native_turn
                    .as_deref()
                    .map(|turn| self.source_admission(&record, turn))
                    .transpose()?
                    .flatten()
                    .is_some()
                {
                    return Err(Error("source_response_ended"));
                }
                return Ok(
                    json!({"status":"waiting","reason":ready.get("reason").and_then(Value::as_str).unwrap_or("source_admission_pending"),
                    "retry_after_seconds":ready.get("retry_after_seconds").and_then(Value::as_u64).unwrap_or(RECONCILE_INTERVAL.as_secs()),
                    "inference_submitted":false}),
                );
            }
        }
        let Some(turn) = record.native_turn.as_deref() else {
            return Ok(pending("source_admission_pending"));
        };
        let Some(admission) = self.source_admission(&record, turn)? else {
            return Ok(pending("source_admission_pending"));
        };
        if admission.account_ref != binding.account_ref
            || admission.source.conversation != conversation
        {
            return Err(Error("development_source_changed"));
        }
        let explicit = args.get("source_message_id").is_some();
        if explicit && args["source_message_id"] != admission.source.message {
            return Err(Error("source_selection_required"));
        }
        let source = match crate::bound_chat::read(
            self.bridge.clone(),
            conversation,
            &admission.source_anchor,
            Some(&admission.source),
            !explicit,
            None,
        )
        .await {
            Err(error) if source_observation_pending(&error) => {
                return Ok(json!({"status":"waiting","reason":error.0,
                    "retry_after_seconds":self.source_retry_seconds().await?,"inference_submitted":false}));
            }
            other => other?,
        }
        .ok_or(Error("bound_chat_input_missing"))?;
        if source.account_ref != admission.account_ref {
            return Err(Error("app_account_changed"));
        }
        if source.end != crate::bound_chat::End::Active {
            return Err(Error("source_response_ended"));
        }
        let pool = self.work_pool(&record).await?;
        native.verify_scope(&record.thread, &pool.workspace).await?;
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":true}),
            )
            .await?;
        if !state["thread"]["turns"]
            .as_array()
            .and_then(|turns| turns.last())
            .is_some_and(|latest| latest["id"] == turn && latest["status"] == "inProgress")
        {
            return Err(Error("source_native_round_unavailable"));
        }
        let Some(lease) = crate::mcp_bridge::source_lease(&self.journal, &record.thread, turn)?
        else {
            return Ok(pending("source_admission_pending"));
        };
        if lease.input != admission.source {
            return Err(Error("source_lease_conflict"));
        }
        let history = Self::history(&native, &record.thread).await?;
        let context = native.public_context(&record.thread).await?;
        let (instructions, policy) = {
            let works = self.works.lock().await;
            let work = works
                .get(&record.thread)
                .ok_or(Error("operator_work_unavailable"))?;
            if !same_native_round(&record, &work.record)
                || work.record.phase != "running"
                || work.stops_in_progress != 0
                || work.app_cancel.is_none()
                || !self
                    .development_binding(&record)?
                    .is_some_and(|binding| binding.enabled)
            {
                return Err(Error("operator_controller_fenced"));
            }
            let Some(request) = work.request.as_ref() else {
                return Ok(pending("native_round_not_ready"));
            };
            (request.instructions.clone(), digest(&request.instructions))
        };
        let receipt = digest(&(
            "source-context-v1",
            &admission,
            record.generation,
            Self::cursor(&history),
            Self::cursor(&context),
            policy,
        ));
        let accepted = match args.get("accept") {
            None => false,
            Some(value) if value.as_str() == Some(receipt.as_str()) => {
                let works = self.works.lock().await;
                let work = works
                    .get(&record.thread)
                    .ok_or(Error("operator_work_unavailable"))?;
                if !same_native_round(&record, &work.record)
                    || work.record.phase != "running"
                    || work.stops_in_progress != 0
                    || work.app_cancel.is_none()
                    || !self
                        .development_binding(&record)?
                        .is_some_and(|binding| binding.enabled)
                {
                    return Err(Error("operator_controller_fenced"));
                }
                self.broker
                    .accept_source_context(&lease.token, &receipt)
                    .await?;
                true
            }
            _ => return Err(Error("source_context_changed")),
        };
        let tools = if accepted {
            Some(self.broker.catalog(&lease.token).await?)
        } else {
            None
        };
        Ok(
            json!({"status":if accepted {"ready"}else{"context_required"},"work_id":record.id,"thread_id":record.thread,
            "native_turn":turn,"source_admission":admission,"history":history,"cursor":Self::cursor(&history),
            "context":context,"context_cursor":Self::cursor(&context),"instructions":instructions,"receipt":receipt,
            "turn_token":if accepted {Some(&lease.token)}else{None},"tools":tools,"inference_submitted":false,
            "completion":"The original ChatGPT public final completes this source turn. Do not call codex_finish or start another response."}),
        )
    }
    fn source_admission(
        &self,
        record: &Record,
        turn: &str,
    ) -> Result<Option<SourceAdmission>, Error> {
        let admission: Option<SourceAdmission> =
            self.journal.read(&source_admission_key(&record.id, turn))?;
        if let Some(source) = &admission {
            if source.schema != SOURCE_ADMISSION_SCHEMA
                || source.work_id != record.id
                || source.thread_id != record.thread
                || source.turn_id != turn
            {
                return Err(Error("source_admission_conflict"));
            }
            source.source.validate()?;
        }
        Ok(admission)
    }
    async fn wait_item(&self, native: &Native, record: &Record, id: &str) -> Result<(), Error> {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if Self::history(native, &record.thread)
                    .await?
                    .iter()
                    .any(|v| v["item"]["id"] == id)
                {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .map_err(|_| Error("native_report_commit_unknown"))?
    }
    async fn emit(
        &self,
        record: &Record,
        id: String,
        text: String,
        final_answer: bool,
    ) -> Result<(), Error> {
        let works = self.works.lock().await;
        let work = works
            .get(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?;
        if work.record.capability != record.capability
            || !matches!(
                work.record.phase.as_str(),
                "running" | "handoff" | "finishing"
            )
        {
            return Err(Error("operator_controller_fenced"));
        }
        let sender = work.active.clone().ok_or(Error("native_round_not_ready"))?;
        drop(works);
        let (ack, receive) = oneshot::channel();
        sender
            .send(Command::Text {
                capability: record.capability.clone(),
                id,
                text,
                final_answer,
                ack,
            })
            .await
            .map_err(|_| Error("native_delivery_unknown"))?;
        receive
            .await
            .map_err(|_| Error("native_delivery_unknown"))?
    }
    async fn settle_finish(&self, native: &Native, record: &Record) -> Result<(), Error> {
        if record.phase == "completed" {
            return Ok(());
        }
        if record.phase != "finishing" || !record.accepted || native.owner != record.execution {
            return Err(Error("operator_reconciliation_required"));
        }
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let response = native
                    .call(
                        "thread/read",
                        json!({"threadId":record.thread,"includeTurns":true}),
                    )
                    .await?;
                let settled = match &record.native_turn {
                    Some(id) => response["thread"]["turns"].as_array().is_some_and(|turns| {
                        turns
                            .iter()
                            .any(|turn| turn["id"] == *id && turn["status"] == "completed")
                    }),
                    None => response["thread"]["status"]["type"] == "idle",
                };
                if settled {
                    return Ok::<_, Error>(());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .map_err(|_| Error("native_completion_unconfirmed"))??;
        if !self.owned_jobs(native, record).await?.is_empty() {
            return Err(Error("operator_background_jobs_unsettled"));
        }
        let mut works = self.works.lock().await;
        let work = works
            .get_mut(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?;
        if work.record.capability != record.capability
            || work.record.generation != record.generation
            || work.record.native_turn != record.native_turn
            || !work.record.accepted
            || work.record.phase != "finishing"
        {
            return Err(Error("operator_controller_fenced"));
        }
        if !work.record.pending.is_empty() {
            return Err(Error("operator_pending_completion"));
        }
        self.broker.retire(&record.token, false).await?;
        let mut next = work.record.clone();
        next.phase = "completed".into();
        self.persist(&next)?;
        work.record = next;
        Ok(())
    }
    async fn private_controls(&self) -> Vec<String> {
        let works = self.works.lock().await;
        let mut values = vec![self.token.clone(), self.journal.owner.clone()];
        for work in works.values() {
            values.extend([work.record.capability.clone(), work.record.token.clone()]);
        }
        values
    }
    async fn ensure_public_text(&self, text: &str) -> Result<(), Error> {
        let values = self.private_controls().await;
        crate::control::ensure_public_text(text, values.iter().map(String::as_str))
    }
    async fn report(&self, args: &Value, final_answer: bool) -> Result<Value, Error> {
        let (record, native) = self.binding(args, false).await?;
        if args["controller"].as_str() != Some(&record.capability) {
            return Err(Error("operator_controller_fenced"));
        }
        let call = text(args, "call_id", 160)?;
        let body = text(args, "text", 256 * 1024)?;
        let fingerprint = digest(&(body, final_answer));
        if let Some((old, id)) = record.receipts.get(call) {
            if old != &fingerprint {
                return Err(Error("operator_call_changed"));
            }
            self.wait_item(&native, &record, id).await?;
            if final_answer {
                self.settle_finish(&native, &record).await?;
            }
            return Ok(
                json!({"native_item":id,"recorded":true,"reused":true,"completed":final_answer}),
            );
        }
        if self
            .app_route(&record, record.native_turn.as_deref())?
            .is_some()
        {
            return Err(Error("app_response_owns_execution"));
        }
        if !record.accepted || record.phase != "running" {
            return Err(Error("operator_controller_fenced"));
        }
        self.ensure_public_text(body).await?;
        self.verify_identity(&record).await?;
        if final_answer && !self.owned_jobs(&native, &record).await?.is_empty() {
            return Err(Error("operator_background_jobs_unsettled"));
        }
        if record.receipts.len() >= 4096 {
            return Err(Error("operator_report_limit"));
        }
        if final_answer && !record.pending.is_empty() {
            return Err(Error("operator_pending_completion"));
        }
        let id = format!("operator_{}", digest(&(&record.id, call)));
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability
                || !work.record.accepted
                || work.record.phase != "running"
            {
                return Err(Error("operator_controller_fenced"));
            }
            if work.record.receipts.contains_key(call) {
                return Err(Error("operator_report_delivery_in_progress"));
            }
            if final_answer {
                if !self.broker.unsettled(&record.token).await?.is_empty() {
                    return Err(Error("operator_pending_completion"));
                }
                work.record.phase = "finishing".into();
            }
            work.record
                .receipts
                .insert(call.into(), (fingerprint, id.clone()));
            self.persist(&work.record)?;
        }
        // Receipt intent precedes delivery: an uncertain retry only observes.
        self.emit(&record, id.clone(), body.into(), final_answer)
            .await?;
        self.wait_item(&native, &record, &id).await?;
        if final_answer {
            let latest = self
                .works
                .lock()
                .await
                .get(&record.thread)
                .ok_or(Error("operator_work_unavailable"))?
                .record
                .clone();
            if latest.capability != record.capability {
                return Err(Error("operator_controller_fenced"));
            }
            self.settle_finish(&native, &latest).await?;
        }
        Ok(json!({"native_item":id,"recorded":true,"completed":final_answer}))
    }
    pub async fn call(&self, name: &str, args: Value) -> Result<Value, Error> {
        validate_arguments(&tools(), name, &args)?;
        // Only identical logical mutations share this wait. Broker tool calls
        // already coalesce original-result observers; reads and Stop must remain
        // independent of a waiting finish or handoff.
        let summary_round = if name == "codex_native_summary" {
            let works = self.works.lock().await;
            Some(
                works
                    .values()
                    .find(|work| Some(work.record.id.as_str()) == args["work_id"].as_str())
                    .ok_or(Error("operator_work_unavailable"))?
                    .round
                    .clone(),
            )
        } else {
            None
        };
        let identity = match name {
            "codex_report" | "codex_finish" => {
                Some(json!(["report", args["work_id"], args["call_id"]]))
            }
            "codex_handoff" => Some(json!([name, args["work_id"], args["call_id"]])),
            "codex_stop" => Some(json!([name, args["work_id"]])),
            "codex_sessions" if args["action"] == "queue" => {
                Some(json!(["native-queue", args["thread_id"], args["call_id"]]))
            }
            "codex_sessions" if args["action"] == "chat_input" => Some(json!([
                "chat-input",
                args["conversation_id"],
                args["source_cursor"]["account"],
                args["call_id"]
            ])),
            "codex_sessions"
                if !matches!(
                    args["action"].as_str(),
                    Some("list" | "events" | "wait" | "synchronize" | "chat_result")
                ) =>
            {
                Some(
                    if matches!(args["action"].as_str(), Some("create" | "resume" | "fork")) {
                        json!(["session-create", args["call_id"]])
                    } else {
                        json!([
                            name,
                            args["work_id"],
                            args["controller"],
                            args["action"],
                            args["call_id"]
                        ])
                    },
                )
            }
            "codex_answer" => Some(json!([
                name,
                args["work_id"],
                args["controller"],
                args["request_id"]
            ])),
            "codex_native_summary" => {
                let identity = args
                    .get("call_id")
                    .map_or_else(|| json!(["round", summary_round]), |id| json!(["call", id]));
                Some(json!([name, args["work_id"], args["controller"], identity]))
            }
            "codex_attach" if args.get("accept").is_some() => Some(json!([
                name,
                args["work_id"],
                args["controller"],
                args["accept"]
            ])),
            _ => None,
        };
        if let Some(identity) = identity {
            let this = self.clone();
            let fingerprint = digest(&(name, &args));
            let name = name.to_owned();
            return self
                .operations
                .run(
                    digest(&identity),
                    fingerprint,
                    Box::pin(async move { this.call_inner(&name, args, summary_round).await }),
                )
                .await;
        }
        self.call_inner(name, args, summary_round).await
    }
    async fn call_inner(
        &self,
        name: &str,
        args: Value,
        summary_round: Option<String>,
    ) -> Result<Value, Error> {
        match name {
            "native_catalog" => {
                self.broker
                    .catalog_readback(text(&args, "turn_token", 48)?)
                    .await
            }
            "native_tool" => {
                let token = text(&args, "turn_token", 48)?.to_owned();
                let mut args = args;
                args.as_object_mut()
                    .ok_or(Error("mcp_call_invalid"))?
                    .remove("turn_token");
                let args: Arguments =
                    serde_json::from_value(args).map_err(|_| Error("mcp_call_invalid"))?;
                if !self.broker.contains_call(&token, &args.call_id)? {
                    self.ensure_public_text(&args.report).await?;
                }
                self.broker.invoke(&token, args).await
            }
            "codex_host" => {
                let action = text(&args, "action", 16)?;
                if action == "stop"
                    && self.works.lock().await.values().any(|work| {
                        work.app_cancel.is_some()
                            || work
                                .record
                                .selection
                                .as_ref()
                                .is_some_and(|s| s.reasoning == Reasoning::Provider)
                                && !matches!(
                                    work.record.phase.as_str(),
                                    "completed" | "stop_requested" | "stopped"
                                )
                            || work.record.driver.is_some()
                                && !matches!(
                                    work.record.phase.as_str(),
                                    "completed" | "stop_requested" | "stopped"
                                )
                    })
                {
                    return Err(Error("host_active_work"));
                }
                crate::control::host(self.bridge.as_ref(), &args).await
            }
            "codex_projects" => {
                let path =
                    crate::store::config_path().map_err(|_| Error("invalid_configuration_path"))?;
                crate::project::control(self.bridge.as_ref(), &self.journal, &path, &args).await
            }
            "codex_chats" => {
                let secrets = self.private_controls().await;
                crate::chats::Chats::new(self.bridge.clone())
                    .with_public_ids(crate::control::public_work_ids(&self.journal)?)
                    .call(args, &secrets)
                    .await
            }
            "codex_sessions" => self.sessions(args).await,
            "codex_attach" => self.attach(args).await,
            "codex_history" => {
                let (record, native) = self.binding(&args, false).await?;
                let history = Self::history(&native, &record.thread).await?;
                let after = match args.get("after").filter(|v| !v.is_null()) {
                    Some(v) => serde_json::from_value::<Cursor>(v.clone())
                        .map_err(|_| Error("operator_cursor_invalid"))?,
                    None => Cursor {
                        count: 0,
                        digest: digest(&Vec::<Value>::new()),
                    },
                };
                if !Self::covered(&history, &after) {
                    return Err(Error("native_history_boundary_changed"));
                }
                let context = native.public_context(&record.thread).await?;
                let after_context = match args.get("after_context") {
                    Some(value) => serde_json::from_value::<Cursor>(value.clone())
                        .map_err(|_| Error("operator_cursor_invalid"))?,
                    None => Self::cursor(&[]),
                };
                if !Self::covered(&context, &after_context) {
                    return Err(Error("native_context_boundary_changed"));
                }
                let turns = history[after.count..]
                    .iter()
                    .filter_map(|row| row["turn_id"].as_str())
                    .collect::<std::collections::BTreeSet<_>>();
                let source_participation = turns.iter().map(|turn| Ok(json!({"turn_id":turn,"participation":self.source_participation(&record,turn)?})))
                    .collect::<Result<Vec<_>,Error>>()?;
                let source_admissions = turns
                    .iter()
                    .map(|turn| self.source_admission(&record, turn))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let source_executions = turns
                    .iter()
                    .map(|turn| self.mcp_host.source_execution(&record.thread, turn))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                Ok(
                    json!({"items":history[after.count..],"cursor":Self::cursor(&history),"context_items":context[after_context.count..],"context_cursor":Self::cursor(&context),"source_admissions":source_admissions,"source_executions":source_executions,"source_participation":source_participation}),
                )
            }
            "codex_tools" => {
                let (record, _) = self.binding(&args, true).await?;
                Ok(
                    json!({"tools":self.broker.catalog(&record.token).await?,"native_context":record.phase}),
                )
            }
            "codex_execute" => {
                let (record, native) = self.binding(&args, false).await?;
                if record
                    .selection
                    .as_ref()
                    .is_some_and(|s| s.reasoning == Reasoning::Provider)
                {
                    return Err(Error("app_response_owns_execution"));
                }
                self.verify_identity(&record).await?;
                let call: Arguments = serde_json::from_value(json!({"call_id":args["call_id"],"name":args["name"],"input":args["input"],"report":args["report"]}))
                    .map_err(|_| Error("mcp_call_invalid"))?;
                if !self.broker.contains_call(&record.token, &call.call_id)? {
                    self.ensure_public_text(&call.report).await?;
                    self.admit_native_call(&record.thread, &call.call_id)
                        .await?;
                }
                {
                    let works = self.works.lock().await;
                    let work = works.get(&record.thread).unwrap();
                    let known = self.broker.contains_call(&record.token, &call.call_id)?;
                    if work.record.capability != record.capability
                        || (!known && (!work.record.accepted || work.record.phase != "running"))
                    {
                        return Err(Error("operator_controller_fenced"));
                    }
                    self.broker.enqueue(&record.token, &call).await?;
                }
                let output = self.broker.invoke(&record.token, call).await?;
                let latest = self
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .unwrap()
                    .record
                    .clone();
                let history = Self::history(&native, &record.thread).await?;
                let context = native.public_context(&record.thread).await?;
                let context_count = latest
                    .observed_context
                    .as_ref()
                    .filter(|cursor| Self::covered(&context, cursor))
                    .map_or(0, |cursor| cursor.count);
                let count = latest
                    .observed
                    .as_ref()
                    .filter(|cursor| Self::covered(&history, cursor))
                    .map_or(0, |cursor| cursor.count);
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                let policy = work.request.as_ref().map(|request| &request.instructions);
                let instructions = policy.filter(|value| Some(digest(value)) != work.record.policy);
                let result = json!({"result":output,"native_context":latest.phase,"work_id":record.id,
                    "native_updates":history[count..],"cursor":Self::cursor(&history),"context_updates":context[context_count..],"context_cursor":Self::cursor(&context),"instructions":instructions});
                // A detached old observer still receives its original result,
                // but cannot acknowledge context for a successor controller.
                if work.record.capability == record.capability {
                    work.record.policy = policy.map(digest);
                    work.record.observed = Some(Self::cursor(&history));
                    work.record.observed_context = Some(Self::cursor(&context));
                    self.persist(&work.record)?;
                }
                Ok(result)
            }
            "codex_answer" => {
                let key = digest(&(
                    "operator-answer-v1",
                    &args["work_id"],
                    &args["controller"],
                    &args["request_id"],
                ));
                let fingerprint = digest(&args);
                if let Some(result) = operation::recover(
                    &self.journal,
                    &key,
                    &fingerprint,
                    "native_answer_reconciliation_required",
                )? {
                    return Ok(result);
                }
                let _admission = self.native_admission(&args).await?;
                let (record, native) = self.binding(&args, false).await?;
                if args["controller"].as_str() != Some(&record.capability)
                    || matches!(
                        record.phase.as_str(),
                        "stop_requested" | "stopped" | "reconciliation_required"
                    )
                {
                    return Err(Error("operator_controller_fenced"));
                }
                text(&args, "request_id", 256)?;
                native
                    .answer_question(
                        &record.thread,
                        text(&args, "request_id", 256)?,
                        args["answers"].clone(),
                        || operation::intent(&self.journal, &key, &fingerprint),
                    )
                    .await?;
                let result = json!({"answer_forwarded":true,"approval_granted":false});
                operation::settle(&self.journal, &key, &fingerprint, &result)?;
                Ok(result)
            }
            "codex_report" => self.report(&args, false).await,
            "codex_finish" => self.report(&args, true).await,
            "codex_status" => self.status(args).await,
            "codex_stop" => self.stop(args).await,
            "codex_handoff" => self.handoff(args).await,
            "codex_native_summary" => {
                let (record, _) = self.binding(&args, false).await?;
                if args["controller"].as_str() != Some(&record.capability) {
                    return Err(Error("operator_controller_fenced"));
                }
                let round = summary_round.ok_or(Error("native_round_not_ready"))?;
                let identity = if args.get("call_id").is_some() {
                    json!(["call", text(&args, "call_id", 160)?])
                } else {
                    json!(["round", round])
                };
                let key = digest(&(
                    "operator-native-summary-v2",
                    &record.id,
                    &record.capability,
                    identity,
                ));
                let fingerprint = digest(&args);
                if let Some(result) = operation::recover(
                    &self.journal,
                    &key,
                    &fingerprint,
                    "native_summary_reconciliation_required",
                )? {
                    return Ok(result);
                }
                let (record, _) = self.binding(&args, true).await?;
                if record.phase != "native_summary" {
                    return Err(Error("native_summary_not_requested"));
                }
                let summary = text(&args, "summary", 256 * 1024)?.to_owned();
                if summary.trim().is_empty() {
                    return Err(Error("empty_compaction_summary"));
                }
                let sender = {
                    let works = self.works.lock().await;
                    let work = works
                        .get(&record.thread)
                        .ok_or(Error("operator_work_unavailable"))?;
                    if work.round != round
                        || work.record.capability != record.capability
                        || !work.record.accepted
                        || work.record.phase != "native_summary"
                    {
                        return Err(Error("operator_controller_fenced"));
                    }
                    work.active.clone().ok_or(Error("native_round_not_ready"))?
                };
                let (ack, response) = oneshot::channel();
                operation::intent_with_context(
                    &self.journal,
                    &key,
                    &fingerprint,
                    json!({"round":round}),
                )?;
                sender
                    .send(Command::Summary {
                        capability: record.capability.clone(),
                        round: round.clone(),
                        text: summary,
                        ack,
                    })
                    .await
                    .map_err(|_| Error("native_delivery_unknown"))?;
                response
                    .await
                    .map_err(|_| Error("native_delivery_unknown"))??;
                let result = json!({"native_summary_offered":true,"chat_replaced":false,"native_round":round});
                operation::settle(&self.journal, &key, &fingerprint, &result)?;
                Ok(result)
            }
            _ => Err(Error("mcp_tool_unknown")),
        }
    }
    async fn attach(&self, args: Value) -> Result<Value, Error> {
        let (record, native) = self.binding(&args, false).await?;
        if args["controller"].as_str() != Some(&record.capability) {
            return Err(Error("operator_controller_fenced"));
        }
        if matches!(
            record.phase.as_str(),
            "reconciliation_required" | "stop_requested" | "completed" | "stopped"
        ) {
            return Err(Error("operator_reconciliation_required"));
        }
        self.verify_identity(&record).await?;
        let history = Self::history(&native, &record.thread).await?;
        let context = native.public_context(&record.thread).await?;
        let mut summary = None;
        let mut count = 0;
        let mut context_count = 0;
        if let Some(id) = &record.summary {
            let item = history
                .iter()
                .find(|v| v["item"]["id"] == *id)
                .ok_or(Error("native_handoff_missing"))?;
            summary = Some(item.clone());
            let source = record
                .source
                .as_ref()
                .ok_or(Error("operator_record_invalid"))?;
            if !Self::covered(&history, source) {
                return Err(Error("native_history_boundary_changed"));
            }
            count = source.count;
            let source_context = record
                .source_context
                .as_ref()
                .ok_or(Error("operator_record_invalid"))?;
            if !Self::covered(&context, source_context) {
                return Err(Error("native_context_boundary_changed"));
            }
            context_count = source_context.count;
        }
        let receipt = digest(&(
            &record.id,
            record.generation,
            &summary,
            Self::cursor(&history),
            Self::cursor(&context),
            self.works
                .lock()
                .await
                .get(&record.thread)
                .and_then(|work| {
                    work.request
                        .as_ref()
                        .map(|request| digest(&request.instructions))
                }),
        ));
        if args["accept"].as_str() == Some(&receipt) {
            if self
                .app_route(&record, record.native_turn.as_deref())?
                .is_some()
            {
                return Err(Error("app_response_owns_execution"));
            }
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability
                || work.stops_in_progress != 0
                || matches!(
                    work.record.phase.as_str(),
                    "reconciliation_required" | "stop_requested" | "completed" | "stopped"
                )
            {
                return Err(Error("operator_controller_fenced"));
            }
            work.record.accepted = true;
            work.record.observed = Some(Self::cursor(&history));
            work.record.observed_context = Some(Self::cursor(&context));
            work.record.policy = work
                .request
                .as_ref()
                .map(|request| digest(&request.instructions));
            if work.record.phase == "handoff" {
                work.record.phase = if work.active.is_some() {
                    "running"
                } else {
                    "waiting_native"
                }
                .into();
            }
            self.persist(&work.record)?;
        }
        let direct = self
            .app_route(&record, record.native_turn.as_deref())?
            .is_none();
        let works = self.works.lock().await;
        let work = works.get(&record.thread).unwrap();
        Ok(
            json!({"work_id":record.id,"thread_id":record.thread,"origin":record.origin,"generation":record.generation,
            "summary":summary,"incremental_history":history[count..],"cursor":Self::cursor(&history),"receipt":receipt,
            "incremental_context":context[context_count..],"context_cursor":Self::cursor(&context),
            "accepted":work.record.accepted,"instructions":work.request.as_ref().map(|request|&request.instructions),
            "progress":direct.then_some(DIRECT_PROGRESS),
            "completion":direct.then_some(DIRECT_COMPLETION),
            "native_context":work.record.phase,"return_bound":record.origin=="codex"}),
        )
    }
    async fn queue_input(&self, args: Value) -> Result<Value, Error> {
        let thread = text(&args, "thread_id", 256)?;
        let call = text(&args, "call_id", 160)?;
        let message = text(&args, "text", 256 * 1024)?;
        let key = queue_key(thread, call);
        let fingerprint = digest(&args);
        let previous = self.journal.read::<Value>(&key)?;
        if let Some(previous) = &previous {
            if previous["fingerprint"] != fingerprint {
                return Err(Error("operator_call_changed"));
            }
            if let Some(result) = previous.get("result") {
                return Ok(result.clone());
            }
        }
        // Only observation transport is recoverable. An outstanding enqueue
        // intent never authorizes another add, even when its queue item is absent.
        let native = if let Some(home) = args.get("execution_home") {
            let home = text_value_path(home, "native_workspace_identity_invalid")?;
            let pool = self.scoped_pool(&home, &self.pool.workspace).await?;
            match pool.for_thread(thread).await {
                Ok(native) => native,
                Err(Error("native_thread_owner_unavailable")) => pool.prepare().await?,
                Err(error) => return Err(error),
            }
        } else {
            match self.pool.discover_thread(thread).await {
                Ok(native) => native,
                Err(Error("native_thread_owner_unavailable")) => self.pool.prepare().await?,
                Err(error) => return Err(error),
            }
        };
        let client_id = queue_client_id(thread, call);
        if previous.is_some() {
            let mut cursor = Value::Null;
            loop {
                let page = native
                    .call(
                        "thread/queue/list",
                        json!({"threadId":thread,"cursor":cursor,"limit":100}),
                    )
                    .await?;
                let rows = page["data"]
                    .as_array()
                    .ok_or(Error("native_reply_invalid"))?;
                if let Some(item) = rows
                    .iter()
                    .find(|item| item["clientUserMessageId"] == client_id)
                {
                    let result = json!({"thread_id":thread,"accepted":true,"delivery":"queued","queued_submission":item});
                    operation::settle(&self.journal, &key, &fingerprint, &result)?;
                    return Ok(result);
                }
                if page["nextCursor"].is_null() {
                    break;
                }
                if page["nextCursor"] == cursor {
                    return Err(Error("native_reply_invalid"));
                }
                cursor = page["nextCursor"].clone();
            }
            for row in Self::history(&native, thread).await? {
                if row["item"]["type"] == "userMessage" && row["item"]["clientId"] == client_id {
                    let result = json!({"thread_id":thread,"accepted":true,"delivery":"committed","native_turn":row["turn_id"]});
                    operation::settle(&self.journal, &key, &fingerprint, &result)?;
                    return Ok(result);
                }
            }
            return Err(Error("native_queue_delivery_unknown"));
        }
        // Establish the saved target exists before recording any mutation intent.
        native
            .call(
                "thread/read",
                json!({"threadId":thread,"includeTurns":false}),
            )
            .await?;
        operation::intent(&self.journal, &key, &fingerprint)?;
        let response = native.call("thread/queue/add", json!({"threadId":thread,"input":[{"type":"text","text":message}],"clientUserMessageId":client_id})).await?;
        let item = &response["queuedSubmission"];
        if item["clientUserMessageId"] != client_id || item["id"].as_str().is_none() {
            return Err(Error("native_reply_invalid"));
        }
        let result = json!({"thread_id":thread,"accepted":true,"delivery":"queued","queued_submission":item});
        operation::settle(&self.journal, &key, &fingerprint, &result)?;
        Ok(result)
    }
    async fn sessions(&self, args: Value) -> Result<Value, Error> {
        let receipt = if matches!(
            args["action"].as_str(),
            Some(
                "start"
                    | "join"
                    | "compact"
                    | "claim"
                    | "takeover"
                    | "evaluate"
                    | "transfer"
                    | "transfer_accept"
                    | "bind"
                    | "unbind"
            )
        ) && args.get("call_id").is_some()
        {
            let call = text(&args, "call_id", 160)?;
            let binding_control =
                matches!(args["action"].as_str(), Some("bind" | "unbind" | "join"));
            let controller_scope = if binding_control {
                &Value::Null
            } else {
                &args["controller"]
            };
            let key = digest(&(
                "operator-session-mutation-v1",
                &args["work_id"],
                controller_scope,
                &args["action"],
                call,
            ));
            // Bind/unbind and join are selected by authenticated work identity,
            // not a caller's old capability. A join retry returns its original
            // generation; it cannot authorize effects after that owner is fenced.
            let fingerprint = if binding_control {
                let mut input = args.clone();
                input.as_object_mut().unwrap().remove("controller");
                digest(&input)
            } else {
                digest(&args)
            };
            if let Some(result) = operation::recover(
                &self.journal,
                &key,
                &fingerprint,
                "native_session_mutation_unknown",
            )? {
                return Ok(result);
            }
            Some((key, fingerprint))
        } else {
            None
        };
        let result = self.session_action(args, receipt.as_ref()).await?;
        if let Some((key, fingerprint)) = receipt {
            operation::settle(&self.journal, &key, &fingerprint, &result)?;
        }
        Ok(result)
    }
    fn mutation_intent(&self, receipt: Option<&(String, String)>) -> Result<(), Error> {
        if let Some((key, fingerprint)) = receipt {
            operation::intent(&self.journal, key, fingerprint)?;
        }
        Ok(())
    }
    async fn resume_unloaded_native(
        &self,
        record: &Record,
        native: &Native,
        receipt: Option<&(String, String)>,
    ) -> Result<bool, Error> {
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":false}),
            )
            .await?;
        if state["thread"]["status"]["type"] != "notLoaded" {
            return Ok(false);
        }
        let pool = self.work_pool(record).await?;
        native.verify_scope(&record.thread, &pool.workspace).await?;
        let history = Self::cursor(&Self::history(native, &record.thread).await?);
        let context = Self::cursor(&native.public_context(&record.thread).await?);
        {
            let works = self.works.lock().await;
            let work = works
                .get(&record.thread)
                .ok_or(Error("operator_work_unavailable"))?;
            if work.record.generation != record.generation
                || work.record.capability != record.capability
                || work.stops_in_progress != 0
            {
                return Err(Error("operator_controller_fenced"));
            }
            self.mutation_intent(receipt)?;
        }
        native
            .call(
                "thread/resume",
                json!({"threadId":record.thread,"cwd":pool.workspace,"config":pool.routing()?}),
            )
            .await?;
        native.verify_scope(&record.thread, &pool.workspace).await?;
        if Self::cursor(&Self::history(native, &record.thread).await?) != history {
            return Err(Error("native_history_boundary_changed"));
        }
        if Self::cursor(&native.public_context(&record.thread).await?) != context {
            return Err(Error("native_context_boundary_changed"));
        }
        Ok(true)
    }
    async fn native_admission(
        &self,
        args: &Value,
    ) -> Result<tokio::sync::OwnedMutexGuard<()>, Error> {
        let lock = {
            let works = self.works.lock().await;
            let work = works
                .values()
                .find(|work| Some(work.record.id.as_str()) == args["work_id"].as_str())
                .ok_or(Error("operator_work_unavailable"))?;
            if work.stops_in_progress != 0 {
                return Err(Error("operator_controller_fenced"));
            }
            work.native_admission.clone()
        };
        let admission = lock.lock_owned().await;
        let works = self.works.lock().await;
        if works
            .values()
            .find(|work| Some(work.record.id.as_str()) == args["work_id"].as_str())
            .is_none_or(|work| work.stops_in_progress != 0)
        {
            return Err(Error("operator_controller_fenced"));
        }
        Ok(admission)
    }
    async fn session_action(
        &self,
        args: Value,
        receipt: Option<&(String, String)>,
    ) -> Result<Value, Error> {
        if (args.get("model").is_some() || args.get("effort").is_some())
            && !matches!(args["action"].as_str(), Some("chat_input" | "start"))
        {
            return Err(Error("operator_arguments_invalid"));
        }
        let _admission = if matches!(
            args["action"].as_str(),
            Some(
                "start"
                    | "join"
                    | "compact"
                    | "claim"
                    | "takeover"
                    | "evaluate"
                    | "transfer"
                    | "transfer_accept"
                    | "bind"
                    | "unbind"
            )
        ) {
            Some(self.native_admission(&args).await?)
        } else {
            None
        };
        match args["action"].as_str() {
            Some("queue") => self.queue_input(args).await,
            Some("synchronize") => self.synchronize_native(args).await,
            Some("source_sync") => self.synchronize_source(args).await,
            Some("source_join") => self.join_verified_source(args).await,
            Some("bind" | "unbind") => self.set_development_binding(args, receipt).await,
            Some("transfer" | "transfer_accept") => self.transfer(args, receipt).await,
            Some("chat_input" | "chat_result") => {
                let input = crate::chat_input::Inputs {
                    bridge: self.bridge.clone(),
                    journal: self.journal.clone(),
                    config: self.config.clone(),
                };
                let secrets = {
                    let works = self.works.lock().await;
                    let mut secrets = vec![self.token.clone(), self.journal.owner.clone()];
                    for work in works.values() {
                        secrets.extend([work.record.capability.clone(), work.record.token.clone()]);
                    }
                    secrets
                };
                if args["action"] == "chat_input" {
                    let _mutation = self.session_mutations.lock().await;
                    input.submit(args, &secrets).await
                } else {
                    input.result(args, &secrets).await
                }
            }
            Some("wait") => {
                let (_, native) = self.binding(&args, false).await?;
                // Subscribe before the first observation so a completion between
                // observation and waiting cannot be lost. This is local event
                // observation, with no coordinating model or app submission.
                let mut events = native.events.subscribe();
                let seconds = args
                    .get("max_wait_seconds")
                    .map(|v| {
                        v.as_u64()
                            .filter(|v| *v <= 300)
                            .ok_or(Error("operator_wait_invalid"))
                    })
                    .transpose()?
                    .unwrap_or(30);
                let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
                loop {
                    let mut query = args.clone();
                    query["action"] = json!("events");
                    // Call the direct action to avoid another mutation receipt.
                    let result = Box::pin(self.session_action(query, None)).await?;
                    if let Some(reason) = wait_reason(&result) {
                        let mut result = result;
                        result["wait_reason"] = json!(reason);
                        return Ok(result);
                    }
                    if tokio::time::Instant::now() >= deadline {
                        let mut result = result;
                        result["wait_reason"] = json!("pending");
                        return Ok(result);
                    }
                    // A periodic refresh covers lost notifications and fresh
                    // host observations without involving a model observer.
                    let until = deadline.min(tokio::time::Instant::now() + Duration::from_secs(5));
                    tokio::select! {_=tokio::time::sleep_until(until)=>{},row=events.recv()=>{if matches!(row,Err(tokio::sync::broadcast::error::RecvError::Closed)){return Err(Error("native_connection_closed"));}}}
                }
            }
            Some("stop_tree") => {
                let (root, _) = self.binding(&args, false).await?;
                if args["controller"].as_str() != Some(&root.capability) {
                    return Err(Error("operator_controller_fenced"));
                }
                let ids = {
                    let mut works = self.works.lock().await;
                    let mut ids = std::collections::BTreeSet::from([root.id.clone()]);
                    loop {
                        let previous = ids.len();
                        for work in works.values() {
                            if work
                                .record
                                .assignment
                                .as_ref()
                                .and_then(|a| a.parent.as_ref())
                                .is_some_and(|parent| ids.contains(parent))
                            {
                                ids.insert(work.record.id.clone());
                            }
                        }
                        if previous == ids.len() {
                            break;
                        }
                    }
                    for work in works
                        .values_mut()
                        .filter(|work| ids.contains(&work.record.id))
                    {
                        work.record.accepted = false;
                        work.record.phase = "stop_requested".into();
                        if let Some(cancel) = &work.app_cancel {
                            cancel.cancel();
                        }
                        self.persist(&work.record)?;
                    }
                    ids
                };
                let mut observations = Vec::new();
                for id in ids {
                    observations.push(json!({"work_id":id,"stop":match self.stop(json!({"work_id":id})).await {Ok(value)=>value,Err(error)=>json!({"state":"unknown","cause":error.0})}}));
                }
                let confirmed = observations.iter().all(|value| {
                    value["stop"]["native_stop_confirmed"] == true
                        && (value["stop"]["host_stop_requested"] == false
                            || value["stop"]["host_stop_confirmed"] == true)
                });
                Ok(
                    json!({"root_work_id":root.id,"fenced":true,"confirmed":confirmed,"work":observations,"inference_submitted":false}),
                )
            }
            Some("evaluate") => {
                let (record, native) = self.binding(&args, false).await?;
                if args["controller"].as_str() != Some(&record.capability)
                    || record.phase != "completed"
                {
                    return Err(Error("assignment_result_unsettled"));
                }
                let decision = text(&args, "decision", 32)?;
                if !matches!(decision, "accepted" | "changes_requested" | "rejected") {
                    return Err(Error("assignment_evaluation_invalid"));
                }
                let findings = text(&args, "text", 256 * 1024)?;
                let history = Self::history(&native, &record.thread).await?;
                let cursor: Cursor = serde_json::from_value(args["cursor"].clone())
                    .map_err(|_| Error("operator_cursor_invalid"))?;
                if cursor != Self::cursor(&history) {
                    return Err(Error("assignment_evaluation_source_changed"));
                }
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.generation != record.generation
                    || work.record.phase != "completed"
                    || work.record.capability != record.capability
                {
                    return Err(Error("operator_controller_fenced"));
                }
                if work.record.assignment.is_none() {
                    return Err(Error("assignment_unavailable"));
                }
                self.mutation_intent(receipt)?;
                work.record.assignment.as_mut().unwrap().evaluation = Some(
                    json!({"decision":decision,"findings":findings,"cursor":cursor,"observed_at_unix_ms":unix_ms()}),
                );
                self.persist(&work.record)?;
                Ok(
                    json!({"work_id":record.id,"evaluation":work.record.assignment.as_ref().unwrap().evaluation,"inference_submitted":false}),
                )
            }
            Some("list") => {
                let works = self.works.lock().await;
                let mut rows = Vec::new();
                for work in works.values() {
                    let conversation =
                        match crate::app::inspect_lane(&self.journal.root, &work.record.thread) {
                            Ok(lane) => lane["conversation_id"].clone(),
                            Err(Error("native_lane_not_found")) => Value::Null,
                            Err(error) => return Err(error),
                        };
                    rows.push(json!({"work_id":work.record.id,"thread_id":work.record.thread,"origin":work.record.origin,
                        "phase":work.record.phase,"selection":work.record.selection,"workspace":work.record.workspace,
                        "execution_home":work.record.execution.home,"source_conversation_id":conversation,"development_binding":self.development_binding(&work.record)?}));
                }
                Ok(json!({"work":rows}))
            }
            Some("events") => {
                let status = self.status(args.clone()).await?;
                let (record, native) = self.binding(&args, false).await?;
                let after = args
                    .get("cursor")
                    .map(|value| {
                        serde_json::from_value::<Cursor>(value.clone())
                            .map_err(|_| Error("operator_cursor_invalid"))
                    })
                    .transpose()?
                    .unwrap_or_else(|| Self::cursor(&[]));
                let history = match Self::history(&native, &record.thread).await {
                    Ok(history) => history,
                    Err(Error("native_history_not_ready")) => {
                        return Ok(json!({"work_id":record.id,"status":status,"events":[],
                            "cursor":after,"history_observation_available":false,
                            "history_observation_error":"native_history_not_ready",
                            "completed_result":null,"completed_result_available":false,
                            "delivery":"pending_native_history","inference_submitted":false}));
                    }
                    Err(error) => return Err(error),
                };
                if !Self::covered(&history, &after) {
                    return Err(Error("native_history_boundary_changed"));
                }
                let completed = completed_result(
                    status["completed"] == true,
                    record.native_turn.as_deref(),
                    &history,
                );
                Ok(
                    json!({"work_id":record.id,"status":status,"events":history[after.count..],"cursor":Self::cursor(&history),"history_observation_available":true,"completed_result_available":completed.is_some(),"completed_result":completed,"delivery":"retrieved_by_original_work_handle","inference_submitted":false}),
                )
            }
            Some("create" | "resume" | "fork") => {
                let _mutation = self.session_mutations.lock().await;
                let action = args["action"].as_str().unwrap();
                let call = text(&args, "call_id", 160)?;
                let key = digest(&("operator-session-call-v1", call));
                let fingerprint = digest(&args);
                if let Some(result) = operation::recover(
                    &self.journal,
                    &key,
                    &fingerprint,
                    "native_session_creation_unknown",
                )? {
                    return Ok(result);
                }
                let selection = match args.get("reasoning") {
                    Some(value) => {
                        let reasoning = serde_json::from_value::<Reasoning>(value.clone())
                            .map_err(|_| Error("operator_arguments_invalid"))?;
                        let return_path = match args.get("return_path") {
                            Some(value) => serde_json::from_value(value.clone())
                                .map_err(|_| Error("operator_arguments_invalid"))?,
                            None => ReturnPath::Mcp,
                        };
                        if reasoning == Reasoning::Direct && return_path != ReturnPath::Mcp {
                            return Err(Error("direct_requires_mcp"));
                        }
                        if reasoning == Reasoning::Provider
                            && self.config.history.mode != crate::config::HistoryMode::Project
                        {
                            return Err(Error("provider_project_required"));
                        }
                        Some(Selection {
                            reasoning,
                            return_path,
                        })
                    }
                    None if args.get("return_path").is_some() => {
                        return Err(Error("reasoning_selection_required"))
                    }
                    None => None,
                };
                let assignment = if args.get("role").is_some()
                    || args.get("parent_work_id").is_some()
                    || args.get("budget").is_some()
                    || args.get("dependencies").is_some()
                {
                    if selection.is_none() {
                        return Err(Error("reasoning_selection_required"));
                    }
                    let role = text(&args, "role", 64)?.to_owned();
                    let budget: Budget = serde_json::from_value(args["budget"].clone())
                        .map_err(|_| Error("assignment_budget_required"))?;
                    if budget.max_parallel == 0
                        || budget.max_parallel > 32
                        || budget.max_app_starts == 0
                        || budget.max_native_calls == 0
                        || budget.max_elapsed_seconds == 0
                    {
                        return Err(Error("assignment_budget_invalid"));
                    }
                    let parent = args
                        .get("parent_work_id")
                        .map(|_| text(&args, "parent_work_id", 48).map(str::to_owned))
                        .transpose()?;
                    let dependencies: Vec<String> = args
                        .get("dependencies")
                        .map(|value| {
                            serde_json::from_value(value.clone())
                                .map_err(|_| Error("assignment_dependencies_invalid"))
                        })
                        .transpose()?
                        .unwrap_or_default();
                    if dependencies.len() > 256
                        || dependencies
                            .iter()
                            .collect::<std::collections::BTreeSet<_>>()
                            .len()
                            != dependencies.len()
                    {
                        return Err(Error("assignment_dependencies_invalid"));
                    }
                    let works = self.works.lock().await;
                    if let Some(parent) = &parent {
                        let owner = works
                            .values()
                            .find(|work| &work.record.id == parent)
                            .ok_or(Error("assignment_parent_unavailable"))?;
                        if args["parent_controller"].as_str() != Some(&owner.record.capability)
                            || owner.record.assignment.is_none()
                        {
                            return Err(Error("assignment_parent_controller_required"));
                        }
                        if budget.max_parallel
                            > owner
                                .record
                                .assignment
                                .as_ref()
                                .unwrap()
                                .budget
                                .max_parallel
                        {
                            return Err(Error("assignment_budget_exceeds_parent"));
                        }
                    }
                    for dependency in &dependencies {
                        let dependency = works
                            .values()
                            .find(|work| &work.record.id == dependency)
                            .ok_or(Error("assignment_dependency_unavailable"))?;
                        if dependency
                            .record
                            .assignment
                            .as_ref()
                            .and_then(|assignment| assignment.parent.as_ref())
                            != parent.as_ref()
                        {
                            return Err(Error("assignment_dependency_scope_changed"));
                        }
                    }
                    Some(Assignment {
                        role,
                        parent,
                        dependencies,
                        budget,
                        admitted_calls: Default::default(),
                        evaluation: None,
                        started_at_unix_ms: None,
                    })
                } else {
                    None
                };
                let home = args
                    .get("execution_home")
                    .map(|v| text_value_path(v, "native_workspace_identity_invalid"))
                    .transpose()?
                    .unwrap_or_else(|| self.pool.home.clone());
                let workspace = args
                    .get("workspace")
                    .map(|v| text_value_path(v, "native_workspace_identity_invalid"))
                    .transpose()?
                    .unwrap_or_else(|| self.pool.workspace.clone());
                let pool = self.scoped_pool(&home, &workspace).await?;
                if args.get("workspace").is_some() {
                    let candidates = self
                        .works
                        .lock()
                        .await
                        .values()
                        .filter(|work| {
                            work.record.phase == "running"
                                && work
                                    .record
                                    .selection
                                    .as_ref()
                                    .is_some_and(|s| s.reasoning == Reasoning::Provider)
                                && work.record.workspace.as_ref().is_some_and(|other| {
                                    workspace.starts_with(other) || other.starts_with(&workspace)
                                })
                        })
                        .map(|work| work.record.id.clone())
                        .collect::<Vec<_>>();
                    for id in candidates {
                        let (record, native) = self.binding(&json!({"work_id":id}), false).await?;
                        self.observe_native(record, &native).await?;
                    }
                }
                if args.get("workspace").is_some()
                    && self.works.lock().await.values().any(|work| {
                        let other = work
                            .record
                            .workspace
                            .as_deref()
                            .unwrap_or(&self.pool.workspace);
                        !matches!(work.record.phase.as_str(), "completed" | "stopped")
                            && (workspace.starts_with(other) || other.starts_with(&workspace))
                    })
                {
                    return Err(Error("native_workspace_writer_busy"));
                }
                if action == "resume"
                    && self
                        .works
                        .lock()
                        .await
                        .contains_key(text(&args, "thread_id", 256)?)
                {
                    return Err(Error("operator_explicit_handoff_required"));
                }
                let native = pool.prepare().await?;
                let mut config = pool.routing()?;
                let mut use_configured_route = action == "create";
                let mut fork_settings = None;
                if action != "create" {
                    let thread = text(&args, "thread_id", 256)?;
                    if action == "fork"
                        || selection
                            .as_ref()
                            .is_some_and(|s| s.reasoning == Reasoning::Provider)
                    {
                        let state = native
                            .call(
                                "thread/read",
                                json!({"threadId":thread,"includeTurns":false}),
                            )
                            .await?;
                        let previous = state["thread"]["modelProvider"]
                            .as_str()
                            .ok_or(Error("native_model_settings_unavailable"))?;
                        use_configured_route = selection
                            .as_ref()
                            .is_some_and(|s| s.reasoning == Reasoning::Provider)
                            && previous != crate::registration::PROVIDER;
                        if action == "fork" && !use_configured_route {
                            let model = state["thread"]["model"]
                                .as_str()
                                .ok_or(Error("native_model_settings_unavailable"))?;
                            fork_settings = Some((
                                model.to_owned(),
                                previous.to_owned(),
                                state["thread"]["reasoningEffort"].clone(),
                            ));
                        }
                    }
                }
                if use_configured_route {
                    config["model_reasoning_effort"] = json!(self.config.model.reasoning_effort);
                } else if let Some((_, _, effort)) = &fork_settings {
                    config["model_reasoning_effort"] = effort.clone();
                }
                let mut params = json!({"cwd":pool.workspace,"config":config});
                if use_configured_route {
                    params["model"] = json!(self.config.model.default);
                    params["modelProvider"] = json!(crate::registration::PROVIDER);
                } else if let Some((model, provider, _)) = fork_settings {
                    params["model"] = json!(model);
                    params["modelProvider"] = json!(provider);
                }
                if action != "create" {
                    params["threadId"] = json!(text(&args, "thread_id", 256)?);
                }
                let method = match action {
                    "create" => "thread/start",
                    "resume" => "thread/resume",
                    _ => "thread/fork",
                };
                operation::intent(&self.journal, &key, &fingerprint)?;
                let response = native.call(method, params).await?;
                let thread = response["thread"]["id"]
                    .as_str()
                    .ok_or(Error("native_reply_invalid"))?;
                let record = self
                    .register_in(thread, "chatgpt", None, pool, selection, assignment)
                    .await?;
                let result = json!({"work_id":record.id,"thread_id":thread,"controller":record.capability,"selection":record.selection,"workspace":record.workspace,"assignment":record.assignment,"ready":false});
                operation::settle(&self.journal, &key, &fingerprint, &result)?;
                Ok(result)
            }
            Some("start" | "join") => {
                let joining = args["action"] == "join";
                if joining {
                    text(&args, "call_id", 160)?;
                    if [
                        "controller",
                        "reasoning",
                        "return_path",
                        "conversation_id",
                        "source_message_id",
                        "accept",
                    ]
                    .iter()
                    .any(|field| args.get(field).is_some())
                    {
                        return Err(Error("operator_arguments_invalid"));
                    }
                }
                let task = text(&args, "task", 256 * 1024)?;
                if args
                    .get("mode")
                    .is_some_and(|mode| !matches!(mode.as_str(), Some("default" | "plan")))
                {
                    return Err(Error("operator_arguments_invalid"));
                }
                let (mut record, native) = self.binding(&args, false).await?;
                let mut resumed = false;
                if joining {
                    if record.phase == "completed" {
                        resumed = self
                            .resume_unloaded_native(&record, &native, receipt)
                            .await?;
                    }
                    record = self.observe_native(record, &native).await?.0;
                    if record.phase != "completed" {
                        return Err(Error("native_writer_unsettled"));
                    }
                    let pool = self.work_pool(&record).await?;
                    native.verify_scope(&record.thread, &pool.workspace).await?;
                    self.recovery_receipts(&native, &record).await?;
                    let state = native
                        .call(
                            "thread/read",
                            json!({"threadId":record.thread,"includeTurns":false}),
                        )
                        .await?;
                    if state["thread"]["status"]["type"] != "idle"
                        || self.mcp_host.pending_external(&record.thread)?.is_some()
                        || self
                            .broker
                            .recorded_calls(&record.token)?
                            .iter()
                            .any(|call| call["settled"] != true)
                    {
                        return Err(Error("native_writer_unsettled"));
                    }
                }
                if !joining
                    && (args["controller"].as_str() != Some(&record.capability)
                        || !matches!(
                            record.phase.as_str(),
                            "waiting_native" | "completed" | "queued"
                        )
                        || record.phase == "completed" && record.selection.is_none())
                {
                    return Err(Error("operator_controller_fenced"));
                }
                if !joining {
                    resumed = self
                        .resume_unloaded_native(&record, &native, receipt)
                        .await?;
                }
                let owner_changed = native.owner != record.execution;
                if owner_changed {
                    // Explicit idle-work admission selects its verified owner
                    // before turn/start can race the asynchronous model exchange.
                    if !joining {
                        let pool = self.work_pool(&record).await?;
                        native.verify_scope(&record.thread, &pool.workspace).await?;
                        self.recovery_receipts(&native, &record).await?;
                    }
                }
                self.wait_assignment(&record).await?;
                let model_override = args
                    .get("model")
                    .map(|_| text(&args, "model", 128))
                    .transpose()?;
                let effort_override = args
                    .get("effort")
                    .map(|_| text(&args, "effort", 128))
                    .transpose()?;
                if (model_override.is_some() || effort_override.is_some())
                    && record
                        .selection
                        .as_ref()
                        .is_none_or(|selection| selection.reasoning != Reasoning::Provider)
                {
                    return Err(Error("native_model_override_requires_provider"));
                }
                let settings = if args.get("mode").is_some()
                    || model_override.is_some()
                    || effort_override.is_some()
                {
                    let state = native
                        .call(
                            "thread/read",
                            json!({"threadId":record.thread,"includeTurns":false}),
                        )
                        .await?;
                    let model = state["thread"]["model"]
                        .as_str()
                        .ok_or(Error("native_model_settings_unavailable"))?;
                    let effort = effort_override.or(state["thread"]["reasoningEffort"].as_str());
                    let model = model_override.unwrap_or(model);
                    if model_override.is_some() || effort_override.is_some() {
                        let effort = effort.ok_or(Error("native_effort_unavailable"))?;
                        if self.config.catalog.route(model, effort).is_none() {
                            return Err(Error("native_model_effort_incompatible"));
                        }
                    }
                    Some(json!({"model":model,"effort":effort}))
                } else {
                    None
                };
                let collaboration_mode = args.get("mode").map(|mode| json!({"mode":mode,"settings":{"model":settings.as_ref().unwrap()["model"],"reasoning_effort":settings.as_ref().unwrap()["effort"],"developer_instructions":null}}));
                {
                    let mut works = self.works.lock().await;
                    if let Some(assignment) = &record.assignment {
                        for dependency in &assignment.dependencies {
                            let dependency = works
                                .values()
                                .find(|work| &work.record.id == dependency)
                                .ok_or(Error("assignment_dependency_unavailable"))?;
                            if dependency.record.phase != "completed"
                                || dependency
                                    .record
                                    .assignment
                                    .as_ref()
                                    .and_then(|a| a.evaluation.as_ref())
                                    .is_none_or(|v| v["decision"] != "accepted")
                            {
                                return Err(Error("assignment_dependency_not_accepted"));
                            }
                        }
                    }
                    let work = works.get_mut(&record.thread).unwrap();
                    if !matches!(
                        work.record.phase.as_str(),
                        "waiting_native" | "completed" | "queued" | "admitting"
                    ) || work.stops_in_progress != 0
                        || work.record.capability != record.capability
                        || work.record.generation != record.generation
                        || work.record.execution != record.execution
                        || (joining || owner_changed)
                            && (work.active.is_some()
                                || work.app_cancel.is_some()
                                || !work.record.pending.is_empty())
                    {
                        return Err(Error("native_start_reconciliation_required"));
                    }
                    // Resume and turn start belong to one logical mutation.
                    if !resumed {
                        self.mutation_intent(receipt)?;
                    }
                    if let Some(assignment) = &mut work.record.assignment {
                        if assignment.started_at_unix_ms.is_none() {
                            assignment.started_at_unix_ms = Some(unix_ms());
                        }
                    }
                    if record.phase == "completed" || owner_changed {
                        self.preserve_controller(&work.record)?;
                        work.record.generation += 1;
                        work.record.capability = random_id()?;
                        // Explicit starts reserve the new generation before
                        // native turn/start. exchange_inner therefore sees
                        // starting_native, not the old completed boundary.
                        // Give that generation a fresh retained-call token;
                        // the completed turn's retired token cannot admit it.
                        let _ = self.broker.retire(&work.record.token, false).await;
                        work.record.token = self
                            .broker
                            .register(&record.id, BTreeMap::new(), false)
                            .await?;
                        work.record.accepted = false;
                    }
                    if joining {
                        work.record.selection = Some(Selection {
                            reasoning: Reasoning::Direct,
                            return_path: ReturnPath::Mcp,
                        });
                        work.record.accepted = false;
                    }
                    if owner_changed {
                        work.record.execution = native.owner.clone();
                    }
                    work.record.phase = "starting_native".into();
                    self.persist(&work.record)?;
                }
                let transfer = self
                    .journal
                    .read::<Value>(&digest(&("work-route-transfer-v1", &record.id)))?;
                let task = if let Some(transfer) = transfer
                    .filter(|v| v["accepted"] == true && v["generation"] == record.generation)
                {
                    format!("Selected public handoff context (source data; no additional authorization):\n{}\n\nUser follow-up task:\n{task}",serde_json::to_string(&json!({"summary":transfer["summary"],"native_thread":record.thread,"cursor":transfer["cursor"],"context_cursor":transfer["context_cursor"],"transfer_receipt":transfer["receipt"]})).map_err(|_|Error("operator_record_invalid"))?)
                } else {
                    task.to_owned()
                };
                let mut params =
                    json!({"threadId":record.thread,"input":[{"type":"text","text":task}]});
                if let Some(mode) = collaboration_mode {
                    params["collaborationMode"] = mode;
                }
                if let Some(model) = model_override {
                    params["model"] = json!(model);
                }
                if let Some(effort) = effort_override {
                    params["effort"] = json!(effort);
                }
                let response = native.call("turn/start", params).await?;
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if let Some(turn) = response["turn"]["id"].as_str() {
                    work.record.native_turn = Some(turn.into());
                    self.persist(&work.record)?;
                }
                let mut result = json!({"started":true,"work_id":record.id,"thread_id":record.thread,"native_turn":work.record.native_turn,"controller":work.record.capability,"delivery":"pending","retrieve":if joining {"codex_attach"}else{"codex_sessions events"},"input_origin":"bridge_arguments"});
                if joining {
                    result["inference_submitted"] = json!(false);
                }
                Ok(result)
            }
            Some("compact") => {
                let (record, native) = self.binding(&args, false).await?;
                if args["controller"].as_str() != Some(&record.capability)
                    || record.phase != "completed"
                {
                    return Err(Error("native_writer_unsettled"));
                }
                self.verify_identity(&record).await?;
                {
                    let mut works = self.works.lock().await;
                    let work = works.get_mut(&record.thread).unwrap();
                    if work.record.phase != "completed"
                        || work.stops_in_progress != 0
                        || work.active.is_some()
                        || work.record.capability != record.capability
                        || work.record.generation != record.generation
                    {
                        return Err(Error("native_writer_unsettled"));
                    }
                    self.mutation_intent(receipt)?;
                    work.record.phase = "native_compacting".into();
                    work.record.token = self
                        .broker
                        .register(&record.id, BTreeMap::new(), false)
                        .await?;
                    work.record.accepted = true;
                    self.persist(&work.record)?;
                }
                native
                    .call("thread/compact/start", json!({"threadId":record.thread}))
                    .await?;
                Ok(json!({"requested":true,"chat_replaced":false}))
            }
            Some("takeover") => {
                let (record, native) = self.binding(&args, false).await?;
                if record.accepted
                    || !(record.phase == "handoff" || record.driver_phase == "start_unknown")
                {
                    return Err(Error("operator_explicit_handoff_required"));
                }
                self.verify_identity(&record).await?;
                let history = Self::history(&native, &record.thread).await?;
                let cursor: Cursor = serde_json::from_value(args["cursor"].clone())
                    .map_err(|_| Error("operator_cursor_invalid"))?;
                if cursor != Self::cursor(&history) {
                    return Err(Error("native_history_boundary_changed"));
                }
                let context = native.public_context(&record.thread).await?;
                let context_cursor: Cursor = serde_json::from_value(args["context_cursor"].clone())
                    .map_err(|_| Error("operator_cursor_invalid"))?;
                if context_cursor != Self::cursor(&context) {
                    return Err(Error("native_context_boundary_changed"));
                }
                if !record.pending.is_empty()
                    || !self.broker.unsettled(&record.token).await?.is_empty()
                {
                    return Err(Error("operator_pending_handoff"));
                }
                if !self.owned_jobs(&native, &record).await?.is_empty() {
                    return Err(Error("operator_background_jobs_unsettled"));
                }
                self.mutation_intent(receipt)?;
                if record.driver.is_some() {
                    self.stop_driver(&record).await?;
                }
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.capability != record.capability
                    || work.stops_in_progress != 0
                    || work.record.accepted
                    || work.record.phase != record.phase
                    || work.record.generation != record.generation
                {
                    return Err(Error("operator_controller_fenced"));
                }
                self.preserve_controller(&work.record)?;
                work.record.generation += 1;
                work.record.capability = random_id()?;
                work.record.driver = None;
                work.record.driver_phase = "explicit_mcp_takeover".into();
                work.record.phase = if work.active.is_some() {
                    "running"
                } else {
                    "waiting_native"
                }
                .into();
                self.persist(&work.record)?;
                Ok(
                    json!({"work_id":record.id,"thread_id":record.thread,"controller":work.record.capability,"inference_submitted":false}),
                )
            }
            Some("claim") => {
                let (record, native) = self.binding(&args, false).await?;
                if !matches!(
                    record.phase.as_str(),
                    "reconciliation_required" | "stop_requested" | "completed" | "stopped"
                ) {
                    return Err(Error("operator_explicit_handoff_required"));
                }
                let history = Self::history(&native, &record.thread).await?;
                let cursor: Cursor = serde_json::from_value(args["cursor"].clone())
                    .map_err(|_| Error("operator_cursor_invalid"))?;
                if cursor != Self::cursor(&history) {
                    return Err(Error("native_history_boundary_changed"));
                }
                let context = native.public_context(&record.thread).await?;
                let context_cursor: Cursor = serde_json::from_value(args["context_cursor"].clone())
                    .map_err(|_| Error("operator_cursor_invalid"))?;
                if context_cursor != Self::cursor(&context) {
                    return Err(Error("native_context_boundary_changed"));
                }
                let resumed = self
                    .resume_unloaded_native(&record, &native, receipt)
                    .await?;
                let recovered = self.recovery_receipts(&native, &record).await?;
                // Restart recovers the same thread at a settled native boundary only.
                let state = native
                    .call(
                        "thread/read",
                        json!({"threadId":record.thread,"includeTurns":false}),
                    )
                    .await?;
                if !matches!(
                    state["thread"]["status"]["type"].as_str(),
                    Some("idle" | "systemError")
                ) || !self.owned_jobs(&native, &record).await?.is_empty()
                {
                    return Err(Error("native_writer_unsettled"));
                }
                let extension = if let Some(value) = args.get("budget") {
                    if !record
                        .selection
                        .as_ref()
                        .is_some_and(|s| s.reasoning == Reasoning::Direct)
                    {
                        return Err(Error(
                            "assignment_budget_extension_requires_direct_reasoning",
                        ));
                    }
                    let assignment = record
                        .assignment
                        .as_ref()
                        .ok_or(Error("assignment_unavailable"))?;
                    let proposed = serde_json::from_value(value.clone())
                        .map_err(|_| Error("assignment_budget_invalid"))?;
                    let proposed = assignment.budget.extend_budget(proposed)?;
                    if let Some(parent) = &assignment.parent {
                        let works = self.works.lock().await;
                        let owner = works
                            .values()
                            .find(|w| &w.record.id == parent)
                            .ok_or(Error("assignment_parent_unavailable"))?;
                        if args["parent_controller"].as_str() != Some(&owner.record.capability) {
                            return Err(Error("assignment_parent_controller_required"));
                        }
                        let budget = &owner
                            .record
                            .assignment
                            .as_ref()
                            .ok_or(Error("assignment_unavailable"))?
                            .budget;
                        if proposed.max_elapsed_seconds > budget.max_elapsed_seconds
                            || proposed.max_native_calls > budget.max_native_calls
                        {
                            return Err(Error("assignment_budget_exceeds_parent"));
                        }
                    }
                    Some(proposed)
                } else {
                    None
                };
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.capability != record.capability
                    || work.active.is_some()
                    || work.stops_in_progress != 0
                {
                    return Err(Error("native_writer_unsettled"));
                }
                if !resumed {
                    self.mutation_intent(receipt)?;
                }
                self.preserve_controller(&work.record)?;
                if let Some(budget) = extension {
                    work.record.assignment.as_mut().unwrap().budget = budget;
                }
                let _ = self.broker.retire(&record.token, true).await;
                work.record.capability = random_id()?;
                work.record.generation += 1;
                work.record.pending.clear();
                work.record.execution = native.owner.clone();
                work.record.origin = "chatgpt".into();
                work.record.token = self
                    .broker
                    .register(&record.id, BTreeMap::new(), false)
                    .await?;
                work.record.phase =
                    if record.phase == "completed" && state["thread"]["status"]["type"] == "idle" {
                        "completed"
                    } else {
                        "waiting_native"
                    }
                    .into();
                work.record.accepted = false;
                self.persist(&work.record)?;
                Ok(
                    json!({"work_id":record.id,"thread_id":record.thread,"controller":work.record.capability,"history":history,"recovered_native_receipts":recovered,"assignment":work.record.assignment,"inference_submitted":false}),
                )
            }
            _ => Err(Error("operator_session_action_invalid")),
        }
    }
    async fn status(&self, args: Value) -> Result<Value, Error> {
        // A native turn can legitimately advance while a read awaits its
        // snapshot. Reobserve that read locally; never replay a mutation or
        // make an ordinary status caller repair a benign observation race.
        for _ in 0..3 {
            match self.observe_status(&args).await {
                Err(Error("native_observation_superseded")) => continue,
                result => return result,
            }
        }
        Err(Error("native_observation_superseded"))
    }
    fn watch_completion(
        &self,
        record: Record,
        initial_native: Option<Arc<Native>>,
        cancel: CancellationToken,
    ) {
        let this = self.clone();
        tokio::spawn(async move {
            let _retire = cancel.clone().drop_guard();
            let mut native = initial_native;
            let mut events = native.as_ref().map(|native| native.events.subscribe());
            let mut polling = tokio::time::interval(RECONCILE_INTERVAL);
            polling.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                let event = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return,
                    event = async { events.as_mut().unwrap().recv().await }, if events.is_some() => Some(event),
                    _ = polling.tick() => None,
                };
                match event {
                    Some(Ok(event)) if event["method"] == "operator/nativeDisconnected" => {
                        native = None;
                        events = None;
                    }
                    Some(Ok(event)) => {
                        let terminal = event["method"] == "turn/completed"
                            || event["method"] == "thread/status/changed"
                                && event["params"]["status"]["type"] == "idle";
                        if !terminal || event["params"]["threadId"] != record.thread {
                            continue;
                        }
                    }
                    None | Some(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => (),
                    Some(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                        native = None;
                        events = None;
                    }
                }
                let current = {
                    let works = this.works.lock().await;
                    let Some(work) = works.get(&record.thread) else {
                        return;
                    };
                    if work.stops_in_progress != 0
                        || matches!(work.record.phase.as_str(), "stop_requested" | "stopped")
                    {
                        return;
                    }
                    if work.record.phase == "transfer_prepared" {
                        continue;
                    }
                    if !matches!(
                        work.record.phase.as_str(),
                        "running" | "completed" | "interrupted"
                    ) {
                        return;
                    }
                    work.record.clone()
                };
                if native
                    .as_ref()
                    .is_some_and(|native| native.owner != current.execution)
                {
                    native = None;
                    events = None;
                }
                if native.is_none() {
                    let observed = async {
                        let pool = this.work_pool(&current).await?;
                        match pool.observe_owner(&current.execution).await {
                            Ok(native) => Ok(native),
                            Err(Error(
                                "native_binding_unavailable"
                                | "stale_process_owner"
                                | "native_runtime_owner_changed",
                            )) => pool.for_thread(&current.thread).await,
                            Err(error) => Err(error),
                        }
                    }
                    .await;
                    match observed {
                        Ok(observed) => {
                            events = Some(observed.events.subscribe());
                            native = Some(observed);
                        }
                        Err(error @ Error("native_thread_owner_unavailable")) => {
                            eprintln!("native_completion_observation_failed: {}", error.0);
                            continue;
                        }
                        Err(error) => {
                            eprintln!("native_completion_observation_failed: {}", error.0);
                            continue;
                        }
                    }
                }
                let current_native = native.as_ref().unwrap();
                match this.observe_native(current.clone(), current_native).await {
                    Ok((observed, _, _))
                        if matches!(observed.phase.as_str(), "completed" | "interrupted") =>
                    {
                        if let Err(error) = this.ensure_development_binding(&observed).await {
                            eprintln!("development_binding_failed: {}", error.0);
                        }
                        return;
                    }
                    Ok(_) => (),
                    Err(Error(
                        "native_delivery_unknown"
                        | "stale_process_owner"
                        | "native_binding_changed",
                    )) => {
                        native = None;
                        events = None;
                    }
                    Err(error) => {
                        eprintln!("native_completion_observation_failed: {}", error.0);
                        return;
                    }
                }
            }
        });
    }
    async fn synchronize_native(&self, args: Value) -> Result<Value, Error> {
        let thread = text(&args, "thread_id", 64)?;
        if args.get("source_message_id").is_some() {
            return Err(Error("explicit_source_sync_required"));
        }
        let record = self
            .works
            .lock()
            .await
            .get(thread)
            .map(|work| work.record.clone());
        let Some(record) = record else {
            return Ok(json!({"thread_id":thread,"status":"ready","inference_submitted":false}));
        };
        if matches!(
            record.phase.as_str(),
            "stop_requested" | "reconciliation_required"
        ) {
            return Err(Error("operator_reconciliation_required"));
        }
        let pool = self.work_pool(&record).await?;
        let native = pool.discover_thread(thread).await?;
        let (record, state, _) = self.observe_native(record, &native).await?;
        let direct_active = self
            .app_route(&record, record.native_turn.as_deref())?
            .is_none()
            && !matches!(
                record.phase.as_str(),
                "completed" | "interrupted" | "stopped"
            )
            // The next Native request proves checkpoint acknowledgment in
            // exchange_inner; it must pass this barrier to reach that check.
            && !(record.phase == "native_summary_offered"
                && state["thread"]["status"]["type"] == "idle");
        let source_active = self
            .mcp_host
            .source_turn_pending(thread, record.native_turn.as_deref())?;
        let active = state["thread"]["status"]["type"] == "active";
        Ok(if direct_active || source_active && active {
            json!({"thread_id":thread,"status":"waiting","reason":"native_turn_active",
                "retry_after_seconds":RECONCILE_INTERVAL.as_secs(),"inference_submitted":false})
        } else {
            json!({"thread_id":thread,"status":"ready","inference_submitted":false})
        })
    }
    async fn synchronize_source(&self, args: Value) -> Result<Value, Error> {
        let thread = text(&args, "thread_id", 64)?;
        let ready = || json!({"thread_id":thread,"status":"ready","inference_submitted":false});
        let pending = |reason: &str, retry: u64| json!({"thread_id":thread,"status":"waiting","reason":reason,"retry_after_seconds":retry,"inference_submitted":false});
        let record = {
            let works = self.works.lock().await;
            let Some(work) = works.get(thread) else {
                return Ok(ready());
            };
            if work.stops_in_progress != 0 || work.record.phase == "stop_requested" {
                return Err(Error("operator_controller_fenced"));
            }
            work.record.clone()
        };
        if !record
            .selection
            .as_ref()
            .is_some_and(|selection| selection.return_path == ReturnPath::Mcp)
            || record.phase == "stopped"
        {
            return Ok(ready());
        }
        if record.phase == "reconciliation_required" {
            return Err(Error("operator_reconciliation_required"));
        }
        if self
            .ensure_development_binding(&record)
            .await?
            .is_some_and(|binding| !binding.enabled)
        {
            return Ok(ready());
        }
        let pool = self.work_pool(&record).await?;
        let native = pool.discover_thread(thread).await?;
        let (record, state, _) = self.observe_native(record, &native).await?;
        if state["thread"]["status"]["type"] == "notLoaded" {
            return Ok(pending(
                "native_thread_not_loaded",
                RECONCILE_INTERVAL.as_secs(),
            ));
        }
        if state["thread"]["modelProvider"] != crate::registration::PROVIDER {
            return Err(Error("bound_chat_native_route_changed"));
        }
        if !source_native_settled(&record, &state) {
            return Ok(
                if self
                    .mcp_host
                    .source_turn_pending(thread, record.native_turn.as_deref())?
                {
                    pending("bound_chat_turn_active", RECONCILE_INTERVAL.as_secs())
                } else {
                    ready()
                },
            );
        }
        let selected = args
            .get("source_message_id")
            .filter(|value| !value.is_null())
            .map(|_| text(&args, "source_message_id", 256))
            .transpose()?;
        let result = async {
            match self.observe_external(&record, &native, selected).await {
                Ok(true) => Ok(pending(
                    "bound_chat_turn_admitted",
                    RECONCILE_INTERVAL.as_secs(),
                )),
                Ok(false) => {
                    let current = native
                        .call(
                            "thread/read",
                            json!({"threadId":thread,"includeTurns":false}),
                        )
                        .await?;
                    let works = self.works.lock().await;
                    let work = works
                        .get(thread)
                        .ok_or(Error("operator_controller_fenced"))?;
                    if work.stops_in_progress != 0
                        || matches!(work.record.phase.as_str(), "stop_requested" | "stopped")
                    {
                        return Err(Error("operator_controller_fenced"));
                    }
                    if work.record.phase == "reconciliation_required" {
                        return Err(Error("operator_reconciliation_required"));
                    }
                    if !same_native_round(&record, &work.record)
                        || self.mcp_host.source_turn_pending(thread, None)?
                    {
                        return Ok(pending(
                            "bound_chat_observation_superseded",
                            RECONCILE_INTERVAL.as_secs(),
                        ));
                    }
                    drop(works);
                    if current["thread"]["modelProvider"] != crate::registration::PROVIDER {
                        return Err(Error("bound_chat_native_route_changed"));
                    }
                    Ok(if source_native_settled(&record, &current) {
                        ready()
                    } else {
                        pending(
                            "bound_chat_observation_superseded",
                            RECONCILE_INTERVAL.as_secs(),
                        )
                    })
                }
                Err(Error("bound_chat_owner_generation_changed")) => {
                    if !matches!(record.phase.as_str(), "completed" | "interrupted") {
                        return Err(Error("bound_chat_reattachment_unsettled"));
                    }
                    native.verify_scope(thread, &pool.workspace).await?;
                    let admission = {
                        let works = self.works.lock().await;
                        let work = works
                            .get(thread)
                            .ok_or(Error("operator_controller_fenced"))?;
                        if !same_native_round(&record, &work.record)
                            || !matches!(work.record.phase.as_str(), "completed" | "interrupted")
                            || work.stops_in_progress != 0
                            || work.app_cancel.is_some()
                        {
                            return Err(Error("bound_chat_observation_superseded"));
                        }
                        work.native_admission.clone()
                    };
                    let Ok(_admission) = admission.try_lock_owned() else {
                        return Ok(pending(
                            "bound_chat_observation_busy",
                            RECONCILE_INTERVAL.as_secs(),
                        ));
                    };
                    let history = native
                        .call(
                            "thread/read",
                            json!({"threadId":thread,"includeTurns":true}),
                        )
                        .await?;
                    let last = history["thread"]["turns"]
                        .as_array()
                        .and_then(|turns| turns.last());
                    if !source_native_settled(&record, &history)
                        || !last.is_some_and(|turn| {
                            turn["id"].as_str() == record.native_turn.as_deref()
                                && settled_source_turn(&record, turn)
                        })
                        || !self.owned_jobs(&native, &record).await?.is_empty()
                    {
                        return Err(Error("bound_chat_reattachment_unsettled"));
                    }
                    {
                        let works = self.works.lock().await;
                        let work = works
                            .get(thread)
                            .ok_or(Error("operator_controller_fenced"))?;
                        if !same_native_round(&record, &work.record)
                            || !matches!(work.record.phase.as_str(), "completed" | "interrupted")
                            || work.stops_in_progress != 0
                            || work.app_cancel.is_some()
                        {
                            return Err(Error("bound_chat_observation_superseded"));
                        }
                    }
                    self.mcp_host.reattach_settled_source(thread).await?;
                    Ok(pending(
                        "bound_chat_reattached",
                        RECONCILE_INTERVAL.as_secs(),
                    ))
                }
                Err(error) => Err(error),
            }
        }
        .await;
        match result {
            Err(error) if source_observation_pending(&error) => {
                self.source_observation_failed(&record, &error).await;
                let retry = self.source_retry_seconds().await?;
                if selected.is_none()
                    && self
                        .development_binding(&record)?
                        .is_some_and(|binding| binding.source_selection_required)
                {
                    return Err(Error("source_selection_required"));
                }
                Ok(pending(error.0, retry))
            }
            Err(error) => {
                self.source_observation_failed(&record, &error).await;
                Err(error)
            }
            Ok(value) => {
                self.save_source_observation(&record, SourceObservation::Observing)
                    .await?;
                Ok(value)
            }
        }
    }

    async fn observe_external(
        &self,
        record: &Record,
        native: &Native,
        selected_message: Option<&str>,
    ) -> Result<bool, Error> {
        if !self
            .development_binding(record)?
            .is_some_and(|binding| binding.enabled)
        {
            return Ok(false);
        }
        if native.owner.home != record.execution.home {
            return Err(Error("bound_chat_native_owner_changed"));
        }
        let current = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":false}),
            )
            .await?;
        if current["thread"]["modelProvider"] != crate::registration::PROVIDER {
            return Err(Error("bound_chat_native_route_changed"));
        }
        if current["thread"]["status"]["type"] == "notLoaded" {
            return Err(Error("native_thread_not_loaded"));
        }
        if !source_native_settled(record, &current) {
            return Err(Error("bound_chat_turn_active"));
        }
        let history = if selected_message.is_some() {
            Self::history(native, &record.thread).await?
        } else {
            Vec::new()
        };
        let Some(source) = self
            .mcp_host
            .external_input(&record.thread, selected_message, &history)
            .await?
        else {
            return Ok(false);
        };
        let pool = self.work_pool(record).await?;
        native.verify_scope(&record.thread, &pool.workspace).await?;
        let admission = {
            let works = self.works.lock().await;
            let work = works
                .get(&record.thread)
                .ok_or(Error("operator_controller_fenced"))?;
            if !same_native_round(record, &work.record)
                || !matches!(
                    work.record.phase.as_str(),
                    "running" | "completed" | "interrupted"
                )
                || work.app_cancel.is_some()
                || work.stops_in_progress != 0
            {
                return Err(Error("bound_chat_observation_superseded"));
            }
            work.native_admission.clone()
        };
        let Ok(_admission) = admission.try_lock_owned() else {
            return Err(Error("bound_chat_observation_busy"));
        };
        if !self
            .development_binding(record)?
            .is_some_and(|binding| binding.enabled)
        {
            return Ok(false);
        }
        {
            let works = self.works.lock().await;
            let work = works
                .get(&record.thread)
                .ok_or(Error("operator_controller_fenced"))?;
            if !same_native_round(record, &work.record)
                || !matches!(
                    work.record.phase.as_str(),
                    "running" | "completed" | "interrupted"
                )
                || work.app_cancel.is_some()
                || work.stops_in_progress != 0
            {
                return Err(Error("bound_chat_observation_superseded"));
            }
        }
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":false}),
            )
            .await?;
        if !source_native_settled(record, &state) {
            return Err(Error("bound_chat_turn_active"));
        }
        if state["thread"]["modelProvider"] != crate::registration::PROVIDER {
            return Err(Error("bound_chat_native_route_changed"));
        }
        {
            let works = self.works.lock().await;
            let work = works
                .get(&record.thread)
                .ok_or(Error("operator_controller_fenced"))?;
            if !same_native_round(record, &work.record)
                || !matches!(
                    work.record.phase.as_str(),
                    "running" | "completed" | "interrupted"
                )
                || work.app_cancel.is_some()
                || work.stops_in_progress != 0
            {
                return Err(Error("bound_chat_observation_superseded"));
            }
        }
        let (input, call_id) = self
            .mcp_host
            .pending_external(&record.thread)?
            .ok_or(Error("bound_chat_observation_superseded"))?;
        if input != source.input {
            return Err(Error("bound_chat_input_changed"));
        }
        self.queue_input(json!({"thread_id":record.thread,"call_id":call_id,"text":source.text,"execution_home":record.execution.home})).await?;
        Ok(true)
    }
    async fn observe_native(
        &self,
        mut record: Record,
        native: &Native,
    ) -> Result<(Record, Value, Option<Value>), Error> {
        let app_routed = self
            .app_route(&record, record.native_turn.as_deref())?
            .is_some();
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":false}),
            )
            .await?;
        if app_routed && record.phase == "interrupted" && source_native_settled(&record, &state) {
            let pool = self.work_pool(&record).await?;
            native.verify_scope(&record.thread, &pool.workspace).await?;
            let admission = {
                let works = self.works.lock().await;
                let work = works
                    .get(&record.thread)
                    .ok_or(Error("operator_controller_fenced"))?;
                if !same_native_round(&record, &work.record)
                    || work.record.phase != "interrupted"
                    || work.stops_in_progress != 0
                    || work.app_cancel.is_some()
                {
                    return Err(Error("bound_chat_observation_superseded"));
                }
                work.native_admission.clone()
            };
            let _admission = admission
                .try_lock_owned()
                .map_err(|_| Error("bound_chat_observation_busy"))?;
            let history = native
                .call(
                    "thread/read",
                    json!({"threadId":record.thread,"includeTurns":true}),
                )
                .await?;
            let last = history["thread"]["turns"]
                .as_array()
                .and_then(|turns| turns.last());
            if !last.is_some_and(|turn| settled_source_turn(&record, turn))
                || !self.owned_jobs(native, &record).await?.is_empty()
            {
                return Err(Error("bound_chat_reattachment_unsettled"));
            }
            let works = self.works.lock().await;
            let work = works
                .get(&record.thread)
                .ok_or(Error("operator_controller_fenced"))?;
            if !same_native_round(&record, &work.record)
                || work.record.phase != "interrupted"
                || work.stops_in_progress != 0
                || work.app_cancel.is_some()
            {
                return Err(Error("bound_chat_observation_superseded"));
            }
            // Source reconciliation only advances observation. Release the
            // work lock before host reads so Stop can still fence admission;
            // the AppHost revalidates its lane before committing the anchor.
            drop(works);
            self.mcp_host
                .reconcile_interrupted_source(
                    &record.thread,
                    record
                        .native_turn
                        .as_deref()
                        .ok_or(Error("native_turn_unavailable"))?,
                )
                .await?;
        }
        let app_lane = if app_routed {
            match crate::app::inspect_lane(&self.journal.root, &record.thread) {
                Ok(lane) => Some(lane),
                Err(Error("native_lane_not_found")) => None,
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        if app_routed
            && record.phase == "running"
            && source_native_settled(&record, &state)
            && app_lane
                .as_ref()
                .is_some_and(|lane| lane["phase"] == "idle" && lane["pending_native_calls"] == 0)
        {
            // A newly allocated thread may have no persisted turn inventory.
            // Its lightweight state is still observable. Read full turns only
            // at a settled completion boundary where that proof is needed.
            let history = native
                .call(
                    "thread/read",
                    json!({"threadId":record.thread,"includeTurns":true}),
                )
                .await?;
            if history["thread"]["turns"]
                .as_array()
                .and_then(|turns| turns.last())
                .is_some_and(|turn| {
                    record.native_turn.is_some()
                        && turn["id"].as_str() == record.native_turn.as_deref()
                        && turn["status"] == "completed"
                })
                && self.owned_jobs(native, &record).await?.is_empty()
            {
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if same_native_round(&record, &work.record)
                    && work.record.phase == "running"
                    && work.stops_in_progress == 0
                {
                    work.record.phase = "completed".into();
                    self.persist(&work.record)?;
                    record = work.record.clone();
                }
            }
        }
        Ok((record, state, app_lane))
    }
    async fn observe_status(&self, args: &Value) -> Result<Value, Error> {
        let mut observed = false;
        let mut host_observation = Value::Null;
        let (record, native) = self.binding(args, false).await?;
        let (mut record, state, app_lane) = self.observe_native(record, &native).await?;
        if let Some(lane) = &record.driver {
            let snapshot = tokio::time::timeout(
                Duration::from_secs(3),
                self.bridge.operation(json!({"op":"snapshot","lane":lane})),
            )
            .await;
            if let Ok(Ok(snapshot)) = snapshot {
                let waiting = host_waiting(&snapshot);
                host_observation = json!({"dispatches":snapshot["dispatches"].as_u64(),"user_messages":snapshot["userMessages"].as_u64(),"tool_messages":snapshot["toolMessages"].as_u64(),"host_reply_messages":snapshot["hostReplyMessages"].as_u64(),"round_starts":snapshot["roundStarts"].as_u64(),"rejected_before_dispatch":snapshot["rejectedBeforeDispatch"].as_bool(),"waiting_for":waiting,"approval_submission_pending":snapshot["approvalPending"].as_bool(),"approval_ui":{"stage":match snapshot["approvalUi"]["stage"].as_str(){Some(v @ ("card"|"menu"|"once"|"clicked"))=>Some(v),_=>None},"cards":snapshot["approvalUi"]["cards"].as_u64(),"triggers":snapshot["approvalUi"]["triggers"].as_u64(),"menus":snapshot["approvalUi"]["menus"].as_u64(),"options":snapshot["approvalUi"]["options"].as_u64()},"error":match snapshot["error"].as_str(){Some("app_submit_failed")=>Some("app_submit_failed"),Some("app_stream_failed")=>Some("app_stream_failed"),Some(_)=>Some("app_error"),None=>None}});
                observed = true;
                record.host_observed_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                );
                record.host_last_state = if snapshot["stopped"] == true {
                    "stop_confirmed"
                } else if snapshot["error"].is_string() {
                    "error_observed"
                } else if waiting == Some("mcp_host_confirmation_required") {
                    "waiting_confirmation"
                } else if waiting == Some("mcp_host_authentication_required") {
                    "waiting_authentication"
                } else if host_settled(&snapshot) {
                    "stream_settled"
                } else {
                    "stream_observed"
                }
                .into();
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if same_native_round(&record, &work.record)
                    && work.record.phase == record.phase
                    && work.record.driver == record.driver
                {
                    work.record.host_observed_at = record.host_observed_at;
                    work.record.host_last_state = record.host_last_state.clone();
                    self.persist(&work.record)?;
                }
            }
        }
        let requests = native
            .human_requests
            .lock()
            .await
            .values()
            .filter(|v| v["params"]["threadId"] == record.thread)
            .cloned()
            .collect::<Vec<_>>();
        let native_observer_running = {
            let works = self.works.lock().await;
            let work = works.get(&record.thread).unwrap();
            if !same_native_round(&record, &work.record) || record.phase != work.record.phase {
                return Err(Error("native_observation_superseded"));
            }
            work.completion_observer
                .as_ref()
                .is_some_and(|observer| !observer.is_cancelled())
        };
        Ok(
            json!({"work_id":record.id,"thread_id":record.thread,"phase":record.phase,"native_status":state["thread"]["status"],
            "native_observer_running":native_observer_running,
            "pending_calls":record.pending,"call_observations":self.broker.recorded_calls(&record.token)?,"human_requests":requests,
            "selection":record.selection,"workspace":record.workspace,"app_lane":app_lane,
            "development_binding":self.development_binding(&record)?,
            "source_admission":record.native_turn.as_deref().map(|turn|self.source_admission(&record,turn)).transpose()?.flatten(),
            "source_execution":record.native_turn.as_deref().map(|turn|self.mcp_host.source_execution(&record.thread,turn)).transpose()?.flatten(),
            "source_participation":record.native_turn.as_deref().map(|turn|self.source_participation(&record,turn)).transpose()?,
            "host_state":if observed{record.host_last_state.as_str()}else{"unknown"},"driver_phase":record.driver_phase,"last_observed_host_state":record.host_last_state,"host_observed_at":record.host_observed_at,"host_observation_available":observed,"host_progress":"unknown","host_observation":host_observation,"completed":record.phase=="completed"}),
        )
    }
    async fn transfer(
        &self,
        args: Value,
        receipt: Option<&(String, String)>,
    ) -> Result<Value, Error> {
        let (record, native) = self.binding(&args, false).await?;
        if args["controller"].as_str() != Some(&record.capability) {
            return Err(Error("operator_controller_fenced"));
        }
        let history = Self::history(&native, &record.thread).await?;
        let context = native.public_context(&record.thread).await?;
        let cursor: Cursor = serde_json::from_value(args["cursor"].clone())
            .map_err(|_| Error("operator_cursor_invalid"))?;
        let context_cursor: Cursor = serde_json::from_value(args["context_cursor"].clone())
            .map_err(|_| Error("operator_cursor_invalid"))?;
        if cursor != Self::cursor(&history) || context_cursor != Self::cursor(&context) {
            return Err(Error("native_history_boundary_changed"));
        }
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":false}),
            )
            .await?;
        if state["thread"]["status"]["type"] != "idle"
            || !record.pending.is_empty()
            || !self.broker.unsettled(&record.token).await?.is_empty()
            || !self.owned_jobs(&native, &record).await?.is_empty()
        {
            return Err(Error("operator_pending_handoff"));
        }
        let key = digest(&("work-route-transfer-v1", &record.id));
        if args["action"] == "transfer_accept" {
            let mut transfer = self
                .journal
                .read::<Value>(&key)?
                .ok_or(Error("operator_transfer_unavailable"))?;
            if record.phase != "transfer_prepared"
                || transfer["generation"] != record.generation
                || transfer["cursor"] != json!(cursor)
                || transfer["context_cursor"] != json!(context_cursor)
                || transfer["receipt"] != args["transfer_receipt"]
                || !args["transfer_receipt"].is_string()
            {
                return Err(Error("operator_transfer_receipt_invalid"));
            }
            let selection: Selection = serde_json::from_value(transfer["selection"].clone())
                .map_err(|_| Error("operator_record_invalid"))?;
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.phase != "transfer_prepared"
                || work.record.generation != record.generation
                || work.record.capability != record.capability
                || work.stops_in_progress != 0
            {
                return Err(Error("operator_controller_fenced"));
            }
            self.mutation_intent(receipt)?;
            transfer["accepted"] = json!(true);
            transfer["accepted_at_unix_ms"] = json!(unix_ms());
            // A durable offer alone never changes the route. Any crash between
            // these writes is fenced by startup reconciliation, never replayed.
            self.journal.write(&key, &transfer)?;
            work.record.selection = Some(selection);
            work.record.phase = "completed".into();
            self.persist(&work.record)?;
            return Ok(
                json!({"work_id":record.id,"thread_id":record.thread,"accepted":true,"selection":work.record.selection,"generation":record.generation,"inference_submitted":false}),
            );
        }
        if record.phase != "completed" {
            return Err(Error("operator_transfer_boundary_required"));
        }
        let selection = Selection {
            reasoning: serde_json::from_value(args["reasoning"].clone())
                .map_err(|_| Error("reasoning_selection_required"))?,
            return_path: args
                .get("return_path")
                .map(|v| {
                    serde_json::from_value(v.clone())
                        .map_err(|_| Error("operator_arguments_invalid"))
                })
                .transpose()?
                .unwrap_or(ReturnPath::Mcp),
        };
        if selection.reasoning == Reasoning::Direct && selection.return_path != ReturnPath::Mcp {
            return Err(Error("direct_requires_mcp"));
        }
        if selection.reasoning == Reasoning::Provider
            && self.config.history.mode != crate::config::HistoryMode::Project
        {
            return Err(Error("provider_project_required"));
        }
        let summary = text(&args, "text", 256 * 1024)?;
        // Reconcile and stop the old owned host lane before offering a new
        // generation. The saved source is retained, never adopted by title.
        if record.selection.as_ref().is_some_and(|s| {
            s.reasoning == Reasoning::Provider && s.return_path != selection.return_path
        }) && crate::app::inspect_lane(&self.journal.root, &record.thread).is_ok()
        {
            let host = if record.selection.as_ref().unwrap().return_path == ReturnPath::Mcp {
                &self.mcp_host
            } else {
                &self.local_host
            };
            host.detach_settled(&record.thread).await?;
        }
        if record.driver.is_some() {
            self.stop_driver(&record).await?;
        }
        let mut works = self.works.lock().await;
        let work = works.get_mut(&record.thread).unwrap();
        if work.record.phase != "completed"
            || work.record.generation != record.generation
            || work.record.capability != record.capability
            || work.active.is_some()
            || work.stops_in_progress != 0
        {
            return Err(Error("operator_controller_fenced"));
        }
        self.mutation_intent(receipt)?;
        self.preserve_controller(&work.record)?;
        work.record.generation += 1;
        work.record.capability = random_id()?;
        work.record.accepted = false;
        work.record.phase = "transfer_prepared".into();
        work.record.driver = None;
        work.record.driver_phase = "transfer_prepared".into();
        let offer = json!({"work_id":record.id,"thread_id":record.thread,"generation":work.record.generation,"selection":selection,"cursor":cursor,"context_cursor":context_cursor,"summary":summary,"receipt":digest(&(&record.id,work.record.generation,&selection,&cursor,&context_cursor,summary)),"accepted":false,"offered_at_unix_ms":unix_ms()});
        self.journal.write(&key, &offer)?;
        self.persist(&work.record)?;
        Ok(
            json!({"offer":offer,"controller":work.record.capability,"source_fenced":true,"destination_accepted":false,"inference_submitted":false}),
        )
    }
    async fn stop(&self, args: Value) -> Result<Value, Error> {
        let (mut record, native) = self.binding(&args, false).await?;
        let admission = {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if !same_native_round(&record, &work.record) {
                return Err(Error("native_observation_superseded"));
            }
            work.record.accepted = false;
            if let Some(observer) = work.completion_observer.take() {
                observer.cancel();
            }
            if let Some(cancel) = &work.app_cancel {
                cancel.cancel();
            }
            work.record.phase = "stop_requested".into();
            self.persist(&work.record)?;
            work.stops_in_progress += 1;
            record = work.record.clone();
            work.native_admission.clone()
        };
        // Fence immediately, then drain an admitted start/compact/answer RPC.
        // Stop never waits behind a report/finish observer or another work.
        let _admission = admission.lock_owned().await;
        let native_result = tokio::time::timeout(Duration::from_secs(15), async {
            let mut state = native
                .call(
                    "thread/read",
                    json!({"threadId":record.thread,"includeTurns":false}),
                )
                .await?;
            if state["thread"]["status"]["type"] == "notLoaded" {
                let pool = self.work_pool(&record).await?;
                native.verify_scope(&record.thread, &pool.workspace).await?;
                native
                    .call(
                        "thread/resume",
                        json!({"threadId":record.thread,"cwd":pool.workspace,"config":pool.routing()?,"excludeTurns":true}),
                    )
                    .await?;
                native.verify_scope(&record.thread, &pool.workspace).await?;
                state = native
                    .call(
                        "thread/read",
                        json!({"threadId":record.thread,"includeTurns":false}),
                    )
                    .await?;
            }
            // A never-started idle thread has no turn inventory to interrupt.
            // Do not require unsupported full-history reads to confirm its Stop.
            if state["thread"]["status"]["type"] != "idle" {
                let history = native
                    .call(
                        "thread/read",
                        json!({"threadId":record.thread,"includeTurns":true}),
                    )
                    .await?;
                for turn in history["thread"]["turns"]
                    .as_array()
                    .ok_or(Error("native_history_unavailable"))?
                {
                    if turn["status"] == "inProgress" {
                        if let Err(error) = native
                            .call(
                                "turn/interrupt",
                                json!({"threadId":record.thread,"turnId":turn["id"]}),
                            )
                            .await
                        {
                            if error != Error("native_request_rejected") {
                                return Err(error);
                            }
                            // The turn can finish between the history read and
                            // interrupt. Confirm settlement without sending it
                            // again; an active/unknown thread keeps the failure.
                            let latest = native.call("thread/read", json!({"threadId":record.thread,"includeTurns":false})).await?;
                            if !matches!(latest["thread"]["status"]["type"].as_str(), Some("idle" | "systemError")) {
                                return Err(error);
                            }
                        }
                    }
                }
            }
            for job in self.owned_jobs(&native, &record).await? {
                native
                    .call(
                        "thread/backgroundTerminals/terminate",
                        json!({"threadId":record.thread,"processId":job["processId"]}),
                    )
                    .await?;
            }
            loop {
                let state = native
                    .call(
                        "thread/read",
                        json!({"threadId":record.thread,"includeTurns":false}),
                    )
                    .await?;
                if matches!(
                    state["thread"]["status"]["type"].as_str(),
                    Some("idle" | "systemError")
                ) && self.owned_jobs(&native, &record).await?.is_empty()
                {
                    return Ok::<_, Error>(());
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;
        let confirmed = matches!(native_result, Ok(Ok(())));
        let app_lane = self
            .app_route(&record, record.native_turn.as_deref())?
            .map(|_| crate::app::inspect_lane(&self.journal.root, &record.thread));
        let host_requested = record.driver.is_some()
            || app_lane.as_ref().is_some_and(|lane| {
                lane.as_ref()
                    .map(app_lane_requires_host_stop)
                    .unwrap_or(true)
            });
        let host_confirmed = if host_requested || app_lane.is_some() {
            // The response worker can finish acknowledged app cancellation
            // while native interruption is being observed. Its owner must
            // confirm/reuse that settlement even if no new Stop is needed.
            self.stop_driver(&record).await.unwrap_or(false)
        } else {
            // Absence is not confirmation. Preserve a previously observed and
            // durably released Stop without recreating the old host lane.
            record.driver_phase == "released_after_stop"
        };
        // Detach pending model observers without claiming uncertain calls executed.
        let _ = self.broker.retire(&record.token, true).await;
        let jobs = self.owned_jobs(&native, &record).await;
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            work.stops_in_progress -= 1;
            if confirmed
                && (!host_requested || host_confirmed)
                && jobs.as_ref().is_ok_and(Vec::is_empty)
                && work.stops_in_progress == 0
                && work.record.phase == "stop_requested"
            {
                work.record.phase = "stopped".into();
                self.persist(&work.record)?;
            }
        }
        Ok(
            json!({"fenced":true,"native_stop_requested":true,"native_stop_confirmed":confirmed,
            "native_stop_state":if confirmed{"confirmed"}else{"unknown"},"host_stop_requested":host_requested,"host_stop_confirmed":host_confirmed,
            "remaining_jobs":jobs.as_ref().ok(),"jobs_observation_available":jobs.is_ok()}),
        )
    }
    async fn handoff(&self, args: Value) -> Result<Value, Error> {
        let call = text(&args, "call_id", 160)?;
        let key = digest(&("operator-handoff-call-v1", call, &args["work_id"]));
        let fingerprint = digest(&args);
        if let Some(result) = operation::recover(
            &self.journal,
            &key,
            &fingerprint,
            "operator_handoff_reconciliation_required",
        )? {
            return Ok(result);
        }
        let (record, native) = self.binding(&args, true).await?;
        if !record.pending.is_empty() {
            return Err(Error("operator_pending_handoff"));
        }
        if !self.owned_jobs(&native, &record).await?.is_empty() {
            return Err(Error("operator_background_jobs_unsettled"));
        }
        let summary = text(&args, "summary", 256 * 1024)?;
        self.ensure_public_text(summary).await?;
        let cursor: Cursor = serde_json::from_value(args["cursor"].clone())
            .map_err(|_| Error("operator_cursor_invalid"))?;
        let history = Self::history(&native, &record.thread).await?;
        if !Self::covered(&history, &cursor) {
            return Err(Error("native_history_boundary_changed"));
        }
        let context = native.public_context(&record.thread).await?;
        let context_cursor: Cursor = serde_json::from_value(args["context_cursor"].clone())
            .map_err(|_| Error("operator_cursor_invalid"))?;
        if !Self::covered(&context, &context_cursor) {
            return Err(Error("native_context_boundary_changed"));
        }
        let id = format!("handoff_{}", random_id()?);
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability
                || !work.record.accepted
                || work.record.phase != "running"
            {
                return Err(Error("operator_controller_fenced"));
            }
            if !self.broker.unsettled(&record.token).await?.is_empty() {
                return Err(Error("operator_pending_handoff"));
            }
            operation::intent(&self.journal, &key, &fingerprint)?;
            work.record.accepted = false;
            work.record.phase = "handoff".into();
            self.persist(&work.record)?;
        }
        self.emit(&record, id.clone(), format!("{HANDOFF}{summary}"), false)
            .await?;
        self.wait_item(&native, &record, &id).await?;
        let next = {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability
                || work.record.generation != record.generation
                || work.record.phase != "handoff"
                || work.stops_in_progress != 0
            {
                return Err(Error("operator_controller_fenced"));
            }
            self.preserve_controller(&work.record)?;
            work.record.generation += 1;
            work.record.capability = random_id()?;
            work.record.summary = Some(id);
            work.record.source = Some(cursor);
            work.record.source_context = Some(context_cursor);
            work.record.driver_phase = "handoff_prepared".into();
            self.persist(&work.record)?;
            work.record.clone()
        };
        if args["renew"] == true {
            let this = self.clone();
            let request = self
                .works
                .lock()
                .await
                .get(&record.thread)
                .and_then(|v| v.request.clone())
                .ok_or(Error("native_round_not_ready"))?;
            let thread = record.thread.clone();
            let generation = next.generation;
            tokio::spawn(async move {
                let result = async {
                    // A local function must return its correlated result before
                    // stopping the source. Never resubmit it after fencing/Stop.
                    let gate = digest(&(
                        "operator-local-renew-gate-v1",
                        &record.id,
                        &record.capability,
                    ));
                    if this.journal.read::<Value>(&gate)?.is_some() {
                        loop {
                            if this
                                .works
                                .lock()
                                .await
                                .get(&record.thread)
                                .is_none_or(|work| {
                                    work.record.generation != generation
                                        || work.record.phase != "handoff"
                                })
                            {
                                return Err(Error("operator_controller_fenced"));
                            }
                            if this
                                .journal
                                .read::<Value>(&gate)?
                                .is_some_and(|v| v["returned"] == true)
                            {
                                let snapshot = this
                                    .bridge
                                    .operation(json!({"op":"snapshot","lane":record.driver}))
                                    .await?;
                                if snapshot["error"].is_string() {
                                    return Err(Error("operator_local_return_unsettled"));
                                }
                                if host_settled(&snapshot) {
                                    break;
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    }
                    if record.driver.is_some() {
                        this.stop_driver(&record).await?;
                    }
                    this.start_driver(next, request).await
                }
                .await;
                if let Err(error) = result {
                    eprintln!("operator_handoff_start_failed: {}", error.0);
                    let mut works = this.works.lock().await;
                    let work = works.get_mut(&record.thread).unwrap();
                    if work.record.generation == generation {
                        work.record.driver_phase = "handoff_start_unconfirmed".into();
                        work.record.accepted = false;
                        let _ = this.persist(&work.record);
                    }
                }
            });
            let result = json!({"prepared":true,"successor_requested":true,"thread_id":thread});
            operation::settle(&self.journal, &key, &fingerprint, &result)?;
            Ok(result)
        } else {
            let result = json!({"prepared":true,"thread_id":record.thread,"controller":next.capability,"generation":next.generation,"successor_requested":false});
            operation::settle(&self.journal, &key, &fingerprint, &result)?;
            Ok(result)
        }
    }
    async fn driver_admitted(&self, record: &Record) -> Result<(), Error> {
        let works = self.works.lock().await;
        let work = works
            .get(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?;
        if work.record.capability != record.capability
            || !matches!(
                work.record.phase.as_str(),
                "running" | "native_summary" | "handoff"
            )
        {
            return Err(Error("operator_controller_fenced"));
        }
        Ok(())
    }
    async fn start_driver(&self, record: Record, request: Request) -> Result<(), Error> {
        let driver_lock = self
            .works
            .lock()
            .await
            .get(&record.thread)
            .ok_or(Error("operator_work_unavailable"))?
            .driver_lock
            .clone();
        let _guard = driver_lock.lock().await;
        self.driver_admitted(&record).await?;
        let reuse = record.driver.is_some()
            && !matches!(
                record.driver_phase.as_str(),
                "handoff_prepared" | "recover_before_start"
            );
        let marker = if reuse {
            record.driver.clone().unwrap()
        } else {
            random_id()?
        };
        if reuse {
            {
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.capability != record.capability {
                    return Err(Error("operator_controller_fenced"));
                }
                work.record.driver_phase = "waiting_previous_completion".into();
                self.persist(&work.record)?;
            }
            loop {
                // Stop fences the record before acquiring driver_lock. Recheck
                // it during this wait so slow remote completion cannot block Stop.
                self.driver_admitted(&record).await?;
                let snapshot = tokio::time::timeout(
                    Duration::from_secs(10),
                    self.bridge
                        .operation(json!({"op":"snapshot","lane":marker})),
                )
                .await
                .map_err(|_| Error("app_previous_round_unobservable"))??;
                if snapshot["error"].is_string() || snapshot["stopped"] == true {
                    return Err(Error("app_previous_round_unsettled"));
                }
                if host_settled(&snapshot) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        let observed = self
            .bridge
            .operation(json!({"op":"identity","lane":marker}))
            .await?;
        let identity = digest(&(
            &observed["generation"],
            &observed["account"],
            &observed["user"],
            &observed["structure"],
        ));
        if record.identity.as_ref().is_some_and(|v| *v != identity) {
            return Err(Error("app_account_changed"));
        }
        if !observed["account"].is_string() || !observed["user"].is_string() {
            return Err(Error("app_identity_unavailable"));
        }
        crate::app::Identity::parse(&observed)?;
        let project = crate::project::resolve(
            self.bridge.as_ref(),
            &self.journal,
            &request.history,
            &observed,
        )
        .await?;
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability
                || !matches!(
                    work.record.phase.as_str(),
                    "running" | "native_summary" | "handoff"
                )
            {
                return Err(Error("operator_controller_fenced"));
            }
            work.record.driver = Some(marker.clone());
            work.record.driver_phase = "opening".into();
            work.record.identity = Some(identity);
            self.persist(&work.record)?;
        }
        let local = self.config.transport.return_path == ReturnPath::LocalFunction;
        let gateway = format!("operator_local_{marker}");
        let signature = if local {
            local_signature(&gateway)
        } else {
            json!({"name":"operator_no_local_tools","description":"Use MCP Codex controls","parameters":{"type":"object","properties":{}}})
        };
        self.bridge
            .operation(json!({"op":"open","lane":marker,"signature":signature,"project":project}))
            .await?;
        let mut instructions = request
            .instructions
            .iter()
            .map(context::text)
            .collect::<Result<Vec<_>, _>>()?;
        let transport = if local {
            format!("the declared local {gateway} function. Call it with name=<control name> and input=<JSON string encoding its control arguments object>. These controls are the same native Operator Host, not app tools. Control schemas: {}. Do not use an MCP connector",serde_json::to_string(&tools()).unwrap())
        } else {
            format!(
                "the installed {} MCP connector. Do not use local functions",
                self.config.transport.connector_name
            )
        };
        instructions.push(format!("Operate local Codex through {transport}. Use codex_attach with work_id={} and controller={}. Read its current history and echo its receipt as accept before effects. Discover full native tools with codex_tools, run them with codex_execute, publish initial/intermediate reports with codex_report, and explicitly codex_finish after actual results. Context renewal uses codex_history then codex_handoff with the confirmed cursor and context_cursor, grounded summary and renew=true. After a successful handoff, end this source Chat turn without further controls; the successor continues. {DIRECT_PROGRESS} {DIRECT_COMPLETION} The same native thread and original return remain bound. If native_context=native_summary, supply codex_native_summary with a unique stable call_id for that request, without replacing Chat. Never invent execution receipts, create another primary thread or reveal private reasoning. Exact retries reuse logical call IDs; uncertain actions are reconciled, never resent.",record.id,record.capability));
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability
                || !matches!(
                    work.record.phase.as_str(),
                    "running" | "native_summary" | "handoff"
                )
            {
                return Err(Error("operator_controller_fenced"));
            }
            work.record.driver_phase = "dispatching".into();
            self.persist(&work.record)?;
        }
        // The initial genuine user request is not origin/context metadata.
        // Fresh successors and summaries still bootstrap without another user turn.
        let bootstrap =
            !reuse && record.driver_phase == "handoff_prepared" || request.compaction.is_some();
        let messages = if !bootstrap {
            let user = request
                .input
                .iter()
                .rfind(|item| item["role"] == "user")
                .ok_or(Error("native_user_input_missing"))?;
            json!([{"role":"user","text":context::text(user)?,"hidden":false}])
        } else {
            json!([{"role":"developer","text":"Use the Codex attachment described in the control instructions and continue its current task.","hidden":true}])
        };
        self.bridge.operation(json!({"op":"submit","lane":marker,"round":random_id()?,"model":request.host_model,"thinkingEffort":request.thinking_effort,
            "messages":messages,"instructions":instructions,"results":[],"bootstrap":bootstrap})).await?;
        if local {
            let this = self.clone();
            let record = record.clone();
            let marker = marker.clone();
            tokio::spawn(async move {
                if let Err(error) = this.drive_local(record.clone(), marker).await {
                    eprintln!("operator_local_transport_failed: {}", error.0);
                    let mut works = this.works.lock().await;
                    if let Some(work) = works.get_mut(&record.thread).filter(|v| {
                        v.record.capability == record.capability && v.record.phase != "completed"
                    }) {
                        work.record.driver_phase = "start_unknown".into();
                        work.record.accepted = false;
                        let _ = this.persist(&work.record);
                    }
                }
            });
        }
        let mut works = self.works.lock().await;
        let work = works.get_mut(&record.thread).unwrap();
        if work.record.capability != record.capability
            || !matches!(
                work.record.phase.as_str(),
                "running" | "native_summary" | "handoff"
            )
        {
            return Ok(());
        }
        work.record.driver_phase = "started_observation_unknown".into();
        self.persist(&work.record)
    }
    fn drive_local(&self, record: Record, marker: String) -> BoxFuture<'static, Result<(), Error>> {
        let this = self.clone();
        Box::pin(async move {
            let recipient = format!("local.operator_local_{marker}");
            let mut corrections = 0u8;
            loop {
                let latest = this
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .ok_or(Error("operator_work_unavailable"))?
                    .record
                    .clone();
                if latest.capability != record.capability
                    || latest.driver.as_ref() != Some(&marker)
                    || matches!(
                        latest.phase.as_str(),
                        "completed" | "stop_requested" | "reconciliation_required"
                    )
                {
                    return Ok(());
                }
                this.verify_identity(&latest).await?;
                let snapshot = this
                    .bridge
                    .operation(json!({"op":"snapshot","lane":marker}))
                    .await?;
                if snapshot["error"].is_string() {
                    return Err(Error("operator_app_round_failed"));
                }
                if snapshot["stopped"] == true {
                    return Ok(());
                }
                if !host_settled(&snapshot) {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                let mut calls = Vec::new();
                for message in snapshot["messages"]
                    .as_array()
                    .ok_or(Error("invalid_app_snapshot"))?
                    .iter()
                    .filter(|v| v["author"]["role"] == "assistant" && v["recipient"] == recipient)
                {
                    let id = text(message, "id", 256)?;
                    let key = digest(&("operator-local-call-v1", &marker, id));
                    let fingerprint = digest(&message["content"]["text"]);
                    if local_return_seen(this.journal.read::<Value>(&key)?.as_ref(), &fingerprint)?
                    {
                        continue;
                    }
                    calls.push(message);
                }
                if calls.is_empty() {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                if calls.len() != 1 {
                    return Err(Error("operator_local_calls_ambiguous"));
                }
                let message = calls[0];
                if message["status"] != "finished_successfully" {
                    return Err(Error("incomplete_host_call"));
                }
                let id = text(message, "id", 256)?.to_owned();
                let fingerprint = digest(&message["content"]["text"]);
                let parsed = local_arguments(message);
                let (name, args) = parsed
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|_| ("invalid_operator_gateway".into(), Value::Null));
                let key = digest(&("operator-local-call-v1", &marker, &id));
                // No replay after an observer/process failure, including the
                // uncertain interval between native effects and app submission.
                if this.journal.read::<Value>(&key)?.is_some() {
                    return Err(Error("operator_local_reconciliation_required"));
                }
                this.journal.write(
                    &key,
                    &json!({"fingerprint":fingerprint,"phase":if parsed.is_ok(){"invoking"}else{"rejected_before_execution"},"name":name}),
                )?;
                let gate = digest(&(
                    "operator-local-renew-gate-v1",
                    &record.id,
                    &record.capability,
                ));
                let renew = name == "codex_handoff" && args["renew"] == true;
                if renew {
                    this.journal
                        .write(&gate, &json!({"returned":false,"call":id}))?;
                }
                let result = match parsed {
                    Ok(_) => {
                        corrections = 0;
                        match this.call(&name, args).await {
                            Ok(value) => value,
                            Err(error) => json!({"error":{"code":error.0}}),
                        }
                    }
                    Err(error) => {
                        corrections += 1;
                        if corrections > 4 {
                            return Err(Error("operator_gateway_correction_limit"));
                        }
                        error
                    }
                };
                this.journal.write(&key,&json!({"fingerprint":fingerprint,"phase":"return_prepared","name":name,"result":result}))?;
                let current = this
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .unwrap()
                    .record
                    .clone();
                let handing_off = renew && result["prepared"] == true;
                if (!handing_off && current.capability != record.capability)
                    || matches!(
                        current.phase.as_str(),
                        "stop_requested" | "reconciliation_required"
                    )
                {
                    return Err(Error("operator_controller_fenced"));
                }
                // The same driver_lock used by Stop protects the final check
                // and dispatch; Stop fences before waiting for this lock.
                let lock = this
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .unwrap()
                    .driver_lock
                    .clone();
                let _guard = lock.lock().await;
                let current = this
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .unwrap()
                    .record
                    .clone();
                if matches!(
                    current.phase.as_str(),
                    "stop_requested" | "reconciliation_required"
                ) {
                    return Err(Error("operator_controller_fenced"));
                }
                let request = this
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .unwrap()
                    .request
                    .clone()
                    .ok_or(Error("native_round_not_ready"))?;
                this.bridge.operation(json!({"op":"submit","lane":marker,"round":random_id()?,"model":request.host_model,"thinkingEffort":request.thinking_effort,"messages":[],"instructions":[],"results":[{"id":id,"result":result}]})).await?;
                this.journal.write(&key,&json!({"fingerprint":fingerprint,"phase":"returned","name":name,"result":result}))?;
                if handing_off {
                    this.journal
                        .write(&gate, &json!({"returned":true,"call":id}))?;
                    return Ok(());
                }
            }
        })
    }
    async fn exchange_inner(
        &self,
        request: Request,
        events: mpsc::Sender<Event>,
        cancel: CancellationToken,
        native: Arc<Native>,
        pool: Arc<Pool>,
    ) -> Result<(), Error> {
        let existed = self.works.lock().await.contains_key(&request.thread);
        let mut record = self
            .register_in(&request.thread, "codex", Some(&request), pool, None, None)
            .await?;
        if self
            .works
            .lock()
            .await
            .get(&request.thread)
            .unwrap()
            .stops_in_progress
            != 0
        {
            return Err(Error("operator_controller_fenced"));
        }
        if request.compaction.is_none() {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&request.thread).unwrap();
            if let Some(checkpoint) = &work.checkpoint {
                if !checkpoint.acknowledged(&request) {
                    return Err(Error("native_summary_commit_unconfirmed"));
                }
                work.checkpoint = None;
                if work.record.phase == "native_summary_offered" {
                    work.record.phase = "completed".into();
                }
                self.persist(&work.record)?;
                record = work.record.clone();
            }
        }
        let manual_compact = record.phase == "native_compacting";
        let recovery = matches!(
            record.phase.as_str(),
            "reconciliation_required" | "stop_requested" | "stopped"
        );
        let new_user_turn = (matches!(record.phase.as_str(), "completed" | "finishing")
            || recovery)
            && request.turn.is_some()
            && request.turn != record.native_turn
            && request.compaction.is_none();
        if recovery && new_user_turn {
            self.recovery_receipts(&native, &record).await?;
        }
        if new_user_turn {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&request.thread).unwrap();
            if work.stops_in_progress != 0 {
                return Err(Error("operator_controller_fenced"));
            }
            self.preserve_controller(&work.record)?;
            let _ = self.broker.retire(&record.token, recovery).await;
            work.record.capability = random_id()?;
            work.record.generation += 1;
            work.record.token = self
                .broker
                .register(&record.id, request.tools.clone(), false)
                .await?;
            work.record.accepted = false;
            work.record.phase = "waiting_native".into();
            work.record.origin = "codex".into();
            work.record.pending.clear();
            if recovery && work.record.driver.is_some() {
                work.record.driver_phase = "recover_before_start".into();
            }
            self.persist(&work.record)?;
            record = work.record.clone();
        }
        if matches!(
            record.phase.as_str(),
            "reconciliation_required" | "stop_requested" | "completed" | "stopped"
        ) {
            return Err(Error("operator_reconciliation_required"));
        }
        let received = request
            .results()
            .filter(|(id, _)| record.pending.iter().any(|v| v == id))
            .map(|(id, v)| (id.to_owned(), v.clone()))
            .collect::<Vec<_>>();
        if received.len() != record.pending.len() {
            return Err(Error("native_tool_result_missing"));
        }
        let (sender, mut commands) = mpsc::channel(64);
        let round = random_id()?;
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&request.thread).unwrap();
            if work.active.is_some() {
                return Err(Error("native_lane_busy"));
            }
            if work.stops_in_progress != 0 {
                return Err(Error("operator_controller_fenced"));
            }
            self.broker
                .settle_with_catalog(&record.token, &received, Some(request.tools.clone()))
                .await?;
            work.record.pending.clear();
            if work.record.phase != "handoff" {
                work.record.phase = if request.compaction.is_some() {
                    "native_summary"
                } else {
                    "running"
                }
                .into();
            }
            work.active = Some(sender);
            work.round = round.clone();
            work.request = Some(request.clone());
            if request.compaction.is_none() && request.turn.is_some() {
                work.record.native_turn = request.turn.clone();
            }
            self.persist(&work.record)?;
        }
        if record
            .selection
            .as_ref()
            .is_none_or(|s| s.reasoning != Reasoning::Direct)
            && (!existed || new_user_turn || (manual_compact && record.driver.is_some()))
        {
            let this = self.clone();
            let record = record.clone();
            let request = request.clone();
            tokio::spawn(async move {
                let result = async {
                    if record.driver_phase == "recover_before_start" {
                        this.stop_driver(&record).await?;
                    }
                    this.start_driver(record.clone(), request).await
                }
                .await;
                if let Err(error) = result {
                    eprintln!("operator_driver_start_failed: {}", error.0);
                    let mut works = this.works.lock().await;
                    let work = works.get_mut(&record.thread).unwrap();
                    if work.record.capability == record.capability {
                        work.record.driver_phase = "start_unknown".into();
                        work.record.accepted = false;
                        let _ = this.persist(&work.record);
                    }
                }
            });
        }
        let mut interval = tokio::time::interval(Duration::from_millis(20));
        let mut host_interval = tokio::time::interval(Duration::from_secs(1));
        let mut reported_wait = None;
        let result = async { loop {
            tokio::select! {
                _=cancel.cancelled()=>break Err(Error("native_observer_lost")),
                _=host_interval.tick()=>{
                    let latest={let works=self.works.lock().await;let work=works.get(&request.thread).unwrap();
                        if work.round!=round {break Err(Error("operator_controller_fenced"));}
                        work.record.clone()};
                    if let Some(error)=driver_round_error(&latest) {break Err(error);}
                    if matches!(latest.phase.as_str(),"running"|"native_summary") {
                        if let Some(driver)=latest.driver.as_ref().filter(|_|latest.driver_phase=="started_observation_unknown") {
                            let snapshot=tokio::time::timeout(Duration::from_secs(3),self.bridge.operation(json!({"op":"snapshot","lane":driver}))).await;
                            if let Ok(Ok(snapshot))=snapshot {
                                let waiting=host_waiting(&snapshot);
                                if waiting!=reported_wait {
                                    reported_wait=waiting;
                                    if let Some(reason)=waiting {
                                        let text=if reason=="mcp_host_confirmation_required" {"ChatGPT is waiting for tool permission in its app. Approve or deny there; work remains pending."} else {"ChatGPT is waiting for authentication in its app. Work remains pending."};
                                        events.send(Event::Text{id:format!("host_wait_{}",random_id()?),phase:"commentary".into(),delta:text.into()}).await.map_err(|_|Error("native_observer_lost"))?;
                                        events.send(Event::Flush).await.map_err(|_|Error("native_observer_lost"))?;
                                    }
                                }
                                let failure=if snapshot["error"].is_string() {
                                    Some(("operator_app_round_failed","ChatGPT app reported a failure. Work is preserved; reconcile before retrying."))
                                } else if waiting.is_none() && host_settled(&snapshot)
                                    && snapshot["messages"].as_array().is_some_and(|messages|messages.iter().any(|message|
                                        message["author"]["role"]=="assistant" && message["channel"]=="final"
                                        && message["content"]["content_type"]=="text" && message["end_turn"]==true
                                        && message["status"]=="finished_successfully")) {
                                    Some(("operator_app_round_incomplete","ChatGPT turn ended without completing the Codex control round. Work is preserved; reconcile before retrying."))
                                } else {None};
                                if let Some((code,text))=failure {
                                    events.send(Event::Text{id:format!("host_error_{}",random_id()?),phase:"commentary".into(),delta:text.into()}).await.map_err(|_|Error("native_observer_lost"))?;
                                    events.send(Event::Flush).await.map_err(|_|Error("native_observer_lost"))?;
                                    break Err(Error(code));
                                }
                            }
                        }
                    }
                },
                command=commands.recv()=>match command {
                    Some(Command::Text{capability,id,text,final_answer,ack})=>{
                        let valid={let works=self.works.lock().await;let work=works.get(&request.thread).unwrap();
                            work.record.capability==capability && ((work.record.accepted && matches!(work.record.phase.as_str(),"running"|"finishing")) || (work.record.phase=="handoff" && id.starts_with("handoff_")))};
                        if !valid {let _=ack.send(Err(Error("operator_controller_fenced")));continue;}
                        let result=async { events.send(Event::Text{id,phase:if final_answer{"final_answer"}else{"commentary"}.into(),delta:text}).await.map_err(|_|Error("native_observer_lost"))?;
                            events.send(Event::Flush).await.map_err(|_|Error("native_observer_lost"))?;
                            if final_answer { self.works.lock().await.get_mut(&request.thread).unwrap().active=None; events.send(Event::Completed{input_tokens:0,output_tokens:0}).await.map_err(|_|Error("native_observer_lost"))?; }
                            Ok(()) }.await;
                        let failed=result.is_err(); let _=ack.send(result); if failed {break Err(Error("native_delivery_unknown"));}
                        if final_answer { break Ok(()); }
                    },
                    Some(Command::Summary{capability,round:summary_round,text,ack})=>{
                        let valid={let works=self.works.lock().await;let work=works.get(&request.thread).unwrap();work.record.capability==capability && work.round==summary_round && work.record.accepted && work.record.phase=="native_summary"};
                        if !valid {let _=ack.send(Err(Error("operator_controller_fenced")));continue;}
                        let result=async { let checkpoint=context::Checkpoint::offer(&request,&text,&summary_round)?;
                            self.works.lock().await.get_mut(&request.thread).unwrap().checkpoint=Some(checkpoint.clone());
                            events.send(checkpoint.event(format!("checkpoint_{summary_round}"))).await.map_err(|_|Error("native_observer_lost"))?;
                            {let mut works=self.works.lock().await;let work=works.get_mut(&request.thread).unwrap();work.active=None;if manual_compact{work.record.phase="native_summary_offered".into();self.persist(&work.record)?;}}
                            events.send(Event::Completed{input_tokens:0,output_tokens:0}).await.map_err(|_|Error("native_observer_lost"))?; Ok(()) }.await;
                        let _=ack.send(result); break Ok(());
                    },
                    None=>break Err(Error("native_observer_lost")),
                },
                _=interval.tick()=>{
                    if request.compaction.is_some() || self.broker.unsettled(&record.token).await?.is_empty() {continue;}
                    let latest=self.works.lock().await.get(&request.thread).unwrap().record.clone();
                    self.verify_identity(&latest).await?;
                    let works=self.works.lock().await;
                    if works.get(&request.thread).is_none_or(|v| !v.record.accepted || v.record.phase!="running") {continue;}
                    let calls=self.broker.offer(&record.token).await?;
                    drop(works);
                    if calls.is_empty(){continue;}
                    let mut output=vec![];
                    for call in &calls { output.extend(gateway_calls(call["id"].as_str().unwrap(),&json!({"name":call["name"],"input":call["input"],"report":call["report"]}).to_string(),&request.tools)?.0); }
                    {let mut works=self.works.lock().await; let work=works.get_mut(&request.thread).unwrap(); if !work.record.accepted || work.record.phase!="running" {break Err(Error("operator_controller_fenced"));} work.record.pending=output.iter().map(|v|v.id.clone()).collect(); work.record.phase="waiting_native".into(); self.persist(&work.record)?;}
                    for call in calls { if let Some(report)=call["report"].as_str().filter(|v|!v.is_empty()){events.send(Event::Text{id:format!("report_{}",call["id"].as_str().unwrap()),phase:"commentary".into(),delta:report.into()}).await.map_err(|_|Error("native_observer_lost"))?;} }
                    for call in output {events.send(Event::Tool(call)).await.map_err(|_|Error("native_observer_lost"))?;}
                    self.works.lock().await.get_mut(&request.thread).unwrap().active=None;
                    events.send(Event::Completed{input_tokens:0,output_tokens:0}).await.map_err(|_|Error("native_observer_lost"))?;
                    break Ok(());
                }
            }
        }}.await;
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&request.thread).unwrap();
            if work.round != round {
                return result;
            }
            work.active = None;
            if result.is_err() {
                // A requested interruption is still awaiting explicit Stop
                // confirmation; the native observer closing must not erase it.
                if !matches!(work.record.phase.as_str(), "stop_requested" | "stopped") {
                    work.record.phase = "reconciliation_required".into();
                }
                work.record.accepted = false;
            }
            self.persist(&work.record)?;
        }
        // A native client disconnect does not prove native jobs or remote Chat stopped.
        let _ = native;
        result
    }
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/mcp", axum::routing::post(http))
            .with_state(self)
    }
    pub async fn close(&self) {
        let mut works = self.works.lock().await;
        for work in works.values_mut() {
            if let Some(observer) = work.completion_observer.take() {
                observer.cancel();
            }
            work.record.accepted = false;
            work.record.phase = recovered_phase(&work.record.phase).to_owned();
            let _ = self.persist(&work.record);
        }
        let drivers = works
            .values()
            .filter(|work| work.record.driver.is_some())
            .map(|work| work.record.clone())
            .collect::<Vec<_>>();
        drop(works);
        self.broker.close().await;
        let mut stops = tokio::task::JoinSet::new();
        for record in drivers {
            let this = self.clone();
            stops.spawn(async move {
                if matches!(
                    tokio::time::timeout(Duration::from_secs(15), this.stop_driver(&record)).await,
                    Ok(Ok(true))
                ) {
                    let released = this
                        .bridge
                        .operation(json!({"op":"release","lane":record.driver}))
                        .await;
                    if matches!(released, Ok(value) if value["released"] == true) {
                        let mut works = this.works.lock().await;
                        let work = works.get_mut(&record.thread).unwrap();
                        if work.record.driver == record.driver {
                            let mut checkpoint = work.record.clone();
                            checkpoint.driver = None;
                            checkpoint.driver_phase = "released_after_stop".into();
                            checkpoint.host_last_state = "stop_confirmed".into();
                            checkpoint.host_observed_at = Some(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                            );
                            if this.persist(&checkpoint).is_ok() {
                                work.record = checkpoint;
                            } else {
                                eprintln!("operator_shutdown_checkpoint_failed");
                            }
                        }
                    }
                }
            });
        }
        while stops.join_next().await.is_some() {}
        self.reviewer.shutdown().await;
        self.mcp_host.shutdown().await;
        self.local_host.shutdown().await;
    }
}
impl Host for Operator {
    fn approve(&self, thread: String, ui: bool) -> BoxFuture<'static, Result<Value, Error>> {
        let this = self.clone();
        Box::pin(async move {
            let record = {
                let works = this.works.lock().await;
                let work = works
                    .get(&thread)
                    .ok_or(Error("operator_work_unavailable"))?;
                work.record.clone()
            };
            if let Some(path) = this.app_route(&record, record.native_turn.as_deref())? {
                if record.phase != "running" {
                    return Err(Error("operator_work_not_running"));
                }
                return this.app_host(path).approve(thread, ui).await;
            }
            if record.phase != "running"
                || this
                    .works
                    .lock()
                    .await
                    .get(&thread)
                    .is_none_or(|w| w.active.is_none())
            {
                return Err(Error("operator_work_not_running"));
            }
            let lane = record.driver.as_ref().ok_or(Error("app_lane_missing"))?;
            let identity = this.verify_identity(&record).await?;
            crate::app::approve_lane(
                this.bridge.clone(),
                lane,
                identity,
                &this.config.transport.connector_name,
                ui,
            )
            .await
        })
    }
    fn reveal(&self, thread: String) -> BoxFuture<'static, Result<(), Error>> {
        let this = self.clone();
        Box::pin(async move {
            let record = this
                .works
                .lock()
                .await
                .get(&thread)
                .ok_or(Error("operator_work_unavailable"))?
                .record
                .clone();
            if let Some(path) = this.app_route(&record, record.native_turn.as_deref())? {
                return this.app_host(path).reveal(thread).await;
            }
            let lane = record.driver.as_ref().ok_or(Error("app_lane_missing"))?;
            let identity = this.verify_identity(&record).await?;
            this.bridge.operation(json!({"op":"reveal","lane":lane,"identity":{"account":identity["account"],"user":identity["user"],"structure":identity["structure"]}})).await?;
            Ok(())
        })
    }
    fn exchange(
        &self,
        request: Request,
        events: mpsc::Sender<Event>,
        cancel: CancellationToken,
    ) -> BoxFuture<'static, Result<(), Error>> {
        if request.guardian {
            return self.reviewer.exchange(request, events, cancel);
        }
        let this = self.clone();
        Box::pin(async move {
            let native = this.pool.discover_thread(&request.thread).await?;
            let workspace = native.workspace(&request.thread).await?;
            let pool = this.scoped_pool(&native.owner.home, &workspace).await?;
            let mut record = this
                .works
                .lock()
                .await
                .get(&request.thread)
                .map(|work| work.record.clone());
            if record.is_none()
                && this.config.execution.mode == crate::config::ExecutionMode::ModelAdapter
            {
                record = Some(
                    this.register_in(
                        &request.thread,
                        "codex",
                        Some(&request),
                        pool.clone(),
                        Some(Selection {
                            reasoning: Reasoning::Provider,
                            return_path: this.config.transport.return_path,
                        }),
                        None,
                    )
                    .await?,
                );
            }
            if let Some(prior) = &record {
                this.admit_native_round(prior, &request, &native, &pool)
                    .await?;
                record = this
                    .works
                    .lock()
                    .await
                    .get(&request.thread)
                    .map(|work| work.record.clone());
            }
            let path = match &record {
                Some(record) => this.app_route(record, request.turn.as_deref())?,
                None if this.config.execution.mode
                    == crate::config::ExecutionMode::ModelAdapter =>
                {
                    Some(this.config.transport.return_path)
                }
                _ => None,
            };
            if let Some(path) = path {
                let thread = request.thread.clone();
                let completion_cancel = CancellationToken::new();
                if let Some(record) = &mut record {
                    let mut works = this.works.lock().await;
                    let work = works.get_mut(&thread).unwrap();
                    // Check and reserve under one lock: Stop cannot fence a
                    // work between admission and installation of its cancel.
                    admit_app_round(work, record, request.turn.as_deref())?;
                    work.app_cancel = Some(cancel.clone());
                    work.record.accepted = false;
                    work.request = Some(request.clone());
                    work.record.native_turn = request.turn.clone();
                    work.record.phase = "running".into();
                    if let Some(previous) =
                        work.completion_observer.replace(completion_cancel.clone())
                    {
                        previous.cancel();
                    }
                    this.persist(&work.record)?;
                    *record = work.record.clone();
                }
                if let Some(record) = &record {
                    this.watch_completion(record.clone(), Some(native.clone()), completion_cancel);
                }
                let deadline = record
                    .as_ref()
                    .and_then(|record| record.assignment.as_ref())
                    .and_then(|assignment| {
                        assignment.started_at_unix_ms.map(|start| {
                            start.saturating_add(
                                assignment.budget.max_elapsed_seconds.saturating_mul(1000),
                            )
                        })
                    });
                let timer = deadline.map(|deadline| {
                    let cancel = cancel.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(
                            deadline.saturating_sub(unix_ms()),
                        ))
                        .await;
                        cancel.cancel();
                    })
                });
                let (sender, mut receiver) = mpsc::channel(64);
                let child_cancel = cancel.clone();
                let forward = async {
                    while let Some(event) = receiver.recv().await {
                        if let Event::Tool(call) = &event {
                            if let Err(error) = this.admit_native_call(&thread, &call.id).await {
                                child_cancel.cancel();
                                return Err(error);
                            }
                        }
                        if events.send(event).await.is_err() {
                            child_cancel.cancel();
                            return Err(Error("native_observer_lost"));
                        }
                    }
                    Ok(())
                };
                let (result, forwarded) = tokio::join!(
                    this.app_host(path).exchange(request, sender, cancel),
                    forward
                );
                if let Some(timer) = timer {
                    timer.abort();
                }
                let result = forwarded.and(result);
                if let Some(record) = &record {
                    let mut works = this.works.lock().await;
                    let work = works.get_mut(&thread).unwrap();
                    if same_native_round(record, &work.record) {
                        work.app_cancel = None;
                        if work.stops_in_progress == 0 && work.record.phase == "running" {
                            work.record.phase = if result.is_ok() {
                                "running"
                            } else if result == Err(Error("bound_chat_turn_interrupted")) {
                                "interrupted"
                            } else {
                                "reconciliation_required"
                            }
                            .into();
                            this.persist(&work.record)?;
                        }
                    }
                }
                result
            } else {
                this.exchange_inner(request, events, cancel, native, pool)
                    .await
            }
        })
    }
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn text_value_path(value: &Value, error: &'static str) -> Result<std::path::PathBuf, Error> {
    let path = std::path::PathBuf::from(value.as_str().ok_or(Error(error))?);
    if !path.is_absolute()
        || std::fs::canonicalize(&path).ok().as_ref() != Some(&path)
        || !path.is_dir()
    {
        return Err(Error(error));
    }
    Ok(path)
}
fn local_return_seen(previous: Option<&Value>, fingerprint: &str) -> Result<bool, Error> {
    let Some(previous) = previous else {
        return Ok(false);
    };
    if previous["fingerprint"] != fingerprint {
        return Err(Error("operator_call_changed"));
    }
    if previous["phase"] == "returned" {
        return Ok(true);
    }
    Err(Error("operator_local_reconciliation_required"))
}
fn local_arguments(message: &Value) -> Result<(String, Value), Value> {
    let rejected = |line: usize, column: usize| json!({"error":{"code":"operator_arguments_invalid","executed":false,"line":line,"column":column,"message":"Rejected before any control or native action. Correct this local call: name must be an exact control name; input must be a JSON string encoding its arguments object. For codex_execute, its inner input is another string: escape quotes/newlines at both JSON layers, or split a large patch. No part of this proposal executed."}});
    let input: Value = serde_json::from_str(
        message["content"]["text"]
            .as_str()
            .ok_or_else(|| rejected(0, 0))?,
    )
    .map_err(|e| rejected(e.line(), e.column()))?;
    let name = text(&input, "name", 128)
        .map_err(|_| rejected(0, 0))?
        .to_owned();
    let args: Value =
        serde_json::from_str(text(&input, "input", LIMIT).map_err(|_| rejected(0, 0))?)
            .map_err(|e| rejected(e.line(), e.column()))?;
    if !args.is_object() {
        return Err(rejected(0, 0));
    }
    Ok((name, args))
}
fn local_signature(gateway: &str) -> Value {
    // This app's local API uses kwargs/params, not OpenAI JSON function schemas.
    // Keep the same string gateway wire type as the qualified model adapter.
    let names = tools()
        .iter()
        .map(|v| v["name"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>()
        .join(", ");
    json!({"name":gateway,"type":"kwargs","description":"Call a native Codex Operator Host control. Native Codex owns execution, approvals and records.","params":[
        {"name":"name","required":true,"type":{"type":"string","description":format!("Exact control name: {names}")}},
        {"name":"input","required":true,"type":{"type":"string","description":"JSON string encoding the selected control's arguments object, using its advertised schema."}}
    ]})
}
fn completed_result(completed: bool, turn: Option<&str>, history: &[Value]) -> Option<Value> {
    if !completed {
        return None;
    }
    let turn = turn?;
    let items: Vec<_> = history
        .iter()
        .filter(|row| {
            row["turn_id"] == turn
                && row["item"]["type"] == "agentMessage"
                && row["item"]["phase"] == "final_answer"
                && row["item"]["text"].as_str().is_some_and(|v| !v.is_empty())
        })
        .map(|row| row["item"].clone())
        .collect();
    (!items.is_empty())
        .then(|| json!({"turn_id":turn,"items":items,"source":"native_thread_items"}))
}
fn source_native_settled(record: &Record, state: &Value) -> bool {
    state["thread"]["status"]["type"] == "idle"
        || record.phase == "interrupted" && state["thread"]["status"]["type"] == "systemError"
}
fn settled_source_turn(record: &Record, turn: &Value) -> bool {
    record.native_turn.is_some()
        && turn["id"].as_str() == record.native_turn.as_deref()
        && match record.phase.as_str() {
            "completed" => turn["status"] == "completed",
            "interrupted" => matches!(turn["status"].as_str(), Some("failed" | "interrupted")),
            _ => false,
        }
}
fn wait_reason(result: &Value) -> Option<&'static str> {
    let status = &result["status"];
    if status["completed"] == true
        && result["history_observation_available"] != false
        && result["completed_result_available"] != false
    {
        Some("completed")
    } else if status["human_requests"]
        .as_array()
        .is_some_and(|v| !v.is_empty())
    {
        Some("human_request")
    } else if matches!(
        status["phase"].as_str(),
        Some("stopped" | "interrupted" | "reconciliation_required" | "handoff")
    ) {
        Some("attention_required")
    } else {
        None
    }
}
fn host_waiting(snapshot: &Value) -> Option<&'static str> {
    if snapshot["error"].is_string() || snapshot["stopped"] == true {
        return None;
    }
    snapshot["messages"]
        .as_array()?
        .iter()
        .find_map(|item| crate::mcp_bridge::host_block(item, &snapshot["hostResponses"]))
}
fn host_settled(snapshot: &Value) -> bool {
    snapshot["accepted"] == true
        && snapshot["pending"] == false
        && snapshot["streamActive"] == false
        && snapshot["approvalPending"] != true
        && !snapshot["error"].is_string()
        && snapshot["stopped"] != true
        && snapshot["terminals"] == json!(["done"])
}
fn text<'a>(args: &'a Value, key: &str, max: usize) -> Result<&'a str, Error> {
    args[key]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= max)
        .ok_or(Error("operator_arguments_invalid"))
}
pub(crate) fn validate_arguments(
    declarations: &[Value],
    name: &str,
    args: &Value,
) -> Result<(), Error> {
    let schema = declarations
        .iter()
        .find(|v| v["name"] == name)
        .ok_or(Error("mcp_tool_unknown"))?["inputSchema"]
        .clone();
    let object = args
        .as_object()
        .ok_or(Error("operator_arguments_invalid"))?;
    if object
        .keys()
        .any(|key| schema["properties"].get(key).is_none())
        || schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|key| !object.contains_key(key.as_str().unwrap()))
    {
        return Err(Error("operator_arguments_invalid"));
    }
    for (key, value) in object {
        let valid = match schema["properties"][key]["type"].as_str() {
            Some("string") => value.is_string(),
            Some("object") => value.is_object(),
            Some("boolean") => value.is_boolean(),
            Some("integer") => value.as_u64().is_some(),
            Some("array") => value.as_array().is_some(),
            _ => false,
        };
        if !valid
            || schema["properties"][key]["enum"]
                .as_array()
                .is_some_and(|allowed| !allowed.contains(value))
        {
            return Err(Error("operator_arguments_invalid"));
        }
    }
    Ok(())
}
pub(crate) fn tools() -> Vec<Value> {
    let field = |name: &str| match name {
        "list_cursor" => {
            json!({"type":"object","properties":{"account":{"type":"string"},"offset":{"type":"integer","minimum":0},"filter":{"type":"string"}},"required":["account","offset","filter"],"additionalProperties":false})
        }
        "reasoning" => json!({"type":"string","enum":["direct","provider"]}),
        "source_cursor" => {
            json!({"type":"object","properties":{"account":{"type":"string"},"conversation":{"type":"string"},"branch":{"type":"string"},"count":{"type":"integer","minimum":0},"prefix":{"type":"string"}},"required":["account","conversation","branch","count","prefix"],"additionalProperties":false})
        }
        "cursor" | "after" | "context_cursor" | "after_context" => {
            json!({"type":"object","properties":{"count":{"type":"integer","minimum":0},"digest":{"type":"string"}},"required":["count","digest"],"additionalProperties":false})
        }
        "renew" => json!({"type":"boolean"}),
        "limit" | "offset" | "updated_after" | "updated_before" | "max_wait_seconds" => {
            json!({"type":"integer","minimum":0})
        }
        "dependencies" => {
            json!({"type":"array","items":{"type":"string"},"maxItems":256,"uniqueItems":true})
        }
        "budget" => {
            json!({"type":"object","properties":{"max_parallel":{"type":"integer","minimum":1,"maximum":32},"max_app_starts":{"type":"integer","minimum":1},"max_native_calls":{"type":"integer","minimum":1},"max_elapsed_seconds":{"type":"integer","minimum":1}},"required":["max_parallel","max_app_starts","max_native_calls","max_elapsed_seconds"],"additionalProperties":false})
        }
        "answers" => {
            json!({"type":"object","additionalProperties":{"type":"object","properties":{"answers":{"type":"array","items":{"type":"string"}}},"required":["answers"],"additionalProperties":false}})
        }
        "action" => {
            json!({"type":"string","enum":["list","create","resume","fork","start","queue","synchronize","source_sync","source_join","claim","compact","takeover","events","wait","chat_input","chat_result","transfer","transfer_accept","bind","unbind","join","evaluate","stop_tree"]})
        }
        _ => json!({"type":"string"}),
    };
    let controls = [
        ("codex_projects", "List authenticated ChatGPT projects without inference. For an explicit request to create or use a project, use action=use with name: create or reuse its owned private destination, read it back, then save history.mode=project and its exact name/id. This completes project creation and selection for new lanes; existing conversations are preserved. Optional project_id selects an exact matching project. Retry the same selection after uncertainty; do not create a duplicate or submit a model task just to create a project.", "action name project_id", "action"),
        ("codex_host", "Observe or explicitly ensure/open/stop the configured app host. Status never acquires a missing host; ensure uses provisioned assets, never downloads or logs in. Open makes no model request. Stop cannot kill shared/unrelated owners or an active lane.", "action", "action"),
        ("codex_chats", "Find or read accessible saved Chats without inference or acquiring work control. Select stable conversation_id/branch, never a title; continue with the account-bound list offset or source-bound read cursor. Hidden and unsupported content and private capability redactions are explicit.", "action conversation_id branch after account offset limit project_id title updated_after updated_before list_cursor", "action"),
        ("codex_sessions", "Select an explicit Native work; create/fork only for a genuinely new task. join continues a completed existing work using task and required stable call_id, reserves Direct reasoning without another model inference or saved-Chat lookup, and returns its controller. Then call codex_attach, read instructions/context and echo receipt as accept before native tools. Use codex_report for complete public progress and codex_finish for the submitted final; these are Bridge-supplied text, not independently observed host messages. Never resend a join under a new call_id after uncertain delivery. A busy/stopped/unreconciled work is rejected, not taken over. Direct means this response uses Native tools; provider means Native starts Provider model execution (MCP default or explicit local_function). start uses an existing controller and selected route; optional model and effort change Provider Native settings and persist according to Native semantics. Provider results return via events/wait completed_result. Independent assignment is separate: role, budget, optional parent_work_id/parent_controller and dependencies. Inspect completed_result, not merely the last history item, and follow history pagination. events/wait observe Native results/questions without new inference. synchronize is a Native-only input barrier; it never retrieves saved Chat. source_sync explicitly verifies/imports saved original input for thread_id; source_join explicitly attaches to that verified active source using work_id, conversation_id, stable call_id and optional original source_message_id, then receipt as accept. Those recovery controls can retrieve saved Chat; respect waiting/retry_after_seconds, exact branch/account and context acceptance. They are not required for ordinary join. A source_join response uses its native_catalog/native_tool lease and observed original final, not codex_finish. bind/unbind changes the retained source verification connection at a completed boundary with controller and exact cursor/context_cursor; bind also requires conversation_id and complete source_cursor. No background history reader is enabled. claim recovers a settled Native controller using exact cursor/context_cursor; transfer/transfer_accept changes explicit ownership/route at a completed boundary. A settled Direct claim can extend native/elapsed budget while retaining concurrency/app caps and consumed counters; Provider extension requires host-ledger reconciliation. chat_input explicitly submits a related saved-Chat task with complete source_cursor, task, call_id and finite budget; optional model and effort override the prepared Chat selection. chat_result observes the original result. queue enqueues text with stable call_id on an explicit thread_id: registration ACK is separate from consumption. Supply execution_home/workspace to select scope. stop_tree fences descendants. All mutations preserve exact retry IDs and reconcile unknown delivery without redispatch.", "action work_id controller thread_id task call_id cursor context_cursor mode reasoning return_path workspace execution_home role budget parent_work_id parent_controller dependencies decision text max_wait_seconds conversation_id source_message_id source_cursor input_id transfer_receipt accept model effort", "action"),
        ("codex_attach", "Read current native context; echo its receipt as accept to acquire the offered controller.", "work_id controller accept", "work_id controller"),
        ("codex_history", "Read public native history since an optional source-bound after cursor.", "work_id after after_context", "work_id"),
        ("codex_tools", "Discover the complete current native function/custom/search schemas.", "work_id controller", "work_id controller"),
        ("codex_execute", "Execute one advertised native tool under upstream approvals and return its original result. Reuse exact call_id/name/input/report on delivery retries.", "work_id controller call_id name input report", "work_id controller call_id name input report"),
        ("codex_answer", "Forward the user's supplied answer to a pending native question. This cannot answer approval/reviewer requests.", "work_id controller request_id answers", "work_id controller request_id answers"),
        ("codex_report", DIRECT_PROGRESS, "work_id controller call_id text", "work_id controller call_id text"),
        ("codex_finish", DIRECT_COMPLETION, "work_id controller call_id text", "work_id controller call_id text"),
        ("codex_status", "Observe native execution, unresolved calls, human requests and unknown host progress.", "work_id", "work_id"),
        ("codex_stop", "Fence new effects and request interruption; report actual confirmation separately.", "work_id", "work_id"),
        ("codex_handoff", "Prepare a grounded summary at a history cursor, fence the old Chat and optionally request a successor on the same Codex thread.", "work_id controller call_id summary cursor context_cursor renew", "work_id controller call_id summary cursor context_cursor"),
        ("codex_native_summary", "Answer an upstream-requested inference summary without replacing Chat. Supply a unique stable call_id per summary request and reuse identical arguments on delivery retries; an old keyed receipt cannot answer a later native round.", "work_id controller summary call_id", "work_id controller summary"),
        ("native_catalog", "Read the current admitted native turn's tool schemas. For a later user request, use codex_sessions join on the existing work and accept codex_attach context; a retired catalog does not admit ordinary follow-up input. Explicit source recovery can return a fresh turn_token after its verified admission; old execution tokens remain retired.", "turn_token", "turn_token"),
        ("native_tool", "Run one advertised native tool and return its original result. Delivery retries reuse exact turn_token/call_id/name/input/report.", "turn_token call_id name input report", "turn_token call_id name input report"),
    ];
    controls.into_iter().map(|(name, description, fields, required)| {
        let mut properties = fields.split_whitespace().map(|key| (key.to_owned(), field(key))).collect::<serde_json::Map<_,_>>();
        if name == "codex_chats" {
            properties.insert("action".into(), json!({"type":"string","enum":["list","read"]}));
            properties.insert("after".into(),json!({"type":"object","properties":{"account":{"type":"string"},"conversation":{"type":"string"},"branch":{"type":"string"},"count":{"type":"integer","minimum":0},"prefix":{"type":"string"}},"required":["account","conversation","branch","count","prefix"],"additionalProperties":false}));
        }
        if name=="codex_host" { properties.insert("action".into(),json!({"type":"string","enum":["ensure","status","open","stop"]})); }
        if name=="codex_projects" { properties.insert("action".into(),json!({"type":"string","enum":["list","use"]})); }
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,
            "required":required.split_whitespace().collect::<Vec<_>>(),"additionalProperties":false},
            "annotations":crate::mcp::tool_annotations(name)})
    }).collect()
}

async fn http(State(operator): State<Arc<Operator>>, request: HttpRequest) -> Response {
    if request.headers().contains_key("origin")
        || !crate::server::authorized(request.headers(), &operator.token)
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some("application/json")
    {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let bytes = match tokio::time::timeout(
        Duration::from_secs(30),
        axum::body::to_bytes(request.into_body(), LIMIT),
    )
    .await
    {
        Ok(Ok(v)) => v,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let row: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if row["jsonrpc"] != "2.0" || !row["method"].is_string() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(id) = row.get("id") else {
        return StatusCode::ACCEPTED.into_response();
    };
    if !id.is_number() && !id.is_string() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let result = match row["method"].as_str().unwrap() {
        "initialize" => match row["params"]["protocolVersion"].as_str() {
            Some(v @ ("2025-03-26" | "2025-06-18" | "2025-11-25")) => Ok(
                json!({"protocolVersion":v,"capabilities":{"tools":{}},"serverInfo":{"name":"termux-codex-operator","version":env!("CARGO_PKG_VERSION")}}),
            ),
            _ => Err(Error("mcp_protocol_unqualified")),
        },
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools":tools()})),
        "tools/call" => {
            let name = row["params"]["name"].as_str().unwrap_or("").to_owned();
            let args = row["params"]["arguments"].clone();
            // Dropping an HTTP observer never cancels a dispatched transition.
            let operation = tokio::spawn(async move { operator.call(&name, args).await });
            let value = match tokio::time::timeout(Duration::from_secs(600), operation).await {
                Ok(Ok(value)) => value,
                _ => Err(Error("mcp_observer_timeout")),
            };
            Ok(match value {
                Ok(v) => json!({"content":[{"type":"text","text":v.to_string()}],"isError":false}),
                Err(e) => {
                    json!({"content":[{"type":"text","text":json!({"error":e.0}).to_string()}],"isError":true})
                }
            })
        }
        _ => Err(Error("method_not_found")),
    };
    let body = match result {
        Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
        Err(e) => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":e.0}}),
    };
    ([("cache-control", "no-store")], Json(body)).into_response()
}

#[cfg(test)]
mod local_tests {
    use super::*;
    #[test]
    fn settled_app_lane_does_not_request_an_unneeded_host_stop() {
        assert!(!app_lane_requires_host_stop(
            &json!({"phase":"idle","pending_native_calls":0,"app_dispatches":0,
                "user_role_objects":0,"tool_return_objects":0,"public_event_count":0,
                "conversation_id":null})
        ));
        assert!(!app_lane_requires_host_stop(
            &json!({"phase":"stopped","pending_native_calls":0})
        ));
        assert!(app_lane_requires_host_stop(
            &json!({"phase":"idle","pending_native_calls":1})
        ));
        assert!(app_lane_requires_host_stop(
            &json!({"phase":"idle","pending_native_calls":0,"app_dispatches":0,
                "user_role_objects":1,"tool_return_objects":0,"public_event_count":0,
                "conversation_id":null})
        ));
        assert!(app_lane_requires_host_stop(
            &json!({"phase":"running","pending_native_calls":0})
        ));
        assert!(app_lane_requires_host_stop(
            &json!({"phase":"unknown","pending_native_calls":0})
        ));
        assert!(app_lane_requires_host_stop(
            &json!({"pending_native_calls":0})
        ));
    }

    #[test]
    fn completed_result_selects_current_final_even_before_later_tool_reports() {
        let history = vec![
            json!({"turn_id":"old","item":{"type":"agentMessage","phase":"final_answer","text":"old"}}),
            json!({"turn_id":"current","item":{"id":"original-final","type":"agentMessage","phase":"final_answer","text":"original final"}}),
            json!({"turn_id":"current","item":{"type":"commandExecution","exitCode":0}}),
            json!({"turn_id":"current","item":{"type":"agentMessage","phase":"commentary","text":"later report"}}),
        ];
        let result = completed_result(true, Some("current"), &history).unwrap();
        assert_eq!(result["turn_id"], "current");
        assert_eq!(result["source"], "native_thread_items");
        assert_eq!(result["items"], json!([history[1]["item"]]));
        assert_eq!(history[3]["item"]["text"], "later report");
    }

    #[test]
    fn completion_never_substitutes_an_old_or_interrupted_turn_final() {
        let history = vec![
            json!({"turn_id":"old","item":{"type":"agentMessage","phase":"final_answer","text":"old final"}}),
        ];
        assert!(completed_result(true, Some("new"), &history).is_none());
        assert!(completed_result(false, Some("old"), &history).is_none());
        assert!(completed_result(true, None, &history).is_none());
        assert!(completed_result(true, Some("old"), &[]).is_none());
    }

    #[test]
    fn wait_requires_the_observed_current_turn_final_before_delivery() {
        let mut result = json!({"status":{"completed":true,"phase":"completed","human_requests":[]},
            "history_observation_available":true,"completed_result_available":false});
        assert_eq!(wait_reason(&result), None);
        result["completed_result_available"] = json!(true);
        assert_eq!(wait_reason(&result), Some("completed"));
    }

    #[test]
    fn wait_does_not_deliver_completion_without_observable_history() {
        let mut result = json!({"status":{"completed":true,"phase":"completed","human_requests":[]},
            "history_observation_available":false,"events":[]});
        assert_eq!(wait_reason(&result), None);
        result["history_observation_available"] = json!(true);
        assert_eq!(wait_reason(&result), Some("completed"));
        result["status"]["completed"] = json!(false);
        result["status"]["phase"] = json!("running");
        assert_eq!(wait_reason(&result), None);
    }

    #[test]
    fn unavailable_history_keeps_real_questions_and_stops_observable() {
        let mut result = json!({"status":{"completed":false,"phase":"running","human_requests":[{"id":1}]},
            "history_observation_available":false});
        assert_eq!(wait_reason(&result), Some("human_request"));
        result["status"]["human_requests"] = json!([]);
        for phase in ["stopped", "reconciliation_required", "handoff"] {
            result["status"]["phase"] = json!(phase);
            assert_eq!(wait_reason(&result), Some("attention_required"));
        }
    }

    fn fixture_record(root: &std::path::Path) -> Record {
        Record {
            schema: WORK_SCHEMA.into(),
            id: "work-one".into(),
            thread: "thread-one".into(),
            execution: Owner {
                directory: root.join("server"),
                home: root.join("home"),
                process: crate::cdp::Process::read(std::process::id()).unwrap(),
            },
            origin: "chatgpt".into(),
            native_turn: None,
            generation: 1,
            capability: "a".repeat(48),
            token: "b".repeat(48),
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
            selection: None,
            workspace: None,
            assignment: None,
        }
    }

    #[test]
    fn reasoning_contract_has_only_direct_and_provider() {
        let catalog = tools();
        let sessions = catalog
            .iter()
            .find(|tool| tool["name"] == "codex_sessions")
            .unwrap();
        assert_eq!(
            sessions["inputSchema"]["properties"]["reasoning"]["enum"],
            json!(["direct", "provider"])
        );
        for name in ["direct", "provider"] {
            let value = json!({"reasoning":name,"return_path":"mcp"});
            validate_arguments(
                &catalog,
                "codex_sessions",
                &json!({"action":"list","reasoning":name}),
            )
            .unwrap();
            let selection: Selection = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(selection).unwrap(), value);
        }
        for name in ["current", "managed", "Direct", "Provider"] {
            assert!(validate_arguments(
                &catalog,
                "codex_sessions",
                &json!({"action":"list","reasoning":name})
            )
            .is_err());
            assert!(serde_json::from_value::<Selection>(
                json!({"reasoning":name,"return_path":"mcp"})
            )
            .is_err());
        }
    }

    #[test]
    fn offline_reasoning_conversion_preserves_originals_restarts_and_downgrades() {
        let root = std::env::temp_dir().join(format!("reasoning-migrate-{}", random_id().unwrap()));
        crate::journal::private_directory(&root).unwrap();
        let state = root.join("state");
        let journal = Journal::open(&state).unwrap();
        let mut value = serde_json::to_value(fixture_record(&root)).unwrap();
        value["schema"] = json!("termux-codex-operator-work-v1");
        value["selection"] = json!({"reasoning":"current","return_path":"mcp"});
        value["summary"] = json!("Original prose mentioning managed and current must not change.");
        let key = digest(&("operator-work-v1", &value["id"]));
        journal.write(&key, &value).unwrap();
        let original = value.clone();
        let mut other = value.clone();
        other["id"] = json!("work-two");
        other["selection"]["reasoning"] = json!("managed");
        let other_key = digest(&("operator-work-v1", &other["id"]));
        journal.write(&other_key, &other).unwrap();
        let untouched =
            json!({"result":{"selection":{"reasoning":"managed"}},"source":"original receipt"});
        let receipt_key = digest(&"immutable-original-receipt");
        journal.write(&receipt_key, &untouched).unwrap();
        assert_eq!(
            migrate_reasoning(&state, &root.join("busy.json"), "direct-provider"),
            Err(Error("provider_already_running"))
        );
        drop(journal);
        let backup = root.join("before.json");
        assert_eq!(
            migrate_reasoning(&state, &backup, "direct-provider").unwrap()["converted_records"],
            2
        );
        // Resume a genuinely mixed transaction after one record write was lost.
        let journal = Journal::open(&state).unwrap();
        journal.write(&key, &original).unwrap();
        drop(journal);
        migrate_reasoning(&state, &backup, "direct-provider").unwrap();
        let journal = Journal::open(&state).unwrap();
        let upgraded: Record = journal.read(&key).unwrap().unwrap();
        assert_eq!(upgraded.schema, WORK_SCHEMA);
        assert_eq!(upgraded.selection.unwrap().reasoning, Reasoning::Direct);
        assert_eq!(
            journal.read::<Value>(&receipt_key).unwrap(),
            Some(untouched)
        );
        let snapshot: ReasoningMigration =
            serde_json::from_slice(&std::fs::read(&backup).unwrap()).unwrap();
        assert_eq!(snapshot.records[&key], original);
        let canonical: Value = journal.read(&key).unwrap().unwrap();
        let mut changed = canonical.clone();
        changed["summary"] = json!("Later user state must not be overwritten by a stale backup.");
        journal.write(&key, &changed).unwrap();
        drop(journal);
        assert_eq!(
            migrate_reasoning(&state, &backup, "direct-provider"),
            Err(Error("reasoning_migration_state_changed"))
        );
        let journal = Journal::open(&state).unwrap();
        assert_eq!(journal.read::<Value>(&key).unwrap(), Some(changed));
        journal.write(&key, &canonical).unwrap();
        drop(journal);
        migrate_reasoning(&state, &root.join("downgrade.json"), "provider14").unwrap();
        let journal = Journal::open(&state).unwrap();
        assert_eq!(journal.read::<Value>(&key).unwrap(), Some(original));
        assert_eq!(journal.read::<Value>(&other_key).unwrap(), Some(other));
        drop(journal);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn old_status_snapshot_cannot_fence_a_rebound_native_round() {
        let snapshot = fixture_record(std::path::Path::new("/owned"));
        let mut current = snapshot.clone();
        current.execution.process.start = "replacement".into();
        current.generation += 1;
        current.phase = "running".into();
        assert!(
            !observed_owner_requires_reconciliation(&snapshot, &current, &current.execution)
                .unwrap()
        );
        assert_eq!(current.phase, "running");
        assert!(
            observed_owner_requires_reconciliation(&snapshot, &snapshot, &current.execution)
                .unwrap()
        );
        let mut superseded = current.execution.clone();
        superseded.process.start = "already-replaced-again".into();
        assert_eq!(
            observed_owner_requires_reconciliation(&snapshot, &current, &superseded),
            Err(Error("native_observation_superseded"))
        );
    }

    #[test]
    fn native_owner_rebind_requires_a_new_canonical_turn_without_pending_calls() {
        let mut record = fixture_record(std::path::Path::new("/owned"));
        record.origin = "codex".into();
        record.native_turn = Some("original".into());
        assert!(new_native_owner_allowed(&record, Some("new-user-turn")));
        assert!(!new_native_owner_allowed(&record, Some("original")));
        assert!(!new_native_owner_allowed(&record, None));
        record.pending.push("unreturned-original-call".into());
        assert!(!new_native_owner_allowed(&record, Some("new-user-turn")));
        record.pending.clear();
        record.origin = "chatgpt".into();
        assert!(new_native_owner_allowed(&record, Some("new-user-turn")));
    }

    #[test]
    fn driver_failures_terminate_even_after_status_fences_the_phase() {
        let mut record = fixture_record(std::path::Path::new("/owned"));
        record.phase = "running".into();
        assert_eq!(driver_round_error(&record), None);
        record.capability = "handoff-controller".into();
        assert_eq!(driver_round_error(&record), None);
        record.driver_phase = "start_unknown".into();
        assert_eq!(
            driver_round_error(&record),
            Some(Error("operator_app_start_unconfirmed"))
        );
        record.phase = "reconciliation_required".into();
        assert_eq!(
            driver_round_error(&record),
            Some(Error("operator_app_start_unconfirmed"))
        );
        record.driver_phase = "started_observation_unknown".into();
        assert_eq!(
            driver_round_error(&record),
            Some(Error("operator_reconciliation_required"))
        );
        for phase in ["stop_requested", "stopped"] {
            record.phase = phase.into();
            assert_eq!(
                driver_round_error(&record),
                Some(Error("operator_controller_fenced"))
            );
        }
    }

    #[test]
    fn native_round_identity_covers_same_owner_followups_and_controller_rotation() {
        let snapshot = fixture_record(std::path::Path::new("/owned"));
        assert!(same_native_round(&snapshot, &snapshot));
        for field in ["turn", "generation", "controller", "owner"] {
            let mut current = snapshot.clone();
            match field {
                "turn" => current.native_turn = Some("new-user-turn".into()),
                "generation" => current.generation += 1,
                "controller" => current.capability = "new-controller".into(),
                "owner" => current.execution.process.start = "replacement".into(),
                _ => unreachable!(),
            }
            assert!(!same_native_round(&snapshot, &current), "{field}");
        }
    }

    #[test]
    fn app_admission_respects_stop_busy_and_obsolete_rounds() {
        let mut record = fixture_record(std::path::Path::new("/owned"));
        record.phase = "running".into();
        record.native_turn = Some("previous".into());
        let mut work = Work {
            record: record.clone(),
            active: None,
            request: None,
            round: String::new(),
            checkpoint: None,
            driver_lock: Arc::new(Mutex::new(())),
            native_admission: Arc::new(Mutex::new(())),
            stops_in_progress: 0,
            app_cancel: None,
            completion_observer: None,
        };
        assert!(admit_app_round(&work, &record, Some("next")).is_ok());
        work.stops_in_progress = 1;
        assert_eq!(
            admit_app_round(&work, &record, Some("next")),
            Err(Error("operator_controller_fenced"))
        );
        work.stops_in_progress = 0;
        work.record.phase = "stop_requested".into();
        assert_eq!(
            admit_app_round(&work, &record, Some("next")),
            Err(Error("operator_controller_fenced"))
        );
        work.record.phase = "stopped".into();
        assert_eq!(
            admit_app_round(&work, &record, Some("previous")),
            Err(Error("operator_controller_fenced"))
        );
        assert!(admit_app_round(&work, &record, Some("next")).is_ok());
        work.app_cancel = Some(CancellationToken::new());
        assert_eq!(
            admit_app_round(&work, &record, Some("next")),
            Err(Error("native_lane_busy"))
        );
        work.app_cancel = None;
        work.record.native_turn = Some("already-newer".into());
        assert_eq!(
            admit_app_round(&work, &record, Some("next")),
            Err(Error("operator_controller_fenced"))
        );
        let mut reserved = record.clone();
        reserved.phase = "starting_native".into();
        work.record = reserved.clone();
        work.record.native_turn = Some("reserved-turn".into());
        assert!(admit_app_round(&work, &reserved, Some("reserved-turn")).is_ok());
        assert!(admit_app_round(&work, &reserved, Some("previous")).is_err());
        work.record.capability = "other-controller".into();
        assert!(admit_app_round(&work, &reserved, Some("reserved-turn")).is_err());
    }

    #[test]
    fn owner_observation_cannot_promote_an_obsolete_controller() {
        let mut record = fixture_record(std::path::Path::new("/owned"));
        record.phase = "running".into();
        record.accepted = true;
        let args = json!({"controller":record.capability});
        assert!(controller_matches(&record, &args));
        record.capability = "new-controller".into();
        assert!(!controller_matches(&record, &args));
        let current = json!({"controller":record.capability});
        assert!(controller_matches(&record, &current));
        record.phase = "stop_requested".into();
        assert!(!controller_matches(&record, &current));
        record.phase = "running".into();
        record.selection = Some(Selection {
            reasoning: Reasoning::Provider,
            return_path: ReturnPath::Mcp,
        });
        assert!(!controller_matches(&record, &current));
    }

    #[test]
    fn interrupted_registration_and_legacy_index_recover_complete_records_once() {
        struct Root(std::path::PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Root(std::env::temp_dir().join(format!("wi-{}", random_id().unwrap())));
        let journal = Journal::open(&root.0).unwrap();
        let record = fixture_record(&root.0);
        let key = digest(&("operator-work-v1", &record.id));
        // Legacy orphan records survive migration even if their index never
        // reached disk. Settled legacy serialization gains no new fields.
        journal.write(&key, &record).unwrap();
        let serialized = serde_json::to_value(&record).unwrap();
        for key in ["selection", "workspace", "assignment"] {
            assert!(serialized.get(key).is_none());
        }
        let migrated = recover_work_index(&journal).unwrap();
        assert_eq!(migrated.ids, vec![record.id.clone()]);
        let mut next = record.clone();
        next.id = "work-two".into();
        next.thread = "thread-two".into();
        journal
            .write(
                &work_index_key(),
                &WorkIndex {
                    ids: migrated.ids,
                    pending: Some(next.clone()),
                },
            )
            .unwrap();
        let recovered = recover_work_index(&journal).unwrap();
        assert_eq!(recovered.ids, vec![record.id, next.id.clone()]);
        assert!(recovered.pending.is_none());
        let saved: Record = journal
            .read(&digest(&("operator-work-v1", &next.id)))
            .unwrap()
            .unwrap();
        assert_eq!(saved.thread, next.thread);
        assert_eq!(recover_work_index(&journal).unwrap().ids, recovered.ids);
        assert_eq!(
            journal
                .read::<Vec<String>>(&digest(&"operator-index-v1"))
                .unwrap()
                .unwrap(),
            recovered.ids
        );
    }
    #[test]
    fn qualified_idempotency_policy_preserves_every_other_annotation() {
        let selected = [
            "native_catalog",
            "native_tool",
            "codex_chats",
            "codex_history",
            "codex_tools",
            "codex_status",
            "codex_execute",
            "codex_report",
            "codex_finish",
            "codex_handoff",
            "codex_answer",
            "codex_attach",
        ];
        let controls = tools();
        assert_eq!(controls.len(), 17);
        assert_eq!(
            controls
                .iter()
                .filter(|v| v["annotations"]["idempotentHint"] == true)
                .count(),
            selected.len()
        );
        for tool in controls {
            let name = tool["name"].as_str().unwrap();
            assert_eq!(
                tool["annotations"],
                json!({"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false,"idempotentHint":selected.contains(&name)})
            );
        }
        assert_eq!(
            crate::mcp::tool_annotations("unknown_tool")["idempotentHint"],
            false
        );
    }
    #[test]
    fn each_local_driver_has_a_distinct_declaration_with_the_full_same_controls() {
        let first = local_signature(&format!("operator_local_{}", "a".repeat(48)));
        let second = local_signature(&format!("operator_local_{}", "b".repeat(48)));
        assert_ne!(first["name"], second["name"]);
        assert_eq!(first["name"].as_str().unwrap().len(), 63);
        assert_eq!(first["type"], "kwargs");
        assert_eq!(first["params"], second["params"]);
        assert!(first.get("parameters").is_none());
        let params = first["params"].as_array().unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0]["name"], "name");
        assert_eq!(params[1]["name"], "input");
        assert!(params
            .iter()
            .all(|v| v["required"] == true && v["type"]["type"] == "string"));
        for control in tools() {
            assert!(params[0]["type"]["description"]
                .as_str()
                .unwrap()
                .contains(control["name"].as_str().unwrap()));
        }
    }
    #[test]
    fn invalid_local_json_is_a_correlated_no_effect_result_and_can_be_corrected() {
        let bad = json!({"content":{"text":json!({"name":"codex_execute","input":"{\"input\":\"patch with \"unescaped\" quote\"}"}).to_string()}});
        let rejection = local_arguments(&bad).unwrap_err();
        assert_eq!(rejection["error"]["executed"], false);
        assert!(rejection["error"]["column"].as_u64().unwrap() > 0);
        let patch = "*** Begin Patch\n*** Add File: owned.py\n+value = \"quoted\"\n*** End Patch";
        let good = json!({"content":{"text":json!({"name":"codex_execute","input":json!({"input":patch,"call_id":"owned"}).to_string()}).to_string()}});
        let (name, args) = local_arguments(&good).unwrap();
        assert_eq!(name, "codex_execute");
        assert_eq!(args["input"], patch);
    }
    #[test]
    fn independent_roots_do_not_share_a_parallel_slot_and_confirmed_stops_survive_recovery() {
        assert!(!same_assignment_parent(None, None));
        assert!(same_assignment_parent(Some("parent"), Some("parent")));
        assert!(!same_assignment_parent(Some("parent"), Some("other")));
        assert!(!same_assignment_parent(Some("parent"), None));
        assert_eq!(recovered_phase("completed"), "completed");
        assert_eq!(recovered_phase("interrupted"), "interrupted");
        assert_eq!(recovered_phase("stopped"), "stopped");
        for phase in [
            "running",
            "admitting",
            "starting_native",
            "stop_requested",
            "waiting_native",
            "queued",
        ] {
            assert_eq!(recovered_phase(phase), "reconciliation_required");
        }
    }

    #[test]
    fn budget_extension_preserves_host_caps_and_rejects_allowance_resets() {
        let original = Budget {
            max_parallel: 2,
            max_app_starts: 12,
            max_native_calls: 100,
            max_elapsed_seconds: 1200,
        };
        let extended = original
            .extend_budget(Budget {
                max_elapsed_seconds: 3600,
                max_native_calls: 1000,
                ..original.clone()
            })
            .unwrap();
        assert_eq!(extended.max_elapsed_seconds, 3600);
        assert_eq!(extended.max_native_calls, 1000);
        for invalid in [
            Budget {
                max_native_calls: 99,
                ..original.clone()
            },
            Budget {
                max_app_starts: 13,
                ..original.clone()
            },
            Budget {
                max_parallel: 3,
                ..original.clone()
            },
            Budget {
                max_elapsed_seconds: 1199,
                ..original.clone()
            },
        ] {
            assert!(matches!(
                original.extend_budget(invalid),
                Err(Error("assignment_budget_extension_invalid"))
            ));
        }
    }
    #[test]
    fn historical_returns_are_ignored_but_uncertain_or_changed_calls_are_fenced() {
        assert!(!local_return_seen(None, "same").unwrap());
        assert!(local_return_seen(
            Some(&json!({"phase":"returned","fingerprint":"same"})),
            "same"
        )
        .unwrap());
        for phase in ["invoking", "return_prepared"] {
            assert_eq!(
                local_return_seen(Some(&json!({"phase":phase,"fingerprint":"same"})), "same"),
                Err(Error("operator_local_reconciliation_required"))
            );
        }
        assert_eq!(
            local_return_seen(
                Some(&json!({"phase":"returned","fingerprint":"other"})),
                "same"
            ),
            Err(Error("operator_call_changed"))
        );
    }
}
