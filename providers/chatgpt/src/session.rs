//! Lightweight persistent Responses session store (`response_threads.json`),
//! per-session single-flight leasing, project name cache, and active MCP
//! connector cache.

use crate::config::random_hex;
use crate::driver::DriverBackend;
use crate::error::{ProviderError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;

pub const MAX_RESPONSE_THREADS: usize = 256;
pub const THREAD_SCHEMA_VERSION: u32 = 3;
const CONNECTOR_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadBinding {
    pub response_id: String,
    #[serde(default)]
    pub remote_head: Option<String>,
    #[serde(default)]
    pub request_input_len: usize,
    #[serde(default)]
    pub request_input_sha256: [u8; 32],
    #[serde(default)]
    pub output_unavailable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadEntry {
    pub chat_id: String,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub input_len: usize,
    #[serde(default)]
    pub prefix_sha256: [u8; 32],
    #[serde(default)]
    pub native: Option<ThreadBinding>,
}

#[derive(Serialize, Deserialize)]
struct ThreadFile {
    schema_version: u32,
    entries: VecDeque<(String, ThreadEntry)>,
}

#[derive(Default)]
struct ProjectCache {
    loaded: bool,
    by_lower_name: HashMap<String, String>,
}

#[derive(Default)]
struct ConnectorCache {
    fetched_at: Option<Instant>,
    names: Vec<String>,
}

#[derive(Default)]
pub struct ResponseThreadMap {
    entries: Mutex<VecDeque<(String, ThreadEntry)>>,
    active: Mutex<HashSet<String>>,
    projects: Mutex<ProjectCache>,
    project_locks: AsyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>,
    connectors: Mutex<ConnectorCache>,
    connector_lock: AsyncMutex<()>,
    store: Option<(PathBuf, File)>,
}

pub struct ThreadLease(Arc<ResponseThreadMap>, String);

impl Drop for ThreadLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.active.lock() {
            active.remove(&self.1);
        }
    }
}

pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
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
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = Vec::with_capacity(data.len() + 72);
    msg.extend_from_slice(data);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for offset in (0..msg.len()).step_by(64) {
        let chunk = &msg[offset..offset + 64];
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[4 * i..4 * i + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (dst, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *dst = dst.wrapping_add(add);
        }
    }
    let mut out = [0u8; 32];
    for (i, word) in state.into_iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn prefix_extend(mut digest: [u8; 32], items: &[Value]) -> [u8; 32] {
    for item in items {
        let mut bytes = digest.to_vec();
        bytes.extend(serde_json::to_vec(item).expect("JSON value serializes"));
        digest = sha256_bytes(&bytes);
    }
    digest
}

pub fn wire_uuid() -> Result<String> {
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

fn check_private_file(file: &File) -> io::Result<()> {
    let meta = file.metadata()?;
    let uid = unsafe { libc::geteuid() };
    if !meta.file_type().is_file() || meta.uid() != uid || (meta.mode() & 0o077) != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "response thread state file must be a private regular file",
        ));
    }
    Ok(())
}

