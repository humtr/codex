//! Shared lightweight MCP server foundation for independent MCP workers:
//! `@TermuxShell`, `@TermuxCodex`, and `@TermuxChatGPT`.
//!
//! Provides:
//! - Uniform immutable MCP tool annotations (`readOnlyHint: true`, `destructiveHint: false`,
//!   `openWorldHint: false`, plus honest per-tool `idempotentHint`).
//! - Disjoint `*_catalog` / `*_call` extension registry with self-correcting schema errors.
//! - Atomic `O_NOFOLLOW` + `fsync` `call_id` transport deduplication receipt store (`0700`/`0600`).
//! - Pure-Rust FIPS 180-4 SHA-256 and Base64 (zero extra crypto crates).
//! - Bounded newline-delimited MCP stdio JSON-RPC 2.0 server loop.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
pub const MAX_CALL_ID_LEN: usize = 160;
/// 512 KiB response payload + 8 KiB headroom for receipt envelope metadata.
pub const MAX_RECEIPT_BYTES: u64 = 512 * 1024 + 8192;
pub const MAX_STDIO_LINE_BYTES: usize = 2 * 1024 * 1024;
static RECEIPT_SERIAL: AtomicU64 = AtomicU64::new(1);

/// Non-negotiable common annotation policy shared by all independent MCP servers.
pub fn tool_annotations(idempotent: bool) -> Value {
    json!({
        "readOnlyHint": true,
        "destructiveHint": false,
        "openWorldHint": false,
        "idempotentHint": idempotent
    })
}

/// Static MCP tool descriptor with enforced uniform annotations.
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub idempotent: bool,
}

impl ToolDef {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            idempotent,
        }
    }

    pub fn to_mcp_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
            "annotations": tool_annotations(self.idempotent)
        })
    }
}

/// One allowlisted action exposed ONLY through a disjoint `*_catalog` / `*_call` slot.
#[derive(Debug, Clone)]
pub struct ExtensionSpec {
    pub action: String,
    pub description: String,
    pub input_schema: Value,
    pub idempotent: bool,
}

impl ExtensionSpec {
    pub fn new(
        action: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        idempotent: bool,
    ) -> Self {
        Self {
            action: action.into(),
            description: description.into(),
            input_schema,
            idempotent,
        }
    }

    pub fn to_catalog_json(&self) -> Value {
        json!({
            "action": self.action,
            "description": self.description,
            "inputSchema": self.input_schema,
            "idempotent": self.idempotent
        })
    }
}

/// Disjoint allowlist registry for `*_catalog` / `*_call` extension actions.
/// Enforces at construction time that extension action names NEVER overlap static tool names.
#[derive(Debug, Clone)]
pub struct ExtensionRegistry {
    static_tools: HashSet<String>,
    actions: BTreeMap<String, ExtensionSpec>,
}

impl ExtensionRegistry {
    pub fn new<'a>(static_tool_names: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            static_tools: static_tool_names
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            actions: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, spec: ExtensionSpec) -> Result<(), String> {
        if self.static_tools.contains(&spec.action) {
            return Err(format!(
                "disjointness violation: extension action '{}' duplicates static tool",
                spec.action
            ));
        }
        if self.actions.contains_key(&spec.action) {
            return Err(format!("duplicate extension action '{}'", spec.action));
        }
        self.actions.insert(spec.action.clone(), spec);
        Ok(())
    }

    pub fn catalog_json(&self) -> Value {
        let items: Vec<Value> = self
            .actions
            .values()
            .map(ExtensionSpec::to_catalog_json)
            .collect();
        json!({
            "actions": items,
            "count": items.len()
        })
    }

    pub fn get(&self, action: &str) -> Option<&ExtensionSpec> {
        self.actions.get(action)
    }

    /// Build a self-correcting error payload including the exact expected schema
    /// (or available actions when the action name itself is unknown).
    pub fn validation_error(&self, action: &str, reason: &str) -> Value {
        if let Some(spec) = self.actions.get(action) {
            json!({
                "state": "rejected",
                "error": reason,
                "action": action,
                "expected_schema": spec.input_schema
            })
        } else {
            let available: Vec<&str> = self.actions.keys().map(String::as_str).collect();
            json!({
                "state": "rejected",
                "error": reason,
                "action": action,
                "available_actions": available
            })
        }
    }
}

/// Validate a caller-supplied `call_id` (non-empty, <= 160 bytes, no NUL).
pub fn validate_call_id(args: &Value) -> Result<&str, &'static str> {
    match args.get("call_id").and_then(Value::as_str) {
        Some(v) if !v.is_empty() && v.len() <= MAX_CALL_ID_LEN && !v.contains('\0') => Ok(v),
        _ => Err("call_id_invalid"),
    }
}

