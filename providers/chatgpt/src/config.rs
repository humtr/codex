//! Configuration and private state layout for `chatgpt-provider`.
//!
//! Unified under the canonical XDG `codex/` namespace with strict role separation:
//! - Config: `~/.config/codex/providers/chatgpt.toml`
//! - Immutable host generations: `~/.local/lib/codex/host/chatgpt-desktop/`
//! - Persistent login profile: `~/.local/share/codex/chatgpt-desktop/profile/`
//! - Ephemeral runtime state: `~/.local/state/codex/providers/chatgpt/`

use crate::error::{ProviderError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub host: HostConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub history: HistoryConfig,
    #[serde(default)]
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Inject a concise one-turn MCP role-bridge developer directive on Codex CLI turns
    /// so ChatGPT uses connected remote MCP tools inside a single turn (default: true).
    #[serde(default = "default_true")]
    pub bridge_guidance: bool,
    /// Optional explicit list of active MCP connector names to surface in bridge guidance.
    /// When empty, connected Stage 2 connectors are discovered automatically.
    #[serde(default)]
    pub preferred_connectors: Vec<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            bridge_guidance: true,
            preferred_connectors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostConfig {
    /// Generation install root (default: `~/.local/lib/codex/host/chatgpt-desktop`).
    pub install_root: Option<PathBuf>,
    /// Optional explicit rootfs override (otherwise `<install_root>/current/rootfs`).
    pub rootfs: Option<PathBuf>,
    /// Separated persistent login profile dir (default: `~/.local/share/codex/chatgpt-desktop/profile`).
    pub profile_dir: Option<PathBuf>,
    /// Optional noVNC static assets directory for `/viewer/{capability}/...`.
    pub viewer_assets: Option<PathBuf>,
    /// Start the Linux ChatGPT host eagerly on server boot (default: false, lazy on demand).
    #[serde(default)]
    pub eager_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Localhost-only bind address (default: `127.0.0.1:4319`).
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Private state directory for `control.token` and runtime metadata.
    pub state_dir: Option<PathBuf>,
    /// Minimum time between AppHost completion dispatches (milliseconds).
    #[serde(default)]
    pub min_request_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_model")]
    pub default: String,
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
    #[serde(default = "default_models")]
    pub available: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    /// Optional explicit ChatGPT Project ID for background utility requests
    /// (`title_gen`, `memories`, `auto_review`). When unset (and `utility_project_name`
    /// is unset), background utility calls run in a temporary chat on `-instant`.
    #[serde(default)]
    pub utility_project_id: Option<String>,
    /// Optional auto-created utility project name (created with `memory_scope: "project_v2"`
    /// so utility traffic stays isolated in project-only memory).
    #[serde(default)]
    pub utility_project_name: Option<String>,
    /// Automatically match or create a ChatGPT Project from Codex's workspace `cwd`
    /// when no explicit `project_id`/`project_name` is set (default: true).
    #[serde(default = "default_true")]
    pub auto_project: bool,
    /// Compatibility field kept so existing `chatgpt.toml` files parse cleanly;
    /// structured Option B is now the unified runtime path.
    #[serde(default = "default_true")]
    pub structured_native_return: bool,
}

fn default_true() -> bool {
    true
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            project_id: None,
            project_name: None,
            utility_project_id: None,
            utility_project_name: None,
            auto_project: true,
            structured_native_return: true,
        }
    }
}

fn default_bind() -> String {
    "127.0.0.1:4319".to_owned()
}

fn default_model() -> String {
    "auto".to_owned()
}

fn default_reasoning_effort() -> String {
    "medium".to_owned()
}

fn default_models() -> Vec<String> {
    vec!["auto".to_owned()]
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            state_dir: None,
            min_request_interval_ms: 0,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model(),
            reasoning_effort: default_reasoning_effort(),
            available: default_models(),
        }
    }
}

impl ProviderConfig {
    pub fn default_path(home: &Path) -> PathBuf {
        home.join(".config/codex/providers/chatgpt.toml")
    }

