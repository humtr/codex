# Session-independent durable orchestration — design contract

> **2026-11-13 integration note:** This is a non-normative design for a
> later development phase, **not** present runtime behavior or an instruction
> to change the installed release. The latest user-ordered path is actual
> A/O/W C1-C8 verification, 0.167.2 product tests and evidence/document
> consolidation **before** implementation of wakeup/queue/budget changes.
> Where an earlier paragraph prioritizes deployment or release activation,
> that older scheduling premise is superseded. The single active plan is
> [HANDOFF](../../../HANDOFF.md); current evidence is
> [CURRENT](../../../CURRENT.md).

Status: PROPOSED / NOT IMPLEMENTED / for independent review and later development.
Owner: user-authorized orchestration, not any individual ChatGPT session.
Scope: assistant, orchestrator, and dynamically provisioned worker sessions.
Related: ../../DOCUMENTATION.md; ../../spec/bridge/WORK.md; ../../spec/bridge/OWNERSHIP.md; ../../spec/bridge/RETRY_AND_EVIDENCE.md.
This proposal does not modify SPEC.md, change approvals, or assert present runtime capability.

Companion pilot (2026-11-13): the private ChatGPT **Durable Orchestration plugin v0.1.3** contains model-led guidance, Native Busy recovery instructions (v0.1.2), and an assistant-facing session dashboard (v0.1.3). Those are **skill/presentation behavior**, not shipped Codex Native/TermuxBridge changes or a durable unattended runtime. Wrapper-level budget revision and addressed report/ACK interoperability are now explicit **deferred development requirements**, with CLI versus Linux AppHost interoperability subject to a real product-path choice. See sections 14-17 and the [Codex/ChatGPT bridge design](CODEX_CHATGPT_ORCHESTRATION_BRIDGE_DESIGN.md). The immediate priority is 0.167.2 release acceptance and safe authorized runtime replacement; no orchestration source change is authorized by this proposal.

## 1. Objective and non-negotiable invariant

A user delegates a bounded or multi-stage job to an assistant. The job must remain durable through repeated assistant, orchestrator, and worker conversation replacements. The job ends only when the user-defined completion criteria have been independently verified, or the user authorizes cancellation or a change of scope. Model turns, tool sessions, network streams, UI visibility, and context windows are never the job lifetime authority.

The goal is to preserve the model's role in planning, delegating, interpreting evidence, recovering uncertainty, and adapting worker allocation. The supporting transport/journal must be deterministic and durable, not a second Native executor.

## 2. Actors and ownership

- User: owns the job objective, authorized boundaries, and changes to completion criteria.
- Assistant (A): replaceable interactive user interface and report recipient; may request creation, inspect state, submit authorized changes, and relay a final verified result. The assistant does not own the job's lifetime.
- Orchestrator (O): persistent logical actor with replaceable ChatGPT reasoning sessions O1, O2, ...; decomposes the job, sets dependency graph, allocates workers, verifies results, resolves conflicts, and submits final report.
- Worker (W): persistent logical actor with replaceable reasoning sessions W1, W2, ...; receives scoped tasks and reports evidence/results. Worker identities are independent of ChatGPT conversation IDs.
- Job runtime: durable ledger, ordered queues, lease/fencing, scheduler/wakeup, checkpoint store, credentials/project binding, and recovery coordinator. A model is invoked when work, failure, approval, or decision needs reasoning.
- Native: retains its existing thread, writer, tool-execution, reviewer, and effect authority. Orchestration may inspect and route; it must not create a competing executor or override approval outcomes.

Every job, logical actor, worker task, ChatGPT conversation, Native work/thread, queue message, and effect receipt receives a distinct identifier. All relationships are saved before effects are dispatched.

## 3. Worker count is demand-driven, not fixed

The orchestrator MUST choose and continually adjust the number of workers according to task graph, separable file ownership, workload, available independent execution resources, approval limits, tokens/effort budget, and observed contention.

