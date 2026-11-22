//! Type 1 OpenAI Responses API (`/v1/responses` & `/v1/models`) for `codex -p chatgpt`.
//!
//! Emits monotonic OpenAI Responses SSE frames (or non-streaming JSON) with
//! single-path in-app tool callbacks (`local_function_signatures`), supporting
//! both top-level `tools` and Codex `0.171.0+` `additional_tools` namespaces,
//! `function_call` and `custom_tool_call` items, and multi-round session delta
//! continuation via `thread-id` / `session-id` / `prompt_cache_key` / `previous_response_id`.
//! Never synthesizes private/hidden reasoning.

use crate::config::{random_hex, HistoryConfig, ModelConfig};
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const MAX_RESPONSE_THREADS: usize = 256;
const MAX_DEVELOPER_BLOCK_CHARS: usize = 4000;
const MAX_TOOL_DESC_CHARS: usize = 600;
const RESPONSES_WAIT_MS: u64 = 240_000;

#[derive(Debug, Clone)]
pub struct ThreadEntry {
    pub chat_id: String,
    pub input_len: usize,
}

#[derive(Default)]
pub struct ResponseThreadMap {
    entries: Mutex<VecDeque<(String, ThreadEntry)>>,
}

impl ResponseThreadMap {
    pub fn get(&self, key: &str) -> Option<ThreadEntry> {
        let guard = self.entries.lock().ok()?;
        guard
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, entry)| entry.clone())
    }

    pub fn upsert(&self, key: String, entry: ThreadEntry) {
        if let Ok(mut guard) = self.entries.lock() {
            if let Some(pos) = guard.iter().position(|(k, _)| k == &key) {
                guard.remove(pos);
            }
            if guard.len() >= MAX_RESPONSE_THREADS {
                guard.pop_front();
            }
            guard.push_back((key, entry));
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub call_id: String,
    pub name: String,
    pub namespace: Option<String>,
    pub arguments: String,
    pub is_custom: bool,
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}...")
    } else {
        head
    }
}

fn collect_tools_from_array(arr: &[Value], namespace: Option<&str>, out: &mut Vec<Value>) {
    for item in arr {
        let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
        if kind == "namespace" {
            let ns_name = item.get("name").and_then(Value::as_str);
            if let Some(inner) = item.get("tools").and_then(Value::as_array) {
                collect_tools_from_array(inner, ns_name, out);
            }
            continue;
        }
        if kind == "function" || kind == "custom" {
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let desc = item
                .get("description")
                .and_then(Value::as_str)
                .map(|d| truncate_chars(d.trim(), MAX_TOOL_DESC_CHARS));
            let effective_ns = namespace.filter(|ns| !ns.is_empty() && *ns != "functions");
            let mut sig = json!({
                "type": kind,
                "name": name,
            });
            if let Some(ns) = effective_ns {
                sig["namespace"] = json!(ns);
            }
            if let Some(d) = desc {
                sig["description"] = json!(d);
            }
            if kind == "function" {
                if let Some(params) = item.get("parameters") {
                    sig["parameters"] = params.clone();
                }
            } else if let Some(fmt) = item.get("format") {
                sig["format"] = fmt.clone();
            }
            out.push(sig);
        }
    }
}

/// Extract clean `local_function_signatures` from both top-level `tools` and
/// Codex `0.171.0+` `input[*]` items with `type: "additional_tools"` (unwrapping namespaces).
pub fn extract_local_function_signatures(body: &Value) -> Vec<Value> {
    let mut sigs = Vec::new();
    if let Some(arr) = body.get("tools").and_then(Value::as_array) {
        collect_tools_from_array(arr, None, &mut sigs);
    }
    if let Some(items) = body.get("input").and_then(Value::as_array) {
        for item in items {
            if item.get("type").and_then(Value::as_str) == Some("additional_tools") {
                if let Some(arr) = item.get("tools").and_then(Value::as_array) {
                    collect_tools_from_array(arr, None, &mut sigs);
                }
            }
        }
    }
    sigs
}

