//! Model-free saved-source retrieval. Reading never grants a work controller.
use crate::{app::Bridge, context::digest, protocol::Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
pub struct Chats {
    bridge: Arc<dyn Bridge>,
    public_ids: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    account: String,
    conversation: String,
    branch: String,
    count: usize,
    prefix: String,
}
/// Stable saved-source account identity; host generation is deliberately absent.
pub(crate) fn account_reference(identity: &Value) -> String {
    digest(
        &json!({"account":identity["account"],"user":identity["user"],"structure":identity["structure"]}),
    )
}
impl Chats {
    pub fn new(bridge: Arc<dyn Bridge>) -> Self {
        Self {
            bridge,
            public_ids: Vec::new(),
        }
    }
    pub(crate) fn with_public_ids(mut self, ids: Vec<String>) -> Self {
        self.public_ids = ids;
        self
    }
    pub async fn call(&self, args: Value, secrets: &[String]) -> Result<Value, Error> {
        let mut result = self.read_source(args).await?;
        let mut redactions = 0;
        redact_public(&mut result, secrets, &self.public_ids, &mut redactions);
        result["redactions"] =
            json!({"count":redactions,"policy":"credentials_and_private_capabilities"});
        Ok(result)
    }
    /// Internal source consumers preserve the exact authorized user text. The
    /// public tools still redact credentials and private capabilities in call().
    pub(crate) async fn read_source(&self, args: Value) -> Result<Value, Error> {
        self.read(args, false).await
    }
    /// One bounded selected-branch snapshot for internal source observation.
    /// Public delivery pagination must not turn into repeated upstream reads.
    pub(crate) async fn read_complete_source(&self, conversation: &str) -> Result<Value, Error> {
        self.read(
            json!({"action":"read","conversation_id":conversation}),
            true,
        )
        .await
    }
    async fn read(&self, args: Value, complete_source: bool) -> Result<Value, Error> {
        let identity = self.bridge.operation(json!({"op":"identity"})).await?;
        let identity = json!({"account":identity["account"],"user":identity["user"],"structure":identity["structure"]});
        if !identity["account"].is_string() || !identity["user"].is_string() {
            return Err(Error("app_identity_unavailable"));
        }
        let account = account_reference(&identity);
        let limit = args
            .get("limit")
            .map(|v| {
                v.as_u64()
                    .filter(|v| (1..=200).contains(v))
                    .ok_or(Error("chat_limit_invalid"))
            })
            .transpose()?
            .unwrap_or(50) as usize;
        let mut result = match args["action"].as_str() {
            Some("list") => {
                let filters = json!({"project_id":args["project_id"],"title":args["title"],"updated_after":args["updated_after"],"updated_before":args["updated_before"]});
                if ["project_id", "title"]
                    .iter()
                    .any(|key| args.get(key).is_some_and(|v| !v.is_string()))
                    || ["updated_after", "updated_before"]
                        .iter()
                        .any(|key| args.get(key).is_some_and(|v| v.as_u64().is_none()))
                {
                    return Err(Error("chat_filter_invalid"));
                }
                if args["updated_after"]
                    .as_u64()
                    .zip(args["updated_before"].as_u64())
                    .is_some_and(|(a, b)| a > b)
                {
                    return Err(Error("chat_filter_invalid"));
                }
                let mut offset = args
                    .get("offset")
                    .map(|v| v.as_u64().ok_or(Error("chat_cursor_invalid")))
                    .transpose()?
                    .unwrap_or(0);
                if let Some(cursor) = args.get("list_cursor") {
                    if args.get("offset").is_some()
                        || cursor["account"] != account
                        || cursor["filter"] != digest(&filters)
                    {
                        return Err(Error("chat_cursor_invalid"));
                    }
                    offset = cursor["offset"]
                        .as_u64()
                        .ok_or(Error("chat_cursor_invalid"))?;
                }
                if offset > 100_000 {
                    return Err(Error("chat_cursor_invalid"));
                }
                if args.get("account").is_some_and(|v| v != &json!(account)) {
                    return Err(Error("chat_account_changed"));
                }
                let filtered = filters.as_object().unwrap().values().any(|v| !v.is_null());
                let mut found = Vec::new();
                let mut pages = 0;
                let total = loop {
                    if pages >= 256 || offset > 100_000 {
                        return Err(Error("chat_pagination_incomplete"));
                    }
                    // Request only the remaining item count. Offset always
                    // advances over the exact inspected source range.
                    let page=self.bridge.operation(json!({"op":"chat_list","identity":identity,"offset":offset,"limit":limit-found.len()})).await?;
                    let items = page["items"]
                        .as_array()
                        .ok_or(Error("app_chat_response_invalid"))?;
                    let total = page["total"]
                        .as_u64()
                        .ok_or(Error("app_chat_response_invalid"))?;
                    if items.len() > limit - found.len() || (items.is_empty() && offset < total) {
                        return Err(Error("chat_pagination_incomplete"));
                    }
                    pages += 1;
                    for item in items {
                        let matched = (!args
                            .get("project_id")
                            .is_some_and(|v| v != &item["project_id"]))
                            && (!args
                                .get("title")
                                .and_then(Value::as_str)
                                .is_some_and(|title| {
                                    !item["title"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_lowercase()
                                        .contains(&title.to_lowercase())
                                }))
                            && (!args["updated_after"].as_u64().is_some_and(|t| {
                                item["updated_at"].as_f64().is_none_or(|v| v < (t as f64))
                            }))
                            && (!args["updated_before"].as_u64().is_some_and(|t| {
                                item["updated_at"].as_f64().is_none_or(|v| v > (t as f64))
                            }));
                        if matched {
                            found.push(item.clone());
                        }
                        offset += 1;
                    }
                    if found.len() == limit || offset >= total {
                        break total;
                    }
                };
                json!({"account":account,"items":found,"next_offset":if offset<total{Some(offset)}else{None},"list_cursor":if offset<total{json!({"account":account,"offset":offset,"filter":digest(&filters)})}else{Value::Null},"complete":offset>=total,"total":if filtered{Value::Null}else{json!(total)},"source_total":total,"source_pages":pages,"filters":filters})
            }
            Some("read") => {
                let conversation = identifier(&args, "conversation_id")?;
                let snapshot = self.bridge.operation(json!({"op":"chat_read","identity":identity,"conversation_id":conversation})).await?;
                if snapshot["conversation_id"] != conversation {
                    return Err(Error("chat_source_changed"));
                }
                let mut graph: BTreeMap<String, Value> = snapshot["mapping"]
                    .as_object()
                    .ok_or(Error("chat_mapping_unavailable"))?
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let branch = args
                    .get("branch")
                    .and_then(Value::as_str)
                    .or(snapshot["current_node"].as_str())
                    .ok_or(Error("chat_branch_unavailable"))?
                    .to_owned();
                let mut pages = 1;
                let mut cursor = snapshot["pagination"]["cursor"].as_str().map(str::to_owned);
                let mut seen = BTreeSet::new();
                while let Some(before) = cursor.take() {
                    if pages >= 256 || !seen.insert(before.clone()) {
                        return Err(Error("chat_pagination_incomplete"));
                    }
                    let page = self.bridge.operation(json!({"op":"chat_page","identity":identity,"conversation_id":conversation,"before":before})).await?;
                    let messages = page["messages"]
                        .as_array()
                        .ok_or(Error("app_chat_response_invalid"))?;
                    let mut parent: Option<String> = None;
                    for message in messages {
                        let id = message["id"]
                            .as_str()
                            .ok_or(Error("app_chat_response_invalid"))?
                            .to_owned();
                        if let Some(old) = graph.get(&id) {
                            if old["message"] != *message {
                                return Err(Error("chat_source_changed"));
                            }
                        } else {
                            graph.insert(
                                id.clone(),
                                json!({"id":id,"parent":parent,"message":message}),
                            );
                        }
                        parent = Some(id);
                    }
                    if let Some(oldest) = snapshot["pagination"]["oldestMessageId"].as_str() {
                        if pages == 1 {
                            if let (Some(parent), Some(node)) =
                                (parent.as_ref(), graph.get_mut(oldest))
                            {
                                node["parent"] = json!(parent);
                            }
                        }
                    }
                    if page["page_info"]["has_previous_page"] == true {
                        cursor = Some(
                            page["page_info"]["start_cursor"]
                                .as_str()
                                .ok_or(Error("chat_pagination_incomplete"))?
                                .to_owned(),
                        );
                        // Connect the next older page to this page's first item.
                        if let Some(id) = messages.first().and_then(|m| m["id"].as_str()) {
                            graph.get_mut(id).unwrap()["parent"] = json!(format!("page:{before}"));
                            graph.insert(format!("page:{before}"), json!({"id":format!("page:{before}"),"parent":null,"message":null,"unresolved":true}));
                        }
                    }
                    // Stitch the preceding page's placeholder to this page's tail.
                    if pages > 1 {
                        let placeholders: Vec<String> = graph
                            .iter()
                            .filter(|(_, v)| v["unresolved"] == true)
                            .map(|(k, _)| k.clone())
                            .collect();
                        for key in placeholders {
                            if key != format!("page:{before}") {
                                let node = graph.get_mut(&key).unwrap();
                                node["parent"] = json!(parent);
                                node["unresolved"] = json!(false);
                            }
                        }
                    }
                    pages += 1;
                }
                let mut path = Vec::new();
                let mut id = Some(branch.clone());
                let mut visited = BTreeSet::new();
                while let Some(current) = id.take() {
                    if !visited.insert(current.clone()) || visited.len() > 100_000 {
                        return Err(Error("chat_graph_invalid"));
                    }
                    let node = graph
                        .get(&current)
                        .ok_or(Error("chat_pagination_incomplete"))?;
                    if node["unresolved"] == true {
                        return Err(Error("chat_pagination_incomplete"));
                    }
                    id = node["parent"].as_str().map(str::to_owned);
                    if !node["message"].is_null() {
                        let mut message = node["message"].clone();
                        message["parent"] = node["parent"].clone();
                        path.push(message);
                    }
                }
                path.reverse();
                let mut messages = Vec::new();
                for message in path {
                    let role = message["role"].as_str().unwrap_or("unknown");
                    let content = &message["content"];
                    let available = message["unavailable"].is_null()
                        && matches!(role, "user" | "assistant" | "tool")
                        && matches!(content["content_type"].as_str(), Some("text" | "code"));
                    messages.push(json!({"id":message["id"],"parent":message["parent"],"role":role,"name":message["name"],"model":message["model"],"end_turn":message["end_turn"],"channel":message["channel"],"recipient":message["recipient"],"status":message["status"],"created_at":message["created_at"],
                        "content":if available {content.clone()} else {Value::Null},"unavailable":if available {Value::Null} else {json!(message["unavailable"].as_str().unwrap_or("unsupported_content"))}}));
                }
                let after = args
                    .get("after")
                    .map(|value| {
                        serde_json::from_value::<Cursor>(value.clone())
                            .map_err(|_| Error("chat_cursor_invalid"))
                    })
                    .transpose()?;
                let start = if let Some(after) = after {
                    if after.account != account
                        || after.conversation != conversation
                        || !visited.contains(&after.branch)
                        || after.count > messages.len()
                        || after.prefix != digest(&&messages[..after.count])
                    {
                        return Err(Error("chat_source_prefix_changed"));
                    }
                    after.count
                } else {
                    0
                };
                if complete_source
                    && serde_json::to_vec(&messages).unwrap().len()
                        > crate::config::MAX_DECODED_BYTES as usize
                {
                    return Err(Error("bound_chat_history_read_bound"));
                }
                let end = if complete_source {
                    messages.len()
                } else {
                    (start + limit).min(messages.len())
                };
                let next = Cursor {
                    account: account.clone(),
                    conversation: conversation.into(),
                    branch: branch.clone(),
                    count: end,
                    prefix: digest(&&messages[..end]),
                };
                // A saved follow-up may cause the app to remove a hidden final
                // suffix. Bind delivery to the preceding public final before
                // submission, while retaining the caller's original cursor.
                let input_cursor = (end == messages.len())
                    .then(|| saved_input_cursor(&messages, &next))
                    .flatten();
                json!({"account":account,"conversation_id":conversation,"branch":branch,"title":snapshot["title"],"project_id":snapshot["project_id"],"messages":messages[start..end],"content_complete":messages[start..end].iter().all(|message|message["unavailable"].is_null()),
                    "cursor":next,"input_cursor":input_cursor,"complete":end==messages.len(),"total":messages.len(),"source_pages":pages,"branches":graph.keys().filter(|k|!graph.values().any(|v|v["parent"].as_str()==Some(k.as_str()))).collect::<Vec<_>>()})
            }
            _ => return Err(Error("chat_action_invalid")),
        };
        let current = self.bridge.operation(json!({"op":"identity"})).await?;
        if json!({"account":current["account"],"user":current["user"],"structure":current["structure"]})
            != identity
        {
            return Err(Error("app_account_changed"));
        }
        result["observed_at_unix_ms"] = json!(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());
        result["inference_submitted"] = json!(false);
        Ok(result)
    }
}
fn saved_input_cursor(messages: &[Value], source: &Cursor) -> Option<Cursor> {
    for (index, message) in messages.iter().enumerate().rev() {
        if message["role"] != "assistant"
            || message["status"] != "finished_successfully"
            || message["end_turn"] != true
            || !matches!(message["channel"].as_str(), None | Some("final"))
        {
            return None;
        }
        if message["unavailable"].is_null() {
            if !matches!(
                message["content"]["content_type"].as_str(),
                Some("text" | "code")
            ) {
                return None;
            }
            return Some(Cursor {
                account: source.account.clone(),
                conversation: source.conversation.clone(),
                branch: message["id"].as_str()?.into(),
                count: index + 1,
                prefix: digest(&&messages[..=index]),
            });
        }
        if message["unavailable"] != "hidden" || message["channel"] != "final" {
            return None;
        }
    }
    None
}
fn identifier<'a>(args: &'a Value, key: &str) -> Result<&'a str, Error> {
    args[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .ok_or(Error("chat_identity_invalid"))
}
#[cfg(test)]
fn redact(value: &mut Value, secrets: &[String], count: &mut usize) {
    redact_public(value, secrets, &[], count)
}
fn redact_public(value: &mut Value, secrets: &[String], public_ids: &[String], count: &mut usize) {
    match value {
        Value::Object(values) => {
            for (key, value) in values {
                if [
                    "controller",
                    "turntoken",
                    "accesstoken",
                    "refreshtoken",
                    "authorization",
                    "apikey",
                    "password",
                    "clientsecret",
                    "cookie",
                    "accept",
                ]
                .contains(&key.replace(['_', '-'], "").to_ascii_lowercase().as_str())
                {
                    *value = json!("[redacted]");
                    *count += 1;
                } else {
                    redact_public(value, secrets, public_ids, count);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_public(value, secrets, public_ids, count);
            }
        }
        Value::String(text) => {
            if let Ok(mut structured) = serde_json::from_str::<Value>(text) {
                if structured.is_object() || structured.is_array() {
                    let before = *count;
                    redact_public(&mut structured, secrets, public_ids, count);
                    if *count != before {
                        *text = structured.to_string();
                        return;
                    }
                }
            }
            for secret in secrets.iter().filter(|s| !s.is_empty()) {
                if text.contains(secret) {
                    *text = text.replace(secret, "[redacted]");
                    *count += 1;
                }
            }
            // Private bridge capabilities are random 48-hex values. Also redact
            // historical caps from unrelated saved tool output, not just live work.
            let mut output = String::new();
            let mut start = 0;
            for (index, ch) in text
                .char_indices()
                .chain(std::iter::once((text.len(), ' ')))
            {
                if ch.is_ascii_hexdigit() {
                    continue;
                }
                let run = &text[start..index];
                if run.len() == 48 && !public_ids.iter().any(|id| id == run) {
                    output.push_str("[redacted]");
                    *count += 1;
                } else {
                    output.push_str(run);
                }
                if index < text.len() {
                    output.push(ch);
                }
                start = index + ch.len_utf8();
            }
            *text = output;
            // Tokens embedded in ordinary tool text are not necessarily JSON.
            for prefix in ["Bearer ", "sk-", "eyJ"] {
                let mut offset = 0;
                while let Some(found) = text[offset..].find(prefix) {
                    let start = offset + found;
                    let token_start = if prefix == "Bearer " {
                        start + prefix.len()
                    } else {
                        start
                    };
                    let end = text[token_start..]
                        .char_indices()
                        .find(|(_, ch)| {
                            !ch.is_ascii_alphanumeric() && !matches!(*ch, '_' | '-' | '.')
                        })
                        .map_or(text.len(), |(index, _)| token_start + index);
                    let token = &text[token_start..end];
                    let secret = if prefix == "Bearer " {
                        !token.is_empty()
                    } else if prefix == "sk-" {
                        token.len() >= 16
                    } else {
                        token.len() >= 30 && token.matches('.').count() == 2
                    };
                    if secret {
                        text.replace_range(token_start..end, "[redacted]");
                        *count += 1;
                        offset = token_start + "[redacted]".len();
                    } else {
                        offset = start + prefix.len();
                    }
                }
            }
            for label in [
                "access_token",
                "accessToken",
                "refresh_token",
                "refreshToken",
                "api_key",
                "apiKey",
                "password",
                "controller",
                "turn_token",
                "client_secret",
                "accept",
            ] {
                let mut offset = 0;
                while let Some(found) = text[offset..].find(label) {
                    let start = offset + found;
                    let after = start + label.len();
                    let rest = text[after..].trim_start_matches(|ch: char| {
                        ch.is_whitespace() || ch == '"' || ch == '\''
                    });
                    if !rest.starts_with([':', '=']) {
                        offset = after;
                        continue;
                    }
                    let value = rest[1..].trim_start_matches(|ch: char| {
                        ch.is_whitespace() || ch == '"' || ch == '\''
                    });
                    let token_start = text.len() - value.len();
                    if value.starts_with("[redacted]") {
                        offset = token_start + "[redacted]".len();
                        continue;
                    }
                    let end = value
                        .char_indices()
                        .find(|(_, ch)| {
                            ch.is_whitespace() || matches!(*ch, '"' | '\'' | ',' | '}' | ']' | ';')
                        })
                        .map_or(text.len(), |(index, _)| token_start + index);
                    if end > token_start && (label != "accept" || end - token_start >= 16) {
                        text.replace_range(token_start..end, "[redacted]");
                        *count += 1;
                        offset = token_start + "[redacted]".len();
                    } else {
                        offset = after;
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_cursor_skips_only_finished_hidden_final_suffix_without_another_read() {
        let messages = vec![
            json!({"id":"public","role":"assistant","channel":"final","end_turn":true,"status":"finished_successfully","content":{"content_type":"text","parts":["answer"]},"unavailable":null}),
            json!({"id":"hidden","role":"assistant","channel":"final","end_turn":true,"status":"finished_successfully","content":null,"unavailable":"hidden"}),
        ];
        let cursor = Cursor {
            account: "a".into(),
            conversation: "chat".into(),
            branch: "hidden".into(),
            count: 2,
            prefix: digest(&messages),
        };
        let resolved = saved_input_cursor(&messages, &cursor).unwrap();
        assert_eq!(resolved.branch, "public");
        assert_eq!(resolved.count, 1);
        assert_eq!(resolved.prefix, digest(&&messages[..1]));
        assert_eq!(cursor.branch, "hidden");
        assert_eq!(cursor.count, 2);
        for (field, value) in [
            ("channel", json!("analysis")),
            ("end_turn", json!(false)),
            ("status", json!("in_progress")),
            ("unavailable", json!("unsupported_content")),
            ("role", json!("tool")),
        ] {
            let mut changed = messages.clone();
            changed[1][field] = value;
            assert!(saved_input_cursor(&changed, &cursor).is_none());
        }
        let public = saved_input_cursor(&messages[..1], &cursor).unwrap();
        assert_eq!(public.branch, "public");
        assert!(saved_input_cursor(&[], &cursor).is_none());
    }
    use futures_util::future::BoxFuture;
    use std::sync::Mutex;
    #[test]
    fn credentials_inside_json_and_tool_text_are_removed_without_exporting_caps() {
        let mut value = json!({"accessToken":"sensitive","content":["Authorization: Bearer ABC-secret-token", "{\"controller\":\"old-private-controller\",\"safe\":7}","api_key='other-private-key'",format!("controller={}","b".repeat(48)),"public text"]});
        let mut count = 0;
        redact(&mut value, &[], &mut count);
        let text = value.to_string();
        for secret in [
            "sensitive",
            "ABC-secret-token",
            "old-private-controller",
            "other-private-key",
            &"b".repeat(48),
        ] {
            assert!(!text.contains(secret));
        }
        assert!(text.contains("public text"));
        assert!(!text.contains("[redacted]]"));
        assert!(count >= 5);
    }
    #[test]
    fn verified_public_work_handles_survive_historical_capability_redaction() {
        let work = "a".repeat(48);
        let capability = "b".repeat(48);
        let mut value = json!({"work_id":work,"controller":capability,"parts":[format!("Use work {work}; controller={capability}"),json!({"work_id":work,"turn_token":capability}).to_string()]});
        let mut count = 0;
        redact_public(
            &mut value,
            std::slice::from_ref(&capability),
            std::slice::from_ref(&work),
            &mut count,
        );
        assert_eq!(value["work_id"], work);
        assert!(value.to_string().contains(&work));
        assert!(!value.to_string().contains(&capability));
        assert!(count >= 3);
    }
    struct Source {
        snapshot: Mutex<Value>,
        commands: Mutex<Vec<Value>>,
    }
    impl Bridge for Source {
        fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
            self.commands.lock().unwrap().push(command.clone());
            let result = match command["op"].as_str() {
                Some("identity") => {
                    json!({"account":"account","user":"user","structure":"personal"})
                }
                Some("chat_read") => self.snapshot.lock().unwrap().clone(),
                Some("chat_page") => self.snapshot.lock().unwrap()["pages"]
                    [command["before"].as_str().unwrap()]
                .clone(),
                Some("chat_list") => {
                    let items = [
                        json!({"id":"one","title":"same","project_id":"project-one","updated_at":10}),
                        json!({"id":"two","title":"same","project_id":"project-two","updated_at":20}),
                    ];
                    let start = command["offset"].as_u64().unwrap_or(0) as usize;
                    let limit = command["limit"].as_u64().unwrap_or(50) as usize;
                    json!({"items":items.iter().skip(start).take(limit).collect::<Vec<_>>(),"total":2})
                }
                _ => panic!(
                    "reading submitted an unexpected operation: {}",
                    command["op"]
                ),
            };
            Box::pin(async move { Ok(result) })
        }
    }
    fn source() -> Arc<Source> {
        let message = |id: &str, text: &str| json!({"id":id,"role":"assistant","channel":"final","content":{"content_type":"text","parts":[text]}});
        Arc::new(Source {
            snapshot: Mutex::new(
                json!({"conversation_id":"conversation","title":"same","current_node":"two","mapping":{
                    "root":{"id":"root","parent":null,"message":null},
                    "one":{"id":"one","parent":"root","message":message("one","public")},
                    "two":{"id":"two","parent":"one","message":message("two",&format!("controller={} secret=private", "a".repeat(48)))},
                    "branch":{"id":"branch","parent":"one","message":message("branch","alternate")}
                }}),
            ),
            commands: Mutex::new(Vec::new()),
        })
    }
    #[tokio::test]
    async fn pages_preserve_ids_branches_prefixes_and_redact_caps_without_inference() {
        let source = source();
        let chats = Chats::new(source.clone());
        let first = chats
            .call(
                json!({"action":"read","conversation_id":"conversation","limit":1}),
                &["private".into()],
            )
            .await
            .unwrap();
        assert_eq!(first["messages"][0]["id"], "one");
        assert_eq!(first["complete"], false);
        let second=chats.call(json!({"action":"read","conversation_id":"conversation","after":first["cursor"],"limit":1}),&["private".into()]).await.unwrap();
        assert_eq!(second["messages"][0]["id"], "two");
        assert_eq!(second["complete"], true);
        assert!(!second.to_string().contains(&"a".repeat(48)));
        assert!(!second.to_string().contains("secret=private"));
        let branch = chats
            .call(
                json!({"action":"read","conversation_id":"conversation","branch":"branch"}),
                &[],
            )
            .await
            .unwrap();
        assert_eq!(branch["messages"][1]["id"], "branch");
        source.snapshot.lock().unwrap()["mapping"]["one"]["message"]["content"]["parts"][0] =
            json!("changed");
        assert_eq!(chats.call(json!({"action":"read","conversation_id":"conversation","after":first["cursor"]}),&[]).await,Err(Error("chat_source_prefix_changed")));
        assert!(source
            .commands
            .lock()
            .unwrap()
            .iter()
            .all(|c| matches!(c["op"].as_str(), Some("identity" | "chat_read"))));
    }
    #[tokio::test]
    async fn duplicate_titles_are_ids_and_missing_parents_never_claim_completeness() {
        let source = source();
        let chats = Chats::new(source.clone());
        let list = chats.call(json!({"action":"list"}), &[]).await.unwrap();
        assert_eq!(list["items"][0]["title"], list["items"][1]["title"]);
        assert_ne!(list["items"][0]["id"], list["items"][1]["id"]);
        assert_eq!(
            chats
                .call(json!({"action":"list","account":"different"}), &[])
                .await,
            Err(Error("chat_account_changed"))
        );
        source.snapshot.lock().unwrap()["mapping"]
            .as_object_mut()
            .unwrap()
            .remove("one");
        assert_eq!(
            chats
                .call(
                    json!({"action":"read","conversation_id":"conversation"}),
                    &[]
                )
                .await,
            Err(Error("chat_pagination_incomplete"))
        );
    }
    #[tokio::test]
    async fn appended_messages_reuse_verified_prefix_and_other_branches_do_not() {
        let source = source();
        let chats = Chats::new(source.clone());
        let before = chats
            .call(
                json!({"action":"read","conversation_id":"conversation"}),
                &[],
            )
            .await
            .unwrap();
        {
            let mut s = source.snapshot.lock().unwrap();
            s["mapping"]["new"] = json!({"id":"new","parent":"two","message":{"id":"new","role":"user","content":{"content_type":"text","parts":["follow up"]}}});
            s["current_node"] = json!("new");
        }
        let after = chats
            .call(
                json!({"action":"read","conversation_id":"conversation","after":before["cursor"]}),
                &[],
            )
            .await
            .unwrap();
        assert_eq!(after["messages"].as_array().unwrap().len(), 1);
        assert_eq!(after["messages"][0]["id"], "new");
        assert_eq!(after["messages"][0]["parent"], "two");
        source.snapshot.lock().unwrap()["current_node"] = json!("branch");
        assert_eq!(chats.call(json!({"action":"read","conversation_id":"conversation","after":before["cursor"]}),&[]).await,Err(Error("chat_source_prefix_changed")));
    }
    #[tokio::test]
    async fn backend_pages_stitch_in_order_and_missing_pages_fail_explicitly() {
        let source = source();
        {
            let mut s = source.snapshot.lock().unwrap();
            s["pagination"] = json!({"cursor":"newer","oldestMessageId":"one"});
            s["pages"] = json!({"newer":{"messages":[{"id":"previous","role":"user","content":{"content_type":"text","parts":["previous"]}}],"page_info":{"has_previous_page":true,"start_cursor":"older"}},"older":{"messages":[{"id":"first","role":"user","content":{"content_type":"text","parts":["first"]}}],"page_info":{"has_previous_page":false}}});
            s["mapping"].as_object_mut().unwrap().remove("root");
        }
        let chats = Chats::new(source.clone());
        let result = chats
            .call(
                json!({"action":"read","conversation_id":"conversation"}),
                &[],
            )
            .await
            .unwrap();
        assert_eq!(result["source_pages"], 3);
        assert_eq!(result["complete"], true);
        assert_eq!(
            result["messages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["first", "previous", "one", "two"]
        );
        source.snapshot.lock().unwrap()["pages"]["older"]["page_info"] =
            json!({"has_previous_page":true,"start_cursor":"newer"});
        assert_eq!(
            chats
                .call(
                    json!({"action":"read","conversation_id":"conversation"}),
                    &[]
                )
                .await,
            Err(Error("chat_pagination_incomplete"))
        );
    }
    #[tokio::test]
    async fn list_filters_scan_source_offsets_and_cursors_bind_filter_and_account() {
        let source = source();
        let chats = Chats::new(source);
        let first = chats
            .call(json!({"action":"list","title":"SAME","limit":1}), &[])
            .await
            .unwrap();
        assert_eq!(first["items"][0]["id"], "one");
        assert_eq!(first["complete"], false);
        let second=chats.call(json!({"action":"list","title":"SAME","limit":1,"list_cursor":first["list_cursor"]}),&[]).await.unwrap();
        assert_eq!(second["items"][0]["id"], "two");
        assert_eq!(second["complete"], true);
        assert_eq!(
            chats
                .call(
                    json!({"action":"list","title":"other","list_cursor":first["list_cursor"]}),
                    &[]
                )
                .await,
            Err(Error("chat_cursor_invalid"))
        );
        let filtered=chats.call(json!({"action":"list","project_id":"project-two","updated_after":15,"updated_before":25,"limit":1}),&[]).await.unwrap();
        assert_eq!(filtered["source_pages"], 2);
        assert_eq!(filtered["items"][0]["id"], "two");
        assert_eq!(filtered["complete"], true);
        assert!(filtered["total"].is_null());
    }
    #[tokio::test]
    async fn enumerated_range_does_not_claim_unavailable_content_was_read() {
        let source = source();
        source.snapshot.lock().unwrap()["mapping"]["two"]["message"]["unavailable"] =
            json!("hidden");
        let result = Chats::new(source)
            .call(
                json!({"action":"read","conversation_id":"conversation"}),
                &[],
            )
            .await
            .unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(result["content_complete"], false);
        assert_eq!(result["messages"][1]["id"], "two");
        assert!(result["messages"][1]["content"].is_null());
        assert!(!result.to_string().contains("secret=private"));
    }
}
