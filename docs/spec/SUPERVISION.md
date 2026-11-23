# Opt-In `runit` 3-State Generation Supervision

**Status:** Normative annex of [SPEC.md](../../SPEC.md) for Stage 2 background
services (`termux-shell-tunnel`, `termux-codex-tunnel`, `chatgpt-provider`,
`termux-chatgpt-tunnel`, and `tunnel-client`). Implemented by
[scripts/runit_service.py](../../scripts/runit_service.py).

---

## 1. Immutable Generation Layout & Opt-In Enrollment

- Every supervised Stage 2 component stores immutable verified generations under
  the unified canonical XDG `codex/` namespace:
  ```text
  $HOME/.local/lib/codex/<component>/generations/<version>/<binary>
  $HOME/.local/lib/codex/<component>/current  -> generations/<active_version>
  $HOME/.local/lib/codex/<component>/previous -> generations/<previous_version>
  ```
  (for example `mcp/termux-shell`, `mcp/termux-codex`, `mcp/termux-chatgpt`,
  `providers/chatgpt`, `transport/tunnel-client`; service logs live under
  `$HOME/.local/state/codex/services/<service>/log`).
- **Opt-in invariant:** Installing or updating a component binary **never**
  auto-enrolls a new `runit` service under `$PREFIX/var/service/`. Enrollment
  happens only through an explicit `enroll` action.
- **Stable `current` execution:** Generated `$PREFIX/var/service/<service>/run`
  scripts execute `$HOME/.local/lib/codex/<component>/current/<binary>` rather than
  hardcoded versioned paths, ensuring reboots and `runsvdir` restarts always
  launch the selected generation.
- **Tunnel-wrapped MCP supervision (`--runner-component` / `--runner-binary`):**
  For services where a transport runner hosts an MCP child process (e.g.
  `termux-{shell,codex,chatgpt}-tunnel` running `transport/tunnel-client` over
  `mcp/termux-*`), `run` executes `<runner>/current/<runner-binary>` while
  `status` and `activate` verify BOTH the live runner PID's `/proc/<pid>/exe`
  and the linked MCP component generation (persisting verified active
  generation metadata under
  `$HOME/.local/state/codex/services/<service>/active-generation.json`).
- **Hyphenated readiness / busy-check commands & timeout safety:**
  `--readiness-cmd` and `--busy-check-cmd` accept hyphenated subcommand flags
  (such as `curl -fsS http://127.0.0.1:4319/healthz`) cleanly; command timeouts
  are treated fail-closed (`readiness_ok=false` in `status`, deferred busy
  guard or automatic rollback in `activate`) rather than crashing the CLI.

---

## 2. Mandatory 3-State Visibility

`status` must never conflate package installation with live service activation.
It reports three independent fields:

1. **`selected_generation`**: version, resolved binary path, and SHA-256 of the
   `current` symlink target (plus retained `previous_version`).
2. **`running_generation`**: live PID, `/proc/<pid>/stat` start ticks,
   `/proc/<pid>/exe` (explicitly flagging `(deleted)` images), derived running
   version, and readiness check result.
3. **`activation_state`**:
   - `active`: running PID's `/proc/<pid>/exe` matches `selected_generation`
     and readiness passes.
   - `pending_activation`: `selected_generation` differs from the running
     process image (e.g. deferred due to active work or awaiting restart).
   - `stopped` / `not_enrolled` / `error`.

---

## 3. Safe Rebinding and Automatic Rollback

- Before restarting an enrolled running service, `activate` runs any declared
  busy-check guard. If active work/leases exist, it does **not** force-kill the
  service; it leaves the old process operational and reports
  `pending_activation`.
- When idle, `activate` atomically updates `previous` and `current` symlinks
  (`os.replace` + `fsync(dir)`), invokes `sv restart`, and verifies a new
  `(pid, start_ticks)`, matching `/proc/<pid>/exe`, and readiness check within
  the bounded timeout.
- On restart or readiness failure, `activate` automatically restores
  `current -> previous` and restarts the previous generation.