Allocation algorithm:
1. Decompose the completion contract into tasks with dependencies, evidence and file/Native writer boundaries.
2. Estimate parallelizable ready tasks, cost, expected value, tool load, and collision risk.
3. Create only as many distinct worker identities/sessions as independent work and resources warrant, subject to a per-job concurrency ceiling and user-approved budget. Zero workers is valid while waiting for prerequisites or review.
4. Scale out when new independent ready tasks exist and capacity is safe; scale in by retiring idle worker instances, preserving completed evidence and logical task ownership.
5. Reassign failed or long-running tasks only after checking original call receipts, retained state and writer ownership. Reassignment is not reexecution of uncertain effects.
6. Do not spin up a worker for simple work the orchestrator can complete safely. Do not keep a fixed G1/G2/G3 count as a product assumption.

A worker cannot write another worker's owned files or share the same Native writer concurrently merely because distinct model sessions exist. The orchestrator must avoid creating so many workers that resource-busy or AppHost limits make the system less productive.

**Busy is scoped, not a global stop decision:** inspect the exact affected call, its owner and resource scope before reducing capacity. A pending opaque Native command may forbid conflicting tool execution while an isolated, submitted-inference **model-only** worker can still run. Choose zero workers only when no independent ready task/capacity remains, not automatically on the first Busy error. Preserve real resource owners and approval boundaries.

## 4. Durable job completion contract

Persist a versioned completion contract at creation:
- Immutable job_id; initiating user and authenticated account/project binding.
- Objective, in-scope deliverables, explicitly forbidden effects, requirements and priority.
- Acceptance criteria per task and for the aggregate job, including what constitutes real product-path evidence.
- Authorized execution environments, reviewer and model policy, dependencies, budget, maximum concurrent workers, and any approval gates.
- Current contract version, exact user-authorized revisions, authorizing input references, and an audit trail. Future orchestration defaults are max 5 active sibling workers, 256 App starts, 1024 Native calls; elapsed duration is configurable rather than a silently invented user default (see section 17).
- Final result routing policy, reporting language, time zone, and notification destination.

A worker completion is not a job completion. An unverified test report or source commit is not proof of live behavior. The orchestrator may mark the job COMPLETED only when each mandatory criterion has evidence and a final verification manifest is durably committed. User-declared cancellation terminates only authorized future work and must settle in-flight effects separately.

If blocked by an unavailable connector, account, approval, external service, or unconfirmed execution, mark BLOCKED or RECONCILIATION_REQUIRED and preserve the job. Never silently downgrade the objective, report success, or lose the obligation.

## 5. Queue and message contract

All assistant-orchestrator-worker communication MUST be addressed to stable logical actor IDs through persistent queues, not to short-lived conversation IDs. Required envelope fields:
- message_id, job_id, logical_sender_id, logical_recipient_id, actor_generation;
- event_type, schema_version, per-recipient monotonic sequence, timestamp, causal_parent_id;
- source conversation/Native work IDs when applicable;
- input fingerprint, idempotency key, payload location/hash and required action;
- ACK/delivery state, original receipt or reason for uncertainty.

Message kinds: ASSIGN, REVISE, PROGRESS, QUESTION, APPROVAL_REQUEST, HEARTBEAT, RESULT, ERROR, RECONCILE, REPLACE, COMPLETE, CANCEL_REQUEST, FINAL_REPORT.

The authoritative state transition is PERSISTED -> OFFERED -> RECEIVED -> ACCEPTED -> RUNNING -> EFFECT_SETTLED -> RESULT_RECORDED -> PARENT_ACKED. A queue insertion ACK only proves PERSISTED, not consumption or success. Messages may be delivered at least once; effects MUST be idempotent by original operation key or reconciled from the original receipt rather than replayed. Every ASSIGN MUST identify the **exact return destination**, Core execution home, native thread, result/ERROR/PROGRESS report command and ACK procedure before dispatch; "report back" by itself is insufficient. See the deferred bridge design section 4. Maintain explicit dead-letter and error pathways; never mark an unknown result as failed or completed by inference.

Queue ordering is per logical recipient with dependencies respected. A recipient lease is fenced before another generation can accept new effects. An exhausted or disconnected ChatGPT session does not delete unacknowledged messages.

