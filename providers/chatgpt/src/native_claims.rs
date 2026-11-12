//! Process-shared Native tool effect admission. This is a short locked journal,
//! not a command executor or a workspace-lifetime lease.
//!
//! A claim is persisted before a handler starts. Only its observed terminal
//! result removes the claim; a dropped future or crashed process cannot do so.
use crate::native_effect::{self, Effect};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

const STATE_MAX: u64 = 4 * 1024 * 1024;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    claims: BTreeMap<String, Entry>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    thread: String,
    fingerprint: String,
    effect: Effect,
    #[serde(default)]
    session: Option<u64>,
    #[serde(default)]
    owner: Option<ProcessOwner>,
    // A side-effectful continuation has its own original call authority. Its
    // reservation is not the session's process claim and cannot be released
    // by a different call's late wait witness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    continuation_effect: Option<ContinuationEffect>,
    #[serde(default)]
    process_hooks_absent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ContinuationEffect {
    pub session: u64,
    pub hooks_absent: bool,
}
impl ContinuationEffect {
    pub(crate) fn settled_by(&self, observation: &ProcessObservation) -> bool {
        observation.proof.as_ref().is_some_and(|proof| {
            proof.session_id == Some(self.session)
                && (proof.process_tree_completed() || proof.running() == Some(self.session))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessOwner {
    pid: u32,
    start: u64,
}

fn process_identity(pid: u32) -> Result<(u64, bool), std::io::Error> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    parse_process_identity(&stat)
}

fn parse_process_identity(stat: &str) -> Result<(u64, bool), std::io::Error> {
    let (_, rest) = stat
        .rsplit_once(')')
        .ok_or_else(|| std::io::Error::other("invalid process identity"))?;
    let mut fields = rest.split_whitespace();
    let state = fields
        .next()
        .ok_or_else(|| std::io::Error::other("invalid process identity"))?;
    let start = fields
        .nth(18)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| std::io::Error::other("invalid process identity"))?;
    // A zombie still has a /proc entry and its original start time, but
    // cannot execute any more writes. Only unoffered claims use this proof.
    Ok((start, matches!(state, "Z" | "X")))
}

impl ProcessOwner {
    fn current() -> Result<Self, String> {
        let pid = std::process::id();
        let (start, _) =
            process_identity(pid).map_err(|_| "native_resource_process_unavailable")?;
        Ok(Self { pid, start })
    }

    fn departed(&self) -> bool {
        match process_identity(self.pid) {
            Ok((start, exited)) => start != self.start || exited,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            // Permission errors or malformed /proc are not departure proof.
            Err(_) => false,
        }
    }
}

#[derive(Clone)]
struct Ledger {
    directory: PathBuf,
}

pub(crate) struct Admission {
    pub effect: Effect,
    pub hooks_absent: bool,
}

pub(crate) struct OriginalResult<'a> {
    pub call_id: &'a str,
    pub value: &'a serde_json::Value,
}

pub(crate) struct Guard {
    ledger: Ledger,
    identity: String,
    thread: String,
    tool: String,
    continuation: Option<u64>,
}

fn fail() -> String {
    "native_resource_journal_unavailable".to_owned()
}

fn fingerprint(tool: &str, input: &str, cwd: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(format!("{tool}\0{input}\0{}", cwd.display()).as_bytes())
    )
}

fn private_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0 => {
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)
            {
                Ok(()) => private_directory(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => private_directory(path),
                Err(_) => Err(fail()),
            }
        }
        _ => Err(fail()),
    }
}

impl Ledger {
    fn new(cwd: &Path) -> Result<Self, String> {
        let prefix = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .ok_or_else(|| "native_resource_prefix_unavailable".to_owned())?;
        Self::at(cwd, &prefix.join("var/tmp/codex-native-effects"))
    }

    fn at(cwd: &Path, home: &Path) -> Result<Self, String> {
        // One private ledger for all Native worktrees under this Termux
        // account: relative cwd spelling, hardlink aliases, overlapping
        // nested repos and Git worktrees with a shared common dir must not
        // partition effect arbitration. The lock guards only the short state
        // transition, never a repo-wide execution lease.
        fs::canonicalize(cwd).map_err(|_| "native_workspace_unavailable".to_owned())?;
        private_directory(home)?;
        let directory = home.join("shared-native-v1");
        private_directory(&directory)?;
        Ok(Self { directory })
    }

    // Diagnostic reads share the existing lock and never create or rewrite state.
    fn snapshot(home: &Path) -> Result<State, String> {
        let directory = home.join("shared-native-v1");
        for path in [home, directory.as_path()] {
            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(State::default());
                }
                Ok(meta)
                    if meta.is_dir()
                        && meta.uid() == unsafe { libc::geteuid() }
                        && meta.mode() & 0o077 == 0 => {}
                _ => return Err(fail()),
            }
        }
        let lock = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(directory.join("lock"))
        {
            Ok(lock) => lock,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && fs::symlink_metadata(directory.join("claims.json"))
                        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                return Ok(State::default());
            }
            Err(_) => return Err(fail()),
        };
        let meta = lock.metadata().map_err(|_| fail())?;
        if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(fail());
        }
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } != 0 {
            return Err("native_resource_journal_snapshot_busy".into());
        }
        Self { directory }.read_state()
    }

    fn read_state(&self) -> Result<State, String> {
        match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(self.directory.join("claims.json"))
        {
            Ok(file) => {
                let metadata = file.metadata().map_err(|_| fail())?;
                if !metadata.is_file()
                    || metadata.len() > STATE_MAX
                    || metadata.uid() != unsafe { libc::geteuid() }
                    || metadata.mode() & 0o077 != 0
                {
                    return Err(fail());
                }
                let mut bytes = Vec::new();
                file.take(STATE_MAX + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| fail())?;
                if bytes.len() as u64 > STATE_MAX {
                    return Err(fail());
                }
                serde_json::from_slice(&bytes).map_err(|_| fail())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
            Err(_) => Err(fail()),
        }
    }

    fn transact<T>(
        &self,
        f: impl FnOnce(&mut State) -> Result<(T, bool), String>,
    ) -> Result<T, String> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(self.directory.join("lock"))
            .map_err(|_| fail())?;
        let m = lock.metadata().map_err(|_| fail())?;
        if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
            return Err(fail());
        }
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(fail());
        }
        // Each operation reopens the state under the inter-process lock. The
        // lock inode is stable: atomic rename replaces only the state file.
        let path = self.directory.join("claims.json");
        let mut state = self.read_state()?;
        let (result, changed) = f(&mut state)?;
        if changed {
            let bytes = serde_json::to_vec(&state).map_err(|_| fail())?;
            if bytes.len() as u64 > STATE_MAX {
                return Err(fail());
            }
            let temp = self.directory.join(format!(
                ".claims-{}",
                crate::journal::random_id().map_err(|_| fail())?
            ));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)
                .map_err(|_| fail())?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|_| fail())?;
            fs::rename(&temp, &path).map_err(|_| fail())?;
            File::open(&self.directory)
                .and_then(|dir| dir.sync_all())
                .map_err(|_| fail())?;
        }
        Ok(result)
    }
}

#[path = "native_completion.rs"]
pub(crate) mod completion;

fn continuation(tool: &str, input: &str) -> Option<u64> {
    (native_effect::process_tool(tool) == Some("write_stdin"))
        .then(|| {
            serde_json::from_str::<serde_json::Value>(input)
                .ok()?
                .get("session_id")?
                .as_u64()
        })
        .flatten()
}

/// Native's legacy wait result has no provenance for its synthetic timeout
/// sentinel. Ordinary OS exit codes remain usable; -1, 124 and text-only signal
/// reports do not prove a wait completed. A later typed Runtime proof can add
/// authority, but old booleans and arbitrary output text cannot.
pub(crate) fn confirmed_exit_code(code: i64) -> bool {
    (0..=255).contains(&code) && code != 124
}

fn legacy_process_result(value: &serde_json::Value) -> (Option<u64>, Option<i64>) {
    let Some(text) = completion::result_text(value) else {
        return (None, None);
    };
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if value["output"].is_string()
            && value["wall_time_seconds"]
                .as_f64()
                .is_some_and(|v| v.is_finite() && v >= 0.0)
            && value
                .get("session_id")
                .is_none_or(|v| v.is_null() || v.is_u64())
            && value
                .get("exit_code")
                .is_none_or(|v| v.is_null() || v.is_i64())
        {
            let running = value["session_id"].as_u64();
            let exit = value["exit_code"]
                .as_i64()
                .filter(|code| confirmed_exit_code(*code));
            return (running, running.is_none().then_some(exit).flatten());
        }
        return (None, None);
    }
    let header = text
        .split_once("\nOutput:\n")
        .map_or(text, |(head, _)| head);
    let mut running = None;
    let mut exit = None;
    let mut root_exit = None;
    for line in header.lines() {
        if let Some(value) = line
            .strip_prefix("Process running with session ID ")
            .or_else(|| line.strip_prefix("Process scope remains active with session ID "))
        {
            let Some(session) = (!value.is_empty() && value.bytes().all(|c| c.is_ascii_digit()))
                .then(|| value.parse::<u64>().ok())
                .flatten()
            else {
                return (None, None);
            };
            if running.replace(session).is_some() {
                return (None, None);
            }
        } else if let Some(value) = line.strip_prefix("Process exited with code ") {
            let Ok(code) = value.parse::<i64>() else {
                return (None, None);
            };
            if exit.replace(code).is_some() {
                return (None, None);
            }
        } else if let Some(value) = line.strip_prefix("Root process exited with code ") {
            let Ok(code) = value.parse::<i64>() else {
                return (None, None);
            };
            if root_exit.replace(code).is_some() {
                return (None, None);
            }
        } else if line.starts_with("Native completion: ") {
            // Validated and bound separately; never authority by itself.
            continue;
        } else if !["Chunk ID: ", "Wall time: ", "Original token count: "]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            return (None, None);
        }
    }
    // A root status is not completion of its supervised descendants. It is
    // meaningful only alongside the still-addressable original Native session.
    if root_exit.is_some() && (running.is_none() || exit.is_some()) {
        return (None, None);
    }
    (
        running,
        running
            .is_none()
            .then_some(exit)
            .flatten()
            .filter(|code| confirmed_exit_code(*code)),
    )
}

