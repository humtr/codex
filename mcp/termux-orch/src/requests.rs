//! Durable MCP admission and receipts for relay-owned calls.
use crate::ledger::{ensure_private_dir, read_private_file, write_atomic_private, Ledger};
use mcp_kit::{sha256_hex, validate_call_id};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_QUEUED: usize = 128;
const MAX_FINISHED_RECORDS: usize = 512;
const PRUNE_BATCH: usize = 32;
const VALID_TOOLS: &[&str] = &["orch_work", "orch_works", "orch_dispatch", "orch_call"];

static COMPLETIONS_SINCE_PRUNE: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tombstone {
    fingerprint: String,
    #[serde(default)]
    original_state: Option<String>,
    submitted_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Operation {
    fingerprint: String,
    submitted_at: u64,
    request: Option<Request>,
    started: bool,
    result: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    tool: String,
    arguments: Value,
}

pub struct Requests {
    root: PathBuf,
}

impl Requests {
    pub fn open(ledger: &Ledger) -> io::Result<Self> {
        let root = ledger.root().join("requests");
        ensure_private_dir(&root)?;
        ensure_private_dir(&root.join("pending"))?;
        ensure_private_dir(&root.join("finished"))?;
        ensure_private_dir(&root.join("tombstones"))?;
        fs::File::open(&root)?.sync_all()?;
        Ok(Self { root })
    }

    fn pending(&self, id: &str) -> PathBuf {
        self.root.join("pending").join(id)
    }

    fn finished(&self, id: &str) -> PathBuf {
        self.root.join("finished").join(id)
    }

    fn tombstone_shard_path(&self, prefix: &str) -> PathBuf {
        self.root.join("tombstones").join(format!("{prefix}.json"))
    }

    fn legacy_tombstones_path(&self) -> PathBuf {
        self.root.join("tombstones.json")
    }

    fn load_tombstone_shard(&self, prefix: &str) -> io::Result<BTreeMap<String, Tombstone>> {
        match read_private_file(&self.tombstone_shard_path(prefix), MAX_RECORD_BYTES as u64) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(e),
        }
    }

    fn save_tombstone_shard(
        &self,
        prefix: &str,
        map: &BTreeMap<String, Tombstone>,
        max_bytes: usize,
    ) -> io::Result<()> {
        let bytes = serde_json::to_vec(map).map_err(io::Error::other)?;
        if bytes.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("tombstone shard {prefix} exceeds size limit ({max_bytes} bytes)"),
            ));
        }
        write_atomic_private(&self.tombstone_shard_path(prefix), &bytes)
    }

    fn find_tombstone(&self, id: &str) -> io::Result<Option<Tombstone>> {
        let prefix = &id[..2];
        if let Some(tombstone) = self.load_tombstone_shard(prefix)?.remove(id) {
            return Ok(Some(tombstone));
        }
        // Backward-compatible read fallback for any pre-sharding tombstones.json.
        match read_private_file(&self.legacy_tombstones_path(), MAX_RECORD_BYTES as u64) {
            Ok(bytes) => {
                let mut legacy: BTreeMap<String, Tombstone> =
                    serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                Ok(legacy.remove(id))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn load(&self, id: &str) -> io::Result<Option<Operation>> {
        // Retry the finished lookup if the relay moved the record during this read.
        for path in [self.pending(id), self.finished(id)] {
            if let Some(record) = read_record(&path)? {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    pub fn admission_lock(&self) -> io::Result<crate::ledger::WorkLock> {
        Ledger::lock(&self.root.join(".lock"), false)
    }

    pub fn submit(
        &self,
        tool: &str,
        args: &Value,
        existing: impl FnOnce() -> Option<Value>,
    ) -> io::Result<Value> {
        let call = if tool == "orch_call" {
            &args["arguments"]
        } else {
            args
        };
        let id = validate_call_id(call).map_err(|_| invalid_id())?;
        let id = sha256_hex(id.as_bytes());
        let fingerprint = sha256_hex(json!([tool, args]).to_string().as_bytes());
        let _lock = self.admission_lock()?;
        if let Some(record) = self.load(&id)? {
            if record.fingerprint != fingerprint {
                return Ok(
                    json!({"state":"rejected","executed":false,"error":"orch_call_id_payload_conflict"}),
                );
            }
            return Ok(record.response(&id, true));
        }
        if let Some(tombstone) = self.find_tombstone(&id)? {
            if tombstone.fingerprint != fingerprint {
                return Ok(
                    json!({"state":"rejected","executed":false,"error":"orch_call_id_payload_conflict"}),
                );
            }
            return Ok(json!({
                "state": "rejected",
                "executed": "unknown",
                "error": "orch_operation_receipt_expired",
                "operation_id": id,
                "replayed_from_receipt": true,
                "original_state": tombstone.original_state,
            }));
        }
        if let Some(receipt) = existing() {
            return Ok(receipt);
        }
        let pending = fs::read_dir(self.root.join("pending"))?
            .map(|entry| entry.map(|entry| valid_id(&entry.file_name().to_string_lossy())))
            .collect::<io::Result<Vec<_>>>()?;
        if pending.into_iter().filter(|queued| *queued).count() >= MAX_QUEUED {
            return Ok(
                json!({"state":"rejected","executed":false,"error":"orch_queue_full","retry_after":2}),
            );
        }
        let record = Operation {
            fingerprint,
            submitted_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros()
                .min(u128::from(u64::MAX)) as u64,
            request: Some(Request {
                tool: tool.to_owned(),
                arguments: args.clone(),
            }),
            started: false,
            result: None,
        };
        write_record(&self.pending(&id), &record)?;
        Ok(record.response(&id, false))
    }

    pub fn status(&self, id: &str) -> io::Result<Value> {
        if !valid_id(id) {
            return Err(invalid_id());
        }
        if let Some(record) = self.load(id)? {
            return Ok(record.response(id, false));
        }
        if let Some(tombstone) = self.find_tombstone(id)? {
            return Ok(json!({
                "state": "expired",
                "operation_id": id,
                "original_state": tombstone.original_state,
            }));
        }
        Ok(json!({"state":"not_found","operation_id":id}))
    }

    fn pending_paths(&self) -> io::Result<Vec<(String, Operation)>> {
        let mut rows = Vec::new();
        for entry in fs::read_dir(self.root.join("pending"))? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if valid_id(&id) {
                if let Some(record) = read_record(&entry.path())? {
                    rows.push((id, record));
                }
            }
        }
        rows.sort_unstable_by_key(|(id, record)| (record.submitted_at, id.clone()));
        Ok(rows)
    }

    /// The caller holds the relay execution lock. No queue lock spans an owner call.
    pub fn run(
        &self,
        mut execute: impl FnMut(&str, &Value, bool) -> Option<Value>,
    ) -> io::Result<()> {
        for (id, mut record) in self.pending_paths()?.into_iter().take(8) {
            let path = self.pending(&id);
            if record.result.is_none() {
                let interrupted = record.started;
                if !interrupted {
                    record.started = true;
                    write_record(&path, &record)?;
                }
                let request = record
                    .request
                    .as_ref()
                    .filter(|request| VALID_TOOLS.contains(&request.tool.as_str()))
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid owner request")
                    })?;
                let result = execute(&request.tool, &request.arguments, interrupted)
                    .unwrap_or_else(|| {
                        let mut outcome = json!({"state":"delivery_unknown","executed":"unknown",
                        "error":"original_effect_unsettled_check_before_new_call"});
                        if request.tool == "orch_work" && request.arguments["action"] == "create" {
                            outcome["work_id"] = json!(format!("work_{}", &id[..32]));
                        }
                        outcome
                    });
                record.result = Some(result);
                record.request = None;
                write_record(&path, &record)?;
            }
            let _lock = self.admission_lock()?;
            fs::rename(&path, self.finished(&id))?;
            fs::File::open(self.root.join("finished"))?.sync_all()?;
            fs::File::open(self.root.join("pending"))?.sync_all()?;
            let completed = COMPLETIONS_SINCE_PRUNE.fetch_add(1, Ordering::AcqRel);
            if completed.is_multiple_of(PRUNE_BATCH) {
                if let Err(error) = self.prune_finished_locked(
                    MAX_FINISHED_RECORDS,
                    MAX_FINISHED_RECORDS + PRUNE_BATCH,
                    MAX_RECORD_BYTES,
                ) {
                    eprintln!(
                        "{}",
                        json!({
                            "component": "termux-orch-relay",
                            "event": "requests_finished_prune_failed",
                            "error": error.to_string()
                        })
                    );
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn prune_finished(&self, keep: usize) -> io::Result<usize> {
        self.prune_finished_with_shard_limit(keep, MAX_RECORD_BYTES)
    }

    #[cfg(test)]
    pub(crate) fn prune_finished_with_shard_limit(
        &self,
        keep: usize,
        max_shard_bytes: usize,
    ) -> io::Result<usize> {
        let _lock = self.admission_lock()?;
        self.prune_finished_locked(keep, keep, max_shard_bytes)
    }

    fn prune_finished_locked(
        &self,
        keep: usize,
        trigger_threshold: usize,
        max_shard_bytes: usize,
    ) -> io::Result<usize> {
        let dir = self.root.join("finished");
        let mut names: Vec<(String, PathBuf)> = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if valid_id(&id) {
                names.push((id, entry.path()));
            }
        }
        if names.len() <= trigger_threshold {
            return Ok(0);
        }
        // Only inspect symlink_metadata (no full JSON reads on retained receipts).
        let mut entries: Vec<(u128, String, PathBuf)> = Vec::with_capacity(names.len());
        for (id, path) in names {
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.file_type().is_file() {
                continue;
            }
            let mtime_ns = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            entries.push((mtime_ns, id, path));
        }
        if entries.len() <= keep {
            return Ok(0);
        }
        entries.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let remove_count = entries.len() - keep;
        let victims: Vec<(String, PathBuf)> = entries
            .into_iter()
            .take(remove_count)
            .map(|(_, id, path)| (id, path))
            .collect();

        // Group victims by 2-hex prefix and persist permanent sharded tombstones
        // BEFORE unlinking full receipts. Never evict tombstones; fail closed on
        // shard overflow so full receipts stay intact on disk.
        let mut by_shard: BTreeMap<String, Vec<(String, PathBuf)>> = BTreeMap::new();
        for (id, path) in victims {
            by_shard
                .entry(id[..2].to_owned())
                .or_default()
                .push((id, path));
        }

        let mut removed = 0usize;
        let mut first_error: Option<io::Error> = None;

        // Ensure the parent `requests/` directory entry for `tombstones/` is durable
        // on disk BEFORE unlinking any full receipts from `finished/`.
        fs::File::open(&self.root)?.sync_all()?;

        for (prefix, shard_victims) in by_shard {
            let mut shard = match self.load_tombstone_shard(&prefix) {
                Ok(s) => s,
                Err(err) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                    continue;
                }
            };
            let mut prepared: Vec<PathBuf> = Vec::with_capacity(shard_victims.len());
            let mut read_failed = false;
            for (id, path) in shard_victims {
                match read_record(&path) {
                    Ok(Some(record)) => {
                        let original_state = record
                            .result
                            .as_ref()
                            .and_then(|r| r.get("state"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned);
                        shard.insert(
                            id,
                            Tombstone {
                                fingerprint: record.fingerprint,
                                original_state,
                                submitted_at: record.submitted_at,
                            },
                        );
                        prepared.push(path);
                    }
                    Ok(None) => {}
                    Err(err) => {
                        if first_error.is_none() {
                            first_error = Some(err);
                        }
                        read_failed = true;
                        break;
                    }
                }
            }
            if read_failed || prepared.is_empty() {
                continue;
            }
            if let Err(err) = self.save_tombstone_shard(&prefix, &shard, max_shard_bytes) {
                // Fail closed: do NOT unlink full receipts if shard persistence failed.
                if first_error.is_none() {
                    first_error = Some(err);
                }
                continue;
            }
            for path in prepared {
                match fs::remove_file(&path) {
                    Ok(()) => removed += 1,
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => {
                        if first_error.is_none() {
                            first_error = Some(err);
                        }
                    }
                }
            }
        }
        if removed > 0 {
            if let Err(err) = fs::File::open(&dir).and_then(|f| f.sync_all()) {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }
        if let Some(err) = first_error {
            return Err(err);
        }
        Ok(removed)
    }
}

impl Operation {
    fn response(&self, id: &str, repeat: bool) -> Value {
        if let Some(mut result) = self.result.clone() {
            result["operation_id"] = json!(id);
            if repeat {
                result["replayed_from_receipt"] = json!(true);
            }
            result
        } else {
            json!({"state":if self.started {"processing"} else {"queued"}, "operation_id":id})
        }
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn invalid_id() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid call_id or operation_id",
    )
}

fn read_record(path: &Path) -> io::Result<Option<Operation>> {
    match read_private_file(path, MAX_RECORD_BYTES as u64) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(io::Error::other),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn write_record(path: &Path, record: &Operation) -> io::Result<()> {
    let bytes = serde_json::to_vec(record).map_err(io::Error::other)?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "owner operation is too large",
        ));
    }
    write_atomic_private(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_finished_keeps_newest_records_and_ignores_unrelated_files() {
        let root = std::env::temp_dir().join(format!(
            "termux-orch-req-prune-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let ledger = Ledger::open(root.join("state")).unwrap();
        let reqs = Requests::open(&ledger).unwrap();
        let mut records = Vec::new();
        for i in 0..8u64 {
            let call_id = format!("prune-call-{i}");
            let args = json!({
                "action": "rename_session",
                "arguments": {"backend":"chatgpt","ref_id":"c1","title":format!("T{i}"),"call_id":call_id}
            });
            let id = sha256_hex(call_id.as_bytes());
            let fp = sha256_hex(json!(["orch_call", args]).to_string().as_bytes());
            let record = Operation {
                fingerprint: fp,
                submitted_at: 1000 + i,
                request: None,
                started: true,
                result: Some(json!({"state": "ok", "idx": i})),
            };
            write_record(&reqs.finished(&id), &record).unwrap();
            records.push((id, call_id, args));
        }
        // Unrelated non-hex filename must not be removed.
        let keep_note = reqs.root.join("finished").join("keep-note.txt");
        fs::write(&keep_note, "ignore").unwrap();

        let removed = reqs.prune_finished(3).unwrap();
        assert_eq!(removed, 5);
        assert!(keep_note.exists());

        let mut remaining = 0usize;
        let mut expired = 0usize;
        for (id, call_id, args) in &records {
            if reqs.finished(id).exists() {
                remaining += 1;
                let replay = reqs.submit("orch_call", args, || None).unwrap();
                assert_eq!(replay["state"], "ok", "{replay}");
                assert_eq!(replay["replayed_from_receipt"], true);
            } else {
                expired += 1;
                // Fail-closed replay on pruned call_id: never re-enqueues!
                let replay = reqs.submit("orch_call", args, || None).unwrap();
                assert_eq!(replay["state"], "rejected", "{replay}");
                assert_eq!(
                    replay["error"], "orch_operation_receipt_expired",
                    "{replay}"
                );
                assert_eq!(replay["original_state"], "ok", "{replay}");
                assert!(!reqs.pending(id).exists());
                // Conflicting payload on pruned call_id is still rejected as conflict!
                let conflict_args = json!({
                    "action": "rename_session",
                    "arguments": {"backend":"chatgpt","ref_id":"c1","title":"DIFFERENT","call_id":call_id}
                });
                let conflict = reqs.submit("orch_call", &conflict_args, || None).unwrap();
                assert_eq!(
                    conflict["error"], "orch_call_id_payload_conflict",
                    "{conflict}"
                );
                // Status reports expired with original_state.
                let st = reqs.status(id).unwrap();
                assert_eq!(st["state"], "expired", "{st}");
                assert_eq!(st["original_state"], "ok", "{st}");
            }
        }
        assert_eq!(remaining, 3);
        assert_eq!(expired, 5);
        assert!(keep_note.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prune_shard_overflow_fails_closed_without_unlinking_receipts_and_never_evicts() {
        let root = std::env::temp_dir().join(format!(
            "termux-orch-req-shard-cap-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let ledger = Ledger::open(root.join("state")).unwrap();
        let reqs = Requests::open(&ledger).unwrap();
        let mut records = Vec::new();
        for i in 0..4u64 {
            let call_id = format!("cap-call-{i}");
            let args = json!({
                "action": "rename_session",
                "arguments": {"backend":"chatgpt","ref_id":"c1","title":format!("C{i}"),"call_id":call_id}
            });
            let id = sha256_hex(call_id.as_bytes());
            let fp = sha256_hex(json!(["orch_call", args]).to_string().as_bytes());
            let record = Operation {
                fingerprint: fp,
                submitted_at: 2000 + i,
                request: None,
                started: true,
                result: Some(json!({"state": "ok", "idx": i})),
            };
            write_record(&reqs.finished(&id), &record).unwrap();
            records.push((id, args));
        }

        // Tiny shard limit (16 bytes) forces shard overflow -> must return Err and keep all full receipts intact!
        let err = reqs.prune_finished_with_shard_limit(1, 16).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        for (id, args) in &records {
            assert!(
                reqs.finished(id).exists(),
                "full receipt {id} must NOT be unlinked when shard save fails"
            );
            let replay = reqs.submit("orch_call", args, || None).unwrap();
            assert_eq!(replay["state"], "ok");
            assert_eq!(replay["replayed_from_receipt"], true);
        }

        // With normal shard limit, consecutive prune rounds retain ALL tombstones without eviction.
        assert_eq!(reqs.prune_finished(2).unwrap(), 2);
        assert_eq!(reqs.prune_finished(0).unwrap(), 2);
        for (id, args) in &records {
            assert!(!reqs.finished(id).exists());
            let replay = reqs.submit("orch_call", args, || None).unwrap();
            assert_eq!(
                replay["error"], "orch_operation_receipt_expired",
                "{replay}"
            );
        }

        // Legacy tombstones.json fallback remains readable and fail-closed.
        let legacy_call_id = "legacy-tombstone-call-1";
        let legacy_args = json!({
            "action": "rename_session",
            "arguments": {"backend":"chatgpt","ref_id":"c1","title":"Legacy","call_id":legacy_call_id}
        });
        let legacy_id = sha256_hex(legacy_call_id.as_bytes());
        let legacy_fp = sha256_hex(json!(["orch_call", legacy_args]).to_string().as_bytes());
        let mut legacy_map = BTreeMap::new();
        legacy_map.insert(
            legacy_id.clone(),
            Tombstone {
                fingerprint: legacy_fp,
                original_state: Some("ok".to_owned()),
                submitted_at: 100,
            },
        );
        write_atomic_private(
            &reqs.legacy_tombstones_path(),
            &serde_json::to_vec(&legacy_map).unwrap(),
        )
        .unwrap();
        let legacy_replay = reqs.submit("orch_call", &legacy_args, || None).unwrap();
        assert_eq!(
            legacy_replay["error"], "orch_operation_receipt_expired",
            "{legacy_replay}"
        );
        assert_eq!(reqs.status(&legacy_id).unwrap()["state"], "expired");

        let _ = fs::remove_dir_all(&root);
    }
}
