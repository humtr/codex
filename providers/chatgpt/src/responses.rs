//! Type 1 OpenAI Responses API (`/v1/responses` & `/v1/models`) for `codex -p chatgpt`.
//!
//! Emits monotonic OpenAI Responses SSE frames (or non-streaming JSON) with
//! single-path in-app tool callbacks (`local_function_signatures`), supporting
//! both top-level `tools` and Codex `0.171.0+` `additional_tools` namespaces,
//! `function_call` and `custom_tool_call` items, and multi-round session delta
//! continuation via `thread-id` / `session-id` / `prompt_cache_key` / `previous_response_id`.
//! Never synthesizes private/hidden reasoning.

use crate::cdp::StreamUpdate;
use crate::config::{random_hex, HistoryConfig, ModelConfig};
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::convert::Infallible;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_RESPONSE_THREADS: usize = 256;
const MAX_DEVELOPER_BLOCK_CHARS: usize = 48_000;
const MAX_TOTAL_PROMPT_CHARS: usize = 180_000;
const MAX_TOOL_DESC_CHARS: usize = 600;
const RESPONSES_WAIT_MS: u64 = 240_000;
const SSE_IDLE: Duration = Duration::from_secs(300);
const SSE_LIMIT: Duration = Duration::from_secs(2 * 60 * 60);
const SSE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadEntry {
    pub chat_id: String,
    pub input_len: usize,
    pub prefix_sha256: [u8; 32],
    pub account_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Default)]
pub struct ResponseThreadMap {
    entries: Mutex<VecDeque<(String, ThreadEntry)>>,
    store: Option<(PathBuf, File)>,
    active: Mutex<HashSet<String>>,
}

/// Pure-Rust SHA-256 (FIPS 180-4) for zero-dependency prefix digest tracking across
/// Codex compaction / history edits.
fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((data.len() + 9 + 63) & !63);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for block in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, delta) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(delta);
        }
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn input_prefix_digest(items: &[Value], len: usize) -> [u8; 32] {
    let slice = &items[..len.min(items.len())];
    let serialized = serde_json::to_vec(slice).unwrap_or_default();
    sha256_bytes(&serialized)
}

/// Detect Codex background utility calls (such as Transcript Title Generation) that
/// should not pollute the user's configured ChatGPT Project.
fn is_utility_title_request(body: &Value) -> bool {
    let has_tools = body
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    if has_tools {
        return false;
    }
    if let Some(inst) = body.get("instructions").and_then(Value::as_str) {
        let lower = inst.to_ascii_lowercase();
        if lower.contains("generate a concise title")
            || lower.contains("transcript title generation")
            || lower.contains("generate a short title")
            || lower.contains("title for this conversation")
        {
            return true;
        }
    }
    if let Some(items) = body.get("input").and_then(Value::as_array) {
        if items.len() <= 3 {
            for item in items {
                let text = extract_content_text(item.get("content"));
                let lower = text.to_ascii_lowercase();
                if lower.contains("transcript title generation")
                    || lower.contains("generate a concise title for")
                {
                    return true;
                }
            }
        }
    }
    false
}

struct CancelTurnGuard {
    driver: Arc<dyn DriverBackend>,
    lane_id: String,
    armed: bool,
}

impl CancelTurnGuard {
    fn new(driver: Arc<dyn DriverBackend>, lane_id: String) -> Self {
        Self {
            driver,
            lane_id,
            armed: true,
        }
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CancelTurnGuard {
    fn drop(&mut self) {
        if self.armed {
            let driver = Arc::clone(&self.driver);
            let lane_id = self.lane_id.clone();
            tokio::spawn(async move {
                let _ = driver.stop_generation(json!({ "lane_id": lane_id })).await;
            });
        }
    }
}

struct MpscSseStream {
    rx: mpsc::Receiver<String>,
}

impl Stream for MpscSseStream {
    type Item = std::result::Result<String, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(chunk))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn format_sse_frame(frame: &Value) -> String {
    let ev = frame["type"].as_str().unwrap_or("message");
    format!("event: {ev}\ndata: {}\n\n", frame)
}

struct ThreadLease(Arc<ResponseThreadMap>, String);

impl Drop for ThreadLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.active.lock() {
            active.remove(&self.1);
        }
    }
}

