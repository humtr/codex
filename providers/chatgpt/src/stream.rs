//! Linear 3-state Responses streaming (`Mutable` -> `Streaming` -> `Committed`),
//! DIL-to-GFM projection, SSE framing, usage accounting, and lane cancellation.

use crate::cdp::{StreamEvent, StreamItemKind};
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use axum::body::Bytes;
use futures_util::Stream;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

pub const RESPONSES_WAIT_MS: u64 = 240_000;
pub const SSE_IDLE: Duration = Duration::from_secs(660);
pub const SSE_IDLE_MIN_SECS: u64 = 30;
pub const SSE_IDLE_MAX_SECS: u64 = 2 * 60 * 60;
pub const SSE_LIMIT: Duration = Duration::from_secs(2 * 60 * 60);
pub const SSE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(12);

pub struct CancelTurnGuard {
    driver: Arc<dyn DriverBackend>,
    lane_id: Option<String>,
}

impl CancelTurnGuard {
    pub fn new(driver: Arc<dyn DriverBackend>, lane_id: String) -> Self {
        Self {
            driver,
            lane_id: Some(lane_id),
        }
    }

    pub fn disarm(&mut self) {
        self.lane_id = None;
    }

    pub async fn cancel_now(&mut self) {
        if let Some(lane_id) = self.lane_id.take() {
            let _ = self
                .driver
                .stop_generation(json!({ "lane_id": lane_id }))
                .await;
        }
    }
}

impl Drop for CancelTurnGuard {
    fn drop(&mut self) {
        if let Some(lane_id) = self.lane_id.take() {
            let driver = Arc::clone(&self.driver);
            tokio::spawn(async move {
                let _ = driver.stop_generation(json!({ "lane_id": lane_id })).await;
            });
        }
    }
}

pub struct MpscSseStream {
    pub rx: mpsc::Receiver<String>,
}

