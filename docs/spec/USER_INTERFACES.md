# User-Facing Interfaces and Standalone `codex-ext` (`cdx`) Contract

**Status:** Normative detail of [SPEC.md](../../SPEC.md).
This annex specifies the Stage 1 `codex` CLI failure-scoping and external
shared-home binding boundaries together with the decoupled Stage 2 Slice 2
`codex-ext` (`cdx`) companion interface. For active code traceability consult
[docs/ARCHITECTURE.md](../ARCHITECTURE.md); for verified device/CI receipts
consult [CURRENT.md](../../CURRENT.md).

---

## 1. Stage 1 `codex` Wrapper CLI & Shared-Home Boundary

### 1.1 Public dispatch and failure scoping

- Wrapper (`$PREFIX/bin/codex`) owns top-level `update`, `doctor`, and `wrapper`
  (`codex wrapper status [--verify [--wrapper-sha256 <SHA256>] [--upstream-version <VER>]] [--json]`,
  `codex wrapper --help`).
- Invoking `update`, `doctor`, or `wrapper` after leading upstream global flags
  (e.g. `codex -c k=v doctor`) fails closed with exit status `2` and directs
  the caller to invoke `codex <subcommand>` directly.
- Retired Phase 2.1 flags (`codex wrapper setup`, `--bare`, `--with`) fail
  closed with exit status `2`.
- Unsupported Android Landlock sandbox modes (`--sandbox read-only|workspace-write`,
  `-sread-only`, `-sworkspace-write`, `sandbox linux`) fail closed with exit
  status `2`.
- Fast inspection invocations (`--version`, `-V`, `--help`, `-h`, `help`, `-hV`)
  bypass daemon settings writes, shared-home writes, and timezone extraction,
  emitting exact upstream output with zero added Wrapper lines.

### 1.2 External profile environment binding (`CODEX_SHARED_HOME`, `CODEX_SQLITE_HOME`)

Wrapper owns no profile registry or profile CRUD. External callers such as
`codex-ext` (`cdx`) isolate OpenAI OAuth `auth.json` and `app-server-daemon`
sockets per `$CODEX_HOME` while sharing conversations, SQLite state, and common
configuration via `CODEX_SHARED_HOME` and `CODEX_SQLITE_HOME`:

- Required ownership & safety: both `$CODEX_HOME` and `$CODEX_SHARED_HOME` must
  be owned, non-symlink, non-group/world-writable absolute directories.
- Shared directories: `sessions`, `archived_sessions`, `thread-writer-locks`,
  `rollout-migrations`, `memories`, `memories_v2`,
  `tui-thread-reference-capabilities`, `skills`, `plugins`, `rules`.
- Shared append-only logs: `session_index.jsonl`, `history.jsonl` (detached
  profile copies are merged by exact line equality under `flock` before
  relinking so renamed threads are never dropped).
- Shared optional files: `installation_id`, `config.toml`, plus caller-supplied
  colon-separated basenames in `CODEX_SHARED_EXTRA_FILES` (max 16 unique
  entries, each <= 128 ASCII graphic chars without `/`; reserved sensitive or
  runtime-local names such as `auth.json`, `*.sqlite*`, `*.lock`,
  `app-server-control`, `app-server-daemon`, `version.json`, and
  `models_cache.json` are rejected; absent files in `$CODEX_SHARED_HOME` are
  skipped cleanly without error).

---

## 2. Standalone `codex-ext` (`cdx`) Companion (Stage 2 Slice 2)

`codex-ext` is a completely independent Rust binary (`crates/codex-ext`, own
`[workspace]`, installed as `$PREFIX/bin/codex-ext` with `$PREFIX/bin/cdx`
symlink) that shares `crates/codex-daemon-client` with `@TermuxCodex` and has
**zero** compile-time or runtime coupling to `crates/wrapper`.

### 2.1 Orthogonal auth and provider model