    pub fn load_or_default(path: Option<&Path>, home: &Path) -> Result<Self> {
        let target = match path {
            Some(p) => p.to_path_buf(),
            None => Self::default_path(home),
        };
        if !target.exists() {
            if path.is_some() {
                return Err(ProviderError::bad_request(
                    "config_not_found",
                    format!("config file not found: {}", target.display()),
                ));
            }
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&target).map_err(|e| {
            ProviderError::bad_request("config_read_failed", format!("{}: {e}", target.display()))
        })?;
        let cfg: Self = toml::from_str(&raw).map_err(|e| {
            ProviderError::bad_request("config_parse_failed", format!("{}: {e}", target.display()))
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        let addr: std::net::SocketAddr = self.server.bind.parse().map_err(|e| {
            ProviderError::bad_request(
                "invalid_bind_address",
                format!("invalid server.bind '{}': {e}", self.server.bind),
            )
        })?;
        if !addr.ip().is_loopback() {
            return Err(ProviderError::bad_request(
                "loopback_bind_required",
                "server.bind must be a loopback address (127.0.0.1 or ::1)",
            ));
        }
        Ok(())
    }

    pub fn install_root(&self, home: &Path) -> PathBuf {
        self.host
            .install_root
            .clone()
            .unwrap_or_else(|| home.join(".local/lib/codex/host/chatgpt-desktop"))
    }

    pub fn resolved_rootfs(&self, home: &Path) -> PathBuf {
        if let Some(ref explicit) = self.host.rootfs {
            return explicit.clone();
        }
        self.install_root(home).join("current/rootfs")
    }

    pub fn profile_dir(&self, home: &Path) -> PathBuf {
        self.host
            .profile_dir
            .clone()
            .unwrap_or_else(|| home.join(".local/share/codex/chatgpt-desktop/profile"))
    }

    pub fn resolved_viewer_assets(&self, home: &Path) -> Option<PathBuf> {
        if let Some(ref explicit) = self.host.viewer_assets {
            return Some(explicit.clone());
        }
        let candidate = self.install_root(home).join("viewer");
        if candidate.join("vnc.html").is_file() {
            return Some(candidate);
        }
        None
    }

    pub fn state_dir(&self, home: &Path) -> PathBuf {
        self.server
            .state_dir
            .clone()
            .unwrap_or_else(|| home.join(".local/state/codex/providers/chatgpt"))
    }

    pub fn response_state_dir(&self, home: &Path) -> PathBuf {
        if self.history.structured_native_return {
            return self.state_dir(home).join("structured-native-return");
        }
        if self.server.state_dir.is_some() {
            return self.state_dir(home);
        }
        env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .map_or_else(
                || self.state_dir(home),
                |root| root.join("codex/providers/chatgpt"),
            )
    }
}

pub fn user_home() -> Result<PathBuf> {
    env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| ProviderError::internal("home_unset", "HOME environment variable is unset"))
}

pub fn ensure_private_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                ProviderError::internal(
                    "private_dir_failed",
                    format!("create parent {}: {e}", parent.display()),
                )
            })?;
        }
    }
    match DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => {
            return Err(ProviderError::internal(
                "private_dir_failed",
                format!("create {}: {e}", path.display()),
            ))
        }
    }
    let meta = fs::symlink_metadata(path).map_err(|e| {
        ProviderError::internal(
            "private_dir_failed",
            format!("stat {}: {e}", path.display()),
        )
    })?;
    if meta.file_type().is_symlink() || !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(ProviderError::internal(
            "private_dir_unsafe",
            format!("unsafe private directory: {}", path.display()),
        ));
    }
    if meta.permissions().mode() & 0o077 != 0 {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| {
            ProviderError::internal(
                "private_dir_chmod_failed",
                format!("chmod {}: {e}", path.display()),
            )
        })?;
    }
    Ok(())
}

pub fn random_hex(num_bytes: usize) -> Result<String> {
    let mut buf = vec![0u8; num_bytes];
    let mut urandom = File::open("/dev/urandom")
        .map_err(|e| ProviderError::internal("random_failed", e.to_string()))?;
    urandom
        .read_exact(&mut buf)
        .map_err(|e| ProviderError::internal("random_failed", e.to_string()))?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(num_bytes * 2);
    for b in buf {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    Ok(out)
}

/// Ensure a private `0600` Bearer token exists at `<state_dir>/control.token`.
pub fn load_or_create_control_token(state_dir: &Path) -> Result<(PathBuf, String)> {
    ensure_private_dir(state_dir)?;
    let token_path = state_dir.join("control.token");
    if let Ok(meta) = fs::symlink_metadata(&token_path) {
        if meta.file_type().is_symlink()
            || !meta.is_file()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.permissions().mode() & 0o077 != 0
        {
            return Err(ProviderError::internal(
                "control_token_unsafe",
                format!("unsafe control token file: {}", token_path.display()),
            ));
        }
        let existing = fs::read_to_string(&token_path)
            .map_err(|e| ProviderError::internal("control_token_read_failed", e.to_string()))?;
        let trimmed = existing.trim().to_owned();
        if trimmed.len() >= 32 {
            return Ok((token_path, trimmed));
        }
    }
    let token = random_hex(32)?;
    let tmp = state_dir.join(format!(
        ".control.token.tmp-{}-{}",
        std::process::id(),
        random_hex(4)?
    ));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)
        .map_err(|e| ProviderError::internal("control_token_write_failed", e.to_string()))?;
    f.write_all(format!("{token}\n").as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| ProviderError::internal("control_token_write_failed", e.to_string()))?;
    drop(f);
    fs::rename(&tmp, &token_path)
        .map_err(|e| ProviderError::internal("control_token_rename_failed", e.to_string()))?;
    Ok((token_path, token))
}

/// Constant-time byte slice comparison for Bearer token authentication.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