impl ResponseThreadMap {
    pub fn lease(self: &Arc<Self>, key: &str) -> Result<ThreadLease> {
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

    pub fn current_response_key(&self, response_id: &str) -> Option<String> {
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
                    let loaded: VecDeque<(String, ThreadEntry)> =
                        if let Ok(disk) = serde_json::from_slice::<ThreadFile>(&data) {
                            if disk.schema_version != THREAD_SCHEMA_VERSION {
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
    pub async fn resolve_project_by_name(
        &self,
        driver: &Arc<dyn DriverBackend>,
        name: &str,
        memory_scope: Option<&str>,
    ) -> Option<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 256 {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();
        let name_lock = {
            let mut locks = self.project_locks.lock().await;
            locks.retain(|_, lock| Arc::strong_count(lock) > 1);
            if !locks.contains_key(&lower) && locks.len() >= 256 {
                return None;
            }
            Arc::clone(
                locks
                    .entry(lower.clone())
                    .or_insert_with(|| Arc::new(AsyncMutex::new(()))),
            )
        };
        let _name_guard = name_lock.lock().await;
        let needs_load = {
            let guard = self.projects.lock().ok()?;
            if let Some(id) = guard.by_lower_name.get(&lower) {
                return Some(id.clone());
            }
            !guard.loaded
        };
        if needs_load {
            let res = driver.list_projects(json!({ "limit": 100 })).await.ok()?;
            if res.get("state").and_then(Value::as_str) != Some("ok") {
                return None;
            }
            if let Ok(mut guard) = self.projects.lock() {
                if let Some(arr) = res.get("projects").and_then(Value::as_array) {
                    for project in arr {
                        let pid = project
                            .get("project_id")
                            .or_else(|| project.get("id"))
                            .and_then(Value::as_str)
                            .filter(|id| !id.is_empty());
                        let pname = project
                            .get("name")
                            .or_else(|| project.get("title"))
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|name| !name.is_empty());
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

    /// Best-effort cached discovery of active custom MCP connector names for the
    /// one-turn bridge directive (filters retired `@TermuxBridge`; fails open).
    pub async fn active_mcp_connectors(&self, driver: &Arc<dyn DriverBackend>) -> Vec<String> {
        if let Ok(guard) = self.connectors.lock() {
            if guard
                .fetched_at
                .is_some_and(|t| t.elapsed() < CONNECTOR_CACHE_TTL)
            {
                return guard.names.clone();
            }
        }
        let _guard = self.connector_lock.lock().await;
        if let Ok(guard) = self.connectors.lock() {
            if guard
                .fetched_at
                .is_some_and(|t| t.elapsed() < CONNECTOR_CACHE_TTL)
            {
                return guard.names.clone();
            }
        }
        let mut names = Vec::new();
        if let Ok(res) = driver.list_connectors(json!({})).await {
            if res.get("state").and_then(Value::as_str) == Some("ok") {
                if let Some(arr) = res.get("connectors").and_then(Value::as_array) {
                    for c in arr {
                        let has_tunnel = c
                            .get("tunnel_id")
                            .and_then(Value::as_str)
                            .is_some_and(|s| !s.trim().is_empty());
                        let name = c
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .unwrap_or("");
                        if has_tunnel
                            && !name.is_empty()
                            && !name.eq_ignore_ascii_case("@TermuxBridge")
                            && !name.eq_ignore_ascii_case("TermuxBridge")
                            && !names.iter().any(|n: &String| n.eq_ignore_ascii_case(name))
                        {
                            names.push(name.to_owned());
                        }
                    }
                }
            }
        }
        if let Ok(mut guard) = self.connectors.lock() {
            guard.fetched_at = Some(Instant::now());
            guard.names = names.clone();
        }
        names
    }

    pub fn record(&self, entries: impl IntoIterator<Item = (String, ThreadEntry)>) -> Result<()> {
        self.update(
            |next| {
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
            },
            |path| File::open(path)?.sync_all(),
        )
    }

    pub fn update(
        &self,
        change: impl FnOnce(&mut VecDeque<(String, ThreadEntry)>) -> Result<()>,
        sync_directory: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<()> {
        let mut guard = self.entries.lock().map_err(|_| {
            ProviderError::internal("response_thread_map_write", "thread map lock poisoned")
        })?;
        let mut next = guard.clone();
        change(&mut next)?;
        if let Some((path, _)) = &self.store {
            let bytes = serde_json::to_vec(&ThreadFile {
                schema_version: THREAD_SCHEMA_VERSION,
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
            *guard = next;
            sync_directory(path.parent().expect("state path has a parent")).map_err(|err| {
                ProviderError::internal("response_thread_map_sync", err.to_string())
            })?;
        } else {
            *guard = next;
        }
        Ok(())
    }
}
