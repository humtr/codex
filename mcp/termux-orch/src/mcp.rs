//! `@TermuxOrch` 6-tool MCP surface (`termux-orch mcp`).
//!
//! Static core tools (uniform immutable annotations `readOnlyHint=true`,
//! `destructiveHint=false`, `openWorldHint=false`):
//! 1. `orch_works`    — `list | read | checkpoint | inbox`
//! 2. `orch_work`     — `create | promote | rebind_front | update_scope | cancel`
//! 3. `orch_dispatch` — `register_worker | register_verifier | wait_set | wait_clear`
//! 4. `orch_report`   — `milestone | need_user | complete | fail`
//! 5. `orch_catalog`  — lists strictly disjoint extension actions
//! 6. `orch_call`     — invokes allowlisted extension action (`lead_rollover`, `budget_extend`,
//!    `reconcile_delivery`, `relay_status`, `rename_session`)

use crate::ledger::{
    format_front_title, format_lead_title, format_participant_title, format_project_name,
    generate_id, is_safe_work_id, now_unix, truncate_chars, FrontBinding, FrontHistoryEntry,
    LeadBinding, LeadHistoryEntry, Ledger, Participant, PendingDelivery, WaitSet, WorkBudget,
    WorkRecord, WORK_SCHEMA,
};
use crate::owners::OwnerClient;
use crate::relay::{enqueue_delivery_if_absent, RelayEngine};
use mcp_kit::{
    sha256_hex, validate_call_id, ExtensionRegistry, ExtensionSpec, McpHandler, ReceiptClaim,
    ReceiptStore, ToolDef,
};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

pub const STATIC_TOOL_NAMES: &[&str] = &[
    "orch_works",
    "orch_work",
    "orch_dispatch",
    "orch_report",
    "orch_catalog",
    "orch_call",
];

