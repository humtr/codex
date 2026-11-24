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
    format_front_title, format_lead_title, format_participant_title, generate_id, is_safe_work_id,
    now_unix, read_private_file, truncate_chars, write_atomic_private, FrontBinding,
    FrontHistoryEntry, Isolation, LeadBinding, Ledger, Participant, PendingDelivery,
    ProjectBinding, SessionPrefixes, ShellContinuation, StepSpec, Verification, VerificationPolicy,
    WaitSet, WorkBudget, WorkRecord, WORK_SCHEMA,
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
            "Read a durable task or its evidence. When the user asks about a specific task from a new ChatGPT conversation, pass the current chat ID from trusted conversation context or a unique user-authored marker in that conversation to move future notices as part of the read. Leave these fields out of list, internal, and automated reads.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "read", "checkpoint", "inbox"] },
                    "work_id": { "type": "string", "minLength": 1, "maxLength": 96, "description": "Required for read, checkpoint, and inbox" },
                    "rebind_front_chat_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "rebind_front_marker": { "type": "string", "minLength": 1, "maxLength": 160 },
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160, "description": "Required if moving the Front." },
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
            "Create or manage a durable Work. Use the current session for questions and steps it can finish itself. For edits, coordination, or persistent follow-up from ChatGPT, delegate the work and return promptly so the Front chat is free. For a fixed shell plan pass steps rather than opening a Lead. Select independent_review for multi-file, contract, state, permission or follow-up fixes. Multi-step instructions still need coordination in the current session. Use direct for a fixed sequence and do not create a separate Lead unless needed. Reuse repository projects with project_scope.",
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
                    "verification_policy": { "type": "string", "enum": ["none", "automated", "independent_review"] },
                    "steps": {"type":"array","maxItems":16,"items":{"type":"object","properties":{
                        "step_id":{"type":"string"},"depends_on":{"type":"array","items":{"type":"string"}},
                        "command":{"type":"string"},"cwd":{"type":"string"},
                        "write_paths":{"type":"array","items":{"type":"string"}},
                        "isolation":{"type":"string","enum":["shared_serial","worktree","readonly"]},
                        "on_failure":{"type":"string","enum":["wake_lead","fail_work"]},
                        "auto_verify":{"type":"boolean","description":"This command verifies the final work and must depend on all edits."}
                    },"required":["step_id","command","cwd"],"additionalProperties":false}},
                    "completion_criteria": { "type": "array", "items": { "type": "string", "maxLength": 300 }, "maxItems": 20 },
                    "project_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "project_scope": { "type": "string", "minLength": 1, "maxLength": 120, "description":"When the repository is known, reuse its stable owner/repo key. Add @branch only for a separate branch project. Keep the same key for related Works." },
                    "project_name": { "type": "string", "minLength": 1, "maxLength": 120, "description":"Optional display name on first bind to a project_scope" },
                    "session_prefixes": { "type":"object", "properties": {
                        "front":{"type":"string","maxLength":80}, "lead":{"type":"string","maxLength":80},
                        "worker":{"type":"string","maxLength":80}, "verifier":{"type":"string","maxLength":80}
                    }, "additionalProperties":false, "description":"Override role prefixes with {n} or {slug}. Empty prefix disables it. A supplied object replaces the Work policy." },
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
                    "write_paths":{"type":"array","items":{"type":"string"},"maxItems":64},
                    "isolation":{"type":"string","enum":["shared_serial","worktree","readonly"]},
                    "read_only": { "type":"boolean", "description":"Declare that this worker makes no workspace changes; does not change the execution owner's permissions" },
                    "model": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "model_provider": { "type":"string", "minLength":1, "maxLength":128, "description":"Configured Codex model provider for a new Codex participant" },
                    "reasoning_effort": { "type": "string", "minLength": 1, "maxLength": 32 },
                    "approval_policy": { "type": "string", "enum": ["untrusted", "on-failure", "on-request", "never"] },
                    "wake_target": { "type": "string", "enum": ["lead", "front"] },
                    "all_of": { "type": "array", "items": { "type": "string", "maxLength": 96 }, "maxItems": 32 },
                    "then_verify_shell": { "type":"object", "properties":{
                        "command":{"type":"string","minLength":1,"maxLength":65536},
                        "cwd":{"type":"string","minLength":1,"maxLength":512}
                    }, "required":["command"], "additionalProperties":false,
                    "description":"Run a fixed shell check after every all_of prerequisite succeeds, then report to the Front. No Lead turn is started." },
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
                    "review_cwd":{"type":"string","description":"Workspace for independent review when reporting completion."},
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
        let mut cursor: Option<String> = None;
        let mut matched = None;
        let mut visited = std::collections::HashSet::new();
        for _ in 0..4 {
            let page = self.owners.chatgpt_chat_page(None, cursor.as_deref())?;
            let chats = page["chats"].as_array().ok_or("chat list missing chats")?;
            for row in chats {
                let Some(cid) = row["id"].as_str() else {
                    continue;
                };
                if let Ok(chat) = self.owners.chatgpt_read_chat(cid, 6) {
                    if chat["messages"].as_array().is_some_and(|messages| {
                        messages.iter().any(|m| {
                            m["role"] == "user"
                                && m["text"].as_str().is_some_and(|text| text.contains(marker))
                        })
                    }) {
                        if matched.as_deref().is_some_and(|prior| prior != cid) {
                            return Err(
                                "front_marker_ambiguous: multiple chats contain the marker"
                                    .to_owned(),
                            );
                        }
                        matched = Some(cid.to_owned());
                    }
                }
            }
            let Some(next) = page["next_cursor"]
                .as_str()
                .filter(|value| !value.is_empty())
            else {
                break;
            };
            if !visited.insert(next.to_owned()) {
                break;
            }
            cursor = Some(next.to_owned());
        }
        matched.ok_or_else(|| {
            "front_marker_not_found: no recent chat contained the user message marker".to_owned()
        })
    }

    fn current_account_id(&self) -> Option<String> {
        let status = self.owners.chatgpt_status().ok()?;
        crate::owners::chatgpt_account_identity(&status).map(ToOwned::to_owned)
    }

    fn bind_project(
        &self,
        scope: &str,
        label: Option<&str>,
        explicit: Option<&str>,
    ) -> Result<(String, String, String), String> {
        let account = self.owners.chatgpt_account_id()?;
        let key = sha256_hex(json!([account, scope]).to_string().as_bytes());
        let (_lock, path) = self
            .ledger
            .lock_project_scope(&sha256_hex(account.as_bytes()), &key)
            .map_err(|e| e.to_string())?;
        let previous: Option<ProjectBinding> = match read_private_file(&path, 8192) {
            Ok(bytes) => Some(serde_json::from_slice(&bytes).map_err(|e| e.to_string())?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        let previous_id = previous.as_ref().and_then(|binding| binding.id.as_deref());
        if previous_id
            .zip(explicit)
            .is_some_and(|(bound, given)| bound != given)
        {
            return Err("project_scope is already bound to another project_id".to_owned());
        }
        let name = previous
            .as_ref()
            .map(|b| b.name.as_str())
            .or(label)
            .unwrap_or(scope);
        let id = previous_id.or(explicit);
        let persist = |binding: &ProjectBinding| {
            let data = serde_json::to_vec(binding).map_err(|e| e.to_string())?;
            write_atomic_private(&path, &data).map_err(|e| e.to_string())
        };
        let require_account = || {
            if self.owners.chatgpt_account_id()? == account {
                Ok(())
            } else {
                Err("ChatGPT account changed during project binding".to_owned())
            }
        };
        require_account()?;
        let (id, name) = match self.owners.chatgpt_find_project(name, id)? {
            Some(project) => project,
            None if id.is_some() => {
                return Err("project_id is unavailable in the pinned account".to_owned())
            }
            None if previous.is_some() => return Err(
                "project creation is unresolved; supply its project_id after checking the account"
                    .to_owned(),
            ),
            None => {
                require_account()?;
                persist(&ProjectBinding {
                    name: name.to_owned(),
                    id: None,
                })?;
                let created = self.owners.chatgpt_create_project(name, None)?;
                let id = created["project_id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or("project creation response lacks an ID")?;
                (
                    id.to_owned(),
                    created["name"].as_str().unwrap_or(name).to_owned(),
                )
            }
        };
        require_account()?;
        persist(&ProjectBinding {
            name: name.clone(),
            id: Some(id.clone()),
        })?;
        Ok((account, id, name))
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
                "steps",
                "verification_policy",
                "project_id",
                "project_scope",
                "project_name",
                "session_prefixes",
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
        let tier = obj.get("tier").and_then(Value::as_str).unwrap_or_else(|| {
            if [
                "lead_kind",
                "lead_ref_id",
                "lead_profile",
                "lead_model",
                "lead_cwd",
                "lead_reasoning_effort",
            ]
            .iter()
            .any(|k| obj.contains_key(*k))
            {
                "orchestrated"
            } else {
                "direct"
            }
        });
        if tier != "direct" && tier != "orchestrated" {
            return (
                fail("invalid_arguments", "'tier' must be direct|orchestrated"),
                false,
            );
        }
        let mut steps = match obj
            .get("steps")
            .cloned()
            .map(serde_json::from_value::<Vec<StepSpec>>)
            .transpose()
        {
            Ok(plan) => plan.unwrap_or_default(),
            Err(error) => return (fail("invalid_steps", error.to_string()), false),
        };
        if let Err(error) = StepSpec::validate_plan(&mut steps) {
            return (fail("invalid_steps", error), false);
        }
        let verification_policy = match obj
            .get("verification_policy")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
        {
            Ok(Some(policy)) => policy,
            Ok(None)
                if steps.iter().any(|step| {
                    step.write_paths.len() > 1
                        || step
                            .write_paths
                            .iter()
                            .any(|path| path == "SPEC.md" || path.starts_with("docs/spec/"))
                }) || steps
                    .iter()
                    .filter(|step| step.isolation == crate::ledger::Isolation::Worktree)
                    .count()
                    > 1 =>
            {
                VerificationPolicy::IndependentReview
            }
            Ok(None) if steps.iter().any(|step| step.auto_verify) => VerificationPolicy::Automated,
            Ok(None) => VerificationPolicy::None,
            Err(error) => return (fail("invalid_arguments", error.to_string()), false),
        };
        if !steps.is_empty()
            && verification_policy == VerificationPolicy::Automated
            && !steps.iter().any(|step| step.auto_verify)
        {
            return (
                fail(
                    "invalid_steps",
                    "automated verification needs a verification step",
                ),
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

        let session_prefixes = match parse_session_prefixes(obj) {
            Ok(prefixes) => prefixes.unwrap_or_default(),
            Err(e) => return (fail("invalid_arguments", e), false),
        };
        let project_scope = match project_label(obj, "project_scope") {
            Ok(scope) => scope,
            Err(e) => return (fail("invalid_arguments", e), false),
        };
        let project_label = match project_label(obj, "project_name") {
            Ok(label) => label,
            Err(e) => return (fail("invalid_arguments", e), false),
        };
        let explicit_project = obj
            .get("project_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let (account_id, project_id, project_name) = match project_scope {
            Some(scope) => match self.bind_project(scope, project_label, explicit_project) {
                Ok((account, id, name)) => (Some(account), Some(id), Some(name)),
                Err(e) => return (fail("project_binding_failed", e), false),
            },
            None if project_label.is_some() => {
                return (
                    fail("invalid_arguments", "project_name requires project_scope"),
                    false,
                )
            }
            None => (
                self.current_account_id(),
                explicit_project.map(ToOwned::to_owned),
                None,
            ),
        };
        let mut warnings = Vec::new();

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
            let ftitle = format_front_title(&session_prefixes, 1, title);
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
            project_scope: project_scope.map(ToOwned::to_owned),
            session_prefixes,
            delivery_mode: delivery_mode.to_owned(),
            front: FrontBinding {
                latest_chat_id: front_chat_id.clone(),
                generation: if front_chat_id.is_some() { 1 } else { 0 },
                history: front_history,
            },
            lead: None,
            pending_lead: None,
            lead_rollover: None,
            lead_replay: None,
            participants: Vec::new(),
            steps,
            verification_policy,
            review_cwd: None,
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

        let mut status = "ok";
        if tier == "orchestrated" {
            match self.spawn_or_bind_lead(&mut work, 1, obj, &mut warnings) {
                Ok(true) => (),
                Ok(false) => status = "pending",
                Err(mut failure) => {
                    failure["work_id"] = json!(work_id);
                    return (failure, true);
                }
            }
        }
        (
            json!({
                "state": status,
                "work_id": work_id,
                "title": title,
                "tier": tier,
                "project_id": work.project_id,
                "project_name": work.project_name,
                "project_scope": work.project_scope,
                "session_prefixes": work.session_prefixes,
                "front_chat_id": work.front.latest_chat_id,
                "lead": work.lead,
                "delivery_mode": work.delivery_mode,
                "warnings": warnings
            }),
            true,
        )
    }

    fn spawn_or_bind_lead(
        &self,
        work: &mut WorkRecord,
        generation: u32,
        obj: &Map<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<bool, Value> {
        if work.pending_lead.is_some() {
            return Ok(false);
        }
        let kind = obj
            .get("lead_kind")
            .and_then(Value::as_str)
            .unwrap_or("chatgpt");
        if !matches!(kind, "chatgpt" | "codex") {
            return Err(fail(
                "invalid_arguments",
                "'lead_kind' must be chatgpt|codex",
            ));
        }
        let mut lead = LeadBinding {
            kind: kind.to_owned(),
            ref_id: String::new(),
            profile: obj
                .get("lead_profile")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            model: obj
                .get("lead_model")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            reasoning_effort: obj
                .get("lead_reasoning_effort")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            generation,
            history: Vec::new(),
            rollover_internal_turns: 0,
            dispatch_id: None,
            turn_id: None,
        };
        if let Some(existing) = obj
            .get("lead_ref_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            lead.ref_id = existing.to_owned();
        } else {
            if kind == "chatgpt" {
                let account = self
                    .owners
                    .chatgpt_account_id()
                    .map_err(|e| fail("lead_account_unavailable", e))?;
                if work
                    .account_id
                    .as_ref()
                    .is_some_and(|pinned| pinned != &account)
                {
                    return Err(fail(
                        "account_mismatch",
                        "Work belongs to a different account",
                    ));
                }
                work.account_id = Some(account);
            }
            let id = generate_id("dispatch");
            lead.dispatch_id = Some(id.clone());
            work.pending_lead = Some(lead.clone());
            self.ledger
                .save_work(work)
                .map_err(|e| fail("ledger_write_failed", e.to_string()))?;
            let criteria = work.completion_criteria.join("\n- ");
            let prompt = format!(
                "[ORCH_DISPATCH {id}]\nYou are Lead [L{generation}] for durable Work `{}` (\"{}\").\n\
                 Goal: {}\nCompletion criteria:\n- {criteria}\n\n\
                 Use the available owner tools to dispatch scoped Workers and Verifiers. Register them and arm \
                 the Wait-Set with @TermuxOrch, then end your turn. Report milestones, blockers, and verified \
                 completion via orch_report. Include expected_lead_generation={generation} in Work mutations.",
                work.work_id, work.title, work.goal
            );
            let result = if kind == "codex" {
                self.owners
                    .codex_dispatch_turn(
                        lead.profile.as_deref(),
                        None,
                        obj.get("lead_cwd").and_then(Value::as_str),
                        &prompt,
                        lead.model.as_deref(),
                        None,
                        None,
                        |tid| {
                            if let Some(pending) = work.pending_lead.as_mut() {
                                pending.ref_id = tid.to_owned();
                            }
                            self.ledger.save_work(work).map_err(|e| e.to_string())
                        },
                    )
                    .and_then(|(tid, turn)| {
                        turn.map(|turn| (tid, Some(turn)))
                            .ok_or("missing turn ID".to_owned())
                    })
            } else {
                self.owners
                    .chatgpt_dispatch_participant(
                        work,
                        &id,
                        &prompt,
                        lead.model.as_deref(),
                        lead.reasoning_effort.as_deref(),
                    )
                    .and_then(|receipt| {
                        if crate::rollover::rejected_before_send(&receipt) {
                            work.pending_lead = None;
                            self.ledger.save_work(work).map_err(|e| e.to_string())?;
                        }
                        receipt["chat_id"]
                            .as_str()
                            .filter(|cid| {
                                !cid.is_empty()
                                    && matches!(
                                        receipt["state"].as_str(),
                                        Some("settled" | "in_progress")
                                    )
                            })
                            .map(|cid| (cid.to_owned(), None))
                            .ok_or("missing confirmed Lead chat ID".to_owned())
                    })
            };
            match result {
                Ok((ref_id, turn_id)) => {
                    lead.ref_id = ref_id;
                    lead.turn_id = turn_id;
                }
                Err(error) if work.pending_lead.is_some() => {
                    warnings.push(format!("lead_dispatch_pending: {error}"));
                    return Ok(false);
                }
                Err(error) => return Err(fail("lead_dispatch_rejected", error)),
            }
        }
        let title = format_lead_title(&work.session_prefixes, generation, &work.title);
        work.bind_lead(lead.clone());
        self.ledger
            .save_work(work)
            .map_err(|e| fail("ledger_write_failed", e.to_string()))?;
        let rename = if kind == "chatgpt" {
            self.owners
                .chatgpt_rename_chat(&lead.ref_id, &title)
                .map(|_| ())
        } else {
            self.owners
                .codex_rename_thread(lead.profile.as_deref(), &lead.ref_id, &title)
        };
        if let Err(error) = rename {
            warnings.push(format!("lead_rename_skipped: {error}"));
        }
        Ok(true)
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
        if !work.can_start()
            || work
                .wait_set
                .as_ref()
                .is_some_and(|w| w.armed && w.then_verify_shell.is_some())
        {
            return (fail("work_unavailable", "Work cannot start a Lead"), false);
        }
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
        match self.spawn_or_bind_lead(&mut work, 1, obj, &mut warnings) {
            Ok(true) => (),
            Ok(false) => {
                return (
                    json!({"state":"pending","work_id":wid,"warnings":warnings}),
                    true,
                )
            }
            Err(error) => return (error, false),
        }
        let lead = work.lead.clone().unwrap();
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

        let Some(pinned) = work.account_id.as_deref() else {
            return (
                fail("pinned_account_required", "Work has no ChatGPT account pin"),
                false,
            );
        };
        match self.owners.chatgpt_account_id() {
            Ok(current) if current == pinned => (),
            Ok(_) => {
                return (
                    fail("account_mismatch", "Work belongs to a different account"),
                    false,
                )
            }
            Err(_) => {
                return (
                    fail(
                        "account_unavailable",
                        "The active ChatGPT account could not be confirmed",
                    ),
                    false,
                )
            }
        }
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

        match self.owners.chatgpt_read_chat(&resolved_chat_id, 1) {
            Ok(chat) if chat["state"] == "ok" && chat["chat_id"] == resolved_chat_id => (),
            _ => {
                return (
                    fail(
                        "front_chat_unverified",
                        "The active account could not read the target chat",
                    ),
                    false,
                )
            }
        }
        if !self
            .owners
            .chatgpt_account_id()
            .is_ok_and(|current| current == pinned)
        {
            return (
                fail(
                    "account_unavailable",
                    "ChatGPT account changed during verification",
                ),
                false,
            );
        }

        let now = now_unix();
        if work.front.latest_chat_id.as_deref() == Some(&resolved_chat_id) {
            return (
                json!({"state":"ok", "work_id":wid, "front_generation":work.front.generation,
                "front_chat_id":resolved_chat_id, "work_state":work.state}),
                true,
            );
        }
        let next_gen = work.front.generation + 1;
        work.front.generation = next_gen;
        work.front.latest_chat_id = Some(resolved_chat_id.clone());
        work.front.history.push(FrontHistoryEntry {
            generation: next_gen,
            chat_id: resolved_chat_id.clone(),
            bound_at_unix: now,
        });

        let mut warnings = Vec::new();
        let ftitle = format_front_title(&work.session_prefixes, next_gen, &work.title);
        if let Err(e) = self.owners.chatgpt_rename_chat(&resolved_chat_id, &ftitle) {
            warnings.push(format!("front_rename_skipped: {e}"));
        }

        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("rebind:{next_gen}:{resolved_chat_id}"),
            "front.rebound",
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
        match parse_session_prefixes(obj) {
            Ok(Some(prefixes)) => work.session_prefixes = prefixes,
            Ok(None) => {}
            Err(e) => return (fail("invalid_arguments", e), false),
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
            "completion_criteria": work.completion_criteria,
            "session_prefixes": work.session_prefixes
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
        if work.is_terminal() {
            return (fail("work_already_terminal", &work.state), false);
        }
        work.state = "cancelling".to_owned();
        if let Some(ws) = work.wait_set.as_mut() {
            ws.armed = false;
        }
        work.pending_deliveries
            .retain(|delivery| delivery.dispatch.is_some());
        let reason = obj
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("cancelled by user");
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            "work:cancel_requested",
            "work.cancel_requested",
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
                "work_state": work.state
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
                "read_only",
                "write_paths",
                "isolation",
                "model",
                "model_provider",
                "reasoning_effort",
                "approval_policy",
                "wake_target",
                "all_of",
                "then_verify_shell",
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
        if !work.can_start() {
            return (
                fail(
                    "work_already_terminal",
                    format!("work '{wid}' is already {}", work.state),
                ),
                false,
            );
        }
        if work.participants.iter().any(|p| p.call_id == call_id) {
            return (
                fail(
                    "participant_call_already_registered",
                    "Use the original participant",
                ),
                false,
            );
        }
        if work
            .wait_set
            .as_ref()
            .is_some_and(|ws| ws.armed && ws.then_verify_shell.is_some())
        {
            return (
                fail(
                    "continuation_armed",
                    "The fixed continuation owns this Work",
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
        let session_title =
            format_participant_title(&work.session_prefixes, role, index, slug, task_summary);
        let verification = (role == "verifier").then(|| Verification {
            subject: work.verification_subject(),
            challenge: generate_id("verify"),
            verdict: None,
            evidence: Vec::new(),
            blocking_issues: vec![],
        });

        let existing = obj
            .get("ref_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let shell_command = obj.get("command").and_then(Value::as_str);
        let prompt = obj.get("prompt").and_then(Value::as_str);
        let isolation = match obj
            .get("isolation")
            .cloned()
            .map(serde_json::from_value::<Isolation>)
            .transpose()
        {
            Ok(Some(value)) => value,
            Ok(None) if obj.get("read_only").and_then(Value::as_bool) == Some(true) => {
                Isolation::Readonly
            }
            Ok(None) => Isolation::SharedSerial,
            Err(error) => return (fail("invalid_arguments", error.to_string()), false),
        };
        let write_paths = match obj
            .get("write_paths")
            .cloned()
            .map(serde_json::from_value::<Vec<String>>)
            .transpose()
        {
            Ok(paths) => paths.unwrap_or_default(),
            Err(error) => return (fail("invalid_arguments", error.to_string()), false),
        };
        if let Err(error) = crate::workspace::validate_paths(&write_paths) {
            return (fail("invalid_arguments", error), false);
        }
        if isolation == Isolation::Readonly && !write_paths.is_empty() {
            return (
                fail(
                    "invalid_arguments",
                    "readonly workers cannot declare write_paths",
                ),
                false,
            );
        }
        if role == "verifier"
            && existing.is_some_and(|id| {
                work.lead
                    .as_ref()
                    .is_some_and(|lead| lead.kind == backend && lead.ref_id == id)
                    || work
                        .participants
                        .iter()
                        .any(|p| p.role == "worker" && p.backend == backend && p.ref_id == id)
            })
        {
            return (
                fail(
                    "verifier_not_independent",
                    "The verifier cannot reuse an implementation session",
                ),
                false,
            );
        }
        let cwd = obj.get("cwd").and_then(Value::as_str);
        let scope = if role == "worker" && existing.is_none() && isolation != Isolation::Readonly {
            match cwd
                .map(|path| crate::workspace::root(std::path::Path::new(path)))
                .transpose()
            {
                Ok(scope) => scope,
                Err(error) => return (fail("invalid_workspace", error), false),
            }
        } else {
            None
        };
        let _workspace_lock = if scope.is_some() {
            match self.ledger.lock_writers() {
                Ok(lease) => Some(lease),
                Err(error) => return (fail("workspace_lock_failed", error.to_string()), false),
            }
        } else {
            None
        };
        let (write_workspace, worktree) = if isolation == Isolation::Worktree {
            let Some(repo) = scope
                .as_deref()
                .filter(|path| same_workspace_root(cwd, path))
            else {
                return (
                    fail(
                        "invalid_workspace",
                        "worktree requires a new worker at the repository root",
                    ),
                    false,
                );
            };
            if backend == "chatgpt" || write_paths.is_empty() {
                return (
                    fail(
                        "invalid_workspace",
                        "worktree requires shell or Codex and declared write_paths",
                    ),
                    false,
                );
            }
            if work.participants.iter().any(|p| {
                !p.is_terminal()
                    && p.worktree.as_ref().is_some_and(|tree| tree.repo == repo)
                    && p.write_paths.iter().any(|path| {
                        write_paths
                            .iter()
                            .any(|other| crate::workspace::overlaps(path, other))
                    })
            }) {
                return (
                    fail(
                        "worker_workspace_busy",
                        "An active isolated worker already owns those paths",
                    ),
                    false,
                );
            }
            match crate::workspace::WorkerTree::plan(&self.ledger, wid, &participant_id, repo) {
                Ok(tree) => (None, Some(tree)),
                Err(error) => return (fail("invalid_workspace", error), false),
            }
        } else {
            if let Some(path) = &scope {
                match self.ledger.writer_busy(path) {
                    Ok(false) => (),
                    Ok(true) => {
                        return (
                            fail(
                                "worker_workspace_busy",
                                "An active worker owns this workspace",
                            ),
                            false,
                        )
                    }
                    Err(error) => return (fail("workspace_read_failed", error.to_string()), false),
                }
            }
            (scope, None)
        };
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
        } else {
            existing.unwrap_or_default().to_owned()
        };
        let launch = backend != "shell" && existing.is_none();
        if launch && prompt.is_none_or(|p| p.trim().is_empty()) {
            return (
                fail("invalid_arguments", "provide a prompt or existing ref_id"),
                false,
            );
        }
        if launch && backend == "chatgpt" && work.account_id.is_none() {
            return (
                fail("pinned_account_required", "Work has no ChatGPT account pin"),
                false,
            );
        }
        let now = now_unix();
        let mut participant = Participant {
            participant_id: participant_id.clone(),
            index,
            role: role.to_owned(),
            backend: backend.to_owned(),
            ref_id,
            turn_id: None,
            profile,
            slug: slug.to_owned(),
            task_summary: task_summary.to_owned(),
            owner_scope,
            write_workspace,
            step_id: None,
            worktree,
            changed_paths: Vec::new(),
            write_paths,
            call_id,
            dispatch_id: launch.then(|| generate_id("dispatch")),
            state: if launch { "dispatching" } else { "running" }.to_owned(),
            result_summary: None,
            verification,
            created_at_unix: now,
            updated_at_unix: now,
        };
        if let Some(tree) = &participant.worktree {
            work.verification_policy = VerificationPolicy::IndependentReview;
            work.participants.push(participant.clone());
            if let Err(error) = self.ledger.save_work(&work) {
                return (fail("ledger_write_failed", error.to_string()), false);
            }
            if let Err(error) = tree.prepare() {
                let saved = work.participants.last_mut().unwrap();
                saved.state = "failed".to_owned();
                saved.result_summary = Some(error.clone());
                let _ = self.ledger.save_work(&work);
                return (fail("worktree_prepare_failed", error), true);
            }
            work.participants.pop();
        }
        let cwd = participant
            .worktree
            .as_ref()
            .and_then(|tree| tree.path.to_str())
            .or(cwd);
        if let Some(id) = &participant.dispatch_id {
            let position = work.participants.len();
            work.participants.push(participant.clone());
            if let Err(error) = self.ledger.save_work(&work) {
                return (fail("ledger_write_failed", error.to_string()), false);
            }
            drop(_workspace_lock);
            let mut text = format!(
                "[ORCH_DISPATCH {id}]\n{}",
                prompt.unwrap_or_default().trim()
            );
            if let Some(verification) = &participant.verification {
                let result = json!({
                    "challenge": verification.challenge,
                    "verdict": "pass|fail|inconclusive",
                    "evidence_refs": ["exact artifact or test result inspected"],
                    "blocking_issues": []
                });
                text.push_str(&format!(
                    "\n\nReview the requested subject without making changes. Return only a JSON \
                     object of this shape: {result}. Use one verdict, repeat the challenge verbatim, \
                     and cite concrete evidence in a non-empty array."
                ));
            }
            let model = obj.get("model").and_then(Value::as_str);
            let result = if backend == "codex" {
                self.owners
                    .codex_dispatch_turn(
                        participant.profile.as_deref(),
                        None,
                        cwd,
                        &text,
                        model,
                        obj.get("model_provider").and_then(Value::as_str),
                        obj.get("approval_policy").and_then(Value::as_str),
                        |tid| {
                            work.participants[position].ref_id = tid.to_owned();
                            self.ledger.save_work(&work).map_err(|e| e.to_string())
                        },
                    )
                    .and_then(|(tid, turn_id)| {
                        turn_id
                            .map(|turn| (tid, Some(turn)))
                            .ok_or("missing turn ID".to_owned())
                    })
            } else {
                self.owners
                    .chatgpt_dispatch_participant(
                        &work,
                        id,
                        &text,
                        model,
                        obj.get("reasoning_effort").and_then(Value::as_str),
                    )
                    .and_then(|receipt| {
                        if crate::rollover::rejected_before_send(&receipt) {
                            work.participants[position].state = "failed".to_owned();
                            work.participants[position].result_summary =
                                receipt["error"].as_str().map(ToOwned::to_owned);
                            self.ledger.save_work(&work).map_err(|e| e.to_string())?;
                            return Err("owner rejected the request before dispatch".to_owned());
                        }
                        receipt["chat_id"]
                            .as_str()
                            .filter(|id| {
                                !id.is_empty()
                                    && matches!(
                                        receipt["state"].as_str(),
                                        Some("settled" | "in_progress")
                                    )
                            })
                            .map(|id| (id.to_owned(), None))
                            .ok_or("missing confirmed ChatGPT chat ID".to_owned())
                    })
            };
            match result {
                Ok((ref_id, turn_id)) => {
                    participant.ref_id = ref_id;
                    participant.turn_id = turn_id;
                    participant.state = "running".to_owned();
                    work.participants[position] = participant.clone();
                    if let Err(error) = self.ledger.save_work(&work) {
                        return (
                            fail("dispatch_confirmation_pending", error.to_string()),
                            true,
                        );
                    }
                }
                Err(error) => {
                    let original = &work.participants[position];
                    return (
                        json!({"state":if original.state == "failed" {"error"} else {"pending"},
                            "work_id":wid, "participant":original, "detail":error}),
                        true,
                    );
                }
            }
        } else {
            work.participants.push(participant.clone());
        }
        let ref_id = &participant.ref_id;
        let mut warnings = Vec::new();
        match backend {
            "chatgpt" => {
                if let Err(e) = self.owners.chatgpt_rename_chat(ref_id, &session_title) {
                    warnings.push(format!("rename_skipped: {e}"));
                }
            }
            "codex" => {
                if let Err(e) = self.owners.codex_rename_thread(
                    participant.profile.as_deref(),
                    ref_id,
                    &session_title,
                ) {
                    warnings.push(format!("rename_skipped: {e}"));
                }
            }
            _ => {}
        }

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
            let result = crate::shell_job::ShellJob::new(&self.ledger, wid, ref_id)
                .and_then(|job| job.spawn(self.ledger.root(), wid, command, cwd));
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
        if !work.can_start() {
            return (
                fail(
                    "work_already_terminal",
                    "cannot arm Wait-Set on terminal work",
                ),
                false,
            );
        }
        if work
            .wait_set
            .as_ref()
            .is_some_and(|ws| ws.armed && ws.then_verify_shell.is_some())
        {
            return (
                fail(
                    "continuation_armed",
                    "Clear the pending continuation or cancel the Work",
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
        let then_verify_shell = match obj.get("then_verify_shell") {
            Some(value) => match serde_json::from_value::<ShellContinuation>(value.clone()) {
                Ok(next)
                    if !next.command.trim().is_empty()
                        && next.command.len() <= 65536
                        && !next.command.contains('\0')
                        && next.cwd.as_ref().is_none_or(|cwd| {
                            !cwd.is_empty() && cwd.len() <= 512 && !cwd.contains('\0')
                        }) =>
                {
                    Some(next)
                }
                _ => {
                    return (
                        fail(
                            "invalid_arguments",
                            "Invalid then_verify_shell command or cwd",
                        ),
                        false,
                    )
                }
            },
            None => None,
        };
        let on_user_event = obj
            .get("on_user_event")
            .and_then(Value::as_bool)
            .unwrap_or(then_verify_shell.is_none());
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
        if then_verify_shell.is_some()
            && (work.tier != "direct"
                || wake_target != "front"
                || all_of.is_empty()
                || !any_of.is_empty()
                || on_user_event
                || all_of
                    .iter()
                    .any(|id| !work.participants.iter().any(|p| &p.participant_id == id)))
        {
            return (
                fail(
                    "invalid_wait_set",
                    "Fixed shell verification needs a direct Work and valid all_of prerequisites",
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
            then_verify_shell,
            continuation_id: None,
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
            if ws.armed && ws.continuation_id.is_some() {
                return (
                    fail(
                        "continuation_started",
                        "Cancel the Work to stop a started continuation",
                    ),
                    false,
                );
            }
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
                "review_cwd",
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
                if !work.can_start() {
                    return (fail("work_already_terminal", &work.state), false);
                }
                if matches!(action, "complete" | "fail")
                    && work
                        .wait_set
                        .as_ref()
                        .is_some_and(|w| w.armed && w.then_verify_shell.is_some())
                {
                    return (
                        fail(
                            "continuation_armed",
                            "The fixed continuation owns the final report",
                        ),
                        false,
                    );
                }
                if action == "fail" && work.participants.iter().any(|p| !p.is_terminal()) {
                    return (
                        fail(
                            "participants_active",
                            "Stop or settle participants before recording failure",
                        ),
                        false,
                    );
                }
                if action == "complete" {
                    if work.steps.iter().any(|step| {
                        !work.participants.iter().any(|p| {
                            p.step_id.as_deref() == Some(&step.step_id) && p.state == "completed"
                        })
                    }) {
                        return (
                            fail(
                                "steps_pending",
                                "The durable plan has unfinished or failed steps",
                            ),
                            false,
                        );
                    }
                    if work.pending_lead.is_some() {
                        return (
                            fail(
                                "lead_dispatch_pending",
                                "Lead launch has not been reconciled",
                            ),
                            false,
                        );
                    }
                    if work.participants.iter().any(|p| !p.is_terminal()) {
                        return (
                            fail("participants_active", "All participants must settle"),
                            false,
                        );
                    }
                    if !work.verification_passed() {
                        if work.verification_policy == VerificationPolicy::IndependentReview
                            && !work
                                .participants
                                .iter()
                                .any(|p| p.role == "verifier" && p.backend != "shell")
                            && work.steps.is_empty()
                        {
                            let requested = obj
                                .get("review_cwd")
                                .and_then(Value::as_str)
                                .map(std::path::Path::new)
                                .or_else(|| {
                                    work.participants
                                        .iter()
                                        .find_map(|p| p.write_workspace.as_deref())
                                });
                            let Some(cwd) = requested else {
                                return (
                                    fail(
                                        "verification_required",
                                        "Supply review_cwd for the independent review",
                                    ),
                                    false,
                                );
                            };
                            let cwd = match crate::workspace::root(cwd) {
                                Ok(cwd) => cwd,
                                Err(error) => return (fail("invalid_workspace", error), false),
                            };
                            work.review_cwd = Some(cwd);
                            work.state = "waiting".to_owned();
                            if let Err(error) = self.ledger.save_work(&work) {
                                return (fail("ledger_write_failed", error.to_string()), false);
                            }
                        }
                        return (
                            fail(
                                "verification_required",
                                "A current passing verdict is required",
                            ),
                            false,
                        );
                    }
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
                if !work.can_start() {
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
                match self.spawn_or_bind_lead(&mut work, next_gen, &lead_args, &mut warnings) {
                    Ok(true) => (),
                    Ok(false) => {
                        return json!({"state":"pending","work_id":wid,"warnings":warnings})
                    }
                    Err(error) => return error,
                }
                let new_lead = work.lead.clone().unwrap();
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
