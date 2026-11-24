# Opt-In `runit` 3-State Generation Supervision

**Status:** Normative annex of [SPEC.md](../../SPEC.md) for Stage 2 background
services (`termux-shell-tunnel`, `termux-codex-tunnel`, `chatgpt-provider`,
`termux-chatgpt-tunnel`, optional Slice 7 `termux-orch-relay` /
`termux-orch-tunnel`, and `tunnel-client`). Implemented by
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
  `mcp/termux-*`), `run` executes `<runner>/current/<runner-binary>`.
  Both `status` and `activate` verify the live runner PID's `/proc/<pid>/exe`
  and the live MCP child's executable (including deleted MCP child images in
  `deleted_image`), failing closed on `/proc` errors or overlapping children. A
  running runner with no observed MCP child remains pending. Shared-ledger
  companion services (such as `termux-orch-relay` alongside `termux-orch-tunnel`)
  use `--companion-service` so coordinated activation verifies and rolls back all
  linked services. When both Orch services are enrolled, use
  `termux-orch-tunnel` as the primary service and pass
  `--companion-service termux-orch-relay`. The activation receipt is stored under
  `$HOME/.local/state/codex/services/<service>/active-generation.json`.
- **Hyphenated readiness / busy-check commands & timeout safety:**
  `--readiness-cmd` and `--busy-check-cmd` accept hyphenated subcommand flags
  (such as `curl -fsS http://127.0.0.1:4319/healthz`) cleanly; command timeouts
  are treated fail-closed (`readiness_ok=false` in `status`, deferred busy
  guard or automatic rollback in `activate`) rather than crashing the CLI.
  For a tunnel with a dynamically assigned port, the readiness command
  reads the service's current `health.url` on each invocation before probing
  `/readyz`.

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

Runit reports the logger separately. A live logger leaves a stopped service
in `stopped` with no running service PID.

---

## 3. Safe Rebinding and Automatic Rollback

- Before restarting an enrolled running service, `activate` runs any declared
  busy-check guard. If active work/leases exist, it does **not** force-kill the
  service; it leaves the old process operational and reports
  `pending_activation`.
  With companions, a busy guard also leaves the shared selection in place.
- Activation and rollback hold the component's activation lock. When idle,
  activation stops the primary and all running companions and waits for their
  observed MCP children to exit before changing `previous` and `current`.
  It uses atomic symlink replacement and starts the companions before the
  primary. Each start requires a new `(pid, start_ticks)`, the matching
  executable, and readiness within the bounded timeout.
- After a failed start or readiness check, activation stops the attempted
  services before restoring the prior selection. It then verifies the
  recovered services. If stopping or recovery fails, it reports incomplete
  recovery. A shared-ledger rollback also requires the prior generation to
  understand the data already written to the ledger.
