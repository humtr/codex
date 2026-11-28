//! `@TermuxOrch` 6-tool MCP surface (`termux-orch mcp`).
//!
//! Static tools use the uniform MCP annotations required by SPEC.md / BRIDGE.md;
//! actual effects are governed by each tool's operation contract:
//! 1. `orch_works`    — `list | read | checkpoint | inbox`
//! 2. `orch_work`     — `create | promote | rebind_front | update_scope | cancel`
//! 3. `orch_dispatch` — `register_worker | register_verifier | wait_set | wait_clear`
//! 4. `orch_report`   — `milestone | need_user | complete | fail`
//! 5. `orch_catalog`  — lists strictly disjoint extension actions
//! 6. `orch_call`     — invokes allowlisted extension action (`lead_rollover`, `budget_extend`,
//!    `reconcile_delivery`, `relay_status`, `rename_session`)

use crate::ledger::{
    format_front_title, format_lead_title, format_participant_title, generate_id, is_safe_work_id,
    now_unix, read_private_file, truncate_chars, write_atomic_private, FrontBinding,
    FrontHistoryEntry, Isolation, LeadBinding, Ledger, Participant, PendingDelivery,
    ProjectBinding, SessionPrefixes, ShellContinuation, StepSpec, Verification, VerificationPolicy,
    WaitSet, WorkBudget, WorkRecord, WORK_SCHEMA,
};
use crate::owners::OwnerClient;
use crate::relay::{enqueue_delivery_if_absent, RelayEngine};
use mcp_kit::{
    sha256_hex, validate_call_id, ExtensionRegistry, McpHandler, ReceiptClaim, ReceiptStore,
    ToolDef,
};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

mod dispatch;
mod extensions;
mod projects;
mod report;
mod schema;
mod work;

pub(crate) use schema::build_extension_registry;
pub use schema::build_static_tools;
#[cfg(test)]
pub use schema::STATIC_TOOL_NAMES;

pub struct OrchMcpHandler {
    ledger: Ledger,
    owners: OwnerClient,
    receipt_dir: PathBuf,
    extensions: ExtensionRegistry,
    admission_only: bool,
}

impl OrchMcpHandler {
    pub fn new(ledger: Ledger, owners: OwnerClient, receipt_dir: PathBuf) -> Self {
        Self {
            ledger,
            owners,
            receipt_dir,
            extensions: build_extension_registry(),
            admission_only: false,
        }
    }

    pub fn admission(ledger: Ledger, owners: OwnerClient, receipt_dir: PathBuf) -> Self {
        let mut handler = Self::new(ledger.with_bounded_locks(), owners, receipt_dir);
        handler.admission_only = true;
        handler
    }

    fn owner_request(&self, tool: &str, args: &Value) -> Option<Value> {
        if !self.admission_only {
            return None;
        }
        let action = args["action"].as_str()?;
        let queued = match (tool, action) {
            ("orch_work", "create" | "promote" | "rebind_front") => true,
            ("orch_works", "read") => {
                args.get("rebind_front_marker").is_some()
                    || args.get("rebind_front_chat_id").is_some()
            }
            ("orch_dispatch", "register_worker" | "register_verifier") => {
                matches!(args["backend"].as_str(), Some("chatgpt" | "codex"))
            }
            ("orch_call", "rename_session" | "lead_rollover") => true,
            _ => false,
        };
        if !queued {
            return None;
        }
        let result = crate::requests::Requests::open(&self.ledger)
            .and_then(|requests| requests.submit(tool, args, || self.reconcile_request(tool, args)))
            .unwrap_or_else(|error| {
                fail(
                    if error.kind() == std::io::ErrorKind::InvalidInput {
                        "invalid_arguments"
                    } else {
                        "orch_queue_unavailable"
                    },
                    error.to_string(),
                )
            });
        if result["state"] == "queued" {
            let _ = crate::wake::request(true);
        }
        Some(result)
    }