impl ResponseThreadMap {
    fn lease(self: &Arc<Self>, key: &str) -> Result<ThreadLease> {
        let mut active = self.active.lock().map_err(|_| {
            ProviderError::internal("response_thread_map_lock", "thread lease lock poisoned")
        })?;
        if !active.insert(key.to_owned()) {
            return Err(ProviderError::unavailable(
                "response_thread_busy",
                "this Codex session has an active response",
            ));
        }
        Ok(ThreadLease(Arc::clone(self), key.to_owned()))
    }

    pub fn get(&self, key: &str) -> Option<ThreadEntry> {
        let guard = self.entries.lock().ok()?;
        guard
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, entry)| entry.clone())
    }

    pub fn open(state_dir: &Path) -> Result<Self> {
        let open = || -> io::Result<Self> {
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(state_dir.join("response_threads.lock"))?;
            check_private_file(&lock)?;
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(io::Error::last_os_error());
            }
            let path = state_dir.join("response_threads.json");
            let entries = match OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
            {
                Ok(file) => {
                    check_private_file(&file)?;
                    let mut data = Vec::new();
                    file.take(1_048_577).read_to_end(&mut data)?;
                    if data.len() > 1_048_576 {
                        return Err(io::Error::other("response thread map exceeds 1 MiB"));
                    }
                    let entries: VecDeque<(String, ThreadEntry)> =
                        serde_json::from_slice(&data).map_err(io::Error::other)?;
                    if entries.len() > MAX_RESPONSE_THREADS
                        || entries
                            .iter()
                            .any(|(_, entry)| entry.account_id.as_deref().is_none_or(str::is_empty))
                    {
                        return Err(io::Error::other("invalid response thread map"));
                    }
                    entries
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => VecDeque::new(),
                Err(err) => return Err(err),
            };
            Ok(Self {
                entries: Mutex::new(entries),
                store: Some((path, lock)),
                ..Self::default()
            })
        };
        open().map_err(|err| ProviderError::internal("response_thread_map_open", err.to_string()))
    }

    fn record(&self, entries: impl IntoIterator<Item = (String, ThreadEntry)>) -> Result<()> {
        let mut guard = self.entries.lock().map_err(|_| {
            ProviderError::internal("response_thread_map_write", "thread map lock poisoned")
        })?;
        let mut next = guard.clone();
        for (key, entry) in entries {
            if let Some(pos) = next.iter().position(|(existing, _)| existing == &key) {
                next.remove(pos);
            }
            next.push_back((key, entry));
            if next.len() > MAX_RESPONSE_THREADS {
                next.pop_front();
            }
        }
        if let Some((path, _)) = &self.store {
            let tmp = path.with_extension(format!("tmp-{}", random_hex(8)?));
            let write = || -> io::Result<()> {
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&tmp)?;
                let mut writer = io::BufWriter::new(file);
                serde_json::to_writer(&mut writer, &next).map_err(io::Error::other)?;
                writer.flush()?;
                writer.get_ref().sync_all()?;
                fs::rename(&tmp, path)?;
                Ok(())
            };
            if let Err(err) = write() {
                let _ = fs::remove_file(&tmp);
                return Err(ProviderError::internal(
                    "response_thread_map_write",
                    err.to_string(),
                ));
            }
            // Rename changes the visible map even if the directory sync fails.
            *guard = next;
            File::open(path.parent().expect("state path has a parent"))
                .and_then(|dir| dir.sync_all())
                .map_err(|err| {
                    ProviderError::internal("response_thread_map_sync", err.to_string())
                })?;
        } else {
            *guard = next;
        }
        Ok(())
    }
}

