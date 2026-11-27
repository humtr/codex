//! Single-turn MCP request planner, utility-turn isolation, workspace project
//! derivation, and extensible `TurnStrategy` seam.

use crate::config::McpConfig;
use crate::error::{ProviderError, Result};
use crate::session::{prefix_extend, wire_uuid, ResponseThreadMap, ThreadBinding, ThreadEntry};
use serde_json::{json, Value};
use std::path::Path;

/// Extensible execution-mode seam for `/v1/responses` turns.
/// Active production runs `McpSingleTurn`; archived client-side local tools
/// (`providers/chatgpt/archive/local-tools/`) can be plugged in via a future
/// variant without changing `catalog.rs`, `session.rs`, or `stream.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TurnStrategy {
    #[default]
    McpSingleTurn,
}

pub struct TurnPlan {
    pub messages: Vec<Value>,
    pub input_len: usize,
    pub input_digest: [u8; 32],
    pub parent: String,
    pub fresh_handoff: bool,
    pub response_id: String,
}

pub fn extract_content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                p.as_str()
                    .or_else(|| p.get("text").and_then(Value::as_str))
                    .or_else(|| p.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn matches_utility_prompt(lower: &str) -> bool {
    lower.contains("part of an agent memory system")
        || lower.contains("produce json with `rollout_summary` and `rollout_slug`")
        || lower.contains("consolidate the supplied rollout summaries into `memory_summary.md`")
        || lower.contains("concise, single-line task title")
        || lower.contains("generate a concise title")
        || lower.contains("transcript title generation")
        || lower.contains("generate a short title")
        || lower.contains("title for this conversation")
        || lower.contains("memory writing agent")
        || lower.contains("memory consolidation")
}

fn matches_utility_kind(kind: &str) -> bool {
    matches!(
        kind,
        "title" | "memory" | "memories" | "guardian" | "auto_review"
    ) || kind.starts_with("title_")
        || kind.starts_with("memory_")
}

/// Detect Codex background utility calls without diverting normal turns or
/// compaction/handoff into the temporary utility lane.
pub fn is_background_utility_request(turn_meta: Option<&Value>, body: &Value) -> bool {
    if let Some(meta) = turn_meta {
        let kinds = ["request_kind", "thread_source", "turn_trigger"].map(|key| {
            meta.get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        });
        if kinds
            .iter()
            .any(|kind| kind.contains("compact") || kind.contains("handoff"))
        {
            return false;
        }
        if !kinds[0].is_empty() {
            return matches_utility_kind(&kinds[0]);
        }
        if kinds[1..].iter().any(|kind| matches_utility_kind(kind)) {
            return true;
        }
    }
    if body
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| !tools.is_empty())
        || body
            .get("previous_response_id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty())
    {
        return false;
    }
    let mut prompt = body
        .get("instructions")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if let Some(Value::Array(items)) = body.get("input") {
        for item in items.iter().rev().take(4) {
            if item
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.ends_with("_call") || kind.ends_with("_call_output"))
            {
                return false;
            }
            prompt.push('\n');
            prompt.push_str(&extract_content_text(item.get("content")));
        }
    }
    let prompt = prompt.to_ascii_lowercase();
    ![
        "context checkpoint compaction",
        "produce a summary of its thinking process",
        "compact the conversation",
    ]
    .iter()
    .any(|directive| prompt.contains(directive))
        && matches_utility_prompt(&prompt)
}

/// Detect Codex's automatic machine-generated `role: "user"` preambles.
/// Kept as `role: "user"` (zero privilege escalation) and marked `hidden: true`
/// so ChatGPT UI displays only genuine human prompts.
pub fn is_codex_machine_user_preamble(parts: &[String]) -> bool {
    let Some(first) = parts.first().map(|s| s.trim_start()) else {
        return false;
    };
    first.starts_with("# AGENTS.md instructions")
        || first.starts_with("<environment_context>")
        || first.starts_with("<collaboration_mode>")
        || first.starts_with("<turn_aborted>")
        || first.starts_with("<subagent_notification>")
}

