//! Type 1 OpenAI Responses API (`/v1/responses` & `/v1/models`) for `codex -p chatgpt`.
//!
//! Emits monotonic OpenAI Responses SSE frames (or non-streaming JSON) with
//! single-path in-app tool callbacks (`local_function_signatures`), supporting
//! both top-level `tools` and Codex `0.171.0+` `additional_tools` namespaces,
//! `function_call` and `custom_tool_call` items, and multi-round session delta
//! continuation via `thread-id` / `session-id` / `prompt_cache_key` / `previous_response_id`.
//! Never synthesizes private/hidden reasoning.

use crate::cdp::{lane_channel, StreamEvent, StreamItemKind};
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
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::convert::Infallible;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_RESPONSE_THREADS: usize = 256;
const NATIVE_THREAD_SCHEMA_VERSION: u32 = 3;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native: Option<NativeThreadBinding>,
}

// Minimal durable continuation state. No raw payloads or private reasoning are persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeCallBinding {
    call_id: String,
    message_id: String,
    recipient: String,
    is_custom: bool,
    name: String,
    #[serde(default)]
    namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeThreadBinding {
    response_id: String,
    remote_head: Option<String>,
    #[serde(default)]
    pending_calls: Vec<NativeCallBinding>,
    #[serde(default)]
    instructions_sha256: Option<[u8; 32]>,
    #[serde(default)]
    request_input_len: usize,
    #[serde(default)]
    request_input_sha256: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct NativeThreadFile {
    schema_version: u32,
    entries: VecDeque<(String, ThreadEntry)>,
}

#[derive(Default)]
struct ProjectCache {
    loaded: bool,
    /// Case-folded project name -> ChatGPT project (`gizmo_id`).
    by_lower_name: HashMap<String, String>,
}

#[derive(Default)]
pub struct ResponseThreadMap {
    entries: Mutex<VecDeque<(String, ThreadEntry)>>,
    store: Option<(PathBuf, File)>,
    active: Mutex<HashSet<String>>,
    projects: Mutex<ProjectCache>,
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

fn matches_utility_prompt(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("concise, single-line task title")
        || lower.contains("generate a concise title")
        || lower.contains("transcript title generation")
        || lower.contains("generate a short title")
        || lower.contains("title for this conversation")
        || lower.contains("memory writing agent")
        || lower.contains("memory consolidation")
}

/// Detect Codex background utility calls (Transcript Title Generation, Memory
/// Consolidation, Guardian/Auto-Review) that must not contend with the active
/// session lease, burn Thinking-model quota, or clutter normal ChatGPT projects.
fn is_background_utility_request(turn_meta: Option<&Value>, body: &Value) -> bool {
    if let Some(meta) = turn_meta.and_then(Value::as_object) {
        if let Some(kind) = meta.get("request_kind").and_then(Value::as_str) {
            let kind = kind.trim().to_ascii_lowercase();
            if !kind.is_empty() && kind != "turn" {
                return true;
            }
        }
        for key in ["thread_source", "turn_trigger"] {
            if let Some(val) = meta.get(key).and_then(Value::as_str) {
                let val = val.trim().to_ascii_lowercase();
                if val == "memory_consolidation"
                    || val.starts_with("memory_")
                    || val.starts_with("title")
                    || val == "auto_review"
                    || val == "guardian"
                {
                    return true;
                }
            }
        }
    }
    if let Some(inst) = body.get("instructions").and_then(Value::as_str) {
        if matches_utility_prompt(inst) {
            return true;
        }
    }
    if let Some(items) = body.get("input").and_then(Value::as_array) {
        // Check the last user/developer prompt (Codex 0.171.0 title_gen sends
        // developer + AGENTS.md + title prompt = 3-4 items, sometimes with tools).
        for item in items.iter().rev().take(4) {
            let text = extract_content_text(item.get("content"));
            if matches_utility_prompt(&text) {
                return true;
            }
        }
    }
    false
}

/// Detect Codex's automatic machine-generated `role: "user"` preambles
/// (`# AGENTS.md instructions ...`, `<environment_context>`, `<collaboration_mode>`,
/// `<turn_aborted>`). Kept as `role: "user"` (zero privilege escalation) but marked
/// `hidden: true` so ChatGPT UI shows only the human's actual prompt.
fn is_codex_machine_user_preamble(parts: &[String]) -> bool {
    let Some(first) = parts.first().map(|s| s.trim_start()) else {
        return false;
    };
    first.starts_with("# AGENTS.md instructions")
        || first.starts_with("<environment_context>")
        || first.starts_with("<collaboration_mode>")
        || first.starts_with("<turn_aborted>")
}

/// Extract the workspace `cwd` from Codex's `<environment_context>` or turn metadata.
fn extract_workspace_cwd(body: &Value, turn_meta: Option<&Value>) -> Option<String> {
    if let Some(cwd) = turn_meta
        .and_then(|m| m.get("cwd"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(cwd.to_owned());
    }
    let items = body.get("input").and_then(Value::as_array)?;
    for item in items {
        let text = extract_content_text(item.get("content"));
        if let Some(start) = text.find("<cwd>") {
            let rest = &text[start + 5..];
            if let Some(end) = rest.find("</cwd>") {
                let cwd = rest[..end].trim();
                if !cwd.is_empty() {
                    return Some(cwd.to_owned());
                }
            }
        }
    }
    None
}

/// Derive a clean ChatGPT Project name from `cwd`, collapsing temp/home paths to `"Codex"`.
fn derive_auto_project_name(cwd: Option<&str>) -> String {
    let Some(raw) = cwd.map(str::trim).filter(|s| !s.is_empty()) else {
        return "Codex".to_owned();
    };
    let trimmed = raw.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "Codex".to_owned();
    }
    let home = std::env::var("HOME").ok();
    let prefix_tmp = std::env::var("PREFIX")
        .ok()
        .map(|p| format!("{}/tmp", p.trim_end_matches('/')));
    if home.as_deref().map(|h| h.trim_end_matches('/')) == Some(trimmed)
        || trimmed == "/tmp"
        || trimmed.starts_with("/tmp/")
        || prefix_tmp
            .as_deref()
            .is_some_and(|t| trimmed == t || trimmed.starts_with(&format!("{t}/")))
    {
        return "Codex".to_owned();
    }
    let base = Path::new(trimmed)
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('.'));
    match base {
        Some(name) => name.chars().take(64).collect(),
        None => "Codex".to_owned(),
    }
}

/// Build slim wire messages for background utility calls (omits 31KB Codex developer
/// boilerplate and 7.5KB AGENTS.md so title/memory generation stays fast and tiny).
fn build_utility_wire_messages(body: &Value) -> Result<Vec<Value>> {
    let mut messages = Vec::new();
    match body.get("input") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            messages.push(json!({
                "id": native_wire_id()?,
                "role": "user",
                "parts": [s],
            }));
        }
        Some(Value::Array(items)) => {
            for item in items {
                let role = item.get("role").and_then(Value::as_str).unwrap_or("user");
                if !matches!(role, "user" | "developer") {
                    continue;
                }
                let text = extract_content_text(item.get("content"));
                if text.trim().is_empty() {
                    continue;
                }
                // Drop Codex's heavy session boilerplate from utility calls.
                let trimmed = text.trim_start();
                if trimmed.starts_with("# AGENTS.md instructions")
                    || trimmed.starts_with("<environment_context>")
                    || trimmed.starts_with("<skills_instructions>")
                    || trimmed.starts_with("<permissions instructions>")
                {
                    continue;
                }
                if role == "developer" && !matches_utility_prompt(trimmed) {
                    continue;
                }
                messages.push(json!({
                    "id": native_wire_id()?,
                    "role": role,
                    "parts": [text],
                }));
            }
        }
        _ => {}
    }
    if let Some(inst) = body
        .get("instructions")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        messages.push(json!({
            "id": native_wire_id()?,
            "role": "developer",
            "parts": [inst],
            "one_turn": true,
        }));
    }
    if messages.is_empty() {
        return Err(ProviderError::bad_request(
            "responses_input_required",
            "utility request had no usable prompt text",
        ));
    }
    Ok(messages)
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
    async fn cancel_now(&mut self) {
        if self.armed {
            self.armed = false;
            let _ = tokio::time::timeout(
                Duration::from_secs(5),
                self.driver
                    .stop_generation(json!({ "lane_id": self.lane_id })),
            )
            .await;
        }
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

    fn current_native_response_key(&self, response_id: &str) -> Option<String> {
        let guard = self.entries.lock().ok()?;
        guard
            .iter()
            .find(|(_, e)| {
                e.native
                    .as_ref()
                    .is_some_and(|n| n.response_id == response_id)
            })
            .map(|(k, _)| k.clone())
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
                    // Seamlessly accept both v3 NativeThreadFile and legacy array files;
                    // drop any incomplete entries with empty chat_id left by prior runs.
                    let loaded: VecDeque<(String, ThreadEntry)> =
                        if let Ok(disk) = serde_json::from_slice::<NativeThreadFile>(&data) {
                            if disk.schema_version != NATIVE_THREAD_SCHEMA_VERSION {
                                return Err(io::Error::other(
                                    "unsupported response thread schema version",
                                ));
                            }
                            disk.entries
                        } else {
                            serde_json::from_slice(&data).map_err(io::Error::other)?
                        };
                    loaded
                        .into_iter()
                        .filter(|(_, e)| !e.chat_id.is_empty())
                        .take(MAX_RESPONSE_THREADS)
                        .collect()
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

    /// Resolve a ChatGPT Project ID by name (case-insensitive), caching `list_projects`
    /// once per process and creating the project if absent (with optional `memory_scope`).
    async fn resolve_project_by_name(
        &self,
        driver: &Arc<dyn DriverBackend>,
        name: &str,
        memory_scope: Option<&str>,
    ) -> Option<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();
        let needs_load = {
            let guard = self.projects.lock().ok()?;
            if let Some(id) = guard.by_lower_name.get(&lower) {
                return Some(id.clone());
            }
            !guard.loaded
        };
        if needs_load {
            if let Ok(res) = driver.list_projects(json!({ "limit": 100 })).await {
                if res.get("state").and_then(Value::as_str) == Some("ok") {
                    if let Ok(mut guard) = self.projects.lock() {
                        if let Some(arr) = res.get("projects").and_then(Value::as_array) {
                            for p in arr {
                                let pid = p
                                    .get("project_id")
                                    .or_else(|| p.get("id"))
                                    .and_then(Value::as_str)
                                    .filter(|s| !s.is_empty());
                                let pname = p
                                    .get("name")
                                    .or_else(|| p.get("title"))
                                    .and_then(Value::as_str)
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty());
                                if let (Some(pid), Some(pname)) = (pid, pname) {
                                    guard
                                        .by_lower_name
                                        .insert(pname.to_ascii_lowercase(), pid.to_owned());
                                }
                            }
                        }
                        guard.loaded = true;
                        if let Some(id) = guard.by_lower_name.get(&lower) {
                            return Some(id.clone());
                        }
                    }
                }
            }
        }
        let mut req = json!({ "name": trimmed, "instructions": "" });
        if let Some(scope) = memory_scope {
            req["memory_scope"] = json!(scope);
        }
        let created = driver.create_project(req).await.ok()?;
        let pid = created
            .get("project_id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())?
            .to_owned();
        if let Ok(mut guard) = self.projects.lock() {
            guard.by_lower_name.insert(lower, pid.clone());
            guard.loaded = true;
        }
        Some(pid)
    }

    fn record(&self, entries: impl IntoIterator<Item = (String, ThreadEntry)>) -> Result<()> {
        self.update(|next| {
            for (key, entry) in entries {
                if let Some(pos) = next.iter().position(|(existing, _)| existing == &key) {
                    next.remove(pos);
                }
                next.push_back((key, entry));
                while next.len() > MAX_RESPONSE_THREADS {
                    next.pop_front();
                }
            }
            Ok(())
        })
    }

    fn update(
        &self,
        change: impl FnOnce(&mut VecDeque<(String, ThreadEntry)>) -> Result<()>,
    ) -> Result<()> {
        let mut guard = self.entries.lock().map_err(|_| {
            ProviderError::internal("response_thread_map_write", "thread map lock poisoned")
        })?;
        let mut next = guard.clone();
        change(&mut next)?;
        if let Some((path, _)) = &self.store {
            let bytes = serde_json::to_vec(&NativeThreadFile {
                schema_version: NATIVE_THREAD_SCHEMA_VERSION,
                entries: next.clone(),
            })
            .map_err(|err| ProviderError::internal("response_thread_map_write", err.to_string()))?;
            if bytes.len() > 1_048_576 {
                return Err(ProviderError::unavailable(
                    "response_thread_map_full",
                    "response thread map exceeds 1 MiB",
                ));
            }
            let tmp = path.with_extension(format!("tmp-{}", random_hex(8)?));
            let write = || -> io::Result<()> {
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&tmp)?;
                let mut writer = io::BufWriter::new(file);
                writer.write_all(&bytes)?;
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
    native_message_id: Option<String>,
    native_recipient: Option<String>,
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
    let mut raw_sigs = Vec::new();
    if let Some(arr) = body.get("tools").and_then(Value::as_array) {
        collect_tools_from_array(arr, None, &mut raw_sigs);
    }
    if let Some(items) = body.get("input").and_then(Value::as_array) {
        for item in items {
            if item.get("type").and_then(Value::as_str) == Some("additional_tools") {
                if let Some(arr) = item.get("tools").and_then(Value::as_array) {
                    collect_tools_from_array(arr, None, &mut raw_sigs);
                }
            }
        }
    }
    let mut sigs = Vec::with_capacity(raw_sigs.len());
    let mut seen: HashSet<(Option<String>, String)> = HashSet::new();
    for sig in raw_sigs {
        let key = (
            sig.get("namespace")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            sig.get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        );
        if seen.insert(key) {
            sigs.push(sig);
        }
    }
    sigs
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

    let native = raw
        .get("native_message_id")
        .and_then(Value::as_str)
        .is_some();
    let matched = if native {
        let mut exact = sigs.iter().filter(|s| {
            s.get("name").and_then(Value::as_str) == Some(name.as_str())
                && s.get("namespace").and_then(Value::as_str) == namespace.as_deref()
        });
        let first = exact.next()?;
        let first_type = first.get("type").and_then(Value::as_str);
        if exact.any(|other| other.get("type").and_then(Value::as_str) != first_type) {
            return None;
        }
        Some(first)
    } else {
        matched
    };
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
    if native {
        let parsed = serde_json::from_str::<Value>(&arguments).ok()?;
        let obj = parsed.as_object()?;
        if is_custom {
            if obj.len() != 1 {
                return None;
            }
            arguments = obj.get("input")?.as_str()?.to_owned();
        }
    } else if is_custom && arguments.trim_start().starts_with('{') {
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
        native_message_id: raw
            .get("native_message_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        native_recipient: raw
            .get("native_recipient")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentState {
    Mutable,
    Committed,
}

#[derive(Debug, Clone)]
enum AssembledContent {
    ReasoningSummary(String),
    Commentary(String),
    Answer(String),
    ToolCall(ParsedToolCall),
}

#[derive(Debug, Clone)]
struct AssembledItem {
    id: String,
    state: ContentState,
    content: AssembledContent,
    emitted: bool,
}

#[derive(Default)]
struct TurnAssembly {
    items: Vec<AssembledItem>,
    seen_tool_calls: HashSet<String>,
    committed_commentaries: HashSet<String>,
    last_committed_commentary: Option<String>,
    turn_completed: bool,
}

impl TurnAssembly {
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

    fn upsert_text(&mut self, id: String, kind: StreamItemKind, mut text: String, committed: bool) {
        if text.is_empty() && !committed {
            self.remove_mutable(&id);
            return;
        }
        let next_state = if committed {
            ContentState::Committed
        } else {
            ContentState::Mutable
        };
        if committed {
            match kind {
                StreamItemKind::Commentary => {
                    let norm = text.trim().to_owned();
                    if norm.is_empty() || !self.committed_commentaries.insert(norm.clone()) {
                        self.remove_mutable(&id);
                        return;
                    }
                    self.last_committed_commentary = Some(norm);
                }
                StreamItemKind::Answer => {
                    if let Some(trimmed_answer) = self.strip_duplicate_commentary_prefix(&text) {
                        text = trimmed_answer;
                    }
                }
                StreamItemKind::ReasoningSummary => {}
            }
        }
        let next_content = match kind {
            StreamItemKind::ReasoningSummary => AssembledContent::ReasoningSummary(text),
            StreamItemKind::Commentary => AssembledContent::Commentary(text),
            StreamItemKind::Answer => AssembledContent::Answer(text),
        };
        if let Some(existing) = self.items.iter_mut().find(|item| item.id == id) {
            if existing.state == ContentState::Committed {
                return;
            }
            existing.content = next_content;
            existing.state = next_state;
            return;
        }
        self.items.push(AssembledItem {
            id,
            state: next_state,
            content: next_content,
            emitted: false,
        });
    }

    fn remove_mutable(&mut self, id: &str) {
        self.items
            .retain(|item| item.id != id || item.state == ContentState::Committed);
    }

    fn push_committed_tool(&mut self, tc: ParsedToolCall) {
        if !self.seen_tool_calls.insert(tc.call_id.clone()) {
            return;
        }
        self.items.push(AssembledItem {
            id: format!("tool:{}", tc.call_id),
            state: ContentState::Committed,
            content: AssembledContent::ToolCall(tc),
            emitted: false,
        });
    }

    fn apply_event(&mut self, event: StreamEvent, all_sigs: &[Value]) {
        match event {
            StreamEvent::Activity => {}
            StreamEvent::ItemUpsert {
                item_id,
                item_kind,
                text,
                committed,
            } => self.upsert_text(item_id, item_kind, text, committed),
            StreamEvent::ItemRemove { item_id } => self.remove_mutable(&item_id),
            StreamEvent::ToolCall {
                id,
                name,
                namespace,
                arguments,
                native_message_id,
                native_recipient,
            } => {
                let raw = json!({
                    "id": id,
                    "name": name,
                    "namespace": namespace,
                    "arguments": arguments,
                    "native_message_id": native_message_id,
                    "native_recipient": native_recipient,
                });
                if let Some(tc) = resolve_parsed_tool_call(&raw, all_sigs) {
                    self.push_committed_tool(tc);
                }
            }
            StreamEvent::TurnCompleted { .. } => {
                self.turn_completed = true;
            }
        }
    }

    fn reconcile_settled(&mut self, driver_res: &Value, all_sigs: &[Value]) -> Result<()> {
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

        // Discard any remaining mutable drafts before committing settled items.
        self.items
            .retain(|item| item.state == ContentState::Committed);
        let committed_ids: HashSet<String> =
            self.items.iter().map(|item| item.id.clone()).collect();

        if let Some(items) = driver_res.get("items").and_then(Value::as_array) {
            for entry in items {
                let kind = entry.get("kind").and_then(Value::as_str).unwrap_or("");
                if kind == "tool_call" {
                    let raw = json!({
                        "id": entry.get("call_id").or_else(|| entry.get("id")),
                        "name": entry.get("name"),
                        "namespace": entry.get("namespace"),
                        "arguments": entry.get("arguments"),
                        "native_message_id": entry.get("native_message_id"),
                        "native_recipient": entry.get("native_recipient"),
                    });
                    if let Some(tc) = resolve_parsed_tool_call(&raw, all_sigs) {
                        self.push_committed_tool(tc);
                    }
                    continue;
                }
                let text = entry.get("text").and_then(Value::as_str).unwrap_or("");
                if text.is_empty() {
                    continue;
                }
                let item_id = entry
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("{kind}_{}", self.items.len()));
                if committed_ids.contains(&item_id)
                    || self.items.iter().any(|existing| existing.id == item_id)
                {
                    continue;
                }
                let stream_kind = match kind {
                    "reasoning_summary" => StreamItemKind::ReasoningSummary,
                    "commentary" => StreamItemKind::Commentary,
                    "answer" => StreamItemKind::Answer,
                    _ => continue,
                };
                self.upsert_text(item_id, stream_kind, text.to_owned(), true);
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
                    true,
                );
            }
            let assistant_text = driver_res
                .get("assistant_text")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !assistant_text.is_empty()
                && !self
                    .items
                    .iter()
                    .any(|item| matches!(item.content, AssembledContent::Answer(_)))
            {
                self.upsert_text(
                    "settled_answer".to_owned(),
                    StreamItemKind::Answer,
                    assistant_text.to_owned(),
                    true,
                );
            }
            if let Some(arr) = driver_res.get("tool_calls").and_then(Value::as_array) {
                for raw in arr {
                    if let Some(tc) = resolve_parsed_tool_call(raw, all_sigs) {
                        self.push_committed_tool(tc);
                    }
                }
            }
        }

        let has_reply_or_tool = self.items.iter().any(|item| {
            matches!(
                item.content,
                AssembledContent::Answer(_)
                    | AssembledContent::Commentary(_)
                    | AssembledContent::ToolCall(_)
            )
        });
        if !has_reply_or_tool {
            let summary_text: String = self
                .items
                .iter()
                .filter_map(|item| match &item.content {
                    AssembledContent::ReasoningSummary(t) if !t.is_empty() => Some(t.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            // Codex keeps reasoning in an expanded transcript. Preserve a
            // summary-only turn in the normal answer view as well.
            self.upsert_text(
                "settled_answer_fallback".to_owned(),
                StreamItemKind::Answer,
                summary_text,
                true,
            );
        }

        self.turn_completed = true;
        Ok(())
    }
}

struct OpenReasoningItem {
    id: String,
    output_index: usize,
    parts: Vec<Value>,
}

struct ResponsesEncoder {
    id: String,
    model: String,
    parallel_tool_calls: bool,
    sequence: u64,
    completed_output: Vec<Value>,
    open_reasoning: Option<OpenReasoningItem>,
}

impl ResponsesEncoder {
    fn new(id: String, model: String) -> Self {
        Self {
            id,
            model,
            parallel_tool_calls: true,
            sequence: 0,
            completed_output: vec![],
            open_reasoning: None,
        }
    }

    fn next_output_index(&self) -> usize {
        self.completed_output.len() + usize::from(self.open_reasoning.is_some())
    }

    fn stamp(&mut self, mut frame: Value) -> Value {
        frame["sequence_number"] = json!(self.sequence);
        self.sequence += 1;
        frame
    }

    fn response(&self, status: &str, output: &[Value]) -> Value {
        json!({
            "id": self.id, "object": "response", "created_at": 0, "status": status,
            "model": self.model, "output": output, "parallel_tool_calls": self.parallel_tool_calls, "error": null
        })
    }

    fn start(&mut self) -> Vec<Value> {
        let response = self.response("in_progress", &[]);
        vec![
            self.stamp(json!({"type": "response.created", "response": response})),
            self.stamp(json!({"type": "response.in_progress", "response": response})),
        ]
    }

    fn close_open_reasoning(&mut self, frames: &mut Vec<Value>) {
        let Some(open) = self.open_reasoning.take() else {
            return;
        };
        let done_item = json!({
            "id": open.id,
            "type": "reasoning",
            "summary": open.parts,
            "status": "completed"
        });
        frames.push(json!({
            "type": "response.output_item.done",
            "output_index": open.output_index,
            "item": done_item
        }));
        self.completed_output.push(done_item);
    }

    fn emit_message_item(&mut self, frames: &mut Vec<Value>, phase: &str, text: &str) {
        let rendered = crate::presentation::render_for_codex_tui(text);
        let text = rendered.as_str();
        let idx = self.next_output_index();
        let item_id = format!("msg_{}_{idx}", self.id);
        let init_part = json!({"type": "output_text", "text": "", "annotations": []});
        let done_part = json!({"type": "output_text", "text": text, "annotations": []});
        let init_item = json!({
            "id": item_id, "type": "message", "role": "assistant", "status": "in_progress",
            "phase": phase, "content": [init_part]
        });
        let done_item = json!({
            "id": item_id, "type": "message", "role": "assistant", "status": "completed",
            "phase": phase, "content": [done_part]
        });
        frames.push(
            json!({"type": "response.output_item.added", "output_index": idx, "item": init_item}),
        );
        frames.push(json!({
            "type": "response.content_part.added",
            "item_id": item_id,
            "output_index": idx,
            "content_index": 0,
            "part": init_part
        }));
        if !text.is_empty() {
            frames.push(json!({
                "type": "response.output_text.delta",
                "item_id": item_id,
                "output_index": idx,
                "content_index": 0,
                "delta": text
            }));
        }
        frames.push(json!({
            "type": "response.output_text.done",
            "item_id": item_id,
            "output_index": idx,
            "content_index": 0,
            "text": text
        }));
        frames.push(json!({
            "type": "response.content_part.done",
            "item_id": item_id,
            "output_index": idx,
            "content_index": 0,
            "part": done_part
        }));
        frames.push(
            json!({"type": "response.output_item.done", "output_index": idx, "item": done_item}),
        );
        self.completed_output.push(done_item);
    }

    fn flush_committed(&mut self, assembly: &mut TurnAssembly, allow_tools: bool) -> Vec<Value> {
        let mut raw_frames = vec![];
        for item in &mut assembly.items {
            if item.emitted || item.state != ContentState::Committed {
                continue;
            }
            // Do not expose a tool to Codex before its original remote binding
            // is durable. Preserve ordering by also deferring following items.
            if !allow_tools && matches!(item.content, AssembledContent::ToolCall(_)) {
                break;
            }
            item.emitted = true;
            match &item.content {
                AssembledContent::ReasoningSummary(text) => {
                    if self.open_reasoning.is_none() {
                        let idx = self.next_output_index();
                        let rs_id = format!("rs_{}_{idx}", self.id);
                        raw_frames.push(json!({
                            "type": "response.output_item.added",
                            "output_index": idx,
                            "item": {
                                "id": rs_id,
                                "type": "reasoning",
                                "summary": [],
                                "status": "in_progress"
                            }
                        }));
                        self.open_reasoning = Some(OpenReasoningItem {
                            id: rs_id,
                            output_index: idx,
                            parts: vec![],
                        });
                    }
                    let open = self.open_reasoning.as_mut().expect("reasoning item opened");
                    let summary_index = open.parts.len();
                    let empty_part = json!({"type": "summary_text", "text": ""});
                    let done_part = json!({"type": "summary_text", "text": text});
                    raw_frames.push(json!({
                        "type": "response.reasoning_summary_part.added",
                        "item_id": open.id,
                        "output_index": open.output_index,
                        "summary_index": summary_index,
                        "part": empty_part
                    }));
                    if !text.is_empty() {
                        raw_frames.push(json!({
                            "type": "response.reasoning_summary_text.delta",
                            "item_id": open.id,
                            "output_index": open.output_index,
                            "summary_index": summary_index,
                            "delta": text
                        }));
                    }
                    raw_frames.push(json!({
                        "type": "response.reasoning_summary_text.done",
                        "item_id": open.id,
                        "output_index": open.output_index,
                        "summary_index": summary_index,
                        "text": text
                    }));
                    raw_frames.push(json!({
                        "type": "response.reasoning_summary_part.done",
                        "item_id": open.id,
                        "output_index": open.output_index,
                        "summary_index": summary_index,
                        "part": done_part
                    }));
                    open.parts.push(done_part);
                }
                AssembledContent::Commentary(text) => {
                    self.close_open_reasoning(&mut raw_frames);
                    self.emit_message_item(&mut raw_frames, "commentary", text);
                }
                AssembledContent::Answer(text) => {
                    self.close_open_reasoning(&mut raw_frames);
                    self.emit_message_item(&mut raw_frames, "final_answer", text);
                }
                AssembledContent::ToolCall(tc) => {
                    self.close_open_reasoning(&mut raw_frames);
                    append_tool_frames(
                        &mut raw_frames,
                        &mut self.completed_output,
                        std::slice::from_ref(tc),
                    );
                }
            }
        }
        raw_frames.into_iter().map(|v| self.stamp(v)).collect()
    }

    fn finish(&mut self, assembly: &mut TurnAssembly) -> (Vec<Value>, Value) {
        let mut stamped = self.flush_committed(assembly, true);
        let mut closing = vec![];
        self.close_open_reasoning(&mut closing);
        let final_resp = self.response("completed", &self.completed_output);
        closing.push(json!({"type": "response.completed", "response": final_resp}));
        for frame in closing {
            stamped.push(self.stamp(frame));
        }
        (stamped, final_resp)
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

pub type LiveModelSet = Arc<RwLock<HashSet<String>>>;

pub fn new_live_model_set<I>(initial: I, catalog_path: Option<&Path>) -> LiveModelSet
where
    I: IntoIterator<Item = String>,
{
    let mut slugs: HashSet<String> = initial
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(path) = catalog_path {
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(doc) = serde_json::from_str::<Value>(&raw) {
                if let Some(arr) = doc.get("models").and_then(Value::as_array) {
                    for entry in arr {
                        if let Some(slug) = entry.get("slug").and_then(Value::as_str) {
                            let trimmed = slug.trim();
                            if !trimmed.is_empty() {
                                slugs.insert(trimmed.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    Arc::new(RwLock::new(slugs))
}

fn update_live_models(set: Option<&LiveModelSet>, models: &[CatalogModel]) {
    let Some(lock) = set else {
        return;
    };
    let fresh: HashSet<String> = models
        .iter()
        .map(|m| m.slug.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if fresh.is_empty() {
        return;
    }
    if let Ok(mut guard) = lock.write() {
        *guard = fresh;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogModel {
    pub slug: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub max_tokens: Option<u64>,
    pub tags: Vec<String>,
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
            let tags = m
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            out.push(CatalogModel {
                slug: slug.to_owned(),
                title,
                description,
                max_tokens,
                tags,
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
            tags: Vec::new(),
        });
    }
    out
}

fn is_codex_only_slug(slug: &str) -> bool {
    let lower = slug.to_ascii_lowercase();
    lower.ends_with("-wm") || lower.contains("-codex") || lower.starts_with("codex-")
}

fn is_hidden_catalog_slug(slug: &str, tags: &[String]) -> bool {
    let lower = slug.to_ascii_lowercase();
    tags.iter().any(|t| t.eq_ignore_ascii_case("hidden"))
        || lower.contains("reserved_gate")
        || lower.starts_with("chatgpt_alpha_model_external_access_")
        || lower == "auto"
        || is_codex_only_slug(slug)
}

fn canonical_family_slug(slug: &str) -> String {
    let lower = slug.trim().to_ascii_lowercase();
    if let Some(base) = lower.strip_suffix("-instant") {
        return base.to_owned();
    }
    if let Some(base) = lower.strip_suffix("-thinking") {
        return base.to_owned();
    }
    if let Some(base) = lower.strip_suffix("-t-mini") {
        return format!("{base}-mini");
    }
    lower
}

struct ParsedGptFamily {
    major: u32,
    minor: u32,
    patch: u32,
    minor_present: bool,
    tier: Option<String>,
}

fn parse_gpt_family(family_slug: &str) -> Option<ParsedGptFamily> {
    let rest = family_slug.strip_prefix("gpt-")?;
    let mut nums: Vec<u32> = Vec::new();
    let mut tier_parts: Vec<&str> = Vec::new();
    let mut in_tier = false;
    for part in rest.split('-') {
        if !in_tier {
            let parsed: Option<Vec<u32>> = if part.is_empty() {
                None
            } else {
                part.split('.')
                    .map(|seg| {
                        if !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()) {
                            seg.parse::<u32>().ok()
                        } else {
                            None
                        }
                    })
                    .collect()
            };
            if let Some(seg_nums) = parsed {
                nums.extend(seg_nums);
                continue;
            }
            in_tier = true;
        }
        if !part.is_empty() {
            tier_parts.push(part);
        }
    }
    let &major = nums.first()?;
    let minor_present = nums.len() > 1;
    let minor = nums.get(1).copied().unwrap_or(0);
    let patch = nums.get(2).copied().unwrap_or(0);
    let tier = if tier_parts.is_empty() {
        None
    } else {
        Some(tier_parts.join("-"))
    };
    Some(ParsedGptFamily {
        major,
        minor,
        patch,
        minor_present,
        tier,
    })
}

fn capitalize_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn canonical_display_name(family_slug: &str, raw_title: Option<&str>) -> String {
    if let Some(parsed) = parse_gpt_family(family_slug) {
        let mut base = if parsed.patch > 0 {
            format!("GPT-{}.{}.{}", parsed.major, parsed.minor, parsed.patch)
        } else if parsed.minor_present {
            format!("GPT-{}.{}", parsed.major, parsed.minor)
        } else {
            format!("GPT-{}", parsed.major)
        };
        if let Some(tier) = parsed.tier.as_deref() {
            match tier {
                "pro" => base.push_str(" Pro"),
                "mini" => base.push_str(" Mini"),
                other => {
                    for word in other.split('-') {
                        if !word.is_empty() {
                            base.push(' ');
                            base.push_str(&capitalize_word(word));
                        }
                    }
                }
            }
        }
        return base;
    }
    raw_title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| family_slug.to_owned())
}

fn family_sort_key(
    family_slug: &str,
) -> (
    u8,
    std::cmp::Reverse<u32>,
    std::cmp::Reverse<u32>,
    std::cmp::Reverse<u32>,
    u8,
    &str,
) {
    use std::cmp::Reverse;
    if let Some(parsed) = parse_gpt_family(family_slug) {
        let tier_rank = match parsed.tier.as_deref() {
            None => 0,
            Some("pro") => 1,
            Some("mini") => 2,
            Some(_) => 3,
        };
        (
            0,
            Reverse(parsed.major),
            Reverse(parsed.minor),
            Reverse(parsed.patch),
            tier_rank,
            family_slug,
        )
    } else {
        (1, Reverse(0), Reverse(0), Reverse(0), 0, family_slug)
    }
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

fn standard_reasoning_levels(include_none: bool) -> Value {
    let mut levels = Vec::new();
    if include_none {
        levels.push(
            json!({ "effort": "none", "description": "Direct response without extended thinking" }),
        );
    }
    levels.extend([
        json!({ "effort": "low", "description": "Lighter reasoning for fast replies" }),
        json!({ "effort": "medium", "description": "Balanced speed and reasoning depth" }),
        json!({ "effort": "high", "description": "Greater reasoning depth for complex tasks" }),
        json!({ "effort": "xhigh", "description": "Extra high reasoning depth for hard tasks" }),
        json!({ "effort": "max", "description": "Maximum ChatGPT reasoning depth" }),
    ]);
    Value::Array(levels)
}

struct CatalogEntrySpec<'a> {
    slug: &'a str,
    display_name: &'a str,
    description: &'a str,
    default_effort: &'a str,
    supported_efforts: Value,
    visibility: &'a str,
    priority: i64,
    max_tokens: Option<u64>,
}

fn make_catalog_entry(spec: CatalogEntrySpec<'_>) -> Value {
    json!({
        "slug": spec.slug,
        "display_name": spec.display_name,
        "description": spec.description,
        "default_reasoning_level": spec.default_effort,
        "supported_reasoning_levels": spec.supported_efforts,
        "shell_type": "unified_exec",
        "visibility": spec.visibility,
        "supported_in_api": true,
        "priority": spec.priority,
        "base_instructions": "",
        "supports_reasoning_summaries": true,
        "supports_reasoning_summary_parameter": true,
        "default_reasoning_summary": "auto",
        "support_verbosity": false,
        "truncation_policy": { "mode": "tokens", "limit": 10000 },
        "context_window": spec.max_tokens.unwrap_or(128_000),
        "effective_context_window_percent": 95,
        "input_modalities": ["text", "image"],
        "experimental_supported_tools": []
    })
}

pub fn resolve_dispatch_model_and_effort(
    requested_model: &str,
    requested_effort: Option<&str>,
    known_slugs: &HashSet<String>,
) -> (String, Option<String>) {
    let model = requested_model.trim();
    let lower = model.to_ascii_lowercase();
    let clean_effort = requested_effort
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    if lower.ends_with("-instant")
        || lower.ends_with("-thinking")
        || lower.ends_with("-t-mini")
        || lower.ends_with("-pro")
        || lower.ends_with("-wm")
        || matches!(lower.as_str(), "auto" | "research")
    {
        return (model.to_owned(), clean_effort);
    }

    let effort_key = clean_effort.as_deref().map(str::to_ascii_lowercase);
    let is_none = matches!(effort_key.as_deref(), Some("none" | "zero"));
    let is_thinking = matches!(
        effort_key.as_deref(),
        Some(
            "minimal"
                | "min"
                | "low"
                | "medium"
                | "standard"
                | "high"
                | "extended"
                | "xhigh"
                | "max"
                | "ultra"
        )
    );

    if let Some(prefix) = lower.strip_suffix("-mini") {
        if is_none {
            let target = if known_slugs.is_empty() || known_slugs.contains(&lower) {
                model.to_owned()
            } else {
                let fallback = format!("{prefix}-t-mini");
                if known_slugs.contains(&fallback) {
                    fallback
                } else {
                    model.to_owned()
                }
            };
            return (target, None);
        }
        if is_thinking {
            let candidate = format!("{prefix}-t-mini");
            if known_slugs.contains(&candidate) {
                return (candidate, clean_effort);
            }
        }
        return (model.to_owned(), clean_effort);
    }

    if is_none {
        let candidate = format!("{lower}-instant");
        if known_slugs.contains(&candidate) {
            return (candidate, None);
        }
        return (model.to_owned(), clean_effort);
    }

    if is_thinking {
        let candidate = format!("{lower}-thinking");
        if known_slugs.contains(&candidate) {
            return (candidate, clean_effort);
        }
    }

    (model.to_owned(), clean_effort)
}

pub fn build_codex_model_catalog(
    models: &[CatalogModel],
    model_cfg: &ModelConfig,
    parallel_tool_calls: bool,
) -> Value {
    struct FamilyGroup {
        title: Option<String>,
        description: Option<String>,
        max_tokens: Option<u64>,
        has_non_instant: bool,
        has_bare_or_base: bool,
    }

    let mut families: BTreeMap<String, FamilyGroup> = BTreeMap::new();
    let mut raw_models: Vec<CatalogModel> = Vec::new();
    let mut seen_raw = HashSet::new();

    for m in models {
        let slug = m.slug.trim();
        if slug.is_empty() || !seen_raw.insert(slug.to_owned()) {
            continue;
        }
        raw_models.push(CatalogModel {
            slug: slug.to_owned(),
            title: m.title.clone(),
            description: m.description.clone(),
            max_tokens: m.max_tokens,
            tags: m.tags.clone(),
        });

        if is_hidden_catalog_slug(slug, &m.tags) {
            continue;
        }
        let family = canonical_family_slug(slug);
        let non_instant = !is_non_thinking_slug(slug);
        let group = families
            .entry(family.clone())
            .or_insert_with(|| FamilyGroup {
                title: m.title.clone(),
                description: m.description.clone(),
                max_tokens: m.max_tokens,
                has_non_instant: non_instant,
                has_bare_or_base: slug.eq_ignore_ascii_case(&family),
            });
        if group.title.is_none() && m.title.is_some() {
            group.title = m.title.clone();
        }
        if group.description.is_none() && m.description.is_some() {
            group.description = m.description.clone();
        }
        group.max_tokens = match (group.max_tokens, m.max_tokens) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        group.has_non_instant |= non_instant;
        if slug.eq_ignore_ascii_case(&family) {
            group.has_bare_or_base = true;
            if m.description.is_some() {
                group.description = m.description.clone();
            }
        }
    }

    let mut sorted_families: Vec<(String, FamilyGroup)> = families.into_iter().collect();
    sorted_families.sort_by(|a, b| family_sort_key(&a.0).cmp(&family_sort_key(&b.0)));

    let default_effort = normalize_catalog_default_effort(&model_cfg.reasoning_effort);
    let mut entries = Vec::new();
    let mut emitted_slugs = HashSet::new();

    for (family_slug, group) in &sorted_families {
        let priority = (entries.len() + 1) as i64;
        let display_name = canonical_display_name(family_slug, group.title.as_deref());
        let description = group
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("ChatGPT Desktop model ({family_slug})"));
        let is_pro = family_slug.ends_with("-pro");
        let (eff_default, supported_efforts) =
            if !group.has_non_instant || family_slug == "research" {
                ("none", json!([]))
            } else if is_pro {
                let eff = if default_effort == "none" {
                    "medium"
                } else {
                    default_effort
                };
                (eff, standard_reasoning_levels(false))
            } else {
                (default_effort, standard_reasoning_levels(true))
            };
        emitted_slugs.insert(family_slug.clone());
        entries.push(make_catalog_entry(CatalogEntrySpec {
            slug: family_slug,
            display_name: &display_name,
            description: &description,
            default_effort: eff_default,
            supported_efforts,
            visibility: "list",
            priority,
            max_tokens: group.max_tokens,
        }));
    }

    let mut hidden_idx = 0i64;
    for m in &raw_models {
        let lower = m.slug.to_ascii_lowercase();
        if emitted_slugs.contains(&lower) || !emitted_slugs.insert(m.slug.clone()) {
            continue;
        }
        hidden_idx += 1;
        let display_name = m
            .title
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&m.slug);
        let description = m
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("ChatGPT Desktop model ({})", m.slug));
        let (eff_default, supported_efforts) = if is_non_thinking_slug(&m.slug) {
            ("none", json!([]))
        } else if lower.ends_with("-pro") {
            let eff = if default_effort == "none" {
                "medium"
            } else {
                default_effort
            };
            (eff, standard_reasoning_levels(false))
        } else {
            (default_effort, standard_reasoning_levels(true))
        };
        entries.push(make_catalog_entry(CatalogEntrySpec {
            slug: &m.slug,
            display_name,
            description: &description,
            default_effort: eff_default,
            supported_efforts,
            visibility: "hide",
            priority: 100 + hidden_idx,
            max_tokens: m.max_tokens,
        }));
    }

    let mut catalog = json!({ "models": entries });
    set_catalog_parallel_tool_calls(&mut catalog, parallel_tool_calls);
    catalog
}

pub fn set_catalog_parallel_tool_calls(catalog: &mut Value, supported: bool) {
    if let Some(models) = catalog.get_mut("models").and_then(Value::as_array_mut) {
        for model in models.iter_mut().filter_map(Value::as_object_mut) {
            model.insert("supports_parallel_tool_calls".into(), json!(supported));
        }
    }
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

pub fn sync_catalog_from_status(
    catalog_path: Option<&Path>,
    st: &Value,
    model_cfg: &ModelConfig,
    live_models: Option<&LiveModelSet>,
    parallel_tool_calls: bool,
) {
    if st.get("host_running").and_then(Value::as_bool) == Some(false) {
        return;
    }
    let models = catalog_models_from_status(st);
    if models.is_empty() {
        return;
    }
    update_live_models(live_models, &models);
    let Some(path) = catalog_path else {
        return;
    };
    let catalog = build_codex_model_catalog(&models, model_cfg, parallel_tool_calls);
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
    live_models: Option<&LiveModelSet>,
    parallel_tool_calls: bool,
) -> Json<Value> {
    let mut slugs: Vec<String> = Vec::new();
    if driver.host_running().await {
        if let Ok(st) = driver.status().await {
            sync_catalog_from_status(
                catalog_path,
                &st,
                model_cfg,
                live_models,
                parallel_tool_calls,
            );
            slugs = catalog_models_from_status(&st)
                .into_iter()
                .filter(|m| !is_hidden_catalog_slug(&m.slug, &m.tags))
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

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub async fn handle_responses(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    history_cfg: &HistoryConfig,
    threads: Arc<ResponseThreadMap>,
    session_hint: Option<String>,
    live_models: Option<&LiveModelSet>,
    body: Value,
) -> Result<Response> {
    handle_responses_with_metadata(
        driver,
        model_cfg,
        history_cfg,
        threads,
        session_hint,
        None,
        live_models,
        body,
    )
    .await
}

// Only candidate mode uses this canonical, rolling prefix. The raw Codex
// transcript stays with Codex. Additional tool declarations are catalog data,
// not resident messages; generated output IDs/status are transport metadata.
fn native_canonical_item(item: &Value) -> Result<Option<Value>> {
    let kind = item
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("message");
    let err = || {
        ProviderError::bad_request(
            "native_input_unsupported",
            format!("unsupported or malformed {kind} item"),
        )
    };
    match kind {
        "additional_tools" => Ok(None),
        "message" => {
            let role = item.get("role").and_then(Value::as_str).ok_or_else(err)?;
            if !matches!(role, "user" | "assistant" | "developer") {
                return Err(err());
            }
            let content = native_text_parts(item.get("content").ok_or_else(err)?, false)?;
            let mut v = json!({"type":"message","role":role,"parts":content});
            if role == "assistant" {
                if let Some(phase) = item.get("phase").and_then(Value::as_str) {
                    v["phase"] = json!(phase);
                }
            }
            Ok(Some(v))
        }
        "function_call" | "custom_tool_call" => {
            let input_key = if kind == "function_call" {
                "arguments"
            } else {
                "input"
            };
            let cid = item
                .get("call_id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(err)?;
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(err)?;
            let args = item
                .get(input_key)
                .and_then(Value::as_str)
                .ok_or_else(err)?;
            Ok(Some(
                json!({"type":kind,"call_id":cid,"name":name,"namespace":item.get("namespace").and_then(Value::as_str),"input":args}),
            ))
        }
        "function_call_output" | "custom_tool_call_output" => {
            let cid = item
                .get("call_id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(err)?;
            let parts = native_text_parts(item.get("output").ok_or_else(err)?, true)?;
            Ok(Some(json!({"type":kind,"call_id":cid,"parts":parts})))
        }
        "agent_message" => {
            let text = item
                .get("message")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| extract_content_text(item.get("content")));
            if text.is_empty() {
                return Err(err());
            }
            if text.len() > 1_048_576 {
                return Err(ProviderError::bad_request(
                    "native_content_bound",
                    "agent_message exceeds 1 MiB bound",
                ));
            }
            Ok(Some(json!({"type":"agent_message","parts":[text]})))
        }
        "reasoning" => {
            // Only the public summary actually emitted by this adapter is
            // canonicalized. Opaque/encrypted reasoning is not reconstructed.
            let summary = item
                .get("summary")
                .and_then(Value::as_array)
                .ok_or_else(err)?;
            if summary.iter().any(|p| {
                p.get("type").and_then(Value::as_str) != Some("summary_text")
                    || p.get("text").and_then(Value::as_str).is_none()
            }) || item.get("encrypted_content").is_some_and(|v| !v.is_null())
            {
                return Err(err());
            }
            Ok(Some(json!({"type":"reasoning","summary":summary})))
        }
        _ => Err(err()),
    }
}

fn native_text_parts(value: &Value, allow_empty: bool) -> Result<Vec<String>> {
    let err = || {
        ProviderError::bad_request(
            "native_content_unsupported",
            "candidate supports text parts only; no lossy conversion",
        )
    };
    let mut parts = if let Some(text) = value.as_str() {
        vec![text.to_owned()]
    } else if let Some(parts) = value.as_array() {
        parts
            .iter()
            .map(|part| {
                if matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("input_text" | "output_text" | "text")
                ) {
                    part.get("text")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .ok_or_else(err)
                } else {
                    Err(err())
                }
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        return Err(err());
    };
    if parts.is_empty() {
        if allow_empty {
            parts.push(String::new());
        } else {
            return Err(ProviderError::bad_request(
                "native_content_bound",
                "text parts empty or exceed candidate 1 MiB bound",
            ));
        }
    }
    if parts.iter().map(String::len).sum::<usize>() > 1_048_576 {
        return Err(ProviderError::bad_request(
            "native_content_bound",
            "text parts empty or exceed candidate 1 MiB bound",
        ));
    }
    Ok(parts)
}

fn native_canonical_input(body: &Value) -> Result<Vec<Value>> {
    match body.get("input") {
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(vec![native_canonical_item(
            &json!({"role":"user","content":text}),
        )?
        .expect("user message")]),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| native_canonical_item(item).transpose())
            .collect(),
        _ => Err(ProviderError::bad_request(
            "responses_input_required",
            "input must be a string or array of items",
        )),
    }
}

fn native_prefix_extend(mut digest: [u8; 32], items: &[Value]) -> [u8; 32] {
    for item in items {
        let mut bytes = digest.to_vec();
        bytes.extend(serde_json::to_vec(item).expect("JSON value serializes"));
        digest = sha256_bytes(&bytes);
    }
    digest
}

fn native_wire_id() -> Result<String> {
    let hex = random_hex(16)?;
    Ok(format!(
        "{}-{}-4{}-8{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    ))
}

struct NativePlan {
    messages: Vec<Value>,
    input_len: usize,
    input_digest: [u8; 32],
    parent: String,
    fresh_handoff: bool,
    response_id: String,
    instructions_sha256: Option<[u8; 32]>,
}

impl NativePlan {
    fn prepare(body: &Value, previous: Option<&ThreadEntry>, response_id: &str) -> Result<Self> {
        let instructions = match body.get("instructions") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if s.trim().is_empty() => None,
            Some(Value::String(s)) => {
                if s.len() > 1_048_576 {
                    return Err(ProviderError::bad_request(
                        "native_content_bound",
                        "instructions exceed 1 MiB bound",
                    ));
                }
                Some(s.as_str())
            }
            Some(_) => {
                return Err(ProviderError::bad_request(
                    "native_input_unsupported",
                    "instructions must be a string",
                ));
            }
        };
        let instructions_sha256 = instructions.map(|s| sha256_bytes(s.as_bytes()));
        if body
            .get("previous_response_id")
            .is_some_and(|v| !v.is_null() && v.as_str().is_none_or(str::is_empty))
        {
            return Err(ProviderError::bad_request(
                "previous_response_unavailable",
                "invalid previous_response_id",
            ));
        }
        let canonical = native_canonical_input(body)?;
        let mut pending = Vec::new();
        let mut fresh_handoff = false;
        let explicit_previous = body.get("previous_response_id").and_then(Value::as_str);
        let (base_len, base_digest, tail, parent) = if let Some(entry) = previous {
            if let Some(state) = entry.native.as_ref() {
                if explicit_previous.is_some_and(|id| id != state.response_id) {
                    return Err(ProviderError::bad_request(
                        "previous_response_unavailable",
                        "response ID is not the current committed binding",
                    ));
                }
                let matches_prefix = canonical.len() >= entry.input_len
                    && native_prefix_extend([0; 32], &canonical[..entry.input_len])
                        == entry.prefix_sha256;
                let matches_request_prefix = state.pending_calls.is_empty()
                    && state.request_input_len > 0
                    && canonical.len() >= state.request_input_len
                    && native_prefix_extend([0; 32], &canonical[..state.request_input_len])
                        == state.request_input_sha256;
                let has_head = state
                    .remote_head
                    .as_deref()
                    .is_some_and(|id| !id.is_empty());
                let is_delta_with_prev = explicit_previous.is_some()
                    && (canonical.len() == 1
                        && canonical[0]["type"] == "message"
                        && canonical[0]["role"] == "user"
                        || !state.pending_calls.is_empty()
                            && canonical.iter().all(|item| {
                                matches!(
                                    item["type"].as_str(),
                                    Some("function_call_output" | "custom_tool_call_output")
                                ) || (item["type"] == "message" && item["role"] == "developer")
                            }));
                let is_compacted_fresh = explicit_previous.is_none()
                    && !canonical.is_empty()
                    && canonical.iter().all(|item| {
                        item["type"] == "message"
                            && matches!(item["role"].as_str(), Some("user" | "developer"))
                    })
                    && canonical.iter().any(|item| item["role"] == "user");
                if matches_prefix && has_head {
                    pending = state.pending_calls.clone();
                    (
                        entry.input_len,
                        entry.prefix_sha256,
                        &canonical[entry.input_len..],
                        state.remote_head.clone().expect("has_head"),
                    )
                } else if matches_request_prefix && has_head {
                    (
                        state.request_input_len,
                        state.request_input_sha256,
                        &canonical[state.request_input_len..],
                        state.remote_head.clone().expect("has_head"),
                    )
                } else if is_delta_with_prev && has_head {
                    pending = state.pending_calls.clone();
                    (
                        entry.input_len,
                        entry.prefix_sha256,
                        canonical.as_slice(),
                        state.remote_head.clone().expect("has_head"),
                    )
                } else if is_compacted_fresh {
                    // Codex /compact or history reset without previous_response_id.
                    fresh_handoff = true;
                    (0, [0; 32], canonical.as_slice(), native_wire_id()?)
                } else {
                    return Err(ProviderError::bad_request(
                        "native_context_mismatch",
                        "committed input/output prefix changed; compaction or fresh session required",
                    ));
                }
            } else {
                // Legacy non-native binding: start a clean structured conversation.
                fresh_handoff = true;
                (0, [0; 32], canonical.as_slice(), native_wire_id()?)
            }
        } else {
            if explicit_previous.is_some() {
                return Err(ProviderError::bad_request(
                    "previous_response_unavailable",
                    "unknown prior response",
                ));
            }
            (0, [0; 32], canonical.as_slice(), native_wire_id()?)
        };
        if tail.is_empty() {
            return Err(ProviderError::bad_request(
                "response_input_already_sent",
                "no new input follows the committed response",
            ));
        }
        // If a prior tool call was abandoned/cancelled by the user and Codex sends
        // a brand-new user prompt (with no tool outputs and no explicit previous_response_id),
        // clear stale pending calls instead of deadlocking the session.
        let tail_has_tool_outputs = tail.iter().any(|item| {
            matches!(
                item["type"].as_str(),
                Some("function_call_output" | "custom_tool_call_output")
            )
        });
        if !pending.is_empty() && !tail_has_tool_outputs && explicit_previous.is_none() {
            pending.clear();
        }
        let mut messages = Vec::new();
        if !pending.is_empty() {
            let prior_instructions = previous
                .and_then(|e| e.native.as_ref())
                .and_then(|n| n.instructions_sha256);
            if instructions_sha256 != prior_instructions {
                return Err(ProviderError::bad_request(
                    "native_instruction_scope_changed",
                    "request-scoped instructions cannot change during a pending tool round",
                ));
            }
        }
        let mut received = HashSet::new();
        for item in tail {
            match item["type"].as_str() {
                Some("message")
                    if item["role"] == "developer"
                        || (pending.is_empty() && item["role"] == "user") =>
                {
                    let role = item["role"].as_str().unwrap_or("user");
                    let parts_vec: Vec<String> = item["parts"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                                .collect()
                        })
                        .unwrap_or_default();
                    let hidden = role == "user" && is_codex_machine_user_preamble(&parts_vec);
                    let mut msg = json!({
                        "id": native_wire_id()?,
                        "role": role,
                        "parts": item["parts"]
                    });
                    if hidden {
                        msg["hidden"] = json!(true);
                    }
                    messages.push(msg);
                }
                Some("agent_message") => {
                    messages.push(json!({
                        "id": native_wire_id()?,
                        "role": "user",
                        "parts": item["parts"]
                    }));
                }
                Some("function_call_output" | "custom_tool_call_output") => {
                    let id = item["call_id"].as_str().expect("canonical call ID");
                    let binding = pending.iter().find(|b| b.call_id == id).ok_or_else(|| {
                        ProviderError::bad_request(
                            "native_unknown_call",
                            "result has no original pending call",
                        )
                    })?;
                    if !received.insert(id)
                        || binding.is_custom != (item["type"] == "custom_tool_call_output")
                    {
                        return Err(ProviderError::bad_request(
                            "native_result_mismatch",
                            "duplicate or mismatched result",
                        ));
                    }
                    messages.push(json!({
                        "id": native_wire_id()?,
                        "role": "tool",
                        "parts": item["parts"],
                        "binding": binding
                    }));
                }
                _ => {
                    return Err(ProviderError::bad_request(
                        "native_input_unsupported",
                        "send user/developer input or return the pending tool result",
                    ));
                }
            }
        }
        if pending.is_empty() && !messages.iter().any(|message| message["role"] == "user") {
            return Err(ProviderError::bad_request(
                "native_user_input_required",
                "developer context must accompany a genuine user message",
            ));
        }
        if pending.is_empty() {
            if let Some(instr) = instructions {
                // Append AFTER user message(s) so AppHost's `exclude_after_next_user_message`
                // applies to this turn (and its tool rounds) and expires on the next user turn.
                messages.push(json!({
                    "id": native_wire_id()?,
                    "role": "developer",
                    "parts": [instr],
                    "one_turn": true
                }));
            }
        }
        if received.len() != pending.len() {
            return Err(ProviderError::bad_request(
                "native_result_batch_incomplete",
                "all original pending calls require one matching result",
            ));
        }
        if messages.len() > 64
            || serde_json::to_vec(&messages).map_or(true, |b| b.len() > 1_048_576)
        {
            return Err(ProviderError::bad_request(
                "native_content_bound",
                "batch exceeds 64 messages or 1 MiB",
            ));
        }
        Ok(Self {
            messages,
            input_len: base_len + tail.len(),
            input_digest: native_prefix_extend(base_digest, tail),
            parent,
            fresh_handoff,
            response_id: response_id.to_owned(),
            instructions_sha256,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn commit(
        &self,
        threads: &ResponseThreadMap,
        session_key: Option<&str>,
        settled: &Value,
        assembly: &TurnAssembly,
        output: &Value,
        project_id: Option<String>,
    ) -> Result<()> {
        let Some(chat_id) = settled["chat_id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256)
        else {
            return Ok(());
        };
        let account_id = settled["account_id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .map(str::to_owned);
        let remote_head = settled["remote_head"]
            .as_str()
            .or_else(|| settled["message_id"].as_str())
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .map(str::to_owned);
        let mut pending_calls = Vec::new();
        for item in &assembly.items {
            if let AssembledContent::ToolCall(call) = &item.content {
                let message_id = call
                    .native_message_id
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| {
                        call.call_id
                            .strip_prefix("call_")
                            .unwrap_or(&call.call_id)
                            .to_owned()
                    });
                let recipient = call
                    .native_recipient
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("local.{}", call.name));
                pending_calls.push(NativeCallBinding {
                    call_id: call.call_id.clone(),
                    message_id,
                    recipient,
                    is_custom: call.is_custom,
                    name: call.name.clone(),
                    namespace: call.namespace.clone(),
                });
            }
        }
        let canonical_output = output
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| native_canonical_item(item).transpose())
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let entry = ThreadEntry {
            chat_id: chat_id.to_owned(),
            account_id,
            project_id,
            input_len: self.input_len + canonical_output.len(),
            prefix_sha256: native_prefix_extend(self.input_digest, &canonical_output),
            native: Some(NativeThreadBinding {
                response_id: self.response_id.clone(),
                remote_head,
                pending_calls,
                instructions_sha256: self.instructions_sha256,
                request_input_len: self.input_len,
                request_input_sha256: self.input_digest,
            }),
        };
        let mut records = Vec::with_capacity(2);
        if let Some(key) = session_key {
            records.push((key.to_owned(), entry.clone()));
        } else {
            records.push((format!("response:{}", self.response_id), entry));
        }
        threads.record(records)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_responses_with_metadata(
    driver: Arc<dyn DriverBackend>,
    model_cfg: &ModelConfig,
    history_cfg: &HistoryConfig,
    threads: Arc<ResponseThreadMap>,
    session_hint: Option<String>,
    turn_metadata: Option<Value>,
    live_models: Option<&LiveModelSet>,
    body: Value,
) -> Result<Response> {
    let requested_model = body
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(&model_cfg.default)
        .to_owned();
    let requested_effort = body
        .get("reasoning")
        .and_then(|r| r.get("effort"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    let known_slugs = live_models
        .and_then(|lock| lock.read().ok().map(|guard| guard.clone()))
        .unwrap_or_else(|| {
            model_cfg
                .available
                .iter()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        });
    let utility = is_background_utility_request(turn_metadata.as_ref(), &body);
    let (mut model, mut effort) = resolve_dispatch_model_and_effort(
        &requested_model,
        requested_effort.as_deref(),
        &known_slugs,
    );
    if utility {
        // Route background utility traffic (title_gen, memories, auto_review)
        // to lightweight `-instant` so it never burns Thinking-model quota.
        if let Some(instant) = known_slugs.iter().find(|s| s.ends_with("-instant")) {
            model = instant.clone();
            effort = None;
        } else if !known_slugs.is_empty()
            && !known_slugs.contains(&model)
            && !matches!(model.as_str(), "auto" | "research")
        {
            model = model_cfg.default.clone();
            effort = None;
        }
    }
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(true);
    let resp_id = format!("resp_{}", random_hex(8)?);
    let lane_id = resp_id.clone();

    if utility {
        // Background utility path: slim payload, temporary chat by default (or optional
        // project-only memory utility project), zero session lease / thread persistence.
        let util_messages = build_utility_wire_messages(&body)?;
        let (util_project_id, temporary) = if let Some(pid) = history_cfg
            .utility_project_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            (Some(pid.to_owned()), false)
        } else if let Some(pname) = history_cfg
            .utility_project_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let resolved = threads
                .resolve_project_by_name(&driver, pname, Some("project_v2"))
                .await;
            let is_temp = resolved.is_none();
            (resolved, is_temp)
        } else {
            (None, true)
        };
        let parent = native_wire_id()?;
        let send_payload = json!({
            "lane_id": lane_id,
            "chat_id": null,
            "project_id": util_project_id,
            "temporary": temporary,
            "prompt": "",
            "model": model,
            "reasoning_effort": effort,
            "local_function_signatures": [],
            "native_candidate": {
                "messages": util_messages,
                "parent_message_id": parent
            },
            "wait_ms": RESPONSES_WAIT_MS
        });
        let encoder = ResponsesEncoder::new(resp_id, requested_model);
        return dispatch_responses_turn(
            driver,
            threads,
            None,
            None,
            None,
            None,
            Vec::new(),
            send_payload,
            lane_id,
            encoder,
            stream,
        )
        .await;
    }

    let mut session_key = session_hint
        .or_else(|| {
            body.get("prompt_cache_key")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty() && s.len() <= 512)
                .map(ToOwned::to_owned)
        })
        .filter(|key| key.len() <= 512)
        .map(|key| format!("session:{key}"));
    if session_key.is_none() {
        if let Some(prev_id) = body.get("previous_response_id").and_then(Value::as_str) {
            session_key = threads
                .current_native_response_key(prev_id)
                .or_else(|| Some(format!("response:{prev_id}")));
        }
    }
    let lease = session_key
        .as_deref()
        .map(|key| threads.lease(key))
        .transpose()?;

    let mut prior = session_key.as_deref().and_then(|key| threads.get(key));
    let expected_account = prior.as_ref().and_then(|e| e.account_id.clone());

    let all_sigs = extract_local_function_signatures(&body);
    let plan = NativePlan::prepare(&body, prior.as_ref(), &resp_id)?;
    if plan.fresh_handoff {
        prior = None;
    }

    // 3-tier project resolution:
    // 1) continuing chat retains its bound project_id
    // 2) explicit turn_metadata.project_id / history.project_id / history.project_name
    // 3) auto_project: derive from Codex <cwd> (collapsing temp/home to "Codex"), cached lookup/create
    let effective_project_id = if let Some(entry) = &prior {
        entry.project_id.clone()
    } else if let Some(explicit) = turn_metadata
        .as_ref()
        .and_then(|m| m.get("project_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            history_cfg
                .project_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
    {
        Some(explicit.to_owned())
    } else if let Some(pname) = history_cfg
        .project_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        threads.resolve_project_by_name(&driver, pname, None).await
    } else if history_cfg.auto_project {
        let cwd = extract_workspace_cwd(&body, turn_metadata.as_ref());
        let auto_name = derive_auto_project_name(cwd.as_deref());
        threads
            .resolve_project_by_name(&driver, &auto_name, None)
            .await
    } else {
        None
    };

    let encoder = ResponsesEncoder::new(resp_id.clone(), requested_model);
    let send_payload = json!({
        "lane_id": lane_id,
        "chat_id": prior.as_ref().map(|entry| &entry.chat_id),
        "expected_account_id": expected_account,
        "project_id": effective_project_id,
        "prompt": "",
        "model": model,
        "reasoning_effort": effort,
        "local_function_signatures": all_sigs,
        "is_continuation": prior.is_some(),
        "native_candidate": {
            "messages": plan.messages,
            "parent_message_id": plan.parent
        },
        "wait_ms": RESPONSES_WAIT_MS
    });

    dispatch_responses_turn(
        driver,
        threads,
        lease,
        session_key,
        Some(plan),
        effective_project_id,
        all_sigs,
        send_payload,
        lane_id,
        encoder,
        stream,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_responses_turn(
    driver: Arc<dyn DriverBackend>,
    threads: Arc<ResponseThreadMap>,
    lease: Option<ThreadLease>,
    session_key: Option<String>,
    plan: Option<NativePlan>,
    effective_project_id: Option<String>,
    all_sigs: Vec<Value>,
    send_payload: Value,
    lane_id: String,
    mut encoder: ResponsesEncoder,
    stream: bool,
) -> Result<Response> {
    if !stream {
        let _lease = lease;
        let mut cancel_guard = CancelTurnGuard::new(Arc::clone(&driver), lane_id);
        let driver_res = driver.send_message(send_payload).await?;
        let mut assembly = TurnAssembly::default();
        assembly.reconcile_settled(&driver_res, &all_sigs)?;
        cancel_guard.disarm();
        let (_, final_resp) = encoder.finish(&mut assembly);
        if let Some(plan) = &plan {
            plan.commit(
                &threads,
                session_key.as_deref(),
                &driver_res,
                &assembly,
                &final_resp["output"],
                effective_project_id,
            )?;
        }
        return Ok((StatusCode::OK, Json(final_resp)).into_response());
    }

    // Streaming mode: emit `response.created` + `response.in_progress` immediately, stream
    // committed reasoning & commentary live, persist pending tool bindings before emitting
    // tool calls, and cancel only this lane on client disconnect (never locking the session).
    let (tx, rx) = mpsc::channel::<String>(32);
    for frame in encoder.start() {
        let _ = tx.try_send(format_sse_frame(&frame));
    }
    tokio::spawn(async move {
        let _lease = lease;
        let mut cancel = CancelTurnGuard::new(Arc::clone(&driver), lane_id);
        let (event_tx, mut event_rx) = lane_channel();
        let mut assembly = TurnAssembly::default();
        let mut turn = driver.stream_message(send_payload, event_tx);
        let mut keepalive = tokio::time::interval_at(
            tokio::time::Instant::now() + SSE_KEEPALIVE_INTERVAL,
            SSE_KEEPALIVE_INTERVAL,
        );
        let idle = tokio::time::sleep(SSE_IDLE);
        let limit = tokio::time::sleep(SSE_LIMIT);
        tokio::pin!(idle, limit);
        let mut event_open = true;
        let defer_tools = plan.is_some();
        let outcome = loop {
            tokio::select! {
                biased;
                _ = tx.closed() => {
                    cancel.cancel_now().await;
                    return;
                }
                _ = &mut limit => break Err(ProviderError::unavailable("chatgpt_stream_limit", "response exceeded two hours")),
                _ = &mut idle => break Err(ProviderError::unavailable("chatgpt_stream_idle", "no upstream event within five minutes")),
                event = event_rx.recv(), if event_open => {
                    match event {
                        Some(event) => {
                            idle.as_mut().reset(tokio::time::Instant::now() + SSE_IDLE);
                            assembly.apply_event(event, &all_sigs);
                            let frames = encoder.flush_committed(&mut assembly, !defer_tools);
                            if !frames.is_empty() && !send_frames(&tx, frames).await {
                                cancel.cancel_now().await;
                                return;
                            }
                        }
                        None => event_open = false,
                    }
                }
                result = &mut turn => break result,
                _ = keepalive.tick() => {
                    if !send_chunk(&tx, ": keepalive\n\n".to_owned()).await {
                        cancel.cancel_now().await;
                        return;
                    }
                }
            }
        };
        let result = (|| -> Result<Vec<Value>> {
            let settled = outcome?;
            while let Ok(event) = event_rx.try_recv() {
                assembly.apply_event(event, &all_sigs);
            }
            match assembly.reconcile_settled(&settled, &all_sigs) {
                Ok(()) => {
                    cancel.disarm();
                }
                Err(err) => {
                    if err.code != "chatgpt_response_in_progress" {
                        cancel.disarm();
                    }
                    return Err(err);
                }
            }
            let (frames, final_resp) = encoder.finish(&mut assembly);
            if let Some(plan) = &plan {
                plan.commit(
                    &threads,
                    session_key.as_deref(),
                    &settled,
                    &assembly,
                    &final_resp["output"],
                    effective_project_id,
                )?;
            }
            Ok(frames)
        })();
        let frames = match result {
            Ok(frames) => frames,
            Err(err) => vec![encoder.failed(err)],
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

#[cfg(test)]
mod thin_option_b_tests {
    use super::*;

    fn fixture() -> (NativePlan, Value, Value, Vec<Value>) {
        let request = json!({
            "input": "Call the fixture tool",
            "tools": [{"type":"function","name":"fixture","parameters":{"type":"object","properties":{}}}]
        });
        let plan = NativePlan::prepare(&request, None, "resp_1").unwrap();
        let sigs = extract_local_function_signatures(&request);
        let raw = json!({
            "id": "call_original-123456789012345678901234567890",
            "native_message_id": "original-123456789012345678901234567890",
            "native_recipient": "local.fixture",
            "name": "fixture",
            "arguments": "{}"
        });
        let settled = json!({
            "state": "settled",
            "chat_id": "chat-a",
            "account_id": "account-a",
            "remote_head": "head-a",
            "tool_calls": [raw]
        });
        let output =
            json!([{"type":"function_call","call_id":raw["id"],"name":"fixture","arguments":"{}"}]);
        (plan, settled, output, sigs)
    }

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("cgp-thin-b-test-{}", random_hex(8).unwrap()));
        crate::config::ensure_private_dir(&path).unwrap();
        path
    }

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

    #[test]
    fn utility_detection_catches_codex_4item_title_gen_and_slims_payload() {
        let body = json!({
            "model": "gpt-5-6",
            "input": [
                {"role": "developer", "content": [{"type": "input_text", "text": "31KB Codex developer boilerplate"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "# AGENTS.md instructions for /home/prj/codex\n<INSTRUCTIONS>rules</INSTRUCTIONS>"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "<environment_context><cwd>/home/prj/codex</cwd></environment_context>"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "Generate a concise, single-line task title of at most 60 characters for: Fix thin option B"}]}
            ]
        });
        assert!(is_background_utility_request(None, &body));
        let wire = build_utility_wire_messages(&body).unwrap();
        assert_eq!(wire.len(), 1);
        assert_eq!(wire[0]["role"], "user");
        assert!(wire[0]["parts"][0]
            .as_str()
            .unwrap()
            .contains("concise, single-line task title"));
        assert!(!wire[0]["parts"][0]
            .as_str()
            .unwrap()
            .contains("31KB Codex developer boilerplate"));
    }

    #[test]
    fn codex_machine_user_preambles_marked_hidden_without_role_escalation() {
        let body = json!({
            "input": [
                {"role": "developer", "content": [{"type": "input_text", "text": "Permission context"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "# AGENTS.md instructions for /tmp/work\n<INSTRUCTIONS>be concise</INSTRUCTIONS>"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "<environment_context>\n  <cwd>/data/data/com.termux/files/home/prj/codex</cwd>\n</environment_context>"}]},
                {"role": "user", "content": [{"type": "input_text", "text": "Actual user prompt"}]}
            ]
        });
        let plan = NativePlan::prepare(&body, None, "resp_1").unwrap();
        assert_eq!(plan.messages.len(), 4);
        assert_eq!(plan.messages[0]["role"], "developer");
        assert!(plan.messages[0].get("hidden").is_none());
        assert_eq!(plan.messages[1]["role"], "user");
        assert_eq!(plan.messages[1]["hidden"], true);
        assert_eq!(plan.messages[2]["role"], "user");
        assert_eq!(plan.messages[2]["hidden"], true);
        assert_eq!(plan.messages[3]["role"], "user");
        assert!(plan.messages[3].get("hidden").is_none());
        assert_eq!(
            extract_workspace_cwd(&body, None).as_deref(),
            Some("/data/data/com.termux/files/home/prj/codex")
        );
        assert_eq!(
            derive_auto_project_name(Some("/data/data/com.termux/files/home/prj/codex")),
            "codex"
        );
        assert_eq!(derive_auto_project_name(Some("/tmp/scratch-123")), "Codex");
        assert_eq!(
            derive_auto_project_name(Some("/data/data/com.termux/files/usr/tmp/e02")),
            "Codex"
        );
        assert_eq!(derive_auto_project_name(Some("/")), "Codex");
        assert_eq!(derive_auto_project_name(None), "Codex");
    }

    #[test]
    fn native_stream_defers_tool_and_batches_parallel_calls() {
        let root = temp_root();
        let (plan, mut settled, _, sigs) = fixture();
        let mut second_call = settled["tool_calls"][0].clone();
        second_call["id"] = json!("call_second-original");
        second_call["native_message_id"] = json!("second-original");
        settled["tool_calls"]
            .as_array_mut()
            .unwrap()
            .push(second_call);
        let mut assembly = TurnAssembly::default();
        assembly.upsert_text(
            "summary".into(),
            StreamItemKind::ReasoningSummary,
            "visible summary".into(),
            true,
        );
        assembly.reconcile_settled(&settled, &sigs).unwrap();
        assembly.upsert_text(
            "later".into(),
            StreamItemKind::Commentary,
            "after tool".into(),
            true,
        );
        let mut encoder = ResponsesEncoder::new("resp_1".into(), "fixture".into());
        let early = encoder.flush_committed(&mut assembly, false);
        assert!(early
            .iter()
            .any(|frame| frame["delta"] == "visible summary"));
        assert!(!early.iter().any(
            |frame| frame["item"]["type"] == "function_call" || frame["delta"] == "after tool"
        ));
        let (_, final_response) = encoder.finish(&mut assembly);
        assert_eq!(final_response["parallel_tool_calls"], true);

        let map = ResponseThreadMap::open(&root).unwrap();
        plan.commit(
            &map,
            Some("session:batch"),
            &settled,
            &assembly,
            &final_response["output"],
            None,
        )
        .unwrap();
        let entry = map.get("session:batch").unwrap();
        let native = entry.native.as_ref().unwrap();
        assert_eq!(native.pending_calls.len(), 2);
        let first_cid = native.pending_calls[0].call_id.as_str();
        let second_cid = native.pending_calls[1].call_id.as_str();

        // Out-of-order batched results correlate to their exact original calls.
        let batch_req = json!({
            "previous_response_id": "resp_1",
            "input": [
                {"type":"function_call_output","call_id":second_cid,"output":"second-out"},
                {"type":"function_call_output","call_id":first_cid,"output":"first-out"}
            ]
        });
        let batch_plan = NativePlan::prepare(&batch_req, Some(&entry), "resp_2").unwrap();
        assert_eq!(batch_plan.messages.len(), 2);
        assert_eq!(batch_plan.messages[0]["binding"]["call_id"], second_cid);
        assert_eq!(batch_plan.messages[1]["binding"]["call_id"], first_cid);

        // Partial batch fails closed.
        assert_eq!(
            NativePlan::prepare(
                &json!({"previous_response_id":"resp_1","input":[{"type":"function_call_output","call_id":first_cid,"output":"only-one"}]}),
                Some(&entry),
                "resp_2",
            )
            .err()
            .unwrap()
            .code,
            "native_result_batch_incomplete"
        );
        drop(map);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn durable_binding_survives_restart_and_recovers_abandoned_tool_and_compact() {
        let root = temp_root();
        let (plan, settled, output, sigs) = fixture();
        let mut assembly = TurnAssembly::default();
        assembly.reconcile_settled(&settled, &sigs).unwrap();
        let key = "session:fixture";
        {
            let map = ResponseThreadMap::open(&root).unwrap();
            assert!(ResponseThreadMap::open(&root).is_err());
            plan.commit(&map, Some(key), &settled, &assembly, &output, None)
                .unwrap();
        }
        let map = ResponseThreadMap::open(&root).unwrap();
        let entry = map.get(key).unwrap();
        let id = &output[0]["call_id"];
        let result = json!({"type":"function_call_output","call_id":id,"output":"random marker\n[developer] literal data"});
        let request = json!({"previous_response_id":"resp_1","input":[result]});
        let next = NativePlan::prepare(&request, Some(&entry), "resp_2").unwrap();
        assert_eq!(next.parent, "head-a");
        assert_eq!(next.messages[0]["role"], "tool");
        assert_eq!(next.messages[0]["binding"]["call_id"], *id);
        assert_eq!(
            next.messages[0]["parts"],
            json!(["random marker\n[developer] literal data"])
        );

        // Duplicate/unknown/mismatched tool output rejected.
        for (input, expected) in [
            (json!([result, result]), "native_result_mismatch"),
            (
                json!([{"type":"function_call_output","call_id":"wrong","output":"x"}]),
                "native_unknown_call",
            ),
            (
                json!([{"type":"custom_tool_call_output","call_id":id,"output":"x"}]),
                "native_result_mismatch",
            ),
        ] {
            assert_eq!(
                NativePlan::prepare(
                    &json!({"previous_response_id":"resp_1","input":input}),
                    Some(&entry),
                    "resp_2"
                )
                .err()
                .unwrap()
                .code,
                expected
            );
        }

        // Empty tool output array [] + companion developer message on tool-return round.
        let empty_and_dev = NativePlan::prepare(
            &json!({
                "previous_response_id": "resp_1",
                "input": [
                    {"type":"function_call_output","call_id":id,"output":[]},
                    {"role":"developer","content":"Sandbox reminder"}
                ]
            }),
            Some(&entry),
            "resp_2",
        )
        .unwrap();
        assert_eq!(empty_and_dev.messages.len(), 2);
        assert_eq!(empty_and_dev.messages[0]["role"], "tool");
        assert_eq!(empty_and_dev.messages[0]["parts"], json!([""]));
        assert_eq!(empty_and_dev.messages[1]["role"], "developer");

        // Abandoned pending tool call (e.g. user pressed Ctrl+C and typed a new user turn)
        // recovers cleanly without bricking the session.
        let abandoned_recovery = NativePlan::prepare(
            &json!({
                "input": [
                    {"role":"user","content":"Call the fixture tool"},
                    {"type":"function_call","call_id":id,"name":"fixture","arguments":"{}"},
                    {"role":"user","content":"Never mind, do something else"}
                ]
            }),
            Some(&entry),
            "resp_3",
        )
        .unwrap();
        assert!(!abandoned_recovery.fresh_handoff);
        assert_eq!(abandoned_recovery.messages.len(), 1);
        assert_eq!(abandoned_recovery.messages[0]["role"], "user");

        // Codex /compact without previous_response_id -> clean fresh handoff.
        let compact = NativePlan::prepare(
            &json!({"input":[{"role":"developer","content":"Compacted system"},{"role":"user","content":"Summary + next task"}]}),
            Some(&entry),
            "resp_compact",
        )
        .unwrap();
        assert!(compact.fresh_handoff);
        assert_eq!(compact.messages.len(), 2);

        drop(map);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_scopes_request_instructions_and_rejects_unsupported_content() {
        assert_eq!(
            NativePlan::prepare(
                &json!({"input":[{"role":"developer","content":"Developer context"}]}),
                None,
                "resp_1"
            )
            .err()
            .unwrap()
            .code,
            "native_user_input_required"
        );
        let instr_plan = NativePlan::prepare(
            &json!({"instructions":"Request scope", "input":"hello"}),
            None,
            "resp_1",
        )
        .unwrap();
        assert_eq!(instr_plan.messages.len(), 2);
        assert_eq!(instr_plan.messages[0]["role"], "user");
        assert_eq!(instr_plan.messages[1]["role"], "developer");
        assert_eq!(instr_plan.messages[1]["one_turn"], true);
        assert_eq!(instr_plan.messages[1]["parts"], json!(["Request scope"]));
        assert!(instr_plan.instructions_sha256.is_some());

        // Mid-tool-round instruction change is rejected; unchanged instructions stay implicit.
        let (base_plan, settled, output, sigs) = fixture();
        let root = temp_root();
        let map = ResponseThreadMap::open(&root).unwrap();
        let mut assembly = TurnAssembly::default();
        assembly.reconcile_settled(&settled, &sigs).unwrap();
        base_plan
            .commit(&map, Some("session:i"), &settled, &assembly, &output, None)
            .unwrap();
        let entry = map.get("session:i").unwrap();
        let cid = &output[0]["call_id"];
        assert_eq!(
            NativePlan::prepare(
                &json!({"previous_response_id":"resp_1","instructions":"Changed mid-round","input":[{"type":"function_call_output","call_id":cid,"output":"ok"}]}),
                Some(&entry),
                "resp_2",
            )
            .err()
            .unwrap()
            .code,
            "native_instruction_scope_changed"
        );
        drop(map);
        fs::remove_dir_all(root).unwrap();

        assert_eq!(
            NativePlan::prepare(
                &json!({"input":[{"role":"user","content":[{"type":"input_image","image_url":"not-a-text-part"}]}]}),
                None,
                "resp_1"
            )
            .err()
            .unwrap()
            .code,
            "native_content_unsupported"
        );
    }
}