- **OpenAI OAuth multi-account profiles (`cdx use <acct>`, `!cdx switch <acct>`):**
  Isolate `auth.json` and `app-server-daemon` per profile under
  `$HOME/.local/share/codex/manager/profiles/<NAME>/home` while setting
  `CODEX_SHARED_HOME=$HOME/.codex`, `CODEX_SQLITE_HOME=$HOME/.codex`, and
  ensuring `hooks.json` is included in `CODEX_SHARED_EXTRA_FILES`. Switching
  between OpenAI accounts resumes the **same thread UUID** once the previous
  idle daemon releases its writer lock.
- **External model providers (`!cdx provider <name>`):** External providers
  (`chatgpt.config.toml`, `ais.config.toml`, etc. with
  `requires_openai_auth = false`) do not use OpenAI OAuth `auth.json`. They
  always launch on canonical `~/.codex` with `codex -p <name>` (or bare `codex`
  for `openai`) as a **new thread**, matching upstream `0.171.0`
  `threads.model_provider` immutability.

### 2.2 Non-PTY launcher & in-TUI `!cdx switch` / `!cdx provider`

- **Direct Termux TTY inheritance (zero PTY proxy):** `cdx [use <acct>] [ARGS...]`
  spawns `$PREFIX/bin/codex` inheriting `fd 0/1/2` directly so native Termux
  TTY behavior, `SIGWINCH`, and upstream OSC 52 `/copy` pass through with zero
  interposed byte cap or latency.
- **Terminal restoration:** Before spawning `codex`, `cdx` captures stdin
  `termios` (when TTY). Across account/provider transitions and on exit it
  restores saved `termios` (`TCSAFLUSH`) and resets alt-screen/cursor/mouse
  modes on TTY stdout.
- **Session registry & daemon-child `!` matching:**
  - Upstream `0.171.0` executes TUI `!` shell commands as children of
    `app-server daemon` (inheriting daemon `CODEX_HOME` and `cwd`, not TUI-only
    env vars).
  - Each live `cdx` parent writes an atomic `0600` session record to
    `$HOME/.local/state/codex/ext/sessions/<pid>.json` containing `pid`,
    `/proc/<pid>/stat` `start_ticks`, `account`, `codex_home`, and `cwd`. Stale
    records whose `(pid, start_ticks)` no longer match a live non-zombie process
    are pruned automatically.
  - `!cdx switch` / `!cdx provider` matches live sessions by `(CODEX_HOME, cwd)`
    unless `--session <PID>` is specified. Multiple matching windows fail closed
    (exit `2`) listing every candidate PID.
- **Zero-interruption pre-flight busy check & writer-lock release:**
  - Upstream default `thread_unload_delay_secs` is preserved (`= 0` is forbidden
    to prevent `turn_aborted` on disconnect).
  - Before submitting a handoff to parent `cdx`, `!cdx switch` / `!cdx provider`
    connects to the current daemon's control socket via
    `crates/codex-daemon-client` (RFC 6455 WebSocket-over-UDS using
    `/proc/self/fd/<dirfd>/...` for Termux `termux-uds-fd35` short paths) and
    inspects `thread/loaded/list` + `thread/read`.
  - The upstream `!` shell command can itself count as a turn. Account and
    provider switches use `CODEX_THREAD_ID` to identify the caller and defer
    the transition when that conversation is the only active one. An account
    switch uses that ID to resume the conversation. When other threads are
    active, the command reports them and leaves the TUI and daemon running.
    Account switches can explicitly wait for those turns with `--wait`.
  - The accepted `!cdx` command writes a `0600` atomic handoff ticket to
    `$HOME/.local/state/codex/ext/handoff/<parent_pid>.json` and exits `0`.
    Current launchers poll for tickets and claim them by rename. Tickets bind
    to the target PID, process `start_ticks` and session `generation`. The
    launcher keeps a claimed ticket until it completes or aborts the switch.
    A second request for that window receives an explicit pending error,
    including during teardown. The registration remains discoverable while
    the switch runs. If the TUI exits on its own, the launcher checks for a
    final ticket under the publication lock before unregistering. Exit `0`
    from the sending CLI acknowledges queueing. The launcher reports the
    outcome. The legacy signal path remains for launchers without a
    generation field. Current launchers require generation-bound tickets.
    Parent `cdx` leaves the TUI running while waiting for the
    daemon to become idle. An expired wait or an unavailable status check
    leaves the existing TUI in place. Deferred provider switches require a
    launcher that advertises support in its session record. Sessions opened by
    older launchers keep running and decline deferred provider requests.
  - Once idle, parent `cdx` terminates the child TUI (`SIGTERM`, 3s grace) and
    restores the terminal. For an account switch it re-checks idle status,
    stops the idle source daemon via official `codex app-server daemon stop`,
    waits up to 3s for advisory `flock` release on
    `$HOME/.codex/thread-writer-locks/<tid>.lock`, and launches `codex resume
    <tid>` in the target profile. For a provider switch it starts a new thread
    on `~/.codex` and leaves the source daemon running.
