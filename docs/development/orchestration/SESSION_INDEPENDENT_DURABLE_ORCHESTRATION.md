# Session-independent durable orchestration — design contract

Status: PROPOSED / NOT IMPLEMENTED / for independent review and later development.
Owner: user-authorized orchestration, not any individual ChatGPT session.
Scope: assistant, orchestrator, and dynamically provisioned worker sessions.
Related: ../../DOCUMENTATION.md; ../../spec/bridge/WORK.md; ../../spec/bridge/OWNERSHIP.md; ../../spec/bridge/RETRY_AND_EVIDENCE.md.
This proposal does not modify SPEC.md, change approvals, or assert present runtime capability.

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

## 4. Durable job completion contract

Persist a versioned completion contract at creation:
- Immutable job_id; initiating user and authenticated account/project binding.
- Objective, in-scope deliverables, explicitly forbidden effects, requirements and priority.
- Acceptance criteria per task and for the aggregate job, including what constitutes real product-path evidence.
- Authorized execution environments, reviewer and model policy, dependencies, budget, maximum concurrent workers, and any approval gates.
- Current contract version, exact user-authorized revisions, authorizing input references, and an audit trail.
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

The authoritative state transition is PERSISTED -> OFFERED -> RECEIVED -> ACCEPTED -> RUNNING -> EFFECT_SETTLED -> RESULT_RECORDED -> PARENT_ACKED. A queue insertion ACK only proves PERSISTED, not consumption or success. Messages may be delivered at least once; effects MUST be idempotent by original operation key or reconciled from the original receipt rather than replayed. Maintain explicit dead-letter and error pathways; never mark an unknown result as failed or completed by inference.

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

Wake the orchestrator on worker result, approval request, crash, deadline, prolonged silence or runnable dependency. Require actual confirmation before marking delivery. Health monitors classify busy resources by evidence and owner; they must not kill or release active work merely because a timer expired.

A skill-only plugin may express this protocol and use currently accessible tools, but cannot itself guarantee automatic execution after all ChatGPT sessions have ended. Durable background execution requires an available scheduler/automation/agent-host capability and persistent tool permissions. A prototype must record this distinction explicitly.

## 9. Result delivery, routing and user reporting

During an active job, orchestrator pushes meaningful progress to the logical assistant recipient and/or a user-visible notification path; reporting must not depend on the assistant manually polling. Progress includes Asia/Seoul timestamp when the user's timezone is Asia/Seoul, active worker count (actual, not configured), completed evidence, uncertain/blocked work, next decision, and changes since last report. Report at milestones or meaningful changes; avoid noisy repeated status.

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
8. Network timeout/HTTP 413/MCP uninitialized and resource busy leave the job recoverable; failed observation does not imply failed execution.
9. Every mandatory completion criterion must pass with source and device/app evidence as applicable; partial worker results cannot close job.
10. Tampered envelopes, stale leases, wrong project/account, unauthorized changes, and uncertain effects are rejected without execution.
11. Operational reports are timestamped and evidence-backed; worker and orchestrator telemetry remain inspectable.
12. Plugin-only test reports limitations honestly: without an event host/wakeup there is no guaranteed autonomous continuation after all sessions stop.

## 13. Delivery phases and relationship to existing Codex work

Phase P0: review this proposal against SPEC, Bridge ownership and existing Native/Provider semantics; record approved contract separately.
Phase P1: proof-of-concept skill/plugin using existing tool capabilities; establish account/project checks, worker allocation and bounded queue/receipt discipline.
Phase P2: durable journal, logical actors, checkpoint schema, generation fencing and message acknowledgment.
Phase P3: model-session replacement, event-driven dispatch/wakeup, dynamic worker scaling and recipient handoff.
Phase P4: exhaustive positive/negative tests, real ChatGPT saved conversation visibility, restart/recovery, user-visible results.
Phase P5: carefully migrate existing jobs after original-effect reconciliation and explicit acceptance; never replay uncertain calls or disrupt operating releases.

This document is a future development candidate. Do not represent Phase P1 as proof of P2-P5. Existing W0-W20 completion targets remain independent until separately accepted.
