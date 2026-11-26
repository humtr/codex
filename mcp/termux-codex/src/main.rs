//! `@TermuxCodex` — stateless thin MCP bridge over the official `codex app-server daemon`.
//!
//! Built on `crates/codex-daemon-client` (direct RFC 6455 WebSocket-over-UDS with
//! Termux FD 35 resolution) and `crates/mcp-kit` (uniform MCP annotations,
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

const RECEIPT_SCHEMA: &str = "termux-codex-mcp-receipt";
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
                    "delivery": { "type": "string", "enum": ["inline"] }
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

const MAX_READ_RESPONSE_BYTES: usize = 512 * 1024;
const DEFAULT_ITEMS_LIMIT: u64 = 20;

fn check_allowed_keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported argument key '{key}'"));
        }
    }
    Ok(())
}

fn optional_nonempty_str<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(v) => match v.as_str() {
            Some(s) if !s.is_empty() && !s.contains('\0') => Ok(Some(s)),
            _ => Err(format!("'{key}' must be a non-empty string without NUL")),
        },
    }
}

fn optional_bool(obj: &Map<String, Value>, key: &str) -> Result<Option<bool>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(v) => match v.as_bool() {
            Some(b) => Ok(Some(b)),
            None => Err(format!("'{key}' must be a boolean")),
        },
    }
}

fn optional_bounded_u64(
    obj: &Map<String, Value>,
    key: &str,
    min: u64,
    max: u64,
) -> Result<Option<u64>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(v) => match v.as_u64() {
            Some(n) if (min..=max).contains(&n) => Ok(Some(n)),
            _ => Err(format!("'{key}' must be an integer in {min}..={max}")),
        },
    }
}

fn validate_cwd_dir(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.is_absolute() || path.contains('\0') || !p.is_dir() {
        return Err("cwd must be an existing absolute directory".to_owned());
    }
    Ok(())
}

fn validate_review_target(target: &Value) -> Result<(), String> {
    let Some(obj) = target.as_object() else {
        return Err("target must be a JSON object".to_owned());
    };
    let Some(kind) = obj.get("type").and_then(Value::as_str) else {
        return Err(
            "target.type must be uncommittedChanges, baseBranch, commit, or custom".to_owned(),
        );
    };
    match kind {
        "uncommittedChanges" => check_allowed_keys(obj, &["type"]),
        "baseBranch" => {
            check_allowed_keys(obj, &["type", "branch"])?;
            if optional_nonempty_str(obj, "branch")?.is_none() {
                return Err("target.branch is required for baseBranch".to_owned());
            }
            Ok(())
        }
        "commit" => {
            check_allowed_keys(obj, &["type", "sha", "title"])?;
            if optional_nonempty_str(obj, "sha")?.is_none() {
                return Err("target.sha is required for commit".to_owned());
            }
            if let Some(title) = obj.get("title") {
                if !title.is_null() && !title.is_string() {
                    return Err("target.title must be a string or null".to_owned());
                }
            }
            Ok(())
        }
        "custom" => {
            check_allowed_keys(obj, &["type", "instructions"])?;
            if optional_nonempty_str(obj, "instructions")?.is_none() {
                return Err("target.instructions is required for custom".to_owned());
            }
            Ok(())
        }
        other => Err(format!("unsupported target.type '{other}'")),
    }
}

fn is_fatal_daemon_transport(err: &DaemonError) -> bool {
    matches!(
        err,
        DaemonError::Io(_)
            | DaemonError::SocketNotFound(_)
            | DaemonError::Handshake(_)
            | DaemonError::Protocol(_)
    )
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
            let profile_root = home.join(".local/share/codex/manager/profiles").join(name);
            let root_meta = fs::symlink_metadata(&profile_root)
                .map_err(|_| format!("profile '{name}' not found: {}", profile_root.display()))?;
            if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
                return Err(format!(
                    "unsafe profile directory: {}",
                    profile_root.display()
                ));
            }
            let dir = profile_root.join("home");
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

