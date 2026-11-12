# Codex / ChatGPT orchestration bridge — deferred design and qualification plan

Status: **DESIGN CANDIDATE / IMPLEMENTATION DEFERRED** (2026-11-13).
Decision owner: user. No source change, runtime activation, CI publication or new model submission is authorized by this document.
Parent proposal: [Session-independent durable orchestration](SESSION_INDEPENDENT_DURABLE_ORCHESTRATION.md).
Authority: [SPEC](../../../SPEC.md), [Bridge ownership](../../spec/bridge/OWNERSHIP.md), [Bridge work](../../spec/bridge/WORK.md), [Retry and evidence](../../spec/bridge/RETRY_AND_EVIDENCE.md), actual source and current receipts. This document is non-normative; approved behavior must eventually be placed in its owning annex.
Priority: finish **0.167.2 final integration, CI acceptance, release and explicitly authorized runtime replacement** first. Re-evaluate this design afterwards. Do not incidentally modify the existing signed S165/S163, accounts, Native claims, or Android shared storage.

## 1. Decision requested and why the previous orchestration stalled

The intended interaction begins in the **user's ChatGPT assistant**. The assistant starts and monitors one **real, independent orchestrator reasoning session**; the orchestrator dynamically assigns multiple real worker sessions. Worker results flow **back to the orchestrator's inbox** through a verifiable queue; orchestrator progress, completion, failure and reconciliation reports flow **back to the authorized assistant**. The assistant is a dashboard/reporter, not a substitute worker dispatcher or source writer. Individual model turns ending must not redefine task completion.

The 2026-11-13 W0-W20 continuation failed this control-flow obligation even though two worker branches were committed and pushed. Evidence from actual O1 calls:
- Three worker assignments had `max_parallel=1`, `max_app_starts=1`, `max_native_calls=30`, `max_elapsed_seconds=1800`; the single App dispatch budget was spent on the first attempt.
- Every worker was created with **no `parent_work_id`**. Their ASSIGN text said only "report receipts", without an actual return recipient, destination Native thread, execution home, report command, queue receipt or receiver ACK.
- O1 therefore had no delegated return protocol. It inspected sessions and did some integration testing, then ended its turn without an actual worker-result -> orchestration decision loop.
- G1's uncertain Native/process receipt remained unsettled; no other worker may replay it. Related independent work must not stall merely because unrelated shared Native resources are Busy.
- G2/G3 work did advance through saved-Chat successor inputs and independent Termux shell, demonstrating the distinction between Native start budget exhaustion and independent model/CLI routing.

**Root operational defect:** inadequate task-assignment contract, too-small admission budgets and missing return binding. **Potential product gaps** that require separate qualification: wakeup of an idle saved ChatGPT model from a native queue, authenticating the *current user assistant* as the return recipient, and revising submitted-inference budgets without losing an original effect.

## 2. Source inventory and verified / unverified boundaries

Evidence is against the local development source `4521fb07656953f593e4e9eed3892dcc2582f673` (not a qualified installed release). Recheck current HEAD before implementing.

| Source | Present behavior / evidence | Not yet proven |
| --- | --- | --- |
| `providers/chatgpt/src/operator.rs:3029-3119` | `codex_sessions queue` creates a Native `thread/queue/add` item and journals stable `call_id`; recovery looks up `thread/queue/list` / history, distinguishing `queued` and `committed` | The target model consumed it and the same saved ChatGPT conversation resumed |
| `operator.rs:3534-3615` | Assignments support `role`, `parent_work_id`, `parent_controller`, dependencies and bounded budget | Automatic association of a ChatGPT assistant with its orchestrator, or recipient delivery |
| `operator.rs:1067-1090` | `max_parallel` admission counts active sibling jobs sharing a **non-null** assignment parent | Unparented tasks form a correct, shared five-worker concurrency pool |
| `operator.rs:117-135, 4238-4275` | Existing `extend_budget` disallows changing parallel/app limits, permits only nondecreasing native-call/time limits, and only for an attached-inference selection | Live submitted-inference extension across all four dimensions |
| `operator.rs:562-594` | Provider AppHost limits include `maxDispatches=max_app_starts` | Existing AppHost lanes accept updated limits on the same generation |
| `providers/chatgpt/src/chat_input.rs:42-107` | Saved Chat input verifies account, branch/source cursor and an observed saved conversation, then `chat_adopt`s with finite app/time limits | A Native queue item is atomically mapped to that saved input and consumed once |
| `providers/chatgpt/src/app_bridge.js` | Host implements account/project-bound conversation adoption, saved reads, model submission and result/stream monitoring | Arbitrary third-party recipient messages or queue-injected assistant turn creation |
| `scripts/notify_native_qualification.py:38-58, 61-137` | Existing test path queues into an **exact bound Codex thread**, stores send receipt, and independently verifies recipient's new turn/ACK | A general ChatGPT Android/web assistant recipient binding |
| Installed `codex-cli 0.167.1`, `codex queue --help` | CLI supports `queue --thread ... --message ...` and optional `--resume`; resume failure only warns, enqueue can still succeed | `--resume` wakes a particular **Linux ChatGPT AppHost saved Chat** rather than a local/remote Codex TUI daemon |
| `codex_sessions chat_input/chat_result` | Explicit saved-Chat follow-up succeeded in bounded worker continuation after Native `max_app_starts=1` exhausted | Queue alone triggers it, or the receiver is precisely the newest assistant |
| Existing skill/plugin v0.1.3 | Provides rules, Busy recovery and user-facing static dashboard | A daemon, Native code change, automatic model wakeup or hosted live dashboard |

