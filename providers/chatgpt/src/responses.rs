//! Type 1 OpenAI Responses API (`/v1/responses` & `/v1/models`) for `codex -p chatgpt`.
//!
//! Emits monotonic OpenAI Responses SSE frames (or non-streaming JSON) with
//! single-path in-app tool callbacks (`local_function_signatures`).
//! Never synthesizes private/hidden reasoning.

use crate::config::{random_hex, ModelConfig};
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

/// Extract clean `local_function_signatures` from OpenAI Responses `tools` declarations.
pub fn extract_local_function_signatures(tools: Option<&Value>) -> Vec<Value> {
    let Some(arr) = tools.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut sigs = Vec::new();
    for item in arr {
        let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
        if kind == "function" || kind == "custom" {
            if let Some(name) = item.get("name").and_then(Value::as_str) {
                sigs.push(json!({
                    "type": kind,
                    "name": name,
                    "description": item.get("description").cloned().unwrap_or(Value::Null),
                    "parameters": item.get("parameters").cloned().unwrap_or(Value::Null),
                }));
            }
        }
    }
    sigs
}

/// Flatten OpenAI Responses `instructions` + `input` (messages & function_call_output)
/// into a clean prompt string for the ChatGPT page driver.
pub fn render_prompt_from_responses_request(body: &Value) -> Result<String> {
    let mut sections: Vec<String> = Vec::new();
    if let Some(inst) = body
        .get("instructions")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        sections.push(format!("[Instructions]\n{inst}"));
    }

    match body.get("input") {
        Some(Value::String(s)) if !s.trim().is_empty() => sections.push(s.clone()),
        Some(Value::Array(items)) => {
            for item in items {
                let kind = item
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("message");
                match kind {
                    "function_call_output" | "custom_tool_call_output" => {
                        let cid = item
                            .get("call_id")
                            .and_then(Value::as_str)
                            .unwrap_or("call");
                        let out = item
                            .get("output")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| {
                                item.get("output").map(Value::to_string).unwrap_or_default()
                            });
                        sections.push(format!("[Tool Output {cid}]\n{out}"));
                    }
                    "function_call" | "custom_tool_call" => {
                        let name = item.get("name").and_then(Value::as_str).unwrap_or("tool");
                        let args = item
                            .get("arguments")
                            .or_else(|| item.get("input"))
                            .and_then(Value::as_str)
                            .unwrap_or("{}");
                        sections.push(format!("[Previous Tool Call {name}]\n{args}"));
                    }
                    _ => {
                        let role = item.get("role").and_then(Value::as_str).unwrap_or("user");
                        let text = extract_content_text(item.get("content"));
                        if !text.is_empty() {
                            sections.push(format!("[{role}]\n{text}"));
                        }
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

fn extract_content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str).or_else(|| p.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
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
        let item_id = format!("fc_{}", tc.call_id);
        let init_item = json!({
            "id": item_id,
            "type": "function_call",
            "call_id": tc.call_id,
            "name": tc.name,
            "arguments": "",
            "status": "in_progress"
        });
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
        let done_item = json!({
            "id": item_id,
            "type": "function_call",
            "call_id": tc.call_id,
            "name": tc.name,
            "arguments": tc.arguments,
            "status": "completed"
        });
        frames.push(next(
            json!({"type": "response.output_item.done", "output_index": idx, "item": done_item}),
        ));
        output_items.push(done_item);
    }

    let final_resp = base_resp("completed", &output_items);
    frames.push(next(
        json!({"type": "response.completed", "response": final_resp}),
    ));
    (frames, final_resp)
}

pub fn handle_models(model_cfg: &ModelConfig) -> Json<Value> {
    let data: Vec<Value> = model_cfg
        .available
        .iter()
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
        .unwrap_or(&model_cfg.reasoning_effort)
        .to_owned();
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(true);
    let prompt = render_prompt_from_responses_request(&body)?;
    let local_function_signatures = extract_local_function_signatures(body.get("tools"));

    let driver_res = driver
        .send_message(json!({
            "prompt": prompt,
            "model": model,
            "reasoning_effort": effort,
            "local_function_signatures": local_function_signatures,
            "wait_ms": 90_000
        }))
        .await?;

    if driver_res.get("state").and_then(Value::as_str) == Some("error") {
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

    let assistant_text = driver_res
        .get("assistant_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let tool_calls: Vec<ParsedToolCall> = driver_res
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(ParsedToolCall {
                        call_id: v.get("id").and_then(Value::as_str)?.to_owned(),
                        name: v.get("name").and_then(Value::as_str)?.to_owned(),
                        arguments: v
                            .get("arguments")
                            .and_then(Value::as_str)
                            .unwrap_or("{}")
                            .to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let resp_id = format!("resp_{}", random_hex(8)?);
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
