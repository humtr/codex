//! Adapt retained MCP calls to the same lane protocol used by local functions.
//! No native execution, alternate transcript or repeated Chat submissions.
use crate::{
    app::Bridge,
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

const CATALOG:&str="Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from Core; the app must not execute tools itself.\n";
#[derive(Default, Serialize, Deserialize)]
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
fn valid_dispatches(snapshot: &Value) -> bool {
    let replies = match snapshot.get("roundHostReplies") {
        None => 0,
        Some(value) => match value.as_u64() {
            Some(value) => value,
            None => return false,
        },
    };
    matches!(replies.checked_add(1), Some(expected) if snapshot["roundStarts"].as_u64() == Some(expected))
}
fn host_block(item: &Value, responses: &Value) -> Option<&'static str> {
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
impl Bridge for McpBridge {
    fn admit_submission(&self, command: Value) -> BoxFuture<'static, Result<(), Error>> {
        let lanes = self.lanes.clone();
        Box::pin(async move {
            let marker = command["lane"].as_str().ok_or(Error("mcp_lane_required"))?;
            let lane = lanes
                .lock()
                .await
                .get(marker)
                .cloned()
                .ok_or(Error("mcp_owner_unavailable"))?;
            let lane = lane.lock().await;
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
            Ok(())
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
            if !matches!(op, "open" | "submit" | "snapshot" | "stop" | "release") {
                return remote.operation(command).await;
            }
            let marker = command["lane"]
                .as_str()
                .ok_or(Error("mcp_lane_required"))?
                .to_owned();
            let mutex = {
                let mut lanes = lanes.lock().await;
                if op == "open" {
                    lanes
                        .entry(marker.clone())
                        .or_insert_with(|| Arc::new(Mutex::new(Lane::default())))
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
            let key = digest(&("mcp-lane-v1", &marker));
            if op == "open" {
                let gateway = command["signature"]["name"]
                    .as_str()
                    .ok_or(Error("mcp_gateway_required"))?;
                if !lane.gateway.is_empty() && lane.gateway != gateway {
                    return Err(Error("app_lane_declaration_changed"));
                }
                lane.gateway = gateway.into();
                journal.write(&key, &*lane)?;
                return remote.operation(command).await;
            }
            if op == "stop" || op == "release" {
                let result = remote.operation(command.clone()).await;
                if let Some(token) = &lane.token {
                    broker.retire(token, true).await?;
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
                    let token = lane.token.as_ref().unwrap().clone();
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
                    let old_round = lane.round.clone();
                    lane.round = round.into();
                    journal.write(&key, &*lane)?;
                    if let Err(error) = broker
                        .settle_with_catalog(&token, &received, catalog.clone())
                        .await
                    {
                        lane.round = old_round;
                        return Err(error);
                    }
                    lane.public = lane.last_public.clone();
                    if let Some(catalog) = catalog {
                        lane.tools = catalog;
                    }
                    lane.offered.clear();
                    lane.submission = digest(&command);
                    journal.write(&key, &*lane)?;
                    return Ok(json!({"started":true,"retained_mcp_return":true}));
                }
                if !results.is_empty() {
                    return Err(Error("mcp_result_without_pending_call"));
                }
                let catalog = tools(&instructions)?.unwrap_or_else(|| lane.tools.clone());
                let token = broker.register(&marker, catalog.clone()).await?;
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
                journal.write(&key, &*lane)?;
                let mut submitted = command;
                let mut native_policy = policy(&instructions);
                native_policy.push(json!(format!("You are the model for a native Codex development task. Use only the installed app {} and its native_catalog/native_tool functions for native tools. Core executes, approves and records actions. Never use Work/handoff or fabricate results. Exact turn_token: {}\nFirst discover the complete native catalog using native_catalog with this token. Preserve canonical names, schemas, deferred search and custom/freeform inputs. For each action use a unique logical call_id; delivery retries MUST reuse exact call_id/name/input/report. Supply useful public initial/intermediate reports in report and a public final answer after actual results. Tool results return on the retained MCP invocation; never request another user message. Never disclose private reasoning.",serde_json::to_string(&connector).unwrap(),token)));
                submitted["instructions"] = json!(native_policy);
                return remote.operation(submitted).await;
            }
            let mut snapshot = remote.operation(command).await?;
            let before = digest(&*lane);
            if snapshot["round"] != lane.real_round {
                return Err(Error("app_round_identity_unconfirmed"));
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
                    if let Some(code) = host_block(item, &snapshot["hostResponses"]) {
                        snapshot["error"] = json!(code);
                    }
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
                if text.len() == old.len() && !old.is_empty() {
                    continue;
                }
                let mut item = item.clone();
                item["content"]["parts"] = json!([&text[old.len()..]]);
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
            } else if snapshot["terminals"] == json!(["done"]) {
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