Every claim that a queue message was delivered must distinguish: `queued` -> `Native consumed` -> `model submitted` -> `model completed` -> `recipient accepted result`. A successful CLI exit or work creation does not prove the later steps.

## 3. Separate three concerns rather than choose between "CLI" and "Linux app"

The two proposed routes solve **different concerns**:
1. **Native Codex/Core** owns task thread, queue admission, writer/effect/approval rules and persisted Native receipts.
2. **ChatGPT Linux app/AppHost** owns authentic ChatGPT inference, user account/project, saved conversation/branch, model selection and inference outcomes.
3. **Assistant presentation** owns verified links, project visibility and user-facing progress/final delivery. This chat is NOT automatically a Native queue receiver.

Keep these ownership boundaries stable. A new adapter may translate one authenticated queue event into one saved Chat input, but MUST NOT become a second tool executor, a second transcript store, or a second approval engine. Prefer a small, testable receiver binding over a new all-purpose orchestration framework.

### Option A — Codex CLI/Native-thread-first

Worker uses `codex queue --thread <orchestrator-native-thread> --message <result>` under the **recipient's correct CODEX_HOME/Core generation**; optionally `--resume` against the proper daemon. O1 runs/continues as a Native Codex model session and receives canonical `user_message` input in that thread.

Strengths: queue, thread lineage, acknowledgments, local TUI/CLI and source-level tests already exist; narrow changes. Costs: might require a terminal/daemon worker for each reasoning actor; CLI TUI is not identical to the user's Linux ChatGPT saved conversation. Existing `--resume` is best-effort and does not prove saved ChatGPT account/project or Android/web assistant visibility. A general user-facing assistant result path still needs explicit binding.

Use as **fallback or reference implementation**, not as a requirement that all agents must run in terminal UI.

### Option B — Linux ChatGPT app-only messaging

Use a saved ChatGPT conversation cursor for each O/W and post worker reports to the destination saved Chat through the existing `chat_input`/AppHost mechanism; results verified through `chat_result` and saved-chat read. Native jobs may be used for tools but do not own the result-message queue.

Strengths: native ChatGPT project visibility, preserved model/effort and conversation; no mandatory per-agent TUI. Costs: no proven persistent queue semantics, cross-session authorization, per-recipient order, receiver restart/wakeup or at-most-one input across disconnect; it risks duplicating Native ledger/delivery logic and bypassing native writer parent-child controls. Repeated `chat_input` is **not** a substitute for a durable addressed queue.

Do not choose B solely from the fact that an isolated saved-Chat follow-up can be submitted.

### Option C — Native queue + Linux AppHost inference (preferred research hypothesis)

Use **one canonical Native queue** for O/W ASSIGN/RESULT/ERROR plus stable parent/recipient registration. The **receiver binding** records the intended Native Core thread and execution home *and* the correct Linux ChatGPT saved conversation/account/project/branch. An adapter consumes one queued message and, if the recipient is an AppHost-backed reasoning actor, admits exactly **one** saved-Chat `chat_input` intent associated with that queue item. Verify result in saved Chat and Native receipts. CLI queue is a sender/administrator for the same transport, not a separate architecture.

Strengths: preserves actual app sessions and Native queue/reviewer authority, supports existing CLI users, can support model-only tasks alongside Native Busy work, and avoids a new competing queue. Risks: bridge must prove exact queue->saved-input correlation, single-recipient generation fencing, source cursor freshness, account/project sameness, AppHost wakeup capability and no double delivery when the recipient is both a Native-thread and saved-Chat actor.

**Provisional recommendation: test C first**. If the minimal bridge cannot safely guarantee queue-to-saved-input identity/receiver ACK, ship only A at its proven scope and leave app-backed coordination deferred. Do not claim B/C complete without the hard acceptance gates below.