/// Outcome of claiming an atomic `call_id` execution intent before spawning an effect.
#[derive(Debug, Clone)]
pub enum ReceiptClaim {
    /// New execution intent durably committed; caller owns running the effect and calling `commit`.
    Acquired(PathBuf),
    /// Existing settled or unsettled/conflict receipt found; caller MUST NOT execute again.
    Existing(Value),
    /// Receipt directory or intent write failed before any effect could start.
    Rejected(&'static str),
}

/// Bounded private (`0700`/`0600`, `O_NOFOLLOW`) `call_id` transport deduplication journal.
#[derive(Debug, Clone)]
pub struct ReceiptStore {
    root: PathBuf,
    schema: String,
    conflict_error: String,
}

impl ReceiptStore {
    pub fn open(
        root: PathBuf,
        schema: impl Into<String>,
        conflict_error: impl Into<String>,
    ) -> Result<Self, &'static str> {
        if let Some(parent) = root.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent).map_err(|_| "receipt_directory_unavailable")?;
            }
        }
        match DirBuilder::new().mode(0o700).create(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("receipt_directory_unavailable"),
        }
        let meta = fs::symlink_metadata(&root).map_err(|_| "receipt_directory_unavailable")?;
        if !meta.file_type().is_dir()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.permissions().mode() & 0o077 != 0
        {
            return Err("receipt_directory_unsafe");
        }
        Ok(Self {
            root,
            schema: schema.into(),
            conflict_error: conflict_error.into(),
        })
    }

    pub fn receipt_path(&self, call_id: &str) -> PathBuf {
        self.root
            .join(format!("{}.json", sha256_hex(call_id.as_bytes())))
    }

    /// Inspect an existing receipt without creating a new intent.
    pub fn inspect(&self, call_id: &str, expected_fingerprints: &[&str]) -> Value {
        let path = self.receipt_path(call_id);
        match self.read_record(&path) {
            Ok(Some(record)) => {
                if record["schema"] != self.schema
                    || !matches!(record["state"].as_str(), Some("started" | "settled"))
                    || !record["fingerprint"]
                        .as_str()
                        .is_some_and(|v| !v.is_empty())
                    || (record["state"] == "settled" && !record["outcome"].is_object())
                {
                    return json!({
                        "state": "delivery_unknown",
                        "executed": "unknown",
                        "error": "receipt_invalid"
                    });
                }
                if !expected_fingerprints.is_empty() {
                    let stored_fp = record["fingerprint"].as_str().unwrap_or("");
                    if !expected_fingerprints.contains(&stored_fp) {
                        return json!({
                            "state": "rejected",
                            "executed": false,
                            "error": self.conflict_error
                        });
                    }
                }
                if record["state"] == "settled" {
                    let mut result = record["outcome"].clone();
                    result["replayed_from_receipt"] = json!(true);
                    result
                } else {
                    json!({
                        "state": "delivery_unknown",
                        "executed": "unknown",
                        "error": "original_effect_unsettled_check_before_new_call"
                    })
                }
            }
            Ok(None) => json!({"state": "not_found", "executed": false}),
            Err(error) => json!({
                "state": "delivery_unknown",
                "executed": "unknown",
                "error": error
            }),
        }
    }

    /// Atomically claim a `call_id` intent (`O_CREAT | O_EXCL | O_NOFOLLOW`, `0600`, `fsync`).
    /// Never executes twice on concurrent identical retries, and retries briefly if a
    /// concurrent pre-dispatch failure released the intent between check and inspect.
    pub fn claim(
        &self,
        call_id: &str,
        primary_fingerprint: &str,
        accepted_fingerprints: &[&str],
    ) -> ReceiptClaim {
        let path = self.receipt_path(call_id);
        for _ in 0..3 {
            if fs::symlink_metadata(&path).is_ok() {
                let inspected = self.inspect(call_id, accepted_fingerprints);
                if inspected["state"] == "not_found" {
                    continue;
                }
                return ReceiptClaim::Existing(inspected);
            }
            match self.write_intent(&path, primary_fingerprint) {
                Ok(true) => return ReceiptClaim::Acquired(path),
                Ok(false) => {
                    let inspected = self.inspect(call_id, accepted_fingerprints);
                    if inspected["state"] == "not_found" {
                        continue;
                    }
                    return ReceiptClaim::Existing(inspected);
                }
                Err(err) => return ReceiptClaim::Rejected(err),
            }
        }
        ReceiptClaim::Rejected("receipt_race")
    }

    /// Release a freshly claimed `started` receipt after a guaranteed pre-dispatch failure
    /// (zero bytes sent / no side effect executed) and durably fsync the parent directory.
    pub fn release_intent(&self, path: &Path) -> Result<(), &'static str> {
        match fs::remove_file(path) {
            Ok(()) => sync_parent(path),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("receipt_release_failed"),
        }
    }

    pub fn write_intent(&self, path: &Path, fingerprint: &str) -> Result<bool, &'static str> {
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
        {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
            Err(_) => return Err("receipt_create_failed"),
        };
        let bytes = json!({
            "schema": self.schema,
            "fingerprint": fingerprint,
            "state": "started",
            "executed": "unknown"
        })
        .to_string();
        file.write_all(bytes.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "receipt_intent_write_failed")?;
        sync_parent(path)?;
        Ok(true)
    }

    pub fn commit(
        &self,
        path: &Path,
        fingerprint: &str,
        outcome: &Value,
    ) -> Result<(), &'static str> {
        let nonce = RECEIPT_SERIAL.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
        let complete = json!({
            "schema": self.schema,
            "fingerprint": fingerprint,
            "state": "settled",
            "outcome": outcome
        });
        let encoded = complete.to_string();
        if encoded.len() as u64 > MAX_RECEIPT_BYTES {
            return Err("receipt_too_large");
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)
            .map_err(|_| "receipt_complete_create_failed")?;
        if file
            .write_all(encoded.as_bytes())
            .and_then(|_| file.sync_all())
            .is_err()
        {
            let _ = fs::remove_file(&tmp);
            return Err("receipt_complete_write_failed");
        }
        drop(file);
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::remove_file(&tmp);
            return Err("receipt_complete_rename_failed");
        }
        sync_parent(path)
    }

    /// Best-effort bounded retention: remove oldest *settled* receipts when count exceeds `max_settled`.
    /// Never removes `started` (unsettled) or unreadable/corrupt receipts so uncertainty is preserved.
    pub fn prune_settled(&self, max_settled: usize) {
        if max_settled == 0 {
            return;
        }
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        let mut settled: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(Some(record)) = self.read_record(&path) {
                if record["schema"] == self.schema && record["state"] == "settled" {
                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    settled.push((mtime, path));
                }
            }
        }
        if settled.len() <= max_settled {
            return;
        }
        settled.sort_by_key(|(mtime, _)| *mtime);
        let to_remove = settled.len() - max_settled;
        for (_, path) in settled.into_iter().take(to_remove) {
            let _ = fs::remove_file(path);
        }
    }

    fn read_record(&self, path: &Path) -> Result<Option<Value>, &'static str> {
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
        {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("receipt_unavailable"),
        };
        let meta = file.metadata().map_err(|_| "receipt_unavailable")?;
        if !meta.is_file()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.permissions().mode() & 0o077 != 0
            || meta.len() > MAX_RECEIPT_BYTES
        {
            return Err("receipt_unsafe");
        }
        let mut bytes = Vec::new();
        file.take(MAX_RECEIPT_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|_| "receipt_unavailable")?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "receipt_incomplete")
    }
}

