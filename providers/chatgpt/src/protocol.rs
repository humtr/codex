//! Responses wire admission and native tool identity. No app or executor policy.
use crate::config::Config;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Function,
    Custom,
    Search,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    pub name: String,
    pub namespace: Option<String>,
    pub kind: ToolKind,
    pub declaration: Value,
}
impl Tool {
    pub fn key(&self) -> String {
        self.namespace
            .as_ref()
            .map_or_else(|| self.name.clone(), |ns| format!("{ns}.{}", self.name))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub model: String,
    pub effort: String,
    pub host_model: String,
    pub thinking_effort: Option<String>,
    pub instructions: Vec<Value>,
    pub input: Vec<Value>,
    pub tools: BTreeMap<String, Tool>,
    pub host_tools: Vec<Value>,
    pub thread: String,
    pub parent: Option<String>,
    pub turn: Option<String>,
    pub guardian: bool,
    pub compaction: Option<crate::context::Compaction>,
    pub history: crate::config::HistoryConfig,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
fn tools(
    values: &[Value],
    namespace: Option<&str>,
    output: &mut BTreeMap<String, Tool>,
) -> Result<(), Error> {
    for value in values {
        let kind = match value.get("type").and_then(Value::as_str) {
            Some("namespace") if namespace.is_none() => {
                let name = value["name"]
                    .as_str()
                    .filter(|s| id(s))
                    .ok_or(Error("invalid_tool_declaration"))?;
                let children = value["tools"]
                    .as_array()
                    .ok_or(Error("invalid_tool_declaration"))?;
                tools(children, Some(name), output)?;
                continue;
            }
            Some("function") => ToolKind::Function,
            Some("custom") => ToolKind::Custom,
            Some("tool_search") if namespace.is_none() => ToolKind::Search,
            _ => return Err(Error("unsupported_tool_declaration")),
        };
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| (kind == ToolKind::Search).then_some("tool_search"))
            .filter(|s| id(s))
            .ok_or(Error("invalid_tool_declaration"))?
            .to_owned();
        let tool = Tool {
            name,
            namespace: namespace.map(str::to_owned),
            kind,
            declaration: value.clone(),
        };
        match output.get(&tool.key()) {
            Some(previous) if previous != &tool => return Err(Error("ambiguous_tool_declaration")),
            Some(_) => (),
            None => {
                output.insert(tool.key(), tool);
            }
        }
        if output.len() > 4096 {
            return Err(Error("too_many_tools"));
        }
    }
    Ok(())
}

impl Request {
    pub fn parse(
        value: Value,
        config: &Config,
        header_thread: Option<&str>,
    ) -> Result<Self, Error> {
        let body = value
            .as_object()
            .ok_or(Error("invalid_responses_request"))?;
        if body.get("stream") != Some(&Value::Bool(true)) {
            return Err(Error("stream_required"));
        }
        if body
            .get("previous_response_id")
            .is_some_and(|v| !v.is_null())
        {
            return Err(Error("response_cursor_unavailable"));
        }
        let mut model = body
            .get("model")
            .and_then(Value::as_str)
            .ok_or(Error("model_required"))?
            .to_owned();
        let mut effort = value["reasoning"]["effort"]
            .as_str()
            .unwrap_or(&config.model.reasoning_effort)
            .to_owned();
        let input = match body.get("input") {
            Some(Value::String(text)) => vec![
                json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]}),
            ],
            Some(Value::Array(items)) => items.clone(),
            _ => return Err(Error("input_required")),
        };
        let mut instructions = Vec::new();
        if let Some(text) = body.get("instructions").filter(|v| !v.is_null()) {
            if !text.is_string() {
                return Err(Error("invalid_instructions"));
            }
            instructions.push(json!({"role":"developer","content":text}));
        }
        let mut declarations = BTreeMap::new();
        let mut host_tools = Vec::new();
        if let Some(values) = body.get("tools").filter(|v| !v.is_null()) {
            let values = values.as_array().ok_or(Error("invalid_tool_declaration"))?;
            for tool in values {
                if tool["type"] == "web_search" {
                    // Host-side search is not a native Codex function. Preserve the
                    // declaration separately; never invent an executable name.
                    if tool.as_object().is_none_or(|v| {
                        v.keys()
                            .any(|k| !matches!(k.as_str(), "type" | "external_web_access"))
                    }) || tool.get("external_web_access").is_some_and(|v| v != true)
                    {
                        return Err(Error("unsupported_host_search_options"));
                    }
                    if host_tools.iter().any(|v: &Value| v["type"] == "web_search") {
                        return Err(Error("ambiguous_tool_declaration"));
                    }
                    host_tools.push(tool.clone());
                } else {
                    tools(std::slice::from_ref(tool), None, &mut declarations)?;
                }
            }
        }
        if let Some(format) = value
            .get("text")
            .and_then(|v| v.get("format"))
            .filter(|v| !v.is_null())
        {
            let instruction=match format["type"].as_str() {
                Some("text") => None,
                Some("json_object") => Some("Return only a JSON object, without Markdown fences.".to_owned()),
                Some("json_schema") if format["schema"].is_object() => Some(format!("Return only JSON conforming to this schema, without Markdown fences. Core validates the result.\n{}",format["schema"])),
                _ => return Err(Error("unsupported_output_format")),
            };
            if let Some(text) = instruction {
                instructions.push(json!({"role":"developer","content":text}));
            }
        }
        let mut results = BTreeSet::new();
        for item in &input {
            let kind = item
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("message");
            match kind {
                "additional_tools" => tools(
                    item["tools"]
                        .as_array()
                        .ok_or(Error("invalid_tool_declaration"))?,
                    None,
                    &mut declarations,
                )?,
                "message" => {
                    if !matches!(
                        item["role"].as_str(),
                        Some("user" | "assistant" | "developer" | "system")
                    ) {
                        return Err(Error("unsupported_message_role"));
                    }
                    match &item["content"] {
                        Value::String(_) => (),
                        Value::Array(parts)
                            if parts.iter().all(|p| {
                                matches!(
                                    p["type"].as_str(),
                                    Some("input_text" | "output_text" | "text")
                                ) && p["text"].is_string()
                            }) => {}
                        _ => return Err(Error("text_only")),
                    }
                }
                "function_call" | "custom_tool_call" | "tool_search_call" | "reasoning" => (),
                "function_call_output" | "custom_tool_call_output" | "tool_search_output" => {
                    let call = item["call_id"]
                        .as_str()
                        .filter(|v| id(v))
                        .ok_or(Error("invalid_tool_result"))?;
                    if !results.insert(call) {
                        return Err(Error("duplicate_tool_result"));
                    }
                    if kind == "tool_search_output" {
                        if let Some(values) = item.get("tools") {
                            tools(
                                values.as_array().ok_or(Error("invalid_tool_declaration"))?,
                                None,
                                &mut declarations,
                            )?;
                        }
                    }
                }
                "compaction" | "compaction_summary" | "context_compaction"
                | "compaction_trigger" => (),
                _ => return Err(Error("unsupported_input_item")),
            }
        }
        // Reject unreadable checkpoints and unsupported input before allocating
        // any app lane; this is admission, not an implicit context conversion.
        let metadata = match &value["client_metadata"]["x-codex-turn-metadata"] {
            Value::String(text) => {
                serde_json::from_str::<Value>(text).map_err(|_| Error("invalid_turn_metadata"))?
            }
            Value::Object(_) => value["client_metadata"]["x-codex-turn-metadata"].clone(),
            Value::Null => json!({}),
            _ => return Err(Error("invalid_turn_metadata")),
        };
        // Native metadata and explicit thread headers are correlation, never app auth.
        let field = |name: &str| metadata[name].as_str().filter(|v| id(v)).map(str::to_owned);
        let thread = field("thread_id")
            .or_else(|| header_thread.filter(|v| id(v)).map(str::to_owned))
            .ok_or(Error("native_thread_identity_required"))?;
        let guardian = field("thread_source").as_deref() == Some("guardian_review")
            || field("agent_name").as_deref() == Some("guardian")
            || field("subagent_kind").as_deref() == Some("guardian");
        let parent = field("parent_thread_id").or_else(|| field("forked_from_thread_id"));
        let compaction = crate::context::Compaction::detect(&input, &metadata)?;
        if let Some((_, handoff)) = crate::handoff::latest(&input)? {
            if handoff.thread != thread && parent.as_ref().is_none_or(|id| id == &thread) {
                return Err(Error("handoff_thread_mismatch"));
            }
            // Native model-downshift compaction can retain the old model ID.
            // Only this explicit control request may use its selected handoff route.
            if compaction.is_some() && model == handoff.from_model {
                model = handoff.model;
                effort = handoff.effort;
            }
        }
        let route = config
            .catalog
            .route(&model, &effort)
            .ok_or(Error("unsupported_desktop_route"))?;
        crate::context::messages(&input)?;
        Ok(Self {
            model,
            effort,
            host_model: route.host_model.clone(),
            thinking_effort: route.thinking_effort.clone(),
            instructions,
            input,
            tools: declarations,
            host_tools,
            thread,
            parent,
            turn: field("turn_id"),
            guardian,
            compaction,
            history: config.history.clone(),
        })
    }

    pub fn results(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.input.iter().filter_map(|v| {
            matches!(
                v["type"].as_str(),
                Some("function_call_output" | "custom_tool_call_output" | "tool_search_output")
            )
            .then(|| Some((v["call_id"].as_str()?, v)))
            .flatten()
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Call {
    pub id: String,
    pub host_id: String,
    pub tool: Tool,
    pub input: String,
}

/// One host gateway call may advertise a batch. Retain its original ID for the
/// single host ACK; derived client IDs identify only individual native effects.
pub fn gateway_calls(
    host_id: &str,
    arguments: &str,
    advertised: &BTreeMap<String, Tool>,
) -> Result<(Vec<Call>, Option<String>), Error> {
    if !id(host_id) {
        return Err(Error("invalid_host_call"));
    }
    let value: Value =
        serde_json::from_str(arguments).map_err(|_| Error("invalid_gateway_arguments"))?;
    let value = value.get("kwargs").cloned().unwrap_or(value);
    let report = value
        .get("report")
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or(Error("invalid_gateway_report"))
        })
        .transpose()?;
    if value.get("calls").is_some() && (value.get("name").is_some() || value.get("input").is_some())
    {
        return Err(Error("invalid_gateway_batch"));
    }
    let batch = match value.get("calls") {
        Some(Value::String(text)) => {
            Some(serde_json::from_str::<Value>(text).map_err(|_| Error("invalid_gateway_batch"))?)
        }
        Some(value) => Some(value.clone()),
        None => None,
    };
    let values = match batch.as_ref() {
        Some(Value::Array(values)) if !values.is_empty() && values.len() <= 32 => values.clone(),
        Some(_) => return Err(Error("invalid_gateway_batch")),
        None if value.get("name").is_some() => vec![value],
        None if report.is_some() => return Ok((Vec::new(), report)),
        _ => return Err(Error("invalid_gateway_arguments")),
    };
    let encoded = crate::context::digest(&host_id);
    let mut calls = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let tool = advertised
            .get(
                value["name"]
                    .as_str()
                    .ok_or(Error("invalid_gateway_arguments"))?,
            )
            .ok_or(Error("unadvertised_tool"))?
            .clone();
        // Preserve malformed ordinary function JSON for Core to diagnose/repair.
        let input = value["input"]
            .as_str()
            .ok_or(Error("invalid_gateway_arguments"))?
            .to_owned();
        if tool.kind == ToolKind::Search
            && !serde_json::from_str::<Value>(&input).is_ok_and(|v| v.is_object())
        {
            return Err(Error("invalid_tool_search_arguments"));
        }
        calls.push(Call {
            id: if values.len() == 1 {
                host_id.to_owned()
            } else {
                format!("termux_{encoded}_{index}")
            },
            host_id: host_id.to_owned(),
            tool,
            input,
        });
    }
    Ok((calls, report))
}

