# Durable Multi-Role Orchestration Contract (`@TermuxOrch` / `mcp/termux-orch`)

**Status:** Normative annex of [SPEC.md](../../SPEC.md) — **Stage 2 Slice 7 (7A and 7B)**.
For active implemented code consult [docs/ARCHITECTURE.md](../ARCHITECTURE.md);
for verified live receipts consult [CURRENT.md](../../CURRENT.md);
for phased execution checklists consult [HANDOFF.md](../../HANDOFF.md) and [docs/ROADMAP.md](../ROADMAP.md).

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
ordinary chats:

1. **Tier 0 — Pure Chat (zero overhead):** Ordinary interactive ChatGPT chat.
   No `Work` created, no `R` involvement.
2. **Tier 1 — Direct Durable Task (`tier = "direct"`, no `L`):** A normal chat
   (`F1`) dispatches one long-running Codex task or build/test and registers a
   lightweight `Work` directly with `R`. If mobile disconnects, the task takes
   20 minutes, or the user opens a new chat (`F2`) tomorrow, `R` preserves the
   task, receipts, and result delivery to `F1`/`F2` without spawning a `Lead`.
3. **Tier 2 — In-Place Escalation (`promote`):** A Tier 1 `direct` `Work` can
   be promoted in place into Tier 3 (`orchestrated`) by attaching `L1`, carrying
   over running workers, receipts, and history without restarting or duplicating
   in-flight effects.
4. **Tier 3 — Full Autonomous Orchestration (`tier = "orchestrated"`):** `F`
   creates a `Work` and spawns `L1` (`lead_kind = "codex" | "chatgpt"`, chosen
   by workload: `codex` with `gpt-6-sol-wm xhigh` for multi-hour engineering
   loops with native compaction; `chatgpt` for project-context/research/multi-tool
   coordination). `L` dynamically spawns parallel `W`/`V` sessions, loops across
   multiple rounds via `Wait-Set` wake-ups, and reports milestones, blockers,
   and verified completion back to the latest bound `Front` (`F1..Fn`).

---

## 3. Project Organization and Consistent Session Naming

- **ChatGPT Project Scoping:** Every `Work` pins `(account_id, project_id)` on
  creation and never silently switches accounts or projects on fallback. If the
  caller supplies an existing `project_id`, `R` reuses it; otherwise `R` creates
  a dedicated project named `[Work] <topic>` via `chatgpt-provider`
  `POST /control/projects`. All ChatGPT sessions (`F`, `L`, `W`, `V`) belonging
  to that `Work` accumulate inside that project.
- **Deterministic Session Titles:** Whenever a ChatGPT chat or Codex thread is
  created or bound for a role, `R` automatically applies a uniform, human-readable
  title via `POST /control/chats/{id}/rename` (ChatGPT) or `thread/name/set`
  (Codex):
  - Front: `[F<gen>] <work-title>` (e.g. `[F1] Slice 7A 구현`, `[F2] Slice 7A 구현`)
  - Lead: `[L<gen>] <work-title>` (e.g. `[L1] Slice 7A 구현`)
  - Worker: `[W<idx>-<slug>] <subtask>` (e.g. `[W1-ledger] Work 원장 구현`)
  - Verifier: `[V<idx>-<slug>] <scope>` (e.g. `[V1-audit] Diff 및 라이브 E2E 검증`)

---

## 4. Core Primitives: `Work Ledger`, `Typed Inbox`, and `Wait-Set`

State root: `~/.local/state/codex/orchestration/jobs/<work_id>/` (`0700`,
`O_NOFOLLOW`, atomic rename + `fsync` on file and parent directory):

1. **`Work Ledger` (`work.json` + `checkpoint.md`):**
   - `work.json` holds `work_id`, `title`, `tier` (`direct | orchestrated`),
     `goal`, `completion_criteria[]`, `state` (`running | waiting | waiting_user | completed | failed | cancelled`),
     pinned `(account_id, project_id)`, `delivery_mode` (`inject_and_notify`
     default | `notify_only`), `front: { latest_chat_id, generation, history[] }`,
     `lead: Option<{ kind, ref_id, profile?, generation, history[] }>`,
     `participants[]` (`role: worker|verifier`, `backend: codex|chatgpt|shell`,
     `ref_id`, `slug`, `owner_scope`, `call_id`, `state`, `result_summary`),
     `consumed_seq`, and `budget` (`max_wakeups`, `wakeups_used`,
     `max_wall_secs`, `max_concurrent_workers`, `min_cooldown_secs`).
   - `checkpoint.md` holds the concise handoff summary updated by `L` before
     ending a round or rolling over, giving `L2` and `Fn` one-read recovery.