fn check_private_file(file: &File) -> io::Result<()> {
    let meta = file.metadata()?;
    if meta.is_file() && meta.uid() == unsafe { libc::geteuid() } && meta.mode() & 0o777 == 0o600 {
        Ok(())
    } else {
        Err(io::Error::other(
            "response thread state must be an owned 0600 regular file",
        ))
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
                .map(|description| {
                    let description = description.trim();
                    let truncated = truncate_chars(description, MAX_TOOL_DESC_CHARS);
                    if kind == "custom" && name == "exec" && description.contains("ALL_TOOLS")
                        && !truncated.contains("ALL_TOOLS")
                    {
                        // Codex's deferred MCP namespaces arrive without tool schemas.
                        // Keep its discovery instruction within the AppHost description cap.
                        return truncate_chars(&format!(
                            "Discover nested tools with text(ALL_TOOLS.filter(t => t.name.includes(\"name\"))). {description}"
                        ), MAX_TOOL_DESC_CHARS);
                    }
                    truncated
                });
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

    let mut joined = sections.join("\n\n").trim().to_owned();
    if joined.is_empty() {
        return Err(ProviderError::bad_request(
            "responses_input_empty",
            "input produced an empty prompt",
        ));
    }
    if joined.chars().count() > MAX_TOTAL_PROMPT_CHARS {
        joined = truncate_chars(&joined, MAX_TOTAL_PROMPT_CHARS);
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

#[derive(Clone, Copy, PartialEq)]
enum OutputKind {
    Reasoning,
    Answer,
}

struct OutputEvents {
    prefix: &'static str,
    index: &'static str,
    added: &'static str,
    delta: &'static str,
    done: &'static str,
    part_done: &'static str,
}

impl OutputKind {
    fn events(self) -> OutputEvents {
        match self {
            Self::Reasoning => OutputEvents {
                prefix: "rs",
                index: "summary_index",
                added: "response.reasoning_summary_part.added",
                delta: "response.reasoning_summary_text.delta",
                done: "response.reasoning_summary_text.done",
                part_done: "response.reasoning_summary_part.done",
            },
            Self::Answer => OutputEvents {
                prefix: "msg",
                index: "content_index",
                added: "response.content_part.added",
                delta: "response.output_text.delta",
                done: "response.output_text.done",
                part_done: "response.content_part.done",
            },
        }
    }
}

struct LiveItem {
    kind: OutputKind,
    id: String,
    text: String,
}

impl LiveItem {
    fn part(&self, text: &str) -> Value {
        match self.kind {
            OutputKind::Reasoning => json!({"type": "summary_text", "text": text}),
            OutputKind::Answer => json!({"type": "output_text", "text": text, "annotations": []}),
        }
    }

    fn item(&self, completed: bool) -> Value {
        let status = if completed {
            "completed"
        } else {
            "in_progress"
        };
        match self.kind {
            OutputKind::Reasoning => {
                let summary = if completed {
                    vec![self.part(&self.text)]
                } else {
                    vec![]
                };
                json!({"id": self.id, "type": "reasoning", "summary": summary, "status": status})
            }
            OutputKind::Answer => json!({
                "id": self.id, "type": "message", "role": "assistant", "status": status,
                "phase": "final_answer", "content": [self.part(&self.text)]
            }),
        }
    }
}

struct LiveResponse {
    id: String,
    model: String,
    sequence: u64,
    items: Vec<LiveItem>,
}

impl LiveResponse {
    fn new(id: String, model: String) -> Self {
        Self {
            id,
            model,
            sequence: 0,
            items: vec![],
        }
    }

    fn stamp(&mut self, mut frame: Value) -> Value {
        frame["sequence_number"] = json!(self.sequence);
        self.sequence += 1;
        frame
    }

    fn response(&self, status: &str, output: &[Value]) -> Value {
        json!({
            "id": self.id, "object": "response", "created_at": 0, "status": status,
            "model": self.model, "output": output, "parallel_tool_calls": true, "error": null
        })
    }

    fn start(&mut self) -> Vec<Value> {
        let response = self.response("in_progress", &[]);
        vec![
            self.stamp(json!({"type": "response.created", "response": response})),
            self.stamp(json!({"type": "response.in_progress", "response": response})),
        ]
    }

    fn close_active(&self, frames: &mut Vec<Value>) {
        let Some((i, item)) = self.items.iter().enumerate().next_back() else {
            return;
        };
        let OutputEvents {
            index,
            done,
            part_done,
            ..
        } = item.kind.events();
        frames.push(json!({"type": done, "item_id": item.id, "output_index": i, (index): 0, "text": item.text}));
        frames.push(json!({"type": part_done, "item_id": item.id, "output_index": i, (index): 0, "part": item.part(&item.text)}));
        frames.push(json!({"type": "response.output_item.done", "output_index": i, "item": item.item(true)}));
    }

    fn snapshot(&mut self, kind: OutputKind, text: &str, settled: bool) -> Result<Vec<Value>> {
        if text.is_empty() && !settled {
            return Ok(vec![]);
        }
        let mut rest = text;
        let mut exists = false;
        for item in self.items.iter().filter(|item| item.kind == kind) {
            exists = true;
            let Some(delta) = rest.strip_prefix(&item.text) else {
                if !settled {
                    return Ok(vec![]);
                }
                return Err(ProviderError::unavailable(
                    "chatgpt_stream_revised",
                    "ChatGPT revised text already delivered in the stream",
                ));
            };
            rest = delta;
        }
        if rest.is_empty() && exists {
            return Ok(vec![]);
        }
        let events = kind.events();
        let mut frames = vec![];
        // Codex associates live text with its one active output item.
        if self.items.last().is_none_or(|item| item.kind != kind) {
            self.close_active(&mut frames);
            let i = self.items.len();
            let item = LiveItem {
                kind,
                id: format!("{}_{}_{i}", events.prefix, self.id),
                text: String::new(),
            };
            let index = events.index;
            frames.push(json!({"type": "response.output_item.added", "output_index": i, "item": item.item(false)}));
            frames.push(json!({"type": events.added, "item_id": item.id, "output_index": i, (index): 0, "part": item.part("")}));
            self.items.push(item);
        }
        let output_index = self.items.len() - 1;
        let item = &mut self.items[output_index];
        if !rest.is_empty() {
            let index = events.index;
            frames.push(json!({
                "type": events.delta, "item_id": item.id, "output_index": output_index, (index): 0, "delta": rest
            }));
            item.text.push_str(rest);
        }
        Ok(frames.into_iter().map(|v| self.stamp(v)).collect())
    }

    fn finish(&mut self, tool_calls: &[ParsedToolCall]) -> (Vec<Value>, Value) {
        let mut frames = vec![];
        self.close_active(&mut frames);
        let mut output = self.items.iter().map(|item| item.item(true)).collect();
        append_tool_frames(&mut frames, &mut output, tool_calls);
        let final_resp = self.response("completed", &output);
        frames.push(json!({"type": "response.completed", "response": final_resp}));
        (
            frames.into_iter().map(|v| self.stamp(v)).collect(),
            final_resp,
        )
    }

    fn failed(&mut self, err: ProviderError) -> Value {
        let mut response = self.response("failed", &[]);
        response["error"] = json!({"code": err.code, "message": err.detail});
        self.stamp(json!({"type": "response.failed", "response": response}))
    }
}

fn append_tool_frames(
    frames: &mut Vec<Value>,
    output_items: &mut Vec<Value>,
    tool_calls: &[ParsedToolCall],
) {
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
            frames.push(
                json!({"type": "response.output_item.added", "output_index": idx, "item": init_item}),
            );
            frames.push(json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": item_id,
                "output_index": idx,
                "delta": tc.arguments
            }));
            frames.push(json!({
                "type": "response.custom_tool_call_input.done",
                "item_id": item_id,
                "output_index": idx,
                "input": tc.arguments
            }));
            frames.push(
                json!({"type": "response.output_item.done", "output_index": idx, "item": done_item}),
            );
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
            frames.push(
                json!({"type": "response.output_item.added", "output_index": idx, "item": init_item}),
            );
            frames.push(json!({
                "type": "response.function_call_arguments.delta",
                "item_id": item_id,
                "output_index": idx,
                "delta": tc.arguments
            }));
            frames.push(json!({
                "type": "response.function_call_arguments.done",
                "item_id": item_id,
                "output_index": idx,
                "arguments": tc.arguments
            }));
            frames.push(
                json!({"type": "response.output_item.done", "output_index": idx, "item": done_item}),
            );
            output_items.push(done_item);
        }
    }
}

