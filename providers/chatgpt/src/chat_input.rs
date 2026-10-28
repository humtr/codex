//! Explicit saved-Chat input receipts. Saved Chat remains the result authority;
//! this journal stores admission/delivery identity and never replays an intent.
use crate::{
    app::Bridge, chats::Chats, config::Config, context::digest, journal::Journal, protocol::Error,
};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

pub(crate) struct Inputs {
    pub bridge: Arc<dyn Bridge>,
    pub journal: Arc<Journal>,
    pub config: Config,
}
impl Inputs {
    async fn identity(&self) -> Result<Value, Error> {
        let identity = self.bridge.operation(json!({"op":"identity"})).await?;
        Ok(
            json!({"account":identity["account"],"user":identity["user"],"structure":identity["structure"]}),
        )
    }
    pub async fn submit(&self, args: Value, secrets: &[String]) -> Result<Value, Error> {
        let call = string(&args, "call_id", 160)?;
        let key = digest(&(
            "saved-chat-input-v1",
            &args["conversation_id"],
            &args["source_cursor"]["account"],
            call,
        ));
        let fingerprint = digest(&args);
        if let Some(record) = self.journal.read::<Value>(&key)? {
            if record["fingerprint"] != fingerprint {
                return Err(Error("operator_call_changed"));
            }
            if self.identity().await? != record["identity"] {
                return Err(Error("app_account_changed"));
            }
            // The first delivery can be unknown. A retry observes that intent;
            // only result reconciliation may later certify saved delivery.
            return Ok(public(&record));
        }
        let conversation = string(&args, "conversation_id", 256)?;
        let task = string(&args, "task", 256 * 1024)?;
        let cursor = &args["source_cursor"];
        let requested_parent = string(cursor, "branch", 256)?;
        if cursor["conversation"] != conversation {
            return Err(Error("chat_cursor_invalid"));
        }
        let source=Chats::new(self.bridge.clone()).with_public_ids(crate::control::public_work_ids(&self.journal)?).call(json!({"action":"read","conversation_id":conversation,"branch":requested_parent,"after":cursor,"limit":1}),secrets).await?;
        if source["complete"] != true || source["messages"].as_array().is_none_or(|v| !v.is_empty())
        {
            return Err(Error("chat_input_source_incomplete"));
        }
        let delivery_cursor = source
            .get("input_cursor")
            .filter(|v| v.is_object())
            .unwrap_or(cursor);
        let branch = string(delivery_cursor, "branch", 256)?;
        let identity = self.identity().await?;
        if digest(&identity) != cursor["account"].as_str().unwrap_or("") {
            return Err(Error("app_account_changed"));
        }
        let starts = args["budget"]["max_app_starts"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= 10_000)
            .ok_or(Error("assignment_budget_invalid"))?;
        let elapsed = args["budget"]["max_elapsed_seconds"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= 86_400)
            .ok_or(Error("assignment_budget_invalid"))?;
        let lane = format!("saved-input-{key}");
        // The message ID is stable across observers, restart and lost delivery.
        let message = format!(
            "{}-{}-4{}-a{}-{}",
            &key[..8],
            &key[8..12],
            &key[13..16],
            &key[17..20],
            &key[20..32]
        );
        let adopted=self.bridge.operation(json!({"op":"chat_adopt","lane":lane,"nativeThread":lane,"limits":{"maxDispatches":starts,"deadlineUnixMs":unix_ms().saturating_add(elapsed.saturating_mul(1000))},"conversation_id":conversation,"identity":identity,"branch":branch,"project":source["project_id"],"returnPath":"mcp","readSelection":true,"signature":{"name":format!("saved_input_{key}")}})).await?;
        let selection = match resolve_selection(&self.config.catalog, &adopted["selection"], &args)
        {
            Ok(selection) => selection,
            Err(error) => {
                let _ = self
                    .bridge
                    .operation(json!({"op":"release","lane":lane}))
                    .await;
                return Err(error);
            }
        };
        let mut record = json!({"schema":"termux-codex-saved-chat-input-v1","input_id":key,"fingerprint":fingerprint,"conversation_id":conversation,"source_cursor":cursor,"delivery_source_cursor":delivery_cursor,"identity":identity,"lane":lane,"message_id":message,"settings":selection,"phase":"delivery_unknown","admitted_at_unix_ms":unix_ms()});
        // A failed journal admission has made no inference. Once durable, no
        // retry of this call can send a second user message.
        if let Err(error) = self.journal.write(&key, &record) {
            let _ = self
                .bridge
                .operation(json!({"op":"release","lane":lane}))
                .await;
            return Err(error);
        }
        let sent=self.bridge.operation(json!({"op":"submit","lane":lane,"round":key,"messageId":message,"model":selection["host_model"],"thinkingEffort":selection["thinking_effort"],"messages":[{"role":"user","text":task,"hidden":false}]})).await;
        if sent.is_ok() {
            record["phase"] = json!("submitted");
        } else if let Err(Error(
            cause @ ("app_model_selection_changed" | "app_chat_source_changed"),
        )) = sent
        {
            record["phase"] = json!("rejected");
            record["cause"] = json!(cause);
            let _ = self
                .bridge
                .operation(json!({"op":"release","lane":lane}))
                .await;
        } else {
            record["cause"] = json!("app_delivery_unknown");
        }
        self.journal.write(&key, &record)?;
        Ok(public(&record))
    }
    pub async fn result(&self, args: Value, secrets: &[String]) -> Result<Value, Error> {
        match self.result_observed(args.clone(), secrets).await {
            Err(error)
                if matches!(
                    error.0,
                    "app_chat_temporarily_unavailable"
                        | "app_chat_rate_limited"
                        | "app_chat_read_pending"
                        | "app_chat_request_failed"
                        | "app_bridge_operation_failed"
                ) =>
            {
                // Every GET, including source validation and pagination, observes
                // the original input; a retrieval error must not cancel it.
                let key = string(&args, "input_id", 64)?;
                let record = self
                    .journal
                    .read::<Value>(key)?
                    .ok_or(Error("chat_input_unavailable"))?;
                let mut result = self.pending_observation(&record).await;
                result["complete"] = json!(false);
                let fallback = if error.0 == "app_chat_rate_limited" {
                    60
                } else {
                    2
                };
                let retry = result["observation"]["read_retry_after_seconds"]
                    .as_u64()
                    .unwrap_or(fallback)
                    .max(fallback);
                result["observation_unavailable"] = json!({"error":error.0,"inference_submitted":false,"retry_after_seconds":retry});
                Ok(result)
            }
            result => result,
        }
    }
    async fn result_observed(&self, args: Value, secrets: &[String]) -> Result<Value, Error> {
        let key = string(&args, "input_id", 64)?;
        if key.len() != 64 || !key.bytes().all(|v| v.is_ascii_hexdigit()) {
            return Err(Error("chat_input_invalid"));
        }
        let mut record = self
            .journal
            .read::<Value>(key)?
            .ok_or(Error("chat_input_unavailable"))?;
        if record["schema"] != "termux-codex-saved-chat-input-v1" {
            return Err(Error("chat_input_invalid"));
        }
        let seconds = args
            .get("max_wait_seconds")
            .map(|v| {
                v.as_u64()
                    .filter(|v| *v <= 300)
                    .ok_or(Error("operator_wait_invalid"))
            })
            .transpose()?
            .unwrap_or(0);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
        loop {
            if self.identity().await? != record["identity"] {
                return Err(Error("app_account_changed"));
            }
            if record["phase"] == "rejected" {
                return Ok(
                    json!({"input":public(&record),"complete":true,"inference_submitted":false}),
                );
            }
            let pending = self.pending_observation(&record).await;
            let observation = &pending["observation"];
            if observation["pending"] == true
                || observation["stream_active"] == true
                || observation["host_confirmation_required"] == true
            {
                if !observation["error"].is_null()
                    || observation["host_confirmation_required"] == true
                    || tokio::time::Instant::now() >= deadline
                {
                    return Ok(pending);
                }
                tokio::time::sleep_until(
                    deadline.min(tokio::time::Instant::now() + Duration::from_secs(2)),
                )
                .await;
                continue;
            }
            let raw=self.bridge.operation(json!({"op":"chat_read","identity":record["identity"],"conversation_id":record["conversation_id"]})).await?;
            let graph = raw["mapping"]
                .as_object()
                .ok_or(Error("chat_mapping_unavailable"))?;
            let message = record["message_id"]
                .as_str()
                .ok_or(Error("chat_input_invalid"))?;
            let delivery_cursor = record
                .get("delivery_source_cursor")
                .unwrap_or(&record["source_cursor"]);
            let parent = delivery_cursor["branch"]
                .as_str()
                .ok_or(Error("chat_input_invalid"))?;
            // The exact delivery parent was bound before inference. Old records
            // keep their original strict parent; never guess a missing binding.
            if let Some(node) = graph.get(message) {
                if node["parent"] != parent || node["message"]["role"] != "user" {
                    return Err(Error("chat_input_source_changed"));
                }
                let mut frontier = vec![message.to_owned()];
                let mut leaves = Vec::new();
                let mut visited = std::collections::BTreeSet::new();
                while let Some(id) = frontier.pop() {
                    if !visited.insert(id.clone()) || visited.len() > 100_000 {
                        return Err(Error("chat_graph_invalid"));
                    }
                    let children: Vec<_> = graph
                        .iter()
                        .filter(|(_, v)| v["parent"].as_str() == Some(&id))
                        .map(|(k, _)| k.clone())
                        .collect();
                    if children.is_empty() {
                        leaves.push(id);
                    } else {
                        frontier.extend(children);
                    }
                }
                if leaves.len() != 1 {
                    return Err(Error("chat_input_branch_ambiguous"));
                }
                let leaf = &leaves[0];
                if let Some(completed_message) = public_completion(graph, leaf) {
                    // The delivery prefix was bound from the verified original
                    // source before submission; the descendant is separate.
                    let old=Chats::new(self.bridge.clone()).with_public_ids(crate::control::public_work_ids(&self.journal)?).call(json!({"action":"read","conversation_id":record["conversation_id"],"branch":parent,"after":delivery_cursor,"limit":1}),secrets).await?;
                    if old["complete"] != true
                        || old["messages"].as_array().is_none_or(|v| !v.is_empty())
                    {
                        return Err(Error("chat_input_source_changed"));
                    }
                    let mut result=Chats::new(self.bridge.clone()).with_public_ids(crate::control::public_work_ids(&self.journal)?).call(json!({"action":"read","conversation_id":record["conversation_id"],"branch":leaf,"limit":200}),secrets).await?;
                    // Read all pages so a large old source never truncates the
                    // result suffix or falsely certifies complete delivery.
                    let mut messages = result["messages"]
                        .as_array()
                        .cloned()
                        .ok_or(Error("app_chat_response_invalid"))?;
                    while result["complete"] != true {
                        result=Chats::new(self.bridge.clone()).with_public_ids(crate::control::public_work_ids(&self.journal)?).call(json!({"action":"read","conversation_id":record["conversation_id"],"branch":leaf,"after":result["cursor"],"limit":200}),secrets).await?;
                        messages.extend(
                            result["messages"]
                                .as_array()
                                .cloned()
                                .ok_or(Error("app_chat_response_invalid"))?,
                        );
                    }
                    let start = messages
                        .iter()
                        .position(|v| v["id"] == message)
                        .ok_or(Error("chat_input_source_changed"))?;
                    let source_binding = json!({"requested_parent":record["source_cursor"]["branch"],"saved_parent":node["parent"],"resolution":if node["parent"]==record["source_cursor"]["branch"] {"exact"} else {"hidden_terminal_to_public_final"},"prefix_verified":true,"delivery_source_cursor":delivery_cursor});
                    record["completed_message_id"] = json!(completed_message);
                    record["terminal_message_id"] = json!(leaf);
                    record["source_binding"] = source_binding;
                    record["phase"] = json!("completed");
                    record["completed_at_unix_ms"] = json!(unix_ms());
                    self.journal.write(key, &record)?;
                    // Release only our settled lane. A restarted app may no
                    // longer have it; saved-source evidence remains authority.
                    let released = self
                        .bridge
                        .operation(json!({"op":"release","lane":record["lane"]}))
                        .await;
                    return Ok(
                        json!({"input":public(&record),"messages":messages[start..],"cursor":result["cursor"],"complete":true,"completed_message_id":record["completed_message_id"],"terminal_message_id":record["terminal_message_id"],"source_binding":record["source_binding"],"lane_released":released.is_ok(),"inference_submitted":false}),
                    );
                }
            }
            let result = self.pending_observation(&record).await;
            if tokio::time::Instant::now() >= deadline {
                return Ok(result);
            }
            tokio::time::sleep_until(
                deadline.min(tokio::time::Instant::now() + Duration::from_secs(2)),
            )
            .await;
        }
    }
    async fn pending_observation(&self, record: &Value) -> Value {
        let mut result = public(record);
        result["complete"] = json!(false);
        if let Ok(snapshot) = self
            .bridge
            .operation(json!({"op":"snapshot","lane":record["lane"]}))
            .await
        {
            result["observation"] = json!({"pending":snapshot["pending"],"stream_active":snapshot["streamActive"],"error":snapshot["error"],"host_confirmation_required":snapshot["approvalPending"],"read_retry_after_seconds":snapshot["savedReadBackoff"]["retry_after_seconds"]});
        }
        result
    }
}
fn public_completion(graph: &serde_json::Map<String, Value>, leaf: &str) -> Option<String> {
    let mut current = leaf;
    for _ in 0..=graph.len() {
        let node = graph.get(current)?;
        let output = &node["message"];
        if output["role"] != "assistant"
            || output["status"] != "finished_successfully"
            || output["end_turn"] != true
            || !matches!(output["channel"].as_str(), None | Some("final"))
        {
            return None;
        }
        if output["unavailable"].is_null() {
            return matches!(
                output["content"]["content_type"].as_str(),
                Some("text" | "code")
            )
            .then(|| current.to_owned());
        }
        // Metadata only: hidden analysis and unknown/unsupported suffixes do
        // not authorize accepting an earlier answer as this turn's completion.
        if output["unavailable"] != "hidden" || output["channel"] != "final" {
            return None;
        }
        current = node["parent"].as_str()?;
    }
    None
}