## 6. Three independent replacement flows

### 6.1 Assistant A1 -> A2 -> A3
The job's record points to the current authorized reporting endpoint, not to A1. New assistant sessions verify user/account/project/job identity and attach via an authenticated claim with monotonic generation. Updating the active report recipient is atomic. Previous recipients become read-only/observers; stale messages cannot change job scope. An assistant disconnect is not a stop signal for workers.

### 6.2 Orchestrator O1 -> O2 -> O3
Before planned replacement, persist an evidence-based checkpoint (contract, DAG, tasks, queue cursors, leases, receipts, unsettled effects, per-worker branch/worktree and observed progress). O2 reads the durable record, compares it to live source and Native state, claims the next actor generation and fences O1's new-effect rights. O2 receives queued worker reports addressed to O regardless of which ChatGPT conversation generated them.

During an unplanned failure or context overflow, the runtime resumes from the last durably acknowledged event and independently observes outstanding effects. It MUST NOT presume a previous model output or tool effect completed because a stream was closed.

### 6.3 Worker W1 -> W2 -> W3
Persist the worker task, Writer boundaries, authoritative Native work/thread, exact Git HEAD/diff, receipts, continuation policy, pending messages and concrete next steps. Resume the same logical worker with a new model conversation only after fencing the old effect rights. Verify original work receipts before continuing; do not rerun compilation, external side effects or a possibly delivered user input under a new call ID. A replacement may be automatically triggered by token budget, timeout, crash, model deprecation, or unavailability.

## 7. Context budgets and checkpoint quality

Compute model-specific safe context thresholds and reserve space for tool returns, evidence, and handoff. A configurable threshold (for example 70-80% of measured usable budget) triggers proactive replacement; this is an initial tuning target, not a fixed product fact.

A checkpoint MUST include: job/contract revision, logical actor/generation, objective, scope, verified completed work, pending criteria, exact evidence hashes, immutable source refs, Git HEAD and dirty paths, Native writer/thread/receipt references, queue cursor and unacknowledged messages, blocked reasons, risk/approval boundaries, and the next safe observation/action. Preserve original logs and source records externally; never treat a summarized narrative as execution authority.

Keep checkpoints short enough for the successor to load selectively. Store large outputs by digest and retrieval path. The successor must reread current AGENTS, CURRENT, affected specifications, source and relevant receipts, rather than trusting inherited summaries.

## 8. Scheduling, monitoring, restart and durability

The runtime must survive ChatGPT app/process disconnect, Termux/Core/AppHost restart, assistant replacement and pending model responses. Persist job state and queues transactionally (for example SQLite WAL with explicit schema migrations, per-message acknowledgments and reconciled cursors). Provide an event-based dispatcher or an independent scheduled wakeup, rather than relying on a single active assistant chat to poll.

Wake the orchestrator on worker result, approval request, crash, deadline, prolonged silence or runnable dependency. Require actual confirmation before marking delivery. Health monitors classify busy resources by evidence and owner; they must not kill or release active work merely because a timer expired. When Stop/status themselves fail behind a shared resource guard, retain original pending effects, report the recovery-path defect and continue independently proven non-conflicting work; never clear shared claims or stop an unrelated process merely to make the global Busy flag disappear.

A skill-only plugin may express this protocol and use currently accessible tools, but cannot itself guarantee automatic execution after all ChatGPT sessions have ended. Durable background execution requires an available scheduler/automation/agent-host capability and persistent tool permissions. A prototype must record this distinction explicitly.

## 9. Result delivery, routing and user reporting

During an active job, orchestrator pushes meaningful progress to a **verified, explicitly bound** logical assistant recipient and/or authorized user-visible notification path; reporting must not depend on the assistant manually polling. An Android/web ChatGPT conversation is not automatically a Native Codex thread. Worker results MUST be queued to the orchestrator's exact Native receiver with independent consumption/ACK evidence; the orchestrator MUST likewise send final/failed status to the authorized assistant endpoint, or persist pending delivery when that capability is absent. Progress begins with compact `[HH:MM]` local time (Asia/Seoul for this project), actual worker count (not configured concurrency), completed evidence, uncertain/blocked work, next decision, and changes since last report. Report at milestones or meaningful changes; avoid noisy repeated status.

