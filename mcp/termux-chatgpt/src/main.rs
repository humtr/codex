//! `@TermuxChatGPT` — stateless thin MCP bridge over `chatgpt-provider`'s local `/control/*` plane.
//!
//! Built on `crates/mcp-kit` (uniform immutable annotations, bounded `call_id` transport
//! deduplication receipts, and strictly disjoint `chatgpt_catalog` / `chatgpt_call` extension
//! slots). Owns zero local code execution privilege, zero shadow task DB, zero CDP connection,
//! and zero `crates/wrapper` coupling.

mod http_client;

use http_client::{
    check_healthz, parse_loopback_base_url, read_control_token, request_json, url_encode,
    HttpClientError, LoopbackEndpoint,
};
use mcp_kit::{
    serve_stdio, sha256_hex, validate_call_id, ExtensionRegistry, ExtensionSpec, McpHandler,
    ReceiptClaim, ReceiptStore, ToolDef,
};
use serde_json::{json, Map, Value};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

const RECEIPT_SCHEMA: &str = "termux-chatgpt-mcp-receipt";
const DEFAULT_PROVIDER_URL: &str = "http://127.0.0.1:4319";
const STATIC_TOOL_NAMES: &[&str] = &[
    "chatgpt_status",
    "chatgpt_chats",
    "chatgpt_projects",
    "chatgpt_send",
    "chatgpt_catalog",
    "chatgpt_call",
];
const MAX_PROMPT_BYTES: usize = 64 * 1024;
const DEFAULT_WAIT_MS: u64 = 30_000;
const MAX_WAIT_MS: u64 = 300_000;
/// Strictly exceeds CdpDriver's worst-case cold-boot (45s) + read RPC (20..30s) budget.
const CONTROL_READ_TIMEOUT: Duration = Duration::from_secs(75);
const CONTROL_RESTART_TIMEOUT: Duration = Duration::from_secs(90);
const DEFAULT_TERMUX_PREFIX: &str = "/data/data/com.termux/files/usr";

fn static_tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "chatgpt_status",
            "Read-only status of chatgpt-provider and the owned Linux ChatGPT Desktop AppHost: host_running, auth_state, ui_state, completion_ready, account summary, active_chat_id, generating flag, and available models. Never exposes tokens or cookies.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "chatgpt_chats",
            "Pure read-only ChatGPT conversation inspection: list conversations (paginated, optional archived filter) or read a conversation's linear message branch (private thoughts excluded).",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "read"] },
                    "chat_id": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Required for action=read" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200, "description": "Page size (1..=100 for list, 1..=200 for read)" },
                    "cursor": { "type": "string", "minLength": 1, "maxLength": 256, "description": "Pagination cursor for action=list" },
                    "archived": { "type": "boolean", "description": "List archived chats when true (action=list only)" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "chatgpt_projects",
            "Pure read-only ChatGPT Project inspection: list projects or read conversations inside a specific project.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "read"] },
                    "project_id": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Required for action=read" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "cursor": { "type": "string", "minLength": 1, "maxLength": 256 }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "chatgpt_send",
            "Drive a ChatGPT Desktop turn via chatgpt-provider: send a prompt (new chat, existing chat_id, or project_id with call_id deduplication and bounded wait_ms), observe an active/settled chat, or stop an in-flight generation. Supply a stable call_id on action=send to deduplicate retries across mobile transport disconnects.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["send", "observe", "stop"] },
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160, "description": "Required for action=send; resend identical args after uncertain delivery to retrieve original receipt" },
                    "chat_id": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Target conversation ID (optional for send/observe/stop)" },
                    "project_id": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Target project ID when creating a new chat inside a project (action=send)" },
                    "prompt": { "type": "string", "minLength": 1, "maxLength": 65536, "description": "Required for action=send" },
                    "model": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Optional model slug (e.g. gpt-6-sol-wm, gpt-5-6-thinking, auto)" },
                    "reasoning_effort": { "type": "string", "enum": ["none", "zero", "minimal", "min", "low", "medium", "standard", "high", "extended", "xhigh", "max", "ultra"] },
                    "wait_ms": { "type": "integer", "minimum": 0, "maximum": 300000, "description": "Bounded wait for response settlement on action=send (default 30000ms, max 300000ms)" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "chatgpt_catalog",
            "List all allowlisted extension actions callable via chatgpt_call with their exact JSON input schemas (strictly disjoint from static tools).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "chatgpt_call",
            "Invoke an allowlisted extension action discovered via chatgpt_catalog (zero local code execution privilege). Validation failures return the exact expected JSON schema in the error body.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "minLength": 1, "maxLength": 64 },
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
            "project_create",
            "Create a new ChatGPT Project with a name and optional custom instructions.",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "minLength": 1, "maxLength": 120 },
                    "instructions": { "type": "string", "maxLength": 16384 }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "chat_rename",
            "Rename an existing ChatGPT conversation.",
            json!({
                "type": "object",
                "properties": {
                    "chat_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 200 }
                },
                "required": ["chat_id", "title"],
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "viewer_url",
            "Inspect whether the capability-scoped localhost noVNC viewer is active and return its URL if available.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "viewer_open",
            "Request opening the capability-scoped noVNC viewer in the local Android browser (for interactive login or visual inspection).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "host_restart",
            "Restart the provider-owned Linux ChatGPT Desktop AppHost (Xvnc + PRoot + Electron) while preserving the separated persistent login profile.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            false,
        ),
    ];
    for spec in specs {
        reg.register(spec)
            .expect("static and extension names must be disjoint");
    }
    reg
}

