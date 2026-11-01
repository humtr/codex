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
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessOwner {
    pid: u32,
    start: u64,
}

fn process_start(pid: u32) -> Result<u64, std::io::Error> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    stat.rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().nth(19))
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| std::io::Error::other("invalid process identity"))
}

impl ProcessOwner {
    fn current() -> Result<Self, String> {
        let pid = std::process::id();
        let start = process_start(pid).map_err(|_| "native_resource_process_unavailable")?;
        Ok(Self { pid, start })
    }

    fn departed(&self) -> bool {
        match process_start(self.pid) {
            Ok(start) => start != self.start,
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
        let mut state = match File::open(&path) {
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
                serde_json::from_slice(&bytes).map_err(|_| fail())?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => State::default(),
            Err(_) => return Err(fail()),
        };
        let (result, changed) = f(&mut state)?;
        if changed {
            let bytes = serde_json::to_vec(&state).map_err(|_| fail())?;
            if bytes.len() as u64 > STATE_MAX {
                return Err(fail());
            }
            let temp = self.directory.join(format!(
                ".claims-{}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
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

fn continuation(tool: &str, input: &str) -> Option<u64> {
    (tool == "functions.write_stdin")
        .then(|| {
            serde_json::from_str::<serde_json::Value>(input)
                .ok()?
                .get("session_id")?
                .as_u64()
        })
        .flatten()
}

fn process_status(value: &serde_json::Value) -> (Option<u64>, bool) {
    fn body(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(text) => Some(text.clone()),
            serde_json::Value::Array(values) => {
                let texts: Vec<_> = values.iter().filter_map(body).collect();
                (!texts.is_empty()).then(|| texts.join("\n"))
            }
            serde_json::Value::Object(fields) => ["body", "text", "output", "content"]
                .iter()
                .find_map(|name| fields.get(*name).and_then(body)),
            _ => None,
        }
    }
    let Some(text) = body(value) else {
        return (None, false);
    };
    let header = text
        .split_once("\nOutput:\n")
        .map_or(text.as_str(), |(head, _)| head);
    let running = header
        .find("Process running with session ID ")
        .and_then(|i| {
            header[i + "Process running with session ID ".len()..]
                .split(|c: char| !c.is_ascii_digit())
                .next()?
                .parse::<u64>()
                .ok()
        });
    let exited = running.is_none()
        && (header.contains("Process exited with code ")
            || header.contains("Process exited with signal "));
    (running, exited)
}

impl Guard {
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
        if native_effect::classify(tool, input, Some(cwd)) == Effect::Read {
            return Ok(());
        }
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
        // Proven reads have no resource claim. A failed/unavailable ledger must
        // not turn an independent read into a blocked Native tool call.
        if native_effect::classify(tool, input, Some(cwd)) == Effect::Read {
            return Ok(());
        }
        let ledger = match private_test_home {
            Some(home) => Ledger::at(cwd, home)?,
            None => Ledger::new(cwd)?,
        };
        Self::begin_with(cwd, thread, call_id, tool, input, ledger)?;
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
        if native_effect::classify(tool, input, Some(cwd)) == Effect::Read {
            return Ok(());
        }
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
        observed.observed(Some(result))
    }

    fn begin_with(
        cwd: &Path,
        thread: &str,
        call_id: &str,
        tool: &str,
        input: &str,
        ledger: Ledger,
    ) -> Result<Self, String> {
        if thread.is_empty() || call_id.is_empty() || thread.len() + call_id.len() > 512 {
            return Err("native_effect_identity_invalid".to_owned());
        }
        let workspace =
            fs::canonicalize(cwd).map_err(|_| "native_workspace_unavailable".to_owned())?;
        let effect = native_effect::classify(tool, input, Some(&workspace));
        let identity = format!("{thread}:{call_id}");
        let existing_session = continuation(tool, input);
        let fingerprint = fingerprint(tool, input, &workspace);
        let owner = ProcessOwner::current()?;
        let resumed = ledger.transact(|state| {
            if let Some(existing) = state.claims.get(&identity) {
                return Err(if existing.fingerprint == fingerprint {
                    "native_effect_duplicate_unconfirmed"
                } else {
                    "native_effect_identity_changed"
                }
                .to_owned());
            }
            let continuing_owned = existing_session.is_some_and(|session| {
                state
                    .claims
                    .values()
                    .any(|held| held.thread == thread && held.session == Some(session))
            });
            if state.claims.values().any(|held| {
                native_effect::conflicts(&effect, &held.effect)
                    && !(continuing_owned
                        && held.thread == thread
                        && held.session == existing_session)
            }) {
                return Err("native_resource_busy".to_owned());
            }
            native_effect::can_offer(&effect, &workspace).map_err(str::to_owned)?;
            if continuing_owned {
                Ok((true, false))
            } else {
                state.claims.insert(
                    identity.clone(),
                    Entry {
                        thread: thread.into(),
                        fingerprint,
                        effect,
                        session: None,
                        owner: Some(owner),
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
            continuation: if resumed { existing_session } else { None },
        })
    }

    /// Only an actual handler result may settle a claim. A dropped future
    /// or an absent receipt never proves completion, including apply_patch.
    pub(crate) fn observed(&mut self, result: Option<&serde_json::Value>) -> Result<(), String> {
        let Some(result) = result else {
            return Ok(());
        };
        let (running, exited) = process_status(result);
        let session = self.continuation;
        self.ledger.transact(|state| {
            if let Some(session) = session {
                if exited {
                    state.claims.retain(|_, held| {
                        !(held.thread == self.thread && held.session == Some(session))
                    });
                    return Ok(((), true));
                }
                return Ok(((), false));
            }
            if self.tool == "functions.exec_command" && running.is_some() && !exited {
                if let Some(claim) = state.claims.get_mut(&self.identity) {
                    claim.session = running;
                }
                return Ok(((), true));
            }
            // An exec result without an identifiable exit or live session is
            // unresolved; only unambiguous terminal evidence frees that claim.
            if self.tool == "functions.exec_command" && !exited {
                return Ok(((), false));
            }
            Ok(((), state.claims.remove(&self.identity).is_some()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "native-ledger-test-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
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
        let a = native_effect::classify(
            "functions.apply_patch",
            "*** Begin Patch\n*** Update File: a\n@@\n-old\n+new\n*** End Patch\n",
            Some(&cwd),
        );
        let b = native_effect::classify(
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
        poll.observed(Some(&serde_json::json!({
            "output":"Process exited with code 0\nOutput:\nProcess running with session ID 83"
        })))
        .unwrap();
        assert!(apply(&cwd, &home, "second", "patch", "a").is_ok());
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
        fs::write(cwd.join("a"), "old\n").unwrap();
        let _writing = apply(&cwd, &home, "writer", "patch", "a").unwrap();
        // An unusable shared journal must not prevent read-only calls,
        // including while an unrelated write reservation is present.
        let blocked_home = home.join("not-a-directory");
        fs::write(&blocked_home, "cannot create a journal beneath a file").unwrap();
        for (thread, call_id, input) in [
            ("reader", "cat", r#"{"cmd":"cat a"}"#),
            ("reader-2", "pwd", r#"{"cmd":"pwd"}"#),
        ] {
            Guard::offer_external(
                &cwd,
                Some(&blocked_home),
                thread,
                call_id,
                "functions.exec_command",
                input,
            )
            .unwrap();
        }
        Guard::settle_external_receipt(
            &cwd,
            Some(&blocked_home),
            "reader",
            "time",
            "clock.curr_time",
            None,
            &serde_json::json!({"value":"read result"}),
        )
        .unwrap();
        assert_eq!(
            native_effect::classify(
                "functions.exec_command",
                r#"{"cmd":"cat a; touch b"}"#,
                Some(&cwd)
            ),
            Effect::Opaque
        );
        assert_eq!(
            native_effect::classify(
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
}
