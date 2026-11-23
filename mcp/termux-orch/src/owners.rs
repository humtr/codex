//! Thin adapters to existing authoritative owners (zero second executor).
//!
//! - ChatGPT sessions/projects: loopback `chatgpt-provider` `/control/*`.
//! - Codex threads/turns: official `codex app-server daemon` via `crates/codex-daemon-client`.
//! - User notifications: `termux-notification` (optional, non-fatal if absent).

use crate::http_client::{
    parse_loopback_base_url, read_control_token, request_json, url_encode, HttpClientError,
    LoopbackEndpoint,
};
use codex_daemon_client::DaemonClient;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_PROVIDER_URL: &str = "http://127.0.0.1:4319";
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
}

#[derive(Debug, Clone)]
pub struct CodexObservation {
    pub status_type: String,
    pub is_active: bool,
    pub last_assistant_text: String,
}

#[derive(Debug, Clone)]
pub struct OwnerClient {
    cfg: OwnerConfig,
}

impl OwnerClient {
    pub fn new(cfg: OwnerConfig) -> Self {
        Self { cfg }
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
        let (ep, token) = self.resolve_provider()?;
        let resp =
            request_json(&ep, method, path, Some(&token), body, timeout).map_err(|e| match e {
                HttpClientError::Unavailable(c, d) | HttpClientError::Config(c, d) => {
                    format!("{c}: {d}")
                }
                HttpClientError::TransportLost(d) => {
                    format!("chatgpt_provider_transport_lost: {d}")
                }
            })?;
        if (200..300).contains(&resp.status) {
            Ok(resp.json.unwrap_or_else(|| json!({ "raw": resp.raw_text })))
        } else {
            Err(format!(
                "chatgpt_provider_http_{}: {}",
                resp.status,
                resp.raw_text.chars().take(300).collect::<String>()
            ))
        }
    }

    pub fn chatgpt_status(&self) -> Result<Value, String> {
        self.control_call("GET", "/control/status", None, READ_TIMEOUT)
    }

    pub fn chatgpt_list_chats(&self, limit: u64) -> Result<Value, String> {
        self.control_call(
            "GET",
            &format!("/control/chats?limit={}&archived=false", limit.clamp(1, 50)),
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

    pub fn chatgpt_rename_chat(&self, chat_id: &str, title: &str) -> Result<Value, String> {
        self.control_call(
            "POST",
            &format!("/control/chats/{}/rename", url_encode(chat_id)),
            Some(&json!({ "title": title })),
            READ_TIMEOUT,
        )
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

    pub fn chatgpt_observe(&self, chat_id: &str) -> Result<ChatGptObservation, String> {
        let val = self.control_call(
            "POST",
            "/control/observe",
            Some(&json!({ "chat_id": chat_id })),
            READ_TIMEOUT,
        )?;
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
        Ok(ChatGptObservation {
            state,
            generating,
            remote_stream_status,
            assistant_text,
            error,
        })
    }

    pub fn chatgpt_send(
        &self,
        chat_id: Option<&str>,
        project_id: Option<&str>,
        prompt: &str,
        model: Option<&str>,
        reasoning_effort: Option<&str>,
        wait_ms: u64,
    ) -> Result<Value, String> {
        let timeout = Duration::from_millis(wait_ms.min(120_000) + 30_000).max(SEND_TIMEOUT);
        self.control_call(
            "POST",
            "/control/send",
            Some(&json!({
                "chat_id": chat_id,
                "project_id": project_id,
                "prompt": prompt,
                "model": model,
                "reasoning_effort": reasoning_effort,
                "wait_ms": wait_ms.min(300_000)
            })),
            timeout,
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

    pub fn codex_observe_thread(
        &self,
        profile: Option<&str>,
        thread_id: &str,
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
        let last_assistant_text = extract_codex_last_assistant_text(thread);
        Ok(CodexObservation {
            status_type,
            is_active,
            last_assistant_text,
        })
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
        approval_policy: Option<&str>,
    ) -> Result<(String, Option<String>), String> {
        let mut client = self.connect_codex(profile)?;
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

        let mut turn_params = json!({
            "threadId": tid,
            "input": [{ "type": "text", "text": prompt }]
        });
        if let Some(dir) = cwd {
            turn_params["cwd"] = json!(dir);
        }
        if let Some(m) = model {
            turn_params["model"] = json!(m);
        }
        if let Some(ap) = approval_policy {
            turn_params["approvalPolicy"] = json!(ap);
        }
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

pub fn extract_codex_last_assistant_text(thread: &Value) -> String {
    let Some(turns) = thread.get("turns").and_then(Value::as_array) else {
        return String::new();
    };
    for turn in turns.iter().rev() {
        if let Some(items) = turn.get("items").and_then(Value::as_array) {
            for item in items.iter().rev() {
                let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
                if kind == "agentMessage" || kind == "assistantMessage" {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        if !text.trim().is_empty() {
                            return text.chars().take(2000).collect();
                        }
                    }
                    if let Some(content) = item.get("content").and_then(Value::as_array) {
                        let joined: String = content
                            .iter()
                            .filter_map(|c| c.get("text").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("\n");
                        if !joined.trim().is_empty() {
                            return joined.chars().take(2000).collect();
                        }
                    }
                }
            }
        }
        if let Some(err) = turn
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
        {
            return format!("turn_error: {}", err.chars().take(500).collect::<String>());
        }
    }
    String::new()
}