2. **`Typed Inbox` (`events.jsonl`):**
   - Append-only immutable events deduplicated by `event_id` with monotonic
     `seq`: `worker.completed`, `worker.failed`, `worker.stalled`,
     `verifier.passed`, `verifier.rejected`, `user.message`,
     `user.scope_updated`, `user.front_rebound`, `timer.deadline`,
     `budget.exhausted`.
   - `R` advances `consumed_seq` only after confirmed wake-up dispatch receipt;
     across crashes, restarts, or `L1 -> L2` handoffs, unconsumed events
     (`seq > consumed_seq`) are preserved without loss or duplicate delivery.
3. **Declarative `Wait-Set`:**
   - Unifies fan-in barriers, fail-fast checks, user interrupts, and stall
     watchdogs into one declarative rule registered before `L` (or `F` in Tier 1)
     ends its turn:
     - `wake_target`: `lead | front`
     - `all_of`: participant IDs that must all settle before waking (Fan-in Barrier)
     - `any_of`: participant IDs where the first settlement/failure wakes immediately
     - `on_user_event`: wake immediately when `Fn` submits a scope update/message
     - `deadline_secs`: stall watchdog timer waking target if no condition fires in time

---

## 5. Mobile `F1 -> Fn` Continuation, Delivery Safety, and Circuit Breakers

- **Seamless `Fn` Re-attachment:** When a user opens a new chat (`Fn`) on
  Android/Web ChatGPT and references prior/ongoing work, `Fn` calls
  `orch_works(action="list")` to discover active/recent `Work`s (Tier 1 & Tier 3)
  without recreating workers, then calls `orch_work(action="rebind_front")`
  with either explicit `chat_id` or a verified prompt marker. `R` confirms the
  matching `(account_id, chat_id)` via `chatgpt-provider` `GET /control/chats`,
  atomically updates `front.latest_chat_id = Fn`, renames the chat `[F<n>] <title>`,
  and records `user.front_rebound`.
- **Safe `inject_and_notify` Delivery:** Before calling `chatgpt-provider`
  `POST /control/send(chat_id=...)` on `F/Fn` or ChatGPT `L`, `R` checks both
  local `activeTurns` and server `/conversation/{id}/stream_status` via
  `POST /control/observe`. If the target chat is currently streaming (e.g. user
  is actively typing on Android), `R` records `PENDING_DELIVERY` and retries
  with exponential backoff rather than colliding. Every milestone, blocker, or
  final delivery also emits a `termux-notification` (and `notify_only` skips
  chat injection entirely when requested).
- **Hard Circuit Breakers:** `R` enforces `max_wakeups` (default `20`),
  `max_wall_secs`, `max_concurrent_workers`, minimum wake-up cooldown, and
  `event_id` idempotency. On budget exhaustion, `R` transitions the `Work` to
  `waiting_user` and notifies `F/Fn` instead of looping indefinitely.

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
- **Static 6-tool core + disjoint extension slot** (uniform immutable MCP
  annotations `readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false`):
  1. `orch_works` (`list | read | checkpoint | inbox`)
  2. `orch_work` (`create | promote | rebind_front | update_scope | cancel`)
  3. `orch_dispatch` (`register_worker | register_verifier | wait_set | wait_clear`)
  4. `orch_report` (`milestone | need_user | complete | fail`)
  5. `orch_catalog`
  6. `orch_call` (`lead_rollover`, `budget_extend`, `reconcile_delivery`, `relay_status`, `rename_session`)
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
  stores it in the budget. The default is 80. Zero disables the turn-count
  trigger. Rollover bootstrap and replay turns do not count toward the next
  generation's threshold.
- `orch_call(lead_rollover)` accepts `work_id`, optional `reason`, and optional
  `checkpoint_md`. The relay saves the checkpoint and a durable intent before
  requesting the replacement. The Provider checks the live account against
  the pinned account before dispatch, passes that identity to CompletionClient,
  and sends in the pinned project. The relay confirms the new project's chat
  and its creation marker, applies the `[L<n>]` title, then persists the new
  generation, history and `lead.rolled_over` Inbox event. Manual rollover can
  return `pending` while the relay reconciles that transaction.
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
