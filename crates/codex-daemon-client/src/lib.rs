//! Minimal public JSON-RPC client for the official `codex app-server daemon`.
//!
//! Shared by `codex-ext` (`cdx`) and `mcp/termux-codex`.
//! Uses direct RFC 6455 WebSocket-over-UDS framing against the daemon's
//! `app-server-control/app-server-control.sock` with `/proc/self/fd/<dirfd>/...`
//! resolution for Termux `termux-uds-fd35` short socket paths.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_WS_FRAME_BYTES: usize = 16 * 1024 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const FD35_PREFIX: &str = "/proc/self/fd/35/";
static NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum DaemonError {
    Io(io::Error),
    SocketNotFound(PathBuf),
    Handshake(String),
    Protocol(String),
    Rpc { code: i64, message: String },
    Cli(String),
}

impl fmt::Display for DaemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "daemon I/O error: {err}"),
            Self::SocketNotFound(path) => {
                write!(
                    f,
                    "app-server daemon control socket not found: {}",
                    path.display()
                )
            }
            Self::Handshake(msg) => write!(f, "daemon WebSocket handshake failed: {msg}"),
            Self::Protocol(msg) => write!(f, "daemon protocol error: {msg}"),
            Self::Rpc { code, message } => write!(f, "daemon RPC error ({code}): {message}"),
            Self::Cli(msg) => write!(f, "daemon CLI error: {msg}"),
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<io::Error> for DaemonError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, DaemonError>;

/// Holds any open temporary directory FD required to keep `/proc/self/fd/<fd>/...` valid.
pub struct ResolvedSocket {
    _dir_fd: Option<OwnedFd>,
    pub connect_path: PathBuf,
}

/// Resolve `$CODEX_HOME/app-server-control/app-server-control.sock` to a connectable path.
///
/// Under Termux `termux-uds-fd35`, the control socket symlink points to
/// `/proc/self/fd/35/<uid>/<hash>` to stay under Linux's 108-byte `SUN_LEN` limit.
/// Rebinding through a fresh `O_RDONLY | O_DIRECTORY | O_CLOEXEC` descriptor on
/// `tmpdir` yields `/proc/self/fd/<dirfd>/<uid>/<hash>` without touching FD 35.
pub fn discover_control_socket(codex_home: &Path, tmpdir: &Path) -> Result<ResolvedSocket> {
    let sock_entry = codex_home
        .join("app-server-control")
        .join("app-server-control.sock");
    let meta = match fs::symlink_metadata(&sock_entry) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(DaemonError::SocketNotFound(sock_entry));
        }
        Err(err) => return Err(DaemonError::Io(err)),
    };

    if meta.file_type().is_symlink() {
        let target = fs::read_link(&sock_entry)?;
        if let Some(target_str) = target.to_str() {
            if let Some(rel) = target_str.strip_prefix(FD35_PREFIX) {
                if rel.is_empty()
                    || rel.starts_with('/')
                    || rel
                        .split('/')
                        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
                {
                    return Err(DaemonError::Protocol(format!(
                        "unsafe FD 35 socket symlink target: {target_str}"
                    )));
                }
                let dir_fd = open_dir_readonly(tmpdir)?;
                let connect_path =
                    PathBuf::from(format!("/proc/self/fd/{}/{rel}", dir_fd.as_raw_fd()));
                return Ok(ResolvedSocket {
                    _dir_fd: Some(dir_fd),
                    connect_path,
                });
            }
        }
        let resolved = if target.is_absolute() {
            target
        } else {
            sock_entry
                .parent()
                .unwrap_or_else(|| Path::new("/"))
                .join(target)
        };
        return Ok(ResolvedSocket {
            _dir_fd: None,
            connect_path: resolved,
        });
    }

    Ok(ResolvedSocket {
        _dir_fd: None,
        connect_path: sock_entry,
    })
}