### 9.1 Assistant-facing session dashboard (plugin v0.1.3)

Whenever an assistant actually creates, discovers, or resumes an orchestrator/worker model session, render a **single compact dashboard for that job**. This is the assistant's *view*, not an independent scheduler, a continually refreshed website, or a new Native API. Use grouped cards or a simple portable table; include **one row per actual session/generation**, not one row per logical job alone. Show an initial snapshot even before the saved Chat is available, then refresh only after meaningful state changes or a user status request.

The dashboard must show:
- Header: short job title and stable job ID, local `[HH:MM]` last-observed time, job completion/blocker summary and next action.
- Totals: actual O generations, registered worker sessions, running/completed/blocked/unknown counts. Never present concurrency budget as worker count.
- **Orchestrators**: O1/O2/... row per generation, mission, actual phase, one-line verified result or blocker, and own ChatGPT conversation link where verified. Retain replaced/interrupted O1 next to successor O2.
- **Workers**: W1/W2/... row per real worker, assigned task, actual phase, one-line result or uncertainty and individual verified conversation link. Do not hide pending or interrupted workers.
- Optional compact technical detail: original `work_id`, `thread_id`, `conversation_id`, model/effort **only if observed**; do not expose controller tokens, account secrets or long raw receipts in the main view.

Verify each conversation ID against saved-chat observation in the **intended account and ChatGPT project** before constructing `https://chatgpt.com/c/{conversation_id}`; Native work/thread IDs are not chat IDs. If only `create` succeeded, show `생성됨 · 추론 시작 전` and `대화 미확인`. `queue accepted=true` means `큐 등록`, not work completed. A started but unobserved model remains `실행 확인 중`; `completed_result`/saved final with matching identity proves actual completion; `stop_requested`, timeout and `MCP server is not initialized` require original-receipt reconciliation. A verified saved Chat link does **not** guarantee user-side UI visibility.

Suggested simple rendering (the link cells are **labels/placeholders**, never invented URLs):

| 세션 | 작업 요약 | 검증 상태 | 대화 |
| --- | --- | --- | --- |
| O1 | 최초 분배 | 중단 · 인계 기록 확인 | 검증된 O1 대화 링크 |
| O2 | 잔여 작업 인계 | 완료 · 원본 결과 확인 | 검증된 O2 대화 링크 |
| W1 | 첫 번째 작업 | 완료 · 결과 영수증 확인 | 검증된 W1 대화 링크 |
| W2 | 두 번째 작업 | 큐 등록 · 모델 실행 미확인 | 대화 미확인 |

The actual UI separates O and W into sections; the combined table above illustrates per-session facts. On assistant replacement reconstruct from current work/chat receipts, mark any stale snapshot **마지막 확인 [HH:MM]**, and never claim a previous message updates automatically. The orchestrator may forward a `session_status_snapshot` with the underlying IDs/evidence; if delivery fails, the assistant reconstructs from reachable source records. Dashboard visibility is **not** an acceptance criterion replacing the job's original completion contract.

On final verified completion:
1. Persist the final proof manifest and exact result.
2. Deliver FINAL_REPORT to the most recently authorized assistant session; wait for receipt/ack.
3. If no authorized assistant is reachable, create or select a report conversation in the same authenticated account and requested ChatGPT project, deliver via a supported authorized API, and read back the exact saved message/destination.
4. Keep the final report durably pending if chat creation, connection or visibility cannot be verified. No cross-account or cross-project fallback.
5. Never send a duplicate final report merely because delivery confirmation was lost; resolve original receipt/ID.

A mere saved conversation ID does not prove that the user can see it in their chat list. Account, project, visibility and host acknowledgment must be checked. The user may replace assistant sessions without changing the job contract.

## 10. Security, authorization and non-goals