    pub(crate) fn reconcile_request(&self, tool: &str, args: &Value) -> Option<Value> {
        let action = args["action"].as_str()?;
        let call_id = validate_call_id(args).ok()?;
        let (operation, input) = match (tool, action) {
            ("orch_works", "read") => {
                let (name, key) = if args.get("rebind_front_chat_id").is_some() {
                    ("rebind_front_chat_id", "front_chat_id")
                } else {
                    ("rebind_front_marker", "front_marker")
                };
                let value = args[name].as_str()?.trim();
                (
                    "orch_work:rebind_front".to_owned(),
                    json!({"action":"rebind_front","call_id":call_id,
                    "work_id":args["work_id"],(key):value}),
                )
            }
            ("orch_work" | "orch_dispatch", _) => (format!("{tool}:{action}"), args.clone()),
            _ => return None,
        };
        let fingerprint = sha256_hex(json!([operation, input]).to_string().as_bytes());
        let receipt = self.open_receipts().ok()?.inspect(call_id, &[&fingerprint]);
        (receipt["state"] != "not_found").then_some(receipt)
    }

    fn open_receipts(&self) -> Result<ReceiptStore, &'static str> {
        ReceiptStore::open(
            self.receipt_dir.clone(),
            "termux-orch-mcp-receipt",
            "orch_call_id_payload_conflict",
        )
    }

    fn handle_works(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "work_id",
                "include_terminal",
                "after_seq",
                "limit",
                "call_id",
                "rebind_front_chat_id",
                "rebind_front_marker",
            ],
        ) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail(
                "invalid_arguments",
                "'action' must be list|read|checkpoint|inbox",
            );
        };
        let binding = [
            ("rebind_front_chat_id", "front_chat_id"),
            ("rebind_front_marker", "front_marker"),
        ]
        .into_iter()
        .filter_map(|(key, target)| obj.get(key).map(|value| (target, value)))
        .collect::<Vec<_>>();
        if binding.len() > 1 || (!binding.is_empty() && action != "read") {
            return fail(
                "invalid_arguments",
                "rebind requires a specific read and exactly one Front identifier",
            );
        }
        match action {
            "list" => {
                let include_terminal = obj
                    .get("include_terminal")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let limit = obj
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(20)
                    .clamp(1, 100) as usize;
                let works = match self.ledger.list_works() {
                    Ok(w) => w,
                    Err(e) => return fail("ledger_read_failed", e.to_string()),
                };
                let items: Vec<Value> = works
                    .into_iter()
                    .filter(|w| include_terminal || !w.is_terminal())
                    .take(limit)
                    .map(|w| {
                        let cp_preview = self
                            .ledger
                            .read_checkpoint(&w.work_id)
                            .ok()
                            .flatten()
                            .map(|c| truncate_chars(c.trim(), 240));
                        json!({
                            "work_id": w.work_id,
                            "title": w.title,
                            "tier": w.tier,
                            "state": w.state,
                            "project_id": w.project_id,
                            "project_name": w.project_name,
                            "front_chat_id": w.front.latest_chat_id,
                            "front_generation": w.front.generation,
                            "lead": w.lead.as_ref().map(|l| json!({
                                "kind": l.kind,
                                "ref_id": l.ref_id,
                                "generation": l.generation
                            })),
                            "participant_count": w.participants.len(),
                            "active_workers": w.active_worker_count(),
                            "pending_deliveries": w.pending_deliveries.len(),
                            "wait_armed": w.wait_set.as_ref().is_some_and(|ws| ws.armed),
                            "wakeups_used": w.budget.wakeups_used,
                            "max_wakeups": w.budget.max_wakeups,
                            "updated_at_unix": w.updated_at_unix,
                            "checkpoint_preview": cp_preview
                        })
                    })
                    .collect();
                json!({
                    "state": "ok",
                    "count": items.len(),
                    "works": items
                })
            }
            "read" => {
                let Some(wid) = obj
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return fail("invalid_work_id", "valid work_id is required");
                };
                if let Some((target, value)) = binding.first() {
                    let Some(value) = value.as_str().map(str::trim).filter(|value| {
                        !value.is_empty()
                            && value.len() <= if *target == "front_chat_id" { 128 } else { 160 }
                    }) else {
                        return fail("invalid_arguments", "invalid Front identifier");
                    };
                    let bind = json!({"action":"rebind_front","work_id":wid,"call_id":obj.get("call_id"),(*target):value});
                    let outcome = self.handle_work(&bind);
                    if outcome["state"] != "ok" {
                        return outcome;
                    }
                    let Ok(current) = self.ledger.load_work(wid) else {
                        return fail("work_not_found", "Work disappeared after Front update");
                    };
                    if !current.account_id.as_deref().is_some_and(|id| {
                        self.owners
                            .chatgpt_account_id()
                            .is_ok_and(|account| account == id)
                    }) {
                        return fail(
                            "account_unavailable",
                            "The pinned account could not be confirmed",
                        );
                    }
                }
                match self.ledger.load_work(wid) {
                    Ok(work) => {
                        let cp = self.ledger.read_checkpoint(wid).ok().flatten();
                        json!({
                            "state": "ok",
                            "work": work,
                            "checkpoint_md": cp
                        })
                    }
                    Err(e) => fail("work_not_found", e.to_string()),
                }
            }
            "checkpoint" => {
                let Some(wid) = obj
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return fail("invalid_work_id", "valid work_id is required");
                };
                match self.ledger.read_checkpoint(wid) {
                    Ok(cp) => json!({
                        "state": "ok",
                        "work_id": wid,
                        "checkpoint_md": cp.unwrap_or_default()
                    }),
                    Err(e) => fail("checkpoint_read_failed", e.to_string()),
                }
            }
            "inbox" => {
                let Some(wid) = obj
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return fail("invalid_work_id", "valid work_id is required");
                };
                let after_seq = obj.get("after_seq").and_then(Value::as_u64).unwrap_or(0);
                let limit = obj
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(30)
                    .clamp(1, 100) as usize;
                let work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return fail("work_not_found", e.to_string()),
                };
                let read = if obj.contains_key("after_seq") {
                    self.ledger.read_first_events(wid, after_seq, limit)
                } else {
                    self.ledger.read_events(wid, after_seq, limit)
                };
                let events = match read {
                    Ok(events) => events,
                    Err(error) => return fail("inbox_read_failed", error.to_string()),
                };
                json!({
                    "state": "ok",
                    "work_id": wid,
                    "consumed_seq": work.consumed_seq,
                    "last_seq": work.last_seq,
                    "events": events
                })
            }
            other => fail("invalid_arguments", format!("unsupported action '{other}'")),
        }
    }

    fn with_call_id_receipt<F>(&self, args: &Value, op_tag: &str, f: F) -> Value
    where
        F: FnOnce() -> (Value, bool),
    {
        let call_id = match validate_call_id(args) {
            Ok(id) => id,
            Err(_) => {
                return fail(
                    "orch_call_id_invalid",
                    "call_id must be a 1..=160 byte string without NUL",
                )
            }
        };
        let admission_lock = if self.admission_only {
            let check = (|| {
                let requests = crate::requests::Requests::open(&self.ledger)?;
                let lock = requests.admission_lock()?;
                let taken =
                    requests.status(&sha256_hex(call_id.as_bytes()))?["state"] != "not_found";
                Ok::<_, std::io::Error>((lock, taken))
            })();
            match check {
                Ok((_, true)) => {
                    return fail(
                        "orch_call_id_payload_conflict",
                        "call_id belongs to an owner operation",
                    )
                }
                Ok((lock, false)) => Some(lock),
                Err(error) => return fail("orch_queue_unavailable", error.to_string()),
            }
        } else {
            None
        };
        let store = match self.open_receipts() {
            Ok(s) => s,
            Err(code) => return fail(code, "cannot open orch receipt store"),
        };
        let fp = sha256_hex(json!([op_tag, args]).to_string().as_bytes());
        let existing = store.inspect(call_id, &[&fp]);
        if existing["state"] != "not_found" {
            return existing;
        }
        let receipt_path = match store.claim(call_id, &fp, &[&fp]) {
            ReceiptClaim::Acquired(p) => p,
            ReceiptClaim::Existing(v) => return v,
            ReceiptClaim::Rejected(code) => return fail(code, "cannot claim call_id receipt"),
        };
        drop(admission_lock);
        let (outcome, commit_outcome) = f();
        if commit_outcome {
            if let Err(error) = store.commit(&receipt_path, &fp, &outcome) {
                return fail("orch_receipt_commit_failed", format!("operation outcome is retained but receipt settlement failed; reconcile call_id {call_id}: {error}"));
            }
            store.prune_settled(512);
        } else if let Err(error) = store.release_intent(&receipt_path) {
            return fail("orch_receipt_release_failed", format!("pre-dispatch failure cleanup requires reconciliation for call_id {call_id}: {error}"));
        }
        outcome
    }
}

