# Stateless `@TermuxChatGPT` MCP Bridge (`termux-chatgpt-mcp`)

**Status:** Normative annex of [SPEC.md](../../SPEC.md).
Implemented in [mcp/termux-chatgpt](../../mcp/termux-chatgpt/src/main.rs) over
shared [crates/mcp-kit](../../crates/mcp-kit/src/lib.rs) and the local
authenticated `/control/*` plane of
[providers/chatgpt](../../providers/chatgpt/src/control.rs)
([CHATGPT_PROVIDER.md](CHATGPT_PROVIDER.md)).

---

## 1. Architecture and Ownership Boundary

- **Stateless thin bridge:** `@TermuxChatGPT` owns **no** CDP pipe, **no**
  conversation/task database, **no** Linux AppHost process, and **zero** local
  shell/code execution privilege. It exclusively relays validated MCP requests
  to `chatgpt-provider`'s loopback-only `/control/*` HTTP API using the private
  `0600` bearer token (`~/.local/state/codex/providers/chatgpt/control.token`,
  verified via `O_NOFOLLOW`, owner UID, and `0600` mode).
- **Loopback-only fail-closed transport:** `CHATGPT_PROVIDER_URL` (default
  `http://127.0.0.1:4319`) must be a literal loopback address (`127.0.0.0/8`,
  `[::1]`, or `localhost` with a non-zero port — never issuing external DNS);
  non-loopback or non-HTTP URLs are rejected before reading the bearer token.
- **Zero Wrapper coupling:** Runs as an independent workspace and binary
  (`~/.local/lib/codex/mcp/termux-chatgpt/current/termux-chatgpt-mcp`,
  `$PREFIX/bin/termux-chatgpt-mcp`), with zero compile-time or runtime
  dependency on `crates/wrapper`, `codex`, or `codex app-server daemon`.
- **Uniform immutable MCP annotations:** Every static tool declares
  `readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false`, plus
  an accurate per-tool `idempotentHint`.

---

## 2. Static 6-Tool Core & Disjoint Extension Slot

| Tool | `idempotentHint` | Responsibility |
| --- | --- | --- |
| `chatgpt_status` | `true` | Read-only provider & Linux ChatGPT AppHost status (`host_running`, `auth_state`, `ui_state`, `completion_ready`, account summary, `active_chat_id`, `generating`, live models). Returns `provider_reachable: false` gracefully when `chatgpt-provider` is stopped. Never exposes tokens or cookies. |
| `chatgpt_chats` | `true` | Pure read-only `list` (`GET /control/chats`, paginated, optional `archived`) and `read` (`GET /control/chats/{id}`, linear branch excluding private `thoughts`). Bounded at `512 KiB`. |
| `chatgpt_projects` | `true` | Pure read-only `list` (`GET /control/projects`) and `read` (`GET /control/projects/{id}`, paginated). Bounded at `512 KiB`. |
| `chatgpt_send` | `true` | `send` (`POST /control/send`, requiring `call_id` + `prompt`, optional `chat_id`, `project_id`, `model`, `reasoning_effort`, bounded `wait_ms` `0..=300000`), `observe` (`POST /control/observe`), and `stop` (`POST /control/stop`). |
| `chatgpt_catalog` | `true` | Lists all allowlisted, strictly disjoint extension actions and their exact JSON schemas. |
| `chatgpt_call` | `false` | Invokes an allowlisted extension action (`project_create`, `chat_rename`, `viewer_url`, `viewer_open`, `host_restart`). Strict schema validation rejects unknown actions/keys/out-of-range values before HTTP dispatch and returns exact `expected_schema` (or `available_actions`) so clients self-correct without connector refresh. |

---

## 3. Transport Deduplication & Outage Isolation

- `chatgpt_send(action="send")` inspects existing `call_id` receipts first (so
  replays and `chatgpt_call_id_payload_conflict` work even while
  `chatgpt-provider` is stopped), verifies `GET /healthz` and token readiness
  **before** claiming a brand-new `call_id` (preventing a stopped provider from
  poisoning the caller's `call_id`), and writes an atomic `0600` (`O_NOFOLLOW`,
  `fsync`) `started` receipt under `$PREFIX/var/tmp/termux-chatgpt-mcp-receipts`
  (schema `termux-chatgpt-mcp-receipt`) strictly before issuing
  `POST /control/send`.
- If `chatgpt_send(action="send")` returns `state: "in_progress"` (when
  `wait_ms` elapses before completion), that result is intentionally committed
  to the receipt so retries never duplicate the in-flight turn; callers follow
  up with `chatgpt_send(action="observe")`.
- Guaranteed pre-dispatch rejections (`HTTP 400/401/403`, `login_required`,
  `generation_already_active`, `completion_client_unavailable`,
  `host_profile_busy`, `host_rootfs_not_found`, and pre-connect transport
  errors) release the freshly claimed `started` receipt (with parent `fsync`)
  and return `executed: false` so the caller can safely retry with the same
  `call_id` after recovery.
- If the local HTTP connection drops or times out mid-send after dispatch,
  `@TermuxChatGPT` leaves the `started` receipt unsettled and returns
  `state: "delivery_unknown"` — never silently resending a potentially accepted
  ChatGPT turn.
- Settled receipts are pruned automatically beyond `512` entries while
  preserving unsettled (`started`) receipts.
