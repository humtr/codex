//! Durable `Work Ledger` (`work.json`, `checkpoint.md`) and `Typed Inbox` (`events.jsonl`)
//! for Stage 2 Slice 7 (`@TermuxOrch` / `termux-orch relay`).
//!
//! Security & crash-safety invariants:
//! - Private `0700` directories and `0600` files (`O_NOFOLLOW`, owner UID checked).
//! - Per-work exclusive advisory `flock` so concurrent `termux-orch mcp` and
//!   `termux-orch relay` processes serialize mutations cleanly.
//! - Atomic write-temp + `fsync(file)` + `rename` + `fsync(dir)` for `work.json`
//!   and `checkpoint.md`; `fsync` on append for `events.jsonl`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const WORK_SCHEMA: &str = "termux-orch-work-v1";
pub const DEFAULT_MAX_WAKEUPS: u32 = 20;
pub const DEFAULT_MAX_WALL_SECS: u64 = 14_400; // 4 hours
pub const DEFAULT_MAX_CONCURRENT_WORKERS: u32 = 8;
pub const DEFAULT_MIN_COOLDOWN_SECS: u64 = 3;
const MAX_CHECKPOINT_BYTES: usize = 64 * 1024;
static TMP_SERIAL: AtomicU64 = AtomicU64::new(1);

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn generate_id(prefix: &str) -> String {
    let mut bytes = [0u8; 8];
    if let Ok(mut f) = File::open("/dev/urandom") {
        let _ = f.read_exact(&mut bytes);
    } else {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        let seq = TMP_SERIAL.fetch_add(1, Ordering::Relaxed);
        bytes[..4].copy_from_slice(&(nanos as u32).to_le_bytes());
        bytes[4..].copy_from_slice(&(seq as u32).to_le_bytes());
    }
    let mut hex = String::with_capacity(prefix.len() + 17);
    hex.push_str(prefix);
    hex.push('_');
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

pub fn format_project_name(title: &str) -> String {
    let t = title.trim();
    if t.starts_with("[Work]") {
        truncate_chars(t, 120)
    } else {
        truncate_chars(&format!("[Work] {t}"), 120)
    }
}

pub fn format_front_title(generation: u32, work_title: &str) -> String {
    truncate_chars(&format!("[F{generation}] {}", work_title.trim()), 180)
}

pub fn format_lead_title(generation: u32, work_title: &str) -> String {
    truncate_chars(&format!("[L{generation}] {}", work_title.trim()), 180)
}

pub fn format_participant_title(role: &str, index: u32, slug: &str, subtask: &str) -> String {
    let tag = if role == "verifier" { "V" } else { "W" };
    let clean_slug: String = slug
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(24)
        .collect();
    let label = if clean_slug.is_empty() {
        format!("[{tag}{index}]")
    } else {
        format!("[{tag}{index}-{clean_slug}]")
    };
    truncate_chars(&format!("{label} {}", subtask.trim()), 180)
}

pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        s.chars().take(max_chars).collect()
    }
}