impl McpHandler for OrchMcpHandler {
    fn server_name(&self) -> &str {
        "TermuxOrch"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<ToolDef> {
        build_static_tools()
    }

    fn call_tool(&mut self, name: &str, arguments: &Value) -> (Value, bool) {
        let val = self
            .owner_request(name, arguments)
            .unwrap_or_else(|| match name {
                "orch_works" => self.handle_works(arguments),
                "orch_work" => self.handle_work(arguments),
                "orch_dispatch" => self.handle_dispatch(arguments),
                "orch_report" => self.handle_report(arguments),
                "orch_catalog" => {
                    let mut cat = self.extensions.catalog_json();
                    cat["state"] = json!("ok");
                    cat
                }
                "orch_call" => self.handle_call(arguments),
                other => fail("unknown_tool", format!("unknown tool '{other}'")),
            });
        let is_error = matches!(
            val.get("state").and_then(Value::as_str),
            Some("rejected" | "error")
        );
        (val, is_error)
    }
}

fn fail(code: &str, detail: impl Into<String>) -> Value {
    json!({
        "state": "rejected",
        "error": code,
        "detail": detail.into()
    })
}

fn project_label<'a>(obj: &'a Map<String, Value>, field: &str) -> Result<Option<&'a str>, String> {
    obj.get(field)
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty() && s.len() <= 120 && !s.chars().any(char::is_control))
                .ok_or_else(|| {
                    format!("{field} must be 1..=120 UTF-8 bytes without control characters")
                })
        })
        .transpose()
}

