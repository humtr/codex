//! MCP operates one native work binding. Responses rounds carry native tool
//! dispatch only; Chat lifetime is independent. Upstream is the transcript owner.
use crate::{
    app::{AppHost, Bridge},
    config::Config,
    context::{self, digest},
    journal::{random_id, Journal},
    mcp::{Arguments, Broker},
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
const LIMIT: usize = 16 * 1024 * 1024;
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
        text: String,
        ack: oneshot::Sender<Result<(), Error>>,
    },
}
type Works = Arc<Mutex<BTreeMap<String, Work>>>;
#[derive(Clone)]
pub struct Operator {
    pool: Arc<Pool>,
    broker: Arc<Broker>,
    journal: Arc<Journal>,
    works: Works,
    config: Config,
    bridge: Arc<dyn Bridge>,
    reviewer: Arc<AppHost>,
    token: String,
    session_mutations: Arc<Mutex<()>>,
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
        for id in journal
            .read::<Vec<String>>(&digest(&"operator-index-v1"))?
            .unwrap_or_default()
        {
            let mut record: Record = journal
                .read(&digest(&("operator-work-v1", &id)))?
                .ok_or(Error("operator_record_missing"))?;
            if record.id != id
                || record.execution.home != pool.home
                || record.schema != "termux-codex-operator-work-v1"
                || works.len() >= 256
            {
                return Err(Error("operator_record_invalid"));
            }
            // A new service never inherits an old controller or retransmits a call.
            record.capability = random_id()?;
            record.accepted = false;
            if record.phase != "completed" {
                record.phase = "reconciliation_required".into();
            }
            works.insert(
                record.thread.clone(),
                Work {
                    record,
                    active: None,
                    request: None,
                    round: String::new(),
                    checkpoint: None,
                    driver_lock: Arc::new(Mutex::new(())),
                },
            );
        }
        Ok(Arc::new(Self {
            pool,
            broker: broker.clone(),
            reviewer: Arc::new(AppHost::new(
                Arc::new(crate::mcp_bridge::McpBridge::new(
                    bridge.clone(),
                    broker.clone(),
                    journal.clone(),
                    config.transport.connector_name.clone(),
                )?),
                journal.clone(),
            )),
            journal,
            works: Arc::new(Mutex::new(works)),
            config,
            bridge,
            token,
            session_mutations: Arc::new(Mutex::new(())),
        }))
    }
    fn persist(&self, record: &Record) -> Result<(), Error> {
        self.journal
            .write(&digest(&("operator-work-v1", &record.id)), record)
    }
    async fn register(
        &self,
        thread: &str,
        origin: &str,
        request: Option<&Request>,
    ) -> Result<Record, Error> {
        let execution = self.pool.for_thread(thread).await?.owner.clone();
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
            .register(&id, request.map_or_else(BTreeMap::new, |v| v.tools.clone()))
            .await?;
        let record = Record {
            schema: "termux-codex-operator-work-v1".into(),
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
        };
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
            },
        );
        self.journal.write(
            &digest(&"operator-index-v1"),
            &works.values().map(|v| &v.record.id).collect::<Vec<_>>(),
        )?;
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
        if require_controller
            && (args["controller"].as_str() != Some(&record.capability)
                || !record.accepted
                || !matches!(
                    record.phase.as_str(),
                    "running" | "waiting_native" | "native_summary"
                ))
        {
            return Err(Error("operator_controller_fenced"));
        }
        drop(works);
        if require_controller {
            self.verify_identity(&record).await?;
        }
        let native = match self.pool.for_thread(&record.thread).await {
            Ok(native) => native,
            Err(Error("native_thread_owner_unavailable"))
                if matches!(
                    record.phase.as_str(),
                    "reconciliation_required" | "completed" | "stop_requested"
                ) =>
            {
                let native = self.pool.prepare().await?;
                if args["action"] == "claim" {
                    native
                        .call(
                            "thread/resume",
                            json!({"threadId":record.thread,"config":self.pool.routing()?}),
                        )
                        .await?;
                }
                native
            }
            Err(error) => return Err(error),
        };
        if native.owner != record.execution {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            work.record.accepted = false;
            if work.record.phase != "completed" {
                work.record.phase = "reconciliation_required".into();
            }
            self.persist(&work.record)?;
            record = work.record.clone();
            if require_controller {
                return Err(Error("native_owner_reconciliation_required"));
            }
        }
        Ok((record, native))
    }
    async fn verify_identity(&self, record: &Record) -> Result<(), Error> {
        let Some(lane) = &record.driver else {
            return Ok(());
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
        Ok(())
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
                    .filter(|job| owned.iter().any(|call| call["id"] == job["itemId"]))
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
            return Ok(false);
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
            return Ok(json!({"native_item":id,"recorded":true,"reused":true}));
        }
        if !record.accepted || record.phase != "running" {
            return Err(Error("operator_controller_fenced"));
        }
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
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability {
                return Ok(json!({"native_item":id,"recorded":true,"completed":true}));
            }
            self.broker.retire(&record.token, false).await?;
            work.record.phase = "completed".into();
            self.persist(&work.record)?;
        }
        Ok(json!({"native_item":id,"recorded":true,"completed":final_answer}))
    }
    pub async fn call(&self, name: &str, args: Value) -> Result<Value, Error> {
        let schema = tools()
            .into_iter()
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
                _ => false,
            };
            if !valid {
                return Err(Error("operator_arguments_invalid"));
            }
        }
        match name {
            "native_catalog" => Ok(json!(
                self.broker.catalog(text(&args, "turn_token", 48)?).await?
            )),
            "native_tool" => {
                let token = text(&args, "turn_token", 48)?.to_owned();
                let mut args = args;
                args.as_object_mut()
                    .ok_or(Error("mcp_call_invalid"))?
                    .remove("turn_token");
                let args = serde_json::from_value(args).map_err(|_| Error("mcp_call_invalid"))?;
                self.broker.invoke(&token, args).await
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
                Ok(
                    json!({"items":history[after.count..],"cursor":Self::cursor(&history),"context_items":context[after_context.count..],"context_cursor":Self::cursor(&context)}),
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
                self.verify_identity(&record).await?;
                let call: Arguments = serde_json::from_value(json!({"call_id":args["call_id"],"name":args["name"],"input":args["input"],"report":args["report"]}))
                    .map_err(|_| Error("mcp_call_invalid"))?;
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
                work.record.policy = policy.map(digest);
                work.record.observed = Some(Self::cursor(&history));
                work.record.observed_context = Some(Self::cursor(&context));
                self.persist(&work.record)?;
                Ok(result)
            }
            "codex_answer" => {
                let (record, native) = self.binding(&args, true).await?;
                native
                    .answer_question(
                        &record.thread,
                        text(&args, "request_id", 256)?,
                        args["answers"].clone(),
                    )
                    .await?;
                Ok(json!({"answer_forwarded":true,"approval_granted":false}))
            }
            "codex_report" => self.report(&args, false).await,
            "codex_finish" => self.report(&args, true).await,
            "codex_status" => self.status(args).await,
            "codex_stop" => self.stop(args).await,
            "codex_handoff" => self.handoff(args).await,
            "codex_native_summary" => {
                let (record, _) = self.binding(&args, true).await?;
                if record.phase != "native_summary" {
                    return Err(Error("native_summary_not_requested"));
                }
                let summary = text(&args, "summary", 256 * 1024)?.to_owned();
                let sender = self
                    .works
                    .lock()
                    .await
                    .get(&record.thread)
                    .and_then(|v| v.active.clone())
                    .ok_or(Error("native_round_not_ready"))?;
                let (ack, response) = oneshot::channel();
                sender
                    .send(Command::Summary {
                        capability: record.capability.clone(),
                        text: summary,
                        ack,
                    })
                    .await
                    .map_err(|_| Error("native_delivery_unknown"))?;
                response
                    .await
                    .map_err(|_| Error("native_delivery_unknown"))??;
                Ok(json!({"native_summary_offered":true,"chat_replaced":false}))
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
            "reconciliation_required" | "stop_requested" | "completed"
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
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            if work.record.capability != record.capability {
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
        let works = self.works.lock().await;
        let work = works.get(&record.thread).unwrap();
        Ok(
            json!({"work_id":record.id,"thread_id":record.thread,"origin":record.origin,"generation":record.generation,
            "summary":summary,"incremental_history":history[count..],"cursor":Self::cursor(&history),"receipt":receipt,
            "incremental_context":context[context_count..],"context_cursor":Self::cursor(&context),
            "accepted":work.record.accepted,"instructions":work.request.as_ref().map(|request|&request.instructions),
            "native_context":work.record.phase,"return_bound":record.origin=="codex"}),
        )
    }
    async fn sessions(&self, args: Value) -> Result<Value, Error> {
        match args["action"].as_str() {
            Some("list") => {
                let works = self.works.lock().await;
                Ok(
                    json!({"work":works.values().map(|v|json!({"work_id":v.record.id,"thread_id":v.record.thread,"origin":v.record.origin,"phase":v.record.phase})).collect::<Vec<_>>()}),
                )
            }
            Some("create" | "resume" | "fork") => {
                let _mutation = self.session_mutations.lock().await;
                let action = args["action"].as_str().unwrap();
                let call = text(&args, "call_id", 160)?;
                let key = digest(&("operator-session-call-v1", call));
                let fingerprint = digest(&args);
                if let Some(previous) = self.journal.read::<Value>(&key)? {
                    if previous["fingerprint"] != fingerprint {
                        return Err(Error("operator_call_changed"));
                    }
                    return previous
                        .get("result")
                        .cloned()
                        .ok_or(Error("native_session_creation_unknown"));
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
                self.journal
                    .write(&key, &json!({"fingerprint":fingerprint}))?;
                let native = self.pool.prepare().await?;
                let mut params = json!({"cwd":self.pool.workspace,"config":self.pool.routing()?,"model":self.config.model.default,"modelProvider":"chatgpt_desktop"});
                if action != "create" {
                    params["threadId"] = json!(text(&args, "thread_id", 256)?);
                }
                let method = match action {
                    "create" => "thread/start",
                    "resume" => "thread/resume",
                    _ => "thread/fork",
                };
                let response = native.call(method, params).await?;
                let thread = response["thread"]["id"]
                    .as_str()
                    .ok_or(Error("native_reply_invalid"))?;
                let record = self.register(thread, "chatgpt", None).await?;
                let result = json!({"work_id":record.id,"thread_id":thread,"controller":record.capability,"ready":false});
                self.journal
                    .write(&key, &json!({"fingerprint":fingerprint,"result":result}))?;
                Ok(result)
            }
            Some("start") => {
                let (record, native) = self.binding(&args, false).await?;
                if args["controller"].as_str() != Some(&record.capability)
                    || record.phase != "waiting_native"
                {
                    return Err(Error("operator_controller_fenced"));
                }
                let task = text(&args, "task", 256 * 1024)?;
                if args
                    .get("mode")
                    .is_some_and(|mode| !matches!(mode.as_str(), Some("default" | "plan")))
                {
                    return Err(Error("operator_arguments_invalid"));
                }
                {
                    let mut works = self.works.lock().await;
                    let work = works.get_mut(&record.thread).unwrap();
                    if work.record.phase != "waiting_native" {
                        return Err(Error("native_start_reconciliation_required"));
                    }
                    work.record.phase = "starting_native".into();
                    self.persist(&work.record)?;
                }
                let mut params =
                    json!({"threadId":record.thread,"input":[{"type":"text","text":task}]});
                if let Some(mode) = args.get("mode") {
                    if !matches!(mode.as_str(), Some("default" | "plan")) {
                        return Err(Error("operator_arguments_invalid"));
                    }
                    params["collaborationMode"] = json!({"mode":mode,"settings":{"model":self.config.model.default,"reasoning_effort":self.config.model.reasoning_effort,"developer_instructions":null}});
                }
                native.call("turn/start", params).await?;
                Ok(json!({"started":true,"work_id":record.id}))
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
                    if work.record.phase != "completed" || work.active.is_some() {
                        return Err(Error("native_writer_unsettled"));
                    }
                    work.record.phase = "native_compacting".into();
                    work.record.token = self.broker.register(&record.id, BTreeMap::new()).await?;
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
                if record.driver.is_some() {
                    self.stop_driver(&record).await?;
                }
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.capability != record.capability || work.record.accepted {
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
                    "reconciliation_required" | "stop_requested" | "completed"
                ) {
                    return Err(Error("operator_explicit_handoff_required"));
                }
                let recovered = self.recovery_receipts(&native, &record).await?;
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
                // Restart recovers the same thread at a settled native boundary only.
                let state = native
                    .call(
                        "thread/read",
                        json!({"threadId":record.thread,"includeTurns":false}),
                    )
                    .await?;
                if state["thread"]["status"]["type"] != "idle" {
                    return Err(Error("native_writer_unsettled"));
                }
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.capability != record.capability || work.active.is_some() {
                    return Err(Error("native_writer_unsettled"));
                }
                self.preserve_controller(&work.record)?;
                let _ = self.broker.retire(&record.token, true).await;
                work.record.capability = random_id()?;
                work.record.generation += 1;
                work.record.pending.clear();
                work.record.execution = native.owner.clone();
                work.record.origin = "chatgpt".into();
                work.record.token = self.broker.register(&record.id, BTreeMap::new()).await?;
                work.record.phase = "waiting_native".into();
                work.record.accepted = false;
                self.persist(&work.record)?;
                Ok(
                    json!({"work_id":record.id,"thread_id":record.thread,"controller":work.record.capability,"history":history,"recovered_native_receipts":recovered,"inference_submitted":false}),
                )
            }
            _ => Err(Error("operator_session_action_invalid")),
        }
    }
    async fn status(&self, args: Value) -> Result<Value, Error> {
        let mut observed = false;
        let (mut record, native) = self.binding(&args, false).await?;
        let state = native
            .call(
                "thread/read",
                json!({"threadId":record.thread,"includeTurns":false}),
            )
            .await?;
        if let Some(lane) = &record.driver {
            let snapshot = tokio::time::timeout(
                Duration::from_secs(3),
                self.bridge.operation(json!({"op":"snapshot","lane":lane})),
            )
            .await;
            if let Ok(Ok(snapshot)) = snapshot {
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
                } else if snapshot["terminals"] == json!(["done"]) {
                    "stream_settled"
                } else {
                    "stream_observed"
                }
                .into();
                let mut works = self.works.lock().await;
                let work = works.get_mut(&record.thread).unwrap();
                if work.record.driver == record.driver {
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
        Ok(
            json!({"work_id":record.id,"thread_id":record.thread,"phase":record.phase,"native_status":state["thread"]["status"],
            "pending_calls":record.pending,"call_observations":self.broker.recorded_calls(&record.token)?,"human_requests":requests,
            "host_state":if observed{record.host_last_state.as_str()}else{"unknown"},"driver_phase":record.driver_phase,"last_observed_host_state":record.host_last_state,"host_observed_at":record.host_observed_at,"host_observation_available":observed,"host_progress":"unknown","completed":record.phase=="completed"}),
        )
    }
    async fn stop(&self, args: Value) -> Result<Value, Error> {
        let (mut record, native) = self.binding(&args, false).await?;
        {
            let mut works = self.works.lock().await;
            let work = works.get_mut(&record.thread).unwrap();
            work.record.accepted = false;
            work.record.phase = "stop_requested".into();
            self.persist(&work.record)?;
            record = work.record.clone();
        }
        let native_result = tokio::time::timeout(Duration::from_secs(15), async {
            let state = native
                .call(
                    "thread/read",
                    json!({"threadId":record.thread,"includeTurns":true}),
                )
                .await?;
            for turn in state["thread"]["turns"]
                .as_array()
                .ok_or(Error("native_history_unavailable"))?
            {
                if turn["status"] == "inProgress" {
                    native
                        .call(
                            "turn/interrupt",
                            json!({"threadId":record.thread,"turnId":turn["id"]}),
                        )
                        .await?;
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
                if state["thread"]["status"]["type"] == "idle"
                    && self.owned_jobs(&native, &record).await?.is_empty()
                {
                    return Ok::<_, Error>(());
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;
        let confirmed = matches!(native_result, Ok(Ok(())));
        let host_confirmed = if record.driver.is_some() {
            self.stop_driver(&record).await.unwrap_or(false)
        } else {
            false
        };
        // Detach pending model observers without claiming uncertain calls executed.
        let _ = self.broker.retire(&record.token, true).await;
        let jobs = self.owned_jobs(&native, &record).await;
        Ok(
            json!({"fenced":true,"native_stop_requested":true,"native_stop_confirmed":confirmed,
            "native_stop_state":if confirmed{"confirmed"}else{"unknown"},"host_stop_requested":record.driver.is_some(),"host_stop_confirmed":host_confirmed,
            "remaining_jobs":jobs.as_ref().ok(),"jobs_observation_available":jobs.is_ok()}),
        )
    }
    async fn handoff(&self, args: Value) -> Result<Value, Error> {
        let call = text(&args, "call_id", 160)?;
        let key = digest(&("operator-handoff-call-v1", call, &args["work_id"]));
        let fingerprint = digest(&args);
        if let Some(previous) = self.journal.read::<Value>(&key)? {
            if previous["fingerprint"] != fingerprint {
                return Err(Error("operator_call_changed"));
            }
            return previous
                .get("result")
                .cloned()
                .ok_or(Error("operator_handoff_reconciliation_required"));
        }
        let (record, native) = self.binding(&args, true).await?;
        if !record.pending.is_empty() {
            return Err(Error("operator_pending_handoff"));
        }
        if !self.owned_jobs(&native, &record).await?.is_empty() {
            return Err(Error("operator_background_jobs_unsettled"));
        }
        let summary = text(&args, "summary", 256 * 1024)?;
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
            self.journal
                .write(&key, &json!({"fingerprint":fingerprint}))?;
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
                    if record.driver.is_some() {
                        this.stop_driver(&record).await?;
                    }
                    this.start_driver(next, request).await
                }
                .await;
                if result.is_err() {
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
            self.journal
                .write(&key, &json!({"fingerprint":fingerprint,"result":result}))?;
            Ok(result)
        } else {
            let result = json!({"prepared":true,"thread_id":record.thread,"controller":next.capability,"generation":next.generation,"successor_requested":false});
            self.journal
                .write(&key, &json!({"fingerprint":fingerprint,"result":result}))?;
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
            tokio::time::timeout(Duration::from_secs(15), async {
                loop {
                    let snapshot = self
                        .bridge
                        .operation(json!({"op":"snapshot","lane":marker}))
                        .await?;
                    if snapshot["terminals"] == json!(["done"]) {
                        return Ok::<_, Error>(());
                    }
                    if snapshot["error"].is_string() || snapshot["stopped"] == true {
                        return Err(Error("app_previous_round_unsettled"));
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .map_err(|_| Error("app_previous_round_unsettled"))??;
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
        self.bridge.operation(json!({"op":"open","lane":marker,"signature":{"name":"operator_no_local_tools","description":"Use MCP Codex controls","parameters":{"type":"object","properties":{}}},"project":project})).await?;
        let mut instructions = request
            .instructions
            .iter()
            .map(context::text)
            .collect::<Result<Vec<_>, _>>()?;
        instructions.push(format!("Operate local Codex through the installed {} MCP connector. Use codex_attach with work_id={} and controller={}. Read its current history and echo its receipt as accept before effects. Discover full native tools with codex_tools, run them with codex_execute, publish initial/intermediate reports with codex_report, and explicitly codex_finish after actual results. Context renewal uses codex_history then codex_handoff with the confirmed cursor and context_cursor, grounded summary and renew=true. The same native thread and original return remain bound. If native_context=native_summary, supply codex_native_summary without replacing Chat. Never invent execution receipts, use local functions, create another primary thread or reveal private reasoning. Exact retries reuse logical call IDs; uncertain actions are reconciled, never resent.",self.config.transport.connector_name,record.id,record.capability));
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
        // A genuine later native user turn reuses this Chat. Initial/renewal
        // bootstrap carries no synthetic user context; attach supplies that data.
        let messages = if reuse && request.compaction.is_none() {
            let user = request
                .input
                .iter()
                .rfind(|item| item["role"] == "user")
                .ok_or(Error("native_user_input_missing"))?;
            json!([{"role":"user","text":context::text(user)?,"hidden":false}])
        } else {
            json!([{"role":"developer","text":"Use the Codex MCP attachment described in the control instructions and continue its current task.","hidden":true}])
        };
        self.bridge.operation(json!({"op":"submit","lane":marker,"round":random_id()?,"model":request.host_model,"thinkingEffort":request.thinking_effort,
            "messages":messages,"instructions":instructions,"results":[]})).await?;
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
    async fn exchange_inner(
        &self,
        request: Request,
        events: mpsc::Sender<Event>,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let native = self.pool.for_thread(&request.thread).await?;
        let existed = self.works.lock().await.contains_key(&request.thread);
        let mut record = self
            .register(&request.thread, "codex", Some(&request))
            .await?;
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
            "reconciliation_required" | "stop_requested"
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
            self.preserve_controller(&work.record)?;
            let _ = self.broker.retire(&record.token, recovery).await;
            work.record.capability = random_id()?;
            work.record.generation += 1;
            work.record.token = self
                .broker
                .register(&record.id, request.tools.clone())
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
            "reconciliation_required" | "stop_requested" | "completed"
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
        if !existed || new_user_turn || (manual_compact && record.driver.is_some()) {
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
                if result.is_err() {
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
        let result = async { loop {
            tokio::select! {
                _=cancel.cancelled()=>break Err(Error("native_observer_lost")),
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
                    Some(Command::Summary{capability,text,ack})=>{
                        let valid={let works=self.works.lock().await;let work=works.get(&request.thread).unwrap();work.record.capability==capability && work.record.accepted && work.record.phase=="native_summary"};
                        if !valid {let _=ack.send(Err(Error("operator_controller_fenced")));continue;}
                        let result=async { let checkpoint=context::Checkpoint::offer(&request,&text)?;
                            self.works.lock().await.get_mut(&request.thread).unwrap().checkpoint=Some(checkpoint.clone());
                            events.send(match request.compaction {Some(context::Compaction::Checkpoint)=>Event::Checkpoint{id:random_id()?,encrypted_content:checkpoint.encoded().to_owned()},_=>Event::Text{id:random_id()?,phase:"final_answer".into(),delta:text}}).await.map_err(|_|Error("native_observer_lost"))?;
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
                work.record.phase = "reconciliation_required".into();
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
            work.record.accepted = false;
            if work.record.phase != "completed" {
                work.record.phase = "reconciliation_required".into();
            }
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
                    let _ = this
                        .bridge
                        .operation(json!({"op":"release","lane":record.driver}))
                        .await;
                }
            });
        }
        while stops.join_next().await.is_some() {}
        self.reviewer.shutdown().await;
    }
}
impl Host for Operator {
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
        Box::pin(async move { this.exchange_inner(request, events, cancel).await })
    }
}
fn text<'a>(args: &'a Value, key: &str, max: usize) -> Result<&'a str, Error> {
    args[key]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= max)
        .ok_or(Error("operator_arguments_invalid"))
}
fn tools() -> Vec<Value> {
    let field = |name: &str| match name {
        "cursor" | "after" | "context_cursor" | "after_context" => {
            json!({"type":"object","properties":{"count":{"type":"integer","minimum":0},"digest":{"type":"string"}},"required":["count","digest"],"additionalProperties":false})
        }
        "renew" => json!({"type":"boolean"}),
        "answers" => {
            json!({"type":"object","additionalProperties":{"type":"object","properties":{"answers":{"type":"array","items":{"type":"string"}}},"required":["answers"],"additionalProperties":false}})
        }
        "action" => {
            json!({"type":"string","enum":["list","create","resume","fork","start","claim","compact","takeover"]})
        }
        _ => json!({"type":"string"}),
    };
    let controls = [
        ("codex_sessions", "List work, create/resume an explicit native thread, start its task, fork independent work or claim settled recovery.", "action work_id controller thread_id task call_id cursor context_cursor mode", "action", false),
        ("codex_attach", "Read current native context; echo its receipt as accept to acquire the offered controller.", "work_id controller accept", "work_id controller", false),
        ("codex_history", "Read public native history since an optional source-bound after cursor.", "work_id after after_context", "work_id", true),
        ("codex_tools", "Discover the complete current native function/custom/search schemas.", "work_id controller", "work_id controller", true),
        ("codex_execute", "Execute one advertised native tool under upstream approvals and return its original result. Reuse exact call_id/name/input/report on delivery retries.", "work_id controller call_id name input report", "work_id controller call_id name input report", false),
        ("codex_answer", "Forward the user's supplied answer to a pending native question. This cannot answer approval/reviewer requests.", "work_id controller request_id answers", "work_id controller request_id answers", false),
        ("codex_report", "Record and display a public progress report in the bound native CLI. Retry the same call_id and text after uncertain delivery.", "work_id controller call_id text", "work_id controller call_id text", false),
        ("codex_finish", "Record a final answer and confirm settled native completion.", "work_id controller call_id text", "work_id controller call_id text", false),
        ("codex_status", "Observe native execution, unresolved calls, human requests and unknown host progress.", "work_id", "work_id", true),
        ("codex_stop", "Fence new effects and request interruption; report actual confirmation separately.", "work_id", "work_id", false),
        ("codex_handoff", "Prepare a grounded summary at a history cursor, fence the old Chat and optionally request a successor on the same Codex thread.", "work_id controller call_id summary cursor context_cursor renew", "work_id controller call_id summary cursor context_cursor", false),
        ("codex_native_summary", "Answer an upstream-requested inference summary without replacing Chat.", "work_id controller summary", "work_id controller summary", false),
        ("native_catalog", "Read the isolated native reviewer tool schemas.", "turn_token", "turn_token", true),
        ("native_tool", "Run an isolated reviewer native tool and return its result.", "turn_token call_id name input report", "turn_token call_id name input report", false),
    ];
    controls.into_iter().map(|(name, description, fields, required, read)| {
        let properties = fields.split_whitespace().map(|key| (key.to_owned(), field(key))).collect::<serde_json::Map<_,_>>();
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,
            "required":required.split_whitespace().collect::<Vec<_>>(),"additionalProperties":false},
            "annotations":{"readOnlyHint":read,"destructiveHint":!read,"idempotentHint":read,"openWorldHint":!read}})
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