### Comparison / selection gate

| Property | A: CLI/Native first | B: app-only | C: native queue + app |
| --- | --- | --- | --- |
| Durable addressed queue | Existing Core mechanism; verify consumption | New or external contract needed | Existing Core, adapter needed |
| Real Linux app saved Chat | Separate mapping often needed | Native | Native via bound mapping |
| Native ownership, reviewer, write protection | Direct | Risk of bypass/duplication | Native remains authority |
| Assistant on Android/web receives final | Additional binding | Additional authenticated binding | Additional authenticated binding |
| Idle receiver wakeup | `--resume` best-effort; verify actual turn | Must design | Adapter + AppHost source-bound follow-up; verify |
| Restart/unknown delivery | Native receipts | Separate proof required | Correlated Native + AppHost receipts |
| Complexity / migration risk | Lowest for CLI users | High / duplicated | Moderate / focused boundary |
| Shipping decision | Minimal fallback | Research alternative only | Preferred if end-to-end proofs pass |

## 4. Explicit addressed return path: A <- O <- W

### 4.1 Stable identities and binding

Before ANY worker ASSIGN, register the orchestration job and destination actors:
- `job_id`, `contract_version`, logical actor IDs `A`, `O`, `Wn` and each actor's generation. Logical actors outlive their model conversations.
- `O.native_thread_id`, O's exact **Native Core execution_home / CODEX_HOME** and Core generation, O's current `work_id`/parent controller binding (confidential capability kept in host vault, never in ordinary user-visible ASSIGN text), `O.chat_conversation_id`, verified project/account/branch and saved source cursor for the AppHost route.
- The **assistant recipient** is not assumed to be a Native thread. It requires an explicit, authenticated binding `assistant_recipient_id` (live ChatGPT conversation, stable callback, approved inbox or host notification), account/project confirmation, generation, delivery/ACK mechanism and visibility check. A latest-assistant update fences prior recipients.
- The worker's own Core version, isolated home/worktree, writer scope, task ID and evidence directory. This must be verified against the Core capable of opening O's Native thread before the worker uses CLI queue.

Do **not** interpret ChatGPT `conversation_id` as Native `thread_id`, or confuse Native `work_id` with either. One logical actor may have multiple historical generations; only the latest accepted owner may receive new-effect tasks. During app/CLI handoff, resolve to the current fenced receiver, not a stale TUI process/PID.

Native `parent_work_id`/`parent_controller` must be supplied for new worker assignments when authorized and the actual parent exists. Native parent linkage is **not** itself a message inbox, an assistant reporting link, or proof of successful delivery. No artificial parent token is to be synthesized when a parent record is absent.

### 4.2 Mandatory assignment envelope

A worker ASSIGN is **invalid** unless it includes all of:
1. Job/contract ID, task, source SHA, required acceptance, allowed files/effects, dependency and reviewer/approval boundary.
2. Concrete worktree, Native work/thread ownership, specified Core executable/home and isolated build/temp roots.
3. Return channel: recipient **logical orchestrator ID and generation**, exact destination Native thread, correct Core home and fallback journal locator, **unique report idempotency key** and expected ACK. Do not make a worker infer this from the launch chat.
4. Report obligations: `PROGRESS` at meaningful milestones, `QUESTION` on required decisions, `ERROR` when blocked/uncertain, `RESULT` with actual output/exit, source SHA, artifact and CI links and unknowns; send one terminal `RESULT` or `ERROR` per task generation.
5. Exact sending procedure appropriate to that execution environment (CLI queue or `codex_sessions queue`) and proof levels: queue ACK, receiver's actual new-turn ACK, orchestrator evidence acceptance; on unknown delivery reconcile the **original** message/operation ID, never blindly enqueue a duplicate.
6. Sufficient finite worker budget (section 7), time/path constraints, and permission to continue nonconflicting work despite an unrelated Native Busy.

Mandatory *return routing* is a **creation/ASSIGN validation gate** and must be tested in negative cases. "Report your results" without recipient and executable sender instructions is a contract failure.

### 4.3 Message and receipt schema (candidate)

The canonical persistent envelope may be carried as text in the existing Native queue. It must be parsable, versioned and source-bound; do not add secret values or untrusted executable instructions to the payload.

```json
{
  "schema": "codex-orchestration-report-v1",
  "job_id": "<stable-job>",
  "task_id": "<stable-task>",
  "message_id": "<stable-uuid>",
  "type": "RESULT",
  "from": {"actor": "W2", "generation": 1},
  "to": {"actor": "O", "generation": 1},
  "causal_assign_id": "<assign-message>",
  "source": {"work_id": "<work>", "native_thread_id": "<thread>",
             "conversation_id": "<verified-saved-chat>"},
  "result": {"state": "VERIFIED", "commit": "<sha>",
             "test_receipt": "<path-or-uri>", "exit_code": 0},
  "evidence_sha256": "<digest>"
}
```