- Model reasoning can recommend actions but cannot invent permissions, alter policies, bypass deny, or auto-approve regulated/high-risk effects.
- Never change signed operating releases, login accounts, user data, shared storage or unrelated writers without explicit separate authorization.
- Do not move work between ChatGPT accounts/projects on fallback; verify identity before creating or resuming model sessions.
- Native's effect ledger is authoritative; orchestrator checkpoints are derived state and pointers, not second executions.
- Every worker operates within explicit file/environment/Native writer boundaries; concurrency must be proven for positive and negative cases.
- The initial plugin experiment does not implement the full product contract in Codex, nor does it promise never-ending ChatGPT inference without a real host wakeup capability.

## 11. Minimum reference interfaces (future design)

Job: create, inspect, update_authorized_contract, cancel_authorized, list_unfinished, proof_manifest.
Actor: allocate, claim_generation, renew_lease, fence, checkpoint, resume_from_checkpoint.
Queue: append_if_absent, claim_next, ack_received, ack_accepted, settle_receipt, ack_parent, inspect_dead_letters.
Workers: adjust_concurrency, assign, reassign_if_safe, observe, report_result.
Recipient: bind_latest_assistant, deliver_progress, deliver_final_exactly_once, verify_visible_chat.
Runtime: reconcile_pending_effect, trigger_on_event, recover_after_restart, verify_capabilities.

Each mutating operation requires an idempotency key and an observable outcome; timeouts return UNKNOWN with original receipt lookup, not permission to replay.

## 12. Required acceptance matrix

1. A1 ends: O and workers continue and A2 can observe without recreating tasks.
2. O1 exhausts context during W work: O2 receives original queue events, exactly one logical owner, unchanged contract.
3. W1 exhausts context during an active command: W2 observes original tool completion once; no duplicate side effect.
4. O dynamically scales worker count 0 -> N -> M based on independent ready tasks and resource capacity, with writer conflict refused.
5. Queue insert ACK without delivery cannot be reported as completed; after restart all unacked events remain recoverable.
6. AppHost/Termux restart preserves source conversation mapping, saved receipts and unresolved effects; no forced login/account switches.
7. A2 replaces A1 several times; final report targets latest authorized A or a verified same-account/project report chat.
8. Network timeout/HTTP 413/MCP uninitialized and resource busy leave the job recoverable; failed observation does not imply failed execution. Confirm original `work_id`/call receipt before any replay and test a non-conflicting model-only task while an unrelated opaque effect remains unresolved.
9. Every mandatory completion criterion must pass with source and device/app evidence as applicable; partial worker results cannot close job.
10. Tampered envelopes, stale leases, wrong project/account, unauthorized changes, and uncertain effects are rejected without execution.
11. Operational reports are timestamped and evidence-backed; worker and orchestrator telemetry remain inspectable.
12. Plugin-only test reports limitations honestly: without an event host/wakeup there is no guaranteed autonomous continuation after all sessions stop.
13. If a running orchestrator is interrupted after submitting a worker start, a successor/observer retrieves that **same worker** by original work ID and verifies the existing saved final; no duplicate child start.
14. Every created/discovered O/W session appears in the assistant dashboard with distinct role, phase, task summary and a saved-chat-verified link or explicit `대화 미확인`; preserve prior generations across handoff and show `[HH:MM]` observed time.
15. On a Stop that returns an error after fencing, `stop_requested` remains distinct from `stopped`; no unrelated live shared owner or claim is cleared.

## 13. Delivery phases and relationship to existing Codex work

Phase P0: review this proposal against SPEC, Bridge ownership and existing Native/Provider semantics; record approved contract separately.
Phase P1: proof-of-concept skill/plugin using existing tool capabilities; establish account/project checks, worker allocation, Busy diagnosis, assistant dashboard and bounded queue/receipt discipline. **Plugin v0.1.3 is available as a private pilot, not a full acceptance of P1-P5.**
Phase P2: durable journal, logical actors, checkpoint schema, generation fencing and message acknowledgment.
Phase P3: model-session replacement, event-driven dispatch/wakeup, dynamic worker scaling and recipient handoff.
Phase P4: exhaustive positive/negative tests, real ChatGPT saved conversation visibility, restart/recovery, user-visible results.
Phase P5: carefully migrate existing jobs after original-effect reconciliation and explicit acceptance; never replay uncertain calls or disrupt operating releases.

