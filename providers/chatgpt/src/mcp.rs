//! Retained MCP invocations. This module never executes a tool or owns native
//! history. A detached HTTP peer may retry its exact logical call; it cannot
//! cause a second native action. Only the common app lane can offer/settle it.
use crate::{
    context::digest,
    journal::{random_id, Journal},
    protocol::{gateway_calls, Error, Tool},
};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Notify};

const LIMIT: usize = 16 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
struct Invocation {
    fingerprint: String,
    pub id: String,
    pub name: String,
    pub input: String,
    pub report: String,
    offered: bool,
    result: Option<Value>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Turn {
    lane: String,
    tools: BTreeMap<String, Tool>,
    calls: BTreeMap<String, Invocation>,
    retired: bool,
    order: u64,
}
pub struct Broker {
    journal: Arc<Journal>,
    turns: Mutex<BTreeMap<String, Turn>>,
    changed: Notify,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arguments {
    pub call_id: String,
    pub name: String,
    pub input: String,
    pub report: String,
}
impl Broker {
    pub fn new(journal: Arc<Journal>) -> Arc<Self> {
        Arc::new(Self {
            journal,
            turns: Mutex::new(BTreeMap::new()),
            changed: Notify::new(),
        })
    }
    fn persist(&self, token: &str, turn: &Turn) -> Result<(), Error> {
        self.journal.write(&digest(&("mcp-turn-v1", token)), turn)
    }
    pub async fn register(
        &self,
        lane: &str,
        tools: BTreeMap<String, Tool>,
    ) -> Result<String, Error> {
        let mut turns = self.turns.lock().await;
        if turns.values().any(|v| !v.retired && v.lane == lane) {
            return Err(Error("mcp_owner_conflict"));
        }
        let order = turns
            .values()
            .map(|v| v.order)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if turns.len() >= 256 {
            let oldest = turns
                .iter()
                .filter(|(_, v)| v.retired)
                .min_by_key(|(_, v)| v.order)
                .map(|(key, _)| key.clone())
                .ok_or(Error("mcp_active_turn_limit"))?;
            turns.remove(&oldest);
        }
        let token = random_id()?;
        let turn = Turn {
            lane: lane.into(),
            tools,
            calls: BTreeMap::new(),
            retired: false,
            order,
        };
        self.persist(&token, &turn)?;
        turns.insert(token.clone(), turn);
        Ok(token)
    }
    // Records are not silently reactivated after process/host loss. A live turn
    // capability belongs to this owner generation, not merely to a journal file.
    pub async fn catalog(&self, token: &str) -> Result<BTreeMap<String, Tool>, Error> {
        let turns = self.turns.lock().await;
        let turn = turns
            .get(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn.tools.clone())
    }
    pub async fn update(&self, token: &str, tools: BTreeMap<String, Tool>) -> Result<(), Error> {
        let mut turns = self.turns.lock().await;
        let turn = turns
            .get_mut(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        if turn.calls.values().any(|v| v.result.is_none()) {
            return Err(Error("mcp_pending_catalog_change"));
        }
        let mut next = turn.clone();
        next.tools = tools;
        self.persist(token, &next)?;
        *turn = next;
        Ok(())
    }
    /// Register before waiting. Cancelling this future only drops the observer.
    pub async fn enqueue(&self, token: &str, args: &Arguments) -> Result<(), Error> {
        if args.call_id.is_empty()
            || args.call_id.len() > 160
            || !args
                .call_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
            || args.input.len() > LIMIT
            || args.report.len() > 64 * 1024
        {
            return Err(Error("mcp_call_invalid"));
        }
        let fingerprint = digest(&(&args.name, &args.input, &args.report));
        let mut turns = self.turns.lock().await;
        let turn = turns.get_mut(token).ok_or(Error("mcp_turn_unavailable"))?;
        if let Some(call) = turn.calls.get(&args.call_id) {
            if call.fingerprint != fingerprint {
                return Err(Error("mcp_call_changed"));
            }
            if turn.retired && call.result.is_none() {
                return Err(Error("mcp_turn_unavailable"));
            }
            return Ok(());
        }
        if turn.retired {
            return Err(Error("mcp_turn_unavailable"));
        }
        if turn.calls.len() >= 4096 {
            return Err(Error("mcp_turn_call_limit"));
        }
        let id = format!("mcp_{}", digest(&(token, &args.call_id)));
        gateway_calls(
            &id,
            &json!({"name":args.name,"input":args.input,"report":args.report}).to_string(),
            &turn.tools,
        )?;
        let mut next = turn.clone();
        next.calls.insert(
            args.call_id.clone(),
            Invocation {
                fingerprint,
                id,
                name: args.name.clone(),
                input: args.input.clone(),
                report: args.report.clone(),
                offered: false,
                result: None,
            },
        );
        self.persist(token, &next)?;
        *turn = next;
        self.changed.notify_waiters();
        Ok(())
    }
    pub fn recorded_calls(&self, token: &str) -> Result<Vec<Value>, Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn
            .calls
            .values()
            .map(
                |call| json!({"id":call.id,"offered":call.offered,"settled":call.result.is_some()}),
            )
            .collect())
    }
    pub fn contains_call(&self, token: &str, call_id: &str) -> Result<bool, Error> {
        let turn: Turn = self
            .journal
            .read(&digest(&("mcp-turn-v1", token)))?
            .ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn.calls.contains_key(call_id))
    }
    pub async fn unsettled(&self, token: &str) -> Result<Vec<String>, Error> {
        let turns = self.turns.lock().await;
        let turn = turns.get(token).ok_or(Error("mcp_turn_unavailable"))?;
        Ok(turn
            .calls
            .values()
            .filter(|v| v.result.is_none())
            .map(|v| v.id.clone())
            .collect())
    }
    pub async fn invoke(&self, token: &str, args: Arguments) -> Result<Value, Error> {
        self.enqueue(token, &args).await?;
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let turns = self.turns.lock().await;
                let turn = turns.get(token).ok_or(Error("mcp_turn_unavailable"))?;
                let call = turn
                    .calls
                    .get(&args.call_id)
                    .ok_or(Error("mcp_turn_unavailable"))?;
                if let Some(result) = &call.result {
                    return Ok(result.clone());
                }
                if turn.retired {
                    return Err(Error("mcp_turn_unavailable"));
                }
            }
            changed.await;
        }
    }
    pub async fn offer(&self, token: &str) -> Result<Vec<Value>, Error> {
        let mut turns = self.turns.lock().await;
        let turn = turns
            .get_mut(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        let mut next = turn.clone();
        let calls = next
            .calls
            .values_mut()
            .filter(|v| !v.offered && v.result.is_none())
            .take(32)
            .map(|v| {
                v.offered = true;
                json!({"id":v.id,"name":v.name,"input":v.input,"report":v.report})
            })
            .collect::<Vec<_>>();
        if !calls.is_empty() {
            self.persist(token, &next)?;
            *turn = next;
        }
        Ok(calls)
    }
    pub async fn settle(&self, token: &str, results: &[(String, Value)]) -> Result<(), Error> {
        self.settle_with_catalog(token, results, None).await
    }
    pub async fn settle_with_catalog(
        &self,
        token: &str,
        results: &[(String, Value)],
        catalog: Option<BTreeMap<String, Tool>>,
    ) -> Result<(), Error> {
        let mut turns = self.turns.lock().await;
        let turn = turns
            .get_mut(token)
            .filter(|v| !v.retired)
            .ok_or(Error("mcp_turn_unavailable"))?;
        let mut next = turn.clone();
        let mut seen = std::collections::BTreeSet::new();
        for (id, result) in results {
            if !seen.insert(id) {
                return Err(Error("mcp_result_invalid"));
            }
            let call = next
                .calls
                .values_mut()
                .find(|v| v.id == *id && v.offered)
                .ok_or(Error("mcp_result_invalid"))?;
            if let Some(old) = &call.result {
                if old != result {
                    return Err(Error("mcp_result_changed"));
                }
            } else {
                call.result = Some(result.clone());
            }
        }
        if let Some(catalog) = catalog {
            if next.calls.values().any(|v| v.result.is_none()) {
                return Err(Error("mcp_pending_catalog_change"));
            }
            next.tools = catalog;
        }
        // All-or-nothing catalog/result validation and durable commit precede
        // waking the model, which may immediately discover/call the new tools.
        self.persist(token, &next)?;
        *turn = next;
        self.changed.notify_waiters();
        Ok(())
    }
    pub async fn retire(&self, token: &str, stopped: bool) -> Result<(), Error> {
        let mut turns = self.turns.lock().await;
        let turn = turns.get_mut(token).ok_or(Error("mcp_turn_unavailable"))?;
        if !stopped && turn.calls.values().any(|v| v.result.is_none()) {
            return Err(Error("mcp_pending_completion"));
        }
        let mut next = turn.clone();
        next.retired = true;
        self.persist(token, &next)?;
        *turn = next;
        // Keep settled results while this owner runs so a lost final HTTP
        // response can be reconciled. Retirement forbids fresh invocations.
        self.changed.notify_waiters();
        Ok(())
    }
    pub async fn close(&self) {
        let mut turns = self.turns.lock().await;
        for (token, turn) in turns.iter_mut() {
            turn.retired = true;
            // Revocation must wake observers even if the filesystem fails;
            // another process never reactivates these generation-bound tokens.
            let _ = self.persist(token, turn);
        }
        self.changed.notify_waiters();
    }
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/mcp", axum::routing::post(http))
            .with_state(self)
    }
}

