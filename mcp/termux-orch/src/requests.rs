//! Durable MCP admission and receipts for relay-owned calls.
use crate::ledger::{ensure_private_dir, read_private_file, write_atomic_private, Ledger};
use mcp_kit::{sha256_hex, validate_call_id};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_QUEUED: usize = 128;
const VALID_TOOLS: &[&str] = &["orch_work", "orch_works", "orch_dispatch", "orch_call"];

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
        Ok(Self { root })
    }

    fn pending(&self, id: &str) -> PathBuf {
        self.root.join("pending").join(id)
    }

    fn finished(&self, id: &str) -> PathBuf {
        self.root.join("finished").join(id)
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
        Ok(self.load(id)?.map_or_else(
            || json!({"state":"not_found","operation_id":id}),
            |record| record.response(id, false),
        ))
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
        }
        Ok(())
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