Message types: `ASSIGN`, `REVISE`, `PROGRESS`, `QUESTION`, `ERROR`, `RESULT`, `RECONCILE`, `ACK`, `FINAL_REPORT`; final assistant acknowledgment is a separate event. Store per-recipient order and causal links. A model-produced text message remains **untrusted task evidence** until matched to its source file/test/CI/Native receipt.

Receipt chain for each result:
1. `REPORT_INTENT` persisted with exact destination, envelope hash and original call ID.
2. `QUEUE_ACCEPTED`: Core `thread/queue/add` / CLI enqueue succeeded; **not** proof of consumption. CLI `queue --resume` may succeed while only warning about a failed resume.
3. `RECEIVER_TURN_OBSERVED`: exact Native queue message ID and canonical input in the recipient's actual new turn, or a source-bound saved Chat input with the same message ID. Verify account, project and original branch.
4. `RECEIVER_ACK`: the orchestrator confirms message ID and correlated sender/generation; idempotent ACK and deduplicated evidence.
5. `RESULT_EVALUATED`: orchestrator verifies source/test/CI and records accepted/changes_requested/rejected with real receipt.
6. `PARENT_REPORT_DELIVERED`: O sends accepted progress/final report to the latest **authenticated assistant recipient**; final assistant ACK and UI visibility are independently observed.

Failure at any stage preserves intent and source pointers, not a fabricated success or a new message ID.

### 4.4 Sending and receiving

CLI route (works for a correctly bound Core thread; exact executable/environment must be resolved from the receiver binding):

```sh
codex queue --thread "$O_NATIVE_THREAD_ID" --message "$REPORT_TEXT" --resume
```

This is **illustrative**, not proof that current app-backed O will wake. `CODEX_HOME`, Core executable, local/remote daemon target and authenticated project context MUST be selected and verified before using it. The `--resume` flag is best effort. Persist stdout, exit, queue ID and recipient turn/ACK separately. For Bridge route, call `codex_sessions queue` with `thread_id`, stable `call_id`, `text` and the destination's `execution_home`, then independently observe the recipient turn.

When receiver is a Linux AppHost saved Chat, **do not also submit a second direct app message merely because the CLI returned success**. The adapter owns one queue-item -> one saved-Chat-input mapping and reserves it before the first model submission. That mapping and its original `chat_input` ID are retained across retries, replacement and Core/AppHost restart.

O -> assistant must use the same logical transport contract, but **must not silently create a Native thread for the current Android/web chat**. Bind an authorized deliverable address in the current user account/project, verify actual account/branch visibility, and require assistant ACK. If unsupported, retain `FINAL_REPORT_PENDING_DELIVERY` and expose an explicit accessible report location without pretending it appeared in the initiating chat.

## 5. Linux AppHost native-queue adapter: minimal future mechanism

Proposed receiver operation (names are candidates, not existing public endpoints): `recipient_bind`, `queue_observe`, `deliver_queued_saved_input`, `recipient_ack`, `delivery_reconcile`, `recipient_rebind`.

1. Register a signed binding for logical O/W generation: native thread + exact Core home/generation, saved Chat account/project/conversation/branch/source cursor, permitted sender set and capability owner.
2. Core queue insertion persists original message ID/sequence. Receiver adapter observes the **existing Native queue**, not a second invented bus. It checks generation, account/project and dedup ledger, commits `DELIVERY_INTENT` with immutable envelope digest.
3. If receiver is CLI-Native attached, allow its ordinary Native queue consumption; do not duplicate a model request. If receiver is AppHost-submitted, use the existing **source-bound** `chat_input` with a deterministic derived operation ID and the latest verified saved-chat cursor. `chat_result` and saved-chat read establish result authority. Correlate Native queue item to app message before ACK.
4. If an app turn is still active, queue but do not interrupt the turn or interpret a new message as an ad hoc recovery command. Preserve ordering and only submit at a verified completed input boundary (or prove an explicit supported concurrent-input contract separately).
5. Observe actual saved message ID, stream terminal status, AppHost receipt, Native queue commit and parent ACK. Advance original message state transactionally; if an observation fails, return `UNKNOWN` with original IDs and reconcile, not a fresh submission.
6. On disconnection/restart, reconstruct original queue item, actor generation and source cursor, then decide **observe / resume original / fence**. No silent model/effort/account fallback; no duplicate shell/Native call.
7. For latest assistant reporting, use a **separate recipient binding** with host-authenticated permission. This is not implied by O1 having a saved ChatGPT project ID or by a user-visible hyperlink.