pub fn build_static_tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "orch_works",
            "Read-only dashboard for durable Works (both Tier 1 direct long tasks and Tier 3 multi-role F/L/W/V/R orchestrations): list active/recent works, read a work's full status & participants, read checkpoint.md, or inspect Typed Inbox events.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "read", "checkpoint", "inbox"] },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96, "description": "Required for read, checkpoint, and inbox" },
                    "include_terminal": { "type": "boolean", "description": "Include completed/failed/cancelled works on action=list (default true)" },
                    "after_seq": { "type": "integer", "minimum": 0, "description": "Event sequence cursor for action=inbox (default 0)" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "orch_work",
            "Create or manage a durable Work lifecycle: create (Tier 1 direct or Tier 3 orchestrated with auto ChatGPT Project & [F1]/[L1] titles), promote (Tier 1 -> Tier 3 in place), rebind_front (F1 -> Fn seamless mobile continuation), update_scope, or cancel.",
            json!({
                "type": "object",
                "properties": {
                    "expected_lead_generation": { "type": "integer", "minimum": 1, "maximum": 4294967295u32 },
                    "action": { "type": "string", "enum": ["create", "promote", "rebind_front", "update_scope", "cancel"] },
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160, "description": "Idempotency key required on create/promote/rebind_front/update_scope/cancel" },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 120 },
                    "goal": { "type": "string", "minLength": 1, "maxLength": 8192 },
                    "tier": { "type": "string", "enum": ["direct", "orchestrated"] },
                    "completion_criteria": { "type": "array", "items": { "type": "string", "maxLength": 300 }, "maxItems": 20 },
                    "project_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "front_chat_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "front_marker": { "type": "string", "minLength": 1, "maxLength": 160, "description": "Unique text marker in the current mobile/web chat to resolve chat_id automatically" },
                    "delivery_mode": { "type": "string", "enum": ["inject_and_notify", "notify_only"] },
                    "lead_kind": { "type": "string", "enum": ["chatgpt", "codex"] },
                    "lead_profile": { "type": "string", "minLength": 1, "maxLength": 64 },
                    "lead_model": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "lead_reasoning_effort": { "type": "string", "minLength": 1, "maxLength": 32 },
                    "lead_cwd": { "type": "string", "minLength": 1, "maxLength": 512 },
                    "lead_ref_id": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Optional pre-created Lead chat_id or thread_id" },
                    "lead_rollover_turns": { "type": "integer", "minimum": 0, "maximum": 1000 },
                    "max_wakeups": { "type": "integer", "minimum": 1, "maximum": 200 },
                    "max_wall_secs": { "type": "integer", "minimum": 60, "maximum": 86400 },
                    "max_concurrent_workers": { "type": "integer", "minimum": 1, "maximum": 16 },
                    "reason": { "type": "string", "maxLength": 1000 }
                },
                "required": ["action", "call_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "orch_dispatch",
            "Register or spawn Worker (W) / Verifier (V) participants with uniform [W1-slug]/[V1-slug] titles, or arm/clear declarative Wait-Set triggers (all_of Fan-in Barrier, any_of, on_user_event, deadline_secs) so Lead/Front can cleanly end its turn and be woken automatically by Runtime (R).",
            json!({
                "type": "object",
                "properties": {
                    "expected_lead_generation": { "type": "integer", "minimum": 1, "maximum": 4294967295u32 },
                    "lead_ref_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "action": { "type": "string", "enum": ["register_worker", "register_verifier", "wait_set", "wait_clear"] },
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160 },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96 },
                    "backend": { "type": "string", "enum": ["codex", "chatgpt", "shell"] },
                    "ref_id": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Existing thread_id or chat_id; omit and pass prompt to spawn via owner" },
                    "profile": { "type": "string", "minLength": 1, "maxLength": 64 },
                    "slug": { "type": "string", "minLength": 1, "maxLength": 24 },
                    "task_summary": { "type": "string", "minLength": 1, "maxLength": 500 },
                    "owner_scope": { "type": "string", "maxLength": 256 },
                    "command": { "type": "string", "minLength": 1, "maxLength": 65536, "description": "Detached shell command for backend=shell" },
                    "prompt": { "type": "string", "minLength": 1, "maxLength": 32768 },
                    "cwd": { "type": "string", "minLength": 1, "maxLength": 512 },
                    "model": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "reasoning_effort": { "type": "string", "minLength": 1, "maxLength": 32 },
                    "approval_policy": { "type": "string", "enum": ["untrusted", "on-failure", "on-request", "never"] },
                    "wake_target": { "type": "string", "enum": ["lead", "front"] },
                    "all_of": { "type": "array", "items": { "type": "string", "maxLength": 96 }, "maxItems": 32 },
                    "any_of": { "type": "array", "items": { "type": "string", "maxLength": 96 }, "maxItems": 32 },
                    "on_user_event": { "type": "boolean" },
                    "deadline_secs": { "type": "integer", "minimum": 5, "maximum": 86400 },
                    "checkpoint_md": { "type": "string", "maxLength": 65536 }
                },
                "required": ["action", "call_id", "work_id"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "orch_report",
            "Emit a structured milestone, need_user blocker, completion, or failure report from Lead/Runtime to the latest bound Front (F1..Fn) with checkpoint.md persistence, inject_and_notify delivery, and Android notification.",
            json!({
                "type": "object",
                "properties": {
                    "expected_lead_generation": { "type": "integer", "minimum": 1, "maximum": 4294967295u32 },
                    "lead_ref_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "action": { "type": "string", "enum": ["milestone", "need_user", "complete", "fail"] },
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160 },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96 },
                    "summary": { "type": "string", "minLength": 1, "maxLength": 8192 },
                    "checkpoint_md": { "type": "string", "maxLength": 65536 },
                    "evidence": { "type": "array", "items": { "type": "string", "maxLength": 400 }, "maxItems": 20 },
                    "flush_now": { "type": "boolean", "description": "Attempt immediate delivery tick (default true)" }
                },
                "required": ["action", "call_id", "work_id", "summary"],
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "orch_catalog",
            "List all allowlisted extension actions callable via orch_call with exact JSON input schemas (strictly disjoint from static tools).",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ToolDef::new(
            "orch_call",
            "Invoke an allowlisted extension action discovered via orch_catalog. Validation failures return the exact expected JSON schema.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "minLength": 1, "maxLength": 64 },
                    "arguments": { "type": "object" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            false,
        ),
    ]
}

pub fn build_extension_registry() -> ExtensionRegistry {
    let mut reg = ExtensionRegistry::new(STATIC_TOOL_NAMES.iter().copied());
    let specs = [
        ExtensionSpec::new(
            "lead_rollover",
            "Roll over Lead L<n> -> L<n+1> in the pinned project/profile with checkpoint.md handoff and generation fencing.",
            json!({
                "type": "object",
                "properties": {
                    "expected_lead_generation": { "type": "integer", "minimum": 1, "maximum": 4294967295u32 },
                    "lead_ref_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "reason": { "type": "string", "maxLength": 240 },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96 },
                    "checkpoint_md": { "type": "string", "maxLength": 65536 },
                    "lead_kind": { "type": "string", "enum": ["chatgpt", "codex"] },
                    "lead_profile": { "type": "string", "maxLength": 64 },
                    "lead_model": { "type": "string", "maxLength": 128 },
                    "lead_reasoning_effort": { "type": "string", "maxLength": 32 }
                },
                "required": ["work_id"],
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "budget_extend",
            "Extend max_wakeups, max_wall_secs, or max_concurrent_workers on an existing Work and optionally resume waiting_user state.",
            json!({
                "type": "object",
                "properties": {
                    "expected_lead_generation": { "type": "integer", "minimum": 1, "maximum": 4294967295u32 },
                    "lead_ref_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96 },
                    "add_wakeups": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "add_wall_secs": { "type": "integer", "minimum": 60, "maximum": 86400 },
                    "max_concurrent_workers": { "type": "integer", "minimum": 1, "maximum": 16 }
                },
                "required": ["work_id"],
                "additionalProperties": false
            }),
            false,
        ),
        ExtensionSpec::new(
            "reconcile_delivery",
            "Run one immediate Runtime (R) tick to observe participants, evaluate Wait-Sets, and flush pending Front/Lead deliveries.",
            json!({
                "type": "object",
                "properties": {
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96 }
                },
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "relay_status",
            "Inspect durable state root, total active works, pending deliveries, and chatgpt-provider reachability.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            true,
        ),
        ExtensionSpec::new(
            "rename_session",
            "Re-apply a consistent role title to a bound ChatGPT chat or Codex thread.",
            json!({
                "type": "object",
                "properties": {
                    "backend": { "type": "string", "enum": ["chatgpt", "codex"] },
                    "ref_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "profile": { "type": "string", "maxLength": 64 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 180 }
                },
                "required": ["backend", "ref_id", "title"],
                "additionalProperties": false
            }),
            true,
        ),
    ];
    for s in specs {
        let _ = reg.register(s);
    }
    reg
}

pub struct OrchMcpHandler {
    ledger: Ledger,
    owners: OwnerClient,
    receipt_dir: PathBuf,
    extensions: ExtensionRegistry,
}

impl OrchMcpHandler {
    pub fn new(ledger: Ledger, owners: OwnerClient, receipt_dir: PathBuf) -> Self {
        Self {
            ledger,
            owners,
            receipt_dir,
            extensions: build_extension_registry(),
        }
    }

    fn open_receipts(&self) -> Result<ReceiptStore, &'static str> {
        ReceiptStore::open(
            self.receipt_dir.clone(),
            "termux-orch-mcp-receipt",
            "orch_call_id_payload_conflict",
        )
    }

    fn resolve_front_chat_by_marker(&self, marker: &str) -> Result<String, String> {
        let listed = self.owners.chatgpt_list_chats(10)?;
        let chats = listed
            .get("chats")
            .and_then(Value::as_array)
            .ok_or_else(|| "chatgpt_list_chats returned no chats array".to_owned())?;
        for c in chats {
            let Some(cid) = c.get("id").and_then(Value::as_str) else {
                continue;
            };
            if c.get("title")
                .and_then(Value::as_str)
                .is_some_and(|t| t.contains(marker))
            {
                return Ok(cid.to_owned());
            }
            if let Ok(rd) = self.owners.chatgpt_read_chat(cid, 6) {
                if let Some(msgs) = rd.get("messages").and_then(Value::as_array) {
                    if msgs.iter().any(|m| {
                        m.get("text")
                            .and_then(Value::as_str)
                            .is_some_and(|t| t.contains(marker))
                    }) {
                        return Ok(cid.to_owned());
                    }
                }
            }
        }
        Err(format!(
            "front_marker_not_found: no recent ChatGPT chat contained marker '{marker}'"
        ))
    }

    fn current_account_id(&self) -> Option<String> {
        let status = self.owners.chatgpt_status().ok()?;
        crate::owners::chatgpt_account_identity(&status).map(ToOwned::to_owned)
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
        let (outcome, commit_outcome) = f();
        if commit_outcome {
            let _ = store.commit(&receipt_path, &fp, &outcome);
            store.prune_settled(512);
        } else {
            let _ = store.release_intent(&receipt_path);
        }
        outcome
    }

    fn handle_work(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "call_id",
                "work_id",
                "expected_lead_generation",
                "title",
                "goal",
                "tier",
                "completion_criteria",
                "project_id",
                "front_chat_id",
                "front_marker",
                "delivery_mode",
                "lead_kind",
                "lead_profile",
                "lead_model",
                "lead_reasoning_effort",
                "lead_cwd",
                "lead_ref_id",
                "max_wakeups",
                "lead_rollover_turns",
                "max_wall_secs",
                "max_concurrent_workers",
                "reason",
            ],
        ) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing action");
        };
        self.with_call_id_receipt(args, &format!("orch_work:{action}"), || match action {
            "create" => self.op_work_create(obj),
            "promote" => self.op_work_promote(obj),
            "rebind_front" => self.op_work_rebind_front(obj),
            "update_scope" => self.op_work_update_scope(obj),
            "cancel" => self.op_work_cancel(obj),
            other => (
                fail("invalid_arguments", format!("unsupported action '{other}'")),
                false,
            ),
        })
    }

    fn op_work_create(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(title) = obj
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 120)
        else {
            return (
                fail(
                    "invalid_arguments",
                    "'title' (1..=120 chars) is required for action=create",
                ),
                false,
            );
        };
        let Some(goal) = obj
            .get("goal")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 8192)
        else {
            return (
                fail(
                    "invalid_arguments",
                    "'goal' (1..=8192 chars) is required for action=create",
                ),
                false,
            );
        };
        let tier = obj
            .get("tier")
            .and_then(Value::as_str)
            .unwrap_or("orchestrated");
        if tier != "direct" && tier != "orchestrated" {
            return (
                fail("invalid_arguments", "'tier' must be direct|orchestrated"),
                false,
            );
        }
        let delivery_mode = obj
            .get("delivery_mode")
            .and_then(Value::as_str)
            .unwrap_or("inject_and_notify");
        if delivery_mode != "inject_and_notify" && delivery_mode != "notify_only" {
            return (
                fail(
                    "invalid_arguments",
                    "'delivery_mode' must be inject_and_notify|notify_only",
                ),
                false,
            );
        }
        let criteria: Vec<String> = obj
            .get("completion_criteria")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| truncate_chars(s.trim(), 300))
                    .filter(|s| !s.is_empty())
                    .take(20)
                    .collect()
            })
            .unwrap_or_default();

        let account_id = self.current_account_id();

        // Resolve or auto-create ChatGPT Project (`[Work] <title>`).
        let mut project_id = obj
            .get("project_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        let mut project_name = project_id.as_ref().map(|_| format_project_name(title));
        let mut warnings: Vec<String> = Vec::new();
        if project_id.is_none() {
            let pname = format_project_name(title);
            let instructions = format!(
                "Durable Work project for: {title}\nGoal: {}\nRoles: [F#]=Front, [L#]=Lead, [W#-slug]=Worker, [V#-slug]=Verifier.",
                truncate_chars(goal, 1000)
            );
            match self
                .owners
                .chatgpt_create_project(&pname, Some(&instructions))
            {
                Ok(res) => {
                    if let Some(pid) = res.get("project_id").and_then(Value::as_str) {
                        project_id = Some(pid.to_owned());
                        project_name = Some(
                            res.get("name")
                                .and_then(Value::as_str)
                                .unwrap_or(&pname)
                                .to_owned(),
                        );
                    }
                }
                Err(e) => warnings.push(format!("project_create_skipped: {e}")),
            }
        }

        // Resolve optional Front chat (explicit id or marker) and apply `[F1] <title>`.
        let mut front_chat_id = obj
            .get("front_chat_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        if front_chat_id.is_none() {
            if let Some(marker) = obj
                .get("front_marker")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                match self.resolve_front_chat_by_marker(marker) {
                    Ok(cid) => front_chat_id = Some(cid),
                    Err(e) => warnings.push(e),
                }
            }
        }

        let now = now_unix();
        let work_id = generate_id("work");
        let mut front_history = Vec::new();
        if let Some(ref fcid) = front_chat_id {
            let ftitle = format_front_title(1, title);
            if let Err(e) = self.owners.chatgpt_rename_chat(fcid, &ftitle) {
                warnings.push(format!("front_rename_skipped: {e}"));
            }
            front_history.push(FrontHistoryEntry {
                generation: 1,
                chat_id: fcid.clone(),
                bound_at_unix: now,
            });
        }

        let mut budget = WorkBudget::default();
        if let Some(turns) = obj.get("lead_rollover_turns").and_then(Value::as_u64) {
            budget.lead_rollover_turns = turns.min(1000) as u32;
        }
        if let Some(mw) = obj.get("max_wakeups").and_then(Value::as_u64) {
            budget.max_wakeups = mw.clamp(1, 200) as u32;
        }
        if let Some(ms) = obj.get("max_wall_secs").and_then(Value::as_u64) {
            budget.max_wall_secs = ms.clamp(60, 86_400);
        }
        if let Some(mc) = obj.get("max_concurrent_workers").and_then(Value::as_u64) {
            budget.max_concurrent_workers = mc.clamp(1, 16) as u32;
        }

        // Spawn or bind Lead L1 when tier == "orchestrated".
        let lead = if tier == "orchestrated" {
            match self.spawn_or_bind_lead(
                &work_id,
                title,
                goal,
                &criteria,
                project_id.as_deref(),
                1,
                obj,
                &mut warnings,
            ) {
                Ok(l) => Some(l),
                Err(err_val) => return (err_val, false),
            }
        } else {
            None
        };

        // Project/Lead operations can start a cold AppHost after the first status read.
        let account_id = account_id.or_else(|| self.current_account_id());
        let mut work = WorkRecord {
            schema: WORK_SCHEMA.to_owned(),
            work_id: work_id.clone(),
            title: title.to_owned(),
            tier: tier.to_owned(),
            goal: goal.to_owned(),
            completion_criteria: criteria,
            state: "running".to_owned(),
            account_id,
            project_id: project_id.clone(),
            project_name: project_name.clone(),
            delivery_mode: delivery_mode.to_owned(),
            front: FrontBinding {
                latest_chat_id: front_chat_id.clone(),
                generation: if front_chat_id.is_some() { 1 } else { 0 },
                history: front_history,
            },
            lead: lead.clone(),
            lead_rollover: None,
            lead_replay: None,
            participants: Vec::new(),
            wait_set: None,
            pending_deliveries: Vec::new(),
            consumed_seq: 0,
            last_seq: 0,
            budget,
            created_at_unix: now,
            updated_at_unix: now,
        };

        let _lock = match self.ledger.lock_work(&work_id) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            "work:created",
            "work.created",
            "front",
            &format!("Work '{title}' created (tier={tier})"),
            json!({ "tier": tier, "project_id": project_id }),
        );
        let initial_cp = format!(
            "# Checkpoint: {title}\n\n- work_id: `{work_id}`\n- tier: `{tier}`\n- goal: {goal}\n"
        );
        let _ = self.ledger.write_checkpoint(&work_id, &initial_cp);
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }

        (
            json!({
                "state": "ok",
                "work_id": work_id,
                "title": title,
                "tier": tier,
                "project_id": work.project_id,
                "project_name": work.project_name,
                "front_chat_id": work.front.latest_chat_id,
                "lead": work.lead,
                "delivery_mode": work.delivery_mode,
                "warnings": warnings
            }),
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_or_bind_lead(
        &self,
        work_id: &str,
        title: &str,
        goal: &str,
        criteria: &[String],
        project_id: Option<&str>,
        generation: u32,
        obj: &Map<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<LeadBinding, Value> {
        let kind = obj
            .get("lead_kind")
            .and_then(Value::as_str)
            .unwrap_or("chatgpt");
        if kind != "chatgpt" && kind != "codex" {
            return Err(fail(
                "invalid_arguments",
                "'lead_kind' must be chatgpt|codex",
            ));
        }
        let profile = obj
            .get("lead_profile")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let model = obj
            .get("lead_model")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let effort = obj
            .get("lead_reasoning_effort")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let cwd = obj.get("lead_cwd").and_then(Value::as_str);
        let lead_title = format_lead_title(generation, title);

        let ref_id = if let Some(existing) = obj
            .get("lead_ref_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            existing.to_owned()
        } else {
            let criteria_text = if criteria.is_empty() {
                "(none specified)".to_owned()
            } else {
                criteria
                    .iter()
                    .map(|c| format!("- {c}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let lead_prompt = format!(
                "You are Lead [L{generation}] for durable Work `{work_id}` (\"{title}\").\n\
                 Goal: {goal}\n\
                 Completion criteria:\n{criteria_text}\n\n\
                 Use `@TermuxCodex` / `@TermuxChatGPT` / `@TermuxShell` to dispatch scoped Workers (W) and Verifiers (V), \
                 register them and arm a Wait-Set with `@TermuxOrch` (`orch_dispatch`), then end your turn so Runtime (R) wakes you automatically. \
                 Report milestones, blockers, and final verified completion via `orch_report`."
            );
            match kind {
                "chatgpt" => {
                    let sent = self
                        .owners
                        .chatgpt_send(
                            None,
                            project_id,
                            &lead_prompt,
                            model.as_deref(),
                            effort.as_deref(),
                            10_000,
                        )
                        .map_err(|e| fail("lead_spawn_failed", e))?;
                    sent.get("chat_id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            fail(
                                "lead_spawn_failed",
                                "chatgpt_send did not return a chat_id for Lead",
                            )
                        })?
                        .to_owned()
                }
                "codex" => {
                    let (tid, _) = self
                        .owners
                        .codex_dispatch_turn(
                            profile.as_deref(),
                            None,
                            cwd,
                            &lead_prompt,
                            model.as_deref(),
                            None,
                            Some("never"),
                        )
                        .map_err(|e| fail("lead_spawn_failed", e))?;
                    tid
                }
                _ => unreachable!(),
            }
        };

        // Apply uniform `[L<gen>] <title>` session name.
        match kind {
            "chatgpt" => {
                if let Err(e) = self.owners.chatgpt_rename_chat(&ref_id, &lead_title) {
                    warnings.push(format!("lead_rename_skipped: {e}"));
                }
            }
            "codex" => {
                if let Err(e) =
                    self.owners
                        .codex_rename_thread(profile.as_deref(), &ref_id, &lead_title)
                {
                    warnings.push(format!("lead_rename_skipped: {e}"));
                }
            }
            _ => {}
        }

        let now = now_unix();
        Ok(LeadBinding {
            kind: kind.to_owned(),
            ref_id: ref_id.clone(),
            profile,
            model,
            reasoning_effort: effort,
            generation,
            rollover_internal_turns: 0,
            history: vec![LeadHistoryEntry {
                generation,
                kind: kind.to_owned(),
                ref_id,
                started_at_unix: now,
            }],
        })
    }

    fn op_work_promote(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (
                fail("invalid_work_id", "valid work_id is required for promote"),
                false,
            );
        };
        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, work.lead.is_some()) {
            return (e, false);
        }
        if work.tier == "orchestrated" && work.lead.is_some() {
            return (
                json!({
                    "state": "ok",
                    "work_id": work.work_id,
                    "tier": "orchestrated",
                    "already_promoted": true,
                    "lead": work.lead
                }),
                true,
            );
        }
        let mut warnings = Vec::new();
        let lead = match self.spawn_or_bind_lead(
            &work.work_id,
            &work.title,
            &work.goal,
            &work.completion_criteria,
            work.project_id.as_deref(),
            1,
            obj,
            &mut warnings,
        ) {
            Ok(l) => l,
            Err(err_val) => return (err_val, false),
        };
        work.tier = "orchestrated".to_owned();
        work.lead = Some(lead.clone());
        if let Some(ws) = work.wait_set.as_mut() {
            if ws.armed && ws.wake_target == "front" {
                ws.wake_target = "lead".to_owned();
            }
        }
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            "work:promoted:1",
            "work.promoted",
            "front",
            &format!(
                "Promoted Tier 1 direct work to Tier 3 orchestrated ({}:{})",
                lead.kind, lead.ref_id
            ),
            json!({ "lead": lead }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "tier": work.tier,
                "lead": work.lead,
                "preserved_participants": work.participants.len(),
                "warnings": warnings
            }),
            true,
        )
    }

    fn op_work_rebind_front(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (
                fail(
                    "invalid_work_id",
                    "valid work_id is required for rebind_front",
                ),
                false,
            );
        };
        let resolved_chat_id = if let Some(cid) = obj
            .get("front_chat_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            cid.to_owned()
        } else if let Some(marker) = obj
            .get("front_marker")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            match self.resolve_front_chat_by_marker(marker) {
                Ok(cid) => cid,
                Err(e) => return (fail("front_resolve_failed", e), false),
            }
        } else {
            return (
                fail(
                    "invalid_arguments",
                    "rebind_front requires front_chat_id or front_marker",
                ),
                false,
            );
        };

        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, true) {
            return (e, false);
        }

        // Pin account_id check (fail closed on cross-account mismatch).
        if let (Some(pinned), Some(current)) = (&work.account_id, self.current_account_id()) {
            if pinned != &current {
                return (
                    fail(
                        "account_mismatch",
                        format!("work belongs to account '{pinned}', current app is '{current}'"),
                    ),
                    false,
                );
            }
        }

        let now = now_unix();
        let next_gen = work.front.generation + 1;
        work.front.generation = next_gen;
        work.front.latest_chat_id = Some(resolved_chat_id.clone());
        work.front.history.push(FrontHistoryEntry {
            generation: next_gen,
            chat_id: resolved_chat_id.clone(),
            bound_at_unix: now,
        });

        let mut warnings = Vec::new();
        let ftitle = format_front_title(next_gen, &work.title);
        if let Err(e) = self.owners.chatgpt_rename_chat(&resolved_chat_id, &ftitle) {
            warnings.push(format!("front_rename_skipped: {e}"));
        }

        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("rebind:{next_gen}:{resolved_chat_id}"),
            "user.front_rebound",
            &format!("F{next_gen}"),
            &format!("Front rebound to F{next_gen} ({resolved_chat_id})"),
            json!({ "generation": next_gen, "chat_id": resolved_chat_id }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        let cp = self.ledger.read_checkpoint(wid).ok().flatten();
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "front_generation": next_gen,
                "front_chat_id": resolved_chat_id,
                "front_title": ftitle,
                "work_state": work.state,
                "tier": work.tier,
                "participants": work.participants,
                "checkpoint_md": cp,
                "warnings": warnings
            }),
            true,
        )
    }

    fn op_work_update_scope(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, true) {
            return (e, false);
        }
        if let Some(g) = obj
            .get("goal")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            work.goal = g.to_owned();
        }
        if let Some(arr) = obj.get("completion_criteria").and_then(Value::as_array) {
            work.completion_criteria = arr
                .iter()
                .filter_map(Value::as_str)
                .map(|s| truncate_chars(s.trim(), 300))
                .filter(|s| !s.is_empty())
                .take(20)
                .collect();
        }
        let reason = obj
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("scope updated by user");
        let cid = obj.get("call_id").and_then(Value::as_str).unwrap_or("upd");
        let scope_data = json!({
            "goal": work.goal,
            "completion_criteria": work.completion_criteria
        });
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("scope:{cid}"),
            "user.scope_updated",
            "front",
            reason,
            scope_data,
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "goal": work.goal,
                "completion_criteria": work.completion_criteria,
                "last_seq": work.last_seq
            }),
            true,
        )
    }

    fn op_work_cancel(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, true) {
            return (e, false);
        }
        work.state = "cancelled".to_owned();
        if let Some(ws) = work.wait_set.as_mut() {
            ws.armed = false;
        }
        work.pending_deliveries.clear();
        let reason = obj
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("cancelled by user");
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            "work:cancelled",
            "work.cancelled",
            "front",
            reason,
            json!({ "reason": reason }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "work_state": "cancelled"
            }),
            true,
        )
    }

    fn handle_dispatch(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "call_id",
                "work_id",
                "expected_lead_generation",
                "lead_ref_id",
                "backend",
                "ref_id",
                "profile",
                "slug",
                "task_summary",
                "owner_scope",
                "prompt",
                "command",
                "cwd",
                "model",
                "reasoning_effort",
                "approval_policy",
                "wake_target",
                "all_of",
                "any_of",
                "on_user_event",
                "deadline_secs",
                "checkpoint_md",
            ],
        ) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing action");
        };
        self.with_call_id_receipt(args, &format!("orch_dispatch:{action}"), || match action {
            "register_worker" => self.op_register_participant(obj, "worker"),
            "register_verifier" => self.op_register_participant(obj, "verifier"),
            "wait_set" => self.op_wait_set(obj),
            "wait_clear" => self.op_wait_clear(obj),
            other => (
                fail("invalid_arguments", format!("unsupported action '{other}'")),
                false,
            ),
        })
    }

    fn op_register_participant(&self, obj: &Map<String, Value>, role: &str) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
        let Some(backend) = obj
            .get("backend")
            .and_then(Value::as_str)
            .filter(|b| matches!(*b, "codex" | "chatgpt" | "shell"))
        else {
            return (
                fail("invalid_arguments", "'backend' must be codex|chatgpt|shell"),
                false,
            );
        };
        let slug = obj
            .get("slug")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if role == "verifier" { "verify" } else { "task" });
        let task_summary = obj
            .get("task_summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("scoped subtask");
        let profile = obj
            .get("profile")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let owner_scope = obj
            .get("owner_scope")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let call_id = obj
            .get("call_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, true) {
            return (e, false);
        }
        if work.is_terminal() {
            return (
                fail(
                    "work_already_terminal",
                    format!("work '{wid}' is already {}", work.state),
                ),
                false,
            );
        }
        if role == "worker" && work.active_worker_count() >= work.budget.max_concurrent_workers {
            return (
                fail(
                    "max_concurrent_workers_reached",
                    format!(
                        "active worker ceiling ({}) reached",
                        work.budget.max_concurrent_workers
                    ),
                ),
                false,
            );
        }

        let index = work.next_role_index(role);
        let tag = if role == "verifier" { "V" } else { "W" };
        let participant_id = format!("{tag}{index}");
        let session_title = format_participant_title(role, index, slug, task_summary);

        let existing = obj
            .get("ref_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let shell_command = obj.get("command").and_then(Value::as_str);
        let prompt = obj.get("prompt").and_then(Value::as_str);
        if shell_command.is_some() && backend != "shell" {
            return (
                fail("invalid_arguments", "command requires backend=shell"),
                false,
            );
        }
        if backend == "shell"
            && shell_command
                .or(prompt)
                .is_some_and(|c| c.trim().is_empty() || c.len() > 65536 || c.contains('\0'))
        {
            return (fail("invalid_arguments", "invalid shell command"), false);
        }
        if backend == "shell"
            && [existing, shell_command, prompt]
                .into_iter()
                .flatten()
                .count()
                != 1
        {
            return (
                fail(
                    "invalid_arguments",
                    "shell worker requires exactly one of ref_id, command, or prompt",
                ),
                false,
            );
        }
        let mut turn_id = None;
        let ref_id = if backend == "shell" {
            let receipt_id = existing.unwrap_or(&participant_id);
            if work
                .participants
                .iter()
                .any(|p| p.backend == "shell" && p.ref_id == receipt_id)
            {
                return (fail("shell_receipt_already_registered", receipt_id), false);
            }
            if let Some(existing) = existing {
                if let Err(e) = crate::shell_job::ShellJob::new(&self.ledger, wid, existing)
                    .and_then(|job| job.validate_attach())
                {
                    return (fail("invalid_shell_receipt", e.to_string()), false);
                }
            }
            receipt_id.to_owned()
        } else if let Some(existing) = existing {
            existing.to_owned()
        } else {
            let Some(prompt) = obj
                .get("prompt")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return (
                    fail(
                        "invalid_arguments",
                        "provide existing 'ref_id' or 'prompt' to spawn participant",
                    ),
                    false,
                );
            };
            match backend {
                "codex" => {
                    match self.owners.codex_dispatch_turn(
                        profile.as_deref(),
                        None,
                        obj.get("cwd").and_then(Value::as_str),
                        prompt,
                        obj.get("model").and_then(Value::as_str),
                        None,
                        obj.get("approval_policy").and_then(Value::as_str),
                    ) {
                        Ok((tid, observed_turn)) => {
                            turn_id = observed_turn;
                            tid
                        }
                        Err(e) => return (fail("participant_spawn_failed", e), false),
                    }
                }
                "chatgpt" => {
                    match self.owners.chatgpt_send(
                        None,
                        work.project_id.as_deref(),
                        prompt,
                        obj.get("model").and_then(Value::as_str),
                        obj.get("reasoning_effort").and_then(Value::as_str),
                        5_000,
                    ) {
                        Ok(val) => match val.get("chat_id").and_then(Value::as_str) {
                            Some(cid) => cid.to_owned(),
                            None => {
                                return (
                                    fail(
                                        "participant_spawn_failed",
                                        "chatgpt_send returned no chat_id",
                                    ),
                                    false,
                                )
                            }
                        },
                        Err(e) => return (fail("participant_spawn_failed", e), false),
                    }
                }
                _ => unreachable!(),
            }
        };

        let mut warnings = Vec::new();
        match backend {
            "chatgpt" => {
                if let Err(e) = self.owners.chatgpt_rename_chat(&ref_id, &session_title) {
                    warnings.push(format!("rename_skipped: {e}"));
                }
            }
            "codex" => {
                if let Err(e) =
                    self.owners
                        .codex_rename_thread(profile.as_deref(), &ref_id, &session_title)
                {
                    warnings.push(format!("rename_skipped: {e}"));
                }
            }
            _ => {}
        }

        let now = now_unix();
        let participant = Participant {
            participant_id: participant_id.clone(),
            index,
            role: role.to_owned(),
            backend: backend.to_owned(),
            ref_id: ref_id.clone(),
            turn_id,
            profile,
            slug: slug.to_owned(),
            task_summary: task_summary.to_owned(),
            owner_scope,
            call_id,
            state: "running".to_owned(),
            result_summary: None,
            created_at_unix: now,
            updated_at_unix: now,
        };
        work.participants.push(participant.clone());
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("registered:{participant_id}:{ref_id}"),
            &format!("{role}.registered"),
            &participant_id,
            &format!("{session_title} ({backend}:{ref_id})"),
            json!({ "participant_id": participant_id, "backend": backend, "ref_id": ref_id }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }

        if backend == "shell" && existing.is_none() {
            let command = shell_command.or(prompt).unwrap_or_default();
            let result =
                crate::shell_job::ShellJob::new(&self.ledger, wid, &ref_id).and_then(|job| {
                    job.spawn(
                        self.ledger.root(),
                        wid,
                        command,
                        obj.get("cwd").and_then(Value::as_str),
                    )
                });
            if let Err(e) = result {
                warnings.push(format!("shell_spawn_failed: {e}"));
                return (
                    json!({
                        "state": "error",
                        "error": "shell_spawn_failed",
                        "work_id": wid,
                        "participant": participant,
                        "warnings": warnings
                    }),
                    true,
                );
            }
        }

        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "participant": participant,
                "session_title": session_title,
                "warnings": warnings
            }),
            true,
        )
    }

    fn op_wait_set(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, true) {
            return (e, false);
        }
        if work.is_terminal() {
            return (
                fail(
                    "work_already_terminal",
                    "cannot arm Wait-Set on terminal work",
                ),
                false,
            );
        }
        let default_target = if work.lead.is_some() { "lead" } else { "front" };
        let wake_target = obj
            .get("wake_target")
            .and_then(Value::as_str)
            .unwrap_or(default_target);
        if wake_target != "lead" && wake_target != "front" {
            return (
                fail("invalid_arguments", "'wake_target' must be lead|front"),
                false,
            );
        }
        if wake_target == "lead" && work.lead.is_none() {
            return (
                fail(
                    "lead_not_bound",
                    "Tier 1 direct work has no Lead; use wake_target='front' or promote first",
                ),
                false,
            );
        }
        let all_of: Vec<String> = obj
            .get("all_of")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let any_of: Vec<String> = obj
            .get("any_of")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let on_user_event = obj
            .get("on_user_event")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let now = now_unix();
        let deadline_unix = obj
            .get("deadline_secs")
            .and_then(Value::as_u64)
            .map(|s| now + s.clamp(5, 86_400));
        if all_of.is_empty() && any_of.is_empty() && deadline_unix.is_none() {
            return (
                fail(
                    "invalid_wait_set",
                    "Wait-Set requires at least one of all_of, any_of, or deadline_secs",
                ),
                false,
            );
        }
        if let Some(cp) = obj.get("checkpoint_md").and_then(Value::as_str) {
            let _ = self.ledger.write_checkpoint(wid, cp);
        }
        let ws = WaitSet {
            wait_id: generate_id("wait"),
            wake_target: wake_target.to_owned(),
            all_of,
            any_of,
            on_user_event,
            deadline_unix,
            armed: true,
            created_at_unix: now,
        };
        work.wait_set = Some(ws.clone());
        work.state = "waiting".to_owned();
        work.updated_at_unix = now;
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "work_state": work.state,
                "wait_set": ws
            }),
            true,
        )
    }

    fn op_wait_clear(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if let Err(e) = check_lead_fence(&work, obj, true) {
            return (e, false);
        }
        if let Some(ws) = work.wait_set.as_mut() {
            ws.armed = false;
        }
        if work.state == "waiting" {
            work.state = "running".to_owned();
        }
        work.updated_at_unix = now_unix();
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "wait_armed": false
            }),
            true,
        )
    }

    fn handle_report(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "call_id",
                "work_id",
                "expected_lead_generation",
                "lead_ref_id",
                "summary",
                "checkpoint_md",
                "evidence",
                "flush_now",
            ],
        ) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing action");
        };
        if !matches!(action, "milestone" | "need_user" | "complete" | "fail") {
            return fail(
                "invalid_arguments",
                format!("unsupported report action '{action}'"),
            );
        }
        self.with_call_id_receipt(args, &format!("orch_report:{action}"), || {
            let Some(wid) = obj
                .get("work_id")
                .and_then(Value::as_str)
                .filter(|s| is_safe_work_id(s))
            else {
                return (fail("invalid_work_id", "valid work_id is required"), false);
            };
            let Some(summary) = obj
                .get("summary")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return (
                    fail("invalid_arguments", "non-empty summary is required"),
                    false,
                );
            };
            let call_id = obj.get("call_id").and_then(Value::as_str).unwrap_or("rep");
            let flush_now = obj
                .get("flush_now")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let evidence: Vec<String> = obj
                .get("evidence")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(|s| truncate_chars(s.trim(), 400))
                        .collect()
                })
                .unwrap_or_default();

            let delivery_id = format!("report:{action}:{call_id}");
            {
                let _lock = match self.ledger.lock_work(wid) {
                    Ok(l) => l,
                    Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
                };
                let mut work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return (fail("work_not_found", e.to_string()), false),
                };
                if let Err(e) = check_lead_fence(&work, obj, true) {
                    return (e, false);
                }
                if let Some(cp) = obj.get("checkpoint_md").and_then(Value::as_str) {
                    let _ = self.ledger.write_checkpoint(wid, cp);
                }
                match action {
                    "need_user" => work.state = "waiting_user".to_owned(),
                    "complete" => {
                        work.state = "completed".to_owned();
                        if let Some(ws) = work.wait_set.as_mut() {
                            ws.armed = false;
                        }
                    }
                    "fail" => {
                        work.state = "failed".to_owned();
                        if let Some(ws) = work.wait_set.as_mut() {
                            ws.armed = false;
                        }
                    }
                    _ => {}
                }
                let ev_lines = if evidence.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\n### Evidence\n{}",
                        evidence
                            .iter()
                            .map(|e| format!("- {e}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };
                let payload_text = format!(
                    "[ORCH_REPORT:{action}] work_id={} title=\"{}\"\n\n{}{}",
                    work.work_id, work.title, summary, ev_lines
                );
                let now = now_unix();
                let del_title = format!("[Work:{action}] {}", work.title);
                enqueue_delivery_if_absent(
                    &mut work,
                    PendingDelivery {
                        delivery_id: delivery_id.clone(),
                        target: "front".to_owned(),
                        kind: action.to_owned(),
                        title: del_title,
                        payload_text,
                        advance_seq_to: None,
                        dispatch: None,
                        receipts_tracked: true,
                        notified: false,
                        attempts: 0,
                        next_attempt_unix: now,
                        last_error: None,
                        created_at_unix: now,
                    },
                );
                let _ = self.ledger.append_event_if_absent(
                    &mut work,
                    &delivery_id,
                    &format!("report.{action}"),
                    "lead",
                    summary,
                    json!({ "evidence": evidence }),
                );
                if let Err(e) = self.ledger.save_work(&work) {
                    return (fail("ledger_write_failed", e.to_string()), false);
                }
            }

            let mut delivered_immediately = false;
            if flush_now {
                let engine = RelayEngine::new(self.ledger.clone(), self.owners.clone());
                let _ = engine.process_work(wid);
                delivered_immediately = self
                    .ledger
                    .delivery_recorded(wid, &delivery_id)
                    .unwrap_or(false);
            }
            let fresh = self.ledger.load_work(wid).ok();
            let pending_left = fresh
                .as_ref()
                .map(|w| w.pending_deliveries.len())
                .unwrap_or(0);
            (
                json!({
                    "state": "ok",
                    "work_id": wid,
                    "report_action": action,
                    "delivery_id": delivery_id,
                    "delivered_immediately": delivered_immediately,
                    "pending_deliveries": pending_left,
                    "work_state": fresh.map(|w| w.state)
                }),
                true,
            )
        })
    }

    fn handle_call(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(obj, &["action", "arguments"]) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing extension action");
        };
        if self.extensions.get(action).is_none() {
            return self.extensions.validation_error(action, "unknown_action");
        }
        let empty = Map::new();
        let ext_args = match obj.get("arguments") {
            None => &empty,
            Some(Value::Object(m)) => m,
            Some(_) => {
                return self
                    .extensions
                    .validation_error(action, "'arguments' must be a JSON object")
            }
        };
        match action {
            "relay_status" => {
                let works = self.ledger.list_works().unwrap_or_default();
                let active = works.iter().filter(|w| !w.is_terminal()).count();
                let pending: usize = works.iter().map(|w| w.pending_deliveries.len()).sum();
                let provider_ok = self.owners.chatgpt_status().is_ok();
                json!({
                    "state": "ok",
                    "state_root": self.ledger.root().display().to_string(),
                    "total_works": works.len(),
                    "active_works": active,
                    "pending_deliveries": pending,
                    "chatgpt_provider_reachable": provider_ok
                })
            }
            "reconcile_delivery" => {
                let engine = RelayEngine::new(self.ledger.clone(), self.owners.clone());
                let tick = engine.tick_once();
                json!({
                    "state": "ok",
                    "works_scanned": tick.works_scanned,
                    "participants_settled": tick.participants_settled,
                    "wait_sets_fired": tick.wait_sets_fired,
                    "deliveries_sent": tick.deliveries_sent,
                    "deliveries_deferred": tick.deliveries_deferred,
                    "budget_trips": tick.budget_trips,
                    "errors": tick.errors
                })
            }
            "budget_extend" => {
                let Some(wid) = ext_args
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return self
                        .extensions
                        .validation_error(action, "valid work_id required");
                };
                let _lock = match self.ledger.lock_work(wid) {
                    Ok(l) => l,
                    Err(e) => return fail("ledger_lock_failed", e.to_string()),
                };
                let mut work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return fail("work_not_found", e.to_string()),
                };
                if let Err(e) = check_lead_fence(&work, ext_args, true) {
                    return e;
                }
                if let Some(add_w) = ext_args.get("add_wakeups").and_then(Value::as_u64) {
                    work.budget.max_wakeups = work
                        .budget
                        .max_wakeups
                        .saturating_add(add_w.clamp(1, 100) as u32);
                }
                if let Some(add_s) = ext_args.get("add_wall_secs").and_then(Value::as_u64) {
                    work.budget.max_wall_secs = work
                        .budget
                        .max_wall_secs
                        .saturating_add(add_s.clamp(60, 86_400));
                }
                if let Some(mc) = ext_args
                    .get("max_concurrent_workers")
                    .and_then(Value::as_u64)
                {
                    work.budget.max_concurrent_workers = mc.clamp(1, 16) as u32;
                }
                if work.state == "waiting_user" {
                    work.state = "running".to_owned();
                }
                work.updated_at_unix = now_unix();
                if let Err(e) = self.ledger.save_work(&work) {
                    return fail("ledger_write_failed", e.to_string());
                }
                json!({
                    "state": "ok",
                    "work_id": work.work_id,
                    "work_state": work.state,
                    "budget": work.budget
                })
            }
            "rename_session" => {
                let Some(backend) = ext_args
                    .get("backend")
                    .and_then(Value::as_str)
                    .filter(|b| matches!(*b, "chatgpt" | "codex"))
                else {
                    return self
                        .extensions
                        .validation_error(action, "backend must be chatgpt|codex");
                };
                let Some(ref_id) = ext_args
                    .get("ref_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                else {
                    return self.extensions.validation_error(action, "ref_id required");
                };
                let Some(title) = ext_args
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty() && s.len() <= 180)
                else {
                    return self
                        .extensions
                        .validation_error(action, "title (1..=180) required");
                };
                let res = match backend {
                    "chatgpt" => self.owners.chatgpt_rename_chat(ref_id, title).map(|_| ()),
                    "codex" => self.owners.codex_rename_thread(
                        ext_args.get("profile").and_then(Value::as_str),
                        ref_id,
                        title,
                    ),
                    _ => unreachable!(),
                };
                match res {
                    Ok(()) => {
                        json!({ "state": "ok", "backend": backend, "ref_id": ref_id, "title": title })
                    }
                    Err(e) => fail("rename_failed", e),
                }
            }
            "lead_rollover" => {
                let Some(wid) = ext_args
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return self
                        .extensions
                        .validation_error(action, "valid work_id required");
                };
                let _lock = match self.ledger.lock_work(wid) {
                    Ok(l) => l,
                    Err(e) => return fail("ledger_lock_failed", e.to_string()),
                };
                let mut work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return fail("work_not_found", e.to_string()),
                };
                if let Err(e) = check_lead_fence(&work, ext_args, true) {
                    return e;
                }
                if work.is_terminal() {
                    return fail("work_already_terminal", &work.state);
                }
                let current_kind = work
                    .lead
                    .as_ref()
                    .map(|lead| lead.kind.as_str())
                    .unwrap_or("chatgpt");
                let next_kind = ext_args
                    .get("lead_kind")
                    .and_then(Value::as_str)
                    .unwrap_or(current_kind);
                if next_kind == "chatgpt" {
                    let rollover = crate::rollover::Rollover::new(&self.ledger, &self.owners);
                    let reason = ext_args
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("manual rollover");
                    return match rollover.start(
                        &mut work,
                        reason,
                        ext_args.get("checkpoint_md").and_then(Value::as_str),
                    ) {
                        Ok(done) => json!({
                            "state": if done { "ok" } else { "pending" },
                            "work_id": wid,
                            "lead": work.lead,
                            "rollover": work.lead_rollover,
                        }),
                        Err(e) => {
                            let code = match e.as_str() {
                                "pinned_account_required"
                                | "chatgpt_pinned_send_unavailable"
                                | "pinned_account_mismatch"
                                | "pinned_project_required"
                                | "pinned_project_mismatch"
                                | "lead_replay_in_progress"
                                | "chatgpt_lead_required" => e.as_str(),
                                _ => "lead_rollover_failed",
                            };
                            fail(code, &e)
                        }
                    };
                }
                if work.lead_rollover.is_some() || work.lead_replay.is_some() {
                    return fail("lead_rollover_in_progress", "ChatGPT handoff is pending");
                }
                if let Some(cp) = ext_args.get("checkpoint_md").and_then(Value::as_str) {
                    let _ = self.ledger.write_checkpoint(wid, cp);
                }
                let next_gen = work.lead.as_ref().map(|l| l.generation).unwrap_or(0) + 1;
                let mut warnings = Vec::new();
                let mut lead_args = ext_args.clone();
                lead_args.insert("lead_kind".to_owned(), json!("codex"));
                let mut new_lead = match self.spawn_or_bind_lead(
                    &work.work_id,
                    &work.title,
                    &work.goal,
                    &work.completion_criteria,
                    work.project_id.as_deref(),
                    next_gen,
                    &lead_args,
                    &mut warnings,
                ) {
                    Ok(l) => l,
                    Err(err_val) => return err_val,
                };
                if let Some(prev) = work.lead.take() {
                    let mut hist = prev.history;
                    hist.extend(new_lead.history);
                    new_lead.history = hist;
                }
                work.tier = "orchestrated".to_owned();
                work.lead = Some(new_lead.clone());
                let _ = self.ledger.append_event_if_absent(
                    &mut work,
                    &format!("rollover:L{next_gen}:{}", new_lead.ref_id),
                    "lead.rolled_over",
                    &format!("L{next_gen}"),
                    &format!(
                        "Lead rolled over to L{next_gen} ({}:{})",
                        new_lead.kind, new_lead.ref_id
                    ),
                    json!({ "generation": next_gen, "ref_id": new_lead.ref_id }),
                );
                if let Err(e) = self.ledger.save_work(&work) {
                    return fail("ledger_write_failed", e.to_string());
                }
                json!({
                    "state": "ok",
                    "work_id": work.work_id,
                    "lead": work.lead,
                    "warnings": warnings
                })
            }
            _ => self.extensions.validation_error(action, "unknown_action"),
        }
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
        let val = match name {
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
        };
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