async fn send_frames(tx: &mpsc::Sender<String>, frames: Vec<Value>) -> bool {
    for frame in frames {
        if !send_chunk(tx, format_sse_frame(&frame)).await {
            return false;
        }
    }
    true
}

// A slow or abandoned HTTP reader cannot hold the lane beyond the send deadline.
async fn send_chunk(tx: &mpsc::Sender<String>, chunk: String) -> bool {
    tokio::time::timeout(Duration::from_secs(30), tx.send(chunk))
        .await
        .is_ok_and(|result| result.is_ok())
}

pub use crate::config::ensure_private_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogModel {
    pub slug: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub max_tokens: Option<u64>,
}

pub fn catalog_models_from_status(st: &Value) -> Vec<CatalogModel> {
    let mut out = Vec::new();
    if let Some(arr) = st.get("models").and_then(Value::as_array) {
        for m in arr {
            let Some(slug) = m
                .get("slug")
                .and_then(Value::as_str)
                .or_else(|| m.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let title = m
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let description = m
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let max_tokens = m
                .get("max_tokens")
                .and_then(Value::as_u64)
                .filter(|&n| n > 0);
            out.push(CatalogModel {
                slug: slug.to_owned(),
                title,
                description,
                max_tokens,
            });
        }
    }
    out
}

pub fn fallback_catalog_models(model_cfg: &ModelConfig) -> Vec<CatalogModel> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let candidates = std::iter::once(model_cfg.default.as_str())
        .chain(model_cfg.available.iter().map(String::as_str));
    for slug in candidates {
        let trimmed = slug.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_owned()) {
            continue;
        }
        out.push(CatalogModel {
            slug: trimmed.to_owned(),
            title: None,
            description: None,
            max_tokens: None,
        });
    }
    out
}

