//! Thin adapters to existing authoritative owners (zero second executor).
//!
//! - ChatGPT sessions/projects: loopback `chatgpt-provider` `/control/*`.
//! - Codex threads/turns: official `codex app-server daemon` via `crates/codex-daemon-client`.
//! - User notifications: `termux-notification` (optional, non-fatal if absent).

use crate::http_client::{
    parse_loopback_base_url, read_control_token, request_json, url_encode, HttpClientError,
    LoopbackEndpoint,
};
use crate::ledger::now_unix;
use codex_daemon_client::{extract_item_text, DaemonClient};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const DEFAULT_PROVIDER_URL: &str = "http://127.0.0.1:4319";
pub const DEFAULT_429_COOLDOWN_SECS: u64 = 60;
pub const DEFAULT_MAX_CONCURRENT_CHATGPT: usize = 2;
const READ_TIMEOUT: Duration = Duration::from_secs(20);
const SEND_TIMEOUT: Duration = Duration::from_secs(90);
const DAEMON_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct OwnerConfig {
    pub provider_url: String,
    pub token_path: Option<PathBuf>,
    pub home: PathBuf,
    pub tmpdir: PathBuf,
    pub codex_socket_override: Option<PathBuf>,
    pub notify_cmd: Option<String>,
}

impl OwnerConfig {
    pub fn from_env() -> Self {
        let home = env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/data/com.termux/files/home"));
        let tmpdir = env::var_os("TMPDIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let prefix = env::var_os("PREFIX")
                    .filter(|v| !v.is_empty())
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/data/data/com.termux/files/usr"));
                prefix.join("tmp")
            });
        let provider_url = env::var("CHATGPT_PROVIDER_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PROVIDER_URL.to_owned());
        let token_path = env::var_os("CHATGPT_PROVIDER_TOKEN_FILE")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| Some(home.join(".local/state/codex/providers/chatgpt/control.token")));
        let codex_socket_override = env::var_os("ORCH_CODEX_SOCKET")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        let notify_cmd = env::var("ORCH_NOTIFY_CMD")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self {
            provider_url,
            token_path,
            home,
            tmpdir,
            codex_socket_override,
            notify_cmd,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatGptObservation {
    pub state: String,
    pub generating: bool,
    pub remote_stream_status: Option<String>,
    pub assistant_text: String,
    pub error: Option<String>,
    pub terminal_turn: bool,
    pub retry_after_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CodexObservation {
    pub status_type: String,
    pub is_active: bool,
    pub last_assistant_text: String,
    pub turn_status: Option<String>,
}

#[derive(Debug, Default)]
struct OwnerGate {
    chatgpt_cooldown_until_unix: AtomicU64,
    rate_limit_hits: AtomicU64,
    remote_chatgpt_calls: AtomicU64,
    local_observe_calls: AtomicU64,
    local_rate_limit_hits: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct OwnerClient {
    cfg: OwnerConfig,
    gate: Arc<OwnerGate>,
}

pub fn parse_retry_after_secs(val: &Value) -> Option<u64> {
    if let Some(n) = val.get("retry_after").and_then(Value::as_u64) {
        return Some(n.clamp(1, 600));
    }
    if let Some(s) = val.get("retry_after").and_then(Value::as_str) {
        if let Ok(n) = s.trim().parse::<u64>() {
            return Some(n.clamp(1, 600));
        }
    }
    None
}

pub fn is_rate_limited(val: &Value) -> bool {
    val.get("state").and_then(Value::as_str) == Some("error")
        && matches!(
            val.get("error").and_then(Value::as_str),
            Some("backend_http_429" | "retry_after_cooldown" | "local_rate_limit")
        )
}

impl OwnerClient {
    pub fn new(cfg: OwnerConfig) -> Self {
        Self {
            cfg,
            gate: Arc::new(OwnerGate::default()),
        }
    }

    pub fn chatgpt_cooldown_until_unix(&self) -> u64 {
        self.gate
            .chatgpt_cooldown_until_unix
            .load(Ordering::Acquire)
    }

    pub fn chatgpt_cooldown_remaining_secs(&self) -> u64 {
        self.chatgpt_cooldown_until_unix()
            .saturating_sub(now_unix())
    }

    pub fn rate_limit_hits(&self) -> u64 {
        self.gate.rate_limit_hits.load(Ordering::Acquire)
    }

    pub fn local_rate_limit_hits(&self) -> u64 {
        self.gate.local_rate_limit_hits.load(Ordering::Acquire)
    }

    pub fn remote_chatgpt_calls(&self) -> u64 {
        self.gate.remote_chatgpt_calls.load(Ordering::Acquire)
    }

    pub fn local_observe_calls(&self) -> u64 {
        self.gate.local_observe_calls.load(Ordering::Acquire)
    }

    pub fn note_chatgpt_rate_limit(&self, retry_after_secs: u64, context: &str) {
        let secs = retry_after_secs.clamp(1, 600);
        let target = now_unix().saturating_add(secs);
        self.gate
            .chatgpt_cooldown_until_unix
            .fetch_max(target, Ordering::AcqRel);
        let hits = self.gate.rate_limit_hits.fetch_add(1, Ordering::AcqRel) + 1;
        eprintln!(
            "{}",
            json!({
                "component": "termux-orch",
                "event": "chatgpt_rate_limit_cooldown",
                "context": context,
                "cooldown_secs": secs,
                "cooldown_until_unix": target,
                "rate_limit_hits": hits
            })
        );
    }

    pub fn max_concurrent_chatgpt(&self) -> usize {
        env::var("ORCH_MAX_CONCURRENT_CHATGPT")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .map(|n| n.clamp(1, 8))
            .unwrap_or(DEFAULT_MAX_CONCURRENT_CHATGPT)
    }

    fn resolve_provider(&self) -> Result<(LoopbackEndpoint, String), String> {
        let ep = parse_loopback_base_url(&self.cfg.provider_url)
            .map_err(|e| format!("{}: {}", e.code(), e.detail()))?;
        let tok_path = self
            .cfg
            .token_path
            .as_deref()
            .ok_or_else(|| "chatgpt_control_token_unavailable: no token path".to_owned())?;
        let token =
            read_control_token(tok_path).map_err(|e| format!("{}: {}", e.code(), e.detail()))?;
        Ok((ep, token))
    }

    fn control_call(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value, String> {
        let local_observe =
            path == "/control/observe" && body.is_some_and(|body| body["local_only"] == true);
        let exempt_from_cooldown =
            path.starts_with("/control/status?") || path == "/control/stop" || local_observe;
        if !exempt_from_cooldown {
            let rem = self.chatgpt_cooldown_remaining_secs();
            if rem > 0 {
                return Ok(json!({
                    "state": "error",
                    "error": "backend_http_429",
                    "detail": "OwnerGate cooldown active",
                    "retry_after": rem.to_string()
                }));
            }
        }
        let (ep, token) = self.resolve_provider()?;
        let counter = if local_observe {
            &self.gate.local_observe_calls
        } else {
            &self.gate.remote_chatgpt_calls
        };
        counter.fetch_add(1, Ordering::AcqRel);
        let resp =
            request_json(&ep, method, path, Some(&token), body, timeout).map_err(|e| match e {
                HttpClientError::Unavailable(c, d) | HttpClientError::Config(c, d) => {
                    format!("{c}: {d}")
                }
                HttpClientError::TransportLost(d) => {
                    format!("chatgpt_provider_transport_lost: {d}")
                }
            })?;
        if resp.status == 429 {
            let secs = resp
                .json
                .as_ref()
                .and_then(parse_retry_after_secs)
                .unwrap_or(DEFAULT_429_COOLDOWN_SECS);
            self.note_chatgpt_rate_limit(secs, path);
            return Ok(json!({
                "state": "error",
                "error": "backend_http_429",
                "detail": resp.raw_text.chars().take(300).collect::<String>(),
                "retry_after": secs.to_string()
            }));
        }
        if (200..300).contains(&resp.status) {
            let val = resp.json.unwrap_or_else(|| json!({ "raw": resp.raw_text }));
            if val["terminal_turn"] == true && path == "/control/observe" {
                return Ok(val);
            }
            if val["error"] == "local_rate_limit" {
                self.gate
                    .local_rate_limit_hits
                    .fetch_add(1, Ordering::AcqRel);
                let secs = parse_retry_after_secs(&val).unwrap_or(2);
                self.gate
                    .chatgpt_cooldown_until_unix
                    .fetch_max(now_unix().saturating_add(secs), Ordering::AcqRel);
            } else if is_rate_limited(&val) {
                let secs = parse_retry_after_secs(&val).unwrap_or(DEFAULT_429_COOLDOWN_SECS);
                self.note_chatgpt_rate_limit(secs, path);
            }
            Ok(val)
        } else {
            Err(format!(
                "chatgpt_provider_http_{}: {}",
                resp.status,
                resp.raw_text.chars().take(300).collect::<String>()
            ))
        }
    }

    pub fn chatgpt_status(&self) -> Result<Value, String> {
        self.control_call(
            "GET",
            "/control/status?account_only=true",
            None,
            READ_TIMEOUT,
        )
    }

    pub fn chatgpt_read_chat(&self, chat_id: &str, limit: u64) -> Result<Value, String> {
        self.control_call(
            "GET",
            &format!(
                "/control/chats/{}?limit={}",
                url_encode(chat_id),
                limit.clamp(1, 100)
            ),
            None,
            READ_TIMEOUT,
        )
    }

    pub fn chatgpt_has_delivery_marker(
        &self,
        chat_id: &str,
        project: Option<&str>,
        marker: &str,
    ) -> Result<bool, String> {
        let chat = self.chatgpt_read_chat(chat_id, 100)?;
        if chat["state"] != "ok" {
            let code = chat["error"].as_str().unwrap_or("read_chat_failed");
            let detail = chat["detail"].as_str().unwrap_or("");
            return Err(if detail.is_empty() {
                code.to_owned()
            } else {
                format!("{code}: {detail}")
            });
        }
        Ok(project.is_none_or(|project| chat["gizmo_id"] == project)
            && has_user_marker(&chat, marker))
    }

    pub fn chatgpt_rename_chat(&self, chat_id: &str, title: &str) -> Result<Value, String> {
        let receipt = self.control_call(
            "POST",
            &format!("/control/chats/{}/rename", url_encode(chat_id)),
            Some(&json!({ "title": title })),
            READ_TIMEOUT,
        )?;
        if receipt["state"] != "ok" {
            return Err(receipt["error"]
                .as_str()
                .unwrap_or("chatgpt_rename_rejected")
                .to_owned());
        }
        Ok(receipt)
    }

    pub fn chatgpt_create_project(
        &self,
        name: &str,
        instructions: Option<&str>,
    ) -> Result<Value, String> {
        self.control_call(
            "POST",
            "/control/projects",
            Some(&json!({
                "name": name,
                "instructions": instructions.unwrap_or("")
            })),
            READ_TIMEOUT,
        )
    }

    pub fn chatgpt_find_project(
        &self,
        name: &str,
        id: Option<&str>,
    ) -> Result<Option<(String, String)>, String> {
        if let Some(id) = id {
            let project = self.control_call(
                "GET",
                &format!("/control/projects/{}?metadata_only=true", url_encode(id)),
                None,
                READ_TIMEOUT,
            )?;
            if project["state"] != "ok" || project["project_id"] != id {
                return Err(project["error"]
                    .as_str()
                    .unwrap_or("project_identity_unavailable")
                    .to_owned());
            }
            let label = project["name"]
                .as_str()
                .filter(|label| !label.is_empty())
                .ok_or("project_name_unavailable")?;
            return Ok(Some((id.to_owned(), label.to_owned())));
        }
        let mut cursor = None::<String>;
        let mut found = None::<(String, String)>;
        let mut cursors = std::collections::HashSet::new();
        for _ in 0..3 {
            let path = match cursor.as_deref() {
                Some(cursor) => {
                    format!("/control/projects?limit=100&cursor={}", url_encode(cursor))
                }
                None => "/control/projects?limit=100".to_owned(),
            };
            let page = self.control_call("GET", &path, None, READ_TIMEOUT)?;
            let projects = page["projects"].as_array().ok_or_else(|| {
                page["error"]
                    .as_str()
                    .unwrap_or("invalid project list")
                    .to_owned()
            })?;
            for project in projects {
                let (Some(pid), Some(label)) = (project["id"].as_str(), project["name"].as_str())
                else {
                    continue;
                };
                if label == name {
                    if found.as_ref().is_some_and(|(old, _)| old != pid) {
                        return Err(
                            "multiple projects match this name; supply project_id".to_owned()
                        );
                    }
                    found = Some((pid.to_owned(), label.to_owned()));
                }
            }
            let Some(next) = page["next_cursor"].as_str().filter(|s| !s.is_empty()) else {
                return Ok(found);
            };
            if !cursors.insert(next.to_owned()) {
                return Err("project list cursor repeated".to_owned());
            }
            cursor = Some(next.to_owned());
        }
        Err("project discovery exceeded three pages; supply project_id".to_owned())
    }

    pub fn chatgpt_observe(&self, chat_id: &str) -> Result<ChatGptObservation, String> {
        self.chatgpt_observe_chat(chat_id, false)
    }

    pub fn chatgpt_observe_chat(
        &self,
        chat_id: &str,
        local_only: bool,
    ) -> Result<ChatGptObservation, String> {
        self.chatgpt_observe_tracked(chat_id, None, local_only)
    }

    pub fn chatgpt_observe_tracked(
        &self,
        chat_id: &str,
        lane_id: Option<&str>,
        local_only: bool,
    ) -> Result<ChatGptObservation, String> {
        let mut args = json!({ "chat_id": chat_id, "local_only": local_only });
        if let Some(lane) = lane_id {
            args["lane_id"] = json!(lane);
        }
        let val = self.control_call("POST", "/control/observe", Some(&args), READ_TIMEOUT)?;
        let state = val
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let remote_stream_status = val
            .get("remote_stream_status")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let generating = val
            .get("generating")
            .and_then(Value::as_bool)
            .unwrap_or(state == "in_progress")
            || remote_stream_status.as_deref() == Some("IS_STREAMING");
        let assistant_text = val
            .get("assistant_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let error = val
            .get("error")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let retry_after_secs = parse_retry_after_secs(&val);
        Ok(ChatGptObservation {
            state,
            generating,
            remote_stream_status,
            assistant_text,
            error,
            terminal_turn: val["terminal_turn"] == true,
            retry_after_secs,
        })
    }

    pub fn chatgpt_account_id(&self) -> Result<String, String> {
        let mut status = self.chatgpt_status()?;
        if status["auth_state"] == "host_stopped" {
            // A read-only control request starts the AppHost after a service restart.
            self.control_call("GET", "/control/projects?limit=1", None, READ_TIMEOUT)?;
            status = self.chatgpt_status()?;
        }
        if status["pinned_send"] != true {
            return Err("chatgpt_pinned_send_unavailable".to_owned());
        }
        if status["auth_state"] != "authenticated" {
            return Err("chatgpt_account_unavailable".to_owned());
        }
        chatgpt_account_identity(&status)
            .map(ToOwned::to_owned)
            .ok_or_else(|| "chatgpt_account_unavailable".to_owned())
    }

    pub fn chatgpt_observe_lane(&self, lane_id: &str) -> Result<Value, String> {
        self.control_call(
            "POST",
            "/control/observe",
            Some(&json!({"lane_id": lane_id, "local_only": true})),
            READ_TIMEOUT,
        )
    }

    pub fn reconcile_model_dispatch(
        &self,
        backend: &str,
        profile: Option<&str>,
        ref_id: &str,
        id: &str,
        account: Option<&str>,
        project: Option<&str>,
    ) -> Option<(String, Option<String>)> {
        let marker = format!("[ORCH_DISPATCH {id}]");
        match backend {
            "codex" if !ref_id.is_empty() => self
                .codex_find_marked_turn(profile, ref_id, &marker)
                .ok()
                .flatten()
                .map(|turn| (ref_id.to_owned(), Some(turn))),
            "chatgpt"
                if account.is_some_and(|expected| {
                    self.chatgpt_account_id()
                        .is_ok_and(|actual| actual == expected)
                }) =>
            {
                let lane = self.chatgpt_observe_lane(id).ok()?;
                let chat = lane["chat_id"]
                    .as_str()
                    .filter(|chat| !chat.is_empty() && lane["lane_id"] == id)?;
                let history = self.chatgpt_read_chat(chat, 100).ok()?;
                (history["state"] == "ok"
                    && history["gizmo_id"].as_str() == project
                    && has_user_marker(&history, &marker))
                .then(|| (chat.to_owned(), None))
            }
            _ => None,
        }
    }

    pub fn chatgpt_stop_lane(&self, lane_id: &str, chat_id: &str) -> Result<Value, String> {
        self.control_call(
            "POST",
            "/control/stop",
            Some(&json!({
                "lane_id":lane_id, "chat_id":chat_id,
            })),
            READ_TIMEOUT,
        )
    }

    pub fn chatgpt_dispatch_participant(
        &self,
        work: &crate::ledger::WorkRecord,
        lane_id: &str,
        prompt: &str,
        model: Option<&str>,
        reasoning_effort: Option<&str>,
    ) -> Result<Value, String> {
        self.control_call(
            "POST",
            "/control/send",
            Some(&json!({
                "expected_account_id":work.account_id, "project_id":work.project_id,
                "lane_id":lane_id, "prompt":prompt, "model":model,
                "reasoning_effort":reasoning_effort, "wait_ms":50, "orch_wake":true
            })),
            SEND_TIMEOUT,
        )
    }

    pub fn chatgpt_recent_chats(&self, project: Option<&str>) -> Result<Value, String> {
        let path = project.map_or_else(
            || "/control/chats?limit=2&archived=false".to_owned(),
            |project| format!("/control/projects/{}?limit=2", url_encode(project)),
        );
        self.control_call("GET", &path, None, READ_TIMEOUT)
    }

    pub fn chatgpt_send_pinned(
        &self,
        work: &crate::ledger::WorkRecord,
        lead: Option<&crate::ledger::LeadBinding>,
        chat_id: Option<&str>,
        lane_id: &str,
        prompt: &str,
    ) -> Result<Value, String> {
        self.control_call(
            "POST",
            "/control/send",
            Some(&json!({
                "chat_id": chat_id,
                "project_id": lead.and(work.project_id.as_deref()),
                "expected_account_id": work.account_id,
                "lane_id": lane_id,
                "prompt": prompt,
                "model": lead.and_then(|lead| lead.model.as_deref()),
                "reasoning_effort": lead.and_then(|lead| lead.reasoning_effort.as_deref()),
                "wait_ms": 50, "orch_wake":true
            })),
            SEND_TIMEOUT,
        )
    }

    pub fn resolve_codex_home(&self, profile: Option<&str>) -> Result<PathBuf, String> {
        let trimmed = profile.map(str::trim).filter(|s| !s.is_empty());
        match trimmed {
            None | Some("default") => Ok(env::var_os("CODEX_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| self.cfg.home.join(".codex"))),
            Some(name) => {
                if name.len() > 64
                    || name == "."
                    || name == ".."
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
                {
                    return Err("invalid codex profile name".to_owned());
                }
                let euid = unsafe { libc::geteuid() };
                let base = self.cfg.home.join(".local/share/codex/manager/profiles");
                let pdir = base.join(name);
                let home_dir = pdir.join("home");
                for check in [&base, &pdir, &home_dir] {
                    if let Ok(m) = fs::symlink_metadata(check) {
                        if m.file_type().is_symlink() || !m.is_dir() || m.uid() != euid {
                            return Err(format!("unsafe codex profile path: {}", check.display()));
                        }
                    }
                }
                Ok(home_dir)
            }
        }
    }

    fn connect_codex(&self, profile: Option<&str>) -> Result<DaemonClient, String> {
        if let Some(ref sock) = self.cfg.codex_socket_override {
            return DaemonClient::connect_socket_path(
                sock,
                DAEMON_TIMEOUT,
                "termux-orch",
                env!("CARGO_PKG_VERSION"),
            )
            .map_err(|e| e.to_string());
        }
        let codex_home = self.resolve_codex_home(profile)?;
        DaemonClient::connect(
            &codex_home,
            &self.cfg.tmpdir,
            DAEMON_TIMEOUT,
            "termux-orch",
            env!("CARGO_PKG_VERSION"),
        )
        .map_err(|e| e.to_string())
    }

    pub fn codex_preflight(&self, profile: Option<&str>) -> Result<(), String> {
        let mut client = self
            .connect_codex(profile)
            .map_err(|e| format!("codex_daemon_unavailable: {e}"))?;
        Self::check_codex_account(&mut client)
    }

    fn check_codex_account(client: &mut DaemonClient) -> Result<(), String> {
        let account = client
            .call("account/read", json!({"refreshToken":false}))
            .map_err(|e| format!("codex_authentication_unavailable: {e}"))?;
        match account["requiresOpenaiAuth"].as_bool() {
            Some(false) => Ok(()),
            Some(true) if account["account"].is_object() => Ok(()),
            Some(true) => Err("codex_authentication_required".to_owned()),
            None => {
                Err("codex_authentication_unavailable: malformed account/read response".to_owned())
            }
        }
    }

    pub fn codex_observe_thread(
        &self,
        profile: Option<&str>,
        thread_id: &str,
    ) -> Result<CodexObservation, String> {
        self.codex_observe_turn(profile, thread_id, None)
    }

    pub fn codex_observe_turn(
        &self,
        profile: Option<&str>,
        thread_id: &str,
        turn_id: Option<&str>,
    ) -> Result<CodexObservation, String> {
        let mut client = self.connect_codex(profile)?;
        let raw = client
            .read_thread_raw(thread_id, true)
            .map_err(|e| e.to_string())?;
        client.close();
        let thread = raw.get("thread").unwrap_or(&raw);
        let status_type = thread
            .get("status")
            .and_then(|s| s.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let is_active = status_type == "active";
        let (turn_status, last_assistant_text) = codex_turn_evidence(thread, turn_id);
        Ok(CodexObservation {
            status_type,
            is_active,
            last_assistant_text,
            turn_status,
        })
    }

    pub fn codex_interrupt_turn(
        &self,
        profile: Option<&str>,
        thread_id: &str,
        turn_id: &str,
    ) -> Result<(), String> {
        let mut client = self.connect_codex(profile)?;
        let result = client.call(
            "turn/interrupt",
            json!({"threadId":thread_id,"turnId":turn_id}),
        );
        client.close();
        result.map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn codex_has_delivery_marker(
        &self,
        profile: Option<&str>,
        thread_id: &str,
        marker: &str,
    ) -> Result<bool, String> {
        Ok(self
            .codex_find_marked_turn(profile, thread_id, marker)?
            .is_some())
    }

    pub fn codex_find_marked_turn(
        &self,
        profile: Option<&str>,
        thread_id: &str,
        marker: &str,
    ) -> Result<Option<String>, String> {
        let mut client = self.connect_codex(profile)?;
        let raw = client
            .read_thread_raw(thread_id, true)
            .map_err(|e| e.to_string())?;
        client.close();
        Ok(raw.get("thread").unwrap_or(&raw)["turns"]
            .as_array()
            .and_then(|turns| {
                turns.iter().find(|turn| {
                    turn["items"].as_array().is_some_and(|items| {
                        items.iter().any(|item| {
                            item["type"] == "userMessage"
                                && item["content"]
                                    .as_array()
                                    .and_then(|content| content.first())
                                    .is_some_and(|part| {
                                        part["type"] == "text"
                                            && part["text"].as_str().is_some_and(|text| {
                                                text.lines().next() == Some(marker)
                                            })
                                    })
                        })
                    })
                })
            })
            .and_then(|turn| turn["id"].as_str())
            .map(ToOwned::to_owned))
    }

    pub fn codex_rename_thread(
        &self,
        profile: Option<&str>,
        thread_id: &str,
        name: &str,
    ) -> Result<(), String> {
        let mut client = self.connect_codex(profile)?;
        let res = client.call(
            "thread/name/set",
            json!({ "threadId": thread_id, "name": name }),
        );
        client.close();
        res.map(|_| ()).map_err(|e| e.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn codex_dispatch_turn(
        &self,
        profile: Option<&str>,
        thread_id: Option<&str>,
        cwd: Option<&str>,
        prompt: &str,
        model: Option<&str>,
        model_provider: Option<&str>,
        reasoning_effort: Option<&str>,
        approval_policy: Option<&str>,
        before_turn: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<(String, Option<String>), String> {
        let mut client = self
            .connect_codex(profile)
            .map_err(|e| format!("codex_preflight_failed: {e}"))?;
        Self::check_codex_account(&mut client)
            .map_err(|e| format!("codex_preflight_failed: {e}"))?;
        let tid = if let Some(existing) = thread_id {
            let _ = client
                .call("thread/resume", json!({ "threadId": existing }))
                .map_err(|e| e.to_string())?;
            existing.to_owned()
        } else {
            let mut start_params = json!({});
            if let Some(dir) = cwd {
                start_params["cwd"] = json!(dir);
            }
            if let Some(m) = model {
                start_params["model"] = json!(m);
            }
            if let Some(mp) = model_provider {
                start_params["modelProvider"] = json!(mp);
            }
            if let Some(ap) = approval_policy {
                start_params["approvalPolicy"] = json!(ap);
            }
            let started = client
                .call("thread/start", start_params)
                .map_err(|e| e.to_string())?;
            started
                .get("thread")
                .and_then(|t| t.get("id"))
                .or_else(|| started.get("threadId"))
                .and_then(Value::as_str)
                .ok_or_else(|| "codex thread/start missing thread id".to_owned())?
                .to_owned()
        };

        before_turn(&tid)?;

        let turn_params = build_codex_turn_start_params(
            &tid,
            cwd,
            prompt,
            model,
            reasoning_effort,
            approval_policy,
        );
        let turn_res = client
            .call("turn/start", turn_params)
            .map_err(|e| e.to_string())?;
        client.close();
        let turn_id = turn_res
            .get("turn")
            .and_then(|t| t.get("id"))
            .or_else(|| turn_res.get("turnId"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        Ok((tid, turn_id))
    }

    /// Best-effort Android Termux notification (bounded 5s, non-fatal if absent).
    pub fn send_notification(&self, id: &str, title: &str, content: &str) -> bool {
        let program = self
            .cfg
            .notify_cmd
            .as_deref()
            .unwrap_or("termux-notification");
        let short_title: String = title.chars().take(120).collect();
        let short_body: String = content.chars().take(400).collect();
        let mut child = match Command::new(program)
            .args([
                "--id",
                id,
                "--title",
                &short_title,
                "--content",
                &short_body,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        false
    }
}

pub fn chatgpt_account_identity(status: &Value) -> Option<&str> {
    let account = status.get("account")?;
    ["account_id", "id"]
        .into_iter()
        .find_map(|key| account[key].as_str().filter(|id| !id.is_empty()))
}

pub fn has_user_marker(chat: &Value, marker: &str) -> bool {
    chat["messages"].as_array().is_some_and(|rows| {
        rows.iter().any(|row| {
            row["role"] == "user"
                && row["text"]
                    .as_str()
                    .is_some_and(|text| text.lines().next() == Some(marker))
        })
    })
}

fn codex_turn_evidence(thread: &Value, turn_id: Option<&str>) -> (Option<String>, String) {
    let turn = thread["turns"].as_array().and_then(|turns| match turn_id {
        Some(id) => turns.iter().find(|turn| turn["id"] == id),
        None => turns.last(),
    });
    let Some(turn) = turn else {
        return (None, String::new());
    };
    let status = turn["status"].as_str().map(ToOwned::to_owned);
    let result = turn["items"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .filter(|item| {
            matches!(
                item["type"].as_str(),
                Some("agentMessage" | "assistantMessage")
            )
        })
        .filter_map(|item| extract_item_text(item, |text| !text.trim().is_empty(), |_| true))
        .find(|text| !text.trim().is_empty())
        .map(|text| text.chars().take(2000).collect())
        .unwrap_or_default();
    (status, result)
}

fn build_codex_turn_start_params(
    thread_id: &str,
    cwd: Option<&str>,
    prompt: &str,
    model: Option<&str>,
    reasoning_effort: Option<&str>,
    approval_policy: Option<&str>,
) -> Value {
    let mut turn_params = json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": prompt }]
    });
    if let Some(dir) = cwd {
        turn_params["cwd"] = json!(dir);
    }
    if let Some(m) = model {
        turn_params["model"] = json!(m);
    }
    if let Some(eff) = reasoning_effort.map(str::trim).filter(|s| !s.is_empty()) {
        turn_params["effort"] = json!(eff);
    }
    if let Some(ap) = approval_policy {
        turn_params["approvalPolicy"] = json!(ap);
    }
    turn_params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_turn_start_params_forward_effort_without_hardcoded_models() {
        let params = build_codex_turn_start_params(
            "thread-1",
            Some("/tmp/work"),
            "check status",
            None,
            Some("high"),
            Some("never"),
        );
        assert_eq!(params["threadId"], "thread-1");
        assert_eq!(params["cwd"], "/tmp/work");
        assert_eq!(params["effort"], "high");
        assert_eq!(params["approvalPolicy"], "never");
        assert!(params.get("model").is_none());
    }

    #[test]
    fn codex_turn_evidence_uses_last_nonblank_agent_item_and_character_limit() {
        let long = "한".repeat(2001);
        let thread = json!({"turns":[{"id":"turn","status":"completed","items":[
            {"type":"agentMessage","text":"old"},
            {"type":"assistantMessage","text":" ","content":[{"text":""},{"text":long}]},
            {"type":"agentMessage","text":" ","content":[{"text":" "}]},
            {"type":"message","text":"ignored"}
        ]}]});
        let (status, text) = codex_turn_evidence(&thread, Some("turn"));
        assert_eq!(status.as_deref(), Some("completed"));
        assert_eq!(text, format!("\n{}", "한".repeat(1999)));
    }

    #[test]
    fn codex_turn_evidence_stays_attached_to_the_registered_turn() {
        let thread = json!({"turns":[
            {"id":"first","status":"failed","items":[]},
            {"id":"second","status":"completed","items":[{"type":"agentMessage","text":"new result"}]}
        ]});
        assert_eq!(
            codex_turn_evidence(&thread, Some("first")),
            (Some("failed".into()), String::new())
        );
        assert_eq!(
            codex_turn_evidence(&thread, Some("missing")),
            (None, String::new())
        );
        assert_eq!(
            codex_turn_evidence(&thread, None),
            (Some("completed".into()), "new result".into())
        );
        assert_eq!(codex_turn_evidence(&json!({}), None), (None, String::new()));
    }
}