This document is a future development candidate. Do not represent Phase P1 as proof of P2-P5. Existing W0-W20 completion targets remain independent until separately accepted.

## 14. Pilot lessons from v0.1.2 (Busy and uncertain execution)

A bounded, isolated `durable-bridge-20261113-a` test through **TermuxBridge `codex_sessions` with `inference=submitted`** verified separate ChatGPT GPT-6 model conversations for an orchestrator and two workers. The orchestrator selected work, created actual independent worker Native work records, queued ASSIGN messages, and the workers consumed the queued inputs and saved independent final answers (`17 * 19 = 323`, `23 * 29 = 667`) in the intended `Codex Development` ChatGPT project. This proves a **Codex/Bridge-hosted ChatGPT model-session path**, not ChatGPT's internal Pro-style subagent mechanism.

The independent pilot also exposed limits that must not be silently smoothed over:

- Unrelated G1 `process_session=49962` retained `pending_result=true` after the underlying Cargo process was no longer observed. G1 Stop changed the work to `stop_requested`, but the Stop/status path returned `native_resource_shared_guard_failed`; this is **not** confirmed Stop or effect settlement.
- A separate process-shared reservation had a **live, matching-PID-and-start-time owner**. It was not cleared or killed. That owner is not interchangeable with G1's opaque pending result.
- Despite the G1 unresolved Native effect, isolated model-only worker assignments **succeeded**. The same Busy observation must not prohibit independent model requests by default.
- One orchestrator round became `interrupted` while a worker launched. An assistant-mediated O1 -> O2 checkpoint preserved W1's verified output and scoped O2 to the remaining W2, without replaying W1. **Autonomous** replacement/fencing was **not** demonstrated.
- An O2 worker `start` reported `MCP server is not initialized`, but later read-only `codex_sessions events.completed_result` and `codex_chats read` for the **original** worker proved the job had already completed. Never invent a new call ID after this error.

**Operator decision sequence** for the plugin now: `identify owner -> inspect original effect, process and saved Chat -> classify actual contention -> request official Stop only if authorized and safely idle -> reconcile exact original receipt -> continue independently non-conflicting tasks -> checkpoint and hand off with unresolved facts`. Do not perform global resets, delete `claims.json`, kill unrelated PIDs or treat a speculative process disappearance as an execution receipt.

The pilot demonstrates these bounded capabilities only. It does **not** prove robust high-concurrency execution, automatic recovery across device/process restart, automatic wakeup, native parent-child ownership, guaranteed visibility of all saved Chats, or final delivery to a newly designated assistant.

## 15. v0.1.3 assistant dashboard: immediate capability versus future requirements

The private plugin **v0.1.3** introduces a skill-backed reporting requirement (section 9.1): immediately show session summaries with links after O/W creation and refresh at meaningful changes. This uses existing ChatGPT answer rendering; it is not a standalone hosted dashboard, web page, stored interactive monitor, or continuously polling system.

**Immediate scope:** retrieve verifiable work/chat identifiers, render a compact snapshot with separate orchestrator and worker groups, preserve generations and completed tasks across handoff, use saved-chat-verified links, and mark unavailable/unknown observations explicitly. Dynamic worker allocation remains the orchestrator's decision.

**Deferred questions for later independent review:**

1. Does the wrapper/TermuxBridge need a structured `session_status_snapshot` and stable logical O/W actor IDs, or can the skill derive them safely from existing `codex_sessions` / `codex_chats` receipts?
2. Can account/project-scoped parent linkage, saved Chat visibility confirmation, and completion callbacks be exposed without duplicating Native effect authority?
3. How should an interrupted AppHost orchestrator be resumed/fenced without manual assistant intervention or duplicate child start? Which evidence establishes exact ownership after an MCP initialization loss?
4. Should Stop/status be inspectable when the very shared guard being diagnosed is unhealthy? Repair must preserve live process owner identities and never bypass denied effects.
5. How can event-driven dashboard updates be delivered to the currently authorized assistant without implying perpetual model inference or relying on unsupported UI refresh semantics?

