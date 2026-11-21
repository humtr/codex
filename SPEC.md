# Codex for Termux — Product Specification

**Status:** Normative top-level product invariants.

This specification defines the **settled, release-independent invariants** of
Codex for Termux: what the final product guarantees, who owns each state and
decision, and what must remain true regardless of future optional extensions.
It is **not** an implementation map, a development checkpoint list, a release
ledger, or a Stage 2 feature backlog.

- Detailed normative contracts: [DELIVERY.md](docs/spec/DELIVERY.md) (installation,
  updates, rollback, doctor, Termux compatibility), [USER_INTERFACES.md](docs/spec/USER_INTERFACES.md)
  (CLI failure scoping, external profile binding, conditional Stage 2 UI/Manager),
  and [BRIDGE.md](docs/spec/BRIDGE.md) (conditional Stage 2 Native/Provider/Bridge).
- Current implemented code & owners: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
- Dated operational state & verified proof: [CURRENT.md](CURRENT.md).
- Unsettled Stage 2 roadmap & follow-up verification: [docs/ROADMAP.md](docs/ROADMAP.md).
- Documentation authority & lifecycle rules: [docs/INDEX.md](docs/INDEX.md).
- Development principles: [AGENTS.md](AGENTS.md).

This specification changes only when a required product invariant changes —
never merely because a new upstream release is adopted or a test passes.

---

## 1. Product Definition and Priorities

The product provides one public `codex` entrypoint that runs official upstream
Codex CLI cleanly on Termux (`aarch64-linux-android`, Android API 24+).

In descending priority order:

1. **Preserve a working, recoverable upstream Codex execution path.**
2. **Never damage system resolver, authentication, profile, session, or installed runtime state.**
3. **Preserve upstream process, conversation, approval, tool, and daemon behavior at the execution boundary.**
4. **Make installation, update, activation, and rollback crash-safe and fail-closed.**
5. **Keep Wrapper small, dependency-free beyond `libc`, fast to build, and near-zero cold-start overhead.**
6. **Keep future optional modules (`Manager`, enhanced `TUI`, `Provider`, signed addon bundles) strictly decoupled and opt-in — never prerequisites for bare execution or upstream updates.**
7. **Maintain independent qualification gates; never trade correctness for velocity.**

This repository is a clean rewrite. Legacy source is historical reference only
([docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md)), never a runtime dependency.

---

## 2. Continuous Upstream Adoption

Continuously following official stable OpenAI Codex releases is a permanent
product invariant.

- **Official archive adaptation:** `codex update` acquires official
  `codex-package-aarch64-unknown-linux-musl.tar.gz` archives directly from
  OpenAI release metadata, verifies the official SHA-256 digest using Wrapper's
  internal FIPS 180-4 SHA-256 implementation (requiring no external compiler,
  source checkout, signing key, remote CI result, or `openssl-tool` runtime
  dependency), unpacks complete package resources, preserves or wraps
  `codex-path/rg`, applies rule-based equal-length path remaps (`termux-fd-remap`,
  FD 33/34) and adaptive UDS cave relocation (`termux-uds-fd35`, FD 35) across
  eligible static AArch64 musl executables in `bin/*`, probes the candidate in a
  disposable account-free home, and atomically activates the new generation.
- **Separation of Wrapper updates and upstream generations:** The POSIX
  installer (`install.sh`) owns `$PREFIX/bin/codex` and its install receipt;
  the Wrapper updater owns upstream generations and `activation-state`.
  - Replacing the Wrapper executable with a newer compatible Wrapper **must not**
    force redundant re-download/re-patching of an already qualified compatible
    upstream generation or evict the retained `previous` upstream rollback slot.
  - Generation compatibility is gated by `(wrapper_api_identity,
    persistent_schema_identity, patch_policy_id)` rather than raw Wrapper binary
    SHA-256 equality (`wrapper_artifact_digest` is retained as provenance
    metadata). Intentional re-patching uses an explicit `--force-rebuild` flag
    or a `patch_policy_id` change when binary adaptation rules change.
  - A new upstream release that fits existing adaptation rules requires no
    Wrapper revision. An unsupported upstream layout fails closed before
    activation while preserving both the active and rollback generations.