fn redact_providers_summary(resp: &Value) -> (Value, Value) {
    let default_provider = resp
        .get("defaultProviderId")
        .or_else(|| resp.get("default_provider_id"))
        .cloned()
        .unwrap_or(Value::Null);
    let providers = resp
        .get("data")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|p| {
                    json!({
                        "id": p.get("id").cloned().unwrap_or(Value::Null),
                        "name": p.get("name").cloned().unwrap_or(Value::Null),
                        "base_url": p.get("baseUrl").or_else(|| p.get("base_url")).cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (default_provider, Value::Array(providers))
}

fn check_bounded_response(raw: &Value, hint: &str) -> Result<(), (Value, bool)> {
    let encoded_len = serde_json::to_string(raw).map(|s| s.len()).unwrap_or(0);
    if encoded_len > MAX_READ_RESPONSE_BYTES {
        return Err(err_payload(
            "codex_response_too_large",
            format!("response ({encoded_len} bytes) exceeds {MAX_READ_RESPONSE_BYTES} byte limit; {hint}"),
        ));
    }
    Ok(())
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
            .unwrap_or_else(|| prefix.join("var/tmp/termux-codex-mcp-receipts"));

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
        let Some(obj) = args.as_object() else {
            return reject_payload(
                "codex_status_arguments_invalid",
                "arguments must be a JSON object",
            );
        };
        if let Err(e) = check_allowed_keys(obj, &["profile"]) {
            return reject_payload("codex_status_arguments_invalid", e);
        }
        let profile = match optional_nonempty_str(obj, "profile") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };
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
                        "default_provider": null,
                        "providers": [],
                        "config": null,
                        "warnings": []
                    }),
                    false,
                );
            }
            Err(err) => return err_payload("codex_daemon_unreachable", err.to_string()),
        };

        let mut warnings: Vec<String> = Vec::new();
        let loaded_ids = match client.loaded_threads() {
            Ok(ids) => ids,
            Err(err) if is_fatal_daemon_transport(&err) => {
                client.close();
                return err_payload("codex_daemon_unreachable", err.to_string());
            }
            Err(err) => {
                warnings.push(format!("thread/loaded/list failed: {err}"));
                Vec::new()
            }
        };
        let mut loaded = Vec::new();
        for id in &loaded_ids {
            match client.read_thread(id, false) {
                Ok(summary) => loaded.push(json!({
                    "id": summary.id,
                    "status": summary.status_type,
                    "active_flags": summary.active_flags,
                    "cwd": summary.cwd,
                    "name": summary.name,
                    "model_provider": summary.model_provider,
                })),
                Err(err) => warnings.push(format!("thread/read({id}) failed: {err}")),
            }
        }
        let (default_provider, providers) = match client.list_model_providers() {
            Ok(v) => redact_providers_summary(&v),
            Err(err) => {
                warnings.push(format!("modelProvider/list failed: {err}"));
                (Value::Null, json!([]))
            }
        };
        let config = match client.call("config/read", json!({})) {
            Ok(v) => redact_config_summary(&v),
            Err(err) => {
                warnings.push(format!("config/read failed: {err}"));
                Value::Null
            }
        };
        client.close();

        (
            json!({
                "state": "ok",
                "profile": profile_label,
                "codex_home": codex_home.display().to_string(),
                "daemon_running": true,
                "loaded_threads": loaded,
                "default_provider": default_provider,
                "providers": providers,
                "config": config,
                "warnings": warnings
            }),
            false,
        )
    }

    fn handle_threads(&self, args: &Value) -> (Value, bool) {
        let Some(obj) = args.as_object() else {
            return reject_payload(
                "codex_threads_arguments_invalid",
                "arguments must be a JSON object",
            );
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "profile",
                "thread_id",
                "turn_id",
                "include_turns",
                "limit",
                "cursor",
            ],
        ) {
            return reject_payload("codex_threads_arguments_invalid", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return reject_payload(
                "codex_threads_action_required",
                "action must be list, read, or items",
            );
        };
        if !matches!(action, "list" | "read" | "items") {
            return reject_payload(
                "codex_threads_action_invalid",
                format!("unsupported action '{action}'; expected list, read, or items"),
            );
        }
        let profile = match optional_nonempty_str(obj, "profile") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };
        let thread_id = match optional_nonempty_str(obj, "thread_id") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_thread_id_invalid", e),
        };
        let turn_id = match optional_nonempty_str(obj, "turn_id") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_turn_id_invalid", e),
        };
        let include_turns = match optional_bool(obj, "include_turns") {
            Ok(v) => v.unwrap_or(false),
            Err(e) => return reject_payload("codex_threads_arguments_invalid", e),
        };
        let limit = match optional_bounded_u64(obj, "limit", 1, 100) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_threads_arguments_invalid", e),
        };
        let cursor = match optional_nonempty_str(obj, "cursor") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_threads_arguments_invalid", e),
        };
        if matches!(action, "read" | "items") && thread_id.is_none() {
            return reject_payload(
                "codex_thread_id_required",
                format!("thread_id is required for action={action}"),
            );
        }

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
            "list" => match client.list_threads(limit.unwrap_or(20) as u32, cursor) {
                Ok(page) => (
                    json!({
                        "state": "ok",
                        "threads": page.threads,
                        "next_cursor": page.next_cursor
                    }),
                    false,
                ),
                Err(e) => err_payload("codex_rpc_failed", e.to_string()),
            },
            "read" => {
                let tid = thread_id.unwrap();
                match client.read_thread_raw(tid, include_turns) {
                    Ok(raw) => match check_bounded_response(
                        &raw,
                        "retry with include_turns=false and paginate via action=items",
                    ) {
                        Ok(()) => (json!({"state": "ok", "data": raw}), false),
                        Err(e) => e,
                    },
                    Err(e) => err_payload("codex_rpc_failed", e.to_string()),
                }
            }
            "items" => {
                let tid = thread_id.unwrap();
                let mut params = json!({
                    "threadId": tid,
                    "limit": limit.unwrap_or(DEFAULT_ITEMS_LIMIT)
                });
                if let Some(turn_id) = turn_id {
                    params["turnId"] = json!(turn_id);
                }
                if let Some(cursor) = cursor {
                    params["cursor"] = json!(cursor);
                }
                match client.call("thread/items/list", params) {
                    Ok(raw) => match check_bounded_response(&raw, "retry with a smaller limit") {
                        Ok(()) => (json!({"state": "ok", "data": raw}), false),
                        Err(e) => e,
                    },
                    Err(e) => err_payload("codex_rpc_failed", e.to_string()),
                }
            }
            _ => unreachable!(),
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
        let mut poll_error: Option<String> = None;

        loop {
            match client.read_thread_raw(thread_id, true) {
                Ok(raw) => {
                    if let Err((payload, _)) = check_bounded_response(&raw, "turn polling") {
                        poll_error = Some(
                            payload["detail"]
                                .as_str()
                                .unwrap_or("turn polling response too large")
                                .to_owned(),
                        );
                        break;
                    }
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
                Err(err) if is_fatal_daemon_transport(&err) => {
                    poll_error = Some(err.to_string());
                    break;
                }
                Err(err) => {
                    poll_error = Some(err.to_string());
                }
            }
            if wait_ms == 0 || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(120));
        }

        let (text, truncated) = truncate_chars(&last_text, MAX_ASSISTANT_TEXT_CHARS);
        let state = if settled {
            "settled"
        } else if poll_error.is_some() {
            "delivery_unknown"
        } else {
            "in_progress"
        };
        let mut out = json!({
            "state": state,
            "executed": true,
            "thread_id": thread_id,
            "turn_id": turn_id,
            "turn_status": if poll_error.is_some() && !settled { "unknown" } else { last_status.as_str() },
            "assistant_text": text,
            "assistant_text_truncated": truncated,
            "waited_ms": started.elapsed().as_millis() as u64
        });
        if let Some(err) = poll_error {
            out["poll_error"] = json!(err);
        }
        out
    }

    fn handle_turn(&self, args: &Value) -> (Value, bool) {
        let Some(obj) = args.as_object() else {
            return reject_payload(
                "codex_turn_arguments_invalid",
                "arguments must be a JSON object",
            );
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "call_id",
                "profile",
                "thread_id",
                "prompt",
                "cwd",
                "model",
                "model_provider",
                "approval_policy",
                "expected_turn_id",
                "turn_id",
                "wait_ms",
            ],
        ) {
            return reject_payload("codex_turn_arguments_invalid", e);
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
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
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

        let prompt = match optional_nonempty_str(obj, "prompt") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_prompt_invalid", e),
        };
        if matches!(action, "start" | "steer") {
            match prompt {
                Some(p) if !p.trim().is_empty() && p.len() <= MAX_PROMPT_BYTES => {}
                _ => {
                    return reject_payload(
                        "codex_prompt_invalid",
                        "prompt must be non-empty <= 64 KiB",
                    )
                }
            }
        }
        let profile = match optional_nonempty_str(obj, "profile") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };
        let thread_id_arg = match optional_nonempty_str(obj, "thread_id") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_thread_id_invalid", e),
        };
        if matches!(action, "steer" | "interrupt") && thread_id_arg.is_none() {
            return reject_payload(
                "codex_thread_id_required",
                format!("thread_id is required for action={action}"),
            );
        }
        let cwd_arg = match optional_nonempty_str(obj, "cwd") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_cwd_invalid", e),
        };
        if let Some(c) = cwd_arg {
            if let Err(e) = validate_cwd_dir(c) {
                return reject_payload("codex_cwd_invalid", e);
            }
        }
        let model_arg = match optional_nonempty_str(obj, "model") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_turn_arguments_invalid", e),
        };
        let model_provider_arg = match optional_nonempty_str(obj, "model_provider") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_turn_arguments_invalid", e),
        };
        let approval_policy_arg = match optional_nonempty_str(obj, "approval_policy") {
            Ok(Some(ap))
                if matches!(ap, "untrusted" | "on-failure" | "on-request" | "never") =>
            {
                Some(ap)
            }
            Ok(None) => None,
            Ok(Some(other)) => {
                return reject_payload(
                    "codex_approval_policy_invalid",
                    format!(
                        "unsupported approval_policy '{other}'; expected untrusted, on-failure, on-request, or never"
                    ),
                )
            }
            Err(e) => return reject_payload("codex_approval_policy_invalid", e),
        };
        let expected_turn_id_arg = match optional_nonempty_str(obj, "expected_turn_id") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_turn_arguments_invalid", e),
        };
        let turn_id_arg = match optional_nonempty_str(obj, "turn_id") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_turn_arguments_invalid", e),
        };
        let wait_ms = match optional_bounded_u64(obj, "wait_ms", 0, MAX_WAIT_MS) {
            Ok(v) => v.unwrap_or(DEFAULT_WAIT_MS),
            Err(e) => return reject_payload("codex_wait_ms_invalid", e),
        };

        let (_, codex_home) = match resolve_codex_home(&self.home, profile) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };

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
                "termux-codex-turn",
                codex_home.display().to_string(),
                action,
                profile,
                thread_id_arg,
                prompt,
                cwd_arg,
                model_arg,
                model_provider_arg,
                approval_policy_arg,
                expected_turn_id_arg,
                turn_id_arg,
            ])
            .to_string()
            .as_bytes(),
        );

        // 1) Inspect existing receipt first so replays/conflicts succeed even if daemon is down.
        let inspected = store.inspect(call_id, &[&fp]);
        if inspected["state"].as_str() != Some("not_found") {
            let is_err = matches!(
                inspected["state"].as_str(),
                Some("rejected" | "delivery_unknown" | "error")
            );
            return (inspected, is_err);
        }

        // 2) Connect to the daemon BEFORE claiming a brand-new call_id so daemon-stopped
        //    never poisons a call_id when zero mutating RPC was sent.
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

        // 3) Read-only pre-flight target resolution for steer/interrupt before claiming receipt.
        let resolved_steer_id = if action == "steer" {
            let tid = thread_id_arg.unwrap();
            let target = match expected_turn_id_arg {
                Some(v) => Some(v.to_owned()),
                None => client
                    .read_thread_raw(tid, true)
                    .ok()
                    .and_then(|r| find_active_turn_id(r.get("thread").unwrap_or(&r))),
            };
            if target.is_none() {
                client.close();
                return (
                    json!({
                        "state": "rejected",
                        "executed": false,
                        "thread_id": tid,
                        "error": "codex_no_active_turn_to_steer"
                    }),
                    true,
                );
            }
            target
        } else {
            None
        };

        let resolved_interrupt_id = if action == "interrupt" {
            let tid = thread_id_arg.unwrap();
            let target = match turn_id_arg {
                Some(v) => Some(v.to_owned()),
                None => client
                    .read_thread_raw(tid, true)
                    .ok()
                    .and_then(|r| find_active_turn_id(r.get("thread").unwrap_or(&r))),
            };
            if target.is_none() {
                client.close();
                return (
                    json!({
                        "state": "idle",
                        "executed": false,
                        "thread_id": tid,
                        "detail": "no active turn in progress"
                    }),
                    false,
                );
            }
            target
        } else {
            None
        };

        // 4) Atomically claim `call_id` receipt strictly BEFORE any mutating RPC!
        let receipt_path = match store.claim(call_id, &fp, &[&fp]) {
            ReceiptClaim::Acquired(p) => p,
            ReceiptClaim::Existing(v) => {
                client.close();
                let is_err = matches!(
                    v["state"].as_str(),
                    Some("rejected" | "delivery_unknown" | "error")
                );
                return (v, is_err);
            }
            ReceiptClaim::Rejected(e) => {
                client.close();
                return reject_payload(e, "could not acquire call_id receipt");
            }
        };

        let mut outcome = match action {
            "start" => {
                let thread_id = if let Some(tid) = thread_id_arg {
                    let loaded = client.loaded_threads().unwrap_or_default();
                    if !loaded.iter().any(|x| x == tid) {
                        if let Err(err) = client.call("thread/resume", json!({ "threadId": tid })) {
                            client.close();
                            // `thread/resume` failed before `turn/start` was dispatched:
                            // release the freshly claimed intent so the caller's call_id is not poisoned.
                            let _ = store.release_intent(&receipt_path);
                            let out = json!({
                                "state": "error",
                                "executed": false,
                                "error": "codex_thread_resume_failed",
                                "detail": err.to_string()
                            });
                            return (out, true);
                        }
                    }
                    tid.to_owned()
                } else {
                    // 1-shot new thread creation!
                    let cwd_str = cwd_arg
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| self.home.display().to_string());
                    let mut sp = json!({ "cwd": cwd_str });
                    if let Some(m) = model_arg {
                        sp["model"] = json!(m);
                    }
                    if let Some(mp) = model_provider_arg {
                        sp["modelProvider"] = json!(mp);
                    }
                    if let Some(ap) = approval_policy_arg {
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
                                // Leave `started` receipt unsettled because thread/start may have committed server-side.
                                let out = json!({
                                    "state": "delivery_unknown",
                                    "executed": "unknown",
                                    "error": "codex_thread_start_malformed"
                                });
                                return (out, true);
                            }
                        },
                        Err(err) => {
                            client.close();
                            if matches!(err, DaemonError::Rpc { .. }) {
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
                            // Mid-flight transport loss: leave `started` receipt unsettled!
                            return (
                                json!({
                                    "state": "delivery_unknown",
                                    "executed": "unknown",
                                    "error": "codex_daemon_transport_lost",
                                    "detail": err.to_string()
                                }),
                                true,
                            );
                        }
                    }
                };

                let mut tp = json!({
                    "threadId": thread_id,
                    "input": [{ "type": "text", "text": prompt.unwrap() }]
                });
                if let Some(c) = cwd_arg {
                    tp["cwd"] = json!(c);
                }
                if let Some(m) = model_arg {
                    tp["model"] = json!(m);
                }
                if let Some(ap) = approval_policy_arg {
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
                    Err(err) => {
                        if !matches!(err, DaemonError::Rpc { .. }) {
                            client.close();
                            return (
                                json!({
                                    "state": "delivery_unknown",
                                    "executed": "unknown",
                                    "thread_id": thread_id,
                                    "error": "codex_daemon_transport_lost",
                                    "detail": err.to_string()
                                }),
                                true,
                            );
                        }
                        json!({
                            "state": "error",
                            "executed": false,
                            "thread_id": thread_id,
                            "error": "codex_turn_start_failed",
                            "detail": err.to_string()
                        })
                    }
                }
            }
            "steer" => {
                let tid = thread_id_arg.unwrap();
                let expected_turn_id = resolved_steer_id.unwrap();
                match client.call(
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
                        wait_ms,
                    ),
                    Err(err) => {
                        if !matches!(err, DaemonError::Rpc { .. }) {
                            client.close();
                            return (
                                json!({
                                    "state": "delivery_unknown",
                                    "executed": "unknown",
                                    "thread_id": tid,
                                    "error": "codex_daemon_transport_lost",
                                    "detail": err.to_string()
                                }),
                                true,
                            );
                        }
                        json!({
                            "state": "error",
                            "executed": false,
                            "thread_id": tid,
                            "error": "codex_turn_steer_failed",
                            "detail": err.to_string()
                        })
                    }
                }
            }
            "interrupt" => {
                let tid = thread_id_arg.unwrap();
                let active_id = resolved_interrupt_id.unwrap();
                match client.call(
                    "turn/interrupt",
                    json!({ "threadId": tid, "turnId": active_id }),
                ) {
                    Ok(_) => json!({
                        "state": "interrupted",
                        "executed": true,
                        "thread_id": tid,
                        "turn_id": active_id
                    }),
                    Err(err) => {
                        if !matches!(err, DaemonError::Rpc { .. }) {
                            client.close();
                            return (
                                json!({
                                    "state": "delivery_unknown",
                                    "executed": "unknown",
                                    "thread_id": tid,
                                    "error": "codex_daemon_transport_lost",
                                    "detail": err.to_string()
                                }),
                                true,
                            );
                        }
                        json!({
                            "state": "error",
                            "executed": false,
                            "thread_id": tid,
                            "error": "codex_turn_interrupt_failed",
                            "detail": err.to_string()
                        })
                    }
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

    fn handle_catalog(&self, args: &Value) -> (Value, bool) {
        let Some(obj) = args.as_object() else {
            return reject_payload(
                "codex_catalog_arguments_invalid",
                "arguments must be an empty JSON object",
            );
        };
        if !obj.is_empty() {
            return reject_payload(
                "codex_catalog_arguments_invalid",
                "codex_catalog takes no arguments",
            );
        }
        (self.registry.catalog_json(), false)
    }

    fn handle_call_extension(&self, args: &Value) -> (Value, bool) {
        let Some(obj) = args.as_object() else {
            return reject_payload(
                "codex_call_arguments_invalid",
                "arguments must be a JSON object",
            );
        };
        if let Err(e) = check_allowed_keys(obj, &["action", "profile", "arguments"]) {
            return reject_payload("codex_call_arguments_invalid", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return reject_payload("codex_call_action_required", "action is required");
        };
        if self.registry.get(action).is_none() {
            return (
                self.registry
                    .validation_error(action, "unknown_extension_action"),
                true,
            );
        }
        let profile = match optional_nonempty_str(obj, "profile") {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };
        let empty = Value::Object(Map::new());
        let ext_args = obj.get("arguments").unwrap_or(&empty);
        let Some(ext_obj) = ext_args.as_object() else {
            return (
                self.registry
                    .validation_error(action, "arguments must be a JSON object"),
                true,
            );
        };

        let (_, codex_home) = match resolve_codex_home(&self.home, profile) {
            Ok(v) => v,
            Err(e) => return reject_payload("codex_profile_invalid", e),
        };

        // Strictly validate keys, types, enums, and bounds BEFORE opening daemon RPC!
        let (rpc_method, rpc_params) = match action {
            "thread_fork" => {
                if let Err(e) =
                    check_allowed_keys(ext_obj, &["thread_id", "last_turn_id", "cwd", "model"])
                {
                    return (self.registry.validation_error(action, &e), true);
                }
                let tid = match optional_nonempty_str(ext_obj, "thread_id") {
                    Ok(Some(v)) => v,
                    _ => {
                        return (
                            self.registry
                                .validation_error(action, "missing required string 'thread_id'"),
                            true,
                        );
                    }
                };
                let mut p = json!({ "threadId": tid });
                match optional_nonempty_str(ext_obj, "last_turn_id") {
                    Ok(Some(v)) => p["lastTurnId"] = json!(v),
                    Ok(None) => {}
                    Err(e) => return (self.registry.validation_error(action, &e), true),
                }
                match optional_nonempty_str(ext_obj, "cwd") {
                    Ok(Some(v)) => {
                        if let Err(e) = validate_cwd_dir(v) {
                            return (self.registry.validation_error(action, &e), true);
                        }
                        p["cwd"] = json!(v);
                    }
                    Ok(None) => {}
                    Err(e) => return (self.registry.validation_error(action, &e), true),
                }
                match optional_nonempty_str(ext_obj, "model") {
                    Ok(Some(v)) => p["model"] = json!(v),
                    Ok(None) => {}
                    Err(e) => return (self.registry.validation_error(action, &e), true),
                }
                ("thread/fork", p)
            }
            "thread_archive" | "thread_unarchive" | "thread_compact" | "goal_get"
            | "goal_clear" => {
                if let Err(e) = check_allowed_keys(ext_obj, &["thread_id"]) {
                    return (self.registry.validation_error(action, &e), true);
                }
                let tid = match optional_nonempty_str(ext_obj, "thread_id") {
                    Ok(Some(v)) => v,
                    _ => {
                        return (
                            self.registry
                                .validation_error(action, "missing required string 'thread_id'"),
                            true,
                        );
                    }
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
                if let Err(e) = check_allowed_keys(ext_obj, &["thread_id", "name"]) {
                    return (self.registry.validation_error(action, &e), true);
                }
                let tid = match optional_nonempty_str(ext_obj, "thread_id") {
                    Ok(Some(v)) => v,
                    _ => {
                        return (
                            self.registry.validation_error(
                                action,
                                "requires non-empty 'thread_id' and 'name'",
                            ),
                            true,
                        );
                    }
                };
                let name = match optional_nonempty_str(ext_obj, "name") {
                    Ok(Some(v)) if v.len() <= 200 => v,
                    _ => {
                        return (
                            self.registry
                                .validation_error(action, "requires non-empty 'name' <= 200 bytes"),
                            true,
                        );
                    }
                };
                ("thread/name/set", json!({ "threadId": tid, "name": name }))
            }
            "review_start" => {
                if let Err(e) = check_allowed_keys(ext_obj, &["thread_id", "target", "delivery"]) {
                    return (self.registry.validation_error(action, &e), true);
                }
                let tid = match optional_nonempty_str(ext_obj, "thread_id") {
                    Ok(Some(v)) => v,
                    _ => {
                        return (
                            self.registry.validation_error(
                                action,
                                "requires 'thread_id' and object 'target'",
                            ),
                            true,
                        );
                    }
                };
                let Some(target) = ext_obj.get("target") else {
                    return (
                        self.registry
                            .validation_error(action, "requires 'thread_id' and object 'target'"),
                        true,
                    );
                };
                if let Err(e) = validate_review_target(target) {
                    return (self.registry.validation_error(action, &e), true);
                }
                let mut p = json!({ "threadId": tid, "target": target });
                match optional_nonempty_str(ext_obj, "delivery") {
                    Ok(Some("inline")) => p["delivery"] = json!("inline"),
                    Ok(None) => {}
                    Ok(Some(other)) => {
                        return (
                            self.registry.validation_error(
                                action,
                                &format!("unsupported delivery '{other}'; expected 'inline'"),
                            ),
                            true,
                        );
                    }
                    Err(e) => return (self.registry.validation_error(action, &e), true),
                }
                ("review/start", p)
            }
            "account_usage" => {
                if let Err(e) = check_allowed_keys(ext_obj, &[]) {
                    return (self.registry.validation_error(action, &e), true);
                }
                ("account/rateLimits/read", json!({}))
            }
            "models_list" => {
                if let Err(e) = check_allowed_keys(ext_obj, &["model_provider", "limit", "cursor"])
                {
                    return (self.registry.validation_error(action, &e), true);
                }
                let mut p = json!({});
                match optional_nonempty_str(ext_obj, "model_provider") {
                    Ok(Some(mp)) => p["modelProvider"] = json!(mp),
                    Ok(None) => {}
                    Err(e) => return (self.registry.validation_error(action, &e), true),
                }
                match optional_bounded_u64(ext_obj, "limit", 1, 100) {
                    Ok(Some(l)) => p["limit"] = json!(l),
                    Ok(None) => {}
                    Err(e) => return (self.registry.validation_error(action, &e), true),
                }
                match optional_nonempty_str(ext_obj, "cursor") {
                    Ok(Some(c)) => p["cursor"] = json!(c),
                    Ok(None) => {}
                    Err(e) => return (self.registry.validation_error(action, &e), true),
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
            "codex_catalog" => self.handle_catalog(arguments),
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