pub fn is_safe_work_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontHistoryEntry {
    pub generation: u32,
    pub chat_id: String,
    pub bound_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontBinding {
    pub latest_chat_id: Option<String>,
    pub generation: u32,
    #[serde(default)]
    pub history: Vec<FrontHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadHistoryEntry {
    pub generation: u32,
    pub kind: String,
    pub ref_id: String,
    pub started_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadBinding {
    /// `"chatgpt"` or `"codex"`
    pub kind: String,
    pub ref_id: String,
    pub profile: Option<String>,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub generation: u32,
    #[serde(default)]
    pub history: Vec<LeadHistoryEntry>,
    #[serde(default)]
    pub rollover_internal_turns: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub participant_id: String,
    pub index: u32,
    /// `"worker"` or `"verifier"`
    pub role: String,
    /// `"codex"`, `"chatgpt"`, or `"shell"`
    pub backend: String,
    pub ref_id: String,
    pub profile: Option<String>,
    pub slug: String,
    pub task_summary: String,
    pub owner_scope: Option<String>,
    pub call_id: Option<String>,
    /// `"running"`, `"completed"`, `"failed"`, `"cancelled"`
    pub state: String,
    pub result_summary: Option<String>,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl Participant {
    pub fn is_terminal(&self) -> bool {
        matches!(self.state.as_str(), "completed" | "failed" | "cancelled")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitSet {
    pub wait_id: String,
    /// `"lead"` or `"front"`
    pub wake_target: String,
    #[serde(default)]
    pub all_of: Vec<String>,
    #[serde(default)]
    pub any_of: Vec<String>,
    #[serde(default = "default_true")]
    pub on_user_event: bool,
    pub deadline_unix: Option<u64>,
    pub armed: bool,
    pub created_at_unix: u64,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDelivery {
    pub delivery_id: String,
    /// `"front"` or `"lead"`
    pub target: String,
    pub kind: String,
    pub title: String,
    pub payload_text: String,
    pub advance_seq_to: Option<u64>,
    pub notified: bool,
    pub attempts: u32,
    pub next_attempt_unix: u64,
    pub last_error: Option<String>,
    pub created_at_unix: u64,
}

fn default_lead_rollover_turns() -> u32 {
    80
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadRollover {
    pub lane_id: String,
    pub reason: String,
    #[serde(default)]
    pub dispatched: bool,
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadReplay {
    pub through_seq: u64,
    pub dispatched_until: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkBudget {
    pub max_wakeups: u32,
    pub wakeups_used: u32,
    pub max_wall_secs: u64,
    pub max_concurrent_workers: u32,
    pub min_cooldown_secs: u64,
    pub last_wakeup_unix: u64,
    #[serde(default = "default_lead_rollover_turns")]
    pub lead_rollover_turns: u32,
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self {
            max_wakeups: DEFAULT_MAX_WAKEUPS,
            wakeups_used: 0,
            max_wall_secs: DEFAULT_MAX_WALL_SECS,
            max_concurrent_workers: DEFAULT_MAX_CONCURRENT_WORKERS,
            min_cooldown_secs: DEFAULT_MIN_COOLDOWN_SECS,
            last_wakeup_unix: 0,
            lead_rollover_turns: default_lead_rollover_turns(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkRecord {
    pub schema: String,
    pub work_id: String,
    pub title: String,
    /// `"direct"` (Tier 1) or `"orchestrated"` (Tier 2/3)
    pub tier: String,
    pub goal: String,
    #[serde(default)]
    pub completion_criteria: Vec<String>,
    /// `"running"`, `"waiting"`, `"waiting_user"`, `"completed"`, `"failed"`, `"cancelled"`
    pub state: String,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    /// `"inject_and_notify"` (default) or `"notify_only"`
    pub delivery_mode: String,
    pub front: FrontBinding,
    pub lead: Option<LeadBinding>,
    #[serde(default)]
    pub lead_rollover: Option<LeadRollover>,
    #[serde(default)]
    pub lead_replay: Option<LeadReplay>,
    #[serde(default)]
    pub participants: Vec<Participant>,
    pub wait_set: Option<WaitSet>,
    #[serde(default)]
    pub pending_deliveries: Vec<PendingDelivery>,
    pub consumed_seq: u64,
    pub last_seq: u64,
    pub budget: WorkBudget,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl WorkRecord {
    pub fn is_terminal(&self) -> bool {
        matches!(self.state.as_str(), "completed" | "failed" | "cancelled")
    }

    pub fn active_worker_count(&self) -> u32 {
        self.participants
            .iter()
            .filter(|p| p.role == "worker" && !p.is_terminal())
            .count() as u32
    }

    pub fn next_role_index(&self, role: &str) -> u32 {
        self.participants
            .iter()
            .filter(|p| p.role == role)
            .map(|p| p.index)
            .max()
            .unwrap_or(0)
            + 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxEvent {
    pub seq: u64,
    pub event_id: String,
    pub kind: String,
    pub timestamp_unix: u64,
    pub actor: String,
    pub summary: String,
    #[serde(default)]
    pub data: Value,
}

pub struct WorkLock {
    _file: File,
}

#[derive(Debug, Clone)]
pub struct Ledger {
    root: PathBuf,
}

impl Ledger {
    pub fn open(state_root: PathBuf) -> io::Result<Self> {
        ensure_private_dir(&state_root)?;
        ensure_private_dir(&state_root.join("jobs"))?;
        Ok(Self { root: state_root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn jobs_dir(&self) -> PathBuf {
        self.root.join("jobs")
    }

    pub fn work_dir(&self, work_id: &str) -> io::Result<PathBuf> {
        if !is_safe_work_id(work_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid work_id",
            ));
        }
        Ok(self.jobs_dir().join(work_id))
    }

    pub fn lock_work(&self, work_id: &str) -> io::Result<WorkLock> {
        let dir = self.work_dir(work_id)?;
        ensure_private_dir(&dir)?;
        let lock_path = dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&lock_path)?;
        loop {
            let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
            if rc == 0 {
                break;
            }
            let err = io::Error::last_os_error();
            if err.kind() != io::ErrorKind::Interrupted {
                return Err(err);
            }
        }
        Ok(WorkLock { _file: file })
    }

    pub fn save_work(&self, work: &WorkRecord) -> io::Result<()> {
        let dir = self.work_dir(&work.work_id)?;
        ensure_private_dir(&dir)?;
        let bytes = serde_json::to_vec_pretty(work)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_atomic_private(&dir.join("work.json"), &bytes)
    }

    pub fn load_work(&self, work_id: &str) -> io::Result<WorkRecord> {
        let dir = self.work_dir(work_id)?;
        let path = dir.join("work.json");
        let raw = read_private_file(&path, 2 * 1024 * 1024)?;
        serde_json::from_slice(&raw).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn list_work_ids(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        let jobs = self.jobs_dir();
        if !jobs.is_dir() {
            return Ok(ids);
        }
        for entry in fs::read_dir(&jobs)? {
            let entry = entry?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if is_safe_work_id(&name) && entry.path().join("work.json").is_file() {
                ids.push(name);
            }
        }
        ids.sort();
        Ok(ids)
    }

    pub fn list_works(&self) -> io::Result<Vec<WorkRecord>> {
        let mut out = Vec::new();
        for id in self.list_work_ids()? {
            if let Ok(rec) = self.load_work(&id) {
                out.push(rec);
            }
        }
        out.sort_by(|a, b| {
            b.updated_at_unix
                .cmp(&a.updated_at_unix)
                .then_with(|| a.work_id.cmp(&b.work_id))
        });
        Ok(out)
    }

    pub fn write_checkpoint(&self, work_id: &str, markdown: &str) -> io::Result<()> {
        let dir = self.work_dir(work_id)?;
        ensure_private_dir(&dir)?;
        let mut end = markdown.len().min(MAX_CHECKPOINT_BYTES);
        while !markdown.is_char_boundary(end) {
            end -= 1;
        }
        let bounded = &markdown[..end];
        write_atomic_private(&dir.join("checkpoint.md"), bounded.as_bytes())
    }

    pub fn read_checkpoint(&self, work_id: &str) -> io::Result<Option<String>> {
        let dir = self.work_dir(work_id)?;
        let path = dir.join("checkpoint.md");
        match read_private_file(&path, MAX_CHECKPOINT_BYTES as u64 + 1024) {
            Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Append an immutable `InboxEvent` deduplicated by `event_id`.
    /// Must be called while holding `lock_work(&work.work_id)`.
    pub fn append_event_if_absent(
        &self,
        work: &mut WorkRecord,
        event_id: &str,
        kind: &str,
        actor: &str,
        summary: &str,
        data: Value,
    ) -> io::Result<(InboxEvent, bool)> {
        let dir = self.work_dir(&work.work_id)?;
        ensure_private_dir(&dir)?;
        let path = dir.join("events.jsonl");

        let mut max_seq = work.last_seq;
        let mut already_present = None;
        if path.exists() {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)?;
            let mut reader = BufReader::new(&file);
            let mut offset = 0;
            let mut line = Vec::new();
            loop {
                line.clear();
                let read = reader.read_until(b'\n', &mut line)?;
                if read == 0 {
                    break;
                }
                if line.last() != Some(&b'\n') {
                    // A previous writer can die during the final append. Every
                    // committed row includes its newline, so discard that tail.
                    file.set_len(offset)?;
                    file.sync_all()?;
                    break;
                }
                offset += read as u64;
                if let Ok(ev) = serde_json::from_slice::<InboxEvent>(&line) {
                    max_seq = max_seq.max(ev.seq);
                    if ev.event_id == event_id {
                        already_present = Some(ev);
                    }
                }
            }
        }
        if let Some(event) = already_present {
            work.last_seq = max_seq;
            return Ok((event, false));
        }

        let next_seq = max_seq + 1;
        let event = InboxEvent {
            seq: next_seq,
            event_id: event_id.to_owned(),
            kind: kind.to_owned(),
            timestamp_unix: now_unix(),
            actor: actor.to_owned(),
            summary: truncate_chars(summary, 2000),
            data,
        };
        let mut line = serde_json::to_string(&event)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)?;
        file.write_all(line.as_bytes())?;
        file.sync_all()?;
        sync_dir(&dir)?;

        work.last_seq = next_seq;
        work.updated_at_unix = event.timestamp_unix;
        Ok((event, true))
    }

    pub fn read_events(
        &self,
        work_id: &str,
        after_seq: u64,
        limit: usize,
    ) -> io::Result<Vec<InboxEvent>> {
        self.scan_events(work_id, after_seq, limit, false)
    }

    pub fn read_first_events(
        &self,
        work_id: &str,
        after_seq: u64,
        limit: usize,
    ) -> io::Result<Vec<InboxEvent>> {
        self.scan_events(work_id, after_seq, limit, true)
    }

    fn scan_events(
        &self,
        work_id: &str,
        after_seq: u64,
        limit: usize,
        first: bool,
    ) -> io::Result<Vec<InboxEvent>> {
        let dir = self.work_dir(work_id)?;
        let path = dir.join("events.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)?;
        let mut events = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(ev) = serde_json::from_str::<InboxEvent>(&line) {
                if ev.seq > after_seq {
                    events.push(ev);
                    if first && events.len() == limit {
                        break;
                    }
                }
            }
        }
        if events.len() > limit {
            events = events.split_off(events.len() - limit);
        }
        Ok(events)
    }
}

pub fn ensure_private_dir(path: &Path) -> io::Result<()> {
    let euid = unsafe { libc::geteuid() };
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_dir() || meta.uid() != euid {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("unsafe directory {}", path.display()),
                ));
            }
            if meta.permissions().mode() & 0o077 != 0 {
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
            }
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            DirBuilder::new().mode(0o700).create(path)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn read_private_file(path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    let euid = unsafe { libc::geteuid() };
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.uid() != euid || meta.mode() & 0o077 != 0 || meta.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("unsafe or oversized file {}", path.display()),
        ));
    }
    let mut buf = Vec::with_capacity(meta.len() as usize);
    file.take(max_bytes + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "private file grew beyond its limit",
        ));
    }
    Ok(buf)
}

pub(crate) fn write_atomic_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing filename"))?;
    let serial = TMP_SERIAL.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!(".{name}.tmp.{}.{serial}", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    let write_res = (|| -> io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    drop(file);
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    sync_dir(parent)
}

fn sync_dir(dir: &Path) -> io::Result<()> {
    let d = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(dir)?;
    d.sync_all()
}