fn fail(code: &str, detail: impl Into<String>) -> Value {
    json!({
        "state": "rejected",
        "error": code,
        "detail": detail.into()
    })
}

fn check_allowed_keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported argument key '{key}'"));
        }
    }
    Ok(())
}

fn validate_safe_id<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
    required: bool,
) -> Result<Option<&'a str>, String> {
    match obj.get(key) {
        None => {
            if required {
                Err(format!("missing required string '{key}'"))
            } else {
                Ok(None)
            }
        }
        Some(v) => match v.as_str() {
            Some(s)
                if !s.is_empty()
                    && s.len() <= 128
                    && s != "."
                    && s != ".."
                    && !s.chars().any(|c| {
                        c.is_control()
                            || c.is_whitespace()
                            || matches!(c, '/' | '\\' | '?' | '#')
                    }) =>
            {
                Ok(Some(s))
            }
            _ => Err(format!(
                "'{key}' must be a 1..=128 byte identifier (not '.'/'..') without '/', '\\', '?', '#', whitespace, or control chars"
            )),
        },
    }
}

fn optional_nonempty_str<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
    max_len: usize,
) -> Result<Option<&'a str>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(v) => match v.as_str() {
            Some(s) if !s.trim().is_empty() && s.len() <= max_len && !s.contains('\0') => {
                Ok(Some(s))
            }
            _ => Err(format!(
                "'{key}' must be a non-empty string <= {max_len} bytes without NUL"
            )),
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

fn default_provider_url() -> String {
    env::var("CHATGPT_PROVIDER_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROVIDER_URL.to_owned())
}

fn default_token_path() -> Result<PathBuf, &'static str> {
    if let Some(custom) = env::var_os("CHATGPT_PROVIDER_TOKEN_FILE").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(custom));
    }
    let home = env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .ok_or("home_unavailable")?;
    Ok(PathBuf::from(home).join(".local/state/codex/providers/chatgpt/control.token"))
}

fn default_receipt_root() -> Result<PathBuf, &'static str> {
    if let Some(custom) = env::var_os("TERMUX_CHATGPT_RECEIPT_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(custom));
    }
    let prefix = env::var_os("PREFIX")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TERMUX_PREFIX));
    Ok(prefix.join("var/tmp/termux-chatgpt-mcp-receipts"))
}

struct TermuxChatGptServer {
    provider_url: Option<String>,
    token_path: Option<PathBuf>,
    receipt_root: Option<PathBuf>,
    registry: ExtensionRegistry,
}

impl TermuxChatGptServer {
    fn new() -> Self {
        Self {
            provider_url: None,
            token_path: None,
            receipt_root: None,
            registry: build_extension_registry(),
        }
    }

    fn resolve_endpoint(&self) -> Result<LoopbackEndpoint, HttpClientError> {
        let raw = self
            .provider_url
            .clone()
            .unwrap_or_else(default_provider_url);
        parse_loopback_base_url(&raw)
    }

    fn resolve_token(&self) -> Result<String, HttpClientError> {
        let path = match &self.token_path {
            Some(p) => p.clone(),
            None => default_token_path().map_err(|code| {
                HttpClientError::Config(code, "cannot determine control.token path".to_owned())
            })?,
        };
        read_control_token(&path)
    }