/// Flatten OpenAI Responses `instructions` + `input` (messages, tool outputs, agent messages)
/// into a clean prompt string for the ChatGPT page driver.
/// When `skip_prefix_items > 0` (multi-round continuation on an existing ChatGPT thread),
/// omits `instructions` and renders only the new delta tail items.
pub fn render_prompt_from_responses_request(
    body: &Value,
    skip_prefix_items: usize,
) -> Result<String> {
    let mut sections: Vec<String> = Vec::new();
    if skip_prefix_items == 0 {
        if let Some(inst) = body
            .get("instructions")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            sections.push(format!(
                "[Instructions]\n{}",
                truncate_chars(inst.trim(), MAX_DEVELOPER_BLOCK_CHARS)
            ));
        }
    }

    match body.get("input") {
        Some(Value::String(s)) if !s.trim().is_empty() => sections.push(s.clone()),
        Some(Value::Array(items)) => {
            let start = if skip_prefix_items < items.len() {
                skip_prefix_items
            } else {
                0
            };
            for item in &items[start..] {
                let kind = item
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("message");
                match kind {
                    "additional_tools" => continue,
                    "function_call_output" | "custom_tool_call_output" => {
                        let cid = item
                            .get("call_id")
                            .and_then(Value::as_str)
                            .unwrap_or("call");
                        let out = extract_tool_output_text(item.get("output"));
                        sections.push(format!("[Tool Output {cid}]\n{out}"));
                    }
                    "function_call" | "custom_tool_call" => {
                        // On continuation turns we only need new tool outputs/messages; the prior
                        // assistant tool call is already in the ChatGPT thread.
                        if skip_prefix_items > 0 {
                            continue;
                        }
                        let name = item.get("name").and_then(Value::as_str).unwrap_or("tool");
                        let args = item
                            .get("arguments")
                            .or_else(|| item.get("input"))
                            .and_then(Value::as_str)
                            .unwrap_or("{}");
                        sections.push(format!("[Previous Tool Call {name}]\n{args}"));
                    }
                    "agent_message" => {
                        let text = item
                            .get("message")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or_else(|| {
                                let c = extract_content_text(item.get("content"));
                                if !c.is_empty() {
                                    c
                                } else {
                                    item.to_string()
                                }
                            });
                        sections.push(format!("[agent_message]\n{text}"));
                    }
                    _ => {
                        let role = item.get("role").and_then(Value::as_str).unwrap_or("user");
                        let mut text = extract_content_text(item.get("content"));
                        if text.is_empty() {
                            continue;
                        }
                        if matches!(role, "developer" | "system") {
                            text = truncate_chars(&text, MAX_DEVELOPER_BLOCK_CHARS);
                        }
                        sections.push(format!("[{role}]\n{text}"));
                    }
                }
            }
        }
        _ => {
            return Err(ProviderError::bad_request(
                "responses_input_required",
                "input must be a non-empty string or array of items",
            ));
        }
    }

    let joined = sections.join("\n\n").trim().to_owned();
    if joined.is_empty() {
        return Err(ProviderError::bad_request(
            "responses_input_empty",
            "input produced an empty prompt",
        ));
    }
    Ok(joined)
}

fn extract_tool_output_text(output: Option<&Value>) -> String {
    match output {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                p.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| p.as_str())
                    .map(ToOwned::to_owned)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn extract_content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                p.get("text")
                    .or_else(|| p.get("encrypted_content"))
                    .and_then(Value::as_str)
                    .or_else(|| p.as_str())
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn resolve_parsed_tool_call(raw: &Value, sigs: &[Value]) -> Option<ParsedToolCall> {
    let call_id = raw.get("id").and_then(Value::as_str)?.to_owned();
    let raw_name = raw.get("name").and_then(Value::as_str)?;
    let raw_ns = raw
        .get("namespace")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    let (mut namespace, name) = if let Some((prefix, rest)) = raw_name.split_once('.') {
        (raw_ns.or_else(|| Some(prefix.to_owned())), rest.to_owned())
    } else {
        (raw_ns, raw_name.to_owned())
    };
    if namespace.as_deref() == Some("functions") {
        namespace = None;
    }

    let matched = sigs.iter().find(|s| {
        s.get("name").and_then(Value::as_str) == Some(name.as_str())
            && (namespace.is_none()
                || s.get("namespace").and_then(Value::as_str) == namespace.as_deref())
    });

    let is_custom = matched
        .and_then(|s| s.get("type").and_then(Value::as_str))
        .is_some_and(|t| t == "custom");
    if namespace.is_none() {
        namespace = matched
            .and_then(|s| s.get("namespace").and_then(Value::as_str))
            .map(ToOwned::to_owned);
    }

    let mut arguments = raw
        .get("arguments")
        .and_then(Value::as_str)
        .unwrap_or(if is_custom { "" } else { "{}" })
        .to_owned();

    // If a custom tool (e.g., Codex `exec`) received a JSON object like {"command":"..."} or {"input":"..."},
    // unwrap the string payload cleanly so Codex executes the raw command string.
    if is_custom && arguments.trim_start().starts_with('{') {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&arguments) {
            if let Some(s) = map
                .get("input")
                .or_else(|| map.get("command"))
                .or_else(|| map.get("cmd"))
                .and_then(Value::as_str)
            {
                arguments = s.to_owned();
            }
        }
    }

    Some(ParsedToolCall {
        call_id,
        name,
        namespace,
        arguments,
        is_custom,
    })
}