fn open_dir_readonly(dir: &Path) -> io::Result<OwnedFd> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "directory path contains NUL"))?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadSummary {
    pub id: String,
    pub status_type: String,
    pub active_flags: Vec<String>,
    pub cwd: Option<String>,
    pub name: Option<String>,
    pub model_provider: Option<String>,
    pub updated_at: Option<i64>,
}

impl ThreadSummary {
    pub fn is_active(&self) -> bool {
        self.status_type == "active"
    }

    fn from_thread_value(val: &Value) -> Result<Self> {
        let id = val
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| DaemonError::Protocol("thread object missing string 'id'".to_owned()))?
            .to_owned();
        let status_obj = val.get("status");
        let status_type = status_obj
            .and_then(|s| s.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let active_flags = status_obj
            .and_then(|s| s.get("activeFlags"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let cwd = val
            .get("cwd")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let name = val
            .get("name")
            .or_else(|| val.get("threadName"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let model_provider = val
            .get("modelProvider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let updated_at = val.get("updatedAt").and_then(Value::as_i64);
        Ok(Self {
            id,
            status_type,
            active_flags,
            cwd,
            name,
            model_provider,
            updated_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadListPage {
    pub threads: Vec<ThreadSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonVersionInfo {
    pub status: String,
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub socket_path: Option<String>,
    #[serde(default)]
    pub cli_version: Option<String>,
    #[serde(default)]
    pub app_server_version: Option<String>,
}

pub struct DaemonClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u64,
    notifications: Vec<Value>,
}

impl DaemonClient {
    /// Discover the daemon's control socket, perform WebSocket handshake, and complete
    /// the JSON-RPC `initialize` + `initialized` sequence.
    pub fn connect(
        codex_home: &Path,
        tmpdir: &Path,
        timeout: Duration,
        client_name: &str,
        client_version: &str,
    ) -> Result<Self> {
        let resolved = discover_control_socket(codex_home, tmpdir)?;
        Self::connect_socket_path(&resolved.connect_path, timeout, client_name, client_version)
    }

    /// Connect directly to a UDS path (also used by tests with local mock servers).
    pub fn connect_socket_path(
        sock_path: &Path,
        timeout: Duration,
        client_name: &str,
        client_version: &str,
    ) -> Result<Self> {
        let stream = UnixStream::connect(sock_path).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound
                || err.kind() == io::ErrorKind::ConnectionRefused
            {
                DaemonError::SocketNotFound(sock_path.to_path_buf())
            } else {
                DaemonError::Io(err)
            }
        })?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;

        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);
        let mut client = Self {
            reader,
            writer,
            next_id: 1,
            notifications: Vec::new(),
        };
        client.ws_handshake()?;
        client.initialize(client_name, client_version)?;
        Ok(client)
    }

    fn ws_handshake(&mut self) -> Result<()> {
        let key_bytes = make_nonce_16();
        let ws_key = base64_encode(&key_bytes);
        let req = format!(
            "GET / HTTP/1.1\r\n\
             Host: localhost\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {ws_key}\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        );
        self.writer.write_all(req.as_bytes())?;
        self.writer.flush()?;

        let mut total = 0usize;
        let mut status_line = String::new();
        let n = self.reader.read_line(&mut status_line)?;
        if n == 0 {
            return Err(DaemonError::Handshake(
                "unexpected EOF during upgrade".to_owned(),
            ));
        }
        total += n;
        if !status_line.starts_with("HTTP/1.1 101") {
            return Err(DaemonError::Handshake(format!(
                "expected HTTP/1.1 101, got: {}",
                status_line.trim()
            )));
        }

        let mut accept_header: Option<String> = None;
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line)?;
            if n == 0 {
                return Err(DaemonError::Handshake(
                    "truncated HTTP upgrade headers".to_owned(),
                ));
            }
            total += n;
            if total > MAX_HTTP_HEADER_BYTES {
                return Err(DaemonError::Handshake(
                    "HTTP upgrade headers exceed limit".to_owned(),
                ));
            }
            let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
            if trimmed.is_empty() {
                break;
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                if k.trim().eq_ignore_ascii_case("sec-websocket-accept") {
                    accept_header = Some(v.trim().to_owned());
                }
            }
        }

        let expected_accept = compute_ws_accept(&ws_key);
        match accept_header.as_deref() {
            Some(got) if got == expected_accept => Ok(()),
            Some(got) => Err(DaemonError::Handshake(format!(
                "Sec-WebSocket-Accept mismatch (expected {expected_accept}, got {got})"
            ))),
            None => Err(DaemonError::Handshake(
                "missing Sec-WebSocket-Accept header".to_owned(),
            )),
        }
    }

    fn initialize(&mut self, client_name: &str, client_version: &str) -> Result<()> {
        let _ = self.call(
            "initialize",
            json!({
                "clientInfo": {
                    "name": client_name,
                    "version": client_version
                }
            }),
        )?;
        self.notify("initialized", json!({}))?;
        Ok(())
    }

    pub fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let payload = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
        .map_err(|err| DaemonError::Protocol(err.to_string()))?;
        self.write_ws_frame(0x1, payload.as_bytes())
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))
        .map_err(|err| DaemonError::Protocol(err.to_string()))?;
        self.write_ws_frame(0x1, payload.as_bytes())?;

        loop {
            let text = self.read_ws_text()?;
            let msg: Value = serde_json::from_str(&text)
                .map_err(|err| DaemonError::Protocol(format!("invalid JSON-RPC frame: {err}")))?;
            if let Some(msg_id) = msg.get("id").and_then(Value::as_u64) {
                if msg_id == id {
                    if let Some(err) = msg.get("error") {
                        let code = err.get("code").and_then(Value::as_i64).unwrap_or(-1);
                        let message = err
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown RPC error")
                            .to_owned();
                        return Err(DaemonError::Rpc { code, message });
                    }
                    return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
                }
            }
            if self.notifications.len() < 256 {
                self.notifications.push(msg);
            }
        }
    }

    pub fn drain_notifications(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.notifications)
    }

    pub fn loaded_threads(&mut self) -> Result<Vec<String>> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..64 {
            let params = match &cursor {
                Some(c) => json!({ "cursor": c }),
                None => json!({}),
            };
            let res = self.call("thread/loaded/list", params)?;
            if let Some(arr) = res.get("data").and_then(Value::as_array) {
                for item in arr {
                    if let Some(id) = item.as_str() {
                        all.push(id.to_owned());
                    }
                }
            }
            cursor = res
                .get("nextCursor")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        Ok(all)
    }

    pub fn read_thread_raw(&mut self, thread_id: &str, include_turns: bool) -> Result<Value> {
        self.call(
            "thread/read",
            json!({
                "threadId": thread_id,
                "includeTurns": include_turns
            }),
        )
    }

    pub fn read_thread(&mut self, thread_id: &str, include_turns: bool) -> Result<ThreadSummary> {
        let res = self.read_thread_raw(thread_id, include_turns)?;
        let thread = res.get("thread").unwrap_or(&res);
        ThreadSummary::from_thread_value(thread)
    }

    pub fn list_threads(&mut self, limit: u32, cursor: Option<&str>) -> Result<ThreadListPage> {
        let mut params = json!({ "limit": limit });
        if let Some(c) = cursor {
            params["cursor"] = Value::String(c.to_owned());
        }
        let res = self.call("thread/list", params)?;
        let mut threads = Vec::new();
        if let Some(arr) = res.get("data").and_then(Value::as_array) {
            for item in arr {
                if let Ok(summary) = ThreadSummary::from_thread_value(item) {
                    threads.push(summary);
                }
            }
        }
        let next_cursor = res
            .get("nextCursor")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        Ok(ThreadListPage {
            threads,
            next_cursor,
        })
    }

    /// Inspect all currently loaded threads in the daemon and return those with an active turn.
    pub fn inspect_busy_threads(
        &mut self,
        exclude_thread: Option<&str>,
    ) -> Result<Vec<ThreadSummary>> {
        let ids = self.loaded_threads()?;
        let mut busy = Vec::new();
        for id in ids {
            if exclude_thread.is_some_and(|ex| ex == id) {
                continue;
            }
            let summary = self.read_thread(&id, false)?;
            if summary.is_active() {
                busy.push(summary);
            }
        }
        Ok(busy)
    }

    pub fn list_model_providers(&mut self) -> Result<Value> {
        self.call("modelProvider/list", json!({}))
    }

    pub fn close(&mut self) {
        let _ = self.write_ws_frame(0x8, &[]);
    }

    fn write_ws_frame(&mut self, opcode: u8, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_WS_FRAME_BYTES {
            return Err(DaemonError::Protocol(
                "outgoing WebSocket frame exceeds 16 MiB".to_owned(),
            ));
        }
        let mask = make_mask_4();
        let mut hdr = Vec::with_capacity(14);
        hdr.push(0x80 | (opcode & 0x0f));
        let len = payload.len();
        if len < 126 {
            hdr.push(0x80 | (len as u8));
        } else if len <= 65535 {
            hdr.push(0x80 | 126);
            hdr.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            hdr.push(0x80 | 127);
            hdr.extend_from_slice(&(len as u64).to_be_bytes());
        }
        hdr.extend_from_slice(&mask);
        let mut masked = Vec::with_capacity(len);
        for (i, &b) in payload.iter().enumerate() {
            masked.push(b ^ mask[i & 3]);
        }
        self.writer.write_all(&hdr)?;
        self.writer.write_all(&masked)?;
        self.writer.flush()?;
        Ok(())
    }

    fn read_ws_text(&mut self) -> Result<String> {
        let mut message = Vec::new();
        let mut in_continuation = false;
        loop {
            let mut hdr = [0u8; 2];
            self.reader.read_exact(&mut hdr)?;
            let fin = (hdr[0] & 0x80) != 0;
            let opcode = hdr[0] & 0x0f;
            let masked = (hdr[1] & 0x80) != 0;
            let mut len = (hdr[1] & 0x7f) as u64;
            if len == 126 {
                let mut ext = [0u8; 2];
                self.reader.read_exact(&mut ext)?;
                len = u16::from_be_bytes(ext) as u64;
            } else if len == 127 {
                let mut ext = [0u8; 8];
                self.reader.read_exact(&mut ext)?;
                len = u64::from_be_bytes(ext);
            }
            if len as usize > MAX_WS_FRAME_BYTES {
                return Err(DaemonError::Protocol(format!(
                    "incoming WebSocket frame exceeds limit ({len} bytes)"
                )));
            }
            let mask_key = if masked {
                let mut m = [0u8; 4];
                self.reader.read_exact(&mut m)?;
                Some(m)
            } else {
                None
            };
            let mut payload = vec![0u8; len as usize];
            self.reader.read_exact(&mut payload)?;
            if let Some(m) = mask_key {
                for (i, b) in payload.iter_mut().enumerate() {
                    *b ^= m[i & 3];
                }
            }

            match opcode {
                0x9 => {
                    // Ping -> reply Pong
                    self.write_ws_frame(0xA, &payload)?;
                }
                0xA => {
                    // Pong -> ignore
                }
                0x8 => {
                    return Err(DaemonError::Protocol(
                        "daemon closed WebSocket connection".to_owned(),
                    ));
                }
                0x1 => {
                    if in_continuation {
                        return Err(DaemonError::Protocol(
                            "unexpected text frame during continuation".to_owned(),
                        ));
                    }
                    message.extend_from_slice(&payload);
                    if fin {
                        return String::from_utf8(message).map_err(|err| {
                            DaemonError::Protocol(format!("non-UTF-8 text frame: {err}"))
                        });
                    }
                    in_continuation = true;
                }
                0x0 => {
                    if !in_continuation {
                        return Err(DaemonError::Protocol(
                            "unexpected continuation frame".to_owned(),
                        ));
                    }
                    if message.len() + payload.len() > MAX_WS_FRAME_BYTES {
                        return Err(DaemonError::Protocol(
                            "fragmented message exceeds 16 MiB".to_owned(),
                        ));
                    }
                    message.extend_from_slice(&payload);
                    if fin {
                        return String::from_utf8(message).map_err(|err| {
                            DaemonError::Protocol(format!("non-UTF-8 text frame: {err}"))
                        });
                    }
                }
                other => {
                    return Err(DaemonError::Protocol(format!(
                        "unsupported WebSocket opcode 0x{other:x}"
                    )));
                }
            }
        }
    }
}

/// Non-blocking check whether `$SHARED_HOME/thread-writer-locks/<thread_id>.lock` is actively locked via `flock`.
pub fn is_thread_writer_lock_held(shared_home: &Path, thread_id: &str) -> Result<bool> {
    if thread_id.is_empty()
        || thread_id.contains('/')
        || thread_id.contains('\0')
        || thread_id == "."
        || thread_id == ".."
    {
        return Err(DaemonError::Protocol(format!(
            "invalid thread id: {thread_id}"
        )));
    }
    let lock_path = shared_home
        .join("thread-writer-locks")
        .join(format!("{thread_id}.lock"));
    let file = match OpenOptions::new().read(true).open(&lock_path) {
        Ok(f) => f,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(DaemonError::Io(err)),
    };
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        let _ = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        Ok(false)
    } else {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EWOULDBLOCK) || err.raw_os_error() == Some(libc::EAGAIN)
        {
            Ok(true)
        } else {
            Err(DaemonError::Io(err))
        }
    }
}

