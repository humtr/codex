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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SessionPrefixes {
    pub front: Option<String>,
    pub lead: Option<String>,
    pub worker: Option<String>,
    pub verifier: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectBinding {
    pub name: String,
    pub id: Option<String>,
}

fn session_title(prefix: &str, number: u32, slug: &str, title: &str) -> String {
    let prefix = prefix
        .replace("{n}", &number.to_string())
        .replace("{slug}", slug);
    let title = format!("{prefix} {}", title.trim());
    let title = title.trim();
    let limit = title
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|i| *i <= 180)
        .last();
    if title.len() > 180 {
        title[..limit.unwrap_or(0)].trim_end().to_owned()
    } else {
        title.to_owned()
    }
}

pub fn format_front_title(prefixes: &SessionPrefixes, generation: u32, work_title: &str) -> String {
    session_title(
        prefixes.front.as_deref().unwrap_or("[F{n}]"),
        generation,
        "",
        work_title,
    )
}

pub fn format_lead_title(prefixes: &SessionPrefixes, generation: u32, work_title: &str) -> String {
    session_title(
        prefixes.lead.as_deref().unwrap_or("[L{n}]"),
        generation,
        "",
        work_title,
    )
}

pub fn format_participant_title(
    prefixes: &SessionPrefixes,
    role: &str,
    index: u32,
    slug: &str,
    subtask: &str,
) -> String {
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
    let (custom, with_slug, without_slug) = if role == "verifier" {
        (&prefixes.verifier, "[V{n}-{slug}]", "[V{n}]")
    } else {
        (&prefixes.worker, "[W{n}-{slug}]", "[W{n}]")
    };
    let prefix = custom.as_deref().unwrap_or(if clean_slug.is_empty() {
        without_slug
    } else {
        with_slug
    });
    session_title(prefix, index, &clean_slug, subtask)
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub subject: String,
    pub challenge: String,
    pub verdict: Option<Verdict>,
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_issues: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub profile: Option<String>,
    pub slug: String,
    pub task_summary: String,
    pub owner_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_workspace: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<crate::workspace::WorkerTree>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_paths: Vec<String>,
    pub call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_id: Option<String>,
    /// `"dispatching"`, `"running"`, `"completed"`, `"failed"`, `"cancelled"`
    pub state: String,
    pub result_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<Verification>,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl Participant {
    pub fn is_terminal(&self) -> bool {
        matches!(self.state.as_str(), "completed" | "failed" | "cancelled")
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Isolation {
    #[default]
    SharedSerial,
    Worktree,
    Readonly,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepFailure {
    WakeLead,
    #[default]
    FailWork,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationPolicy {
    #[default]
    None,
    Automated,
    IndependentReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepSpec {
    pub step_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub command: String,
    pub cwd: PathBuf,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub isolation: Isolation,
    #[serde(default)]
    pub on_failure: StepFailure,
    #[serde(default)]
    pub auto_verify: bool,
}

impl StepSpec {
    pub fn validate_plan(steps: &mut [Self]) -> Result<(), String> {
        if steps.len() > 16 {
            return Err("at most 16 steps are supported".to_owned());
        }
        if steps
            .iter()
            .enumerate()
            .any(|(i, step)| step.auto_verify && i + 1 != steps.len())
        {
            return Err("the verification command must be the final step".to_owned());
        }
        let mut preceding = std::collections::HashSet::new();
        for step in steps {
            if step.step_id.is_empty()
                || step.step_id.len() > 48
                || !step
                    .step_id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                || preceding.contains(&step.step_id)
                || !step.depends_on.iter().all(|id| preceding.contains(id))
            {
                return Err(
                    "step IDs must be unique and dependencies must precede their consumers"
                        .to_owned(),
                );
            }
            preceding.insert(step.step_id.clone());
            if step.command.trim().is_empty()
                || step.command.len() > 65536
                || step.command.contains('\0')
            {
                return Err(format!("invalid command for step {}", step.step_id));
            }
            let root = crate::workspace::root(&step.cwd)?;
            step.cwd = step.cwd.canonicalize().map_err(|err| err.to_string())?;
            if step.isolation == Isolation::Worktree && step.cwd != root {
                return Err("worktree steps require a repository root".to_owned());
            }
            crate::workspace::validate_paths(&step.write_paths)?;
            if step.isolation == Isolation::Worktree && step.write_paths.is_empty() {
                return Err("worktree steps require declared write_paths".to_owned());
            }
            if step.isolation == Isolation::Readonly && !step.write_paths.is_empty() {
                return Err("readonly steps cannot declare write_paths".to_owned());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellContinuation {
    pub command: String,
    pub cwd: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then_verify_shell: Option<ShellContinuation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_id: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryDispatch {
    pub backend: String,
    pub ref_id: String,
    pub profile: Option<String>,
    pub lane_id: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch: Option<DeliveryDispatch>,
    #[serde(default)]
    pub receipts_tracked: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_cursor: Option<String>,
    #[serde(default)]
    pub size_rejection: bool,
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
            lead_rollover_turns: 0,
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
    #[serde(default)]
    pub project_scope: Option<String>,
    #[serde(default)]
    pub session_prefixes: SessionPrefixes,
    /// `"inject_and_notify"` (default) or `"notify_only"`
    pub delivery_mode: String,
    pub front: FrontBinding,
    pub lead: Option<LeadBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_lead: Option<LeadBinding>,
    #[serde(default)]
    pub lead_rollover: Option<LeadRollover>,
    #[serde(default)]
    pub lead_replay: Option<LeadReplay>,
    #[serde(default)]
    pub participants: Vec<Participant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepSpec>,
    #[serde(default)]
    pub verification_policy: VerificationPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_cwd: Option<PathBuf>,
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
        matches!(
            self.state.as_str(),
            "completed" | "failed" | "failed_verification" | "cancelled"
        )
    }

    pub fn can_start(&self) -> bool {
        !self.is_terminal() && self.state != "cancelling"
    }

    pub fn bind_lead(&mut self, mut lead: LeadBinding) {
        let mut history = self
            .lead
            .take()
            .map(|prev| prev.history)
            .unwrap_or_default();
        history.push(LeadHistoryEntry {
            generation: lead.generation,
            kind: lead.kind.clone(),
            ref_id: lead.ref_id.clone(),
            started_at_unix: now_unix(),
        });
        lead.history = history;
        self.lead = Some(lead);
        self.pending_lead = None;
        self.tier = "orchestrated".to_owned();
        if let Some(wait) = self.wait_set.as_mut().filter(|wait| wait.armed) {
            wait.wake_target = "lead".to_owned();
        }
    }

    pub fn active_worker_count(&self) -> u32 {
        self.participants
            .iter()
            .filter(|p| p.role == "worker" && !p.is_terminal())
            .count() as u32
    }

    pub fn verification_subject(&self) -> String {
        let workers: Vec<_> = self
            .participants
            .iter()
            .filter(|p| p.role == "worker")
            .map(|p| (&p.participant_id, &p.ref_id, &p.turn_id, &p.state))
            .collect();
        let mut subject = serde_json::json!([self.goal, self.completion_criteria, workers]);
        // The file snapshot is scoped to workspaces the Work is managing.
        if self.verification_policy != VerificationPolicy::None {
            let roots: std::collections::BTreeSet<_> = self
                .steps
                .iter()
                .map(|step| &step.cwd)
                .chain(self.review_cwd.iter())
                .chain(self.participants.iter().filter_map(|p| {
                    if p.role != "worker" {
                        return None;
                    }
                    p.worktree
                        .as_ref()
                        .map(|tree| &tree.path)
                        .or(p.write_workspace.as_ref())
                }))
                .collect();
            let mut snapshots = Vec::new();
            for root in roots {
                match crate::workspace::verification_fingerprint(root) {
                    Ok(Some(hash)) => snapshots.push((root, hash)),
                    Ok(None) => (),
                    // Unreadable evidence must never match a previous review.
                    Err(_) => return generate_id("unreadable-evidence"),
                }
            }
            if !snapshots.is_empty() {
                subject
                    .as_array_mut()
                    .expect("verification subject is an array")
                    .push(serde_json::json!(snapshots));
            }
        }
        mcp_kit::sha256_hex(subject.to_string().as_bytes())
    }

    pub fn verification_passed(&self) -> bool {
        let subject = self.verification_subject();
        let mut checked = std::collections::HashSet::new();
        let latest: Vec<_> = self
            .participants
            .iter()
            .rev()
            .filter(|p| p.role == "verifier" && checked.insert(&p.slug))
            .collect();
        let any_required = match self.verification_policy {
            VerificationPolicy::None => true,
            VerificationPolicy::Automated => latest.iter().any(|p| p.backend == "shell"),
            VerificationPolicy::IndependentReview => latest.iter().any(|p| p.backend != "shell"),
        };
        any_required
            && latest.into_iter().all(|p| {
                p.verification
                    .as_ref()
                    .is_some_and(|v| v.subject == subject && v.verdict == Some(Verdict::Pass))
            })
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
        Self::lock(&dir.join(".lock"))
    }

    pub fn lock_writers(&self) -> io::Result<WorkLock> {
        Self::lock(&self.root.join("writers.lock"))
    }

    pub fn writer_busy(&self, workspace: &Path) -> io::Result<bool> {
        for id in self.list_work_ids()? {
            let work = self.load_work(&id)?;
            if work.participants.iter().any(|p| {
                !p.is_terminal()
                    && p.write_workspace
                        .as_ref()
                        .is_some_and(|old| workspace.starts_with(old) || old.starts_with(workspace))
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn lock_project_scope(&self, account: &str, key: &str) -> io::Result<(WorkLock, PathBuf)> {
        let dir = self.root.join("projects");
        ensure_private_dir(&dir)?;
        Ok((
            Self::lock(&dir.join(format!("{account}.lock")))?,
            dir.join(format!("{key}.json")),
        ))
    }

    fn lock(lock_path: &Path) -> io::Result<WorkLock> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(lock_path)?;
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
        let mut events = std::collections::VecDeque::new();
        self.visit_events(work_id, after_seq, |event| {
            if limit != 0 {
                if events.len() == limit {
                    events.pop_front();
                }
                events.push_back(event);
            }
            true
        })?;
        Ok(events.into())
    }

    pub fn read_first_events(
        &self,
        work_id: &str,
        after_seq: u64,
        limit: usize,
    ) -> io::Result<Vec<InboxEvent>> {
        let mut events = Vec::new();
        if limit != 0 {
            self.visit_events(work_id, after_seq, |event| {
                events.push(event);
                events.len() < limit
            })?;
        }
        Ok(events)
    }

    pub fn has_user_event_after(&self, work_id: &str, after_seq: u64) -> io::Result<bool> {
        let mut found = false;
        self.visit_events(work_id, after_seq, |event| {
            found = matches!(event.kind.as_str(), "user.message" | "user.scope_updated");
            !found
        })?;
        Ok(found)
    }

    pub fn delivery_recorded(&self, work_id: &str, delivery_id: &str) -> io::Result<bool> {
        let id = format!("delivered:{delivery_id}");
        let mut found = false;
        self.visit_events(work_id, 0, |event| {
            found = event.event_id == id;
            !found
        })?;
        Ok(found)
    }

    fn visit_events(
        &self,
        work_id: &str,
        after_seq: u64,
        mut visit: impl FnMut(InboxEvent) -> bool,
    ) -> io::Result<()> {
        let path = self.work_dir(work_id)?.join("events.jsonl");
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Inbox is not a regular file",
            ));
        }
        let mut reader = BufReader::new(file);
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 || line.last() != Some(&b'\n') {
                break;
            }
            if let Ok(event) = serde_json::from_slice::<InboxEvent>(&line) {
                if event.seq > after_seq && !visit(event) {
                    break;
                }
            }
        }
        Ok(())
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
