# Active Work Order: Pre-Slice-7 Residual Tasks & Stage 2 Slice 7 (`@TermuxOrch`, 7A / 7B)

- **Branch / Base:** `main` (calibrated at `336b9c1c`, after Stage 1 graduation and Stage 2 Slices 1–6 live rollout)
- **Lifecycle:** Ephemeral execution work order per [docs/INDEX.md](docs/INDEX.md). Archive or remove once Slice 7 and residual milestones close and receipts move to [CURRENT.md](CURRENT.md).
- **Normative authorities:** [SPEC.md](SPEC.md), [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md), [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md), [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md), [docs/spec/CHATGPT_PROVIDER.md](docs/spec/CHATGPT_PROVIDER.md), [docs/spec/MCP_CHATGPT.md](docs/spec/MCP_CHATGPT.md), [docs/spec/SUPERVISION.md](docs/spec/SUPERVISION.md), and [docs/spec/ORCHESTRATION.md](docs/spec/ORCHESTRATION.md).
- **Current implemented map & live evidence:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [CURRENT.md](CURRENT.md), [docs/ROADMAP.md](docs/ROADMAP.md).

---

## 1. Completed Stage 2 Baseline (Slices 1–6 — Verified & Live on `main`)

All 6 original decoupled Stage 2 slices are complete, hardened (`ed3d9daf`, CI 7/7 green), upgraded with programmatic `/control/connectors*` (`64667a35`) and multi-lane parallel execution (`336b9c1c`, CI 3/3 green), enrolled in `runit`, and live in ChatGPT workspace `3ebe5ee5-8050-4549-8efb-873f30a8906e` (exact SHA-256s, PIDs, connector IDs, and E2E receipts in [CURRENT.md](CURRENT.md)):

- [x] **Slice 1:** Wrapper slimming + universal Ed25519 `SHA256SUMS.sig` signed `wrapper` release (`sequence=1`) & live Termux cutover.
- [x] **Slice 2:** Shared `crates/codex-daemon-client` + standalone `codex-ext` (`cdx`) + §7 isolated proof + retirement of `contrib/codex-account`.
- [x] **Slice 3:** Shared `crates/mcp-kit` + `@TermuxShell` (`termux-shell-mcp`) + `tunnel-client v0.0.21` + `runit` 3-state supervision (`termux-shell-tunnel` live).
- [x] **Slice 4:** Stateless `@TermuxCodex` (`termux-codex-mcp`, 5 static tools + 10 extensions, `termux-codex-tunnel` live).
- [x] **Slice 5:** Immutable `host/chatgpt-desktop` (`26.1117.41037`) + slim `chatgpt-provider` (`0.1.0+multilane`, `runit` active, multi-lane up to 8 lanes, programmatic `/control/connectors*`).
- [x] **Slice 6:** Stateless `@TermuxChatGPT` (`termux-chatgpt-mcp`, `0.1.0+multilane`, `termux-chatgpt-tunnel` live, per-`chat_id` stop) + live `@TermuxShell`/`@TermuxCodex`/`@TermuxChatGPT` registration, multi-worker iterative loop E2E, and 3-lane re-entrant parallel E2E.

---

## 2. Pre-Slice-7 Residual Operational & Verification Backlog

Items originally planned before Slice 7 was requested — kept distinct from Slice 7 development so each can be executed or deferred explicitly by operator decision:

| ID | Category | Residual Task | Current Boundary / Blocker |
| --- | --- | --- | --- |
| **R-1** | Signed Releases | Publish initial Ed25519-signed GitHub Releases for the 6 Stage 2 workflows (`codex-ext`, `termux-shell-mcp`, `termux-codex-mcp`, `chatgpt-provider`, `termux-chatgpt-mcp`, `tunnel-client`) | All 6 workflows & `.github/scripts/publish_stage2_release.py` pass CI on push; awaits explicit operator-authorized `workflow_dispatch` (`publish_release=true`) |
| **R-2** | Live Auth Verification | Online multi-account `/switch` cross-account UUID handoff (Account A → writer release → Account B resume on an approved test thread) & §7.2 live `session_index.jsonl` rename/delete 4-path check ([ROADMAP.md §7](docs/ROADMAP.md#7-follow-up-verification-session-handoff-and-session_indexjsonl)) | Isolated `0.171.0` proof passed in Slice 2; real online multi-account test requires explicit test-thread execution |
| **R-3** | Physical UI / Client Verification | Physical interactive Termux UI `/copy` OSC 52 long-payload paste-back check & external `codex remote-control` client pairing | Requires live human interaction on Android keyboard/clipboard & external client |
| **R-4** | Disk Reclamation | Reclaim legacy per-home `packages/app-server-daemon/releases/` and archived `~/.local/state/codex/legacy-0.167.2-archive/` after retention window | Preserved safely until operator authorizes deletion |
| **R-5** | Phase 2.6 Canary (Deferred) | Optional read-only `workflow_dispatch` upstream compatibility canary + restricted dedicated physical-device promotion gate ([ROADMAP.md §3](docs/ROADMAP.md#3-phase-26--read-only-upstream-discovery-and-qualification)) | Explicitly deferred; local `codex update --upstream` works independently |
| **R-6** | Hardware Boundaries (Informational) | Actual physical power-cut test & real 16 KiB kernel page-size Android device execution | Only `SIGKILL`/fault-injection and 16 KiB ELF geometry on API 37 (4 KiB kernel) are testable on this device |

---

## 3. Slice 7 — Durable Multi-Role Orchestration (`@TermuxOrch` / `mcp/termux-orch`)

Normative specification: [docs/spec/ORCHESTRATION.md](docs/spec/ORCHESTRATION.md).
**Non-negotiable decoupling invariant:** Zero changes to `crates/wrapper`, root `Cargo.toml`/`Cargo.lock`, or the stateless semantics of `@TermuxShell`, `@TermuxCodex`, and `@TermuxChatGPT`.

### 3.1 Independent Workspace & Packaging Layout

| Artifact | Path / Service | Responsibility |
| --- | --- | --- |
| Workspace & binary | `mcp/termux-orch/Cargo.toml` (`[workspace]`) → `termux-orch` | Single Rust binary reusing `crates/mcp-kit` + `crates/codex-daemon-client` + loopback `chatgpt-provider /control/*` |
| Runtime daemon (`R`) | `termux-orch relay` → `runit` service `termux-orch-relay` | Watches `Work Ledger` / `Typed Inbox` / `Wait-Set`, polls owners, enforces Circuit Breakers, executes `inject_and_notify` wake-ups + `termux-notification` |
| 4th MCP connector | `termux-orch mcp` → `runit` service `termux-orch-tunnel` → **`@TermuxOrch`** | 6 static tools (`orch_works`, `orch_work`, `orch_dispatch`, `orch_report`, `orch_catalog`, `orch_call`) registered via `POST /control/connectors` |
| Durable state root | `~/.local/state/codex/orchestration/jobs/<work_id>/` (`0700`) | `work.json`, `events.jsonl`, `checkpoint.md` (atomic rename + `fsync`, `O_NOFOLLOW`) |
| CI workflow | `.github/workflows/termux-orch.yml` | Path-triggered lint/test/cross-compile + `publish_stage2_release.py` |

### 3.2 Slice 7A Checklist (Core Engine + Mobile `F/L/W/V/R` End-to-End) *(Complete & Live)*

- [x] **7A-1 `providers/chatgpt` `stream_status` observation:** Extended `page_driver.js` `observeChat` to inspect server `/conversation/{id}/stream_status` when no local lane is active; unit tests in `page_driver.test.mjs` (6/6 PASS); rebuilt & activated `chatgpt-provider` `0.1.0+7a` (`8e18211f...`, live `remote_stream_status: "COMPLETE"` verified).
- [x] **7A-2 `mcp/termux-orch` core ledger & state machine:** Implemented `Work Ledger` (`work.json`, `checkpoint.md`), `Typed Inbox` (`events.jsonl`, monotonic `seq`/`consumed_seq`, `event_id` deduplication), declarative `Wait-Set` (`all_of`, `any_of`, `on_user_event`, `deadline_secs`), and hard Circuit Breakers (`max_wakeups`, `max_wall_secs`, `max_concurrent_workers`, cooldown) in `mcp/termux-orch/src/ledger.rs` & `relay.rs`.
- [x] **7A-3 Project scoping & uniform session naming:** Auto-created/reused pinned `(account_id, project_id)` (`[Work] <topic>`) and auto-assigned `[F<gen>] <title>`, `[L<gen>] <title>`, `[W<idx>-<slug>] <subtask>`, `[V<idx>-<slug>] <scope>` across ChatGPT (`POST /control/chats/{id}/rename`) and Codex (`thread/name/set`).
- [x] **7A-4 Tier 1–3 lifecycle + `F1 -> Fn` rebinding:** Implemented `orch_works`, `orch_work` (`create`, `promote`, `rebind_front`, `update_scope`, `cancel`), `orch_dispatch` (`register_worker`, `register_verifier`, `wait_set`, `wait_clear`), `orch_report` (`milestone`, `need_user`, `complete`, `fail`), and disjoint `orch_catalog`/`orch_call` in `mcp/termux-orch/src/mcp.rs`.
- [x] **7A-5 `termux-orch relay` daemon:** Loopback `/healthz`, Codex daemon & `chatgpt-provider` observation, Wait-Set evaluation, `inject_and_notify` (default) / `notify_only` delivery with `PENDING_DELIVERY` backoff + `termux-notification`, and restart recovery preserving unconsumed `Inbox` events.
- [x] **7A-6 Static, unit, & synthetic fault gates:** `cargo fmt --check`, `clippy --all-targets --locked -- -D warnings`, dual `umask 0022`/`0077` `cargo test --locked` (4 unit + 1 compiled stdio MCP E2E PASS), plus `.github/workflows/termux-orch.yml`.
- [x] **7A-7 Live deployment & E2E verification:** Provisioned `TermuxOrch` OpenAI tunnel (`tunnel_6b03ddf5129c819188e0e908ff223271`), enrolled & upgraded `termux-orch-relay` & `termux-orch-tunnel` (`0.1.0+7a`, `fce5ea31...`) in `runit`, programmatically registered `@TermuxOrch` (`asdk_app_6b03de61dd748191bf56fef2b1029a2e`, `ACTIVE`, `schema_agreement: true`), and proved live (receipts in [CURRENT.md](CURRENT.md)):
  1. Tier 1 single task (`work_76854f714e504e2a`) → new chat `F2` seamless `front_marker` `rebind_front` & live relay delivery;
  2. Tier 1 → Tier 2 `promote` escalation (`work_5b00ad2bfda737fa`) preserving `W1`;
  3. Tier 3 `F1 → L1 → (W1 Codex || W2 ChatGPT) → sv restart termux-orch-relay recovery → Wait-Set Barrier auto-wakeup → L1 autonomously spawning Codex V1 Verifier → Round-2 wakeup → F2 completion` (`work_ff1ddcc0d878543c`).

### 3.3 Slice 7B Checklist (Long-Horizon Resilience Upgrades)

- [x] **7B-1 Automatic ChatGPT `L1 -> L2` context rollover:** Detect context-limit / turn-threshold on ChatGPT `Lead`, snapshot `checkpoint.md`, spawn `[L2] <title>` in the same pinned `project_id`, increment `generation` to fence `L1`, and replay unconsumed `Inbox` (`seq > consumed_seq`).
- [x] **7B-2 Detached long-running Shell worker watcher:** Track `>10 min` builds/tests via verified `(pid, start_ticks)` + atomic exit-receipt file inside `Work Ledger` and feed `worker.completed`/`worker.failed` into `Typed Inbox`.
- [x] **7B-3 Instant `codex-ext` `Stop` hook push:** Optional push trigger from `codex-ext notify emit Stop` into `termux-orch relay` so Codex worker settlements wake `Lead`/`Front` immediately without waiting for poll intervals.
- [x] **7B-4 Live resilience E2E:** Prove `L1 -> L2` rollover with fencing, detached shell barrier completion, and instant `Stop`-hook wake-up.

Device and test receipts are in [CURRENT.md](CURRENT.md#2026-11-23--stage-2-slice-7b-resilience-on-android-api-37-termux).