/// Poll until `<thread_id>.lock` is no longer held by a daemon writer, returning `true` if released.
pub fn wait_writer_lock_released(
    shared_home: &Path,
    thread_id: &str,
    timeout: Duration,
) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        if !is_thread_writer_lock_held(shared_home, thread_id)? {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn configure_daemon_cli_env(cmd: &mut Command, codex_home: &Path, shared_home: Option<&Path>) {
    cmd.env("CODEX_HOME", codex_home);
    match shared_home {
        Some(shared) => {
            cmd.env("CODEX_SHARED_HOME", shared);
            cmd.env("CODEX_SQLITE_HOME", shared);
        }
        None => {
            cmd.env_remove("CODEX_SHARED_HOME");
            cmd.env_remove("CODEX_SQLITE_HOME");
        }
    }
}

/// Query official `codex app-server daemon version` JSON for a specific `$CODEX_HOME`.
pub fn daemon_version_via_cli(
    codex_bin: &Path,
    codex_home: &Path,
    shared_home: Option<&Path>,
) -> Result<DaemonVersionInfo> {
    let mut cmd = Command::new(codex_bin);
    cmd.args(["app-server", "daemon", "version"]);
    configure_daemon_cli_env(&mut cmd, codex_home, shared_home);
    let out = cmd.output()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or(stdout.trim());
    if !out.status.success() && line.is_empty() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(DaemonError::Cli(format!(
            "codex app-server daemon version exited {}: {}",
            out.status,
            stderr.trim()
        )));
    }
    serde_json::from_str(line).map_err(|err| {
        DaemonError::Cli(format!(
            "failed to parse daemon version JSON ({err}): {line}"
        ))
    })
}

