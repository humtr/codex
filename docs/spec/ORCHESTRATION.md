# Durable Multi-Role Orchestration Contract (`@TermuxOrch` / `mcp/termux-orch`)

**Status:** Normative annex of [SPEC.md](../../SPEC.md) — **Stage 2 Slice 7 (7A and 7B)**.
For active implemented code consult [docs/ARCHITECTURE.md](../ARCHITECTURE.md);
for verified live receipts consult [CURRENT.md](../../CURRENT.md).
The completed rollout is recorded in [Stage 2 milestones](../history/STAGE2_MILESTONES.md).

This annex defines the optional, decoupled Stage 2 durable orchestration layer
that ensures **both simple single-task chats and complex multi-worker, multi-hour
workflows** survive mobile disconnects, context exhaustion, new chat sessions
(`F1 -> Fn`), and Termux restarts without introducing a second executor or
coupling into `$PREFIX/bin/codex` (`crates/wrapper`) or the three stateless
core MCP bridges (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`).

---

## 1. Role Taxonomy (`F · L · W · V · R`) and Authority

| Code | Name | Type | Responsibility & Authority |
| --- | --- | --- | --- |
| **`F`** | **Front** (`F1..Fn`) | Replaceable ChatGPT chat | User-facing interactive desk, dashboard, scope/approval relay, and report recipient. Owns **no** work lifetime or code execution. In pure chat (Tier 0), `F` operates alone with zero orchestration overhead. |
| **`L`** | **Lead** (`L1..Ln`) | Optional coordinator (`codex` thread or `chatgpt` chat) | Decomposes goals into scoped subtasks, dynamically scales `W`/`V` count (`0 -> N -> M`), reviews evidence, maintains `checkpoint.md`, and decides next rounds vs completion proposal. Spawned only for Tier 2/3 work. |
| **`W`** | **Worker** (`W1..Wn`) | Scoped executor (`codex` / `chatgpt` / `shell`) | Performs bounded implementation, research, or command work within explicit file/branch/profile ownership and returns verifiable evidence. Never writes across another active worker's owned scope. |
| **`V`** | **Verifier** (`V1..Vn`) | On-demand independent reviewer | Spawned with fresh context and non-mutating/verification-only scope to check actual boundary evidence (tests, disk state, spec invariants) independently of `W` and `L`. |
| **`R`** | **Runtime** (`termux-orch relay`) | Deterministic non-LLM Rust daemon | Sole authority over durable `Work Ledger`, `Typed Inbox` (`seq`/`consumed_seq`), declarative `Wait-Set` triggers, generation fencing (`L1->L2`, `F1->Fn`), budget/Circuit Breaker limits, and wake-up/notification delivery. Models may request transitions but can never bypass `R`'s hard guards. |

**Single-owner rule (zero second executor):** `R` never runs model inference or
shell tasks directly. It observes and triggers only existing authoritative
owners:
- Codex `L`/`W`/`V` → official `codex app-server daemon` via `crates/codex-daemon-client`
- ChatGPT `F`/`L`/`W`/`V` → multi-lane `chatgpt-provider` loopback `/control/*`
- Long detached Shell `W` (Slice 7B) → verified `(pid, start_ticks)` + exit receipt file

---

## 2. Four-Tier Progressive Durability (Simple Chats & Complex Orchestration)

One unified `Work` model covers every workflow without forcing ceremony on
ordinary chats. Entry tools describe when a **natural-language** request should
remain in the current session and when to delegate and release Front. After
registration, the runtime enforces dependencies, ownership and completion.
Stable plans can use the runtime's durable step queue with no extra Lead turn.
Managed writers in a shared working tree are serialized. Parallel writers use
explicit isolated worktrees and declared write paths, with actual changes
reviewed for scope and integration conflicts. Independent review requires a
structured verdict for the current work subject. An authenticated Front lookup
can update the delivery target with the lookup. Unconfirmed sends are reconciled
against their original target. New Works use context-driven rollover by default.
Direct ChatGPT recovery uses a 32 KiB checkpoint budget and at most one fresh-chat
attempt for the same size rejection. Historical explicit turn thresholds retain
their meaning:

1. **Tier 0 — Pure Chat & Single-Turn MCP (zero overhead):** Ordinary
   interactive ChatGPT chat or single-turn `@TermuxShell` / `@TermuxChatGPT` /
   `@TermuxCodex` execution that finishes within one turn. No `Work` created,
   no `R` involvement.
2. **Tier 1 — Direct Durable Task (`tier = "direct"`, no `L`):** A normal chat
   (`F1`) dispatches one long-running Codex task, fixed shell plan (`steps`), or
   build/test and registers a lightweight `Work` directly with `R`. If mobile
   disconnects, the task takes 20 minutes, or the user opens a new chat (`F2`)
   tomorrow, `R` preserves the task, receipts, and result delivery to `F1`/`F2`
   without spawning a `Lead`.
3. **Tier 2 — In-Place Escalation (`promote`):** A Tier 1 `direct` `Work` can
   be promoted in place into Tier 3 (`orchestrated`) by attaching `L1`, carrying
   over running workers, receipts, and history without restarting or duplicating
   in-flight effects.
   New Works use `direct` when no tier or Lead configuration is supplied.
4. **Tier 3 — Full Autonomous Orchestration (`tier = "orchestrated"`):** `F`
   creates a `Work` and spawns `L1` (`lead_kind = "codex" | "chatgpt"`, chosen
   by workload: `codex` using its active configured model/profile for multi-hour
   engineering loops with native compaction; `chatgpt` using MCP single-turn
   connectors for project-context/research/multi-tool coordination). `L`
   dynamically spawns parallel `W`/`V` sessions, loops across multiple rounds
   via `Wait-Set` wake-ups, and reports milestones, blockers, and verified
   completion back to the latest bound `Front` (`F1..Fn`).

---

## 3. Project Organization and Consistent Session Naming

- **ChatGPT Project Scoping.** Use `project_scope` for a repository or branch
  key, such as `humtr/codex` or `humtr/codex@feature`. The account and scope
  resolve to one durable project binding reused across Works. On first use,
  `project_name` overrides the display name, which defaults to the scope.
  Runtime looks for one exact name and creates a project if the name is absent.
  An explicit `project_id` selects an existing project and resolves ambiguous
  names. A call without a project ID or scope leaves the Work unscoped. Existing
  Work bindings are preserved. An uncertain creation is reconciled by lookup
  before any further create is allowed.
  An explicit name that differs from the saved scope name fails before a
  project mutation. A pinned project ID is verified by direct lookup. A name
  lookup without an ID must finish within the bounded discovery budget or
  require the caller to supply the ID.
- **Deterministic Session Titles:** Whenever a ChatGPT chat or Codex thread is
  created or bound for a role, `R` automatically applies a uniform, human-readable
  title via `POST /control/chats/{id}/rename` (ChatGPT) or `thread/name/set`
  (Codex):
  - Front: `[F<gen>] <work-title>` (e.g. `[F1] Slice 7A 구현`, `[F2] Slice 7A 구현`)
  - Lead: `[L<gen>] <work-title>` (e.g. `[L1] Slice 7A 구현`)
  - Worker: `[W<idx>-<slug>] <subtask>` (e.g. `[W1-ledger] Work 원장 구현`)
  - Verifier: `[V<idx>-<slug>] <scope>` (e.g. `[V1-audit] Diff 및 라이브 E2E 검증`)

  Optional `session_prefixes` on `create` or `update_scope` replaces these
  defaults for subsequent names. Keys are `front`, `lead`, `worker`, and
  `verifier`. `{n}` expands to the generation or participant index, and
  `{slug}` expands to the normalized participant slug. An empty string removes
  the prefix. Use `rename_session` for a session that already exists.

---

## 4. Core Primitives: `Work Ledger`, `Typed Inbox`, and `Wait-Set`

State root: `~/.local/state/codex/orchestration/jobs/<work_id>/` (`0700`,
`O_NOFOLLOW`, atomic rename + `fsync` on file and parent directory):

1. **`Work Ledger` (`work.json` + `checkpoint.md`):**
   - `work.json` holds `work_id`, `title`, `tier` (`direct | orchestrated`),
     `goal`, `completion_criteria[]`, `state` (`running | waiting | waiting_user | completed | failed | failed_verification | cancelled`),
     pinned `(account_id, project_id)`, `delivery_mode` (`inject_and_notify`
     default | `notify_only`), `front: { latest_chat_id, generation, history[] }`,
     `lead: Option<{ kind, ref_id, profile?, generation, history[] }>`,
     `participants[]` (`role: worker|verifier`, `backend: codex|chatgpt|shell`,
     `ref_id`, `slug`, `owner_scope`, `call_id`, `state`, `result_summary`),
     `consumed_seq`, and `budget` (`max_wakeups`, `wakeups_used`,
     `max_wall_secs`, `max_concurrent_workers`, `min_cooldown_secs`).
   - The reader validates `termux-orch-work-v1` and the Work ID in the record.
     It preserves unknown fields on the Work, participant and wait-set
     records across updates. Strict nested records, such as step definitions
     and session prefixes, reject unrecognized fields. Work listing and
     relay status surface unreadable Work IDs as errors.
   - `checkpoint.md` holds the concise handoff summary updated by `L` before
     ending a round or rolling over, giving `L2` and `Fn` one-read recovery.
2. **`Typed Inbox` (`events.jsonl`):**
   - Append-only immutable events deduplicated by `event_id` with monotonic
     `seq`: `worker.completed`, `worker.failed`, `worker.stalled`,
     `verifier.passed`, `verifier.rejected`, `user.message`,
     `user.scope_updated`, `front.rebound`, `timer.deadline`,
     `budget.exhausted`.
   - `R` advances `consumed_seq` only after confirmed wake-up dispatch receipt;
     across crashes, restarts, or `L1 -> L2` handoffs, unconsumed events
     (`seq > consumed_seq`) are preserved without loss or duplicate delivery.
   - A Wait-Set wake-up includes a bounded oldest-first prefix. Its receipt
     advances the cursor through the last included event. A notification-only
     wake leaves the Inbox cursor unchanged. An explicit `after_seq` Inbox
     query returns the next page in sequence order.
3. **Declarative `Wait-Set`:**
   - Unifies fan-in barriers, fail-fast checks, user interrupts, and stall
     watchdogs into one declarative rule registered before `L` (or `F` in Tier 1)
     ends its turn:
     - `wake_target`: `lead | front`
     - `all_of`: participant IDs that must all settle before waking (Fan-in Barrier)
     - `any_of`: participant IDs where the first settlement/failure wakes immediately
     - Both lists must use registered `participant_id` values, not slugs. An unknown
       ID is rejected as `unknown_participant_id` with the known IDs before arming.
     - `on_user_event`: wake immediately when `Fn` submits a scope update/message
     - `deadline_secs`: stall watchdog timer waking target if no condition fires in time
   - A direct Work can set `then_verify_shell` with an `all_of` list. The relay
     starts the fixed shell check after the named prerequisites succeed, using
     the Work ledger and shell identity receipt. It reports success only
     after the check and all required verification settle. If a prerequisite
     fails, it preserves that result and does not start the check.

---

## 5. Mobile `F1 -> Fn` Continuation, Delivery Safety, and Circuit Breakers

- **Seamless `Fn` Re-attachment:** When a user opens a new chat (`Fn`) on
  Android/Web ChatGPT and references prior/ongoing work, `Fn` calls
  `orch_works(action="list")` to discover active/recent `Work`s (Tier 1 & Tier 3)
  without recreating workers. The user-facing `orch_works(action="read")` can
  include `rebind_front_chat_id` or `rebind_front_marker` and a `call_id`.
  The same account and chat checks apply to explicit
  `orch_work(action="rebind_front")`. MCP stdio has no authenticated chat
  identity field. Pass an address from trusted session context or a unique
  user-authored message marker. `R` confirms the account and the exact chat
  via `chatgpt-provider` `GET /control/chats/{id}`,
  atomically updates `front.latest_chat_id = Fn`, names the chat using the Work policy,
  and records `front.rebound`. A missing account pin, unavailable identity,
  or unreadable target chat prevents rebinding. Moving the Front alone does
  not trigger a user-event Wait-Set. A previous delivery attempt is reconciled
  at its original destination. Front delivery follows the destination chat's
  project. The Work's project continues to scope its Lead and workers.
  Marker lookup reads at most two candidates updated in the last ten minutes.
  A caller can supply the exact Front chat ID when the marker is outside that window.
- **Safe `inject_and_notify` Delivery:** Before calling `chatgpt-provider`
  `POST /control/send(chat_id=...)` on `F/Fn` or ChatGPT `L`, `R` checks both
  local `activeTurns` and server `/conversation/{id}/stream_status` via
  `POST /control/observe`. If the target chat is currently streaming (e.g. user
  is actively typing on Android), `R` records `PENDING_DELIVERY` and retries
  with exponential backoff rather than colliding. Every milestone, blocker, or
  final delivery also emits a `termux-notification` (and `notify_only` skips
  chat injection entirely when requested). Before dispatch to ChatGPT or
  Codex, `R` persists the destination and a delivery marker. After an
  uncertain response, `R` reads back the original destination for matching
  owner evidence. A missing marker leaves the dispatch pending. A failed
  notification in `notify_only` mode remains pending for retry.
- **Codex participant settlement:** A newly dispatched Codex participant
  records the returned turn ID. Settlement uses that turn's terminal state
  and result. Older attached records can observe the latest turn in their
  bound thread.
- **Model dispatch:** Register the participant or pending Lead and a durable
  request ID before sending its first model turn. Codex records the new
  thread UUID before `turn/start`. When the relay takes the Work lock and a
  pending Codex intent still has no thread UUID, it records a failed dispatch
  and releases any worker reservation. ChatGPT dispatch uses a pinned account and
  request lane. An uncertain dispatch stays pending until the original
  owner confirms the turn. Repeating the registration does not resend it.
- **Fixed step plans:** `orch_work(create)` accepts a `steps` array of shell
  commands with `cwd`, unique `step_id`, and optional `depends_on`,
  `write_paths`, `isolation`, `on_failure`, and `auto_verify`. Dependencies
  refer to preceding entries. A final `auto_verify` command runs after the
  work steps and supplies its shell receipt. The relay reserves the
  participant before dispatch and can resume across restarts without a
  Lead turn. A failed prerequisite blocks its dependents.
- **Managed write order:** `shared_serial` reserves a canonical working
  tree across managed Works. Conflicting scheduled steps wait. An
  interactive registration gets `worker_workspace_busy` and can be
  resubmitted after the owner settles. `worktree` gives a new shell or
  Codex worker a dedicated Git worktree and branch with `write_paths`.
  Runtime checks tracked and untracked changes and overlapping paths before
  accepting the result. Completion also checks the declared scope of retained
  worktrees. Changes stay in the worktree for Lead to integrate.
  `readonly` is a declaration. Separate paths provide repository isolation
  under the same Termux UID and carry the owner's existing OS permissions.
- **Cancellation:** A cancelling Work blocks new starts. Runtime requests
  interruption of its identified Codex turn or ChatGPT lane. The shell writer
  sends `SIGTERM` and escalates to `SIGKILL` for its command's process group.
  It waits for live members of its session to exit before it writes the
  receipt. Cancellation stays pending while an owner is unconfirmed. Attached
  sessions retain their original owner.
- **Verification and completion:** A registered verifier supplies a result
  separately from the terminal status of its turn. A model verifier returns
  the assigned challenge, a `pass`, `fail`, or `inconclusive` verdict, and
  `evidence_refs`, and `blocking_issues`. The relay accepts a strict JSON object
  or, for a ChatGPT verifier, that object inside exactly one literal
  `<escape>...</escape>` presentation envelope. It does not extract JSON from
  prose, execute presentation markup, or relax challenge, evidence, subject,
  or terminal-state validation. A shell verifier uses the owner exit
  receipt. `verification_policy=independent_review` starts a fresh Codex
  verifier after a completed plan or an eligible completion request that
  identifies the review workspace. Confirmed failure records
  `failed_verification`. An inconclusive result returns `waiting_user`. The
  Work binds verification to the goal, criteria, and worker results it was
  registered to check. New or changed worker results and scope require fresh
  verification. `orch_report(complete)` requires all participants to be
  settled and the latest verifier for each registered check to have passed
  for the current subject. Older completed Works remain readable. A missing
  or unverified legacy verdict in an active Work requires a fresh check.
- **Hard Circuit Breakers, Rate-Limit Discipline & Admission Control:** `R`
  enforces `max_wakeups` (default `20`), `max_wall_secs`,
  `max_concurrent_workers`, a per-account ChatGPT concurrency cap (default `2`
  active ChatGPT workers, configurable via `ORCH_MAX_CONCURRENT_CHATGPT`),
  minimum wake-up cooldown, and `event_id` idempotency. On budget exhaustion,
  `R` transitions the `Work` to `waiting_user` and notifies `F/Fn` instead of
  looping indefinitely.
  - **Local vs. remote polling separation:** Routine relay ticks use Shell
    exit receipts, Codex UDS state, and the Provider's in-memory ChatGPT turns.
    Participants without a tracked turn use a known-chat remote observation at
    most once per minute. Automatic Lead rollover reacts to tracked context-limit
    errors. The optional turn-count trigger reads history at most once per
    minute per Work by default. Explicit delivery and reconciliation use the
    original destination and the Provider admission limits.
  - **Global `429` cooldown gate & 3-state delivery:** `backend_http_429`
    (with `retry_after`, default `60s`) pauses remote ChatGPT calls across all
    Works until cooldown expiry and is classified as `RejectedBeforeSend`
    (clearing unsubmitted `dispatch` instead of sticking in
    `delivery_receipt_unconfirmed`). Marker readback propagates provider errors
    as errors rather than `not_found`, and bounded ambiguous delivery
    reconciliation escalates to `waiting_user` + `termux-notification`.
  - **Dispatch preflight & structured telemetry:** `orch_dispatch` fails fast
    before creating worktrees when ChatGPT is in `429` cooldown or at the
    ChatGPT concurrency cap. `termux-orch relay` emits concise secret-free
    operational logs and exposes cooldown / rate-limit telemetry on `/healthz`.
    Both `/healthz` and `orch_call(relay_status)` expose `local_observe_calls`
    (attempts to `/control/observe` with `local_only=true`) separately from
    `remote_chatgpt_calls` (all other Provider control attempts, not a count of
    actual upstream HTTP requests). Both count after cooldown admission and
    Provider resolution, before transport, including failed transport attempts.

---

## 6. Independent Packaging (`@TermuxOrch`) & Phased Delivery (Slice 7A / 7B)

- **Independent workspace:** `mcp/termux-orch` (own `[workspace]`, reusing
  `crates/mcp-kit` + `crates/codex-daemon-client` + loopback `chatgpt-provider`;
  zero `crates/wrapper` or root `Cargo.lock` dependency).
- **Single binary, two explicit subcommands:**
  - `termux-orch relay` — opt-in `runit` daemon (`termux-orch-relay`, loopback
    `/healthz` readiness endpoint + `termux-wake-lock` guidance).
  - `termux-orch mcp` — opt-in `runit` tunnel-wrapped stdio MCP (`termux-orch-tunnel`,
    registered programmatically via `POST /control/connectors` as `@TermuxOrch`).
- **Asynchronous owner operations.** The MCP process validates obvious local
  argument errors before enqueuing and durably accepts Work creation, promotion,
  Front rebinding, model dispatch and session mutations that require an owner
  call. After waking a live relay, MCP may short-wait briefly (up to 3.5s,
  strictly clamped) and return the settled receipt directly when complete;
  otherwise `state=queued` or `processing` includes an `operation_id`.
  `orch_call(operation_status)` reads that operation's local receipt.
  `processing` means relay claimed the request. Settled operation receipts are
  amortized-pruned every 32 completions to the newest 512 full files while
  retaining permanent sharded fail-closed tombstones
  (`requests/tombstones/<2-hex-prefix>.json`, never evicted; shard size
  overflow fails closed before unlinking full receipts;
  `orch_operation_receipt_expired` / payload conflict) so pruned `call_id`s
  never silently re-execute. A receipt can include a
  Work-level `pending` result for owner reconciliation. Retrying an identical
  call ID retrieves the original receipt. A conflicting payload is rejected.
  Follow-up dispatch, Wait-Set registration and reporting use the authoritative
  IDs from the settled operation. Relay runs the owner calls under one owner
  gate. It reports an interrupted execution as `delivery_unknown` and
  reconciles any persisted Work intent without replaying an uncertain mutation.
  Rename and manual rollover extension arguments include a `call_id`. Local
  reports persist delivery and can request an immediate relay wake.
  `reconcile_delivery` requests a wake and returns its acceptance separately
  from delivery. Codex model launches check the selected daemon and its
  authentication requirement before submitting a turn.
- **Provider admission and wake.** The AppHost page driver budgets reads
  (five burst, 0.5/s refill, 25 per rolling minute) and sends/writes (three
  burst, 0.2/s refill, ten per rolling minute). `/me` and stop are exempt.
  When a token shortage is brief (`<= 1500ms`), the rolling-minute cap has not
  been reached, and concurrent waiters are below four, the driver paces once
  before re-checking admission; otherwise admission failures return
  `local_rate_limit` with `retry_after` before the guarded request reaches
  ChatGPT. Concurrent equivalent conversation reads share one in-progress
  request and successful account checks are cached for 30 seconds. Provider
  sends a coalesced authenticated loopback wake on tracked Orch turn
  settlement. The relay continues periodic recovery if a wake is lost.
- **Static 6-tool core + disjoint extension slot** (uniform immutable MCP
  annotations `readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false`):
  1. `orch_works` (`list | read | checkpoint | inbox`)
  2. `orch_work` (`create | promote | rebind_front | update_scope | cancel`)
  3. `orch_dispatch` (`register_worker | register_verifier | wait_set | wait_clear`)
  4. `orch_report` (`milestone | need_user | complete | fail`)
  5. `orch_catalog`
  6. `orch_call` (`lead_rollover`, `budget_extend`, `reconcile_delivery`, `relay_status`, `rename_session`, `operation_status`)
- **Phased delivery split:**
  - **Slice 7A:** `chatgpt-provider` server `stream_status` observation +
    `mcp/termux-orch` (`Work Ledger`, `Typed Inbox`, `Wait-Set`, Tier 1–3,
    `F1->Fn` rebind, `Verifier` role & naming, project/title automation,
    `inject_and_notify` + `termux-notification`, Circuit Breaker, CI, `runit`
    supervision, `@TermuxOrch` connector registration, and live E2E).
  - **Slice 7B:** Automatic ChatGPT `L1 -> L2` context-limit rollover with
    generation fencing, detached long-running (`>10 min`) Shell worker
    `(pid, start_ticks)` watcher, and instant `codex-ext` `Stop` hook push
    into `termux-orch relay`.

## 7. Lead rollover, detached shell receipts, and wake pushes

- A ChatGPT Lead rolls over after a context-limit signal or a configured
  user-turn count. Set `lead_rollover_turns` on `orch_work(create)`; the Work
  stores it in the budget. New Works default to zero, which disables the
  turn-count trigger. Existing Works retain their persisted threshold and
  older records without the field retain their legacy default. The runtime
  budgets its generated checkpoint and replay payloads at 32 KiB. For
  `input_too_large` on the active Lead it tries one bounded handoff in a
  fresh chat. A further size rejection records `payload_too_large` and
  pauses for the user. Rollover
  bootstrap and replay turns do not count toward the next
  generation's threshold.
- `orch_call(lead_rollover)` accepts `work_id`, optional `reason`, and optional
  `checkpoint_md`. The relay saves the checkpoint and a durable intent before
  requesting the replacement. The Provider checks the live account against
  the pinned account before dispatch, passes that identity to CompletionClient,
  and sends within the Work's project scope, including unscoped chats. The
  relay confirms the replacement chat and its creation marker, applies the
  `[L<n>]` title, then persists the new
  generation, history and `lead.rolled_over` Inbox event. Manual rollover can
  return `pending` while the relay reconciles that transaction. A missing
  replacement ID is recovered only from the original Provider lane. If that
  lane cannot identify the chat, the original intent is retained and the Work
  pauses for user reconciliation. The relay does not scan chat history or
  resubmit the creation.
- Callers can fence a Work mutation with `expected_lead_generation`. Where
  accepted as a fence, `lead_ref_id` must also match the current Lead. A
  mismatch returns `stale_lead_generation`. Omission preserves legacy callers.
- Replay starts at the saved `consumed_seq`, in ascending batches. The relay
  confirms each exact user-message marker in the replacement chat before
  advancing the cursor. An uncertain write remains pending for reconciliation
  against its original marker. The existing Wait-Set stays in the ledger.
- A Shell Worker accepts one of `command` or `prompt` with an optional
  `cwd`, or a `ref_id` identifying a verified receipt in the same Work's
  shell directory. The detached receipt writer publishes `<id>.meta.json`
  with its PID and Linux start ticks, an 8 KiB combined `<id>.output.log` tail,
  and an atomic `<id>.exit.json`. The relay settles against a matching exit
  receipt or against a vanished process identity after rechecking for exit.
- The relay accepts loopback `POST /wake` with the bearer token in the
  adjacent owner-only `wake.token` file. An accepted request schedules an
  immediate tick and increments `wake_pushes` in `/healthz`. Bursts can share
  a tick. The optional `codex-ext notify emit Stop` push uses the canonical
  relay service directory. An unavailable relay does not affect notification
  delivery.