    fn open_receipt_store(&self) -> Result<ReceiptStore, &'static str> {
        let root = match &self.receipt_root {
            Some(p) => p.clone(),
            None => default_receipt_root()?,
        };
        ReceiptStore::open(root, RECEIPT_SCHEMA, "chatgpt_call_id_payload_conflict")
    }

    fn control_call(
        &self,
        method: &str,
        path_and_query: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value, HttpClientError> {
        let ep = self.resolve_endpoint()?;
        let token = self.resolve_token()?;
        let resp = request_json(&ep, method, path_and_query, Some(&token), body, timeout)?;
        if resp.status >= 200 && resp.status < 300 {
            if let Some(val) = resp.json {
                return Ok(val);
            }
            return Err(HttpClientError::TransportLost(
                "provider returned non-JSON 2xx response".to_owned(),
            ));
        }
        if let Some(mut err_json) = resp.json {
            if let Some(obj) = err_json.as_object_mut() {
                obj.entry("state").or_insert_with(|| json!("error"));
                obj.entry("http_status")
                    .or_insert_with(|| json!(resp.status));
                return Ok(err_json);
            }
        }
        let code = match resp.status {
            401 | 403 => "chatgpt_control_unauthorized",
            400 => "chatgpt_control_bad_request",
            404 => "chatgpt_control_not_found",
            503 => "chatgpt_provider_unavailable",
            _ => "chatgpt_provider_http_error",
        };
        Ok(json!({
            "state": "error",
            "error": code,
            "http_status": resp.status,
            "detail": resp.raw_text.chars().take(512).collect::<String>()
        }))
    }

    fn handle_status(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(msg) = check_allowed_keys(obj, &[]) {
            return fail("invalid_arguments", msg);
        }
        let ep = match self.resolve_endpoint() {
            Ok(ep) => ep,
            Err(err) => return fail(err.code(), err.detail()),
        };
        let token = match self.resolve_token() {
            Ok(t) => t,
            Err(HttpClientError::Unavailable(code, detail)) => {
                // Distinguish a stopped provider from a running provider with a missing/unreadable token.
                if check_healthz(&ep).is_ok() {
                    return json!({
                        "state": "error",
                        "provider_reachable": true,
                        "error": code,
                        "detail": detail
                    });
                }
                return json!({
                    "state": "ok",
                    "provider_reachable": false,
                    "host_running": false,
                    "auth_state": "provider_stopped",
                    "error": code,
                    "detail": detail
                });
            }
            Err(err) => return fail(err.code(), err.detail()),
        };
        match request_json(
            &ep,
            "GET",
            "/control/status",
            Some(&token),
            None,
            CONTROL_READ_TIMEOUT,
        ) {
            Ok(resp) => {
                if resp.status >= 200 && resp.status < 300 {
                    if let Some(mut v) = resp.json {
                        if let Some(o) = v.as_object_mut() {
                            o.entry("provider_reachable").or_insert_with(|| json!(true));
                            return v;
                        }
                    }
                    return fail(
                        "chatgpt_provider_transport_lost",
                        "provider returned non-object 2xx status response",
                    );
                }
                if let Some(mut err_json) = resp.json {
                    if let Some(obj) = err_json.as_object_mut() {
                        obj.entry("state").or_insert_with(|| json!("error"));
                        obj.entry("provider_reachable")
                            .or_insert_with(|| json!(true));
                        obj.entry("http_status")
                            .or_insert_with(|| json!(resp.status));
                        return err_json;
                    }
                }
                json!({
                    "state": "error",
                    "provider_reachable": true,
                    "error": match resp.status {
                        401 | 403 => "chatgpt_control_unauthorized",
                        400 => "chatgpt_control_bad_request",
                        404 => "chatgpt_control_not_found",
                        503 => "chatgpt_provider_unavailable",
                        _ => "chatgpt_provider_http_error",
                    },
                    "http_status": resp.status,
                    "detail": resp.raw_text.chars().take(512).collect::<String>()
                })
            }
            Err(HttpClientError::Unavailable(code, detail)) => json!({
                "state": "ok",
                "provider_reachable": false,
                "host_running": false,
                "auth_state": "provider_stopped",
                "error": code,
                "detail": detail
            }),
            Err(err) => fail(err.code(), err.detail()),
        }
    }

    fn handle_chats(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(msg) =
            check_allowed_keys(obj, &["action", "chat_id", "limit", "cursor", "archived"])
        {
            return fail("invalid_arguments", msg);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "'action' must be 'list' or 'read'");
        };
        match action {
            "list" => {
                if obj.contains_key("chat_id") {
                    return fail(
                        "invalid_arguments",
                        "'chat_id' is only valid for action=read",
                    );
                }
                let limit = match optional_bounded_u64(obj, "limit", 1, 100) {
                    Ok(v) => v.unwrap_or(20),
                    Err(e) => return fail("invalid_arguments", e),
                };
                let cursor = match optional_nonempty_str(obj, "cursor", 256) {
                    Ok(v) => v,
                    Err(e) => return fail("invalid_arguments", e),
                };
                let archived = match optional_bool(obj, "archived") {
                    Ok(v) => v.unwrap_or(false),
                    Err(e) => return fail("invalid_arguments", e),
                };
                let mut qs = format!("/control/chats?limit={limit}&archived={archived}");
                if let Some(c) = cursor {
                    qs.push_str("&cursor=");
                    qs.push_str(&url_encode(c));
                }
                self.finish_control(self.control_call("GET", &qs, None, CONTROL_READ_TIMEOUT))
            }
            "read" => {
                if obj.contains_key("cursor") || obj.contains_key("archived") {
                    return fail(
                        "invalid_arguments",
                        "'cursor' and 'archived' are only valid for action=list",
                    );
                }
                let chat_id = match validate_safe_id(obj, "chat_id", true) {
                    Ok(Some(id)) => id,
                    Ok(None) => return fail("invalid_arguments", "'chat_id' is required"),
                    Err(e) => return fail("invalid_arguments", e),
                };
                let limit = match optional_bounded_u64(obj, "limit", 1, 200) {
                    Ok(v) => v.unwrap_or(50),
                    Err(e) => return fail("invalid_arguments", e),
                };
                let path = format!("/control/chats/{}?limit={limit}", url_encode(chat_id));
                self.finish_control(self.control_call("GET", &path, None, CONTROL_READ_TIMEOUT))
            }
            other => fail(
                "invalid_arguments",
                format!("unsupported chatgpt_chats action '{other}' (expected list|read)"),
            ),
        }
    }

    fn handle_projects(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(msg) = check_allowed_keys(obj, &["action", "project_id", "limit", "cursor"]) {
            return fail("invalid_arguments", msg);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "'action' must be 'list' or 'read'");
        };
        let limit = match optional_bounded_u64(obj, "limit", 1, 100) {
            Ok(v) => v.unwrap_or(20),
            Err(e) => return fail("invalid_arguments", e),
        };
        let cursor = match optional_nonempty_str(obj, "cursor", 256) {
            Ok(v) => v,
            Err(e) => return fail("invalid_arguments", e),
        };
        match action {
            "list" => {
                if obj.contains_key("project_id") {
                    return fail(
                        "invalid_arguments",
                        "'project_id' is only valid for action=read",
                    );
                }
                let mut qs = format!("/control/projects?limit={limit}");
                if let Some(c) = cursor {
                    qs.push_str("&cursor=");
                    qs.push_str(&url_encode(c));
                }
                self.finish_control(self.control_call("GET", &qs, None, CONTROL_READ_TIMEOUT))
            }
            "read" => {
                let project_id = match validate_safe_id(obj, "project_id", true) {
                    Ok(Some(id)) => id,
                    Ok(None) => return fail("invalid_arguments", "'project_id' is required"),
                    Err(e) => return fail("invalid_arguments", e),
                };
                let mut qs = format!("/control/projects/{}?limit={limit}", url_encode(project_id));
                if let Some(c) = cursor {
                    qs.push_str("&cursor=");
                    qs.push_str(&url_encode(c));
                }
                self.finish_control(self.control_call("GET", &qs, None, CONTROL_READ_TIMEOUT))
            }
            other => fail(
                "invalid_arguments",
                format!("unsupported chatgpt_projects action '{other}' (expected list|read)"),
            ),
        }
    }

    fn handle_send(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(msg) = check_allowed_keys(
            obj,
            &[
                "action",
                "call_id",
                "chat_id",
                "project_id",
                "prompt",
                "model",
                "reasoning_effort",
                "wait_ms",
            ],
        ) {
            return fail("invalid_arguments", msg);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail(
                "invalid_arguments",
                "'action' must be 'send', 'observe', or 'stop'",
            );
        };

        match action {
            "observe" => {
                if obj.contains_key("call_id") && validate_call_id(args).is_err() {
                    return fail(
                        "chatgpt_call_id_invalid",
                        "optional call_id must be a 1..=160 byte string without NUL",
                    );
                }
                for forbidden in [
                    "prompt",
                    "project_id",
                    "model",
                    "reasoning_effort",
                    "wait_ms",
                ] {
                    if obj.contains_key(forbidden) {
                        return fail(
                            "invalid_arguments",
                            format!("'{forbidden}' is not accepted for action=observe"),
                        );
                    }
                }
                let chat_id = match validate_safe_id(obj, "chat_id", false) {
                    Ok(v) => v,
                    Err(e) => return fail("invalid_arguments", e),
                };
                let body = json!({ "chat_id": chat_id });
                self.finish_control(self.control_call(
                    "POST",
                    "/control/observe",
                    Some(&body),
                    CONTROL_READ_TIMEOUT,
                ))
            }
            "stop" => {
                if obj.contains_key("call_id") && validate_call_id(args).is_err() {
                    return fail(
                        "chatgpt_call_id_invalid",
                        "optional call_id must be a 1..=160 byte string without NUL",
                    );
                }
                for forbidden in [
                    "project_id",
                    "prompt",
                    "model",
                    "reasoning_effort",
                    "wait_ms",
                ] {
                    if obj.contains_key(forbidden) {
                        return fail(
                            "invalid_arguments",
                            format!("'{forbidden}' is not accepted for action=stop"),
                        );
                    }
                }
                let chat_id = match validate_safe_id(obj, "chat_id", false) {
                    Ok(v) => v,
                    Err(e) => return fail("invalid_arguments", e),
                };
                let body = json!({ "chat_id": chat_id });
                self.finish_control(self.control_call(
                    "POST",
                    "/control/stop",
                    Some(&body),
                    CONTROL_READ_TIMEOUT,
                ))
            }
            "send" => self.handle_send_dispatch(args, obj),
            other => fail(
                "invalid_arguments",
                format!("unsupported chatgpt_send action '{other}' (expected send|observe|stop)"),
            ),
        }
    }

    fn handle_send_dispatch(&self, args: &Value, obj: &Map<String, Value>) -> Value {
        let call_id = match validate_call_id(args) {
            Ok(id) => id,
            Err(_) => {
                return fail(
                    "chatgpt_call_id_invalid",
                    "action=send requires non-empty call_id <= 160 bytes without NUL",
                )
            }
        };
        let prompt = match optional_nonempty_str(obj, "prompt", MAX_PROMPT_BYTES) {
            Ok(Some(p)) => p,
            Ok(None) => return fail("chatgpt_prompt_required", "action=send requires prompt"),
            Err(e) => return fail("invalid_arguments", e),
        };
        let chat_id = match validate_safe_id(obj, "chat_id", false) {
            Ok(v) => v,
            Err(e) => return fail("invalid_arguments", e),
        };
        let project_id = match validate_safe_id(obj, "project_id", false) {
            Ok(v) => v,
            Err(e) => return fail("invalid_arguments", e),
        };
        if chat_id.is_some() && project_id.is_some() {
            return fail(
                "invalid_arguments",
                "specify either 'chat_id' (continue existing chat) or 'project_id' (new chat in project), not both",
            );
        }
        let model = match optional_nonempty_str(obj, "model", 128) {
            Ok(v) => v,
            Err(e) => return fail("invalid_arguments", e),
        };
        let reasoning_effort = match obj.get("reasoning_effort") {
            None => None,
            Some(v) => match v.as_str() {
                Some(
                    eff @ ("none" | "zero" | "minimal" | "min" | "low" | "medium" | "standard"
                    | "high" | "extended" | "xhigh" | "max" | "ultra"),
                ) => Some(eff),
                _ => {
                    return fail(
                        "invalid_arguments",
                        "reasoning_effort must be one of none|zero|minimal|min|low|medium|standard|high|extended|xhigh|max|ultra",
                    )
                }
            },
        };
        let wait_ms = match optional_bounded_u64(obj, "wait_ms", 0, MAX_WAIT_MS) {
            Ok(v) => v.unwrap_or(DEFAULT_WAIT_MS),
            Err(e) => return fail("invalid_arguments", e),
        };

        let endpoint = match self.resolve_endpoint() {
            Ok(ep) => ep,
            Err(err) => return fail(err.code(), err.detail()),
        };
        let store = match self.open_receipt_store() {
            Ok(s) => s,
            Err(code) => return fail(code, "cannot open chatgpt-send receipt directory"),
        };

        // Exclude `wait_ms` from the payload fingerprint (matching `@TermuxCodex`) so retrying
        // the identical turn with a different wait budget replays the receipt cleanly.
        let fp = sha256_hex(
            json!([
                "chatgpt-send",
                endpoint.canonical_url,
                chat_id,
                project_id,
                prompt,
                model,
                reasoning_effort
            ])
            .to_string()
            .as_bytes(),
        );

        // 1. Inspect existing receipt FIRST so replays and payload conflicts work even while provider is down.
        let existing = store.inspect(call_id, &[&fp]);
        if existing["state"] != "not_found" {
            return existing;
        }

        // 2. Verify provider reachability & token BEFORE claiming a brand-new call_id so a stopped
        //    provider never poisons the caller's call_id.
        if let Err(err) = check_healthz(&endpoint) {
            return json!({
                "state": "rejected",
                "executed": false,
                "error": err.code(),
                "detail": err.detail()
            });
        }
        let token = match self.resolve_token() {
            Ok(t) => t,
            Err(err) => {
                return json!({
                    "state": "rejected",
                    "executed": false,
                    "error": err.code(),
                    "detail": err.detail()
                })
            }
        };

        // 3. Atomically claim durable `started` receipt strictly before dispatching POST /control/send.
        let receipt_path = match store.claim(call_id, &fp, &[&fp]) {
            ReceiptClaim::Acquired(p) => p,
            ReceiptClaim::Existing(v) => return v,
            ReceiptClaim::Rejected(code) => {
                return json!({
                    "state": "rejected",
                    "executed": false,
                    "error": code
                })
            }
        };

        let payload = json!({
            "chat_id": chat_id,
            "project_id": project_id,
            "prompt": prompt,
            "model": model,
            "reasoning_effort": reasoning_effort,
            "wait_ms": wait_ms
        });
        // Exceed CdpDriver worst-case cold boot (45s) + sendMessage (wait_ms + 45s) with headroom.
        let timeout = Duration::from_millis(wait_ms.saturating_add(105_000));
        match request_json(
            &endpoint,
            "POST",
            "/control/send",
            Some(&token),
            Some(&payload),
            timeout,
        ) {
            Ok(resp) => {
                let mut outcome = if resp.status >= 200 && resp.status < 300 {
                    match resp.json {
                        Some(v) if v.is_object() => v,
                        _ => {
                            return json!({
                                "state": "delivery_unknown",
                                "executed": "unknown",
                                "error": "chatgpt_provider_non_object_reply",
                                "detail": "provider returned non-object JSON body; call chatgpt_send(action='observe') before retrying"
                            });
                        }
                    }
                } else if let Some(mut err_v) = resp.json.filter(Value::is_object) {
                    if let Some(o) = err_v.as_object_mut() {
                        o.entry("state").or_insert_with(|| json!("error"));
                        o.entry("http_status").or_insert_with(|| json!(resp.status));
                    }
                    err_v
                } else {
                    json!({
                        "state": "error",
                        "error": match resp.status {
                            401 | 403 => "chatgpt_control_unauthorized",
                            400 => "chatgpt_control_bad_request",
                            503 => "chatgpt_provider_unavailable",
                            _ => "chatgpt_provider_http_error",
                        },
                        "http_status": resp.status,
                        "detail": resp.raw_text.chars().take(512).collect::<String>()
                    })
                };

                // Guaranteed pre-dispatch rejections (nothing was sent to ChatGPT backend):
                // release the freshly claimed intent so the caller can safely retry with the
                // same call_id after resolving auth/token/host/busy state.
                let err_code = outcome.get("error").and_then(Value::as_str).unwrap_or("");
                let is_pre_dispatch_rejection = matches!(resp.status, 400 | 401 | 403)
                    || matches!(
                        err_code,
                        "login_required"
                            | "generation_already_active"
                            | "completion_client_unavailable"
                            | "host_rootfs_not_found"
                            | "official_app_rootfs_required"
                            | "host_profile_busy"
                            | "prompt_required"
                            | "prompt_invalid"
                            | "invalid_chat_id"
                            | "invalid_project_id"
                            | "control_auth_required"
                            | "control_auth_invalid"
                            | "control_auth_rejected"
                            | "chatgpt_control_unauthorized"
                            | "chatgpt_control_bad_request"
                    );
                if is_pre_dispatch_rejection {
                    let _ = store.release_intent(&receipt_path);
                    if let Some(o) = outcome.as_object_mut() {
                        o.insert("executed".to_owned(), json!(false));
                    }
                    return outcome;
                }

                if let Some(o) = outcome.as_object_mut() {
                    o.insert("replay_safe".to_owned(), json!(true));
                }
                if let Err(err) = store.commit(&receipt_path, &fp, &outcome) {
                    if let Some(o) = outcome.as_object_mut() {
                        o.insert("receipt_error".to_owned(), json!(err));
                        o.insert("replay_safe".to_owned(), json!(false));
                    }
                } else {
                    store.prune_settled(512);
                }
                outcome
            }
            Err(HttpClientError::TransportLost(detail)) => {
                // Leave the `started` receipt unsettled so exact retry returns delivery_unknown
                // instead of re-sending a potentially accepted ChatGPT turn.
                json!({
                    "state": "delivery_unknown",
                    "executed": "unknown",
                    "error": "chatgpt_provider_transport_lost",
                    "detail": detail
                })
            }
            Err(err @ (HttpClientError::Unavailable(_, _) | HttpClientError::Config(_, _))) => {
                // Guaranteed pre-dispatch failure (zero bytes written to provider): release the
                // freshly claimed intent (with parent fsync) so the caller's call_id is never poisoned.
                let _ = store.release_intent(&receipt_path);
                json!({
                    "state": "rejected",
                    "executed": false,
                    "error": err.code(),
                    "detail": err.detail()
                })
            }
        }
    }

    fn handle_catalog(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(msg) = check_allowed_keys(obj, &[]) {
            return fail("invalid_arguments", msg);
        }
        let mut catalog = self.registry.catalog_json();
        catalog["state"] = json!("ok");
        catalog
    }

    fn handle_call(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(msg) = check_allowed_keys(obj, &["action", "arguments"]) {
            return fail("invalid_arguments", msg);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return self
                .registry
                .validation_error("", "missing_required_action");
        };
        if self.registry.get(action).is_none() {
            return self
                .registry
                .validation_error(action, "unknown_extension_action");
        }
        let empty_map = Map::new();
        let act_args = match obj.get("arguments") {
            None | Some(Value::Null) => &empty_map,
            Some(Value::Object(m)) => m,
            Some(_) => {
                return self
                    .registry
                    .validation_error(action, "arguments_must_be_object")
            }
        };

        match action {
            "project_create" => {
                if check_allowed_keys(act_args, &["name", "instructions"]).is_err() {
                    return self
                        .registry
                        .validation_error(action, "unsupported_argument_key");
                }
                let Ok(Some(name)) = optional_nonempty_str(act_args, "name", 120) else {
                    return self.registry.validation_error(action, "name_invalid");
                };
                let instructions = match act_args.get("instructions") {
                    None => None,
                    Some(v) => match v.as_str() {
                        Some(s) if s.len() <= 16_384 && !s.contains('\0') => Some(s),
                        _ => {
                            return self
                                .registry
                                .validation_error(action, "instructions_invalid")
                        }
                    },
                };
                let body = json!({ "name": name.trim(), "instructions": instructions });
                self.finish_control(self.control_call(
                    "POST",
                    "/control/projects",
                    Some(&body),
                    CONTROL_READ_TIMEOUT,
                ))
            }
            "chat_rename" => {
                if check_allowed_keys(act_args, &["chat_id", "title"]).is_err() {
                    return self
                        .registry
                        .validation_error(action, "unsupported_argument_key");
                }
                let Ok(Some(chat_id)) = validate_safe_id(act_args, "chat_id", true) else {
                    return self.registry.validation_error(action, "chat_id_invalid");
                };
                let Ok(Some(title)) = optional_nonempty_str(act_args, "title", 200) else {
                    return self.registry.validation_error(action, "title_invalid");
                };
                let path = format!("/control/chats/{}/rename", url_encode(chat_id));
                let body = json!({ "title": title.trim() });
                self.finish_control(self.control_call(
                    "POST",
                    &path,
                    Some(&body),
                    CONTROL_READ_TIMEOUT,
                ))
            }
            "viewer_url" => {
                if check_allowed_keys(act_args, &[]).is_err() {
                    return self
                        .registry
                        .validation_error(action, "unsupported_argument_key");
                }
                self.finish_control(self.control_call(
                    "GET",
                    "/control/viewer",
                    None,
                    CONTROL_READ_TIMEOUT,
                ))
            }
            "viewer_open" => {
                if check_allowed_keys(act_args, &[]).is_err() {
                    return self
                        .registry
                        .validation_error(action, "unsupported_argument_key");
                }
                self.finish_control(self.control_call(
                    "POST",
                    "/control/viewer/open",
                    Some(&json!({})),
                    CONTROL_READ_TIMEOUT,
                ))
            }
            "host_restart" => {
                if check_allowed_keys(act_args, &[]).is_err() {
                    return self
                        .registry
                        .validation_error(action, "unsupported_argument_key");
                }
                self.finish_control(self.control_call(
                    "POST",
                    "/control/host/restart",
                    Some(&json!({})),
                    CONTROL_RESTART_TIMEOUT,
                ))
            }
            _ => self
                .registry
                .validation_error(action, "unknown_extension_action"),
        }
    }

    fn finish_control(&self, res: Result<Value, HttpClientError>) -> Value {
        match res {
            Ok(v) => v,
            Err(err) => json!({
                "state": "error",
                "error": err.code(),
                "detail": err.detail()
            }),
        }
    }
}