The adapter should be a small component on the **existing Provider/AppHost/Native boundary**, not a competing executor, model planner, global file writer or generic multi-agent database. Reuse native queue persistence and evidence ledger. Only add new API surface when existing `queue`, `chat_input`, `chat_result` and source-bound notifications cannot prove a required correlation.

## 6. Wakeup, continuity, failure and scope

A native `queue` ACK is intentionally separate from a model turn. The same saved ChatGPT conversation can presently be continued through `chat_input` with an account/branch cursor, but that does not prove that a queued Native event will *automatically* trigger it. Qualification must compare:
- `codex queue --resume` with a known local daemon and a Codex-native recipient, and with AppHost-submitted recipient. Inspect real receiving turn, not exit code.
- `codex_sessions queue` (enqueue only) followed by verified Native consumption, and the new AppHost receiver adapter if implemented.
- Saved `chat_input` after an O model response, after a disconnect, with an active O inference, and after account/project or branch drift.

Wake triggers should be event-driven (Native queue-arrival, worker completed/failed, approval, explicit deadline), **not periodic blind polling or scheduled ChatGPT automations**. A small bound host listener can wake or resume a *specific accepted actor generation*; it must not create a replacement job or bypass a denied Native action. If the host has no supported wakeup, state is `NEEDS_WAKEUP`, and progress cannot be claimed autonomous. Revisit daemon semantics only after a small proof of the actual existing CLI `--resume` behavior.

An unrelated `native_resource_busy`/opaque effect must be diagnosed by owning thread/resource; it does not prohibit independently safe model-only sessions or Termux shell paths. Stop changes from `stop_requested` to `stopped` only on settlement. Keep unrelated PIDs and the shared lease ledger intact. Never retry an uncertain process, Native effect, queued message or saved-Chat submission under a new ID. A compact status dashboard shows **last verified** observations, not continuous UI telemetry.

## 7. Budget contract: defaults, extend_budget and safe updates

### 7.1 User-directed target defaults (for future implementation)

| Dimension | Future default | Scope / semantics |
| --- | ---: | --- |
| `max_parallel` | **5** | Maximum concurrently admitted independent siblings/jobs under a logical parent; actual worker count remains dynamic 0-5, and file/Native writer exclusion still applies |
| `max_app_starts` | **256** | Upper bound on AppHost/Provider model-dispatch starts for the assigned work, **not** a target number of submissions |
| `max_native_calls` | **1024** | Upper bound on admitted Native tool calls, retaining stable call identity and recorded consumption |
| `max_elapsed_seconds` | **configurable / mutable** | User did not select a global numeric default; propose **43,200 s (12h)** only as a reviewed per-job *candidate* and keep configurable short/long duration. Distinguish idle lease, job duration and per-call timeout |

Default orchestration budget: `max_parallel=5`, `max_app_starts=256`, `max_native_calls=1024`; `max_elapsed_seconds` remains an explicit configurable duration, with 43,200 seconds only a proposed example.

These are **product target defaults**, not implemented defaults or permissions to spend 256 model calls/1024 tools. Finite caps, concurrent-owner safety, model access, approvals and physical resource controls continue to limit actual use. Never silently apply these values to ordinary non-orchestration requests or replace an explicit caller budget.

The current `Budget` struct requires all four fields and validates parallel `1..32`; `chat_input` accepts `max_app_starts <= 10,000` and `max_elapsed_seconds <= 86,400`. The proposed 5/256/1024 and optional 12h candidate fit those known boundaries, but actual Provider and Core runtime caps must be reverified for the final source SHA.

### 7.2 `extend_budget` must support **all four** dimensions

Present source `operator.rs:124-133` rejects any change to `max_parallel` and `max_app_starts`, and `operator.rs:4238-4275` rejects submitted-inference extension. This was a direct cause of the W0-W20 worker stall; it is **an explicit future code-change requirement**.