The user has now explicitly requested a **deferred wrapper/Bridge improvement design** for addressed O/W/A reporting and four-field budget revision. Its implementation remains deferred until after the 0.167.2 release path. First test the existing CLI queue and Linux AppHost saved-Chat interop; prefer minimal demonstrated fixes to a new orchestration framework. See the [bridge design](CODEX_CHATGPT_ORCHESTRATION_BRIDGE_DESIGN.md).

## 16. Implementation and documentation boundary

Current deliverables are: this **non-normative proposal** and a **private ChatGPT skill/plugin v0.1.3** that documents Busy handling, evidence-backed model work and an assistant-facing session dashboard. The separate pilot's saved conversation/tool receipts establish only their original bounded test scope. The plugin and its packaged SQLite helper do **not** implement an always-on scheduler, a true Native executor, source-level status/Stop repair, cross-session durable delivery, autonomous session replacement, or a user-visible hosted dashboard.

The user has **deferred implementation of the planned Codex wrapper orchestration improvements** for separate future review, with 0.167.2 final acceptance, deployment and safe runtime replacement prioritized. Preserve that decision; do not advance P2-P5, modify signed releases or claim broad product acceptance solely because the dashboard renders or two small worker calculations succeeded.

For future implementation, review the current SPEC, Bridge ownership/effect/Stop contracts and actual deployed source version independently; distinguish possible wrapper fixes from skill-only orchestration. Keep the immutable completion contract and test requirements in sections 1-13 as a proposal, not a claim of current delivery.

## 17. Deferred Codex wrapper budget and addressed-report implementation requirements

The user requested that this proposal be expanded into a **concrete future development design**, without implementing it in the current 0.167.2 release path. The companion [Codex/ChatGPT orchestration bridge design](CODEX_CHATGPT_ORCHESTRATION_BRIDGE_DESIGN.md) is the detailed, reviewable authority for research hypotheses, producer/consumer scope, implementation sequence, positive/negative tests and acceptance boundaries; it is still **non-normative and unimplemented**.

**Decision already made for the future target:**
- Default orchestration capacity: `max_parallel=5`, `max_app_starts=256`, `max_native_calls=1024`; `max_elapsed_seconds` must be adjustable, and its global numeric default remains a separate configuration/user decision (12 hours is a design example, **not** a user-selected global default).
- `extend_budget` must be revised so all four effective dimensions can be changed safely, including live submitted-inference paths after receipt reconciliation, with immutable consumed counters, stable original IDs, parent ceilings, safe decreases, reentry and Host admission proof. Current source rejects parallel/App changes and submitted-inference extension.
- Worker ASSIGN must specify O's exact native thread/Core home, authenticated logical recipient, report envelope/command, progress/result/error criteria, queue receipt and actual receiving-turn/parent ACK. An assignment that lacks return-path instructions is invalid.
- O must similarly deliver its `FINAL_REPORT` or `ERROR` to the **latest authenticated assistant recipient**, with a distinct ACK, without equating the current ChatGPT Android/web chat with a Native thread.
- Distinguish Native queue insertion, receiving-turn consumption, actual Linux AppHost model submission, final model result and parent/assistant acceptance. An accepted `codex queue` or a successful saved-Chat `chat_input` alone cannot prove the full chain.

**Architecture review remains open:** evaluate CLI/Native-only actor routing versus Linux AppHost-saved Chat-only messaging versus a **single Native queue with a bound Linux AppHost saved-chat adapter**. The latter is the preferred research candidate because it can preserve real ChatGPT conversation/account/project and native approval/effect authority without forcing every actor into TUI. No app-backed queue auto-wakeup or current Android assistant recipient is assumed supported until the full source-bound positive/negative receipts exist.

**Do not start implementation as a side effect of this documentation**. Complete 0.167.2 source CI, actual product path, signing/publication and runtime replacement only under their separate approvals and existing rollout safeguards. After that work, revalidate source/version/capabilities, select the architecture using the detailed acceptance matrix and implement the smallest cohesive wrapper changes.