async fn http(State(broker): State<Arc<Broker>>, request: Request) -> Response {
    if request.headers().contains_key("origin") {
        return StatusCode::FORBIDDEN.into_response();
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
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
    };
    let row: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"invalid_json"}}),
            ),
        )
            .into_response(),
    };
    let id = row.get("id").cloned();
    if row["jsonrpc"] != "2.0"
        || !row["method"].is_string()
        || id
            .as_ref()
            .is_some_and(|v| !v.is_string() && !v.is_number())
    {
        return (StatusCode::BAD_REQUEST,Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"invalid_request"}}))).into_response();
    }
    let method = row["method"].as_str().unwrap();
    let Some(id) = id else {
        return if method.starts_with("notifications/") {
            StatusCode::ACCEPTED.into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        };
    };
    let result = match method {
        "initialize" => match row["params"]["protocolVersion"].as_str() {
            Some(version @ ("2025-03-26" | "2025-06-18" | "2025-11-25")) => Ok(
                json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"termux-codex","version":env!("CARGO_PKG_VERSION")}}),
            ),
            _ => Err(Error("mcp_protocol_unqualified")),
        },
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools":[
            {"name":"native_catalog","description":"Read the tool names and input schemas available to the selected native Codex turn.","inputSchema":{"type":"object","properties":{"turn_token":{"type":"string"}},"required":["turn_token"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}},
            {"name":"native_tool","description":"Run one advertised native Codex tool and return its execution result.","inputSchema":{"type":"object","properties":{"turn_token":{"type":"string"},"call_id":{"type":"string"},"name":{"type":"string"},"input":{"type":"string"},"report":{"type":"string"}},"required":["turn_token","call_id","name","input","report"],"additionalProperties":false},"annotations":{"readOnlyHint":false,"destructiveHint":true,"idempotentHint":false,"openWorldHint":true}}
        ]})),
        "tools/call" => {
            let args = &row["params"]["arguments"];
            let token = args["turn_token"]
                .as_str()
                .filter(|v| v.len() == 48 && v.bytes().all(|b| b.is_ascii_hexdigit()));
            let value = if let Some(token) = token {
                match row["params"]["name"].as_str() {
                    Some("native_catalog") if args.as_object().is_some_and(|v| v.len() == 1) => {
                        broker.catalog(token).await.map(|v| json!(v))
                    }
                    Some("native_tool") => {
                        let mut input = args.clone();
                        input.as_object_mut().unwrap().remove("turn_token");
                        match serde_json::from_value(input) {
                            Ok(input) => match tokio::time::timeout(
                                Duration::from_secs(600),
                                broker.invoke(token, input),
                            )
                            .await
                            {
                                Ok(v) => v,
                                Err(_) => Err(Error("mcp_observer_timeout")),
                            },
                            Err(_) => Err(Error("mcp_call_invalid")),
                        }
                    }
                    _ => Err(Error("mcp_tool_unknown")),
                }
            } else {
                Err(Error("mcp_turn_unavailable"))
            };
            Ok(match value {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                }
                Err(error) => {
                    json!({"content":[{"type":"text","text":json!({"error":error.0}).to_string()}],"isError":true})
                }
            })
        }
        _ => Err(Error("method_not_found")),
    };
    let body = match result {
        Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
        Err(e) => {
            json!({"jsonrpc":"2.0","id":id,"error":{"code":if e.0=="method_not_found"{-32601}else{-32602},"message":e.0}})
        }
    };
    ([("cache-control", "no-store")], Json(body)).into_response()
}