/// Extract workspace `cwd` from turn metadata or Codex's `<environment_context>`.
pub fn extract_workspace_cwd(body: &Value, turn_meta: Option<&Value>) -> Option<String> {
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
pub fn derive_auto_project_name(cwd: Option<&str>) -> String {
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
        || trimmed == "/var/tmp"
        || trimmed.starts_with("/var/tmp/")
        || trimmed == "/data/data/com.termux/files/usr/tmp"
        || trimmed.starts_with("/data/data/com.termux/files/usr/tmp/")
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

/// Build slim wire messages for background utility calls (omits heavy Codex boilerplate).
pub fn build_utility_wire_messages(body: &Value) -> Result<Vec<Value>> {
    let mut messages = Vec::new();
    match body.get("input") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            messages.push(json!({
                "id": wire_uuid()?,
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
                let trimmed = text.trim_start();
                if trimmed.starts_with("# AGENTS.md instructions")
                    || trimmed.starts_with("<environment_context>")
                    || trimmed.starts_with("<skills_instructions>")
                    || trimmed.starts_with("<permissions instructions>")
                {
                    continue;
                }
                if role == "developer" && !matches_utility_prompt(&trimmed.to_ascii_lowercase()) {
                    continue;
                }
                messages.push(json!({
                    "id": wire_uuid()?,
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
            "id": wire_uuid()?,
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

fn text_parts(value: &Value, allow_empty: bool) -> Result<Vec<String>> {
    let err = || {
        ProviderError::bad_request(
            "native_content_unsupported",
            "provider supports text parts only; no lossy conversion",
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
                "text parts empty or exceed 1 MiB bound",
            ));
        }
    }
    if parts.iter().map(String::len).sum::<usize>() > 1_048_576 {
        return Err(ProviderError::bad_request(
            "native_content_bound",
            "text parts empty or exceed 1 MiB bound",
        ));
    }
    Ok(parts)
}

pub fn canonical_item(item: &Value) -> Result<Option<Value>> {
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
            let content = text_parts(item.get("content").ok_or_else(err)?, false)?;
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
            Ok(Some(json!({
                "type": kind,
                "call_id": cid,
                "name": name,
                "namespace": item.get("namespace").and_then(Value::as_str),
                "input": args
            })))
        }
        "function_call_output" | "custom_tool_call_output" => {
            let cid = item
                .get("call_id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(err)?;
            let parts = text_parts(item.get("output").ok_or_else(err)?, true)?;
            Ok(Some(json!({"type": kind, "call_id": cid, "parts": parts})))
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
            Ok(Some(json!({"type": "agent_message", "parts": [text]})))
        }
        "reasoning" => {
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
            Ok(Some(json!({"type": "reasoning", "summary": summary})))
        }
        _ => Err(err()),
    }
}

pub fn canonical_input(body: &Value) -> Result<Vec<Value>> {
    match body.get("input") {
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(vec![canonical_item(
            &json!({"role": "user", "content": text}),
        )?
        .expect("user message")]),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| canonical_item(item).transpose())
            .collect(),
        _ => Err(ProviderError::bad_request(
            "responses_input_required",
            "input must be a string or array of items",
        )),
    }
}

/// Build the concise one-turn MCP execution bridge directive appended to the
/// trailing `developer` message so ChatGPT uses registered remote MCP connectors
/// directly inside this single `/f/conversation` turn instead of retired local
/// CLI function tools (`exec_command`, `apply_patch`).
pub fn format_mcp_bridge_directive(
    cwd: Option<&str>,
    active_connectors: &[String],
    mcp_cfg: &McpConfig,
) -> Option<String> {
    if !mcp_cfg.bridge_guidance {
        return None;
    }
    let connectors: Vec<&str> = if !mcp_cfg.preferred_connectors.is_empty() {
        mcp_cfg
            .preferred_connectors
            .iter()
            .map(String::as_str)
            .filter(|s| !s.trim().is_empty())
            .collect()
    } else if !active_connectors.is_empty() {
        active_connectors.iter().map(String::as_str).collect()
    } else {
        vec!["TermuxShell", "TermuxChatGPT", "TermuxCodex"]
    };
    let mut lines = vec![
        "[Termux MCP Single-Turn Execution Mode]".to_owned(),
        "Client-side local CLI tools (exec_command, apply_patch, update_plan, etc.) are disabled on this transport. Complete all host work inside this single turn using the connected ChatGPT MCP connectors:".to_owned(),
        format!("- Active MCP connectors: {}", connectors.join(", ")),
    ];
    if let Some(dir) = cwd.map(str::trim).filter(|s| !s.is_empty()) {
        lines.push(format!("- Workspace cwd: {dir}"));
    }
    lines.push(
        "- Role split: use TermuxShell (`termux_shell` with required `call_id` and `command`) for host shell/file/git/build/test work; use TermuxChatGPT to read/manage other ChatGPT chats or projects (never target the currently generating chat); use TermuxCodex for independent Codex daemon threads."
            .to_owned(),
    );
    lines.push(
        "- Batch independent commands where sensible, avoid retired `@TermuxBridge`, and return the full answer in this single turn."
            .to_owned(),
    );
    Some(lines.join("\n"))
}

impl TurnPlan {
    pub fn prepare(
        body: &Value,
        previous: Option<&ThreadEntry>,
        response_id: &str,
        _strategy: TurnStrategy,
        cwd: Option<&str>,
        active_connectors: &[String],
        mcp_cfg: &McpConfig,
    ) -> Result<Self> {
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
        if body
            .get("previous_response_id")
            .is_some_and(|v| !v.is_null() && v.as_str().is_none_or(str::is_empty))
        {
            return Err(ProviderError::bad_request(
                "previous_response_unavailable",
                "invalid previous_response_id",
            ));
        }
        let canonical = canonical_input(body)?;
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
                    && prefix_extend([0; 32], &canonical[..entry.input_len]) == entry.prefix_sha256;
                let matches_request_prefix = state.request_input_len > 0
                    && canonical.len() >= state.request_input_len
                    && prefix_extend([0; 32], &canonical[..state.request_input_len])
                        == state.request_input_sha256;
                let has_head = state
                    .remote_head
                    .as_deref()
                    .is_some_and(|id| !id.is_empty());
                let is_delta_with_prev = explicit_previous.is_some()
                    && !canonical.is_empty()
                    && canonical.iter().all(|item| {
                        item["type"] == "message"
                            && matches!(item["role"].as_str(), Some("developer" | "user"))
                    })
                    && canonical.iter().any(|item| item["role"] == "user");
                let is_compacted_fresh = explicit_previous.is_none()
                    && !canonical.is_empty()
                    && canonical
                        .iter()
                        .any(|item| item["type"] == "message" && item["role"] == "user");
                if matches_prefix && has_head {
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
                    (
                        entry.input_len,
                        entry.prefix_sha256,
                        canonical.as_slice(),
                        state.remote_head.clone().expect("has_head"),
                    )
                } else if is_compacted_fresh {
                    fresh_handoff = true;
                    (0, [0; 32], canonical.as_slice(), wire_uuid()?)
                } else {
                    return Err(ProviderError::bad_request(
                        "native_context_mismatch",
                        "committed input/output prefix changed; compaction or fresh session required",
                    ));
                }
            } else {
                fresh_handoff = true;
                (0, [0; 32], canonical.as_slice(), wire_uuid()?)
            }
        } else {
            if explicit_previous.is_some() {
                return Err(ProviderError::bad_request(
                    "previous_response_unavailable",
                    "unknown prior response",
                ));
            }
            (0, [0; 32], canonical.as_slice(), wire_uuid()?)
        };

        if tail.is_empty() {
            return Err(ProviderError::bad_request(
                "response_input_already_sent",
                "no new input follows the committed response",
            ));
        }

        let mut messages = Vec::new();
        for item in tail {
            match item["type"].as_str() {
                Some("message") if item["role"] == "developer" || item["role"] == "user" => {
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
                        "id": wire_uuid()?,
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
                        "id": wire_uuid()?,
                        "role": "user",
                        "parts": item["parts"]
                    }));
                }
                Some("function_call_output" | "custom_tool_call_output") => {
                    return Err(ProviderError::bad_request(
                        "local_tools_retired",
                        "client-side local_function tool outputs are retired; use ChatGPT remote MCP connectors in single-turn mode",
                    ));
                }
                // Historical assistant/reasoning/call items may appear when resuming an older
                // compacted transcript on a fresh handoff; skip them and forward user/developer.
                Some("message" | "reasoning" | "function_call" | "custom_tool_call")
                    if fresh_handoff || previous.is_none() => {}
                _ => {
                    return Err(ProviderError::bad_request(
                        "native_input_unsupported",
                        "send user or developer input in single-turn MCP mode",
                    ));
                }
            }
        }
        if !messages.iter().any(|message| message["role"] == "user") {
            return Err(ProviderError::bad_request(
                "native_user_input_required",
                "developer context must accompany a genuine user message",
            ));
        }

        let bridge = format_mcp_bridge_directive(cwd, active_connectors, mcp_cfg);
        let combined_developer = match (instructions, bridge) {
            (Some(inst), Some(guide)) => Some(format!("{inst}\n\n{guide}")),
            (Some(inst), None) => Some(inst.to_owned()),
            (None, Some(guide)) => Some(guide),
            (None, None) => None,
        };
        if let Some(dev_text) = combined_developer {
            messages.push(json!({
                "id": wire_uuid()?,
                "role": "developer",
                "parts": [dev_text],
                "one_turn": true
            }));
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
            input_digest: prefix_extend(base_digest, tail),
            parent,
            fresh_handoff,
            response_id: response_id.to_owned(),
        })
    }

    pub fn commit(
        &self,
        threads: &ResponseThreadMap,
        session_key: Option<&str>,
        settled: &Value,
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
        let canonical_output = output
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| canonical_item(item).transpose())
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let entry = ThreadEntry {
            chat_id: chat_id.to_owned(),
            account_id,
            project_id,
            input_len: self.input_len + canonical_output.len(),
            prefix_sha256: prefix_extend(self.input_digest, &canonical_output),
            native: Some(ThreadBinding {
                response_id: self.response_id.clone(),
                remote_head,
                request_input_len: self.input_len,
                request_input_sha256: self.input_digest,
            }),
        };
        let key = match session_key {
            Some(k) => k.to_owned(),
            None => format!("response:{}", self.response_id),
        };
        threads.record([(key, entry)])
    }
}
