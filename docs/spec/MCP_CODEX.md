# Stateless `@TermuxCodex` MCP Bridge (`termux-codex-mcp`)

**Status:** Normative annex of [SPEC.md](../../SPEC.md).
Implemented in [mcp/termux-codex](../../mcp/termux-codex/src/main.rs) over
shared [crates/codex-daemon-client](../../crates/codex-daemon-client/src/lib.rs)
and [crates/mcp-kit](../../crates/mcp-kit/src/lib.rs).

---

## 1. Architecture and Ownership Boundary

- **Stateless thin bridge:** `@TermuxCodex` owns **no** conversation database,
  task state machine, execution lease, or credential store. All thread/turn
  state lives authoritatively inside the official `codex app-server daemon`
  queried over its public RFC 6455 WebSocket-over-UDS control socket.
- **Zero Wrapper/Provider coupling:** Runs as an independent binary
  (`~/.local/lib/codex/mcp/termux-codex/current/termux-codex-mcp`, `$PREFIX/bin/termux-codex-mcp`)
  and can be retired cleanly without affecting `codex`, `codex-ext`,
  `@TermuxShell`, or `chatgpt-provider` once official `codex remote-control`
  covers the same mobile workflow.
- **Uniform immutable MCP annotations:** Every static tool declares
  `readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false`, plus
  an accurate per-tool `idempotentHint`.

---

## 2. Static 5-Tool Core & Disjoint Extension Slot

| Tool | `idempotentHint` | Responsibility |
| --- | --- | --- |
| `codex_status` | `true` | Read-only daemon running state, loaded threads, redacted model provider summaries (`id`, `name`, `base_url`), default provider, redacted config summary (`model`, `model_provider`, `approval_policy`, `sandbox_mode`), and explicit `warnings` on partial RPC degradation. Never exposes auth tokens. |
| `codex_threads` | `true` | Pure read-only `list` (`thread/list`), `read` (`thread/read`), and `items` (`thread/items/list`, default `limit: 20`) with pagination cursors and a `512 KiB` response bound. |
| `codex_turn` | `true` | `start` (1-shot `thread/start` + `turn/start` when `thread_id` is omitted, or `thread/resume` + `turn/start`), `steer` (`turn/steer` with automatic active-turn resolution), and `interrupt` (`turn/interrupt`), protected by atomic `call_id` transport deduplication receipts and bounded `wait_ms` (`0..=120000`). |
| `codex_catalog` | `true` | Lists all allowlisted, strictly disjoint extension actions and their exact JSON schemas. |
| `codex_call` | `false` | Invokes an allowlisted extension action (`thread_fork`, `thread_archive`, `thread_unarchive`, `thread_rename`, `thread_compact`, `review_start`, `account_usage`, `goal_get`, `goal_clear`, `models_list`). Strict schema validation rejects unknown keys and out-of-range values before daemon RPC and returns the exact `expected_schema` (or `available_actions`) so clients self-correct without connector refresh. |

---

## 3. Transport Deduplication & Partial Outage Isolation

- `codex_turn` inspects existing `call_id` receipts first (so replays and
  payload-conflict checks work even while a daemon is stopped), connects to the
  daemon before claiming a brand-new `call_id` (so a stopped daemon returns
  `codex_daemon_not_running` without poisoning the `call_id`), and writes an
  atomic `0600` (`O_NOFOLLOW`, `fsync`) `started` receipt under
  `$PREFIX/var/tmp/termux-codex-mcp-receipts` strictly before issuing any
  mutating RPC, pruning oldest settled receipts beyond 512 entries.
- If the daemon disconnects mid-turn while `codex_turn` is polling, the bridge
  breaks immediately instead of waiting out `wait_ms` and reports
  `state: "delivery_unknown"` with `poll_error`.
- If the target profile's `codex app-server daemon` is stopped, `codex_status`
  returns `daemon_running: false` cleanly and other tools fail with structured
  `codex_daemon_not_running` errors without terminating the MCP worker.