fn public(record: &Value) -> Value {
    let mut result = json!({"input_id":record["input_id"],"conversation_id":record["conversation_id"],"message_id":record["message_id"],"phase":record["phase"],"cause":record["cause"],"delivery":"observe_saved_original_message_without_resend","retrieve":"codex_sessions chat_result"});
    if let Some(settings) = record.get("settings") {
        result["settings"] = settings.clone();
    }
    result
}
fn resolve_selection(
    catalog: &crate::models::Catalog,
    selected: &Value,
    args: &Value,
) -> Result<Value, Error> {
    let slug = selected["slug"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or(Error("app_model_selection_unavailable"))?;
    let thinking = match &selected["thinkingEffort"] {
        Value::Null => None,
        Value::String(v) if !v.is_empty() => Some(v.as_str()),
        _ => return Err(Error("app_model_selection_unavailable")),
    };
    let host_models: Vec<_> = catalog
        .models
        .iter()
        .flat_map(|m| {
            m.routes
                .iter()
                .filter(move |r| r.host_model == slug)
                .map(move |r| (m, r))
        })
        .collect();
    let matching: Vec<_> = host_models
        .iter()
        .filter(|(_, r)| r.thinking_effort.as_deref() == thinking)
        .copied()
        .collect();
    let inherited_effort = matching.first().and_then(|(_, first)| {
        matching
            .iter()
            .all(|(_, r)| r.effort == first.effort)
            .then_some(first.effort.as_str())
    });
    let model = args
        .get("model")
        .map(|_| string(args, "model", 128))
        .transpose()?;
    let requested_effort = args
        .get("effort")
        .map(|_| string(args, "effort", 128))
        .transpose()?;
    if model.is_none() && requested_effort.is_none() {
        let canonical = matching.first().and_then(|(m, _)| {
            matching
                .iter()
                .all(|(other, _)| other.id == m.id)
                .then_some(m.id.as_str())
        });
        return Ok(
            json!({"model":canonical,"effort":inherited_effort,"host_model":slug,"thinking_effort":thinking,"model_source":"chat","effort_source":"chat","chat_selection":selected}),
        );
    }
    if model.is_none() && host_models.is_empty() {
        return Err(Error("chat_input_model_unavailable"));
    }
    let effort = requested_effort
        .or(inherited_effort)
        .ok_or(Error("chat_input_effort_unavailable"))?;
    let chosen = if let Some(model) = model {
        catalog.find(model).ok_or(Error("unknown_model"))?
    } else {
        let mut candidates = host_models
            .iter()
            .filter(|(m, _)| !m.hidden)
            .map(|(m, _)| *m);
        let first = candidates
            .next()
            .or_else(|| host_models.first().map(|(m, _)| *m))
            .ok_or(Error("unknown_model"))?;
        if candidates.any(|m| m.id != first.id) {
            return Err(Error("chat_input_model_ambiguous"));
        }
        first
    };
    let route = chosen
        .routes
        .iter()
        .find(|r| r.effort == effort)
        .ok_or(Error("chat_input_effort_incompatible"))?;
    Ok(
        json!({"model":chosen.id,"effort":route.effort,"host_model":route.host_model,"thinking_effort":route.thinking_effort,"model_source":if model.is_some(){"explicit"}else{"chat"},"effort_source":if args.get("effort").is_some(){"explicit"}else{"chat"},"chat_selection":selected}),
    )
}
fn string<'a>(value: &'a Value, key: &str, max: usize) -> Result<&'a str, Error> {
    value[key]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= max)
        .ok_or(Error("chat_input_arguments_invalid"))
}
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use std::sync::Mutex;
    struct Source {
        state: Mutex<Value>,
    }
    impl Bridge for Source {
        fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
            let mut state = self.state.lock().unwrap();
            let result = match command["op"].as_str().unwrap() {
                "identity" => Ok(
                    json!({"account":state.get("account").and_then(Value::as_str).unwrap_or("a"),"user":"u","structure":"personal"}),
                ),
                "chat_read" => {
                    let count = state["read_count"].as_u64().unwrap_or(0) + 1;
                    state["read_count"] = json!(count);
                    let cause = if state["read_fail_at"].is_number()
                        && state["read_fail_at"].as_u64() != Some(count)
                    {
                        None
                    } else {
                        state["read_error"].as_str()
                    };
                    match cause {
                        Some("transport") => Err(Error("app_chat_request_failed")),
                        Some("legacy") => Err(Error("app_bridge_operation_failed")),
                        Some("busy") => Err(Error("app_chat_temporarily_unavailable")),
                        Some("rate") => Err(Error("app_chat_rate_limited")),
                        Some("read_pending") => Err(Error("app_chat_read_pending")),
                        Some("account") => Err(Error("app_account_changed")),
                        Some("forbidden") => Err(Error("app_chat_forbidden")),
                        Some("invalid") => Err(Error("app_chat_response_invalid")),
                        _ => Ok(state["snapshot"].clone()),
                    }
                }
                "chat_adopt" => {
                    state["adoptions"] = json!(state["adoptions"].as_u64().unwrap() + 1);
                    Ok(
                        json!({"localId":"chat","selection":{"slug":"gpt-6-thinking","thinkingEffort":"extended"}}),
                    )
                }
                "submit" => {
                    if state["selection_changed"] == true {
                        return Box::pin(async { Err(Error("app_model_selection_changed")) });
                    }
                    state["submitted_command"] = command.clone();
                    state["submissions"] = json!(state["submissions"].as_u64().unwrap() + 1);
                    let id = command["messageId"].as_str().unwrap().to_owned();
                    state["snapshot"]["mapping"][&id] = json!({"id":id,"parent":"old","message":{"id":id,"role":"user","content":{"content_type":"text","parts":["task"]},"status":"finished_successfully"}});
                    state["snapshot"]["mapping"]["result"] = json!({"id":"result","parent":id,"message":{"id":"result","role":"assistant","channel":"final","content":{"content_type":"text","parts":["answer", "controller=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]},"status":"finished_successfully","end_turn":true}});
                    state["snapshot"]["current_node"] = json!("result");
                    Err(Error("fixture_lost_submit_reply"))
                }
                "release" => Ok(json!({"released":true})),
                "snapshot" => Ok(state
                    .get("lane_snapshot")
                    .cloned()
                    .unwrap_or_else(|| json!({"pending":false,"streamActive":false}))),
                _ => panic!("unexpected operation {}", command["op"]),
            };
            Box::pin(async move { result })
        }
    }
    fn fixture() -> Arc<Source> {
        Arc::new(Source {
            state: Mutex::new(
                json!({"adoptions":0,"submissions":0,"snapshot":{"conversation_id":"chat","current_node":"old","mapping":{"old":{"id":"old","parent":null,"message":{"id":"old","role":"assistant","channel":"final","content":{"content_type":"text","parts":["old answer"]},"status":"finished_successfully","end_turn":true}}}}}),
            ),
        })
    }
    struct Root(std::path::PathBuf);
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[tokio::test]
    async fn saved_chat_uses_prepared_selection_and_keeps_exact_input_on_retry() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let task = "  exact input\r\nsecond line  ";
        let args = json!({"action":"chat_input","call_id":"settings","conversation_id":"chat","task":task,"source_cursor":cursor,"budget":{"max_app_starts":1,"max_elapsed_seconds":60}});
        let admitted = input.submit(args.clone(), &[]).await.unwrap();
        assert_eq!(admitted["settings"]["effort"], "high");
        assert_eq!(admitted["settings"]["thinking_effort"], "extended");
        {
            let state = source.state.lock().unwrap();
            assert_eq!(state["submitted_command"]["model"], "gpt-6-thinking");
            assert_eq!(state["submitted_command"]["thinkingEffort"], "extended");
            assert_eq!(state["submitted_command"]["messages"][0]["text"], task);
        }
        assert_eq!(input.submit(args, &[]).await.unwrap(), admitted);
        assert_eq!(source.state.lock().unwrap()["submissions"], 1);
    }
    #[test]
    fn selection_validates_explicit_options_and_inherited_effort() {
        let catalog = crate::models::Catalog::default();
        let discovered = json!({"slug":"app-new-model","thinkingEffort":"new-effort"});
        let retained = resolve_selection(&catalog, &discovered, &json!({})).unwrap();
        assert_eq!(retained["host_model"], "app-new-model");
        assert_eq!(retained["thinking_effort"], "new-effort");
        assert!(retained["model"].is_null() && retained["effort"].is_null());
        let app_pair = json!({"slug":"gpt-6","thinkingEffort":"extended"});
        let retained = resolve_selection(&catalog, &app_pair, &json!({})).unwrap();
        assert_eq!(retained["host_model"], "gpt-6");
        assert_eq!(retained["thinking_effort"], "extended");
        assert!(retained["effort"].is_null());
        assert_eq!(
            resolve_selection(
                &catalog,
                &app_pair,
                &json!({"model":"chatgpt-desktop/gpt-6"})
            ),
            Err(Error("chat_input_effort_unavailable"))
        );
        let selected = json!({"slug":"gpt-6-thinking","thinkingEffort":"extended"});
        let explicit = resolve_selection(&catalog, &selected, &json!({"effort":"xhigh"})).unwrap();
        assert_eq!(explicit["thinking_effort"], "max");
        assert_eq!(explicit["effort_source"], "explicit");
        assert_eq!(
            resolve_selection(
                &catalog,
                &selected,
                &json!({"model":"chatgpt-desktop/instant"})
            ),
            Err(Error("chat_input_effort_incompatible"))
        );
        let pro = resolve_selection(
            &catalog,
            &selected,
            &json!({"model":"chatgpt-desktop/gpt-6-pro","effort":"none"}),
        )
        .unwrap();
        assert_eq!(pro["host_model"], "gpt-6-pro");
    }
    #[tokio::test]
    async fn known_selection_change_finishes_without_dispatch_or_replay() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        source.state.lock().unwrap()["selection_changed"] = json!(true);
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let args = json!({"action":"chat_input","call_id":"changed-selection","conversation_id":"chat","task":"task","source_cursor":cursor,"budget":{"max_app_starts":1,"max_elapsed_seconds":60}});
        let rejected = input.submit(args.clone(), &[]).await.unwrap();
        assert_eq!(rejected["phase"], "rejected");
        assert_eq!(rejected["cause"], "app_model_selection_changed");
        assert_eq!(input.submit(args, &[]).await.unwrap(), rejected);
        let result = input
            .result(json!({"input_id":rejected["input_id"]}), &[])
            .await
            .unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(result["inference_submitted"], false);
        assert_eq!(source.state.lock().unwrap()["submissions"], 0);
        source.state.lock().unwrap()["account"] = json!("another-account");
        assert_eq!(
            input
                .result(json!({"input_id":rejected["input_id"]}), &[])
                .await,
            Err(Error("app_account_changed"))
        );
    }
    #[tokio::test]
    async fn lost_reply_and_restart_reconcile_saved_original_without_another_input() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        let args = json!({"action":"chat_input","call_id":"stable","conversation_id":"chat","task":"task","source_cursor":cursor,"budget":{"max_app_starts":1,"max_elapsed_seconds":60}});
        let id;
        {
            let input = Inputs {
                bridge: source.clone(),
                journal: Arc::new(Journal::open(&root.0).unwrap()),
                config: Config::default(),
            };
            let first = input.submit(args.clone(), &[]).await.unwrap();
            assert_eq!(first["phase"], "delivery_unknown");
            assert_eq!(input.submit(args.clone(), &[]).await.unwrap(), first);
            let mut changed = args.clone();
            changed["task"] = json!("changed");
            assert_eq!(
                input.submit(changed, &[]).await,
                Err(Error("operator_call_changed"))
            );
            id = first["input_id"].clone();
        }
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        assert_eq!(
            input.submit(args, &[]).await.unwrap()["phase"],
            "delivery_unknown"
        );
        let result = input.result(json!({"input_id":id}), &[]).await.unwrap();
        assert_eq!(result["input"]["phase"], "completed");
        assert_eq!(result["messages"].as_array().unwrap().len(), 2);
        assert!(!result.to_string().contains(&"b".repeat(48)));
        assert_eq!(
            input.result(json!({"input_id":id}), &[]).await.unwrap()["messages"],
            result["messages"]
        );
        let state = source.state.lock().unwrap();
        assert_eq!(state["submissions"], 1);
        assert_eq!(state["adoptions"], 1);
    }
    #[tokio::test]
    async fn unavailable_saved_read_preserves_admitted_input_until_fresh_completion() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let admitted = input.submit(json!({"call_id":"read-gap","conversation_id":"chat","task":"task","source_cursor":cursor,"budget":{"max_app_starts":1,"max_elapsed_seconds":60}}), &[]).await.unwrap();
        let args = json!({"input_id":admitted["input_id"]});
        for cause in ["transport", "legacy", "busy", "rate", "read_pending"] {
            source.state.lock().unwrap()["read_error"] = json!(cause);
            let pending = input.result(args.clone(), &[]).await.unwrap();
            assert_eq!(pending["complete"], false);
            assert_eq!(pending["phase"], admitted["phase"]);
            assert_eq!(pending["message_id"], admitted["message_id"]);
            assert!(pending["observation_unavailable"]["error"].is_string());
            assert_eq!(pending["observation"]["stream_active"], false);
            assert!(pending.get("messages").is_none());
        }
        for read in [2, 3] {
            {
                let mut state = source.state.lock().unwrap();
                state["read_error"] = json!("transport");
                state["read_count"] = json!(0);
                state["read_fail_at"] = json!(read);
            }
            let pending = input.result(args.clone(), &[]).await.unwrap();
            assert_eq!(pending["complete"], false);
            assert_eq!(pending["message_id"], admitted["message_id"]);
            assert!(pending["observation_unavailable"].is_object());
        }
        source.state.lock().unwrap()["read_fail_at"] = Value::Null;
        for (cause, code) in [
            ("account", "app_account_changed"),
            ("forbidden", "app_chat_forbidden"),
            ("invalid", "app_chat_response_invalid"),
        ] {
            source.state.lock().unwrap()["read_error"] = json!(cause);
            assert_eq!(input.result(args.clone(), &[]).await, Err(Error(code)));
        }
        source.state.lock().unwrap()["read_error"] = Value::Null;
        let completed = input.result(args, &[]).await.unwrap();
        assert_eq!(completed["complete"], true);
        assert_eq!(completed["input"]["message_id"], admitted["message_id"]);
        let state = source.state.lock().unwrap();
        assert_eq!(state["submissions"], 1);
        assert_eq!(state["adoptions"], 1);
    }

    #[tokio::test]
    async fn active_owned_stream_uses_local_progress_without_saved_get_or_false_completion() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let admitted=input.submit(json!({"call_id":"active","conversation_id":"chat","task":"task","source_cursor":cursor,"budget":{"max_app_starts":1,"max_elapsed_seconds":60}}),&[]).await.unwrap();
        let args = json!({"input_id":admitted["input_id"],"max_wait_seconds":0});
        let reads = source.state.lock().unwrap()["read_count"].clone();
        for snapshot in [
            json!({"pending":true,"streamActive":false}),
            json!({"pending":false,"streamActive":true}),
            json!({"pending":false,"streamActive":true,"error":"mcp_host_confirmation_required","approvalPending":true}),
        ] {
            source.state.lock().unwrap()["lane_snapshot"] = snapshot;
            let pending = input.result(args.clone(), &[]).await.unwrap();
            assert_eq!(pending["complete"], false);
            assert_eq!(source.state.lock().unwrap()["read_count"], reads);
        }
        source.state.lock().unwrap()["lane_snapshot"] =
            json!({"pending":false,"streamActive":false});
        let result = input.result(args, &[]).await.unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(result["input"]["message_id"], admitted["message_id"]);
        assert_eq!(source.state.lock().unwrap()["submissions"], 1);
    }

    #[test]
    fn hidden_final_terminal_retains_public_result_without_reading_hidden_content() {
        let mut graph = json!({
            "answer":{"parent":"input","message":{"role":"assistant","channel":"final",
                "end_turn":true,"status":"finished_successfully","content":{"content_type":"text","parts":["public answer"]},"unavailable":null}},
            "terminal":{"parent":"answer","message":{"role":"assistant","channel":"final",
                "end_turn":true,"status":"finished_successfully","content":null,"unavailable":"hidden"}}
        });
        assert_eq!(
            public_completion(graph.as_object().unwrap(), "terminal"),
            Some("answer".into())
        );
        for (field, value) in [
            ("role", json!("tool")),
            ("channel", json!("analysis")),
            ("channel", Value::Null),
            ("status", json!("in_progress")),
            ("end_turn", json!(false)),
            ("unavailable", json!("unsupported_content")),
        ] {
            let mut invalid = graph.clone();
            invalid["terminal"]["message"][field] = value;
            assert_eq!(
                public_completion(invalid.as_object().unwrap(), "terminal"),
                None
            );
        }
        graph["answer"]["message"]["channel"] = json!("commentary");
        graph["answer"]["message"]["end_turn"] = json!(false);
        assert_eq!(
            public_completion(graph.as_object().unwrap(), "terminal"),
            None
        );
    }

    #[tokio::test]
    async fn saved_result_preserves_hidden_terminal_and_identifies_public_final() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let admitted = input
            .submit(
                json!({"call_id":"terminal","conversation_id":"chat","task":"task",
            "source_cursor":cursor,"budget":{"max_app_starts":1,"max_elapsed_seconds":60}}),
                &[],
            )
            .await
            .unwrap();
        {
            let mut state = source.state.lock().unwrap();
            state["snapshot"]["mapping"]["terminal"] = json!({"id":"terminal","parent":"result","message":{
                "id":"terminal","role":"assistant","channel":"final","end_turn":true,
                "status":"finished_successfully","content":null,"unavailable":"hidden"}});
            state["snapshot"]["current_node"] = json!("terminal");
        }
        let result = input
            .result(json!({"input_id":admitted["input_id"]}), &[])
            .await
            .unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(result["completed_message_id"], "result");
        assert_eq!(result["terminal_message_id"], "terminal");
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages.last().unwrap()["unavailable"], "hidden");
        assert!(messages.last().unwrap()["content"].is_null());
        assert_eq!(source.state.lock().unwrap()["submissions"], 1);
    }

    #[tokio::test]
    async fn saved_input_reconciles_hidden_source_parent_without_replay_or_cursor_rewrite() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        {
            let mut state = source.state.lock().unwrap();
            state["snapshot"]["mapping"]["hidden"] = json!({"id":"hidden","parent":"old","message":{
                "id":"hidden","role":"assistant","channel":"final","end_turn":true,
                "status":"finished_successfully","content":null,"unavailable":"hidden"}});
            state["snapshot"]["current_node"] = json!("hidden");
        }
        let cursor = Chats::new(source.clone())
            .call(json!({"action":"read","conversation_id":"chat"}), &[])
            .await
            .unwrap()["cursor"]
            .clone();
        assert_eq!(cursor["branch"], "hidden");
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let admitted = input.submit(json!({"call_id":"hidden-source","conversation_id":"chat","task":"task","source_cursor":cursor,
            "budget":{"max_app_starts":1,"max_elapsed_seconds":60}}), &[]).await.unwrap();
        source.state.lock().unwrap()["snapshot"]["mapping"]
            .as_object_mut()
            .unwrap()
            .remove("hidden");
        let result = input
            .result(json!({"input_id":admitted["input_id"]}), &[])
            .await
            .unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(
            result["source_binding"]["resolution"],
            json!("hidden_terminal_to_public_final")
        );
        assert_eq!(result["completed_message_id"], "result");
        let record = input
            .journal
            .read::<Value>(admitted["input_id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(record["source_cursor"], cursor);
        assert_eq!(record["delivery_source_cursor"]["branch"], "old");
        assert_eq!(result["source_binding"]["saved_parent"], "old");
        assert_eq!(source.state.lock().unwrap()["submissions"], 1);
        // A later mutation of either source content still refuses reconciliation.
        source.state.lock().unwrap()["snapshot"]["mapping"]["old"]["message"]["content"]["parts"] =
            json!(["changed original"]);
        assert_eq!(
            input
                .result(json!({"input_id":admitted["input_id"]}), &[])
                .await,
            Err(Error("chat_source_prefix_changed"))
        );
        assert_eq!(source.state.lock().unwrap()["submissions"], 1);
    }

    #[tokio::test]
    async fn incomplete_source_is_rejected_before_adoption_or_submission() {
        let root =
            Root(std::env::temp_dir().join(format!("fi-{}", crate::journal::random_id().unwrap())));
        let source = fixture();
        let cursor = json!({"account":digest(&json!({"account":"a","user":"u","structure":"personal"})),"conversation":"chat","branch":"old","count":0,"prefix":digest(&Vec::<Value>::new())});
        let input = Inputs {
            bridge: source.clone(),
            journal: Arc::new(Journal::open(&root.0).unwrap()),
            config: Config::default(),
        };
        let result=input.submit(json!({"call_id":"incomplete","conversation_id":"chat","task":"task","source_cursor":cursor}),&[]).await;
        assert_eq!(result, Err(Error("chat_input_source_incomplete")));
        let state = source.state.lock().unwrap();
        assert_eq!(state["submissions"], 0);
        assert_eq!(state["adoptions"], 0);
    }
}