impl Stream for MpscSseStream {
    type Item = std::result::Result<Bytes, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(Bytes::from(chunk)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn format_sse_frame(frame: &Value) -> String {
    let event = frame
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("message");
    format!("event: {event}\ndata: {frame}\n\n")
}

pub async fn send_frames(tx: &mpsc::Sender<String>, frames: Vec<Value>) -> bool {
    for frame in frames {
        if tx.send(format_sse_frame(&frame)).await.is_err() {
            return false;
        }
    }
    true
}

pub async fn send_chunk(tx: &mpsc::Sender<String>, chunk: String) -> bool {
    tx.send(chunk).await.is_ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentState {
    Mutable,
    Streaming,
    Committed,
}

#[derive(Debug, Clone)]
pub enum AssembledContent {
    ReasoningSummary(String),
    Commentary(String),
    Answer(String),
}

#[derive(Debug, Clone)]
pub struct AssembledItem {
    pub id: String,
    pub state: ContentState,
    pub content: AssembledContent,
    pub emitted: bool,
}

#[derive(Default)]
pub struct TurnAssembly {
    pub items: Vec<AssembledItem>,
    preserve_text: bool,
    committed_commentaries: HashSet<String>,
    last_committed_commentary: Option<String>,
    lane_overflowed: bool,
}

impl TurnAssembly {
    pub fn new(preserve_text: bool) -> Self {
        Self {
            preserve_text,
            ..Self::default()
        }
    }

    fn strip_duplicate_commentary_prefix(&self, text: &str) -> Option<String> {
        let prev = self.last_committed_commentary.as_deref()?;
        if prev.is_empty() {
            return None;
        }
        let trimmed = text.trim_start();
        let rest = trimmed.strip_prefix(prev)?;
        if !rest.is_empty() && !rest.starts_with('\n') {
            return None;
        }
        let remainder = rest.trim_start_matches(['\r', '\n']);
        if remainder.is_empty() {
            None
        } else {
            Some(remainder.to_owned())
        }
    }

    pub fn upsert_text(
        &mut self,
        id: String,
        kind: StreamItemKind,
        mut text: String,
        state: ContentState,
    ) {
        let committed = state == ContentState::Committed;
        if text.is_empty() && !committed {
            self.remove_mutable(&id);
            return;
        }
        if self
            .items
            .iter()
            .any(|item| item.id == id && item.state == ContentState::Committed)
        {
            return;
        }
        if !self.preserve_text && state != ContentState::Mutable {
            match kind {
                StreamItemKind::Commentary if committed => {
                    let norm = text.trim().to_owned();
                    if norm.is_empty() || !self.committed_commentaries.insert(norm.clone()) {
                        self.remove_mutable(&id);
                        return;
                    }
                    self.last_committed_commentary = Some(norm);
                }
                StreamItemKind::Answer => {
                    if !committed
                        && self.last_committed_commentary.as_ref().is_some_and(|prev| {
                            prev.starts_with(text.trim_start()) || text.trim_start() == prev
                        })
                    {
                        return;
                    }
                    if let Some(trimmed_answer) = self.strip_duplicate_commentary_prefix(&text) {
                        text = trimmed_answer;
                    }
                }
                _ => {}
            }
        }
        let next_content = match kind {
            StreamItemKind::ReasoningSummary => AssembledContent::ReasoningSummary(text),
            StreamItemKind::Commentary => AssembledContent::Commentary(text),
            StreamItemKind::Answer => AssembledContent::Answer(text),
        };
        if let Some(index) = self.items.iter().position(|item| item.id == id) {
            if self.items[index].state == ContentState::Mutable && state != ContentState::Mutable {
                self.items.remove(index);
                self.items.push(AssembledItem {
                    id,
                    state,
                    content: next_content,
                    emitted: false,
                });
                return;
            }
            let existing = &mut self.items[index];
            if existing.state == ContentState::Streaming {
                let extends = match (&existing.content, &next_content) {
                    (
                        AssembledContent::ReasoningSummary(old),
                        AssembledContent::ReasoningSummary(new),
                    )
                    | (AssembledContent::Answer(old), AssembledContent::Answer(new)) => {
                        new.starts_with(old)
                    }
                    _ => false,
                };
                if matches!(&existing.content, AssembledContent::Answer(_)) {
                    // Ignore mutable revisions until settlement. A raw rewrite
                    // can still extend the projected prefix; only the encoder
                    // knows which bytes actually reached the consumer.
                    if !committed && (!extends || state == ContentState::Mutable) {
                        return;
                    }
                } else if !extends || state == ContentState::Mutable {
                    eprintln!("provider: append-only item revised; preserving published prefix");
                    existing.state = ContentState::Committed;
                    return;
                }
            }
            existing.content = next_content;
            existing.state = state;
            return;
        }
        self.items.push(AssembledItem {
            id,
            state,
            content: next_content,
            emitted: false,
        });
    }

    pub fn remove_mutable(&mut self, id: &str) {
        for item in &mut self.items {
            if item.id == id
                && item.state == ContentState::Streaming
                && !matches!(&item.content, AssembledContent::Answer(_))
            {
                eprintln!("provider: append-only item removed; preserving published prefix");
                item.state = ContentState::Committed;
            }
        }
        self.items
            .retain(|item| item.id != id || item.state != ContentState::Mutable);
    }

    pub fn apply_event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Activity | StreamEvent::TurnCompleted { .. } => {}
            StreamEvent::Overflow => {
                self.lane_overflowed = true;
            }
            StreamEvent::ItemUpsert {
                item_id,
                item_kind,
                text,
                committed,
                streamable,
            } => self.upsert_text(
                item_id,
                item_kind,
                text,
                if committed {
                    ContentState::Committed
                } else if streamable && item_kind != StreamItemKind::Commentary {
                    ContentState::Streaming
                } else {
                    ContentState::Mutable
                },
            ),
            StreamEvent::ItemRemove { item_id } => self.remove_mutable(&item_id),
            StreamEvent::ToolCall { .. } => {}
        }
    }

    pub fn reconcile_settled(&mut self, driver_res: &Value) -> Result<()> {
        if self.lane_overflowed {
            return Err(ProviderError::unavailable(
                "cdp_lane_overflow",
                "lane event queue exceeded its bounded capacity",
            ));
        }
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
            let retry_after = driver_res
                .get("retry_after")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            return Err(classify_driver_error(code, detail, retry_after));
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

        self.items
            .retain(|item| item.state != ContentState::Mutable);
        let committed_ids: HashSet<String> = self
            .items
            .iter()
            .filter(|item| item.state == ContentState::Committed)
            .map(|item| item.id.clone())
            .collect();

        if let Some(items) = driver_res.get("items").and_then(Value::as_array) {
            let mut settled_answer_ids = HashSet::new();
            for entry in items {
                let kind = entry.get("kind").and_then(Value::as_str).unwrap_or("");
                let text = entry.get("text").and_then(Value::as_str).unwrap_or("");
                let item_id = entry
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("{kind}_{}", self.items.len()));
                if kind == "answer" {
                    settled_answer_ids.insert(item_id.clone());
                }
                if committed_ids.contains(&item_id) || (text.is_empty() && kind != "answer") {
                    continue;
                }
                let stream_kind = match kind {
                    "reasoning_summary" => StreamItemKind::ReasoningSummary,
                    "commentary" => StreamItemKind::Commentary,
                    "answer" => StreamItemKind::Answer,
                    _ => continue,
                };
                self.upsert_text(
                    item_id,
                    stream_kind,
                    text.to_owned(),
                    ContentState::Committed,
                );
            }
            // The authoritative items array may omit a removed answer. Do
            // not promote its previously published raw draft into success.
            for item in &mut self.items {
                if item.state == ContentState::Streaming
                    && matches!(&item.content, AssembledContent::Answer(_))
                    && !settled_answer_ids.contains(&item.id)
                {
                    item.content = AssembledContent::Answer(String::new());
                }
            }
        } else {
            let summary = driver_res
                .get("reasoning_summary")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !summary.is_empty()
                && !self
                    .items
                    .iter()
                    .any(|item| matches!(item.content, AssembledContent::ReasoningSummary(_)))
            {
                self.upsert_text(
                    "settled_summary".to_owned(),
                    StreamItemKind::ReasoningSummary,
                    summary.to_owned(),
                    ContentState::Committed,
                );
            }
            let assistant_text = driver_res
                .get("assistant_text")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !assistant_text.is_empty() {
                let mut answers = self
                    .items
                    .iter()
                    .filter(|item| matches!(item.content, AssembledContent::Answer(_)));
                let id = match (answers.next(), answers.next()) {
                    (None, _) => Some("settled_answer".to_owned()),
                    (Some(item), None) if item.state == ContentState::Streaming => {
                        Some(item.id.clone())
                    }
                    _ => None,
                };
                if let Some(id) = id {
                    self.upsert_text(
                        id,
                        StreamItemKind::Answer,
                        assistant_text.to_owned(),
                        ContentState::Committed,
                    );
                }
            }
        }

        for item in &mut self.items {
            if item.state == ContentState::Streaming {
                item.state = ContentState::Committed;
            }
        }
        let has_reply = self.items.iter().any(|item| {
            matches!(
                item.content,
                AssembledContent::Answer(_) | AssembledContent::Commentary(_)
            )
        });
        if !has_reply {
            let summary_text: String = self
                .items
                .iter()
                .filter_map(|item| match &item.content {
                    AssembledContent::ReasoningSummary(t) if !t.is_empty() => Some(t.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            self.upsert_text(
                "settled_answer_fallback".to_owned(),
                StreamItemKind::Answer,
                summary_text,
                ContentState::Committed,
            );
        }

        Ok(())
    }
}

struct OpenTextItem {
    item: Value,
    part_id: Option<String>,
    text: String,
}

impl OpenTextItem {
    fn reasoning(&self) -> bool {
        self.item["type"] == "reasoning"
    }
    fn part_key(&self) -> &'static str {
        if self.reasoning() {
            "summary"
        } else {
            "content"
        }
    }
    fn part(&self, text: &str) -> Value {
        if self.reasoning() {
            json!({"type":"summary_text","text":text})
        } else {
            json!({"type":"output_text","text":text,"annotations":[]})
        }
    }
    fn frame(&self, index: usize, event: &str, field: &str, value: Value) -> Value {
        let (part_event, text_event, part_index) = if self.reasoning() {
            (
                "reasoning_summary_part",
                "reasoning_summary_text",
                "summary_index",
            )
        } else {
            ("content_part", "output_text", "content_index")
        };
        let prefix = if field == "part" {
            part_event
        } else {
            text_event
        };
        let mut frame = json!({
            "type": format!("response.{prefix}.{event}"),
            "item_id": self.item["id"],
            "output_index": index
        });
        frame[part_index] = json!(self.item[self.part_key()].as_array().unwrap().len());
        frame[field] = value;
        frame
    }
}

pub fn estimate_tokens(value: &Value) -> u64 {
    (value.to_string().len() as u64).div_ceil(4)
}

pub struct ResponsesEncoder {
    id: String,
    model: String,
    sequence: u64,
    completed_output: Vec<Value>,
    open: Option<OpenTextItem>,
    pub input_tokens: u64,
    usage: Value,
}

impl ResponsesEncoder {
    pub fn new(id: String, model: String) -> Self {
        Self {
            id,
            model,
            sequence: 0,
            completed_output: vec![],
            open: None,
            input_tokens: 0,
            usage: Value::Null,
        }
    }

    fn stamp(&mut self, mut frame: Value) -> Value {
        frame["sequence_number"] = json!(self.sequence);
        self.sequence += 1;
        frame
    }

    fn response(&self, status: &str, output: &[Value]) -> Value {
        json!({
            "id": self.id,
            "object": "response",
            "created_at": 0,
            "status": status,
            "model": self.model,
            "output": output,
            "parallel_tool_calls": false,
            "error": null,
            "usage": self.usage
        })
    }

    pub fn start(&mut self) -> Vec<Value> {
        let response = self.response("in_progress", &[]);
        vec![
            self.stamp(json!({"type": "response.created", "response": response})),
            self.stamp(json!({"type": "response.in_progress", "response": response})),
        ]
    }

    fn close_open(&mut self, frames: &mut Vec<Value>) {
        if let Some(mut open) = self.open.take() {
            debug_assert!(open.part_id.is_none());
            open.item["status"] = json!("completed");
            frames.push(json!({
                "type": "response.output_item.done",
                "output_index": self.completed_output.len(),
                "item": open.item
            }));
            self.completed_output.push(open.item);
        }
    }

    pub fn flush_ready(&mut self, assembly: &mut TurnAssembly) -> Result<Vec<Value>> {
        let mut frames = vec![];
        for item in &mut assembly.items {
            if item.emitted || item.state == ContentState::Mutable {
                continue;
            }
            if self
                .open
                .as_ref()
                .and_then(|o| o.part_id.as_ref())
                .is_some_and(|id| id != &item.id)
            {
                continue;
            }
            let (reasoning, phase, text) = match &item.content {
                AssembledContent::ReasoningSummary(t) => (true, "", t),
                AssembledContent::Commentary(t) => (false, "commentary", t),
                AssembledContent::Answer(t) => (false, "final_answer", t),
            };
            let committed = item.state == ContentState::Committed;
            let rendered = if reasoning || assembly.preserve_text {
                text.clone()
            } else if committed {
                crate::presentation::render_for_codex_tui(text)
            } else {
                crate::presentation::render_stream_prefix(text)
            };
            if rendered.is_empty() && !committed {
                break;
            }
            if self
                .open
                .as_ref()
                .is_some_and(|o| o.part_id.is_none() && (!reasoning || !o.reasoning()))
            {
                self.close_open(&mut frames);
            }
            let index = self.completed_output.len();
            if self.open.is_none() {
                let id = format!(
                    "{}_{}_{index}",
                    if reasoning { "rs" } else { "msg" },
                    self.id
                );
                let output = if reasoning {
                    json!({"id":id,"type":"reasoning","summary":[],"status":"in_progress"})
                } else {
                    json!({"id":id,"type":"message","role":"assistant","phase":phase,"content":[],"status":"in_progress"})
                };
                let open = OpenTextItem {
                    item: output,
                    part_id: None,
                    text: String::new(),
                };
                let mut initial = open.item.clone();
                if !reasoning {
                    initial["content"] = json!([open.part("")]);
                }
                frames.push(json!({"type":"response.output_item.added","output_index":index,"item":initial}));
                self.open = Some(open);
            }
            let open = self.open.as_mut().unwrap();
            if open.part_id.is_some()
                && (open.reasoning() != reasoning || (!reasoning && open.item["phase"] != phase))
            {
                if committed {
                    return Err(ProviderError::internal(
                        "chatgpt_output_prefix_inconsistent",
                        "Settled item changed its published phase",
                    ));
                }
                break;
            }
            if open.part_id.is_none() {
                open.part_id = Some(item.id.clone());
                frames.push(open.frame(index, "added", "part", open.part("")));
            }
            if let Some(suffix) = rendered.strip_prefix(&open.text) {
                if !suffix.is_empty() {
                    frames.push(open.frame(index, "delta", "delta", json!(suffix)));
                    open.text = rendered;
                }
            } else if committed {
                return Err(ProviderError::internal(
                    "chatgpt_output_prefix_inconsistent",
                    "Settled output does not extend the published text prefix",
                ));
            }
            if !committed {
                break;
            }
            let part = open.part(&open.text);
            frames.push(open.frame(index, "done", "text", json!(open.text)));
            frames.push(open.frame(index, "done", "part", part.clone()));
            let key = open.part_key();
            open.item[key].as_array_mut().unwrap().push(part);
            open.part_id = None;
            open.text.clear();
            item.emitted = true;
            if !reasoning {
                self.close_open(&mut frames);
            }
        }
        Ok(frames.into_iter().map(|v| self.stamp(v)).collect())
    }

    pub fn finish(
        &mut self,
        assembly: &mut TurnAssembly,
        settled: &Value,
    ) -> Result<(Vec<Value>, Value)> {
        let mut frames = self.flush_ready(assembly)?;
        let mut closing = vec![];
        self.close_open(&mut closing);
        let usage = &settled["usage"];
        self.usage = match (
            usage["input_tokens"].as_u64(),
            usage["output_tokens"].as_u64(),
        ) {
            (Some(input), Some(output)) if input.checked_add(output).is_some() => {
                json!({"input_tokens":input,"output_tokens":output,"total_tokens":input+output})
            }
            _ => {
                let output = self
                    .completed_output
                    .iter()
                    .map(|item| {
                        let mut item = item.clone();
                        if let Some(obj) = item.as_object_mut() {
                            obj.remove("id");
                        }
                        estimate_tokens(&item)
                    })
                    .sum::<u64>();
                json!({
                    "input_tokens": self.input_tokens,
                    "output_tokens": output,
                    "total_tokens": self.input_tokens.saturating_add(output),
                    "estimated": true
                })
            }
        };
        let response = self.response("completed", &self.completed_output);
        closing.push(json!({"type": "response.completed", "response": response}));
        frames.extend(closing.into_iter().map(|v| self.stamp(v)));
        Ok((frames, response))
    }

    pub fn failed(&mut self, err: ProviderError) -> Value {
        let mut response = self.response("failed", &[]);
        let mut error_obj = json!({"code": err.code, "message": err.detail});
        if let Some(ra) = err.retry_after {
            error_obj["headers"] = json!({"retry-after": ra});
        }
        response["error"] = error_obj;
        self.stamp(json!({"type": "response.failed", "response": response}))
    }
}

pub fn classify_driver_error(
    code: &str,
    detail: &str,
    retry_after: Option<String>,
) -> ProviderError {
    let code_lc = code.to_ascii_lowercase();
    let detail_lc = detail.to_ascii_lowercase();
    let message = format!("{code}: {detail}");
    if code_lc == "backend_http_429"
        || code_lc.contains("too_many_requests")
        || code_lc.contains("rate_limit")
        || detail_lc.contains("too many requests")
        || detail_lc.contains("rate limit")
    {
        return ProviderError::rate_limited("rate_limit_exceeded", message)
            .with_retry_after(retry_after);
    }
    if matches!(
        code_lc.as_str(),
        "backend_http_502"
            | "backend_http_503"
            | "backend_http_529"
            | "server_is_overloaded"
            | "policy_capacity_unavailable"
            | "too_many_active_generations"
    ) {
        return ProviderError::unavailable("server_is_overloaded", message)
            .with_retry_after(retry_after);
    }
    if code_lc.contains("insufficient_quota")
        || code_lc.contains("credit_balance_exhausted")
        || code_lc.contains("usage_limit")
    {
        return ProviderError::unavailable("insufficient_quota", message);
    }
    if matches!(
        code_lc.as_str(),
        "invalid_prompt" | "backend_http_400" | "backend_http_422"
    ) {
        return ProviderError::bad_request("invalid_prompt", message);
    }
    ProviderError::unavailable("chatgpt_driver_failed", message).with_retry_after(retry_after)
}
