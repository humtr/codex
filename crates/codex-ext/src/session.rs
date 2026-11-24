use crate::paths::{
    atomic_write_private, ensure_private_dir, is_live_matching_process, now_millis,
    proc_start_ticks, require_owned_regular_file, ExtPaths,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub pid: u32,
    pub start_ticks: u64,
    #[serde(default)]
    pub generation: u64,
    pub account: String,
    pub codex_home: String,
    pub cwd: String,
    pub provider: Option<String>,
    #[serde(default)]
    pub deferred_provider_switch: bool,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffTicket {
    pub kind: String, // "switch" | "provider"
    pub target: String,
    pub thread_id: Option<String>,
    pub wait: bool,
    pub requested_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_start_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_generation: Option<u64>,
}

pub enum SessionMatch {
    Exact(SessionRecord),
    Ambiguous(Vec<SessionRecord>),
    None,
}

pub fn session_file_path(paths: &ExtPaths, pid: u32) -> PathBuf {
    paths.sessions_dir.join(format!("{pid}.json"))
}

pub fn handoff_file_path(paths: &ExtPaths, parent_pid: u32) -> PathBuf {
    paths.handoff_dir.join(format!("{parent_pid}.json"))
}

pub fn claimed_handoff_path(paths: &ExtPaths, parent_pid: u32) -> PathBuf {
    handoff_file_path(paths, parent_pid).with_extension("claimed")
}

pub fn cleanup_handoff_files(paths: &ExtPaths, parent_pid: u32) {
    let _ = fs::remove_file(handoff_file_path(paths, parent_pid));
    let _ = fs::remove_file(claimed_handoff_path(paths, parent_pid));
}

pub fn normalize_match_path(path: &Path) -> String {
    if let Ok(canon) = fs::canonicalize(path) {
        return canon.display().to_string();
    }
    let s = path.display().to_string();
    let trimmed = s.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
pub fn register_current_session(
    paths: &ExtPaths,
    account: &str,
    codex_home: &Path,
    cwd: &Path,
    provider: Option<&str>,
) -> Result<SessionRecord, String> {
    register_current_session_generation(paths, account, codex_home, cwd, provider, 1)
}

pub fn register_current_session_generation(
    paths: &ExtPaths,
    account: &str,
    codex_home: &Path,
    cwd: &Path,
    provider: Option<&str>,
    generation: u64,
) -> Result<SessionRecord, String> {
    let pid = std::process::id();
    let start_ticks = proc_start_ticks(pid).unwrap_or(0);
    let rec = SessionRecord {
        pid,
        start_ticks,
        generation,
        account: account.to_owned(),
        codex_home: normalize_match_path(codex_home),
        cwd: normalize_match_path(cwd),
        provider: provider.map(ToOwned::to_owned),
        deferred_provider_switch: true,
        created_at_ms: now_millis(),
    };
    let body = serde_json::to_string_pretty(&rec)
        .map_err(|e| format!("failed to serialize session record: {e}"))?
        + "\n";
    atomic_write_private(&session_file_path(paths, pid), body.as_bytes(), 0o600)?;
    Ok(rec)
}

pub fn remove_session_file(paths: &ExtPaths, pid: u32) {
    let p = session_file_path(paths, pid);
    let _ = fs::remove_file(p);
}

pub fn write_handoff_ticket(
    paths: &ExtPaths,
    parent_pid: u32,
    ticket: &HandoffTicket,
) -> Result<(), String> {
    ensure_private_dir(&paths.handoff_dir)?;
    let body = serde_json::to_string_pretty(ticket)
        .map_err(|e| format!("failed to serialize handoff ticket: {e}"))?
        + "\n";
    atomic_write_private(
        &handoff_file_path(paths, parent_pid),
        body.as_bytes(),
        0o600,
    )
}

pub fn take_handoff_ticket(
    paths: &ExtPaths,
    parent_pid: u32,
) -> Result<Option<HandoffTicket>, String> {
    let Some(claimed) = claim_handoff_ticket(paths, parent_pid)? else {
        return Ok(None);
    };
    read_claimed_handoff(&claimed).map(Some)
}

fn claim_handoff_ticket(paths: &ExtPaths, parent_pid: u32) -> Result<Option<PathBuf>, String> {
    let p = handoff_file_path(paths, parent_pid);
    let claimed = p.with_extension("claimed");
    match fs::rename(&p, &claimed) {
        Ok(()) => Ok(Some(claimed)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("cannot claim handoff ticket: {err}")),
    }
}

fn read_claimed_handoff(claimed: &Path) -> Result<HandoffTicket, String> {
    let raw = require_owned_regular_file(claimed, "handoff ticket").and_then(|()| {
        fs::read_to_string(claimed).map_err(|e| format!("cannot read handoff ticket: {e}"))
    });
    let _ = fs::remove_file(claimed);
    serde_json::from_str(&raw?).map_err(|e| format!("corrupt handoff ticket: {e}"))
}

pub fn list_live_sessions(paths: &ExtPaths) -> Result<Vec<SessionRecord>, String> {
    if !paths.sessions_dir.exists() {
        return Ok(Vec::new());
    }
    let mut live = Vec::new();
    for entry in
        fs::read_dir(&paths.sessions_dir).map_err(|e| format!("cannot read sessions dir: {e}"))?
    {
        let entry = entry.map_err(|e| format!("sessions dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if require_owned_regular_file(&path, "session file").is_err() {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(rec) = serde_json::from_str::<SessionRecord>(&raw) else {
            continue;
        };
        if rec.start_ticks > 0 && is_live_matching_process(rec.pid, rec.start_ticks) {
            live.push(rec);
        } else {
            // Stale dead session file: prune quietly.
            let _ = fs::remove_file(&path);
        }
    }
    live.sort_by_key(|s| s.pid);
    Ok(live)
}

pub fn find_matching_session(
    paths: &ExtPaths,
    explicit_pid: Option<u32>,
    current_codex_home: &Path,
    current_cwd: &Path,
) -> Result<SessionMatch, String> {
    let live = list_live_sessions(paths)?;
    if let Some(pid) = explicit_pid {
        return match live.into_iter().find(|s| s.pid == pid) {
            Some(rec) => Ok(SessionMatch::Exact(rec)),
            None => Err(format!(
                "no live codex-ext session found for --session {pid}"
            )),
        };
    }
    let home_str = normalize_match_path(current_codex_home);
    let cwd_str = normalize_match_path(current_cwd);
    let matches: Vec<SessionRecord> = live
        .into_iter()
        .filter(|s| s.codex_home == home_str && s.cwd == cwd_str)
        .collect();
    match matches.len() {
        0 => Ok(SessionMatch::None),
        1 => Ok(SessionMatch::Exact(matches.into_iter().next().unwrap())),
        _ => Ok(SessionMatch::Ambiguous(matches)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_registration_matching_and_stale_pruning() {
        let root =
            std::env::temp_dir().join(format!("cdx-sess-{}-{}", std::process::id(), now_millis()));
        let paths = ExtPaths {
            prefix: root.join("usr"),
            tmpdir: root.join("tmp"),
            codex_bin: root.join("usr/bin/codex"),
            shared_home: root.join(".codex"),
            manager_dir: root.join(".local/share/codex/manager"),
            profiles_dir: root.join(".local/share/codex/manager/profiles"),
            default_profile_file: root.join(".local/share/codex/manager/default-profile"),
            sessions_dir: root.join(".local/state/codex/ext/sessions"),
            handoff_dir: root.join(".local/state/codex/ext/handoff"),
            notify_config_file: root.join(".config/codex/ext/notify.json"),
        };

        let cwd = root.join("proj");
        let rec =
            register_current_session(&paths, "default", &paths.shared_home, &cwd, None).unwrap();
        // Also write a fake dead session file (bogus start_ticks)
        let dead = SessionRecord {
            pid: 999_999,
            start_ticks: 1,
            generation: 0,
            account: "dead".to_owned(),
            codex_home: paths.shared_home.display().to_string(),
            cwd: cwd.display().to_string(),
            provider: None,
            deferred_provider_switch: false,
            created_at_ms: 0,
        };
        atomic_write_private(
            &session_file_path(&paths, 999_999),
            serde_json::to_string(&dead).unwrap().as_bytes(),
            0o600,
        )
        .unwrap();

        match find_matching_session(&paths, None, &paths.shared_home, &cwd).unwrap() {
            SessionMatch::Exact(found) => assert_eq!(found.pid, rec.pid),
            _ => panic!("expected exact live session match"),
        }
        assert!(!session_file_path(&paths, 999_999).exists());

        // Write and take handoff ticket
        let ticket = HandoffTicket {
            kind: "switch".to_owned(),
            target: "szob".to_owned(),
            thread_id: Some("tid-123".to_owned()),
            wait: false,
            requested_at_ms: now_millis(),
            target_start_ticks: Some(rec.start_ticks),
            target_generation: Some(rec.generation),
        };
        write_handoff_ticket(&paths, rec.pid, &ticket).unwrap();
        let claimed = claim_handoff_ticket(&paths, rec.pid).unwrap().unwrap();
        let later = HandoffTicket {
            target: "janmori101".to_owned(),
            ..ticket
        };
        write_handoff_ticket(&paths, rec.pid, &later).unwrap();
        let taken = read_claimed_handoff(&claimed).unwrap();
        assert_eq!(taken.target, "szob");
        assert_eq!(
            take_handoff_ticket(&paths, rec.pid)
                .unwrap()
                .unwrap()
                .target,
            "janmori101"
        );
        assert!(take_handoff_ticket(&paths, rec.pid).unwrap().is_none());

        // Legacy JSON without target_start_ticks/target_generation deserializes cleanly,
        // and cleanup_handoff_files removes both pending and claimed tickets.
        let legacy_json = r#"{"kind":"switch","target":"szob","thread_id":null,"wait":false,"requested_at_ms":1}"#;
        let legacy: HandoffTicket = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(legacy.target_start_ticks, None);
        assert_eq!(legacy.target_generation, None);
        write_handoff_ticket(&paths, rec.pid, &legacy).unwrap();
        let _claimed2 = claim_handoff_ticket(&paths, rec.pid).unwrap().unwrap();
        write_handoff_ticket(&paths, rec.pid, &legacy).unwrap();
        cleanup_handoff_files(&paths, rec.pid);
        assert!(!handoff_file_path(&paths, rec.pid).exists());
        assert!(!claimed_handoff_path(&paths, rec.pid).exists());

        remove_session_file(&paths, rec.pid);
        let _ = fs::remove_dir_all(&root);
    }
}