Preserve an externally compatible `extend_budget` API while implementing a documented budget-revision transaction with:
- Proposed full effective values for **parallel, App starts, Native calls and elapsed seconds**, `expected_budget_revision`, actor/work/generation and stable `call_id`.
- Safe **increase** of any dimension after checking the *current* parent/job ceiling and approval policy. Reductions of any dimension should also be supported by an explicit `adjust_budget` mode or versioned API, with no forced cancellation; reject a decrease below already consumed, reserved or in-flight usage.
- New revision counters, immutable operation history, actual consumed/reserved/remaining counters and effective admission threshold. Do not reset `started_at`, app starts, Native calls or deadline on controller claim, save/restart or extension.
- Compare-and-swap update under the Native operator work admission/ledger lock; prior actor generation and concurrent stale proposals must be fenced. Report actual accepted budget and revision in `codex_status`/events, not only the requested values.
- For `max_parallel`: revise the **shared parent sibling ceiling consistently**, not just one isolated child; enforce active + admitting sibling count <= effective ceiling and never preempt active writers. A decrease to below currently active jobs must become a safe `draining` state until the count decreases, not a kill. Clearly separate one worker's local tool parallelism from global worker concurrency.
- For `max_app_starts`: update the AppHost `maxDispatches` admission **on the same verified lane/generation** if Host supports it; otherwise apply at the next settled lane/reentry boundary with no new unobserved submission. The update must not create a second `chat_adopt` that replays prior user input. For submitted ChatGPT App sessions, reconcile saved `chat_input` and original AppHost ledger before granting further dispatches.
- For `max_native_calls`: admit only calls with original ID/receipt tracking, no reset or synthetic completion of unknown effects.
- For `max_elapsed_seconds`: recompute policy-relative deadline with the original `started_at`, coordinate the Host deadline and any Native lease, and distinguish **already expired but settled** from **live unsettled**; extension must not implicitly resume/fork an expired effect.
- Parent/child ceilings must be enforced after revision. If O's limits are insufficient, an authorized parent revision and descendant budget consistency check must happen first, without erasing already admitted work.
- If any part of Provider/Native/Host update fails, return `BUDGET_CHANGE_PENDING_RECONCILIATION` with original key and last committed revision, never a fake all-or-nothing success or a partly untracked increase. If a safe atomic commit across components is impossible, use journaled two-phase admission with `proposed`/confirmed effective limits and a monotonic *minimum safe limit* until both sides reconcile.

A proposed interface is conceptual, NOT a present MCP schema:

```json
{
  "action": "extend_budget",
  "work_id": "<existing-work>",
  "call_id": "<stable-revision-operation>",
  "expected_budget_revision": 7,
  "mode": "adjust",
  "budget": {
    "max_parallel": 5,
    "max_app_starts": 256,
    "max_native_calls": 1024,
    "max_elapsed_seconds": 43200
  }
}
```

Do not treat the numeric elapsed example as a user-approved global default. Backward-compatibility requirements: old recorded budgets remain unchanged until explicitly revised; missing fields on **new** orchestration creates get configured defaults; a legacy complete `budget` object retains its explicit limits. No retroactive renumbering or alteration of existing in-flight records.

### 7.3 Budget and source acceptance tests

- Fresh orchestrator and worker with omitted orchestration budget receive effective 5/256/1024 plus configured duration; ordinary non-orchestration works retain existing semantics. User explicit lower cap wins.
- Four independent single-field increases, combined increase, safe reduction, in-flight reduction/drain, unbounded/zero/invalid rejection, stale revision, wrong controller, mismatched parent, exceeded parent, and sibling 5-accept / 6th-wait.
- Exhausted `max_app_starts=1` can be revised **only through a truly settled, owner-verified transaction**, then one original next model turn is admitted without replacing the existing ChatGPT conversation or replaying prior input. If Host cannot do it, report a precise missing capability and retain saved-Chat continuation as a separate safe route.
- No budget extension resets consumed call count or lost-delivery receipt; after Core/AppHost restart, exact effective values and counters agree on both sides.
- `max_elapsed_seconds` extension across an expired, settled boundary is explicit and safe; a stalled opaque effect remains unresolved, not implicitly retried.
- Rate/permission/project/model guardrails continue to prevail under high defaults. Concurrency cannot override Native writer exclusivity, Stop fences or reviewer denials.

## 8. Focused implementation work packages (future only)

These are **design ownership boundaries**, not current worker assignments.

| Package | Producer / consumer and likely files | Required output |
| --- | --- | --- |
| D0: exact product-path baseline | Current `AGENTS`/`SPEC` annexes; `CURRENT`, `HANDOFF`; installed Core and Provider binary hashes; actual `codex queue --resume`, `codex_sessions queue`, saved `chat_input` | Evidence-backed applicability matrix; root causes and compatibility decision |
| D1: budget policy/admission | `providers/chatgpt/src/operator.rs` (Budget, parent, start, extend/admission/status); `app.rs`, `chat_input.rs`, `app_bridge.js` limits | Configured defaults, safe all-field revision, submitted/attached reconciliation; backward-compat and negative tests |
| D2: recipient registration + queue protocol | Existing `operator.rs` Native queue, `main.rs` Core selection, Core `queue` invocation, `notify_native_qualification.py` receiver ACK | Stable O and A recipient bindings, one message per ID, correct Core/ExecutionHome, queue/turn/parent ACK |
| D3: Linux AppHost receiver route | Existing `chat_input.rs`, `app_bridge.js`, `chats.rs`, bound chat/source cursor, Native queue observation | Correlated queue-to-saved-Chat input, same account/project/branch/model, real wakeup/turn proof, bounded unknown recovery |
| D4: model orchestration workflow | Model-led assignment contract/skill; existing `codex_sessions` create/start/chat_input; app project binding | True O/W sessions with dynamic 0-5 workers, mandatory return instructions, report/ACK loop and safe handoff |
| D5: qualification, release integration | `scripts/test_native_qualification_notice.py`, Provider operator and ChatGPT continuity tests, integration CI, Termux device acceptance | Complete evidence matrix below, exact source SHA and required negative proofs, no regression of 0.167.2 behavior |