- **Discovery safety:** Unpinned discovery must never silently downgrade an
  installed version or accept a changed official archive digest for an already
  installed version. Explicit `--version`, `--path`, and `--rollback` retain
  their declared semantics. Any CI compatibility canary is strictly read-only
  and is never an online prerequisite for local `codex update`.

---

## 3. Public Command Contract

Wrapper owns top-level `update`, `doctor`, and `wrapper` commands and blocks
all attempts to invoke upstream's unadapted self-updater `update` (including
after leading global flags or `codex wrapper --bare`). Invoking `update`,
`doctor`, or `wrapper` after leading upstream global flags (e.g.
`codex -c k=v doctor`) fails with usage status `2` and instructs the user to
call `codex <subcommand>` directly. Every zero-arg or upstream argument vector
— including `app-server daemon`, `remote-control`, option values, and post-`--`
prompts — passes through to the active adapted upstream runtime.

| Command | Owner | Required Invariant |
| --- | --- | --- |
| `codex` / `codex [UPSTREAM_ARGS...]` | Wrapper → upstream | On an interactive bare `codex` launch (zero args, stdin/stdout/stderr all TTYs, neither `CODEX_DISABLE_UPDATE_PROMPT` nor an explicit release pin set), check official stable release metadata using a 15-minute local cache (`update-check.cache`) and short fail-open timeout. Prompt `Update now? [y/N]` when a newer release exists. A single `y` or `Y` runs the Wrapper updater for the offered release. Continue with the active generation after a decline or update failure. Then prepare FD 33/34/35, Termux env/timezone, optional `CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME` binding, and sandbox guard, and `exec` upstream directly with argv preserved byte-for-byte (rejecting Linux Landlock `--sandbox read-only`/`workspace-write`, `-sread-only`/`-sworkspace-write`, and `sandbox linux` with exit `2`) |
| `codex --version`, `-V`, `--help`, `-h`, `help`, `-hV` | Upstream (fast path) | Bypass daemon settings, shared-home writes, timezone extraction, and addon checks; print exact upstream output with zero added Wrapper rows (`codex version` is not introduced) |
| `codex update [--upstream] [--version <M.m.p>] [--force-rebuild]` | Wrapper | Resolve & cross-check official stable release metadata, verify SHA-256 internally, skip rebuild if active generation is compatible and matches `(version, archive_sha256)` unless `--force-rebuild`, otherwise adapt `bin/*`, verify runtime/helper/companion digests, probe in disposable home, and atomically activate. Recover even if `current` is damaged. Synchronize eligible daemon homes on success, including when the generation is already current |
| `codex update --path <FILE_OR_DIR>` | Wrapper | Adapt an offline official `.tar.gz` archive or validate, probe, and activate a prepared `package-root` generation directory. Recover even if `current` is damaged. Synchronize eligible daemon homes on success |
| `codex update --check [--version <M.m.p>] [--json]` | Wrapper | Resolve & cross-check official metadata and compare against the active generation (`current` / `available` / `older`) with zero downloads, activation, journal recovery, or installed-state writes |
| `codex update --rollback` | Wrapper | Independently validate the retained `previous` complete generation even if `current` is damaged, atomically swap `current` and `previous`, and synchronize eligible daemon homes to the rolled-back generation |
| `codex update --help` / `[INVALID_ARGS...]` | Wrapper | Print usage (`0`) or reject unsupported/Stage-2/conflicting options (`2`) without invoking upstream or mutating state |
| `codex doctor [--color \| --json]` | Wrapper | Read-only composition of bounded upstream doctor (with timeout, bounded stderr/stdout capture, child-group reap, and secret redaction) and Wrapper/Termux checks with explicit component attribution; never mutates upstream DBs, `$CODEX_HOME`, or Wrapper state |
| `codex wrapper status [--verify [--wrapper-sha256 <SHA256>] [--upstream-version <VER>]] [--json]` | Wrapper | Report active/previous generation, layout, patch report, roots, and opt-in settings under a generation launch lease; `--verify` qualifies clean journal state, compatibility, on-disk asset digests, optional Wrapper/version bindings, and candidate probe |
| `codex wrapper setup [...]` / `--bare` / `--with <MODULES>` / `--help` | Wrapper (opt-in boundary) | Strictly parse and atomically update `wrapper.toml` (`setup --bare` may reset malformed regular `wrapper.toml`); `--bare` bypasses optional selections for one run; `--with` fails closed (`1`) when no admitted module exists; unavailable persisted selections never block bare upstream launch |