/// Build an ordered sequence of OpenAI Responses SSE events with monotonic `sequence_number`.
pub fn build_responses_sse_frames(
    response_id: &str,
    model: &str,
    assistant_text: &str,
    tool_calls: &[ParsedToolCall],
) -> (Vec<Value>, Value) {
    let mut seq = 0u64;
    let mut next = |mut v: Value| {
        v["sequence_number"] = json!(seq);
        seq += 1;
        v
    };

    let base_resp = |status: &str, output: &[Value]| {
        json!({
            "id": response_id,
            "object": "response",
            "created_at": 0,
            "status": status,
            "model": model,
            "output": output,
            "parallel_tool_calls": true,
            "error": null
        })
    };

    let mut frames = Vec::new();
    let mut output_items: Vec<Value> = Vec::new();
    let init = base_resp("in_progress", &[]);
    frames.push(next(json!({"type": "response.created", "response": init})));
    frames.push(next(
        json!({"type": "response.in_progress", "response": init}),
    ));

    if !assistant_text.is_empty() || tool_calls.is_empty() {
        let idx = output_items.len();
        let msg_id = format!("msg_{response_id}");
        let mut item = json!({
            "id": msg_id,
            "type": "message",
            "role": "assistant",
            "status": "in_progress",
            "phase": "final_answer",
            "content": [{"type": "output_text", "text": "", "annotations": []}]
        });
        frames.push(next(
            json!({"type": "response.output_item.added", "output_index": idx, "item": item}),
        ));
        frames.push(next(json!({
            "type": "response.content_part.added",
            "item_id": msg_id,
            "output_index": idx,
            "content_index": 0,
            "part": {"type": "output_text", "text": "", "annotations": []}
        })));
        if !assistant_text.is_empty() {
            frames.push(next(json!({
                "type": "response.output_text.delta",
                "item_id": msg_id,
                "output_index": idx,
                "content_index": 0,
                "delta": assistant_text
            })));
        }
        item["status"] = json!("completed");
        item["content"][0]["text"] = json!(assistant_text);
        frames.push(next(json!({
            "type": "response.output_text.done",
            "item_id": msg_id,
            "output_index": idx,
            "content_index": 0,
            "text": assistant_text
        })));
        frames.push(next(json!({
            "type": "response.content_part.done",
            "item_id": msg_id,
            "output_index": idx,
            "content_index": 0,
            "part": item["content"][0]
        })));
        frames.push(next(
            json!({"type": "response.output_item.done", "output_index": idx, "item": item}),
        ));
        output_items.push(item);
    }

    for tc in tool_calls {
        let idx = output_items.len();
        if tc.is_custom {
            let item_id = format!("ct_{}", tc.call_id);
            let mut init_item = json!({
                "id": item_id,
                "type": "custom_tool_call",
                "call_id": tc.call_id,
                "name": tc.name,
                "input": "",
                "status": "in_progress"
            });
            let mut done_item = json!({
                "id": item_id,
                "type": "custom_tool_call",
                "call_id": tc.call_id,
                "name": tc.name,
                "input": tc.arguments,
                "status": "completed"
            });
            if let Some(ref ns) = tc.namespace {
                init_item["namespace"] = json!(ns);
                done_item["namespace"] = json!(ns);
            }
            frames.push(next(
                json!({"type": "response.output_item.added", "output_index": idx, "item": init_item}),
            ));
            frames.push(next(json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": item_id,
                "output_index": idx,
                "delta": tc.arguments
            })));
            frames.push(next(json!({
                "type": "response.custom_tool_call_input.done",
                "item_id": item_id,
                "output_index": idx,
                "input": tc.arguments
            })));
            frames.push(next(
                json!({"type": "response.output_item.done", "output_index": idx, "item": done_item}),
            ));
            output_items.push(done_item);
        } else {
            let item_id = format!("fc_{}", tc.call_id);
            let mut init_item = json!({
                "id": item_id,
                "type": "function_call",
                "call_id": tc.call_id,
                "name": tc.name,
                "arguments": "",
                "status": "in_progress"
            });
            let mut done_item = json!({
                "id": item_id,
                "type": "function_call",
                "call_id": tc.call_id,
                "name": tc.name,
                "arguments": tc.arguments,
                "status": "completed"
            });
            if let Some(ref ns) = tc.namespace {
                init_item["namespace"] = json!(ns);
                done_item["namespace"] = json!(ns);
            }
            frames.push(next(
                json!({"type": "response.output_item.added", "output_index": idx, "item": init_item}),
            ));
            frames.push(next(json!({
                "type": "response.function_call_arguments.delta",
                "item_id": item_id,
                "output_index": idx,
                "delta": tc.arguments
            })));
            frames.push(next(json!({
                "type": "response.function_call_arguments.done",
                "item_id": item_id,
                "output_index": idx,
                "arguments": tc.arguments
            })));
            frames.push(next(
                json!({"type": "response.output_item.done", "output_index": idx, "item": done_item}),
            ));
            output_items.push(done_item);
        }
    }

    let final_resp = base_resp("completed", &output_items);
    frames.push(next(
        json!({"type": "response.completed", "response": final_resp}),
    ));
    (frames, final_resp)
}