fn is_hidden_catalog_slug(slug: &str) -> bool {
    let lower = slug.to_ascii_lowercase();
    lower.contains("reserved_gate")
        || lower.starts_with("chatgpt_alpha_model_external_access_")
        || lower == "auto"
}

fn is_non_thinking_slug(slug: &str) -> bool {
    let lower = slug.to_ascii_lowercase();
    lower.contains("instant") || lower == "research"
}

fn normalize_catalog_default_effort(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" | "zero" => "none",
        "minimal" | "min" | "low" => "low",
        "high" | "extended" => "high",
        "xhigh" => "xhigh",
        "max" | "ultra" => "max",
        _ => "medium",
    }
}

pub fn build_codex_model_catalog(models: &[CatalogModel], model_cfg: &ModelConfig) -> Value {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let default_effort = normalize_catalog_default_effort(&model_cfg.reasoning_effort);
    for m in models {
        let slug = m.slug.trim();
        if slug.is_empty() || !seen.insert(slug.to_owned()) {
            continue;
        }
        let priority = (entries.len() + 1) as i64;
        let display_name = m.title.as_deref().filter(|s| !s.is_empty()).unwrap_or(slug);
        let description = m
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("ChatGPT Desktop model ({slug})"));
        let visibility = if is_hidden_catalog_slug(slug) {
            "hide"
        } else {
            "list"
        };
        let (model_default_effort, supported_efforts) = if is_non_thinking_slug(slug) {
            ("none", json!([]))
        } else {
            (
                default_effort,
                json!([
                    { "effort": "none", "description": "Direct response without extended thinking" },
                    { "effort": "low", "description": "Lighter reasoning for fast replies" },
                    { "effort": "medium", "description": "Balanced speed and reasoning depth" },
                    { "effort": "high", "description": "Greater reasoning depth for complex tasks" },
                    { "effort": "xhigh", "description": "Extra high reasoning depth for hard tasks" },
                    { "effort": "max", "description": "Maximum ChatGPT reasoning depth" }
                ]),
            )
        };
        entries.push(json!({
            "slug": slug,
            "display_name": display_name,
            "description": description,
            "default_reasoning_level": model_default_effort,
            "supported_reasoning_levels": supported_efforts,
            "shell_type": "unified_exec",
            "visibility": visibility,
            "supported_in_api": true,
            "priority": priority,
            "base_instructions": "",
            "supports_reasoning_summaries": true,
            "default_reasoning_summary": "auto",
            "support_verbosity": false,
            "truncation_policy": { "mode": "tokens", "limit": 10000 },
            "context_window": m.max_tokens.unwrap_or(128_000),
            "effective_context_window_percent": 95,
            "input_modalities": ["text", "image"],
            "supports_parallel_tool_calls": true,
            "experimental_supported_tools": []
        }));
    }
    json!({ "models": entries })
}