- **Standalone shell switch recovery:** When invoked outside a `cdx` parent and
  the source daemon has active turns on an interactive TTY without `--wait`,
  `cdx switch` prompts `[w] wait / [r] return / [q] cancel` rather than leaving
  the user stranded. A Codex shell command with no registered launcher fails
  before standalone handling for both commands, since it cannot replace the
  originating TUI.

### 2.3 Profile lifecycle (`cdx profile`)

- Storage layout (compatible with existing profiles):
  - Directory: `$HOME/.local/share/codex/manager/profiles/<NAME>/` (`0700`),
    child `home/` (`0700`), metadata `profile.meta` (`0600`).
  - New metadata uses `codex-manager-profile\n`. Existing `ai` profiles with
    `codex-manager-profile-v2\n` or `codex-manager-profile-v1\nid\t<NAME>\n`
    are also accepted. The v1 ID must match the directory name.
  - Default selection pointer: `$HOME/.local/share/codex/manager/default-profile`
    (`0600`, `codex-manager-default-profile\nprofile\t<NAME>\n`).
- Name validation: 1..=64 chars matching `[A-Za-z0-9][A-Za-z0-9._-]*`; `default`,
  `home`, `wrapper`, and `termux` are reserved.
- Safety: `rename` and `delete` fail closed if a managed daemon is currently
  running for that profile; non-interactive `delete` requires `--yes`.
- Replaces and retires `contrib/codex-account`.

### 2.4 Notifications (`cdx notify` — Option A)

- Preferences stored in `$HOME/.config/codex/ext/notify.json` (`0600`).
- **`Stop` (turn completion):** Managed as top-level
  `notify = ["codex-ext", "notify", "emit", "Stop"]` in `$HOME/.codex/config.toml`
  via comment- and table-preserving atomic edits (works immediately across all
  accounts without interactive trust approval).
- **Additional events (`PermissionRequest`, `SubagentStop`, `Interrupt`, etc.):**
  Managed in `$HOME/.codex/hooks.json` (preserving unrelated user hooks) and
  shared across named accounts via `CODEX_SHARED_EXTRA_FILES=hooks.json`.
  Enabling `hooks.json` events prints a notice regarding Codex `0.171.0+`'s
  one-time hook hash trust approval.
- **Delivery (`cdx notify emit <EVENT> [JSON]`):** Accepts trailing JSON argv
  (`config.toml` `notify`) or bounded stdin JSON (`hooks.json`, <= 64 KiB),
  sanitizes and truncates excerpts (<= 160 chars, control chars stripped, no
  secrets), and invokes Termux:API (`termux-notification` / `termux-toast`)
  with a bounded 4s timeout so notification failures never break Codex.