---

## 4. Ownership and State Invariants

Each state root has **one** authoritative writer. No component may build a
second executor, conversation database, credential store, or update controller.

- **POSIX installer (`install.sh`)** exclusively owns `$PREFIX/bin/codex`, its
  previous-executable backup, and `.codex-install.lock` transaction receipts.
  Installer and updater transitions are independent and reconciled separately
  without replaying an uncertain updater.
- **Wrapper & `codex-release-builder`** exclusively own:
  - public command dispatch, sandbox guard, and opt-in `wrapper.toml` boundary;
  - immutable generations under `$HOME/.codex/packages/standalone/releases/<id>/`
    and authoritative `activation-state` / `activation-journal` under
    `$HOME/.local/share/codex/wrapper/` (keeping `standalone/current` synchronized
    under `activation_lock` without clobbering ordinary files/directories);
  - FD 33/34/35 preparation, Termux environment/timezone/TLS setup, bounded
    post-update artifact retention, and read-only `doctor` composition.
- **Upstream Codex** exclusively owns authentication (`auth.json`), `$CODEX_HOME`
  threads/rollout/SQLite databases, model/route decisions, approvals, Native
  tool execution, compaction, writer locks, idle thread lifetime
  (`thread_unload_delay_secs`), and `app-server daemon` / `remote-control`
  process lifecycles.