Before editing, resolve file writer ownership and whether these paths belong to upstream Native patch, TUI patch, Provider or Bridge. Do not patch a generated or downstream copy that will be overwritten by the release composer. Update all producers and consumers in one cohesive change; reuse verified artifacts instead of rerunning long CI without changed proof inputs.

## 9. Acceptance matrix: design -> implementation -> actual product path

| ID | Scenario and method | Exact pass evidence |
| --- | --- | --- |
| A01 | Create assistant-linked O and 1-5 workers from **ChatGPT entry**, never assistant-directed source writing; dynamically choose 0 when no ready work | Real actor IDs, assignment DAG, O/W saved Chat identities, account/project and source receipt; no invented subagents |
| A02 | O creates W with legal Native parent/controller, required recipient binding and budget 5/256/1024 | Core parent relation verified, no missing mandatory report fields, correct effective budgets and counters |
| A03 | W1 and W2 complete different owned tasks and `codex queue` RESULT to **O's Native thread** using the exact receiver environment | Two unique queue IDs, correct Native/turn ingestion and O's independent ACK and evidence evaluation; no assistant relay |
| A04 | W ERROR/QUESTION/PROGRESS and later RESULT, O responds REVISE/ACK and **continues work** after worker response | Saved model messages and parent queue receipts for each, new real assignments or final acceptance |
| A05 | O sends FINAL_REPORT/ERROR to **latest A**, A acknowledges, earlier A does not receive mutating deliverables | Verified authorized recipient readback, authenticated account/project, visible receipt, no wrong-thread injection |
| A06 | A is Android/web Chat without Native thread, while O/W are Linux AppHost-backed | Actual app-to-assistant authorized delivery OR explicit blocked capability (not fictional Native thread) |
| A07 | Native `queue` in CLI thread with `--resume` | `QUEUE_ACCEPTED`, separate new-turn `RECEIVED` and valid ACK; warn-only resume failure handled |
| A08 | Native `queue` -> same Linux saved O Chat with candidate adapter | Exact envelope and `clientUserMessageId` correlate to one `chat_input` and one saved reply, no duplicate model request |
| A09 | Source cursor stale/changed branch, account/project mismatch, token change, unsupported Chat origin | No new App inference, explanatory denial, original message safely pending |
| A10 | Submitted / attached budget amendments across all 4 values, high defaults, sibling 5/6 admission | Effective revision, consumed counters, actual Host and Native limits and no unauthorized effects |
| A11 | 2-5 worker capacity under separate homes; write conflict and unrelated Busy while model-only lane ready | Independent safe work progresses; only conflicting writer blocked, no shared-claims deletion |
| A12 | Concurrent enqueue / duplicate report ID / lost queue ACK / MCP uninitialized / HTTP 413 | Exact original operation history and dedup, no second externally executed effect |
| A13 | Termux/Provider/Core/AppHost restart with queued RESULT and a stopped O turn | Same logical actor, same original saved Chat if supported, delivered once or explicitly unknown with preserved receipt |
| A14 | Native `stop_requested` but Stop API errors, real unrelated PID alive, worker/source test unknown | No false stopped/completed or unrelated kill, other eligible tasks continue |
| A15 | O or W context exhaustion and successor Chat generation | New generation fences old, original queue and tasks retained, no repeated worker/effect; dashboard shows both generations |
| A16 | Model selection/effort, CLI auth boundary, Provider app login, project membership and saved Chat link | Exact catalog/model/effort entitlement and independent auth verified; no forced login/fallback or credential leak |
| A17 | Existing Full/TUI/Provider/Manager CI and exact device smoke on new source | Source SHA-bound logs, genuine exits, no weakened assertions; unchanged paths not repeatedly retested |
| A18 | O sends progress/failure/final while ChatGPT assistant is not currently active | The actual recipient wakeup/notification and subsequent assistant ACK; if missing, `NEEDS_WAKEUP`, not acceptance |

