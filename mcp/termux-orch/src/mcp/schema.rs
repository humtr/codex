use mcp_kit::{ExtensionRegistry, ExtensionSpec, ToolDef};
use serde_json::json;

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
            "Create or manage a durable Work. Keep single-turn questions, quick checks, and direct @TermuxShell edits in the current session (Tier 0). Delegate via @TermuxOrch when work needs durability across disconnects/sessions (F1->Fn), long background execution, parallel/worktree workers, scheduled shell steps, or independent_review, then return promptly so the Front chat stays free. For a fixed shell plan pass steps (tier=direct) rather than opening a Lead. Select independent_review for multi-file, contract, state, permission or follow-up fixes. Reuse repository projects with project_scope.",
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
                    "lead_model_provider": { "type": "string", "minLength": 1, "maxLength": 128, "description": "Optional configured Codex model provider for a new Codex Lead" },
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
            "operation_status",
            "Read an asynchronous owner operation receipt by operation_id.",
            json!({"type":"object","properties":{"operation_id":{"type":"string","minLength":64,"maxLength":64}},
                "required":["operation_id"],"additionalProperties":false}),
            true,
        ),
        ExtensionSpec::new(
            "lead_rollover",
            "Roll over Lead L<n> -> L<n+1> in the pinned project/profile with checkpoint.md handoff and generation fencing.",
            json!({
                "type": "object",
                "properties": {
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160 },
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
                "required": ["call_id", "work_id"],
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
            "Request an immediate relay tick for participants, Wait-Sets and pending deliveries.",
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
            "Inspect durable state, pending deliveries and relay telemetry.",
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
                    "call_id": { "type": "string", "minLength": 1, "maxLength": 160 },
                    "backend": { "type": "string", "enum": ["chatgpt", "codex"] },
                    "ref_id": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "profile": { "type": "string", "maxLength": 64 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 180 }
                },
                "required": ["call_id", "backend", "ref_id", "title"],
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