impl Call {
    pub fn output_item(&self) -> Result<Value, Error> {
        let mut item =
            json!({"id":self.id,"call_id":self.id,"name":self.tool.name,"status":"completed"});
        if let Some(namespace) = &self.tool.namespace {
            item["namespace"] = json!(namespace);
        }
        match self.tool.kind {
            ToolKind::Function => {
                item["type"] = json!("function_call");
                item["arguments"] = json!(self.input);
            }
            ToolKind::Custom => {
                item["type"] = json!("custom_tool_call");
                item["input"] = json!(self.input);
            }
            ToolKind::Search => {
                // Unlike function/custom calls, the upstream search item has
                // no name. Extra fields disappear at canonical persistence.
                item.as_object_mut().unwrap().remove("name");
                item["type"] = json!("tool_search_call");
                item["execution"] = json!("client");
                item["arguments"] = serde_json::from_str(&self.input)
                    .map_err(|_| Error("invalid_tool_search_arguments"))?;
            }
        }
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Value {
        json!({"model":"chatgpt-desktop/gpt-6","stream":true,"input":"test","client_metadata":{"x-codex-turn-metadata":{"thread_id":"native-thread"}},"tools":[{"type":"namespace","name":"functions","tools":[{"type":"function","name":"exec","parameters":{"type":"object"}},{"type":"custom","name":"patch","format":{"type":"text"}}]},{"type":"tool_search"}]})
    }
    #[test]
    fn host_web_search_is_not_invented_as_a_native_executable() {
        let mut body = json!({"model":"chatgpt-desktop/gpt-6","stream":true,"input":"task","tools":[{"type":"web_search","external_web_access":true}]});
        let request = Request::parse(body.clone(), &Config::default(), Some("owned")).unwrap();
        assert!(request.tools.is_empty());
        assert_eq!(request.host_tools.len(), 1);
        assert_eq!(
            gateway_calls(
                "host-id",
                r#"{"name":"web_search","input":"{}"}"#,
                &request.tools
            )
            .unwrap_err(),
            Error("unadvertised_tool")
        );
        body["tools"][0]["external_web_access"] = json!(false);
        assert_eq!(
            Request::parse(body, &Config::default(), Some("owned")).unwrap_err(),
            Error("unsupported_host_search_options")
        );
    }
    #[test]
    fn config_drives_route_and_discovered_tools_preserve_namespaces() {
        let mut body = request();
        body["reasoning"] = json!({"effort":"xhigh"});
        body["input"] = json!([{"type":"tool_search_output","call_id":"search","tools":[{"type":"namespace","name":"mcp__tdev","tools":[{"type":"function","name":"run","parameters":{"type":"object"}}]}]}]);
        let value = Request::parse(body, &Config::default(), None).unwrap();
        assert_eq!(value.host_model, "gpt-6-thinking");
        assert_eq!(value.thinking_effort.as_deref(), Some("max"));
        assert!(value.tools.contains_key("mcp__tdev.run"));
        let (calls, _) = gateway_calls(
            "original",
            r#"{"name":"mcp__tdev.run","input":"{"}"#,
            &value.tools,
        )
        .unwrap();
        assert_eq!(calls[0].output_item().unwrap()["namespace"], "mcp__tdev");
        assert_eq!(calls[0].input, "{");
        assert_eq!(calls[0].id, "original");
        assert_eq!(calls[0].host_id, "original");
    }
    #[test]
    fn batches_keep_host_ack_and_native_call_result_identity() {
        let value = Request::parse(request(), &Config::default(), None).unwrap();
        let (calls, report) = gateway_calls("host-1", r#"{"calls":[{"name":"functions.exec","input":"{}"},{"name":"functions.patch","input":"raw patch\n"}],"report":"Working"}"#, &value.tools).unwrap();
        assert_eq!(report.as_deref(), Some("Working"));
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|c| c.host_id == "host-1"));
        assert_ne!(calls[0].id, calls[1].id);
        assert_eq!(calls[1].output_item().unwrap()["type"], "custom_tool_call");
        assert_eq!(
            gateway_calls("host", r#"{"name":"missing","input":"{}"}"#, &value.tools).unwrap_err(),
            Error("unadvertised_tool")
        );
    }
    #[test]
    fn unsafe_or_unsupported_requests_never_reach_host() {
        for (field, replacement, code) in [
            ("stream", json!(false), "stream_required"),
            ("model", json!("gpt-6.1-sol"), "unsupported_desktop_route"),
            (
                "previous_response_id",
                json!("r"),
                "response_cursor_unavailable",
            ),
            (
                "input",
                json!([{"role":"user","content":[{"type":"input_image","image_url":"private"}]}]),
                "text_only",
            ),
        ] {
            let mut value = request();
            value[field] = replacement;
            assert_eq!(
                Request::parse(value, &Config::default(), None).unwrap_err(),
                Error(code)
            );
        }
        let mut value = request();
        value["input"] = json!([{"type":"function_call_output","call_id":"same","output":"one"},{"type":"function_call_output","call_id":"same","output":"two"}]);
        assert_eq!(
            Request::parse(value, &Config::default(), None).unwrap_err(),
            Error("duplicate_tool_result")
        );
    }

    #[test]
    fn native_additional_tools_and_reviewer_output_format_are_retained() {
        let mut body = request();
        body["text"] = json!({"format":{"type":"json_schema","name":"review","schema":{"type":"object","properties":{"outcome":{"type":"string"}}}}});
        body["input"] = json!([{"type":"additional_tools","role":"developer","tools":[{"type":"namespace","name":"mcp__owned","tools":[{"type":"function","name":"read","parameters":{"type":"object"}}]}]},{"role":"user","content":"Review the owned command"}]);
        body["client_metadata"] = json!({"x-codex-turn-metadata":json!({"thread_id":"guardian-thread","parent_thread_id":"parent","agent_name":"guardian"}).to_string()});
        let parsed = Request::parse(body, &Config::default(), None).unwrap();
        assert!(parsed.tools.contains_key("mcp__owned.read"));
        assert!(parsed.guardian);
        assert_eq!(parsed.parent.as_deref(), Some("parent"));
        assert!(parsed.instructions.last().unwrap()["content"]
            .as_str()
            .unwrap()
            .contains("outcome"));
        let visible = crate::context::messages(&parsed.input).unwrap();
        assert_eq!(visible.len(), 1);
        assert!(!visible[0].hidden);
        let mut actual = request();
        actual["client_metadata"] = json!({"x-codex-turn-metadata":json!({
            "thread_id":"native-review","agent_name":"/root", "thread_source":"guardian_review"
        }).to_string()});
        assert!(
            Request::parse(actual, &Config::default(), None)
                .unwrap()
                .guardian
        );
    }
}