/// Gracefully stop the managed daemon for a specific `$CODEX_HOME` via official upstream CLI.
pub fn daemon_stop_via_cli(
    codex_bin: &Path,
    codex_home: &Path,
    shared_home: Option<&Path>,
) -> Result<()> {
    let mut cmd = Command::new(codex_bin);
    cmd.args(["app-server", "daemon", "stop"]);
    configure_daemon_cli_env(&mut cmd, codex_home, shared_home);
    let out = cmd.output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(DaemonError::Cli(format!(
            "codex app-server daemon stop exited {}: {}",
            out.status,
            stderr.trim()
        )));
    }
    Ok(())
}

fn make_nonce_16() -> [u8; 16] {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = File::open("/dev/urandom") {
        if f.read_exact(&mut buf).is_ok() {
            return buf;
        }
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let c = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id() as u64;
    buf[..8].copy_from_slice(&(nanos ^ (pid << 16)).to_le_bytes());
    buf[8..].copy_from_slice(&c.wrapping_mul(0x9e37_79b9_7f4a_7c15).to_le_bytes());
    buf
}

fn make_mask_4() -> [u8; 4] {
    let n = make_nonce_16();
    [n[0], n[1], n[2], n[3]]
}

fn compute_ws_accept(ws_key: &str) -> String {
    let mut input = Vec::with_capacity(ws_key.len() + WS_GUID.len());
    input.extend_from_slice(ws_key.as_bytes());
    input.extend_from_slice(WS_GUID);
    base64_encode(&sha1(&input))
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x6745_2301;
    let mut h1: u32 = 0xEFCD_AB89;
    let mut h2: u32 = 0x98BA_DCFE;
    let mut h3: u32 = 0x1032_5476;
    let mut h4: u32 = 0xC3D2_E1F0;

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 80];
    for chunk in msg.as_chunks::<64>().0 {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDCu32),
                _ => (b ^ c ^ d, 0xCA62_C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn server_read_ws_text(reader: &mut BufReader<UnixStream>) -> (u8, Vec<u8>) {
        let mut hdr = [0u8; 2];
        reader.read_exact(&mut hdr).unwrap();
        let opcode = hdr[0] & 0x0f;
        let masked = (hdr[1] & 0x80) != 0;
        let mut len = (hdr[1] & 0x7f) as usize;
        if len == 126 {
            let mut ext = [0u8; 2];
            reader.read_exact(&mut ext).unwrap();
            len = u16::from_be_bytes(ext) as usize;
        }
        let mask = if masked {
            let mut m = [0u8; 4];
            reader.read_exact(&mut m).unwrap();
            Some(m)
        } else {
            None
        };
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).unwrap();
        if let Some(m) = mask {
            for (i, b) in payload.iter_mut().enumerate() {
                *b ^= m[i & 3];
            }
        }
        (opcode, payload)
    }

    fn server_write_ws(writer: &mut UnixStream, fin: bool, opcode: u8, payload: &[u8]) {
        let mut hdr = Vec::new();
        hdr.push((if fin { 0x80 } else { 0 }) | (opcode & 0x0f));
        if payload.len() < 126 {
            hdr.push(payload.len() as u8);
        } else {
            hdr.push(126);
            hdr.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        writer.write_all(&hdr).unwrap();
        writer.write_all(payload).unwrap();
        writer.flush().unwrap();
    }

    #[test]
    fn rfc6455_accept_vector_matches() {
        assert_eq!(
            compute_ws_accept("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn mock_ws_daemon_rpc_notifications_ping_fragmentation_and_busy_detection() {
        let root = temp_dir("codex-dc-ws");
        let tmp_sub = root.join("tmp");
        let home = root.join("home");
        fs::create_dir_all(tmp_sub.join("10593")).unwrap();
        fs::create_dir_all(home.join("app-server-control")).unwrap();
        let real_sock = tmp_sub.join("10593/mock.sock");
        let listener = UnixListener::bind(&real_sock).unwrap();

        // Simulate Termux FD 35 symlink
        std::os::unix::fs::symlink(
            "/proc/self/fd/35/10593/mock.sock",
            home.join("app-server-control/app-server-control.sock"),
        )
        .unwrap();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            let mut key = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        key = v.trim().to_owned();
                    }
                }
            }
            let accept = compute_ws_accept(&key);
            let resp = format!(
                "HTTP/1.1 101 Switching Protocols\r\n\
                 Upgrade: websocket\r\n\
                 Connection: Upgrade\r\n\
                 Sec-WebSocket-Accept: {accept}\r\n\r\n"
            );
            writer.write_all(resp.as_bytes()).unwrap();
            writer.flush().unwrap();

            loop {
                let (op, data) = server_read_ws_text(&mut reader);
                if op == 0x8 {
                    break;
                }
                if op == 0xA {
                    continue;
                }
                let msg: Value = serde_json::from_slice(&data).unwrap();
                let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
                let id = msg.get("id").and_then(Value::as_u64);
                match method {
                    "initialize" => {
                        let out =
                            json!({"id": id, "result": {"userAgent": "mock/0.171.0"}}).to_string();
                        server_write_ws(&mut writer, true, 0x1, out.as_bytes());
                    }
                    "initialized" => {}
                    "thread/loaded/list" => {
                        // Send ping + unsolicited notification + fragmented response!
                        server_write_ws(&mut writer, true, 0x9, b"hi");
                        let notif = json!({"method": "remoteControl/status/changed", "params": {}})
                            .to_string();
                        server_write_ws(&mut writer, true, 0x1, notif.as_bytes());
                        let out = json!({"id": id, "result": {"data": ["t-idle", "t-busy"], "nextCursor": null}}).to_string();
                        let mid = out.len() / 2;
                        server_write_ws(&mut writer, false, 0x1, &out.as_bytes()[..mid]);
                        server_write_ws(&mut writer, true, 0x0, &out.as_bytes()[mid..]);
                    }
                    "thread/read" => {
                        let tid = msg["params"]["threadId"].as_str().unwrap();
                        let st = if tid == "t-busy" { "active" } else { "idle" };
                        let out = json!({
                            "id": id,
                            "result": {
                                "thread": {
                                    "id": tid,
                                    "status": {"type": st, "activeFlags": []},
                                    "cwd": "/tmp/work",
                                    "modelProvider": "openai"
                                }
                            }
                        })
                        .to_string();
                        server_write_ws(&mut writer, true, 0x1, out.as_bytes());
                    }
                    _ => {
                        let out =
                            json!({"id": id, "error": {"code": -32601, "message": "not found"}})
                                .to_string();
                        server_write_ws(&mut writer, true, 0x1, out.as_bytes());
                    }
                }
            }
        });

        let mut client =
            DaemonClient::connect(&home, &tmp_sub, Duration::from_secs(3), "test", "0.1").unwrap();
        let busy = client.inspect_busy_threads(None).unwrap();
        assert_eq!(busy.len(), 1);
        assert_eq!(busy[0].id, "t-busy");
        assert!(busy[0].is_active());
        assert_eq!(client.drain_notifications().len(), 1);

        let err = client.call("unknown/method", json!({})).unwrap_err();
        assert!(matches!(err, DaemonError::Rpc { code: -32601, .. }));

        client.close();
        server.join().unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn writer_lock_probe_detects_held_and_released_flock() {
        let root = temp_dir("codex-dc-lock");
        let lock_dir = root.join("thread-writer-locks");
        fs::create_dir_all(&lock_dir).unwrap();
        let tid = "01999999-0000-7000-8000-000000000001";
        assert!(!is_thread_writer_lock_held(&root, tid).unwrap());

        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(lock_dir.join(format!("{tid}.lock")))
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        // Same process holds an open flock on another FD -> distinct FD probe sees EWOULDBLOCK!
        assert!(is_thread_writer_lock_held(&root, tid).unwrap());
        drop(lock_file);
        assert!(!is_thread_writer_lock_held(&root, tid).unwrap());
        let _ = fs::remove_dir_all(&root);
    }
}