pub fn sync_model_catalog_file(path: &Path, catalog: &Value) -> io::Result<()> {
    let mut encoded = serde_json::to_vec_pretty(catalog)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    encoded.push(b'\n');
    if let Ok(existing) = fs::read(path) {
        if existing == encoded {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent).map_err(|err| io::Error::other(err.to_string()))?;
    }
    let tmp = path.with_extension(format!(
        "json.tmp-{}-{}",
        std::process::id(),
        random_hex(4).map_err(|err| io::Error::other(err.to_string()))?
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    let write_res = file.write_all(&encoded).and_then(|()| file.sync_all());
    drop(file);
    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    Ok(())
}

pub fn sync_catalog_from_status(catalog_path: Option<&Path>, st: &Value, model_cfg: &ModelConfig) {
    let Some(path) = catalog_path else {
        return;
    };
    let models = catalog_models_from_status(st);
    if models.is_empty() {
        return;
    }
    let catalog = build_codex_model_catalog(&models, model_cfg);
    if let Err(err) = sync_model_catalog_file(path, &catalog) {
        eprintln!(
            "chatgpt-provider: warning: failed to update {}: {err}",
            path.display()
        );
    }
}

pub async fn handle_models(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    catalog_path: Option<&Path>,
) -> Json<Value> {
    let mut slugs: Vec<String> = Vec::new();
    if driver.host_running().await {
        if let Ok(st) = driver.status().await {
            sync_catalog_from_status(catalog_path, &st, model_cfg);
            slugs = catalog_models_from_status(&st)
                .into_iter()
                .map(|m| m.slug)
                .collect();
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
    threads: Arc<ResponseThreadMap>,
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

    let utility = is_utility_title_request(&body);
    let effective_project_id = if utility {
        None
    } else {
        history_cfg.project_id.clone()
    };
    let session_key = if utility {
        None
    } else {
        session_hint.or_else(|| {
            body.get("prompt_cache_key")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty() && s.len() <= 512)
                .map(ToOwned::to_owned)
        })
    };
    let session_key = session_key
        .filter(|key| key.len() <= 512)
        .map(|key| format!("session:{key}"));
    let _lease = session_key
        .as_deref()
        .map(|key| threads.lease(key))
        .transpose()?;
    let input_items = body.get("input").and_then(Value::as_array);
    let input_len = input_items.map_or(0, Vec::len);
    let current_digest = input_items
        .map(|items| input_prefix_digest(items, items.len()))
        .unwrap_or([0u8; 32]);

    let mut prior = None;
    let mut expected_account = None;
    let mut skip_prefix = 0;
    if let Some(entry) = session_key.as_deref().and_then(|key| threads.get(key)) {
        expected_account = entry.account_id.clone();
        if entry.project_id == effective_project_id
            && entry.input_len > 0
            && input_len >= entry.input_len
            && input_items.is_some_and(|items| {
                input_prefix_digest(items, entry.input_len) == entry.prefix_sha256
            })
        {
            if input_len == entry.input_len {
                return Err(ProviderError::bad_request(
                    "response_input_already_sent",
                    "no new input follows the committed ChatGPT response",
                ));
            }
            skip_prefix = entry.input_len;
            prior = Some(entry);
        }
    } else if session_key.is_none() && !utility {
        if let Some(rid) = body.get("previous_response_id").and_then(Value::as_str) {
            let entry = threads
                .get(&format!("response:{rid}"))
                .filter(|entry| entry.project_id == effective_project_id)
                .ok_or_else(|| {
                    ProviderError::bad_request(
                        "previous_response_unavailable",
                        "no continuation in the configured project",
                    )
                })?;
            expected_account = entry.account_id.clone();
            prior = Some(entry);
        }
    }

    let prompt = render_prompt_from_responses_request(&body, skip_prefix)?;
    let all_sigs = extract_local_function_signatures(&body);
    let resp_id = format!("resp_{}", random_hex(8)?);
    let mut live = LiveResponse::new(resp_id.clone(), model.clone());
    let lane_id = resp_id.clone();
    let send_payload = json!({
        "lane_id": lane_id,
        "chat_id": prior.as_ref().map(|entry| &entry.chat_id),
        "expected_account_id": expected_account,
        "verify_identity": true,
        "strict_project": true,
        "project_id": effective_project_id,
        "prompt": prompt,
        "model": model,
        "reasoning_effort": effort,
        "local_function_signatures": all_sigs,
        "is_continuation": skip_prefix > 0,
        "wait_ms": RESPONSES_WAIT_MS
    });

    if !stream {
        let mut cancel_guard = CancelTurnGuard::new(Arc::clone(&driver), lane_id);
        let driver_res = driver.send_message(send_payload).await?;
        cancel_guard.disarm();
        let (assistant_text, tool_calls, _) = parse_settled_driver_result(&driver_res, &all_sigs)?;
        record_thread_entries(
            &threads,
            &resp_id,
            session_key.as_deref(),
            &driver_res,
            effective_project_id,
            input_len,
            current_digest,
        )?;
        if !assistant_text.is_empty() || tool_calls.is_empty() {
            live.snapshot(OutputKind::Answer, &assistant_text, true)?;
        }
        return Ok((StatusCode::OK, Json(live.finish(&tool_calls).1)).into_response());
    }

    // Streaming mode: emit `response.created` + `response.in_progress` immediately, send
    // periodic SSE keepalive comments while waiting, and call `driver.stop_generation()`
    // if the client disconnects before settlement.
    let (tx, rx) = mpsc::channel::<String>(32);
    for frame in live.start() {
        let _ = tx.try_send(format_sse_frame(&frame));
    }
    tokio::spawn(async move {
        let _lease = _lease;
        let mut cancel = CancelTurnGuard::new(Arc::clone(&driver), lane_id);
        let (event_tx, mut event_rx) = mpsc::channel(32);
        let mut turn = driver.stream_message(send_payload, event_tx);
        let mut keepalive = tokio::time::interval_at(
            tokio::time::Instant::now() + SSE_KEEPALIVE_INTERVAL,
            SSE_KEEPALIVE_INTERVAL,
        );
        let idle = tokio::time::sleep(SSE_IDLE);
        let limit = tokio::time::sleep(SSE_LIMIT);
        tokio::pin!(idle, limit);
        let mut event_open = true;
        let outcome = loop {
            tokio::select! {
                biased;
                _ = tx.closed() => return,
                _ = &mut limit => break Err(ProviderError::unavailable("chatgpt_stream_limit", "response exceeded two hours")),
                _ = &mut idle => break Err(ProviderError::unavailable("chatgpt_stream_idle", "no upstream event within five minutes")),
                event = event_rx.recv(), if event_open => {
                    match event {
                        Some(event) => {
                            idle.as_mut().reset(tokio::time::Instant::now() + SSE_IDLE);
                            let frames = match event {
                                StreamUpdate::Activity => continue,
                                StreamUpdate::Reasoning(text) => live.snapshot(OutputKind::Reasoning, &text, false),
                                StreamUpdate::Answer(text) => live.snapshot(OutputKind::Answer, &text, false),
                            };
                            match frames {
                                Ok(frames) if !frames.is_empty() => {
                                    if !send_frames(&tx, frames).await { return; }
                                }
                                Ok(_) => {}
                                Err(err) => break Err(err),
                            }
                        }
                        None => event_open = false,
                    }
                }
                result = &mut turn => break result,
                _ = keepalive.tick() => {
                    if !send_chunk(&tx, ": keepalive\n\n".to_owned()).await { return; }
                }
            }
        };
        let result = async {
            let settled = outcome?;
            let (text, calls, _) = parse_settled_driver_result(&settled, &all_sigs)?;
            cancel.disarm();
            let summary = settled["reasoning_summary"].as_str().unwrap_or("");
            // Codex keeps reasoning in an expanded transcript. Preserve a
            // summary-only turn in the normal answer view as well.
            let answer = if text.is_empty() && calls.is_empty() {
                summary
            } else {
                &text
            };
            let mut frames = vec![];
            let had_summary = live
                .items
                .iter()
                .any(|item| item.kind == OutputKind::Reasoning);
            let had_answer = live
                .items
                .iter()
                .any(|item| item.kind == OutputKind::Answer);
            if !had_answer && (!summary.is_empty() || had_summary) {
                frames.extend(live.snapshot(OutputKind::Reasoning, summary, true)?);
            }
            if !answer.is_empty() || calls.is_empty() || had_answer {
                frames.extend(live.snapshot(OutputKind::Answer, answer, true)?);
            }
            if had_answer && (!summary.is_empty() || had_summary) {
                frames.extend(live.snapshot(OutputKind::Reasoning, summary, true)?);
            }
            record_thread_entries(
                &threads,
                &resp_id,
                session_key.as_deref(),
                &settled,
                effective_project_id,
                input_len,
                current_digest,
            )?;
            frames.extend(live.finish(&calls).0);
            Ok::<_, ProviderError>(frames)
        }
        .await;
        let frames = match result {
            Ok(frames) => frames,
            Err(err) => vec![live.failed(err)],
        };
        send_frames(&tx, frames).await;
    });

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/event-stream; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(MpscSseStream { rx }),
    )
        .into_response())
}

fn parse_settled_driver_result(
    driver_res: &Value,
    all_sigs: &[Value],
) -> Result<(String, Vec<ParsedToolCall>, Option<String>)> {
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
        .unwrap_or("")
        .to_owned();
    let tool_calls: Vec<ParsedToolCall> = driver_res
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| resolve_parsed_tool_call(v, all_sigs))
                .collect()
        })
        .unwrap_or_default();
    let chat_id = driver_res
        .get("chat_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);
    Ok((assistant_text, tool_calls, chat_id))
}

