//! `@TermuxCodex` — stateless thin MCP bridge over the official `codex app-server daemon`.
//!
//! Built on `crates/codex-daemon-client` (direct RFC 6455 WebSocket-over-UDS with
//! Termux FD 35 resolution) and `crates/mcp-kit` (uniform immutable annotations,
//! bounded `call_id` transport deduplication receipts, and disjoint `codex_catalog` /
//! `codex_call` extension slots). Zero shadow task DB; independently retirable once
//! official `codex remote-control` matures.

use codex_daemon_client::{DaemonClient, DaemonError};
use mcp_kit::{
    serve_stdio, sha256_hex, validate_call_id, ExtensionRegistry, ExtensionSpec, McpHandler,
    ReceiptClaim, ReceiptStore, ToolDef,
};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const RECEIPT_SCHEMA: &str = "termux-codex-mcp-receipt-v1";
const STATIC_TOOL_NAMES: &[&str] = &[
    "codex_status",
    "codex_threads",
    "codex_turn",
    "codex_catalog",
    "codex_call",
];
const MAX_PROMPT_BYTES: usize = 64 * 1024;
const MAX_ASSISTANT_TEXT_CHARS: usize = 32 * 1024;
const DEFAULT_WAIT_MS: u64 = 15_000;
const MAX_WAIT_MS: u64 = 120_000;
const DAEMON_RPC_TIMEOUT: Duration = Duration::from_secs(10);

fn static_tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "codex_status",
            "Read-only status of the official codex app-server daemon: running state, loaded threads, model providers, and redacted config summary.",
            json!({
                "type": "object",
                "properties": {
                    "profile": { "type": "string", "description": "Optional account profile name (default: current CODEX_HOME or 'default')" }
                },
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "codex_threads",
            "Read-only Codex thread inspection: list threads (paginated), read a thread summary/turns, or list paginated thread items.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "read", "items"] },
                    "profile": { "type": "string" },
                    "thread_id": { "type": "string", "description": "Required for action=read or action=items" },
                    "turn_id": { "type": "string", "description": "Optional turn filter for action=items" },
                    "include_turns": { "type": "boolean", "description": "Include turn details on action=read (default false)" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "cursor": { "type": "string" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "codex_turn",
            "Drive a Codex turn via official app-server daemon JSON-RPC: start (1-shot new thread + first turn when thread_id is omitted, or resume existing thread), steer an active turn, or interrupt an active turn. Supply a stable call_id to deduplicate retries across mobile transport disconnects.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["start", "steer", "interrupt"] },
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160 },
                    "profile": { "type": "string" },
                    "thread_id": { "type": "string", "description": "Existing thread UUID; omit on action=start to create a new thread and run its first turn in 1 shot" },
                    "prompt": { "type": "string", "minLength": 1, "maxLength": 65536, "description": "Required for start and steer" },
                    "cwd": { "type": "string", "description": "Optional absolute working directory for new thread or turn" },
                    "model": { "type": "string" },
                    "model_provider": { "type": "string" },
                    "approval_policy": { "type": "string", "enum": ["untrusted", "on-failure", "on-request", "never"] },
                    "expected_turn_id": { "type": "string", "description": "Optional active turn ID for steer (auto-resolved if omitted)" },
                    "turn_id": { "type": "string", "description": "Optional active turn ID for interrupt (auto-resolved if omitted)" },
                    "wait_ms": { "type": "integer", "minimum": 0, "maximum": 120000, "description": "Bounded wait for turn settlement on start/steer (default 15000ms)" }
                },
                "required": ["action", "call_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "codex_catalog",
            "List all allowlisted extension actions callable via codex_call with their exact JSON input schemas (strictly disjoint from static tools).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "codex_call",
            "Invoke an allowlisted extension action discovered via codex_catalog. Validation failures return the exact expected JSON schema in the error body.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "minLength": 1, "maxLength": 64 },
                    "profile": { "type": "string" },
                    "arguments": { "type": "object" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            false,
        ),
    ]
}

fn build_extension_registry() -> ExtensionRegistry {
    let mut reg = ExtensionRegistry::new(STATIC_TOOL_NAMES.iter().copied());
    let specs = [
        ExtensionSpec::new(
            "thread_fork",
            "Fork an existing Codex thread into a new thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 },
                    "last_turn_id": { "type": "string" },
                    "cwd": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["thread_id"],
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "thread_archive",
            "Archive a Codex thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 }
                },
                "required": ["thread_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "thread_unarchive",
            "Unarchive a previously archived Codex thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 }
                },
                "required": ["thread_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "thread_rename",
            "Set a human-readable name on a Codex thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 },
                    "name": { "type": "string", "minLength": 1, "maxLength": 200 }
                },
                "required": ["thread_id", "name"],
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "thread_compact",
            "Trigger history compaction on a Codex thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 }
                },
                "required": ["thread_id"],
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "review_start",
            "Start a Codex code review on a thread (target: uncommittedChanges | baseBranch | commit | custom).",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 },
                    "target": { "type": "object" },
                    "delivery": { "type": "string", "enum": ["inline", "detached"] }
                },
                "required": ["thread_id", "target"],
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "account_usage",
            "Read account rate limits and usage status from the daemon (secrets redacted).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "goal_get",
            "Read the active goal attached to a thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 }
                },
                "required": ["thread_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "goal_clear",
            "Clear the active goal attached to a thread.",
            json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "string", "minLength": 1 }
                },
                "required": ["thread_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "models_list",
            "List available models reported by the Codex daemon.",
            json!({
                "type": "object",
                "properties": {
                    "model_provider": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "cursor": { "type": "string" }
                },
                "additionalProperties": false
            }),
            true,
        ),
    ];
    for spec in specs {
        reg.register(spec)
            .expect("static and extension names must be disjoint");
    }
    reg
}

fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        || matches!(name, "wrapper" | "termux")
    {
        return Err(format!("invalid profile name: '{name}'"));
    }
    Ok(())
}

fn resolve_codex_home(home: &Path, profile: Option<&str>) -> Result<(String, PathBuf), String> {
    match profile {
        None => {
            if let Some(env_home) = env::var_os("CODEX_HOME").filter(|v| !v.is_empty()) {
                return Ok(("env".to_owned(), PathBuf::from(env_home)));
            }
            Ok(("default".to_owned(), home.join(".codex")))
        }
        Some("default") | Some("home") => Ok(("default".to_owned(), home.join(".codex"))),
        Some(name) => {
            validate_profile_name(name)?;
            let dir = home
                .join(".local/share/codex/manager/profiles")
                .join(name)
                .join("home");
            let meta = fs::symlink_metadata(&dir)
                .map_err(|_| format!("profile '{name}' home not found: {}", dir.display()))?;
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err(format!("unsafe profile directory: {}", dir.display()));
            }
            Ok((name.to_owned(), dir))
        }
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> (String, bool) {
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            return (out, true);
        }
        out.push(ch);
    }
    (out, false)
}

fn extract_turn_assistant_text(turn: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(items) = turn.get("items").and_then(Value::as_array) {
        for item in items {
            let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
            if matches!(kind, "agentMessage" | "assistantMessage" | "message") {
                if let Some(t) = item.get("text").and_then(Value::as_str) {
                    if !t.is_empty() {
                        parts.push(t.to_owned());
                    }
                } else if let Some(content) = item.get("content").and_then(Value::as_array) {
                    for c in content {
                        if let Some(t) = c.get("text").and_then(Value::as_str) {
                            if !t.is_empty() {
                                parts.push(t.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    parts.join("\n")
}

fn find_active_turn_id(thread_val: &Value) -> Option<String> {
    let turns = thread_val.get("turns").and_then(Value::as_array)?;
    for turn in turns.iter().rev() {
        if turn.get("status").and_then(Value::as_str) == Some("inProgress") {
            if let Some(id) = turn.get("id").and_then(Value::as_str) {
                return Some(id.to_owned());
            }
        }
    }
    None
}

fn find_turn_by_id<'a>(thread_val: &'a Value, turn_id: &str) -> Option<&'a Value> {
    let turns = thread_val.get("turns").and_then(Value::as_array)?;
    turns
        .iter()
        .rev()
        .find(|t| t.get("id").and_then(Value::as_str) == Some(turn_id))
}

fn redact_config_summary(cfg_resp: &Value) -> Value {
    let cfg = cfg_resp.get("config").unwrap_or(cfg_resp);
    json!({
        "model": cfg.get("model").cloned().unwrap_or(Value::Null),
        "model_provider": cfg.get("model_provider").or_else(|| cfg.get("modelProvider")).cloned().unwrap_or(Value::Null),
        "approval_policy": cfg.get("approval_policy").or_else(|| cfg.get("approvalPolicy")).cloned().unwrap_or(Value::Null),
        "sandbox_mode": cfg.get("sandbox_mode").or_else(|| cfg.get("sandboxMode")).cloned().unwrap_or(Value::Null),
    })
}

fn err_payload(code: &str, detail: impl Into<String>) -> (Value, bool) {
    (
        json!({
            "state": "error",
            "error": code,
            "detail": detail.into()
        }),
        true,
    )
}

fn reject_payload(code: &str, detail: impl Into<String>) -> (Value, bool) {
    (
        json!({
            "state": "rejected",
            "executed": false,
            "error": code,
            "detail": detail.into()
        }),
        true,
    )
}

pub struct CodexMcpServer {
    home: PathBuf,
    tmpdir: PathBuf,
    receipt_root: PathBuf,
    registry: ExtensionRegistry,
}

impl CodexMcpServer {
    pub fn from_env() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is unset".to_owned())?;
        let prefix = env::var_os("PREFIX")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/data/com.termux/files/usr"));
        let tmpdir = env::var_os("TMPDIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| prefix.join("tmp"));
        let receipt_root = env::var_os("TERMUX_CODEX_RECEIPT_DIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| prefix.join("var/tmp/termux-codex-mcp-receipts-v1"));

        Ok(Self {
            home,
            tmpdir,
            receipt_root,
            registry: build_extension_registry(),
        })
    }

    #[cfg(test)]
    fn for_test(home: PathBuf, tmpdir: PathBuf, receipt_root: PathBuf) -> Self {
        Self {
            home,
            tmpdir,
            receipt_root,
            registry: build_extension_registry(),
        }
    }

    fn connect(&self, codex_home: &Path) -> Result<DaemonClient, DaemonError> {
        DaemonClient::connect(
            codex_home,
            &self.tmpdir,
            DAEMON_RPC_TIMEOUT,
            "termux-codex-mcp",
            env!("CARGO_PKG_VERSION"),
        )
    }

    fn handle_status(&self, args: &Value) -> (Value, bool) {
        if !args
            .as_object()
            .is_some_and(|m| m.keys().all(|k| k == "profile"))
        {
            return reject_payload(
                "codex_status_arguments_invalid",
                "only optional 'profile' is accepted",
            );
        }
        let profile = args.get("profile").and_then(Value::as_str);
        let (profile_label, codex_home) = match resolve_codex_home(&self.home, profile) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };
        let mut client = match self.connect(&codex_home) {
            Ok(c) => c,
            Err(DaemonError::SocketNotFound(_)) => {
                return (
                    json!({
                        "state": "ok",
                        "profile": profile_label,
                        "codex_home": codex_home.display().to_string(),
                        "daemon_running": false,
                        "loaded_threads": [],
                        "providers": [],
                        "config": null
                    }),
                    false,
                );
            }
            Err(err) => return err_payload("codex_daemon_unreachable", err.to_string()),
        };

        let loaded_ids = client.loaded_threads().unwrap_or_default();
        let mut loaded = Vec::new();
        for id in &loaded_ids {
            if let Ok(summary) = client.read_thread(id, false) {
                loaded.push(json!({
                    "id": summary.id,
                    "status": summary.status_type,
                    "active_flags": summary.active_flags,
                    "cwd": summary.cwd,
                    "name": summary.name,
                    "model_provider": summary.model_provider,
                }));
            }
        }
        let providers = client
            .list_model_providers()
            .ok()
            .and_then(|v| v.get("data").cloned())
            .unwrap_or_else(|| json!([]));
        let config = client
            .call("config/read", json!({}))
            .ok()
            .map(|v| redact_config_summary(&v))
            .unwrap_or(Value::Null);
        client.close();

        (
            json!({
                "state": "ok",
                "profile": profile_label,
                "codex_home": codex_home.display().to_string(),
                "daemon_running": true,
                "loaded_threads": loaded,
                "providers": providers,
                "config": config
            }),
            false,
        )
    }

    fn handle_threads(&self, args: &Value) -> (Value, bool) {
        if !args.as_object().is_some_and(|m| {
            m.keys().all(|k| {
                matches!(
                    k.as_str(),
                    "action"
                        | "profile"
                        | "thread_id"
                        | "turn_id"
                        | "include_turns"
                        | "limit"
                        | "cursor"
                )
            })
        }) {
            return reject_payload(
                "codex_threads_arguments_invalid",
                "unsupported argument key",
            );
        }
        let Some(action) = args.get("action").and_then(Value::as_str) else {
            return reject_payload(
                "codex_threads_action_required",
                "action must be list, read, or items",
            );
        };
        let profile = args.get("profile").and_then(Value::as_str);
        let (_, codex_home) = match resolve_codex_home(&self.home, profile) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };
        let mut client = match self.connect(&codex_home) {
            Ok(c) => c,
            Err(DaemonError::SocketNotFound(_)) => {
                return err_payload(
                    "codex_daemon_not_running",
                    "codex app-server daemon is not running",
                );
            }
            Err(err) => return err_payload("codex_daemon_unreachable", err.to_string()),
        };

        let result = match action {
            "list" => {
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(20)
                    .clamp(1, 100) as u32;
                let cursor = args.get("cursor").and_then(Value::as_str);
                match client.list_threads(limit, cursor) {
                    Ok(page) => (
                        json!({
                            "state": "ok",
                            "threads": page.threads,
                            "next_cursor": page.next_cursor
                        }),
                        false,
                    ),
                    Err(e) => err_payload("codex_rpc_failed", e.to_string()),
                }
            }
            "read" => {
                let Some(tid) = args
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                else {
                    client.close();
                    return reject_payload(
                        "codex_thread_id_required",
                        "thread_id is required for action=read",
                    );
                };
                let include_turns = args
                    .get("include_turns")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                match client.read_thread_raw(tid, include_turns) {
                    Ok(raw) => (json!({"state": "ok", "data": raw}), false),
                    Err(e) => err_payload("codex_rpc_failed", e.to_string()),
                }
            }
            "items" => {
                let Some(tid) = args
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                else {
                    client.close();
                    return reject_payload(
                        "codex_thread_id_required",
                        "thread_id is required for action=items",
                    );
                };
                let mut params = json!({ "threadId": tid });
                if let Some(turn_id) = args.get("turn_id").and_then(Value::as_str) {
                    params["turnId"] = json!(turn_id);
                }
                if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                    params["limit"] = json!(limit.clamp(1, 100));
                }
                if let Some(cursor) = args.get("cursor").and_then(Value::as_str) {
                    params["cursor"] = json!(cursor);
                }
                match client.call("thread/items/list", params) {
                    Ok(raw) => (json!({"state": "ok", "data": raw}), false),
                    Err(e) => err_payload("codex_rpc_failed", e.to_string()),
                }
            }
            other => reject_payload(
                "codex_threads_action_invalid",
                format!("unsupported action '{other}'; expected list, read, or items"),
            ),
        };
        client.close();
        result
    }

    fn poll_turn_completion(
        client: &mut DaemonClient,
        thread_id: &str,
        turn_id: Option<&str>,
        wait_ms: u64,
    ) -> Value {
        let started = Instant::now();
        let deadline = started + Duration::from_millis(wait_ms);
        let mut last_status = "inProgress".to_owned();
        let mut last_text = String::new();
        let mut settled = false;

        loop {
            if let Ok(raw) = client.read_thread_raw(thread_id, true) {
                let thread = raw.get("thread").unwrap_or(&raw);
                let target_turn = match turn_id {
                    Some(tid) => find_turn_by_id(thread, tid),
                    None => thread
                        .get("turns")
                        .and_then(Value::as_array)
                        .and_then(|a| a.last()),
                };
                if let Some(turn) = target_turn {
                    let st = turn
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("inProgress");
                    last_status = st.to_owned();
                    last_text = extract_turn_assistant_text(turn);
                    if st != "inProgress" {
                        settled = true;
                        break;
                    }
                }
            }
            if wait_ms == 0 || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(120));
        }

        let (text, truncated) = truncate_chars(&last_text, MAX_ASSISTANT_TEXT_CHARS);
        json!({
            "state": if settled { "settled" } else { "in_progress" },
            "executed": true,
            "thread_id": thread_id,
            "turn_id": turn_id,
            "turn_status": last_status,
            "assistant_text": text,
            "assistant_text_truncated": truncated,
            "waited_ms": started.elapsed().as_millis() as u64
        })
    }

    fn handle_turn(&self, args: &Value) -> (Value, bool) {
        if !args.as_object().is_some_and(|m| {
            m.keys().all(|k| {
                matches!(
                    k.as_str(),
                    "action"
                        | "call_id"
                        | "profile"
                        | "thread_id"
                        | "prompt"
                        | "cwd"
                        | "model"
                        | "model_provider"
                        | "approval_policy"
                        | "expected_turn_id"
                        | "turn_id"
                        | "wait_ms"
                )
            })
        }) {
            return reject_payload("codex_turn_arguments_invalid", "unsupported argument key");
        }
        let call_id = match validate_call_id(args) {
            Ok(id) => id,
            Err(_) => {
                return reject_payload(
                    "codex_call_id_invalid",
                    "call_id must be 1..=160 bytes without NUL",
                )
            }
        };
        let Some(action) = args.get("action").and_then(Value::as_str) else {
            return reject_payload(
                "codex_turn_action_required",
                "action must be start, steer, or interrupt",
            );
        };
        if !matches!(action, "start" | "steer" | "interrupt") {
            return reject_payload(
                "codex_turn_action_invalid",
                format!("unsupported action '{action}'"),
            );
        }

        let prompt = args.get("prompt").and_then(Value::as_str);
        if matches!(action, "start" | "steer") {
            match prompt {
                Some(p)
                    if !p.trim().is_empty() && p.len() <= MAX_PROMPT_BYTES && !p.contains('\0') => {
                }
                _ => {
                    return reject_payload(
                        "codex_prompt_invalid",
                        "prompt must be non-empty <= 64 KiB",
                    )
                }
            }
        }
        if matches!(action, "steer" | "interrupt")
            && args
                .get("thread_id")
                .and_then(Value::as_str)
                .is_none_or(|s| s.is_empty())
        {
            return reject_payload(
                "codex_thread_id_required",
                format!("thread_id is required for action={action}"),
            );
        }
        if let Some(cwd_val) = args.get("cwd") {
            match cwd_val.as_str() {
                Some(c)
                    if Path::new(c).is_absolute() && !c.contains('\0') && Path::new(c).is_dir() => {
                }
                _ => {
                    return reject_payload(
                        "codex_cwd_invalid",
                        "cwd must be an existing absolute directory",
                    )
                }
            }
        }
        let wait_ms = match args.get("wait_ms") {
            None => DEFAULT_WAIT_MS,
            Some(v) => match v.as_u64() {
                Some(ms) if ms <= MAX_WAIT_MS => ms,
                _ => return reject_payload("codex_wait_ms_invalid", "wait_ms must be 0..=120000"),
            },
        };

        let profile = args.get("profile").and_then(Value::as_str);
        let (_, codex_home) = match resolve_codex_home(&self.home, profile) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };

        // Claim atomic `call_id` transport deduplication receipt BEFORE any mutating RPC!
        let store = match ReceiptStore::open(
            self.receipt_root.clone(),
            RECEIPT_SCHEMA,
            "codex_call_id_payload_conflict",
        ) {
            Ok(s) => s,
            Err(e) => return reject_payload(e, "receipt store unavailable"),
        };
        let fp = sha256_hex(
            json!([
                "termux-codex-turn-v1",
                action,
                profile,
                args.get("thread_id"),
                args.get("prompt"),
                args.get("cwd"),
                args.get("model"),
                args.get("model_provider"),
                args.get("approval_policy"),
                args.get("expected_turn_id"),
                args.get("turn_id"),
            ])
            .to_string()
            .as_bytes(),
        );

        let receipt_path = match store.claim(call_id, &fp, &[&fp]) {
            ReceiptClaim::Acquired(p) => p,
            ReceiptClaim::Existing(v) => {
                let is_err = matches!(
                    v["state"].as_str(),
                    Some("rejected" | "delivery_unknown" | "error")
                );
                return (v, is_err);
            }
            ReceiptClaim::Rejected(e) => {
                return reject_payload(e, "could not acquire call_id receipt")
            }
        };

        let mut client = match self.connect(&codex_home) {
            Ok(c) => c,
            Err(err) => {
                let out = json!({
                    "state": "error",
                    "executed": false,
                    "error": "codex_daemon_unreachable",
                    "detail": err.to_string(),
                    "replay_safe": true
                });
                let _ = store.commit(&receipt_path, &fp, &out);
                return (out, true);
            }
        };

        let mut outcome = match action {
            "start" => {
                let thread_id = if let Some(tid) = args
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    let loaded = client.loaded_threads().unwrap_or_default();
                    if !loaded.iter().any(|x| x == tid) {
                        if let Err(err) = client.call("thread/resume", json!({ "threadId": tid })) {
                            client.close();
                            let out = json!({
                                "state": "error",
                                "executed": false,
                                "error": "codex_thread_resume_failed",
                                "detail": err.to_string(),
                                "replay_safe": true
                            });
                            let _ = store.commit(&receipt_path, &fp, &out);
                            return (out, true);
                        }
                    }
                    tid.to_owned()
                } else {
                    // 1-shot new thread creation!
                    let cwd_str = args
                        .get("cwd")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| self.home.display().to_string());
                    let mut sp = json!({ "cwd": cwd_str });
                    if let Some(m) = args.get("model").and_then(Value::as_str) {
                        sp["model"] = json!(m);
                    }
                    if let Some(mp) = args.get("model_provider").and_then(Value::as_str) {
                        sp["modelProvider"] = json!(mp);
                    }
                    if let Some(ap) = args.get("approval_policy").and_then(Value::as_str) {
                        sp["approvalPolicy"] = json!(ap);
                    }
                    match client.call("thread/start", sp) {
                        Ok(res) => match res
                            .get("thread")
                            .and_then(|t| t.get("id"))
                            .and_then(Value::as_str)
                        {
                            Some(id) => id.to_owned(),
                            None => {
                                client.close();
                                let out = json!({
                                    "state": "error",
                                    "executed": false,
                                    "error": "codex_thread_start_malformed",
                                    "replay_safe": true
                                });
                                let _ = store.commit(&receipt_path, &fp, &out);
                                return (out, true);
                            }
                        },
                        Err(err) => {
                            client.close();
                            let out = json!({
                                "state": "error",
                                "executed": false,
                                "error": "codex_thread_start_failed",
                                "detail": err.to_string(),
                                "replay_safe": true
                            });
                            let _ = store.commit(&receipt_path, &fp, &out);
                            return (out, true);
                        }
                    }
                };

                let mut tp = json!({
                    "threadId": thread_id,
                    "input": [{ "type": "text", "text": prompt.unwrap() }]
                });
                if let Some(c) = args.get("cwd").and_then(Value::as_str) {
                    tp["cwd"] = json!(c);
                }
                if let Some(m) = args.get("model").and_then(Value::as_str) {
                    tp["model"] = json!(m);
                }
                if let Some(ap) = args.get("approval_policy").and_then(Value::as_str) {
                    tp["approvalPolicy"] = json!(ap);
                }
                match client.call("turn/start", tp) {
                    Ok(res) => {
                        let turn_id = res
                            .get("turn")
                            .and_then(|t| t.get("id"))
                            .or_else(|| res.get("turnId"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned);
                        Self::poll_turn_completion(
                            &mut client,
                            &thread_id,
                            turn_id.as_deref(),
                            wait_ms,
                        )
                    }
                    Err(err) => json!({
                        "state": "error",
                        "executed": false,
                        "thread_id": thread_id,
                        "error": "codex_turn_start_failed",
                        "detail": err.to_string()
                    }),
                }
            }
            "steer" => {
                let tid = args["thread_id"].as_str().unwrap();
                let expected = match args.get("expected_turn_id").and_then(Value::as_str) {
                    Some(v) if !v.is_empty() => Some(v.to_owned()),
                    _ => client
                        .read_thread_raw(tid, true)
                        .ok()
                        .and_then(|r| find_active_turn_id(r.get("thread").unwrap_or(&r))),
                };
                match expected {
                    Some(expected_turn_id) => match client.call(
                        "turn/steer",
                        json!({
                            "threadId": tid,
                            "expectedTurnId": expected_turn_id,
                            "input": [{ "type": "text", "text": prompt.unwrap() }]
                        }),
                    ) {
                        Ok(_) => Self::poll_turn_completion(
                            &mut client,
                            tid,
                            Some(&expected_turn_id),
                            wait_ms.min(15_000),
                        ),
                        Err(err) => json!({
                            "state": "error",
                            "executed": false,
                            "thread_id": tid,
                            "error": "codex_turn_steer_failed",
                            "detail": err.to_string()
                        }),
                    },
                    None => json!({
                        "state": "rejected",
                        "executed": false,
                        "thread_id": tid,
                        "error": "codex_no_active_turn_to_steer"
                    }),
                }
            }
            "interrupt" => {
                let tid = args["thread_id"].as_str().unwrap();
                let turn_id = match args.get("turn_id").and_then(Value::as_str) {
                    Some(v) if !v.is_empty() => Some(v.to_owned()),
                    _ => client
                        .read_thread_raw(tid, true)
                        .ok()
                        .and_then(|r| find_active_turn_id(r.get("thread").unwrap_or(&r))),
                };
                match turn_id {
                    Some(active_id) => match client.call(
                        "turn/interrupt",
                        json!({ "threadId": tid, "turnId": active_id }),
                    ) {
                        Ok(_) => json!({
                            "state": "interrupted",
                            "executed": true,
                            "thread_id": tid,
                            "turn_id": active_id
                        }),
                        Err(err) => json!({
                            "state": "error",
                            "executed": false,
                            "thread_id": tid,
                            "error": "codex_turn_interrupt_failed",
                            "detail": err.to_string()
                        }),
                    },
                    None => json!({
                        "state": "idle",
                        "executed": false,
                        "thread_id": tid,
                        "detail": "no active turn in progress"
                    }),
                }
            }
            _ => unreachable!(),
        };

        client.close();
        outcome["replay_safe"] = json!(true);
        if let Err(err) = store.commit(&receipt_path, &fp, &outcome) {
            outcome["receipt_error"] = json!(err);
            outcome["replay_safe"] = json!(false);
        } else {
            store.prune_settled(512);
        }
        let is_err = matches!(
            outcome["state"].as_str(),
            Some("error" | "rejected" | "delivery_unknown")
        );
        (outcome, is_err)
    }

    fn handle_call_extension(&self, args: &Value) -> (Value, bool) {
        if !args.as_object().is_some_and(|m| {
            m.keys()
                .all(|k| matches!(k.as_str(), "action" | "profile" | "arguments"))
        }) {
            return reject_payload(
                "codex_call_arguments_invalid",
                "expected action, optional profile, optional arguments",
            );
        }
        let Some(action) = args.get("action").and_then(Value::as_str) else {
            return reject_payload("codex_call_action_required", "action is required");
        };
        if self.registry.get(action).is_none() {
            return (
                self.registry
                    .validation_error(action, "unknown_extension_action"),
                true,
            );
        }
        let empty = Value::Object(Map::new());
        let ext_args = args.get("arguments").unwrap_or(&empty);
        let Some(ext_obj) = ext_args.as_object() else {
            return (
                self.registry
                    .validation_error(action, "arguments must be a JSON object"),
                true,
            );
        };

        let profile = args.get("profile").and_then(Value::as_str);
        let (_, codex_home) = match resolve_codex_home(&self.home, profile) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };

        // Validate arguments against the exact action contract BEFORE opening daemon RPC!
        let (rpc_method, rpc_params) = match action {
            "thread_fork" => {
                let Some(tid) = ext_obj
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                else {
                    return (
                        self.registry
                            .validation_error(action, "missing required string 'thread_id'"),
                        true,
                    );
                };
                let mut p = json!({ "threadId": tid });
                if let Some(v) = ext_obj.get("last_turn_id").and_then(Value::as_str) {
                    p["lastTurnId"] = json!(v);
                }
                if let Some(v) = ext_obj.get("cwd").and_then(Value::as_str) {
                    p["cwd"] = json!(v);
                }
                if let Some(v) = ext_obj.get("model").and_then(Value::as_str) {
                    p["model"] = json!(v);
                }
                ("thread/fork", p)
            }
            "thread_archive" | "thread_unarchive" | "thread_compact" | "goal_get"
            | "goal_clear" => {
                let Some(tid) = ext_obj
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                else {
                    return (
                        self.registry
                            .validation_error(action, "missing required string 'thread_id'"),
                        true,
                    );
                };
                let method = match action {
                    "thread_archive" => "thread/archive",
                    "thread_unarchive" => "thread/unarchive",
                    "thread_compact" => "thread/compact/start",
                    "goal_get" => "thread/goal/get",
                    "goal_clear" => "thread/goal/clear",
                    _ => unreachable!(),
                };
                (method, json!({ "threadId": tid }))
            }
            "thread_rename" => {
                let tid = ext_obj
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty());
                let name = ext_obj
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty() && s.len() <= 200);
                let (Some(tid), Some(name)) = (tid, name) else {
                    return (
                        self.registry
                            .validation_error(action, "requires non-empty 'thread_id' and 'name'"),
                        true,
                    );
                };
                ("thread/name/set", json!({ "threadId": tid, "name": name }))
            }
            "review_start" => {
                let tid = ext_obj
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty());
                let target = ext_obj.get("target").filter(|v| v.is_object());
                let (Some(tid), Some(target)) = (tid, target) else {
                    return (
                        self.registry
                            .validation_error(action, "requires 'thread_id' and object 'target'"),
                        true,
                    );
                };
                let mut p = json!({ "threadId": tid, "target": target });
                if let Some(d) = ext_obj.get("delivery").and_then(Value::as_str) {
                    p["delivery"] = json!(d);
                }
                ("review/start", p)
            }
            "account_usage" => ("account/rateLimits/read", json!({})),
            "models_list" => {
                let mut p = json!({});
                if let Some(mp) = ext_obj.get("model_provider").and_then(Value::as_str) {
                    p["modelProvider"] = json!(mp);
                }
                if let Some(l) = ext_obj.get("limit").and_then(Value::as_u64) {
                    p["limit"] = json!(l.clamp(1, 100));
                }
                if let Some(c) = ext_obj.get("cursor").and_then(Value::as_str) {
                    p["cursor"] = json!(c);
                }
                ("model/list", p)
            }
            _ => unreachable!(),
        };

        let mut client = match self.connect(&codex_home) {
            Ok(c) => c,
            Err(DaemonError::SocketNotFound(_)) => {
                return err_payload(
                    "codex_daemon_not_running",
                    "codex app-server daemon is not running",
                );
            }
            Err(err) => return err_payload("codex_daemon_unreachable", err.to_string()),
        };
        let res = match client.call(rpc_method, rpc_params) {
            Ok(data) => (
                json!({"state": "ok", "action": action, "data": data}),
                false,
            ),
            Err(err) => err_payload("codex_rpc_failed", err.to_string()),
        };
        client.close();
        res
    }
}