fn sync_parent(path: &Path) -> Result<(), &'static str> {
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path.parent().ok_or("receipt_directory_unavailable")?)
        .map_err(|_| "receipt_directory_unavailable")?;
    directory
        .sync_all()
        .map_err(|_| "receipt_directory_sync_failed")
}

/// Format an MCP `tools/call` JSON-RPC result envelope wrapping a JSON payload as text content.
pub fn tool_result_response(id: Value, payload: &Value, is_error: bool) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{"type": "text", "text": payload.to_string()}],
            "isError": is_error
        }
    })
}

pub fn jsonrpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// Handler trait implemented by each independent MCP server.
pub trait McpHandler {
    fn server_name(&self) -> &str;
    fn server_version(&self) -> &str;
    fn tools(&self) -> Vec<ToolDef>;
    fn call_tool(&mut self, name: &str, arguments: &Value) -> (Value, bool);
}

/// Process a single parsed MCP JSON-RPC message. Returns `None` for notifications.
pub fn handle_message<H: McpHandler>(handler: &mut H, msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(Value::as_str);
    // JSON-RPC notifications have no "id".
    let id = msg.get("id")?.clone();

    let Some(method) = method else {
        return Some(jsonrpc_error(id, -32600, "invalid request: missing method"));
    };

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": handler.server_name(),
                    "version": handler.server_version()
                }
            }
        })),
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        })),
        "tools/list" => {
            let tools: Vec<Value> = handler.tools().iter().map(ToolDef::to_mcp_json).collect();
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tools }
            }))
        }
        "tools/call" => {
            let params = msg.get("params").and_then(Value::as_object);
            let Some(params) = params else {
                return Some(jsonrpc_error(
                    id,
                    -32602,
                    "tools/call requires params object",
                ));
            };
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Some(jsonrpc_error(
                    id,
                    -32602,
                    "tools/call requires string 'name'",
                ));
            };
            let default_args = Value::Object(Map::new());
            let args = match params.get("arguments") {
                None | Some(Value::Null) => &default_args,
                Some(v) => v,
            };
            let (payload, is_error) = handler.call_tool(name, args);
            Some(tool_result_response(id, &payload, is_error))
        }
        _ => Some(jsonrpc_error(
            id,
            -32601,
            &format!("method not found: {method}"),
        )),
    }
}

