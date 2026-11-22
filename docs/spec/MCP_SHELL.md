# Independent Termux MCP Shell (`@TermuxShell`)

**Status:** Normative annex of [SPEC.md](../../SPEC.md).
[docs/ARCHITECTURE.md](../ARCHITECTURE.md) tracks implemented code and
[CURRENT.md](../../CURRENT.md) records verified device/CI receipts.

---

## 1. Purpose and Zero-Dependency Recovery Plane

`termux_shell` is the single static MCP tool exposed by the standalone
`termux-shell-mcp` worker (`mcp/termux-shell`, own `[workspace]`, built over
`crates/mcp-kit`). It runs host commands under the Termux app UID with **zero**
runtime dependency on `codex` Wrapper, `codex app-server daemon`,
`chatgpt-provider`, or Linux ChatGPT AppHost so it remains available as an
independent recovery and automation plane even if every other component is down.

- **Exactly one static tool:** `termux_shell` — no additional receipt, status,
  or extension `catalog`/`call` tools are exposed by `@TermuxShell`.
- **Uniform immutable annotation policy:** `readOnlyHint: true`,
  `destructiveHint: false`, `openWorldHint: false`, `idempotentHint: true`
  (matching the common Stage 2 MCP annotation contract).

---

## 2. Arguments, Atomic `call_id` Receipts, and Process-Group Bounds

- **Input schema (`additionalProperties: false`):**
  - `call_id` (required, string `1..=160` bytes, no NUL)
  - `command` (required, non-blank string `1..=32768` bytes, no NUL)
  - `cwd` (optional, absolute path without NUL; defaults to `$HOME`)
  - `timeout_ms` (optional, integer `1000..=600000`, default `60000`)
- **Private receipt store:** `$PREFIX/var/tmp/termux-mcp-shell-receipts`
  (`0700` owned directory, `O_NOFOLLOW`, `0600` receipt files, <= `512 KiB`,
  schema `termux-mcp-shell-receipt`).
  - Before spawning a shell, the worker atomically commits a `state: "started"`
    intent (`O_CREAT | O_EXCL | O_NOFOLLOW`, `fsync(file)`, `fsync(dir)`).
  - Resending the same `call_id` with exact matching arguments returns the
    original settled outcome (`replayed_from_receipt: true`) without executing
    a second process; an unsettled or corrupt receipt returns
    `state: "delivery_unknown"` and never re-executes.
  - Reusing a `call_id` with different arguments returns
    `termux_call_id_payload_conflict` (`executed: false`).
- **Process group & output bounds:**
  - Each invocation runs `$PREFIX/bin/sh -c <command>` in a private process
    group (`process_group(0)`).
  - Timeout or drop terminates **only** the owned process group (`kill(-pid, SIGKILL)`),
    preserving already-captured stdout/stderr and actual exit/signal metadata.
    Ordinary surviving descendants in the group are reaped on exit; explicitly
    detached sessions are never claimed as managed Codex jobs.
  - Each output stream (`stdout`, `stderr`) is capped at `64 KiB` with explicit
    `stdout_truncated`/`stderr_truncated` flags and both UTF-8 lossy text and
    Base64 (`stdout_base64`/`stderr_base64`).

---

## 3. Packaging, Tunnel, and Connector Selection

- `termux-shell-mcp` is packaged and signed independently via
  `.github/workflows/termux-shell.yml` using the universal `SHA256SUMS.sig`
  Ed25519 contract ([DELIVERY.md](DELIVERY.md)).
- It connects to ChatGPT via its own opt-in `runit` tunnel service
  (`termux-shell-tunnel` running `tunnel-client` over stdio to
  `termux-shell-mcp`) using stable `current` generation symlinks
  ([SUPERVISION.md](SUPERVISION.md)).
- `termux-shell-mcp --describe-tools` prints the exact static `tools/list`
  payload for offline connector schema verification.