fn parse_session_prefixes(obj: &Map<String, Value>) -> Result<Option<SessionPrefixes>, String> {
    let Some(value) = obj.get("session_prefixes") else {
        return Ok(None);
    };
    let prefixes: SessionPrefixes =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if [
        &prefixes.front,
        &prefixes.lead,
        &prefixes.worker,
        &prefixes.verifier,
    ]
    .into_iter()
    .flatten()
    .any(|p| p.len() > 80 || p.chars().any(char::is_control))
    {
        return Err(
            "session prefixes must be at most 80 UTF-8 bytes without control characters".to_owned(),
        );
    }
    Ok(Some(prefixes))
}

fn same_workspace_root(cwd: Option<&str>, root: &std::path::Path) -> bool {
    cwd.and_then(|path| std::fs::canonicalize(path).ok())
        .is_some_and(|path| path == root)
}

fn check_allowed_keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported argument key '{key}'"));
        }
    }
    Ok(())
}

fn check_lead_fence(
    work: &WorkRecord,
    args: &Map<String, Value>,
    fence_ref: bool,
) -> Result<(), Value> {
    let generation = args.get("expected_lead_generation");
    let reference = args.get("lead_ref_id").filter(|_| fence_ref);
    if generation.is_none() && reference.is_none() {
        return Ok(());
    }
    let Some(lead) = work.lead.as_ref() else {
        return Err(fail("stale_lead_generation", "no current Lead is bound"));
    };
    if generation.is_some_and(|value| value.as_u64() != Some(u64::from(lead.generation)))
        || reference.is_some_and(|value| value.as_str() != Some(lead.ref_id.as_str()))
    {
        return Err(fail(
            "stale_lead_generation",
            format!("current Lead generation is {}", lead.generation),
        ));
    }
    Ok(())
}