/// Run a bounded newline-delimited MCP JSON-RPC 2.0 loop over arbitrary reader/writer.
pub fn serve_io<H: McpHandler, R: Read, W: Write>(
    handler: &mut H,
    reader: R,
    mut writer: W,
) -> io::Result<()> {
    let mut buf_reader = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = read_bounded_line(&mut buf_reader, &mut line, MAX_STDIO_LINE_BYTES)?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = match std::str::from_utf8(&line) {
            Ok(s) => s.trim(),
            Err(_) => {
                let err = jsonrpc_error(Value::Null, -32700, "parse error: invalid UTF-8");
                writeln!(writer, "{err}")?;
                writer.flush()?;
                continue;
            }
        };
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(trimmed) {
            Ok(msg) => handle_message(handler, &msg),
            Err(_) => Some(jsonrpc_error(
                Value::Null,
                -32700,
                "parse error: malformed JSON",
            )),
        };
        if let Some(resp) = response {
            writeln!(writer, "{resp}")?;
            writer.flush()?;
        }
    }
}

/// Run the MCP stdio server on process stdin/stdout.
pub fn serve_stdio<H: McpHandler>(handler: &mut H) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_io(handler, stdin.lock(), stdout.lock())
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    out: &mut Vec<u8>,
    max_bytes: usize,
) -> io::Result<usize> {
    let mut total = 0usize;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(total);
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            let take = pos + 1;
            total += take;
            if total > max_bytes {
                out.clear();
                out.extend_from_slice(b"{\"oversized\":");
            } else {
                out.extend_from_slice(&available[..take]);
            }
            reader.consume(take);
            return Ok(total);
        }
        let len = available.len();
        let room = max_bytes.saturating_sub(out.len());
        out.extend_from_slice(&available[..len.min(room)]);
        reader.consume(len);
        total += len;
        if total > max_bytes {
            // Drain until newline or EOF so stream stays synchronized, then return invalid JSON.
            out.clear();
            out.extend_from_slice(b"{\"oversized\":");
            loop {
                let avail = reader.fill_buf()?;
                if avail.is_empty() {
                    return Ok(total);
                }
                if let Some(pos) = avail.iter().position(|&b| b == b'\n') {
                    reader.consume(pos + 1);
                    return Ok(total);
                }
                let l = avail.len();
                reader.consume(l);
            }
        }
    }
}

/// Pure-Rust FIPS 180-4 SHA-256 hex digest.
pub fn sha256_hex(data: &[u8]) -> String {
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
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut w = [0u32; 64];
    for chunk in msg.as_chunks::<64>().0 {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
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
        let delta = [a, b, c, d, e, f, g, hh];
        for i in 0..8 {
            h[i] = h[i].wrapping_add(delta[i]);
        }
    }
    let mut hex = String::with_capacity(64);
    for word in h {
        for byte in word.to_be_bytes() {
            use std::fmt::Write as _;
            let _ = write!(&mut hex, "{byte:02x}");
        }
    }
    hex
}