#[derive(Default)]
pub(crate) struct ProcessObservation {
    pub running: Option<u64>,
    pub exit_code: Option<i64>,
    pub terminal: bool,
    pub not_spawned: bool,
    pub proof: Option<completion::Proof>,
    pub completion_protocol: bool,
}
pub(crate) fn process_observation(
    value: &serde_json::Value,
    call_id: &str,
    session: Option<u64>,
) -> ProcessObservation {
    // These fields are present on some canonical projections and absent on
    // other already authenticated transport envelopes. Never accept a
    // conflicting explicit identity/type/name merely because its text parses.
    if value
        .get("call_id")
        .is_some_and(|id| id.as_str() != Some(call_id))
        || value
            .get("id")
            .is_some_and(|id| id.as_str() != Some(call_id))
        || value
            .get("type")
            .is_some_and(|kind| kind != "function_call_output")
        || value.get("name").is_some_and(|name| {
            !matches!(
                name.as_str(),
                Some(
                    "functions.exec_command"
                        | "functions.write_stdin"
                        | "exec_command"
                        | "write_stdin"
                )
            )
        })
    {
        return ProcessObservation {
            completion_protocol: true,
            ..ProcessObservation::default()
        };
    }
    let (running, legacy_exit) = legacy_process_result(value);
    if session.is_some() && running.is_some() && session != running {
        return ProcessObservation {
            completion_protocol: true,
            ..ProcessObservation::default()
        };
    }
    match completion::result_proof(value) {
        completion::Presence::Absent => ProcessObservation {
            running,
            exit_code: legacy_exit,
            terminal: false,
            ..ProcessObservation::default()
        },
        completion::Presence::Unconfirmed => ProcessObservation {
            running,
            completion_protocol: true,
            ..ProcessObservation::default()
        },
        completion::Presence::Invalid => ProcessObservation {
            completion_protocol: true,
            ..ProcessObservation::default()
        },
        completion::Presence::Present(proof) => {
            let rejected_current_invocation = proof.not_started() && proof.call_id == call_id;
            if !(proof.belongs_to(call_id, session) || rejected_current_invocation)
                || running.is_some_and(|id| proof.session_id != Some(id))
                || legacy_exit.is_some_and(|code| {
                    !proof.wait_completed() && proof.running().is_none()
                        || proof.waited().is_some_and(|actual| actual != code)
                })
                || proof.not_started() && (running.is_some() || legacy_exit.is_some())
                || proof.running().is_some() && session != proof.running()
            {
                return ProcessObservation {
                    completion_protocol: true,
                    ..ProcessObservation::default()
                };
            }
            ProcessObservation {
                running: if proof.wait_completed() {
                    None
                } else {
                    proof.running().or(running)
                },
                exit_code: proof.waited(),
                terminal: proof.process_tree_completed(),
                not_spawned: proof.not_started(),
                proof: Some(proof),
                completion_protocol: true,
            }
        }
    }
}
/// Unbound callers can inspect a running session, but typed completion always
/// requires the exact original call/continuation at the settlement site.
pub(crate) fn process_result(value: &serde_json::Value) -> (Option<u64>, Option<i64>) {
    let status = legacy_process_result(value);
    match completion::result_proof(value) {
        completion::Presence::Absent => status,
        completion::Presence::Unconfirmed => (status.0, None),
        _ => (None, None),
    }
}

#[cfg(test)]
fn process_status(value: &serde_json::Value) -> (Option<u64>, bool) {
    let (running, exit) = process_result(value);
    (running, exit.is_some())
}