pub async fn handle_models(driver: Arc<dyn DriverBackend>, model_cfg: &ModelConfig) -> Json<Value> {
    let mut slugs: Vec<String> = Vec::new();
    if driver.host_running().await {
        if let Ok(st) = driver.status().await {
            if let Some(arr) = st.get("models").and_then(Value::as_array) {
                for m in arr {
                    if let Some(slug) = m
                        .get("slug")
                        .and_then(Value::as_str)
                        .or_else(|| m.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        slugs.push(slug.to_owned());
                    }
                }
            }
        }
    }
    if slugs.is_empty() {
        slugs = model_cfg.available.clone();
    }
    let data: Vec<Value> = slugs
        .into_iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "owned_by": "chatgpt-provider"
            })
        })
        .collect();
    Json(json!({ "object": "list", "data": data }))
}

pub async fn handle_responses(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    history_cfg: &HistoryConfig,
    threads: &ResponseThreadMap,
    session_hint: Option<String>,
    body: Value,
) -> Result<Response> {
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(&model_cfg.default)
        .to_owned();
    let effort = body
        .get("reasoning")
        .and_then(|r| r.get("effort"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(true);

    let session_key = session_hint.or_else(|| {
        body.get("prompt_cache_key")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(ToOwned::to_owned)
    });
    let input_len = body
        .get("input")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    let (prev_chat_id, skip_prefix) = if let Some(ref key) = session_key {
        if let Some(entry) = threads.get(key) {
            let skip = if entry.input_len > 0 && input_len > entry.input_len {
                entry.input_len
            } else {
                0
            };
            (Some(entry.chat_id), skip)
        } else {
            (None, 0)
        }
    } else if let Some(rid) = body.get("previous_response_id").and_then(Value::as_str) {
        (threads.get(rid).map(|e| e.chat_id), 0)
    } else {
        (None, 0)
    };

    let prompt = render_prompt_from_responses_request(&body, skip_prefix)?;
    let all_sigs = extract_local_function_signatures(&body);

    let driver_res = driver
        .send_message(json!({
            "chat_id": prev_chat_id,
            "project_id": history_cfg.project_id,
            "prompt": prompt,
            "model": model,
            "reasoning_effort": effort,
            "local_function_signatures": all_sigs,
            "is_continuation": skip_prefix > 0,
            "wait_ms": RESPONSES_WAIT_MS
        }))
        .await?;

    let state = driver_res.get("state").and_then(Value::as_str);
    if state == Some("error") {
        let code = driver_res
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("driver_error");
        let detail = driver_res
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or("ChatGPT page driver failed");
        return Err(ProviderError::unavailable(
            "chatgpt_driver_failed",
            format!("{code}: {detail}"),
        ));
    }
    if state == Some("in_progress") {
        return Err(ProviderError::unavailable(
            "chatgpt_response_in_progress",
            "ChatGPT response did not finish within the wait budget",
        ));
    }
    if state == Some("stopped") {
        return Err(ProviderError::unavailable(
            "chatgpt_generation_stopped",
            "ChatGPT generation was stopped before completion",
        ));
    }

    let assistant_text = driver_res
        .get("assistant_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let tool_calls: Vec<ParsedToolCall> = driver_res
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| resolve_parsed_tool_call(v, &all_sigs))
                .collect()
        })
        .unwrap_or_default();

    let resp_id = format!("resp_{}", random_hex(8)?);
    if let Some(cid) = driver_res
        .get("chat_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        threads.upsert(
            resp_id.clone(),
            ThreadEntry {
                chat_id: cid.to_owned(),
                input_len: 0,
            },
        );
        if let Some(key) = session_key {
            threads.upsert(
                key,
                ThreadEntry {
                    chat_id: cid.to_owned(),
                    input_len,
                },
            );
        }
    }
    let (frames, final_resp) =
        build_responses_sse_frames(&resp_id, &model, assistant_text, &tool_calls);

    if !stream {
        return Ok((StatusCode::OK, Json(final_resp)).into_response());
    }

    let mut sse = String::new();
    for frame in frames {
        let ev = frame["type"].as_str().unwrap_or("message");
        sse.push_str("event: ");
        sse.push_str(ev);
        sse.push_str("\ndata: ");
        sse.push_str(&frame.to_string());
        sse.push_str("\n\n");
    }
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")],
        sse,
    )
        .into_response())
}
