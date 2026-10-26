//! Adapt retained MCP calls to the same lane protocol used by local functions.
//! No native execution, alternate transcript or repeated Chat submissions.
use crate::{
    app::{committed_final, valid_dispatches, Bridge},
    context::digest,
    journal::{random_id, Journal},
    mcp::Broker,
    protocol::{Error, Tool},
};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

const CATALOG:&str="Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from native Codex; the app must not execute tools itself.\n";
#[derive(Clone, Serialize, Deserialize)]
struct Settlement {
    round: String,
    submission: String,
    received: Vec<(String, Value)>,
    catalog: Option<BTreeMap<String, Tool>>,
    #[serde(default)]
    prepared: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Lane {
    gateway: String,
    tools: BTreeMap<String, Tool>,
    token: Option<String>,
    round: String,
    real_round: String,
    submission: String,
    model: Value,
    effort: Value,
    policy_digest: Value,
    host_tools_digest: Value,
    offered: Vec<Value>,
    public: BTreeMap<String, String>,
    last_public: BTreeMap<String, String>,
    complete: bool,
    #[serde(default)]
    settlement: Option<Settlement>,
    #[serde(default)]
    source: Option<(crate::bound_chat::Input, String)>,
    #[serde(default)]
    discovery_from: Option<String>,
    #[serde(default)]
    discovery_root: Option<String>,
}
async fn settle_round(
    lane: &mut Lane,
    broker: &Broker,
    journal: &Journal,
    key: &str,
) -> Result<(), Error> {
    let Some(intent) = lane.settlement.clone() else {
        return Ok(());
    };
    let token = lane.token.clone().ok_or(Error("mcp_turn_unavailable"))?;
    // Reapplying the exact retained results is safe even if the first durable
    // Broker commit succeeded but the lane commit/reply was lost. Never submit
    // to the app or dispatch native effects while reconciling this intent.
    let result = broker
        .settle_prepared(&token, &intent.received, intent.catalog.clone(), || {
            lane.settlement.as_mut().unwrap().prepared = true;
            journal.write(key, &*lane)
        })
        .await;
    if let Err(error) = result {
        if !lane.settlement.as_ref().unwrap().prepared {
            // Broker rejected before preparation or effects, so the old durable
            // lane is still authoritative. Do not fence it with a false intent.
            lane.settlement = None;
        }
        return Err(error);
    }
    let mut next = lane.clone();
    next.round = intent.round;
    next.submission = intent.submission;
    next.public = next.last_public.clone();
    if let Some(catalog) = intent.catalog {
        next.tools = catalog;
    }
    next.offered.clear();
    next.settlement = None;
    journal.write(key, &next)?;
    *lane = next;
    Ok(())
}
type Lanes = Arc<Mutex<BTreeMap<String, Arc<Mutex<Lane>>>>>;
pub struct McpBridge {
    remote: Arc<dyn Bridge>,
    broker: Arc<Broker>,
    journal: Arc<Journal>,
    connector: String,
    epoch: String,
    lanes: Lanes,
}
impl McpBridge {
    pub fn new(
        remote: Arc<dyn Bridge>,
        broker: Arc<Broker>,
        journal: Arc<Journal>,
        connector: String,
    ) -> Result<Self, Error> {
        Ok(Self {
            remote,
            broker,
            journal,
            connector,
            epoch: random_id()?,
            lanes: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }
}
fn tools(instructions: &[Value]) -> Result<Option<BTreeMap<String, Tool>>, Error> {
    let catalogs = instructions
        .iter()
        .filter_map(|v| v.as_str()?.strip_prefix(CATALOG))
        .collect::<Vec<_>>();
    if catalogs.len() > 1 {
        return Err(Error("mcp_catalog_ambiguous"));
    }
    catalogs
        .first()
        .map(|v| serde_json::from_str(v).map_err(|_| Error("mcp_catalog_invalid")))
        .transpose()
}
fn policy(instructions: &[Value]) -> Vec<Value> {
    instructions
        .iter()
        .filter(|v| !v.as_str().is_some_and(|v| v.starts_with(CATALOG)))
        .cloned()
        .collect()
}
fn changed_policy(lane: &Lane, command: &Value) -> bool {
    if command["policyDigest"].is_string() {
        lane.policy_digest != command["policyDigest"]
            || lane.host_tools_digest != command["hostToolsDigest"]
    } else {
        !policy(
            command["instructions"]
                .as_array()
                .map_or(&[], Vec::as_slice),
        )
        .is_empty()
    }
}
pub(crate) fn host_block(item: &Value, responses: &Value) -> Option<&'static str> {
    if responses
        .as_array()
        .is_some_and(|v| v.iter().any(|id| id == &item["id"]))
    {
        return None;
    }
    let metadata = &item["metadata"];
    let server = metadata
        .get("jit_plugin_data")
        .or_else(|| metadata.get("elicitation_metadata"))
        .and_then(|v| v.get("plugin_data").or(Some(v)))
        .and_then(|v| v.get("from_server"));
    let elicitation = metadata.get("codex_mcp_elicitation").or_else(|| {
        server
            .and_then(|v| v.get("body"))
            .and_then(|v| v.get("codex_mcp_elicitation"))
    });
    if metadata
        .get("codex_mcp_elicitation_response")
        .is_some_and(|v| !v.is_null())
        || elicitation.is_some_and(|v| v["status"].as_str().is_some_and(|s| s != "pending"))
    {
        return None;
    }
    if metadata
        .get("codex_plugin_auth_required")
        .is_some_and(|v| v["status"].as_str().is_none_or(|s| s == "pending"))
        || server.is_some_and(|v| {
            matches!(
                v["type"].as_str(),
                Some("plugin_auth_required" | "oauth_required" | "link_required")
            )
        })
    {
        Some("mcp_host_authentication_required")
    } else if server.is_some_and(|v| {
        matches!(
            v["type"].as_str(),
            Some("confirm_action" | "approve_tool_in_context_scope")
        )
    }) {
        Some("mcp_host_confirmation_required")
    } else {
        None
    }
}
fn apply_host_block(snapshot: &mut Value, item: &Value) {
    // A historical unanswered confirmation cannot hide a failed app stream.
    if !snapshot["error"].is_string() {
        if let Some(code) = host_block(item, &snapshot["hostResponses"]) {
            snapshot["error"] = json!(code);
        }
    }
}
impl Bridge for McpBridge {
    fn fence_tools(&self, marker: String) -> BoxFuture<'static, Result<(), Error>> {
        let broker = self.broker.clone();
        let journal = self.journal.clone();
        Box::pin(async move {
            let recorded: Option<Lane> = journal.read(&digest(&("mcp-lane-v1", &marker)))?;
            broker
                .stop_lane(
                    &marker,
                    recorded.as_ref().and_then(|lane| lane.token.as_deref()),
                )
                .await
        })
    }
    fn admit_submission(&self, command: Value) -> BoxFuture<'static, Result<(), Error>> {
        let lanes = self.lanes.clone();
        let remote = self.remote.clone();
        Box::pin(async move {
            let marker = command["lane"].as_str().ok_or(Error("mcp_lane_required"))?;
            let lane = lanes
                .lock()
                .await
                .get(marker)
                .cloned()
                .ok_or(Error("mcp_owner_unavailable"))?;
            let lane = lane.lock().await;
            if !command["source_input"].is_null() {
                if !lane.complete || lane.settlement.is_some() || !lane.offered.is_empty() {
                    return Err(Error("bound_chat_native_admission_invalid"));
                }
                serde_json::from_value::<crate::bound_chat::Input>(command["source_input"].clone())
                    .map_err(|_| Error("bound_chat_input_invalid"))?
                    .validate()?;
                drop(lane);
                return remote.admit_submission(command).await;
            }
            if lane.token.is_some() && !lane.complete {
                if command["messages"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                {
                    return Err(Error("mcp_pending_new_input"));
                }
                if changed_policy(&lane, &command) {
                    return Err(Error("mcp_active_policy_change"));
                }
                if lane.model != command["model"] || lane.effort != command["thinkingEffort"] {
                    return Err(Error("mcp_active_route_change"));
                }
            }
            drop(lane);
            remote.admit_submission(command).await
        })
    }
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let (remote, broker, journal, connector, lanes) = (
            self.remote.clone(),
            self.broker.clone(),
            self.journal.clone(),
            self.connector.clone(),
            self.lanes.clone(),
        );
        let epoch = self.epoch.clone();
        Box::pin(async move {
            let op = command["op"]
                .as_str()
                .ok_or(Error("mcp_operation_invalid"))?;
            if op == "identity" {
                let mut observed = remote.operation(command).await?;
                let generation = observed["generation"]
                    .as_str()
                    .ok_or(Error("app_identity_unavailable"))?;
                observed["generation"] =
                    json!(digest(&("mcp-owner-generation-v1", &epoch, generation)));
                return Ok(observed);
            }
            let restoring = op == "chat_adopt" && command["bound_source_anchor"].is_string();
            if !restoring && !matches!(op, "open" | "submit" | "snapshot" | "stop" | "release") {
                return remote.operation(command).await;
            }
            let marker = command["lane"]
                .as_str()
                .ok_or(Error("mcp_lane_required"))?
                .to_owned();
            let key = digest(&("mcp-lane-v1", &marker));
            let restored = if restoring {
                let old: Lane = journal.read(&key)?.ok_or(Error("mcp_owner_unavailable"))?;
                if !old.complete || old.settlement.is_some() || !old.offered.is_empty() {
                    return Err(Error("mcp_settlement_reconciliation_required"));
                }
                Some(old)
            } else {
                None
            };
            let mutex = {
                let mut lanes = lanes.lock().await;
                if op == "open" || restoring {
                    lanes
                        .entry(marker.clone())
                        .or_insert_with(|| Arc::new(Mutex::new(restored.unwrap_or_default())))
                        .clone()
                } else if let Some(lane) = lanes.get(&marker) {
                    lane.clone()
                } else if op == "release" {
                    return remote.operation(command).await;
                } else {
                    return Err(Error("mcp_owner_unavailable"));
                }
            };
            let mut lane = mutex.lock().await;
            if op == "open" || restoring {
                let gateway = command["signature"]["name"]
                    .as_str()
                    .ok_or(Error("mcp_gateway_required"))?;
                if !lane.gateway.is_empty() && lane.gateway != gateway {
                    return Err(Error("app_lane_declaration_changed"));
                }
                lane.gateway = gateway.into();
                if restoring {
                    let released = remote
                        .operation(json!({"op":"release","lane":marker}))
                        .await?;
                    if released["released"] != true {
                        return Err(Error("app_lane_release_unconfirmed"));
                    }
                    let observed = remote.operation(command).await?;
                    let last = lane.token.as_deref().ok_or(Error("mcp_turn_unavailable"))?;
                    let root = lane.discovery_root.as_deref().unwrap_or(last);
                    broker.restore_source_discovery(root, last, &marker).await?;
                    journal.write(&key, &*lane)?;
                    return Ok(observed);
                }
                journal.write(&key, &*lane)?;
                return remote.operation(command).await;
            }
            if op == "stop" || op == "release" {
                let preserve = op == "release" && command["preserve_settled_source"] == true;
                if preserve
                    && (!lane.complete || lane.settlement.is_some() || !lane.offered.is_empty())
                {
                    return Err(Error("mcp_pending_completion"));
                }
                let result = remote.operation(command.clone()).await;
                if preserve {
                    if let Some(token) = &lane.token {
                        broker.retire(token, false).await?;
                    }
                } else {
                    broker.stop_lane(&marker, lane.token.as_deref()).await?;
                }
                // Preserve stopped state/last round for actual stop observation.
                // Only explicit release forgets an already stopped/settled lane.
                if op == "release" && result.is_ok() {
                    lanes.lock().await.remove(&marker);
                }
                return result;
            }
            if op == "submit" {
                let round = command["round"]
                    .as_str()
                    .ok_or(Error("mcp_round_required"))?;
                if let Some(intent) = &lane.settlement {
                    if intent.round != round || intent.submission != digest(&command) {
                        return Err(Error("mcp_settlement_reconciliation_required"));
                    }
                    settle_round(&mut lane, &broker, &journal, &key).await?;
                    return Ok(json!({"started":true,"retained_mcp_return":true,"reused":true}));
                }
                if lane.round == round {
                    if lane.submission != digest(&command) {
                        return Err(Error("mcp_submission_changed"));
                    }
                    return Ok(json!({"started":true,"reused":true}));
                }
                let instructions = command["instructions"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let messages = command["messages"].as_array().cloned().unwrap_or_default();
                let results = command["results"].as_array().cloned().unwrap_or_default();
                if lane.token.is_some() && !lane.complete {
                    if !messages.is_empty() {
                        return Err(Error("mcp_pending_new_input"));
                    }
                    if changed_policy(&lane, &command) {
                        return Err(Error("mcp_active_policy_change"));
                    }
                    if lane.model != command["model"] || lane.effort != command["thinkingEffort"] {
                        return Err(Error("mcp_active_route_change"));
                    }
                    let received = results
                        .iter()
                        .map(|v| {
                            Ok((
                                v["id"]
                                    .as_str()
                                    .ok_or(Error("mcp_result_invalid"))?
                                    .to_owned(),
                                v.get("result")
                                    .cloned()
                                    .ok_or(Error("mcp_result_invalid"))?,
                            ))
                        })
                        .collect::<Result<Vec<_>, Error>>()?;
                    if received.len() != lane.offered.len()
                        || !lane
                            .offered
                            .iter()
                            .all(|v| received.iter().any(|(id, _)| v["id"] == id.as_str()))
                    {
                        return Err(Error("mcp_result_missing"));
                    }
                    let catalog = tools(&instructions)?;
                    // Record the next virtual-round intent before unblocking the
                    // app's original HTTP invocation. There is no second submit.
                    let mut next = lane.clone();
                    next.settlement = Some(Settlement {
                        round: round.into(),
                        submission: digest(&command),
                        received,
                        catalog,
                        prepared: false,
                    });
                    // Keep memory fenced too if the durable write outcome is
                    // uncertain. A later observer may retry this same commit.
                    *lane = next;
                    settle_round(&mut lane, &broker, &journal, &key).await?;
                    return Ok(json!({"started":true,"retained_mcp_return":true}));
                }
                if !results.is_empty() {
                    return Err(Error("mcp_result_without_pending_call"));
                }
                let source = if command["source_input"].is_null() {
                    None
                } else {
                    let input: crate::bound_chat::Input =
                        serde_json::from_value(command["source_input"].clone())
                            .map_err(|_| Error("bound_chat_input_invalid"))?;
                    let anchor = command["source_anchor"]
                        .as_str()
                        .ok_or(Error("bound_chat_anchor_missing"))?
                        .to_owned();
                    input.validate()?;
                    Some((input, anchor))
                };
                let catalog = tools(&instructions)?.unwrap_or_else(|| lane.tools.clone());
                let previous_token = lane.token.clone();
                let token = broker.register(&marker, catalog.clone(), true).await?;
                if source.is_none() {
                    lane.discovery_root = Some(token.clone());
                }
                // Observation can be rate-limited. Keep this prepared round
                // stable; only a validated source snapshot publishes its tools.
                lane.discovery_from = source.as_ref().and(previous_token);
                lane.tools = catalog;
                lane.token = Some(token.clone());
                lane.round = round.into();
                lane.real_round = round.into();
                lane.model = command["model"].clone();
                lane.effort = command["thinkingEffort"].clone();
                lane.policy_digest = command["policyDigest"].clone();
                lane.host_tools_digest = command["hostToolsDigest"].clone();
                lane.public.clear();
                lane.last_public.clear();
                lane.offered.clear();
                lane.complete = false;
                lane.submission = digest(&command);
                lane.source = source;
                journal.write(&key, &*lane)?;
                let mut submitted = command;
                if let Some((input, _)) = &lane.source {
                    submitted["op"] = json!("follow");
                    submitted["conversation_id"] = json!(input.conversation);
                    return remote.operation(submitted).await;
                }
                let mut native_policy = policy(&instructions);
                native_policy.push(json!(format!("You are the model for a native Codex development task. Use only the installed app {} and its native_catalog/native_tool functions for native tools. Native Codex executes, approves and records actions. Never use Work/handoff or fabricate results. Exact turn_token: {}\nFirst discover the complete native catalog using native_catalog with this token. Preserve canonical names, schemas, deferred search and custom/freeform inputs. For each action use a unique logical call_id; delivery retries MUST reuse exact call_id/name/input/report. Supply useful public initial/intermediate reports in report and a public final answer after actual results. Tool results return on the retained MCP invocation; never request another user message. Never disclose private reasoning.",serde_json::to_string(&connector).unwrap(),token)));
                submitted["instructions"] = json!(native_policy);
                return remote.operation(submitted).await;
            }
            if lane.settlement.is_some() {
                settle_round(&mut lane, &broker, &journal, &key).await?;
            }
            let mut snapshot = remote.operation(command).await?;
            if snapshot["round"] == lane.real_round && snapshot["stopped"] == true {
                snapshot["round"] = json!(lane.round);
                return Ok(snapshot);
            }
            let mut source_validated = false;
            if let Some((input, anchor)) = &lane.source {
                snapshot["sourceObservation"] = serde_json::to_value(input).unwrap();
                if snapshot["savedReadBackoff"]["cooling_down"] == true {
                    snapshot["messages"] = json!([]);
                    snapshot["terminals"] = json!([]);
                } else {
                    let observed = crate::bound_chat::read(
                        remote.clone(),
                        &input.conversation,
                        anchor,
                        Some(input),
                    )
                    .await?
                    .ok_or(Error("bound_chat_input_missing"))?;
                    snapshot["messages"] = json!(observed.messages);
                    snapshot["terminals"] = if observed.ended {
                        json!(["done"])
                    } else {
                        json!([])
                    };
                    source_validated = true;
                }
                if snapshot["round"] != lane.real_round {
                    // Attaching observation has no model/effect dispatch. A lost
                    // attach reply can be reconciled using this exact identity.
                    remote
                        .operation(json!({"op":"follow","lane":marker,"round":lane.real_round,
                        "conversation_id":input.conversation}))
                        .await?;
                    let mut attached = remote
                        .operation(json!({"op":"snapshot","lane":marker}))
                        .await?;
                    for field in ["messages", "terminals", "sourceObservation"] {
                        attached[field] = snapshot[field].take();
                    }
                    snapshot = attached;
                }
            }
            let before = digest(&*lane);
            if snapshot["round"] != lane.real_round {
                return Err(Error("app_round_identity_unconfirmed"));
            }
            if source_validated {
                if let Some(previous) = &lane.discovery_from {
                    let token = lane.token.as_deref().ok_or(Error("mcp_turn_unavailable"))?;
                    broker.source_discovery(previous, token).await?;
                    lane.discovery_from = None;
                    journal.write(&key, &*lane)?;
                }
            }
            if snapshot["stopped"] == true {
                snapshot["round"] = json!(lane.round);
                return Ok(snapshot);
            }
            let token = lane
                .token
                .as_ref()
                .ok_or(Error("mcp_turn_unavailable"))?
                .clone();
            if lane.offered.is_empty() && !lane.complete {
                lane.offered = broker.offer(&token).await?;
            }
            let pending = !lane.offered.is_empty();
            let raw = snapshot["messages"]
                .as_array()
                .cloned()
                .ok_or(Error("invalid_app_snapshot"))?;
            let mut visible = Vec::new();
            let mut observed = BTreeMap::new();
            for item in &raw {
                if item["author"]["role"] == "tool" {
                    apply_host_block(&mut snapshot, item);
                    continue;
                }
                if item["author"]["role"] != "assistant"
                    || item["recipient"].as_str().is_some_and(|v| v != "all")
                    || item["metadata"]["is_visually_hidden_from_conversation"] == true
                    || !matches!(
                        item["channel"].as_str(),
                        None | Some("final" | "commentary")
                    )
                {
                    continue;
                }
                let id = item["id"].as_str().ok_or(Error("invalid_public_message"))?;
                let text = item["content"]["parts"]
                    .as_array()
                    .ok_or(Error("invalid_public_message"))?
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<String>();
                let old = lane.public.get(id).map_or("", String::as_str);
                if !text.starts_with(old) {
                    return Err(Error("public_message_changed"));
                }
                observed.insert(id.into(), text.clone());
                let complete_final = !pending && committed_final(item);
                let full_final = complete_final && !old.is_empty();
                if text.len() == old.len() && !old.is_empty() && !full_final {
                    continue;
                }
                let mut item = item.clone();
                // The app-wire consumer retains physical identity separately
                // from the distinct canonical IDs required by virtual rounds.
                item["provider_source_message"] = json!(id);
                // One app message can cross several native Responses rounds.
                // Reusing its source ID would replace earlier canonical text.
                // Commit the complete final under a separate item if its prefix
                // was already emitted before a native tool result returned.
                item["id"] = json!(digest(&("mcp-public-item-v1", &lane.round, id, full_final)));
                item["content"]["parts"] = json!([if full_final {
                    &text[..]
                } else {
                    &text[old.len()..]
                }]);
                if pending {
                    item["end_turn"] = json!(false);
                    item["status"] = json!("finished_partial_completion");
                }
                visible.push(item);
            }
            lane.last_public = observed;
            if pending {
                if snapshot["terminals"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                {
                    return Err(Error("mcp_terminal_with_pending_calls"));
                }
                if !valid_dispatches(&snapshot) {
                    return Err(Error("mcp_dispatch_unobserved"));
                }
                for call in &lane.offered {
                    visible.push(json!({"id":call["id"],"author":{"role":"assistant"},"recipient":format!("local.{}",lane.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"name":call["name"],"input":call["input"],"report":call["report"]}}).to_string()},"status":"finished_successfully","end_turn":false}));
                }
                snapshot["terminals"] = json!(["done"]);
            } else if !snapshot["error"].is_string() && snapshot["terminals"] == json!(["done"]) {
                if let Some((source, _)) = lane.source.as_ref().filter(|_| !lane.complete) {
                    let final_id = raw
                        .iter()
                        .rev()
                        .find(|message| committed_final(message))
                        .and_then(|message| message["id"].as_str())
                        .ok_or(Error("invalid_public_message"))?;
                    remote
                        .operation(
                            json!({"op":"follow_settle","lane":marker,"round":lane.real_round,
                        "conversation_id":source.conversation,"branch":final_id,
                        "text":lane.last_public.get(final_id).ok_or(Error("invalid_public_message"))?}),
                        )
                        .await?;
                }
                broker.retire(&token, false).await?;
                lane.complete = true;
            }
            snapshot["round"] = json!(lane.round);
            snapshot["messages"] = json!(visible);
            // This count represents actual app requests, not virtual Responses
            // boundaries or MCP JSON-RPC calls. Host permission replies have
            // their own hostReplyMessages counter; MCP results return in place.
            snapshot["toolMessages"] = json!(0);
            if before != digest(&*lane) {
                journal.write(&key, &*lane)?;
            }
            Ok(snapshot)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SettlementRoot(std::path::PathBuf);
    impl Drop for SettlementRoot {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn original_call() -> crate::mcp::Arguments {
        crate::mcp::Arguments {
            call_id: "native-once".into(),
            name: "exec".into(),
            input: "{}".into(),
            report: "Owned original call".into(),
        }
    }
    async fn pending_settlement() -> (SettlementRoot, Arc<Journal>, Arc<Broker>, Lane) {
        let root = SettlementRoot(
            std::env::temp_dir().join(format!("mcp-settlement-{}", random_id().unwrap())),
        );
        let journal = Arc::new(Journal::open(&root.0).unwrap());
        let broker = Broker::new(journal.clone());
        let request = crate::protocol::Request::parse(
            json!({"stream":true,"model":"chatgpt-desktop/gpt-6","input":[{"role":"user","content":"Owned settlement fixture"}],"tools":[{"type":"function","name":"exec","parameters":{"type":"object"}}]}),
            &crate::config::Config::default(),
            Some("owned-native-thread"),
        )
        .unwrap();
        let token = broker
            .register("owned-native-thread", request.tools.clone(), false)
            .await
            .unwrap();
        broker.enqueue(&token, &original_call()).await.unwrap();
        let offered = broker.offer(&token).await.unwrap();
        assert_eq!(offered.len(), 1);
        let lane = Lane {
            token: Some(token),
            tools: request.tools,
            round: "original-round".into(),
            real_round: "physical-app-round".into(),
            offered: offered.clone(),
            settlement: Some(Settlement {
                round: "next-round".into(),
                submission: "original-submission-fingerprint".into(),
                received: vec![(
                    offered[0]["id"].as_str().unwrap().into(),
                    json!({"output":"original native result"}),
                )],
                catalog: None,
                prepared: false,
            }),
            ..Lane::default()
        };
        (root, journal, broker, lane)
    }
    #[tokio::test]
    async fn rejected_settlement_keeps_the_original_round_without_preparing_an_intent() {
        let (_root, journal, broker, mut lane) = pending_settlement().await;
        let key = digest(&"owned-virtual-lane");
        let mut previous = lane.clone();
        previous.settlement = None;
        journal.write(&key, &previous).unwrap();
        lane.settlement.as_mut().unwrap().received[0].0 = "unknown-call".into();
        assert_eq!(
            settle_round(&mut lane, &broker, &journal, &key).await,
            Err(Error("mcp_result_invalid"))
        );
        assert_eq!(lane.round, previous.round);
        assert!(lane.settlement.is_none());
        let stored: Lane = journal.read(&key).unwrap().unwrap();
        assert!(stored.settlement.is_none());
        assert_eq!(stored.round, previous.round);
        assert!(
            !broker.recorded_calls(lane.token.as_ref().unwrap()).unwrap()[0]["settled"]
                .as_bool()
                .unwrap()
        );
    }
    #[tokio::test]
    async fn broker_commit_failure_retains_prepared_intent_until_original_result_reconciles() {
        let (_root, journal, broker, mut lane) = pending_settlement().await;
        let key = digest(&"owned-virtual-lane");
        let token = lane.token.clone().unwrap();
        let broker_path = journal
            .root
            .join(format!("{}.json", digest(&("mcp-turn-v1", &token))));
        let backup = journal.root.join("owned-broker-record-before-fault");
        std::fs::rename(&broker_path, &backup).unwrap();
        crate::journal::private_directory(&broker_path).unwrap();
        let failed = settle_round(&mut lane, &broker, &journal, &key).await;
        std::fs::remove_dir(&broker_path).unwrap();
        std::fs::rename(&backup, &broker_path).unwrap();
        assert!(failed.is_err());
        assert_eq!(lane.round, "original-round");
        assert!(lane.settlement.as_ref().unwrap().prepared);
        let stored: Lane = journal.read(&key).unwrap().unwrap();
        assert!(stored.settlement.as_ref().unwrap().prepared);
        assert_eq!(broker.recorded_calls(&token).unwrap()[0]["settled"], false);
        settle_round(&mut lane, &broker, &journal, &key)
            .await
            .unwrap();
        assert_eq!(lane.round, "next-round");
        assert!(lane.settlement.is_none());
        assert_eq!(broker.recorded_calls(&token).unwrap().len(), 1);
        assert!(broker.offer(&token).await.unwrap().is_empty());
        assert_eq!(
            broker.invoke(&token, original_call()).await.unwrap(),
            json!({"output":"original native result"})
        );
    }
    #[tokio::test]
    async fn lane_ack_loss_after_broker_commit_reuses_the_original_result() {
        let (_root, journal, broker, mut lane) = pending_settlement().await;
        let key = digest(&"owned-virtual-lane");
        let token = lane.token.clone().unwrap();
        lane.settlement.as_mut().unwrap().prepared = true;
        journal.write(&key, &lane).unwrap();
        broker
            .settle(&token, &lane.settlement.as_ref().unwrap().received)
            .await
            .unwrap();
        let mut recovered: Lane = journal.read(&key).unwrap().unwrap();
        assert_eq!(recovered.round, "original-round");
        settle_round(&mut recovered, &broker, &journal, &key)
            .await
            .unwrap();
        assert_eq!(recovered.round, "next-round");
        assert_eq!(recovered.real_round, "physical-app-round");
        assert!(recovered.offered.is_empty() && recovered.settlement.is_none());
        assert_eq!(broker.recorded_calls(&token).unwrap().len(), 1);
        assert_eq!(
            broker.invoke(&token, original_call()).await.unwrap(),
            json!({"output":"original native result"})
        );
    }
    #[tokio::test]
    async fn a_new_broker_cannot_reactivate_a_prepared_old_round() {
        let (_root, journal, broker, mut lane) = pending_settlement().await;
        let key = digest(&"owned-virtual-lane");
        let token = lane.token.clone().unwrap();
        lane.settlement.as_mut().unwrap().prepared = true;
        journal.write(&key, &lane).unwrap();
        drop(broker);
        let replacement = Broker::new(journal.clone());
        let mut recovered: Lane = journal.read(&key).unwrap().unwrap();
        assert_eq!(
            settle_round(&mut recovered, &replacement, &journal, &key).await,
            Err(Error("mcp_turn_unavailable"))
        );
        assert!(recovered.settlement.as_ref().unwrap().prepared);
        assert_eq!(recovered.round, "original-round");
        assert_eq!(replacement.recorded_calls(&token).unwrap().len(), 1);
        assert_eq!(
            replacement.catalog(&token).await,
            Err(Error("mcp_turn_unavailable"))
        );
    }
    #[test]
    fn historical_confirmation_does_not_mask_a_failed_stream() {
        let item = json!({"id":"confirm","metadata":{"jit_plugin_data":{"from_server":{"type":"confirm_action"}}}});
        let mut snapshot = json!({"hostResponses":[],"error":"app_stream_failed"});
        apply_host_block(&mut snapshot, &item);
        assert_eq!(snapshot["error"], "app_stream_failed");
        snapshot["error"] = Value::Null;
        apply_host_block(&mut snapshot, &item);
        assert_eq!(snapshot["error"], "mcp_host_confirmation_required");
    }
    #[test]
    fn only_explicitly_counted_host_replies_extend_the_original_dispatch() {
        assert!(valid_dispatches(&json!({"roundStarts":1})));
        assert!(valid_dispatches(
            &json!({"roundStarts":2,"roundHostReplies":1})
        ));
        assert!(!valid_dispatches(
            &json!({"roundStarts":2,"roundHostReplies":0})
        ));
        assert!(!valid_dispatches(
            &json!({"roundStarts":1,"roundHostReplies":"wrong"})
        ));
        assert!(!valid_dispatches(
            &json!({"roundStarts":0,"roundHostReplies":u64::MAX})
        ));
    }
    #[test]
    fn only_unresolved_matching_host_actions_block_the_retained_request() {
        let mut item = json!({"id":"confirmation","metadata":{"jit_plugin_data":{"from_server":{"type":"confirm_action"}}}});
        assert_eq!(
            host_block(&item, &json!([])),
            Some("mcp_host_confirmation_required")
        );
        assert_eq!(
            host_block(&item, &json!(["unrelated"])),
            Some("mcp_host_confirmation_required")
        );
        assert_eq!(host_block(&item, &json!(["confirmation"])), None);
        item["metadata"]["codex_mcp_elicitation_response"] =
            json!({"target_message_id":"confirmation"});
        assert_eq!(host_block(&item, &json!([])), None);
        item["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("codex_mcp_elicitation_response");
        item["metadata"]["codex_mcp_elicitation"] = json!({"status":"accepted"});
        assert_eq!(host_block(&item, &json!([])), None);
        item["metadata"]["codex_mcp_elicitation"] = json!({"status":"pending"});
        assert_eq!(
            host_block(&item, &json!([])),
            Some("mcp_host_confirmation_required")
        );
        let auth = json!({"metadata":{"codex_plugin_auth_required":{"status":"pending"}}});
        assert_eq!(
            host_block(&auth, &json!([])),
            Some("mcp_host_authentication_required")
        );
        let resolved = json!({"metadata":{"codex_plugin_auth_required":{"status":"completed"}}});
        assert_eq!(host_block(&resolved, &json!([])), None);
    }
}