**Evidence tiers:** static source code and unit tests < synthetic integration < real Termux Core queue/CLI < real Linux ChatGPT saved project/branch/model inference < Android/web assistant visible delivery < signed/released installed runtime. A lower tier cannot certify a higher one.

Negative checks are mandatory: missing `parent_controller`; wrong `thread_id`/Core `CODEX_HOME`; absent report recipient; stale actor generation; replayed report ID; unauthorized budget escalation/decrease; wrong user/account/project; denied reviewer action; socket path over 107 encoded bytes; Termux shared-storage redirection; pending opaque effect; and concurrent writer collision.

Timing/cost reporting: count actual model starts and replies, host app starts separately, Native admissions, worker/parent wait, queue-to-turn latency, app source read/submit/ack latency, elapsed/CPU/RSS, file size, original compile/test exit and cost. No claims about invisible billing. Preserve original logs and hashes and publish a concise exact-SHA acceptance manifest. Use short owner-private socket/TMPDIR; verify every UNIX socket full path is <= 107 encoded bytes.

## 10. Future implementation order and decision checkpoints

1. **Research-only D0 first**: reproduce with actual installed and current development routes whether CLI `--resume` triggers O's saved Chat, whether `codex_sessions queue` can feed an AppHost-backed turn and whether A has a real authenticated return address. Use small disposable O/W and one harmless RESULT before proposing new APIs.
2. **Minimal interop prototype C**: only if D0 identifies an addressable queue item + saved-Chat receipt binding. Prove a single W -> O report **and** O -> A return, with wrong-recipient rejection, before extending the design. No arbitrary background model polling.
3. **Budget D1 independently qualified**: implement default values and four-field safe revision plus Host/sibling admission parity, preserving old records and counters.
4. **Scale and harden D2-D4**: reporter instructions mandatory at worker creation, 0-5 dynamic workers, ACK/evaluation/wakeup, context replacement, no duplicate effect under disconnect.
5. **Finish D5**: exact-source Core/Provider/TUI/GitHub and real Linux AppHost, Android assistant recipient and device acceptance. Choose C only if its acceptance genuinely passes; otherwise select proven A scope or retain a clearly blocked design candidate.
6. Update the owning product specification **only after the user reviews an intentional product contract**. Record source/current results separately; this proposal is not that approval.

The design must be elegant in the sense of **one Native queue, one Native effect authority, one AppHost source conversation and one logical actor binding**, with stable receipts at their boundaries. Avoid implementing two parallel dispatchers, inventing new model aliases, or forcing every Linux ChatGPT worker into an unnecessary terminal UI.

## 11. Release separation: what must happen now

**Immediate priority remains Codex for Termux 0.167.2 integration and safe runtime replacement.** This document is a future work item, not a new gate for 0.167.2 and not authorization to change operating installation. Continue the already approved W0-W20 final source consolidation, G1 receipt reconciliation, G2/G3 verified commits, final CI/unsigned Full qualification and actual target product-path checks. Deployment/signing/activation must follow the user's separate explicit authorization and exact installed/rollback checks.

Do NOT insert orchestration D1-D5 code changes into the frozen 0.167.2 release candidate unless independently approved as a necessary release blocker repair. Do not insist on re-running hours-long CI for documentation-only changes. After 0.167.2 is safely accepted, record exact source/device/Provider versions and rebase this proposal's evidence matrix to the then-current code before implementing.

### Open questions requiring *real* tests, not speculation

- Can `codex queue --resume` target a Linux AppHost-saved O thread without an interactive TUI, or is it a local Codex daemon/TUI-only path?
- Can the current Native queue item be correlated with app saved input/turn using existing identity/cursor metadata without a new adapter endpoint? What does the app actually do when the parent conversation is idle or actively streaming?
- How can a user-initiated **Android/web ChatGPT conversation** receive O's final report with positive ACK? Can the platform expose a supported authorized receiver binding, or must the result be retained in a separately verified same-account/project destination until the assistant next attaches?
- Can AppHost update maxDispatches/deadline in place on submitted inference without resetting the lane or duplicating a user message? Which changes need an explicitly settled transition?
- What is the exact parent-concurrency interpretation across child models, Native writers and one shared AppHost process? Verify five safe siblings and refusal of the sixth.
- How should queue-to-saved-input intents be fenced when the orchestrator saved Chat is replaced during delivery, especially if a prior dispatch is still pending?
- Can the existing source-bound notification ACK be generalized to O/W and A without new transcript stores or periodic models?
- What exact changes to `docs/spec/bridge/` are necessary after product review, with reproducible tests and no breakage of existing TUI/CLI/legacy/native routes?