#[cfg(test)]
fn verified_exec_not_spawned(value: &serde_json::Value) -> bool {
    value
        .get("call_id")
        .or_else(|| value.get("id"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|id| process_observation(value, id, None).not_spawned)
}

impl Guard {
    /// Observe the actual process-shared reservations without asserting that a
    /// missing Provider mirror or a departed process proves effect completion.
    pub(crate) fn shared_status(
        private_test_home: Option<&Path>,
        owned_lanes: &[&str],
        lane_owners: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, String> {
        let home = match private_test_home {
            Some(home) => home.to_path_buf(),
            None => PathBuf::from(
                std::env::var_os("PREFIX")
                    .ok_or_else(|| "native_resource_prefix_unavailable".to_owned())?,
            )
            .join("var/tmp/codex-native-effects"),
        };
        let state = Ledger::snapshot(&home)?;
        let rows = state.claims.iter().take(128).map(|(id, entry)| {
            let owner_work = lane_owners.get(&entry.thread).map(String::as_str)
                .or_else(|| owned_lanes.contains(&entry.thread.as_str()).then_some(entry.thread.as_str()));
            let owned = owned_lanes.contains(&entry.thread.as_str()) || owner_work.is_some_and(|id| owned_lanes.contains(&id));
            let owner_state = match entry.owner.as_ref() {
                Some(owner) => match process_identity(owner.pid) {
                    Ok((start, exited)) if start != owner.start || exited => "departed",
                    Ok(_) => "live",
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => "departed",
                    Err(_) => "unverified",
                },
                None => "unrecorded",
            };
            let effect = entry.effect.claim_scope();
            serde_json::json!({
                "claim":format!("{:x}", Sha256::digest(id.as_bytes())),
                "owner_work":owner_work, "owned":owned,
                "resource_kind":match effect { Effect::Read=>"read", Effect::Paths(_)=>"bounded", Effect::Opaque=>"opaque" },
                "scope":match effect { Effect::Opaque=>"unknown_global", _=>"verified_resources" },
                "scope_proof":if &entry.effect != effect { "unverified_legacy_or_unknown" } else { "current" },
                "process_session":if owned { entry.session } else { None },
                "owner_process_state":owner_state,
                "pending_result":true,
                "settlement":"requires_original_native_result_or_confirmed_non_dispatch"
            })
        }).collect::<Vec<_>>();
        Ok(
            serde_json::json!({"available":true,"active":rows,"unconfirmed_effects":state.claims.len(),"truncated":state.claims.len()>128}),
        )
    }

    /// Read-only trigger for a confirmed stopped work whose shared reservation
    /// outlived its Provider mirror. Never treat a live process session as orphaned.
    #[cfg(test)]
    pub(crate) fn has_stopped_unexecuted(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
    ) -> Result<bool, String> {
        let ledger = match private_test_home {
            Some(home) => Ledger::at(cwd, home)?,
            None => Ledger::new(cwd)?,
        };
        ledger.transact(|state| {
            Ok((
                state
                    .claims
                    .values()
                    .any(|held| held.thread == thread && held.session.is_none()),
                false,
            ))
        })
    }

    /// Reconcile only a Provider call that its authoritative journal still
    /// marks as *not offered*. No upstream handler may have received that
    /// call. A crash between the shared claim and the offer journal commit can
    /// therefore leave an orphaned reservation that is safe to replace.
    /// This must never be used for a previously offered or uncertain call.
    pub(crate) fn reconcile_unoffered(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
    ) -> Result<(), String> {
        Self::clear_unoffered(cwd, private_test_home, thread, call_id, tool, input, false)
    }

    /// A known failed journal commit cannot have delivered the tool. It may
    /// release its own same-process reservation immediately, but may never
    /// revoke a different live process's unresolved claim.
    pub(crate) fn abort_unoffered(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
    ) -> Result<(), String> {
        Self::clear_unoffered(cwd, private_test_home, thread, call_id, tool, input, true)
    }

    fn clear_unoffered(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        abort_current: bool,
    ) -> Result<(), String> {
        let ledger = match private_test_home {
            Some(home) => Ledger::at(cwd, home)?,
            None => Ledger::new(cwd)?,
        };
        let cwd = fs::canonicalize(cwd).map_err(|_| "native_workspace_unavailable".to_owned())?;
        let identity = format!("{thread}:{call_id}");
        let expected = fingerprint(tool, input, &cwd);
        let current = ProcessOwner::current()?;
        ledger.transact(|state| {
            let Some(held) = state.claims.get(&identity) else {
                return Ok(((), false));
            };
            if held.fingerprint != expected || held.thread != thread {
                return Err("native_effect_identity_changed".into());
            }
            if held.session.is_some() {
                return Err("native_effect_duplicate_unconfirmed".into());
            }
            if !held
                .owner
                .as_ref()
                .is_some_and(|owner| owner.departed() || (abort_current && *owner == current))
            {
                return Err("native_effect_duplicate_unconfirmed".into());
            }
            state.claims.remove(&identity);
            Ok(((), true))
        })
    }

    /// Provider Direct/AppHost offers reserve the very same OS-backed
    /// resources as other Provider-mediated routes. Provider keeps its
    /// original journal as the authoritative call/receipt owner.
    pub(crate) fn offer_external(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
    ) -> Result<(), String> {
        let effect = native_effect::classify(tool, input, Some(cwd));
        Self::offer_external_with_effect(
            cwd,
            private_test_home,
            thread,
            call_id,
            tool,
            input,
            effect,
        )
    }

    /// The broker derives the effect once from its Native-bound environment.
    /// Shared arbitration consumes that same proof instead of reclassifying
    /// against the Provider process environment or a second policy.
    pub(crate) fn offer_external_with_effect(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        effect: Effect,
    ) -> Result<(), String> {
        Self::offer_external_with_continuation(
            cwd,
            private_test_home,
            thread,
            call_id,
            tool,
            input,
            Admission {
                effect,
                hooks_absent: false,
            },
        )
    }

    pub(crate) fn offer_external_with_continuation(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        admission: Admission,
    ) -> Result<(), String> {
        // In-process discovery/metadata has no execution dependency. Shell
        // reads instead retain their Path inputs through the same ledger as
        // bounded writers, including when they only run the pwd builtin.
        if admission.effect == Effect::Read {
            return Ok(());
        }
        let ledger = match private_test_home {
            Some(home) => Ledger::at(cwd, home)?,
            None => Ledger::new(cwd)?,
        };
        Self::begin_effect_with_continuation(cwd, thread, call_id, tool, input, ledger, admission)?;
        // No Drop-based release: an offered call may run without this process
        // and is settled only against the original Provider result receipt.
        Ok(())
    }

    pub(crate) fn settle_external(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        result: &serde_json::Value,
    ) -> Result<(), String> {
        // This path reconciles an original reserved (or legacy-unknown) offer.
        // Current tool classification cannot erase an older exact reservation.
        // Known unreserved reads are skipped by the Broker's durable offer fact.
        let ledger = match private_test_home {
            Some(home) => Ledger::at(cwd, home)?,
            None => Ledger::new(cwd)?,
        };
        let cwd = fs::canonicalize(cwd).map_err(|_| "native_workspace_unavailable".to_owned())?;
        let identity = format!("{thread}:{call_id}");
        let fingerprint = fingerprint(tool, input, &cwd);
        let original_session = continuation(tool, input);
        ledger.transact(|state| {
            if let Some(record) = state.claims.get(&identity) {
                if record.fingerprint != fingerprint || record.thread != thread {
                    return Err("native_effect_identity_changed".into());
                }
            } else if !original_session.is_some_and(|session| {
                state
                    .claims
                    .values()
                    .any(|held| held.thread == thread && held.session == Some(session))
            }) {
                // A previously observed settled/read call is idempotent.
                return Ok(((), false));
            }
            Ok(((), false))
        })?;
        let mut observed = Self {
            ledger,
            identity,
            thread: thread.into(),
            tool: tool.into(),
            continuation: original_session,
        };
        observed.observed(Some(result))
    }

    /// AppHost transport retains the work-bound tool identity and verified
    /// session but not the original input string in its public receipt. Do
    /// not persist commands/secrets just to release a shared resource.
    pub(crate) fn settle_external_receipt(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        continued_session: Option<u64>,
        result: &serde_json::Value,
    ) -> Result<(), String> {
        Self::settle_external_receipt_for(
            cwd,
            private_test_home,
            thread,
            call_id,
            tool,
            continued_session,
            OriginalResult {
                call_id,
                value: result,
            },
        )
    }

    pub(crate) fn settle_external_receipt_for(
        cwd: &Path,
        private_test_home: Option<&Path>,
        thread: &str,
        call_id: &str,
        tool: &str,
        continued_session: Option<u64>,
        result: OriginalResult<'_>,
    ) -> Result<(), String> {
        // AppHost receipts have no original input. Only tools proven to be
        // reads even without input can bypass a broken shared writer ledger.
        if continued_session.is_none() && native_effect::classify(tool, "", None) == Effect::Read {
            return Ok(());
        }
        let ledger = match private_test_home {
            Some(home) => Ledger::at(cwd, home)?,
            None => Ledger::new(cwd)?,
        };
        let identity = format!("{thread}:{call_id}");
        let permitted = ledger.transact(|state| {
            let exists = state
                .claims
                .get(&identity)
                .is_some_and(|held| held.thread == thread);
            let owned_session = continued_session.is_some_and(|session| {
                state
                    .claims
                    .values()
                    .any(|held| held.thread == thread && held.session == Some(session))
            });
            Ok((exists || owned_session, false))
        })?;
        if !permitted {
            return Ok(());
        }
        let mut observed = Self {
            ledger,
            identity,
            thread: thread.into(),
            tool: tool.into(),
            continuation: continued_session,
        };
        observed.observed_for(Some(result.value), result.call_id)
    }

    #[cfg(test)]
    fn begin_with(
        cwd: &Path,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        ledger: Ledger,
    ) -> Result<Self, String> {
        Self::begin_effect(
            cwd,
            thread,
            call_id,
            tool,
            input,
            ledger,
            native_effect::classify_with_fixture(tool, input, Some(cwd)),
        )
    }

    #[cfg(test)]
    fn begin_effect(
        cwd: &Path,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        ledger: Ledger,
        effect: Effect,
    ) -> Result<Self, String> {
        Self::begin_effect_with_continuation(
            cwd,
            thread,
            call_id,
            tool,
            input,
            ledger,
            Admission {
                effect,
                hooks_absent: false,
            },
        )
    }

    fn begin_effect_with_continuation(
        cwd: &Path,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        ledger: Ledger,
        admission: Admission,
    ) -> Result<Self, String> {
        let Admission {
            mut effect,
            hooks_absent,
        } = admission;
        if thread.is_empty() || call_id.is_empty() || thread.len() + call_id.len() > 512 {
            return Err("native_effect_identity_invalid".to_owned());
        }
        let workspace =
            fs::canonicalize(cwd).map_err(|_| "native_workspace_unavailable".to_owned())?;
        let identity = format!("{thread}:{call_id}");
        let existing_session = continuation(tool, input);
        let fingerprint = fingerprint(tool, input, &workspace);
        let owner = ProcessOwner::current()?;
        ledger.transact(|state| {
            if let Some(existing) = state.claims.get(&identity) {
                return Err(if existing.fingerprint == fingerprint {
                    "native_effect_duplicate_unconfirmed"
                } else {
                    "native_effect_identity_changed"
                }
                .to_owned());
            }
            let mut owners = state.claims.values().filter(|held| {
                existing_session.is_some()
                    && held.thread == thread
                    && held.session == existing_session
            });
            let original = owners.next();
            let continuing_owned = original.is_some();
            let inherited_poll = original.is_some_and(|held| {
                native_effect::empty_session_poll(tool, input) == existing_session
                    && owners.next().is_none()
                    && matches!(held.effect.claim_scope(), Effect::Paths(_))
                    && held.effect == effect
            });
            // The Broker supplied current hook proof; recheck the exact
            // original scope/session under the shared lock. Never invent a
            // fresh path scope for an unknown or ambiguous continuation.
            if existing_session.is_some() && matches!(effect, Effect::Paths(_)) && !inherited_poll {
                effect = Effect::Opaque;
            }
            if state.claims.values().any(|held| {
                native_effect::conflicts(effect.admission_scope(), held.effect.claim_scope())
                    && !(continuing_owned
                        && held.thread == thread
                        && held.session == existing_session)
            }) {
                return Err("native_resource_busy".to_owned());
            }
            if !inherited_poll {
                // Outputs may already have changed since the original launch.
                // A pure poll does not start that command a second time.
                native_effect::can_offer(&effect, &workspace).map_err(str::to_owned)?;
            }
            if continuing_owned {
                if inherited_poll || effect == Effect::Read {
                    return Ok((true, false));
                }
                // Widen under the existing flock before dispatch. Preserve
                // the original fingerprint and keep this invocation separate
                // from an unrelated original process completion.
                for held in state
                    .claims
                    .values_mut()
                    .filter(|held| held.thread == thread && held.session == existing_session)
                {
                    held.process_hooks_absent |=
                        matches!(held.effect.claim_scope(), Effect::Paths(_));
                    held.effect = Effect::Opaque;
                }
                state.claims.insert(
                    identity.clone(),
                    Entry {
                        thread: thread.into(),
                        fingerprint,
                        effect: Effect::Opaque,
                        session: None,
                        owner: Some(owner),
                        process_hooks_absent: hooks_absent,
                        continuation_effect: Some(ContinuationEffect {
                            session: existing_session.unwrap(),
                            hooks_absent,
                        }),
                    },
                );
                Ok((true, true))
            } else {
                state.claims.insert(
                    identity.clone(),
                    Entry {
                        thread: thread.into(),
                        fingerprint,
                        effect,
                        session: existing_session,
                        owner: Some(owner),
                        continuation_effect: None,
                        process_hooks_absent: hooks_absent,
                    },
                );
                Ok((false, true))
            }
        })?;
        Ok(Self {
            ledger,
            identity,
            thread: thread.into(),
            tool: tool.into(),
            continuation: existing_session,
        })
    }

    /// Only an actual handler result may settle a claim. A dropped future
    /// or an absent receipt never proves completion, including apply_patch.
    pub(crate) fn observed(&mut self, result: Option<&serde_json::Value>) -> Result<(), String> {
        let call_id = self
            .identity
            .strip_prefix(&format!("{}:", self.thread))
            .ok_or_else(|| "native_effect_identity_changed".to_owned())?
            .to_owned();
        self.observed_for(result, &call_id)
    }

    fn observed_for(
        &mut self,
        result: Option<&serde_json::Value>,
        call_id: &str,
    ) -> Result<(), String> {
        let Some(result) = result else {
            return Ok(());
        };
        let observation = process_observation(result, call_id, self.continuation);
        let running = observation.running;
        let exited = observation.terminal;
        let not_spawned = native_effect::process_tool(&self.tool) == Some("exec_command")
            && observation.not_spawned;
        let session = self.continuation;
        self.ledger.transact(|state| {
            if session.is_some() && observation.not_spawned {
                // The current continuation never reached its hooks/backend.
                // This proves nothing about the already running original call.
                return Ok(((), state.claims.remove(&self.identity).is_some()));
            }
            if observation.proof.as_ref().is_some_and(|proof| {
                state
                    .claims
                    .get(&self.identity)
                    .and_then(|held| held.session)
                    .is_some_and(|session| proof.session_id != Some(session))
            }) {
                return Err("native_effect_identity_changed".into());
            }
            if let Some(pending) = state
                .claims
                .get(&self.identity)
                .and_then(|held| held.continuation_effect.as_ref())
            {
                if session != Some(pending.session) || !pending.settled_by(&observation) {
                    return Ok(((), false));
                }
                state.claims.remove(&self.identity);
                if !exited {
                    // The invocation returned with no hooks, but its original
                    // process remains alive under the monotonically wide claim.
                    return Ok(((), true));
                }
            }
            if let Some(session) = session {
                if exited {
                    state.claims.retain(|_, held| {
                        !(held.thread == self.thread && held.session == Some(session))
                    });
                    state.claims.remove(&self.identity);
                    return Ok(((), true));
                }
                return Ok(((), false));
            }
            if native_effect::process_tool(&self.tool) == Some("exec_command")
                && running.is_some()
                && !exited
                && !not_spawned
            {
                if let Some(claim) = state.claims.get_mut(&self.identity) {
                    claim.session = running;
                }
                return Ok(((), true));
            }
            // An exec result without an identifiable exit or live session is
            // unresolved; only unambiguous terminal evidence frees that claim.
            if native_effect::process_tool(&self.tool) == Some("exec_command")
                && !exited
                && !not_spawned
            {
                return Ok(((), false));
            }
            Ok(((), state.claims.remove(&self.identity).is_some()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn waited(call: &str, session: Option<u64>, exit: i64, output: &str) -> serde_json::Value {
        let proof = serde_json::json!({"schema":"codex-native-completion-v1","call_id":call,
            "session_id":session,"outcome":"process_tree_completed","exit_code":exit,"hooks":"absent"});
        serde_json::json!({"type":"function_call_output","call_id":call,
            "output":format!("Native completion: {proof}\nProcess exited with code {exit}\nOutput:\n{output}")})
    }
    #[test]
    fn parent_wait_never_releases_bounded_or_opaque_process_scope() {
        for tool in ["exec_command", "functions.exec_command"] {
            for bounded in [false, true] {
                for (outcome, code, closes) in [
                    ("wait_completed", Some(0), false),
                    ("wait_completed", Some(7), false),
                    ("wait_completed", None, false),
                    ("process_tree_completed", Some(0), true),
                    ("process_tree_completed", Some(7), true),
                    ("process_tree_completed", None, true),
                ] {
                    let cwd = root();
                    let home = root();
                    fs::write(cwd.join("a.c"), "int main(void) { return 0; }\n").unwrap();
                    let facts = native_effect::compiler_fixture(&cwd);
                    let input = if bounded {
                        serde_json::json!({"cmd":"clang -O0 a.c -o a.out","workdir":cwd,"login":false}).to_string()
                    } else {
                        r#"{"cmd":"owned opaque process"}"#.into()
                    };
                    let effect = native_effect::classify_with_execution(
                        tool,
                        &input,
                        Some(&cwd),
                        Some(&facts),
                    );
                    assert_eq!(matches!(effect, Effect::Paths(_)), bounded);
                    let ledger = Ledger::at(&cwd, &home).unwrap();
                    let mut claim = Guard::begin_effect_with_continuation(
                        &cwd,
                        "owner",
                        "original",
                        tool,
                        &input,
                        ledger.clone(),
                        Admission {
                            effect,
                            hooks_absent: true,
                        },
                    )
                    .unwrap();
                    let before = ledger.read_state().unwrap().claims["owner:original"]
                        .fingerprint
                        .clone();
                    let proof = serde_json::json!({"schema":"codex-native-completion-v1","call_id":"original",
                        "session_id":null,"outcome":outcome,"exit_code":code,"hooks":"absent"});
                    let result = serde_json::json!({"type":"function_call_output","call_id":"original",
                        "output":format!("Native completion: {proof}\nProcess exited with code {}\nOutput:\nimmutable", code.unwrap_or(1))});
                    claim.observed(Some(&result)).unwrap();
                    let state = ledger.read_state().unwrap();
                    assert_eq!(
                        state.claims.is_empty(),
                        closes,
                        "{tool}/{bounded}/{outcome}/{code:?}"
                    );
                    if !closes {
                        assert_eq!(state.claims["owner:original"].fingerprint, before);
                        claim.observed(Some(&serde_json::json!({"output":"Process exited with code 0\nOutput:\n"}))).unwrap();
                        assert!(ledger
                            .read_state()
                            .unwrap()
                            .claims
                            .contains_key("owner:original"));
                    }
                    fs::remove_dir_all(cwd).unwrap();
                    fs::remove_dir_all(home).unwrap();
                }
            }
        }
    }

    #[test]
    fn session_running_after_parent_exit_settles_only_current_poll() {
        let cwd = root();
        let home = root();
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut original = Guard::begin_with(
            &cwd,
            "owner",
            "start",
            "functions.exec_command",
            r#"{"cmd":"owned supervised tree"}"#,
            ledger.clone(),
        )
        .unwrap();
        original.observed(Some(&serde_json::json!({"output":"Native completion: null\nProcess running with session ID 51\nOutput:\n"}))).unwrap();
        let mut poll = Guard::begin_with(
            &cwd,
            "owner",
            "poll",
            "functions.write_stdin",
            r#"{"session_id":51,"chars":"input"}"#,
            ledger.clone(),
        )
        .unwrap();
        let proof = serde_json::json!({"schema":"codex-native-completion-v1","call_id":"poll",
            "session_id":51,"outcome":"session_running","exit_code":null,"hooks":"absent"});
        let result = serde_json::json!({"type":"function_call_output","call_id":"poll",
            "output":format!("Native completion: {proof}\nProcess exited with code 0\nProcess running with session ID 51\nOutput:\noriginal parent output")});
        let observed = process_observation(&result, "poll", Some(51));
        assert_eq!(observed.running, Some(51));
        assert!(!observed.terminal);
        let projected = serde_json::json!({"call_id":"poll","output":
            serde_json::json!({"native_completion":proof,"exit_code":0,"session_id":51,
                "wall_time_seconds":0.1,"output":"original parent output"}).to_string()});
        let observed_projection = process_observation(&projected, "poll", Some(51));
        assert_eq!(observed_projection.running, Some(51));
        assert!(!observed_projection.terminal);
        let mut scope_pending = result.clone();
        scope_pending["output"] = serde_json::json!(result["output"].as_str().unwrap().replace(
            "Process running with session ID ",
            "Process scope remains active with session ID "
        ));
        let observed_scope = process_observation(&scope_pending, "poll", Some(51));
        assert_eq!(observed_scope.running, Some(51));
        assert!(!observed_scope.terminal);
        let mut unknown_scope = scope_pending.clone();
        unknown_scope["output"] = serde_json::json!(
            "Native completion: null\nProcess exited with code 0\nProcess scope remains active with session ID 51\nOutput:\n"
        );
        let unknown = process_observation(&unknown_scope, "poll", Some(51));
        assert_eq!(unknown.running, Some(51));
        assert!(!unknown.terminal && unknown.proof.is_none());
        poll.observed(Some(&scope_pending)).unwrap();
        let state = ledger.read_state().unwrap();
        assert_eq!(state.claims.len(), 1);
        assert_eq!(state.claims["owner:start"].session, Some(51));
        assert_eq!(state.claims["owner:start"].effect, Effect::Opaque);
        assert!(process_observation(&result, "other", Some(51))
            .proof
            .is_none());
        assert!(process_observation(&result, "poll", Some(52))
            .proof
            .is_none());
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn pending_root_status_preserves_session_without_terminal_authority() {
        let body = "Root process exited with code 7\nProcess running with session ID 51\nOutput:\nProcess exited with code 0\nProcess running with session ID 999";
        let result = serde_json::json!({"type":"function_call_output","call_id":"original",
            "output":format!("Native completion: null\n{body}")});
        assert_eq!(legacy_process_result(&result), (Some(51), None));
        let pending = process_observation(&result, "original", None);
        assert_eq!(pending.running, Some(51));
        assert!(!pending.terminal && pending.exit_code.is_none() && pending.proof.is_none());
        let proof = serde_json::json!({"schema":"codex-native-completion-v1","call_id":"poll",
            "session_id":51,"outcome":"session_running","exit_code":null,"hooks":"absent"});
        let poll = serde_json::json!({"type":"function_call_output","call_id":"poll",
            "output":format!("Native completion: {proof}\n{body}")});
        let running = process_observation(&poll, "poll", Some(51));
        assert_eq!(running.running, Some(51));
        assert!(!running.terminal && running.exit_code.is_none());
        assert!(running.proof.is_some());
        assert!(process_observation(&poll, "poll", Some(52)).proof.is_none());
        for header in [
            "Root process exited with code 7",
            "Root process exited with code bad\nProcess running with session ID 51",
            "Root process exited with code 7\nRoot process exited with code 8\nProcess running with session ID 51",
            "Root process exited with code 7\nProcess exited with code 0\nProcess running with session ID 51",
        ] {
            let invalid = serde_json::json!({"call_id":"original",
                "output":format!("Native completion: null\n{header}\nOutput:\n")});
            assert_eq!(legacy_process_result(&invalid), (None, None), "{header}");
            let observed = process_observation(&invalid, "original", None);
            assert!(observed.running.is_none() && !observed.terminal && observed.proof.is_none());
        }
    }

    #[test]
    fn completion_safety_new_unknown_cannot_downgrade_to_legacy_exit() {
        for native in [
            serde_json::Value::Null,
            serde_json::json!({"schema":"unknown"}),
        ] {
            let result = serde_json::json!({"output":format!(
                "Native completion: {native}\nProcess exited with code 0\nOutput:\n")});
            let observed = process_observation(&result, "original", None);
            assert!(observed.completion_protocol);
            assert_eq!(observed.exit_code, None);
            assert_eq!(process_result(&result), (None, None));
        }
        let running = serde_json::json!({"output":
            "Native completion: null\nProcess running with session ID 41\nOutput:\n"});
        let observed = process_observation(&running, "original", None);
        assert!(observed.completion_protocol);
        assert_eq!(observed.running, Some(41));
        assert_eq!(observed.exit_code, None);
    }
    #[test]
    fn completion_safety_synthetic_status_never_releases_shared_claim() {
        for status in [
            "Process exited with code -1",
            "Process exited with code 124",
            "Process exited with signal 9",
            "Process exited with code unknown",
        ] {
            let cwd = root();
            let home = root();
            fs::write(cwd.join("a"), "old\n").unwrap();
            let mut claim = Guard::begin_with(
                &cwd,
                "owner",
                "original",
                "functions.exec_command",
                r#"{"cmd":"unknown"}"#,
                Ledger::at(&cwd, &home).unwrap(),
            )
            .unwrap();
            claim
                .observed(Some(
                    &serde_json::json!({"output":format!("{status}\nOutput:\n")}),
                ))
                .unwrap();
            assert_eq!(
                apply(&cwd, &home, "other", "conflicting", "a")
                    .err()
                    .as_deref(),
                Some("native_resource_busy"),
                "{status} is not a confirmed wait result"
            );
            fs::remove_dir_all(cwd).unwrap();
            fs::remove_dir_all(home).unwrap();
        }
    }

    #[test]
    fn completion_safety_rejected_startup_is_not_non_dispatch_evidence() {
        let value = serde_json::json!({"type":"function_call_output","call_id":"original",
            "output":"exec_command failed: CreateProcess { message: \"Rejected(\\\"backend failure after spawn\\\")\" }"});
        assert!(!verified_exec_not_spawned(&value));
        let projected = serde_json::json!({"id":"original","name":"functions.exec_command","output":value["output"]});
        assert!(!verified_exec_not_spawned(&projected));
    }

    #[test]
    fn unreaped_process_exit_is_not_an_active_unoffered_owner() {
        // The closing parenthesis belongs to comm, even if comm contains ')'.
        let prefix = format!("12 (shell) worker) Z {} 742", vec!["0"; 18].join(" "));
        assert_eq!(parse_process_identity(&prefix).unwrap(), (742, true));
        assert_eq!(
            parse_process_identity(&prefix.replacen(" Z ", " X ", 1)).unwrap(),
            (742, true)
        );
        assert_eq!(
            parse_process_identity(&prefix.replacen(" Z ", " S ", 1)).unwrap(),
            (742, false)
        );
        assert!(parse_process_identity("12 (malformed) Z").is_err());
    }

    #[test]
    fn rejected_before_spawn_releases_opaque_writer_but_stdout_cannot_impersonate_it() {
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        let proof = serde_json::json!({"schema":"codex-native-completion-v1","call_id":"review-deny",
            "session_id":null,"outcome":"pre_backend_rejected","exit_code":null,"hooks":"absent"});
        let denial = serde_json::json!({"type":"function_call_output","call_id":"review-deny",
            "output":format!("Native completion: {proof}\nOutput:\ninitial approval denied")});
        assert!(verified_exec_not_spawned(&denial));
        let mut projected = serde_json::json!({"id":"review-deny", "name":"functions.exec_command",
            "output":denial["output"]});
        assert!(verified_exec_not_spawned(&projected));
        projected["type"] = serde_json::json!("unrecognized_output");
        assert!(!verified_exec_not_spawned(&projected));
        projected.as_object_mut().unwrap().remove("type");
        projected["name"] = serde_json::json!("other_tool");
        assert!(!verified_exec_not_spawned(&projected));
        projected["name"] = serde_json::json!("functions.exec_command");
        projected.as_object_mut().unwrap().remove("id");
        assert!(!verified_exec_not_spawned(&projected));
        let output_as_stdout = serde_json::json!({
            "type":"function_call_output",
            "output": format!("Chunk ID: abc\nProcess running with session ID 81\nOutput:\n{}",denial["output"].as_str().unwrap())
        });
        assert!(!verified_exec_not_spawned(&output_as_stdout));
        let projected_stdout = serde_json::json!({"id":"review-deny", "name":"functions.exec_command",
            "output":output_as_stdout["output"]});
        assert!(!verified_exec_not_spawned(&projected_stdout));
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut running = Guard::begin_with(
            &cwd,
            "one",
            "exec",
            "functions.exec_command",
            r#"{"cmd":"printf unknown > a"}"#,
            ledger.clone(),
        )
        .unwrap();
        running.observed(Some(&output_as_stdout)).unwrap();
        assert_eq!(
            apply(&cwd, &home, "two", "patch", "a").err().as_deref(),
            Some("native_resource_busy")
        );
        let mut stopped = Guard::begin_with(
            &cwd,
            "one",
            "poll",
            "functions.write_stdin",
            r#"{"session_id":81,"chars":""}"#,
            ledger.clone(),
        )
        .unwrap();
        stopped
            .observed(Some(
                &serde_json::json!({"output":format!("Native completion: {}\nOutput:\n",
                    serde_json::json!({"schema":"codex-native-completion-v1","call_id":"poll",
                    "session_id":81,"outcome":"process_tree_completed","exit_code":0,"hooks":"absent"}))}),
            ))
            .unwrap();
        let mut denied = Guard::begin_with(
            &cwd,
            "one",
            "review-deny",
            "functions.exec_command",
            r#"{"cmd":"printf denied > a"}"#,
            ledger,
        )
        .unwrap();
        assert_eq!(
            apply(&cwd, &home, "two", "before", "a").err().as_deref(),
            Some("native_resource_busy")
        );
        denied.observed(Some(&denial)).unwrap();
        assert!(apply(&cwd, &home, "two", "after", "a").is_ok());
    }

    #[test]
    fn stopped_claim_reconciliation_retains_an_observed_live_session() {
        let cwd = root();
        let home = root();
        let mut executing = Guard::begin_with(
            &cwd,
            "running-work",
            "call",
            "functions.exec_command",
            r#"{"cmd":"sleep 30"}"#,
            Ledger::at(&cwd, &home).unwrap(),
        )
        .unwrap();
        executing
            .observed(Some(&serde_json::json!({
                "output":"Process running with session ID 83\nOutput:\n"
            })))
            .unwrap();
        assert!(!Guard::has_stopped_unexecuted(&cwd, Some(&home), "running-work").unwrap());
        executing
            .observed(Some(&serde_json::json!({
                "error":"native_call_not_dispatched","executed":false
            })))
            .unwrap();
        assert_eq!(
            apply(&cwd, &home, "other-work", "blocked", "any")
                .err()
                .as_deref(),
            Some("native_resource_busy")
        );
    }

    #[test]
    fn zombie_owner_releases_only_a_verified_unoffered_claim() {
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        let input = "*** Begin Patch\n*** Update File: a\n@@\n-old\n+new\n*** End Patch\n";
        let mut child = std::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap();
        let observed = (0..100).find_map(|_| {
            let dead = process_identity(child.id()).ok().filter(|(_, dead)| *dead);
            if dead.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            dead
        });
        let start = observed.expect("child must become unreaped zombie").0;
        let thread = "orphaned";
        let call_id = "unoffered";
        let owner = ProcessOwner {
            pid: child.id(),
            start,
        };
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let root = fs::canonicalize(&cwd).unwrap();
        ledger
            .transact(|state| {
                state.claims.insert(
                    format!("{thread}:{call_id}"),
                    Entry {
                        thread: thread.into(),
                        fingerprint: fingerprint("functions.apply_patch", input, &root),
                        effect: native_effect::classify_with_fixture(
                            "functions.apply_patch",
                            input,
                            Some(&root),
                        ),
                        session: None,
                        owner: Some(owner),
                        continuation_effect: None,
                        process_hooks_absent: false,
                    },
                );
                Ok(((), true))
            })
            .unwrap();
        assert_eq!(
            apply(&cwd, &home, "another", "before", "a")
                .err()
                .as_deref(),
            Some("native_resource_busy")
        );
        Guard::reconcile_unoffered(
            &cwd,
            Some(&home),
            thread,
            call_id,
            "functions.apply_patch",
            input,
        )
        .unwrap();
        child.wait().unwrap();
        assert!(apply(&cwd, &home, "another", "after", "a").is_ok());
    }
    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "native-ledger-test-{}",
            crate::journal::random_id().unwrap()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        path
    }
    fn apply(
        cwd: &Path,
        home: &Path,
        thread: &str,
        id: &str,
        target: &str,
    ) -> Result<Guard, String> {
        let patch =
            format!("*** Begin Patch\n*** Update File: {target}\n@@\n-old\n+new\n*** End Patch\n");
        Guard::begin_with(
            cwd,
            thread,
            id,
            "functions.apply_patch",
            &patch,
            Ledger::at(cwd, home).unwrap(),
        )
    }
    #[test]
    fn distinct_effects_and_persisted_conflicts() {
        let cwd = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        fs::write(cwd.join("b"), "old\n").unwrap();
        let home = root();
        // Verify the actual process-shared state and deterministic resource
        // decisions, independent of Broker's process-local mutex.
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let a = native_effect::classify_with_fixture(
            "functions.apply_patch",
            "*** Begin Patch\n*** Update File: a\n@@\n-old\n+new\n*** End Patch\n",
            Some(&cwd),
        );
        let b = native_effect::classify_with_fixture(
            "functions.apply_patch",
            "*** Begin Patch\n*** Update File: b\n@@\n-old\n+new\n*** End Patch\n",
            Some(&cwd),
        );
        ledger
            .transact(|state| {
                state.claims.insert(
                    "one".into(),
                    Entry {
                        thread: "one".into(),
                        fingerprint: "a".into(),
                        effect: a.clone(),
                        session: None,
                        owner: Some(ProcessOwner::current().unwrap()),
                        continuation_effect: None,
                        process_hooks_absent: false,
                    },
                );
                Ok(((), true))
            })
            .unwrap();
        let shared = Ledger::at(&cwd, &home).unwrap();
        shared
            .transact(|state| {
                assert!(state
                    .claims
                    .values()
                    .any(|v| native_effect::conflicts(&v.effect, &a)));
                assert!(!state
                    .claims
                    .values()
                    .any(|v| native_effect::conflicts(&v.effect, &b)));
                Ok(((), false))
            })
            .unwrap();
    }

    #[test]
    fn legacy_claim_scope_is_opaque_without_rewriting_identity_or_original_receipt() {
        for mode in ["missing", "unknown", "read"] {
            let cwd = root();
            let home = root();
            fs::write(cwd.join("a"), "old\n").unwrap();
            fs::write(cwd.join("b"), "old\n").unwrap();
            let input = "*** Begin Patch\n*** Update File: a\n@@\n-old\n+new\n*** End Patch\n";
            let _held = apply(&cwd, &home, "original", "call", "a").unwrap();
            let path = Ledger::at(&cwd, &home)
                .unwrap()
                .directory
                .join("claims.json");
            let mut old: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let record = &mut old["claims"]["original:call"];
            if mode == "read" {
                record["effect"] = serde_json::json!("Read");
            } else {
                let files = record["effect"]["Paths"].as_array_mut().unwrap();
                for file in files {
                    if mode == "missing" {
                        file.as_object_mut().unwrap().remove("scope_schema");
                    } else {
                        file["scope_schema"] = serde_json::json!(99);
                    }
                }
            }
            let original = serde_json::to_vec(&old).unwrap();
            fs::write(&path, &original).unwrap();
            assert_eq!(
                apply(&cwd, &home, "independent", "b", "b").err().as_deref(),
                Some("native_resource_busy"),
                "{mode}"
            );
            assert_eq!(
                apply(&cwd, &home, "original", "call", "a").err().as_deref(),
                Some("native_effect_duplicate_unconfirmed")
            );
            assert_eq!(
                apply(&cwd, &home, "original", "call", "b").err().as_deref(),
                Some("native_effect_identity_changed")
            );
            let status =
                Guard::shared_status(Some(&home), &["original"], &BTreeMap::new()).unwrap();
            assert_eq!(status["active"][0]["resource_kind"], "opaque");
            assert_eq!(
                status["active"][0]["scope_proof"],
                "unverified_legacy_or_unknown"
            );
            assert_eq!(fs::read(&path).unwrap(), original);
            Guard::settle_external(
                &cwd,
                Some(&home),
                "original",
                "call",
                "functions.apply_patch",
                input,
                &serde_json::json!({"output":"original patch result"}),
            )
            .unwrap();
            assert_eq!(
                Guard::shared_status(Some(&home), &[], &BTreeMap::new()).unwrap()
                    ["unconfirmed_effects"],
                0
            );
            assert!(apply(&cwd, &home, "independent", "after", "b").is_ok());
            assert_eq!(fs::read_to_string(cwd.join("a")).unwrap(), "old\n");
            fs::remove_dir_all(cwd).unwrap();
            fs::remove_dir_all(home).unwrap();
        }
    }

    #[test]
    fn concurrent_independent_paths_and_unconfirmed_reservations() {
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        fs::write(cwd.join("b"), "old\n").unwrap();
        let mut first = apply(&cwd, &home, "one", "write", "a").unwrap();
        assert_eq!(
            apply(&cwd, &home, "two", "write", "a").err().as_deref(),
            Some("native_resource_busy")
        );
        let mut second = apply(&cwd, &home, "two", "other", "b").unwrap();
        first.observed(None).unwrap();
        assert_eq!(
            apply(&cwd, &home, "third", "unconfirmed", "a")
                .err()
                .as_deref(),
            Some("native_resource_busy"),
            "a missing apply_patch result cannot free an unfinished write"
        );
        // A completed patch handler result is a terminal observation.
        first
            .observed(Some(&serde_json::json!({"output":"patched"})))
            .unwrap();
        second
            .observed(Some(&serde_json::json!({"output":"patched"})))
            .unwrap();
        assert!(apply(&cwd, &home, "two", "again", "a").is_ok());
    }

    #[test]
    fn opaque_process_session_survives_until_observed_exit() {
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        let mut shell = Guard::begin_with(
            &cwd,
            "first",
            "exec",
            "functions.exec_command",
            r#"{"cmd":"printf output > a"}"#,
            Ledger::at(&cwd, &home).unwrap(),
        )
        .unwrap();
        shell
            .observed(Some(&serde_json::json!({
                "output":"Process running with session ID 83\nOutput:\nstart"
            })))
            .unwrap();
        assert_eq!(
            apply(&cwd, &home, "second", "patch", "a").err().as_deref(),
            Some("native_resource_busy")
        );
        let mut poll = Guard::begin_with(
            &cwd,
            "first",
            "poll",
            "functions.write_stdin",
            r#"{"session_id":83,"chars":""}"#,
            Ledger::at(&cwd, &home).unwrap(),
        )
        .unwrap();
        poll.observed(Some(&waited(
            "poll",
            Some(83),
            0,
            "Process running with session ID 83",
        )))
        .unwrap();
        assert!(apply(&cwd, &home, "second", "patch", "a").is_ok());
    }

    #[test]
    fn process_tree_completion_releases_only_original_call_and_session() {
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        let input = r#"{"cmd":"sleep 30; printf output > a"}"#;
        let mut running = Guard::begin_with(
            &cwd,
            "stopped-thread",
            "exec",
            "functions.exec_command",
            input,
            Ledger::at(&cwd, &home).unwrap(),
        )
        .unwrap();
        running
            .observed(Some(&serde_json::json!({
                "output":"Process running with session ID 83\nOutput:\n"
            })))
            .unwrap();
        let blocked = || apply(&cwd, &home, "other-thread", "patch", "a");
        assert_eq!(blocked().err().as_deref(), Some("native_resource_busy"));
        for (thread, call) in [("other-thread", "exec"), ("stopped-thread", "other-call")] {
            Guard::settle_external(
                &cwd,
                Some(&home),
                thread,
                call,
                "functions.exec_command",
                input,
                &waited(call, Some(83), 0, ""),
            )
            .unwrap();
            assert_eq!(blocked().err().as_deref(), Some("native_resource_busy"));
        }
        let parent = serde_json::json!({"schema":"codex-native-completion-v1","call_id":"exec",
            "session_id":83,"outcome":"wait_completed","exit_code":0,"hooks":"absent"});
        running
            .observed(Some(&serde_json::json!({"output":format!(
            "Native completion: {parent}\nProcess exited with code 0\nOutput:\n")})))
            .unwrap();
        assert_eq!(blocked().err().as_deref(), Some("native_resource_busy"));
        assert_eq!(
            running.observed(Some(&waited("exec", Some(84), 0, ""))),
            Err("native_effect_identity_changed".into())
        );
        assert_eq!(blocked().err().as_deref(), Some("native_resource_busy"));
        running
            .observed(Some(&waited("exec", Some(83), 0, "")))
            .unwrap();
        assert!(blocked().is_ok());
    }

    #[test]
    fn core_content_items_and_untrusted_stdout_do_not_forge_exit() {
        let item = serde_json::json!({"output":[{"type":"input_text",
            "text":"Chunk ID: 1\nWall time: 0.1 seconds\nProcess exited with code 0\nOutput:\nhi"}]});
        assert_eq!(process_status(&item), (None, true));
        let spoof = serde_json::json!({"output":[{"type":"input_text",
            "text":"Chunk ID: 1\nProcess running with session ID 9\nOutput:\nProcess exited with code 0"}]});
        assert_eq!(process_status(&spoof), (Some(9), false));
    }

    #[test]
    fn nc_read_remains_unblocked_during_write() {
        let cwd = root();
        let home = root();
        let execution = native_effect::compiler_fixture(&cwd);
        fs::write(cwd.join("a"), "old\n").unwrap();
        let _writing = apply(&cwd, &home, "writer", "patch", "a").unwrap();
        // Live file contents are not a pinned snapshot. Reading a remains
        // independent of its writer, while shell/PATH dependencies are held.
        let blocked_home = home.join("not-a-directory");
        fs::write(&blocked_home, "cannot create a journal beneath a file").unwrap();
        for (thread, call_id, input) in [
            ("reader", "cat", r#"{"cmd":"cat a","login":false}"#),
            ("reader-2", "pwd", r#"{"cmd":"pwd","login":false}"#),
        ] {
            let effect = native_effect::classify_with_execution(
                "functions.exec_command",
                input,
                Some(&cwd),
                Some(&execution),
            );
            assert!(matches!(effect, Effect::Paths(_)));
            Guard::offer_external_with_effect(
                &cwd,
                Some(&home),
                thread,
                call_id,
                "functions.exec_command",
                input,
                effect,
            )
            .unwrap();
        }
        // A broken writer ledger cannot block metadata/status discovery.
        Guard::offer_external(
            &cwd,
            Some(&blocked_home),
            "reader",
            "search",
            "tool_search",
            "{}",
        )
        .unwrap();
        Guard::settle_external_receipt(
            &cwd,
            Some(&blocked_home),
            "reader",
            "time",
            "tool_search",
            None,
            &serde_json::json!({"value":"read result"}),
        )
        .unwrap();
        assert_eq!(
            native_effect::classify_with_fixture(
                "functions.exec_command",
                r#"{"cmd":"cat a; touch b"}"#,
                Some(&cwd)
            ),
            Effect::Opaque
        );
        assert_eq!(
            native_effect::classify_with_fixture(
                "functions.exec_command",
                r#"{"cmd":"cat ../outside"}"#,
                Some(&cwd)
            ),
            Effect::Opaque
        );
    }

    #[test]
    fn cross_process_admission_reuses_same_physical_ledger() {
        if let (Ok(cwd), Ok(home)) = (
            std::env::var("NATIVE_CLAIMS_PROBE_CWD"),
            std::env::var("NATIVE_CLAIMS_PROBE_HOME"),
        ) {
            let result = apply(Path::new(&cwd), Path::new(&home), "child", "write", "a");
            std::process::exit(if result.is_ok() { 0 } else { 42 });
        }
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        let mut first = apply(&cwd, &home, "parent", "write", "a").unwrap();
        let probe = || {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "native_claims::tests::cross_process_admission_reuses_same_physical_ledger",
                ])
                .env("NATIVE_CLAIMS_PROBE_CWD", &cwd)
                .env("NATIVE_CLAIMS_PROBE_HOME", &home)
                .status()
                .unwrap()
                .code()
        };
        assert_eq!(probe(), Some(42));
        first
            .observed(Some(&serde_json::json!({"output":"patched"})))
            .unwrap();
        assert_eq!(probe(), Some(0));
    }

    #[test]
    fn shared_git_and_external_effects_across_distinct_roots() {
        let home = root();
        let first_root = root();
        let second_root = root();
        fs::write(first_root.join("a"), "old\n").unwrap();
        fs::write(second_root.join("b"), "old\n").unwrap();
        let first = apply(&first_root, &home, "first", "file-a", "a").unwrap();
        let mut other = apply(&second_root, &home, "second", "file-b", "b").unwrap();
        // Different files (including different Git worktrees) are genuinely
        // independent, even though all claims use one process-shared journal.
        assert_eq!(
            Ledger::at(&first_root, &home).unwrap().directory,
            Ledger::at(&second_root, &home).unwrap().directory
        );
        assert!(Guard::begin_with(
            &second_root,
            "second",
            "git",
            "functions.exec_command",
            r#"{"cmd":"git commit -m shared"}"#,
            Ledger::at(&second_root, &home).unwrap()
        )
        .is_err());
        other
            .observed(Some(&serde_json::json!({"output":"patched"})))
            .unwrap();
        drop(first);
        // Losing the first task's process/handle does not clear its effect.
        assert_eq!(
            Guard::begin_with(
                &second_root,
                "second",
                "git-again",
                "functions.exec_command",
                r#"{"cmd":"git commit -m shared"}"#,
                Ledger::at(&second_root, &home).unwrap()
            )
            .err()
            .as_deref(),
            Some("native_resource_busy")
        );
    }

    #[test]
    fn unrelated_known_file_effects_do_not_block_across_roots() {
        let home = root();
        let first_root = root();
        let second_root = root();
        fs::write(first_root.join("a"), "old\n").unwrap();
        fs::write(second_root.join("a"), "old\n").unwrap();
        let first = apply(&first_root, &home, "one", "update", "a").unwrap();
        let second = apply(&second_root, &home, "two", "update", "a").unwrap();
        assert_eq!(first.tool, "functions.apply_patch");
        assert_eq!(second.tool, "functions.apply_patch");
    }

    #[test]
    fn unoffered_reservation_is_reclaimed_but_running_process_is_not() {
        let cwd = root();
        let home = root();
        fs::write(cwd.join("a"), "old\n").unwrap();
        let input = "*** Begin Patch\n*** Update File: a\n@@\n-old\n+new\n*** End Patch\n";
        Guard::offer_external(
            &cwd,
            Some(&home),
            "direct",
            "one",
            "functions.apply_patch",
            input,
        )
        .unwrap();
        assert_eq!(
            apply(&cwd, &home, "second", "two", "a").err().as_deref(),
            Some("native_resource_busy")
        );
        assert_eq!(
            Guard::reconcile_unoffered(
                &cwd,
                Some(&home),
                "direct",
                "one",
                "functions.apply_patch",
                input,
            )
            .err()
            .as_deref(),
            Some("native_effect_duplicate_unconfirmed"),
            "same-process work cannot be mistaken for a crashed owner"
        );
        Ledger::at(&cwd, &home)
            .unwrap()
            .transact(|state| {
                state.claims.get_mut("direct:one").unwrap().owner = Some(ProcessOwner {
                    pid: std::process::id(),
                    start: 0,
                });
                Ok(((), true))
            })
            .unwrap();
        Guard::reconcile_unoffered(
            &cwd,
            Some(&home),
            "direct",
            "one",
            "functions.apply_patch",
            input,
        )
        .unwrap();
        let mut second = apply(&cwd, &home, "second", "two", "a").unwrap();
        second
            .observed(Some(&serde_json::json!({"output":"patched"})))
            .unwrap();

        let mut running = Guard::begin_with(
            &cwd,
            "third",
            "process",
            "functions.exec_command",
            r#"{"cmd":"sleep 30"}"#,
            Ledger::at(&cwd, &home).unwrap(),
        )
        .unwrap();
        running
            .observed(Some(&serde_json::json!({
                "output":"Process running with session ID 42\nOutput:\nstarted"
            })))
            .unwrap();
        assert_eq!(
            Guard::reconcile_unoffered(
                &cwd,
                Some(&home),
                "third",
                "process",
                "functions.exec_command",
                r#"{"cmd":"sleep 30"}"#,
            )
            .err()
            .as_deref(),
            Some("native_effect_duplicate_unconfirmed")
        );
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn unproved_continuation_widens_shared_scope_before_an_independent_offer() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for chars in ["", "input"] {
                let cwd = root();
                let home = root();
                let facts = native_effect::compiler_fixture(&cwd);
                let ledger = Ledger::at(&cwd, &home).unwrap();
                fs::write(cwd.join("a.c"), "int main(void){return 0;}").unwrap();
                fs::write(cwd.join("b.c"), "int main(void){return 0;}").unwrap();
                let input =
                    serde_json::json!({"cmd":"clang -O0 a.c -o a.out","workdir":cwd,"login":false})
                        .to_string();
                let scope = native_effect::classify_with_execution(
                    exec_tool,
                    &input,
                    Some(&cwd),
                    Some(&facts),
                );
                assert!(matches!(scope, Effect::Paths(_)));
                let mut original = Guard::begin_effect(
                    &cwd,
                    "a",
                    "start",
                    exec_tool,
                    &input,
                    ledger.clone(),
                    scope,
                )
                .unwrap();
                original
                    .observed(Some(
                        &serde_json::json!({"output":"Process running with session ID 51"}),
                    ))
                    .unwrap();
                let fingerprint = ledger.read_state().unwrap().claims["a:start"]
                    .fingerprint
                    .clone();
                let poll = serde_json::json!({"session_id":51,"chars":chars}).to_string();
                Guard::begin_effect(
                    &cwd,
                    "a",
                    "unproved",
                    poll_tool,
                    &poll,
                    ledger.clone(),
                    Effect::Opaque,
                )
                .unwrap();
                let state = ledger.read_state().unwrap();
                assert_eq!(
                    state.claims["a:start"].effect.claim_scope(),
                    &Effect::Opaque
                );
                assert_eq!(state.claims["a:start"].fingerprint, fingerprint);
                let other =
                    serde_json::json!({"cmd":"clang -O0 b.c -o b.out","workdir":cwd,"login":false})
                        .to_string();
                let scope = native_effect::classify_with_execution(
                    exec_tool,
                    &other,
                    Some(&cwd),
                    Some(&facts),
                );
                assert_eq!(
                    Guard::begin_effect(
                        &cwd,
                        "b",
                        "independent",
                        exec_tool,
                        &other,
                        Ledger::at(&cwd, &home).unwrap(),
                        scope
                    )
                    .err()
                    .as_deref(),
                    Some("native_resource_busy")
                );
                fs::remove_dir_all(cwd).unwrap();
                fs::remove_dir_all(home).unwrap();
            }
        }
    }

    #[test]
    fn running_continuation_settles_only_its_exact_invocation_and_null_does_not() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            for proof_present in [false, true] {
                let cwd = root();
                let home = root();
                let ledger = Ledger::at(&cwd, &home).unwrap();
                let mut original = Guard::begin_effect_with_continuation(
                    &cwd,
                    "a",
                    "start",
                    exec_tool,
                    r#"{"cmd":"owned interactive process"}"#,
                    ledger.clone(),
                    Admission {
                        effect: Effect::Opaque,
                        hooks_absent: true,
                    },
                )
                .unwrap();
                original.observed(Some(&serde_json::json!({"output":"Native completion: null\nProcess running with session ID 51\nOutput:\n"}))).unwrap();
                let mut pending = Guard::begin_effect_with_continuation(
                    &cwd,
                    "a",
                    "poll",
                    poll_tool,
                    r#"{"session_id":51,"chars":"input"}"#,
                    ledger.clone(),
                    Admission {
                        effect: Effect::Opaque,
                        hooks_absent: true,
                    },
                )
                .unwrap();
                let proof = if proof_present {
                    serde_json::json!({"schema":"codex-native-completion-v1","call_id":"poll",
                    "session_id":51,"outcome":"session_running","exit_code":null,"hooks":"absent"})
                } else {
                    serde_json::Value::Null
                };
                let reply = serde_json::json!({"call_id":"poll","output":format!("Native completion: {proof}\nProcess running with session ID 51\nOutput:\n")});
                pending.observed(Some(&reply)).unwrap();
                let state = ledger.read_state().unwrap();
                assert_eq!(state.claims["a:start"].effect, Effect::Opaque);
                assert_eq!(state.claims["a:start"].session, Some(51));
                assert_eq!(state.claims.contains_key("a:poll"), !proof_present);
                assert_eq!(
                    process_observation(&reply, "foreign", Some(51)).running,
                    None
                );
                assert_eq!(process_observation(&reply, "poll", Some(52)).running, None);
                if proof_present {
                    assert!(process_observation(&reply, "poll", None).proof.is_none());
                }
                fs::remove_dir_all(cwd).unwrap();
                fs::remove_dir_all(home).unwrap();
            }
        }
    }

    #[test]
    fn rejected_continuation_settles_only_current_invocation_without_retiring_process() {
        let cwd = root();
        let home = root();
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut original = Guard::begin_with(
            &cwd,
            "owner",
            "start",
            "functions.exec_command",
            r#"{"cmd":"interactive"}"#,
            ledger.clone(),
        )
        .unwrap();
        original.observed(Some(&serde_json::json!({"output":"Native completion: null\nProcess running with session ID 51\nOutput:\n"}))).unwrap();
        let mut pending = Guard::begin_with(
            &cwd,
            "owner",
            "poll",
            "functions.write_stdin",
            r#"{"session_id":51,"chars":"input"}"#,
            ledger.clone(),
        )
        .unwrap();
        assert_eq!(
            Guard::begin_with(
                &cwd,
                "owner",
                "another",
                "functions.write_stdin",
                r#"{"session_id":51,"chars":"more"}"#,
                ledger.clone()
            )
            .err()
            .as_deref(),
            Some("native_resource_busy")
        );
        let proof = serde_json::json!({"schema":"codex-native-completion-v1","call_id":"poll",
            "session_id":null,"outcome":"pre_backend_rejected","exit_code":null,"hooks":"absent"});
        let result = serde_json::json!({"type":"function_call_output","call_id":"poll",
            "output":format!("Native completion: {proof}\nOutput:\nchanged hook snapshot")});
        let mut wrong = result.clone();
        wrong["call_id"] = serde_json::json!("foreign");
        pending.observed(Some(&wrong)).unwrap();
        assert_eq!(ledger.read_state().unwrap().claims.len(), 2);
        pending.observed(Some(&result)).unwrap();
        let state = ledger.read_state().unwrap();
        assert_eq!(state.claims.len(), 1);
        assert_eq!(state.claims["owner:start"].session, Some(51));
        assert!(!state.claims.contains_key("owner:poll"));
        let _later = Guard::begin_with(
            &cwd,
            "owner",
            "another",
            "functions.write_stdin",
            r#"{"session_id":51,"chars":"more"}"#,
            ledger.clone(),
        )
        .unwrap();
        pending.observed(Some(&result)).unwrap();
        let state = ledger.read_state().unwrap();
        assert_eq!(state.claims.len(), 2);
        assert!(state.claims.contains_key("owner:start"));
        assert!(state.claims.contains_key("owner:another"));
        assert_eq!(
            apply(&cwd, &home, "other", "conflict", "a")
                .err()
                .as_deref(),
            Some("native_resource_busy")
        );
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn continuation_scope_rejection_abort_and_original_wait_preserve_pending_invocation() {
        for (exec_tool, poll_tool) in [
            ("exec_command", "write_stdin"),
            ("functions.exec_command", "functions.write_stdin"),
        ] {
            let cwd = root();
            let home = root();
            let ledger = Ledger::at(&cwd, &home).unwrap();
            let facts = native_effect::compiler_fixture(&cwd);
            for name in ["a.c", "b.c"] {
                fs::write(cwd.join(name), "int main(void){return 0;}\n").unwrap();
            }
            let input =
                serde_json::json!({"cmd":"clang -O0 a.c -o a.out","workdir":cwd,"login":false})
                    .to_string();
            let scope =
                native_effect::classify_with_execution(exec_tool, &input, Some(&cwd), Some(&facts));
            let mut original =
                Guard::begin_effect(&cwd, "a", "start", exec_tool, &input, ledger.clone(), scope)
                    .unwrap();
            original
                .observed(Some(
                    &serde_json::json!({"output":"Process running with session ID 51\nOutput:\n"}),
                ))
                .unwrap();
            let other_input =
                serde_json::json!({"cmd":"clang -O0 b.c -o b.out","workdir":cwd,"login":false})
                    .to_string();
            let other_scope = native_effect::classify_with_execution(
                exec_tool,
                &other_input,
                Some(&cwd),
                Some(&facts),
            );
            let mut other = Guard::begin_effect(
                &cwd,
                "b",
                "start",
                exec_tool,
                &other_input,
                ledger.clone(),
                other_scope.clone(),
            )
            .unwrap();
            let before = serde_json::to_value(ledger.read_state().unwrap()).unwrap();
            let poll = r#"{"session_id":51,"chars":"input"}"#;
            assert_eq!(
                Guard::begin_effect(
                    &cwd,
                    "a",
                    "blocked",
                    poll_tool,
                    poll,
                    ledger.clone(),
                    Effect::Opaque
                )
                .err()
                .as_deref(),
                Some("native_resource_busy")
            );
            assert_eq!(
                serde_json::to_value(ledger.read_state().unwrap()).unwrap(),
                before
            );
            let wait = |id: &str, session: Option<u64>| {
                serde_json::json!({"output":format!("Native completion: {}\nOutput:\n",
                serde_json::json!({"schema":"codex-native-completion-v1","call_id":id,"session_id":session,
                    "outcome":"process_tree_completed","exit_code":0,"hooks":"absent"}))})
            };
            other.observed(Some(&wait("start", None))).unwrap();
            let mut pending = Guard::begin_effect(
                &cwd,
                "a",
                "pending",
                poll_tool,
                poll,
                ledger.clone(),
                Effect::Opaque,
            )
            .unwrap();
            assert_eq!(
                ledger.read_state().unwrap().claims["a:start"].effect,
                Effect::Opaque
            );
            original.observed(Some(&wait("start", Some(51)))).unwrap();
            assert!(!ledger.read_state().unwrap().claims.contains_key("a:start"));
            assert!(ledger
                .read_state()
                .unwrap()
                .claims
                .contains_key("a:pending"));
            pending
                .observed(Some(
                    &serde_json::json!({"output":"Process exited with code 0\nOutput:\n"}),
                ))
                .unwrap();
            assert!(ledger
                .read_state()
                .unwrap()
                .claims
                .contains_key("a:pending"));
            assert_eq!(
                Guard::begin_effect(
                    &cwd,
                    "b",
                    "after-original",
                    exec_tool,
                    &other_input,
                    ledger.clone(),
                    other_scope.clone()
                )
                .err()
                .as_deref(),
                Some("native_resource_busy")
            );
            pending.observed(Some(&wait("pending", Some(51)))).unwrap();
            assert!(Guard::begin_effect(
                &cwd,
                "b",
                "after-exact-poll",
                exec_tool,
                &other_input,
                ledger,
                other_scope
            )
            .is_ok());
            fs::remove_dir_all(cwd).unwrap();
            fs::remove_dir_all(home).unwrap();
        }
    }

    #[test]
    fn session_poll_rechecks_exact_shared_owner_and_original_scope_under_lock() {
        let cwd = root();
        let home = root();
        let facts = native_effect::compiler_fixture(&cwd);
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut scopes = Vec::new();
        for (lane, session) in [("a", 51), ("b", 52)] {
            fs::write(cwd.join(format!("{lane}.c")), "int main(void){return 0;}\n").unwrap();
            let input = serde_json::json!({"cmd":format!("clang -O0 {lane}.c -o {lane}.out"),"workdir":cwd,"login":false}).to_string();
            let effect = native_effect::classify_with_execution(
                "functions.exec_command",
                &input,
                Some(&cwd),
                Some(&facts),
            );
            assert!(matches!(effect, Effect::Paths(_)));
            let mut guard = Guard::begin_effect(
                &cwd,
                lane,
                "build",
                "functions.exec_command",
                &input,
                ledger.clone(),
                effect.clone(),
            )
            .unwrap();
            guard.observed(Some(&serde_json::json!({"output":format!("Process running with session ID {session}")}))).unwrap();
            scopes.push(effect);
        }
        let before = fs::read(ledger.directory.join("claims.json")).unwrap();
        for (owner, session, chars, scope) in [
            ("foreign", 51, "", &scopes[0]),
            ("a", 52, "", &scopes[0]),
            ("a", 51, "x", &scopes[0]),
            ("a", 51, "", &scopes[1]),
            ("a", 99, "", &scopes[0]),
        ] {
            let input = serde_json::json!({"session_id":session,"chars":chars}).to_string();
            assert_eq!(
                Guard::begin_effect(
                    &cwd,
                    owner,
                    "invalid-poll",
                    "functions.write_stdin",
                    &input,
                    ledger.clone(),
                    scope.clone()
                )
                .err()
                .as_deref(),
                Some("native_resource_busy")
            );
            assert_eq!(
                fs::read(ledger.directory.join("claims.json")).unwrap(),
                before
            );
        }
        fs::write(cwd.join("a.out"), "original process output").unwrap();
        let input = r#"{"session_id":51,"chars":""}"#;
        let mut poll = Guard::begin_effect(
            &cwd,
            "a",
            "poll",
            "functions.write_stdin",
            input,
            ledger.clone(),
            scopes[0].clone(),
        )
        .unwrap();
        assert_eq!(
            fs::read(ledger.directory.join("claims.json")).unwrap(),
            before,
            "poll must preserve original claim identity and scope"
        );
        poll.observed(Some(
            &serde_json::json!({"output":"Process running with session ID 51"}),
        ))
        .unwrap();
        assert_eq!(
            fs::read(ledger.directory.join("claims.json")).unwrap(),
            before
        );
        poll.observed(Some(
            &serde_json::json!({"output":"Process exited with code 0"}),
        ))
        .unwrap();
        assert_eq!(
            fs::read(ledger.directory.join("claims.json")).unwrap(),
            before,
            "legacy parent exit cannot close the original process tree"
        );
        poll.observed(Some(&waited("poll", Some(51), 0, "")))
            .unwrap();
        let state = ledger.read_state().unwrap();
        assert_eq!(state.claims.len(), 1);
        assert!(state
            .claims
            .values()
            .all(|entry| entry.thread == "b" && entry.session == Some(52)));
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn continuation_without_prior_claim_cleans_up_on_exit() {
        let cwd = root();
        let home = root();
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut stopped = Guard::begin_with(
            &cwd,
            "one",
            "poll",
            "functions.write_stdin",
            r#"{"session_id":81,"chars":""}"#,
            ledger.clone(),
        )
        .unwrap();
        stopped
            .observed(Some(&waited("poll", Some(81), 0, "")))
            .unwrap();
        assert!(apply(&cwd, &home, "two", "after", "a").is_ok());
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn shared_status_reads_original_ledger_without_retiring_or_disclosing_commands() {
        let cwd = root();
        let home = root();
        let missing = home.join("absent");
        let empty = Guard::shared_status(Some(&missing), &[], &BTreeMap::new()).unwrap();
        assert_eq!(empty["unconfirmed_effects"], 0);
        assert!(!missing.exists(), "status must not create a ledger");
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut held = Guard::begin_with(
            &cwd,
            "owner",
            "secret-call",
            "functions.exec_command",
            r#"{"cmd":"private-token=secret"}"#,
            ledger.clone(),
        )
        .unwrap();
        let path = ledger.directory.join("claims.json");
        let before = fs::read(&path).unwrap();
        let status = Guard::shared_status(Some(&home), &["owner"], &BTreeMap::new()).unwrap();
        assert_eq!(status["unconfirmed_effects"], 1);
        assert_eq!(status["active"][0]["resource_kind"], "opaque");
        assert_eq!(status["active"][0]["scope"], "unknown_global");
        assert_eq!(status["active"][0]["owner_process_state"], "live");
        assert_eq!(status["active"][0]["owned"], true);
        assert!(!status.to_string().contains("secret"));
        assert!(!status.to_string().contains(cwd.to_str().unwrap()));
        assert_eq!(fs::read(&path).unwrap(), before);
        let other = Guard::shared_status(Some(&home), &["other"], &BTreeMap::new()).unwrap();
        assert_eq!(other["active"][0]["owner_work"], serde_json::Value::Null);
        held.observed(Some(&waited("secret-call", None, 0, "")))
            .unwrap();
        assert_eq!(
            Guard::shared_status(Some(&home), &[], &BTreeMap::new()).unwrap()
                ["unconfirmed_effects"],
            0
        );
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn diagnostic_snapshot_refuses_locked_or_symlinked_state_without_changes() {
        let cwd = root();
        let home = root();
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let held = apply(&cwd, &home, "one", "call", "a").unwrap();
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(ledger.directory.join("lock"))
            .unwrap();
        // Establish the fixture's exclusive owner with the same acquisition as
        // production; another parallel test may briefly inherit a closing fd.
        assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) }, 0);
        assert_eq!(
            Guard::shared_status(Some(&home), &[], &BTreeMap::new()).unwrap_err(),
            "native_resource_journal_snapshot_busy"
        );
        drop(lock);
        drop(held);
        let state = ledger.directory.join("claims.json");
        let saved = ledger.directory.join("original.json");
        fs::rename(&state, &saved).unwrap();
        std::os::unix::fs::symlink(&saved, &state).unwrap();
        assert!(Guard::shared_status(Some(&home), &[], &BTreeMap::new()).is_err());
        assert!(ledger.transact(|_| Ok(((), false))).is_err());
        assert!(saved.is_file());
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn orphan_continuation_retains_uncertain_session_until_original_terminal_receipt() {
        let cwd = root();
        let home = root();
        let ledger = Ledger::at(&cwd, &home).unwrap();
        let mut held = Guard::begin_with(
            &cwd,
            "one",
            "poll",
            "functions.write_stdin",
            r#"{"session_id":81,"chars":""}"#,
            ledger.clone(),
        )
        .unwrap();
        held.observed(Some(&serde_json::json!({"output":"Process running with session ID 81\nOutput:\nProcess exited with code 0"}))).unwrap();
        assert_eq!(
            apply(&cwd, &home, "other", "writer", "a").err().as_deref(),
            Some("native_resource_busy")
        );
        let status = Guard::shared_status(Some(&home), &["one"], &BTreeMap::new()).unwrap();
        assert_eq!(status["active"][0]["process_session"], 81);
        held.observed(None).unwrap();
        assert_eq!(
            Guard::shared_status(Some(&home), &[], &BTreeMap::new()).unwrap()
                ["unconfirmed_effects"],
            1
        );
        held.observed(Some(&waited("poll", Some(81), 3, "")))
            .unwrap();
        assert!(apply(&cwd, &home, "other", "next", "a").is_ok());
        fs::remove_dir_all(cwd).unwrap();
        fs::remove_dir_all(home).unwrap();
    }
}