impl McpHandler for TermuxChatGptServer {
    fn server_name(&self) -> &str {
        "TermuxChatGPT"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<ToolDef> {
        static_tools()
    }

    fn call_tool(&mut self, name: &str, arguments: &Value) -> (Value, bool) {
        let result = match name {
            "chatgpt_status" => self.handle_status(arguments),
            "chatgpt_chats" => self.handle_chats(arguments),
            "chatgpt_projects" => self.handle_projects(arguments),
            "chatgpt_send" => self.handle_send(arguments),
            "chatgpt_catalog" => self.handle_catalog(arguments),
            "chatgpt_call" => self.handle_call(arguments),
            _ => fail(
                "termux_chatgpt_tool_unknown",
                format!("unknown tool '{name}'"),
            ),
        };
        // Fail closed: only explicit non-error terminal/active states clear `isError`.
        let is_error = !matches!(
            result.get("state").and_then(Value::as_str),
            Some("ok" | "settled" | "in_progress" | "stopped" | "idle")
        );
        (result, is_error)
    }
}

fn print_help() {
    println!(
        "termux-chatgpt-mcp {} — stateless @TermuxChatGPT MCP stdio bridge\n\n\
         Usage:\n\
         \x20 termux-chatgpt-mcp                   Run MCP stdio JSON-RPC 2.0 server\n\
         \x20 termux-chatgpt-mcp --describe-tools  Print static tools/list JSON and exit\n\
         \x20 termux-chatgpt-mcp --version         Print version and exit\n\
         \x20 termux-chatgpt-mcp --help            Show this help\n\n\
         Environment:\n\
         \x20 CHATGPT_PROVIDER_URL         Loopback provider URL (default http://127.0.0.1:4319)\n\
         \x20 CHATGPT_PROVIDER_TOKEN_FILE  Path to 0600 control.token (default ~/.local/state/codex/providers/chatgpt/control.token)\n\
         \x20 TERMUX_CHATGPT_RECEIPT_DIR   Private call_id receipt directory",
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
            println!("termux-chatgpt-mcp {}", env!("CARGO_PKG_VERSION"));
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
            eprintln!("termux-chatgpt-mcp: unknown argument '{other}' (try --help)");
            std::process::exit(2);
        }
    }