impl McpHandler for CodexMcpServer {
    fn server_name(&self) -> &str {
        "TermuxCodex"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<ToolDef> {
        static_tools()
    }

    fn call_tool(&mut self, name: &str, arguments: &Value) -> (Value, bool) {
        match name {
            "codex_status" => self.handle_status(arguments),
            "codex_threads" => self.handle_threads(arguments),
            "codex_turn" => self.handle_turn(arguments),
            "codex_catalog" => (self.registry.catalog_json(), false),
            "codex_call" => self.handle_call_extension(arguments),
            _ => reject_payload("codex_tool_unknown", format!("unknown tool '{name}'")),
        }
    }
}

fn print_help() {
    println!(
        "termux-codex-mcp {} — stateless @TermuxCodex MCP stdio worker\n\n\
         Usage:\n\
         \x20 termux-codex-mcp                   Run MCP stdio JSON-RPC 2.0 server\n\
         \x20 termux-codex-mcp --describe-tools  Print static 5-tool tools/list JSON and exit\n\
         \x20 termux-codex-mcp --version         Print version and exit\n\
         \x20 termux-codex-mcp --help            Show this help",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("-h" | "--help") if args.len() == 1 => {
            print_help();
            return;
        }
        Some("-V" | "--version") if args.len() == 1 => {
            println!("termux-codex-mcp {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--describe-tools") if args.len() == 1 => {
            let tools: Vec<Value> = static_tools().iter().map(ToolDef::to_mcp_json).collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "tools": tools })).unwrap()
            );
            return;
        }
        None => {}
        Some(other) => {
            eprintln!("termux-codex-mcp: unknown argument '{other}' (try --help)");
            std::process::exit(2);
        }
    }

    let mut server = match CodexMcpServer::from_env() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("termux-codex-mcp: startup error: {err}");
            std::process::exit(1);
        }
    };
    if let Err(err) = serve_stdio(&mut server) {
        eprintln!("termux-codex-mcp: stdio loop error: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_exactly_five_static_tools_with_uniform_annotations_and_disjoint_extensions() {
        let tools = static_tools();
        assert_eq!(tools.len(), 5);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, STATIC_TOOL_NAMES);

        for tool in &tools {
            let j = tool.to_mcp_json();
            assert_eq!(j["annotations"]["readOnlyHint"], true);
            assert_eq!(j["annotations"]["destructiveHint"], false);
            assert_eq!(j["annotations"]["openWorldHint"], false);
        }
        assert!(tools[0].idempotent); // codex_status
        assert!(tools[1].idempotent); // codex_threads
        assert!(tools[2].idempotent); // codex_turn (call_id deduplicated)
        assert!(tools[3].idempotent); // codex_catalog
        assert!(!tools[4].idempotent); // codex_call

        let reg = build_extension_registry();
        let cat = reg.catalog_json();
        let actions = cat["actions"].as_array().unwrap();
        assert!(actions.len() >= 9);
        for a in actions {
            let act_name = a["action"].as_str().unwrap();
            assert!(!STATIC_TOOL_NAMES.contains(&act_name));
        }
    }

    #[test]
    fn stopped_daemon_returns_structured_status_and_schema_errors_self_correct() {
        let dir = env::temp_dir().join(format!("termux-codex-unit-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut srv = CodexMcpServer::for_test(dir.clone(), dir.clone(), dir.join("rcpt"));

        let (st, is_err) = srv.call_tool("codex_status", &json!({}));
        assert!(!is_err);
        assert_eq!(st["daemon_running"], false);

        // Missing thread_id on extension action returns exact expected_schema!
        let (bad, is_err2) = srv.call_tool(
            "codex_call",
            &json!({"action": "thread_rename", "arguments": {}}),
        );
        assert!(is_err2);
        assert_eq!(
            bad["expected_schema"]["required"],
            json!(["thread_id", "name"])
        );

        // Unknown extension action lists available_actions
        let (unk, is_err3) = srv.call_tool("codex_call", &json!({"action": "not_a_real_action"}));
        assert!(is_err3);
        assert!(unk["available_actions"].as_array().unwrap().len() >= 9);

        let _ = fs::remove_dir_all(&dir);
    }
}