fn record_thread_entries(
    threads: &ResponseThreadMap,
    resp_id: &str,
    session_key: Option<&str>,
    settled: &Value,
    project_id: Option<String>,
    input_len: usize,
    prefix_sha256: [u8; 32],
) -> Result<()> {
    let Some(chat_id) = settled["chat_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256)
    else {
        return Ok(());
    };
    let account_id = settled["account_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .map(str::to_owned);
    if threads.store.is_some() && account_id.is_none() {
        return Err(ProviderError::internal(
            "response_thread_identity",
            "missing verified ChatGPT account",
        ));
    }
    let alias = ThreadEntry {
        chat_id: chat_id.to_owned(),
        account_id,
        project_id,
        input_len: 0,
        prefix_sha256: [0; 32],
    };
    let mut entries = vec![(format!("response:{resp_id}"), alias.clone())];
    if let Some(key) = session_key {
        entries.push((
            key.to_owned(),
            ThreadEntry {
                input_len,
                prefix_sha256,
                ..alias
            },
        ));
    }
    threads.record(entries)
}

#[cfg(test)]
mod deferred_tools_tests {
    use super::*;

    #[test]
    fn code_mode_signature_preserves_deferred_tool_discovery() {
        let description = format!(
            "{} Find deferred tools in ALL_TOOLS.",
            "ordinary tool description ".repeat(42)
        );
        let body = json!({"input":[{"type":"additional_tools","tools":[{
            "type":"namespace","name":"functions","tools":[
                {"type":"custom","name":"exec","description":description},
                {"type":"custom","name":"other","description":description}
            ]
        }]}]});
        let signatures = extract_local_function_signatures(&body);
        let code_mode = signatures[0]["description"].as_str().unwrap();
        assert!(code_mode.contains("text(ALL_TOOLS.filter("));
        assert!(code_mode.chars().count() <= MAX_TOOL_DESC_CHARS + 3);
        assert!(!signatures[1]["description"]
            .as_str()
            .unwrap()
            .contains("ALL_TOOLS"));
    }
}