- **Narrowly scoped daemon defaults and post-update synchronization:**
  - Wrapper ensures `{"updater":{"autoUpdateEnabled":false}}` in
    `$CODEX_HOME/app-server-daemon/settings.json` on daemon-relevant launches to
    disable automatic upstream daemon package updates.
  - On an uninitialized execution home, on the first TUI invocation eligible to
    use a managed daemon or an explicit `app-server daemon {start,restart,bootstrap}`
    / `remote-control [start]` route, Wrapper may create the non-clobbering link
    `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current`.
    Existing dedicated selections, symlinked roots, and prior modern/legacy launch
    traces are strictly preserved on ordinary launch; help/version, queries, `--no-daemon`, `--remote`,
    and `daemon update --from-cli` skip preparation.
  - On successful `codex update` operations, Wrapper aligns eligible initialized
    homes to the shared active executable through `standalone/current`. Each
    home retains its upstream process and socket ownership. Wrapper uses upstream
    lifecycle commands to restart a verified running daemon when its executable
    or version differs or its legacy PID path needs migration. Stopped homes
    remain stopped after link alignment. Dedicated package files are retained.
    Synchronization failures are reported per home after activation. See the
    [delivery contract](docs/spec/DELIVERY.md#managed-daemon-synchronization)
    for scope and failure handling.
- **External shared-home binding (`CODEX_SHARED_HOME` / `CODEX_SQLITE_HOME`):**
  - Wrapper owns no profile registry or profile CRUD. When callers supply
    `CODEX_SHARED_HOME`, Wrapper isolates `auth.json` and daemon sockets per
    `$CODEX_HOME`, shares conversations/SQLite/config (`installation_id`,
    `config.toml`, plus explicit opt-in `CODEX_SHARED_EXTRA_FILES` validated
    against sensitive/path-unsafe names), and merges append-only logs
    (`session_index.jsonl`, `history.jsonl`) by exact line equality so thread
    renames are never dropped.
- **External services & future Stage 2 modules:** Independently running services
  (Provider, AppHost, Bridge, legacy runtimes) are never made Wrapper-owned by
  coexistence. Any future Stage 2 module (`Manager`, enhanced `TUI`, `Provider`,
  Ed25519 signed Wrapper/addon bundles) must attach behind explicit opt-in
  contracts ([USER_INTERFACES.md](docs/spec/USER_INTERFACES.md),
  [DELIVERY.md](docs/spec/DELIVERY.md), [BRIDGE.md](docs/spec/BRIDGE.md))
  without becoming a build or runtime prerequisite for standalone `codex`.

---

## 5. Termux Runtime and Adaptation Invariants

- **Prebuilt target:** Release users receive a prebuilt `aarch64-linux-android`
  Wrapper binary with zero Rust/Clang/Cargo prerequisite. All paths derive from
  the live environment (`HOME`, `PREFIX`, `TMPDIR`, `CODEX_HOME`) with zero
  hardcoded user-account paths.
- **Process execution boundary:** Before `exec`, Wrapper constructs a clean
  environment (fencing `LD_*`/preload and unreachable X11/Wayland/WSL browser
  variables), preserves stdin/stdout/stderr/TTY/signals/exit status and raw
  argv byte-for-byte, binds read-only FD 33 (resolver), FD 34 (managed Termux
  config), and read-only FD 35 (temp/UDS root) across `exec`, and supplies
  Android `TZif` timezone data when needed.
- **Protected system & user files:** Wrapper must never create, modify, chmod,
  or delete `$PREFIX/etc/resolv.conf`, user `auth.json`, or upstream SQLite/
  rollout files. Wrapper never invokes, installs, or repairs `bwrap`.
- **Managed FD 34 config & fast path:** Base FD 34 `config.toml` sets only
  `sandbox_mode = "danger-full-access"` and `mcp_oauth_credentials_store = "file"`,
  leaving `thread_unload_delay_secs` to upstream and preserving user/`-c`
  precedence. Managed files are materialized lazily only on creation or content
  mismatch, with zero addon IPC, network probes, or retention pruning on
  ordinary launches.
- **Fail-closed binary adaptation:** Main `bin/codex` adaptation must match
  both `/etc/resolv.conf` (FD 33) and `/etc/codex/config.toml` (FD 34), and must
  fail closed if `"codex-daemon-"` is present without a supported FD 35 patch
  pattern. Companion binaries in `bin/*` are adapted and digest-recorded in
  `generation.meta`.

---

## 6. Qualification and Release Acceptance Invariants

Acceptance requires evidence at the actual boundary crossed; passing an earlier
boundary never implies a later one:

1. **Source & lint** (`cargo fmt`, `clippy -D warnings`, `Cargo.lock` boundary,
   shell lint, tracked Markdown link validation);
2. **Unit & synthetic fault tests** (including umask `0022`/`0077`, installer
   `SIGKILL` recovery, activation journal interruption, mock release provenance);
3. **Linux + Android Bionic CI smoke** (NDK `aarch64-linux-android` bundle &
   16 KiB ELF alignment geometry);
4. **Actual Android/Termux isolated execution** (disposable `HOME`/`PREFIX`/
   `CODEX_HOME`/`TMPDIR` with real official archives, upgrade/rollback, daemon
   lifecycle, PTY, read-only `doctor`, and `CODEX_SHARED_HOME`);
5. **Authenticated / external-client flows** (real account inference, external
   Remote Control pairing — distinct from account-free qualification);
6. **Public publication readback** (exact peeled Git tag commit, release
   metadata, prerelease flag, and downloaded asset byte SHA-256 verification);
7. **Live device activation** (explicitly authorized operator cutover preserving
   rollback generations, accounts, and unrelated running services).