/// Standard Base64 encoder (RFC 4648).
pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(prefix: &str) -> PathBuf {
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{ns}", std::process::id()))
    }

    #[test]
    fn sha256_and_base64_vectors_match() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
    }

    #[test]
    fn annotations_and_disjoint_extension_registry() {
        let ann = tool_annotations(true);
        assert_eq!(ann["readOnlyHint"], true);
        assert_eq!(ann["destructiveHint"], false);
        assert_eq!(ann["openWorldHint"], false);
        assert_eq!(ann["idempotentHint"], true);

        let mut reg = ExtensionRegistry::new(["codex_status", "codex_turn"]);
        assert!(reg
            .register(ExtensionSpec::new("codex_status", "dup", json!({}), true))
            .is_err());
        reg.register(ExtensionSpec::new(
            "thread_fork",
            "Fork thread",
            json!({"type":"object","required":["thread_id"]}),
            false,
        ))
        .unwrap();
        let err = reg.validation_error("thread_fork", "missing thread_id");
        assert_eq!(err["expected_schema"]["required"], json!(["thread_id"]));
        let unknown = reg.validation_error("nope", "unknown_action");
        assert_eq!(unknown["available_actions"], json!(["thread_fork"]));
    }

    #[test]
    fn receipt_store_prevents_replay_and_detects_conflict_and_unsettled() {
        let dir = tmp_dir("mcp-kit-rcpt");
        let store = ReceiptStore::open(dir.clone(), "test-receipt", "payload_conflict").unwrap();
        let path = match store.claim("id-1", "fp-a", &["fp-a"]) {
            ReceiptClaim::Acquired(p) => p,
            other => panic!("expected Acquired, got {other:?}"),
        };
        // Unsettled intent -> delivery_unknown, never re-acquired!
        match store.claim("id-1", "fp-a", &["fp-a"]) {
            ReceiptClaim::Existing(v) => assert_eq!(v["state"], "delivery_unknown"),
            other => panic!("expected Existing delivery_unknown, got {other:?}"),
        }
        // Payload conflict
        match store.claim("id-1", "fp-b", &["fp-b"]) {
            ReceiptClaim::Existing(v) => assert_eq!(v["error"], "payload_conflict"),
            other => panic!("expected conflict, got {other:?}"),
        }
        // Commit settled outcome -> replay returns original outcome with replayed_from_receipt=true
        store
            .commit(&path, "fp-a", &json!({"state":"ok","stdout":"hi"}))
            .unwrap();
        match store.claim("id-1", "fp-a", &["fp-a"]) {
            ReceiptClaim::Existing(v) => {
                assert_eq!(v["stdout"], "hi");
                assert_eq!(v["replayed_from_receipt"], true);
            }
            other => panic!("expected settled replay, got {other:?}"),
        }
        // Bounded pruning keeps unsettled receipts untouched and prunes oldest settled ones
        let p2 = match store.claim("id-2", "fp-2", &["fp-2"]) {
            ReceiptClaim::Acquired(p) => p,
            other => panic!("expected Acquired, got {other:?}"),
        };
        store.commit(&p2, "fp-2", &json!({"state":"ok"})).unwrap();
        let _unsettled = match store.claim("id-unsettled", "fp-u", &["fp-u"]) {
            ReceiptClaim::Acquired(p) => p,
            other => panic!("expected Acquired, got {other:?}"),
        };
        store.prune_settled(1);
        assert_eq!(
            store.inspect("id-unsettled", &["fp-u"])["state"],
            "delivery_unknown"
        );
        let settled_remaining = [
            store.receipt_path("id-1").exists(),
            store.receipt_path("id-2").exists(),
        ]
        .into_iter()
        .filter(|&x| x)
        .count();
        assert_eq!(settled_remaining, 1);

        // Oversized single line ending with newline in the same chunk must still be rejected
        let mut cursor = io::Cursor::new(b"{\"id\":1,\"method\":\"ping\"}     \n".as_slice());
        let mut buf = Vec::new();
        let _ = read_bounded_line(&mut cursor, &mut buf, 20).unwrap();
        assert!(serde_json::from_slice::<Value>(&buf).is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    struct DummyHandler;
    impl McpHandler for DummyHandler {
        fn server_name(&self) -> &str {
            "DummyMCP"
        }
        fn server_version(&self) -> &str {
            "0.1.0"
        }
        fn tools(&self) -> Vec<ToolDef> {
            vec![ToolDef::new(
                "ping_tool",
                "Ping",
                json!({"type":"object"}),
                true,
            )]
        }
        fn call_tool(&mut self, name: &str, _arguments: &Value) -> (Value, bool) {
            (json!({"tool": name, "ok": true}), false)
        }
    }

    #[test]
    fn stdio_loop_handles_initialize_list_call_and_notifications() {
        let input = concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"ping_tool\",\"arguments\":{}}}\n"
        );
        let mut out = Vec::new();
        let mut h = DummyHandler;
        serve_io(&mut h, input.as_bytes(), &mut out).unwrap();
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(lines[1]["result"]["tools"][0]["name"], "ping_tool");
        assert_eq!(
            lines[1]["result"]["tools"][0]["annotations"]["readOnlyHint"],
            true
        );
        assert_eq!(lines[2]["result"]["isError"], false);
    }
}