    let mut server = TermuxChatGptServer::new();
    if let Err(err) = serve_stdio(&mut server) {
        eprintln!("termux-chatgpt-mcp: stdio loop error: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_kit::handle_message;
    use std::collections::HashSet;

    #[test]
    fn exposes_exactly_six_static_tools_with_uniform_annotations_and_disjoint_extensions() {
        let mut srv = TermuxChatGptServer::new();
        let list_resp = handle_message(
            &mut srv,
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
        )
        .unwrap();
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 6);

        let mut static_names = HashSet::new();
        for t in tools {
            let name = t["name"].as_str().unwrap();
            static_names.insert(name.to_owned());
            assert_eq!(t["annotations"]["readOnlyHint"], true);
            assert_eq!(t["annotations"]["destructiveHint"], false);
            assert_eq!(t["annotations"]["openWorldHint"], false);
            if name == "chatgpt_call" {
                assert_eq!(t["annotations"]["idempotentHint"], false);
            } else {
                assert_eq!(t["annotations"]["idempotentHint"], true);
            }
        }
        assert_eq!(
            static_names,
            STATIC_TOOL_NAMES
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<HashSet<_>>()
        );

        let (cat, is_err) = srv.call_tool("chatgpt_catalog", &json!({}));
        assert!(!is_err);
        let actions = cat["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 5);
        for a in actions {
            let act_name = a["action"].as_str().unwrap();
            assert!(!static_names.contains(act_name));
            assert!(a["inputSchema"].is_object());
        }
    }

    #[test]
    fn strict_validation_self_corrects_and_stopped_provider_status_is_graceful() {
        let mut srv = TermuxChatGptServer::new();
        srv.provider_url = Some("http://127.0.0.1:1".to_owned());

        // Unknown extension action returns available_actions
        let (unk, is_err) = srv.call_tool("chatgpt_call", &json!({"action": "shell_exec"}));
        assert!(is_err);
        assert_eq!(unk["error"], "unknown_extension_action");
        assert!(unk["available_actions"]
            .as_array()
            .unwrap()
            .contains(&json!("project_create")));

        // Invalid extension payload returns exact expected_schema
        let (bad, is_err) = srv.call_tool(
            "chatgpt_call",
            &json!({"action": "chat_rename", "arguments": {"chat_id": "bad/id", "title": "x"}}),
        );
        assert!(is_err);
        assert_eq!(bad["error"], "chat_id_invalid");
        assert_eq!(
            bad["expected_schema"]["required"],
            json!(["chat_id", "title"])
        );

        // Non-loopback URL rejected fail-closed without external DNS
        srv.provider_url = Some("http://example.com:4319".to_owned());
        let (non_lb, is_err) = srv.call_tool("chatgpt_status", &json!({}));
        assert!(is_err);
        assert_eq!(non_lb["error"], "chatgpt_provider_non_loopback_rejected");

        // Traversal IDs ('.', '..', control chars, whitespace) rejected fail-closed
        for bad_id in [".", "..", "a\tb", "a b", "x\x1by"] {
            let (_, is_err) = srv.call_tool(
                "chatgpt_projects",
                &json!({"action": "read", "project_id": bad_id}),
            );
            assert!(is_err, "project_id={bad_id:?} must be rejected");
        }

        // Specifying both chat_id and project_id on send must be rejected
        let (both, is_err) = srv.call_tool(
            "chatgpt_send",
            &json!({"action": "send", "call_id": "c1", "chat_id": "ch1", "project_id": "p1", "prompt": "hi"}),
        );
        assert!(is_err);
        assert_eq!(both["error"], "invalid_arguments");

        // Null `arguments` on zero-arg tools and chatgpt_call accepted cleanly
        let null_resp = handle_message(
            &mut srv,
            &json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "chatgpt_catalog", "arguments": null}}),
        )
        .unwrap();
        assert_eq!(null_resp["result"]["isError"], false);
    }
}
