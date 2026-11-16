# Stage 2 Reintegration Procedure and Implementation Plan

> **Status:** Implementation execution guide & architecture transition plan (derived from [SPEC.md](../../SPEC.md), [DELIVERY.md](../spec/DELIVERY.md), [USER_INTERFACES.md](../spec/USER_INTERFACES.md), [BRIDGE.md](../spec/BRIDGE.md), and inspected preserved sources).
> **Current implementation state:** **Stage 1** is implemented; **Phase 2.1 configuration/CLI groundwork** is implemented but the FD 34 snapshot exit gate is open. Other Stage 2 phases are not qualified. Complete the phase exit gates in order, preserving standalone Wrapper independence.

---

## 1. Ground Rules & Non-Negotiable Constraints

1. **Preserve Stage 1 independence at every phase:**
   - Bare `codex` (and `codex --version`, `-V`, `--help`, `-h`, `help`, `codex update --upstream`, `codex doctor`, `codex wrapper status`) must remain runnable with **zero** dependency on Node.js, Manager, enhanced TUI, Provider, network indices, or Termux:API apps.
   - `crates/wrapper` (`codex` CLI binary) keeps bare runtime fast path: zero addon probes, zero Manager IPC spawns, and zero extra writes/locks/`fsync` calls on unchanged launches when no optional module is enabled.
   - `Cargo.lock` for bare Stage 1 Wrapper currently locks only `codex`, `codex-release-builder`, and `libc` (`=0.2.186`), enforced by [`.github/workflows/wrapper.yml`](../../.github/workflows/wrapper.yml). Any Stage 2 workspace additions (`crates/manager`, `crates/runtime-binding`, TOML/JSON/WS dependencies) must keep bare `crates/wrapper` dependency-isolated or intentionally update the CI workspace check alongside separate package boundaries so Manager/source-builder crates never become runtime or build prerequisites for bare execution.
2. **Authoritative Source Lineage — Do NOT Wholesale-Restore `ebb0b519`:**
   - `origin/main` (`f9c75f55`) tracks only 19 release/index/config files (`release-source-v1`, `release-intent-v1`, `release-frontend-v1`, `provider-source-v1`, `provider-index-v1(.sig)`, `update-index-v1(.sig)`, `update-index-base-v1(.sig)`, `.github/`, `AGENTS.md`, `README.md`) — it has **zero** Rust/frontend/provider source crates.
   - **`rewrite/rust-core` (`c200e676`)** is the authoritative preserved Stage 2 source tree (`crates/core`, `crates/runtime-binding`, `crates/manager`, `crates/release-builder`, `frontend/tui`, `providers/chatgpt`, `transport/tunnel-client`, `.github/scripts`).
   - **Pre-split `ebb0b519` (`rewrite/wrapper` history) trap:** `ebb0b519` branched near `merge-base = 27103ec0` while `c200e676` continued receiving ~70+ commits of security, concurrency, locking, and completeness fixes. Blindly resurrecting files from `ebb0b519` **silently reverts** critical fixes in:
     - `crates/runtime-binding/src/native.rs` (`writer_free_until`, `writer_directories`, structured error codes, retired-record safety checks);
     - `crates/manager/src/{origin,terminal,window,profile,profile_resume}.rs` (`origin::capture`, `target_execution_home`, `launch_lock`, `validate_private_reentry`);
     - `frontend/tui/src/` (`termux_reentry.rs`, `account_observation.rs`, `provider_presentation.rs`, `CODEX_CHAT_MODEL_REENTRY_PID` check);
     - `crates/release-builder/src/provider.rs` (`valid_new_id`, `O_NOFOLLOW`, full ELF checks);
     - `providers/chatgpt/src/` (`termux_mcp_worker.rs`, `termux_shell.rs`, `native_compiler.rs`, `native_completion.rs`).
   - **Porting rule:** Always take implementation logic from `git show c200e676:<path>`, and selectively apply ONLY verified transition fixes from `ebb0b519` (notably `window.rs`'s `--ei com.termux.RUN_COMMAND_SESSION_ACTION 0` fix, `PackageRoot` `bin/codex` runtime matching, and `CODEX_PROFILE_WRAPPER` naming). Note also that actual TUI directory in `c200e676` is `frontend/tui/` (not `frontend/manager-tui/`).
3. **Suffix Removal & Unified Naming Policy (`wrapper` Cutover):**
   - Stage 2 Wrapper, Manager, Release, and Provider management surfaces follow the Stage 1 clean cutover — **no** `Core`, `codex termux`, `minimal-wrapper`, `99b`, `R10` (`helpers/0..2`), `RootCodeModeHost`, or gratuitous `-v1`/`-v2`/`-v3`/`-v4` suffixes:
     - CLI route: `codex wrapper ...` and unified **`codex wrapper provider ...`** (`codex termux` removed; no separate top-level `codex provider`).
     - Env vars: `CODEX_TERMUX_WRAPPER_API`, `CODEX_TERMUX_WRAPPER_ENTRYPOINT`, `CODEX_TERMUX_WRAPPER_ONLY`, `CODEX_PROFILE_WRAPPER`.
     - Identities & descriptors: `wrapper-api`, `schema`, `codex-wrapper`, `package-root`, `termux-fd-remap`, `termux-uds-fd35`, `codex-local-generation`, `codex-activation-state`, `codex-activation-journal`.
     - Stage 2 release/manager/provider surfaces: `update-index`, `update-index-base`, `provider-index`, `codex-release`, `asset-map.json`, `codex-manager-artifact`, `codex-manager-wrapper`, `codex-manager-profile`, `codex-manager-default-profile`, `notifications/config`, `codex-manager-notify`, `codex-update-hold`, `codex-rollback-wrapper-guard`, `codex-provider-payload`, `codex-provider-state`, `termux-codex-bridge-qualification`, `execution.wrapper`, `termux-codex-operator-work`, `operator-work`, `work-route-transfer`, `codex-chatgpt-catalog`, `app-model-catalog`, and unversioned internal IPC/owner identities (`__prepare-runtime`, `__prepare-launch`, `__task-*`, `__terminal-*`, `__app-server-*`, `config`, `catalog`).
   - **No legacy Bridge work-record backward compatibility (`BRIDGE.md` cleanup):** Inside `providers/chatgpt`, `c200e676`'s `v1` -> `v2` -> `v3` work-record chain (`termux-codex-operator-work-v3`, `migrate-inference --target attached-submitted|provider21|provider14`, `config-v1`, `catalog-v1`) existed only for historical downgrade/migration across pre-rewrite iterations. Because `rewrite/wrapper` drops pre-cutover backward compatibility, Phase 2.5 cuts straight to one single suffix-free schema (`termux-codex-operator-work` with `selection.inference = attached|submitted`, plus suffix-free journal/catalog/owner records) and omits `v1`/`v2`/`v3`/`provider14`/`provider21`/`migrate-inference`/`catalog-v1`/`config-v1` legacy parsers and rollback paths (initializing a clean Provider state root rather than carrying old work stores across cutover). External live `@TermuxBridge` MCP connector tool schemas remain aligned during deployment.
   - **`runtime-binding::settings_identity` domain tag:** `c200e676` hashes `OsStr::new("core-system-config-v1")`. Before Phase 2.3 merges, decide intentionally whether to rename that hash domain to `"wrapper-system-config"` (clean cutover, fresh `servers/<hex16>` namespaces) or maintain a transitional server-retire lookup.
4. **Live Termux Device & Service Preservation:**
   - Do **not** overwrite `/data/data/com.termux/files/usr/bin/codex`, stop or mutate running `codex-chatgpt-provider` / `codex-chatgpt-host` / `codex-chatgpt-tunnel`, alter live `@TermuxBridge` registrations, or modify `~/.config/codex/providers/chatgpt.toml` during development or tests.
   - All smoke and integration tests must run in isolated temporary `$HOME`, `$PREFIX`, `$CODEX_HOME`, and `$TMPDIR` roots.

---

## 2. Structural Differences Between `HEAD` and `c200e676` to Reconcile

| Area | Stage 1 `HEAD` (`rewrite/wrapper`) | `c200e676` (`rewrite/rust-core`) | Required Stage 2 Reconciliation |
| --- | --- | --- | --- |
| Generation layout | Single `package-root`: `bin/codex`, `0..N` companions (`bin/*`), `codex-package.json`, `codex-path/`, `codex-resources/`, `browser/{open,manual}/curl`; rejects root `runtime` / `compat` | Flat files: `core` (or `wrapper`), `runtime`, `codex-code-mode-host`, `helpers/{0,1,2}` | Port all Stage 2 consumers (`release-builder publish`, `remote_release`, `runtime-binding`, `manager`, `providers/chatgpt`, CI scripts) to `package-root` (`bin/codex` + `browser/` + `wrapper` + optional `manager` / `tui`) |
| Subcommands | Top-level `update`, `doctor`, `wrapper status`; else passthrough | `termux ...` (`core` env vars), `update --core/--full/--signed/--remote/--local/--build-local` | Keep top-level `update` / `doctor` / `wrapper`; attach `setup`, `--bare`, `--with`, `provider`, `launch`, `profile`, `task`, `notify`, `usage`, `rotation` cleanly under `codex wrapper` |
| Signing & state | `activation-state` (`LOCAL_ZERO` keys); local `--upstream`/`--path`/`--rollback` | Strict Ed25519 non-zero keys (`codex-activation-state-v3`); remote signed indices only | Support **both**: bare `LOCAL_ZERO` `--upstream`/`--path` works with zero keys/network; `--signed` enables Ed25519 chain + rotation without bricking bare generations or cross-mode `--rollback` |
| FD 34 config | Lazy write only on create/mismatch (`sandbox_mode`, `mcp_oauth_credentials_store`); no `requirements.toml` | Always hashes `config/snapshots/<id>/config.toml` + `thread_unload_delay_secs = 0` + Manager fragment + `requirements.toml`, dry-runs `features list` | Bare mode keeps Stage 1 zero-write fast path (`thread_unload_delay_secs = 0` only when enabled or validated safely); snapshots & fragments activate only when Stage 2 modules/shared servers attach |
| FD 36 lease | None (no execution lease / `shared_server` / `shared_layout`) | `execution_lease.rs` duplicates `flock(LOCK_SH)` of physical `CODEX_HOME` onto FD 36 across `exec` | Add only when `manager` / shared-server topology attaches in Phase 2.3 |
| Enhanced TUI | Unpatched upstream `bin/codex` + adaptive binary ELF patch | `frontend/tui` (`native.patch` pinned to `086ccc8...` / `0.167.0`), `tui.rs` checks exact `upstream_package_version` match! | Rebase `frontend/tui` onto `>= 0.169.0`, decouple TUI capability probe from `upstream_package_version`, and keep `codex update --upstream` independent of TUI |

---

## 3. Phased Execution Procedure

### Phase 2.1 — Generic Extension Boundary (`wrapper.toml`, CLI Hooks, FD 34 Snapshots)
**Goal:** Establish the clean configuration, opt-in routing, and FD 34 snapshot extension point inside `crates/wrapper` **without** adding external crate bloat or connecting any Manager/TUI/Provider binaries yet.

1. **Inspect reference code (read-only):**
   - `git show c200e676:crates/core/src/system_config.rs` (snapshot hashing, `flock(LOCK_SH)`, fragment bounds, pruning)
   - `git show c200e676:crates/runtime-binding/src/host_config.rs` & `lib.rs` (`CONFIG_LIMIT = 16384`, `settings_identity`)
2. **Design & Implement:**
   - **Config schema (`~/.local/share/codex/wrapper/wrapper.toml`):** minimal schema storing enabled optional modules (`manager`, `tui`, `provider` booleans or list) and `signed_update = false`. Implement a tiny strict parser directly in `crates/wrapper` (avoiding adding `toml`/`serde` crates to bare `codex` so `Cargo.lock` and startup latency remain minimal).
   - **CLI additions under `codex wrapper`:**
     - `codex wrapper setup [show|--bare|--enable <MODULES>|--disable <MODULES>|--signed-update <true|false>]`
     - `codex wrapper --bare [--] [UPSTREAM_ARGS...]` (session-only bypass of enabled Stage 2 modules)
     - `codex wrapper --with <MODULES> [--] [UPSTREAM_ARGS...]` (session-only module opt-in; fails cleanly with exit `1` or usage `2` if requested module executable is absent — never corrupts bare `codex`)
   - **FD 34 snapshot & fragment extension point:**
     - When no module overrides exist (default Stage 1 / `--bare` / empty `wrapper.toml`), retain the exact Stage 1 zero-write lazy `config/config.toml` path (zero dry-run `features list` spawns, zero snapshot dirs).
     - When enabled modules supply bounded config fragments (`<= 8 KiB`), materialize immutable `config/snapshots/<settings_id>/config.toml` (+ optional bounded `requirements.toml` only when a module requires shared topology constraints), validate with bounded `features list`, bind FD 34 + `flock(LOCK_SH)` over child lifetime, and prune unheld expired snapshots safely.
     - Keep `codex wrapper status --json` accurately reporting active config (`"addons":"none"` when bare, listing enabled modules when active).
3. **Exit Gate (Must Pass Before Phase 2.2):**
   - `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `.github/workflows/wrapper.yml` Android Bionic smoke pass.
   - New unit/E2E tests verify: absent/empty `wrapper.toml` performs **zero** snapshot writes or subprocess probes on `--version` and ordinary launch; malformed `wrapper.toml` fails closed safely; `--bare` ignores configured modules; `--with manager` fails cleanly when no Manager binary exists rather than running unguarded or crashing bare launch.

---

### Phase 2.2 — Signed Release Channel & `codex-release-builder publish`
**Goal:** Add Ed25519 signed channel verification (`codex update --signed`), signed publication (`codex-release-builder publish`), rollback guards, and optional-asset separation while keeping default `codex update` (`--upstream`) 100% independent.

1. **Inspect reference code (read-only):**
   - `git show c200e676:crates/core/src/{remote_release.rs,release_display.rs,rollback_guard.rs,component_metadata.rs,maintenance.rs}`
   - `git show ebb0b519:crates/wrapper/src/remote_release.rs` (`wrapper` vs `core` flat-asset diff)
   - `git show c200e676:crates/release-builder/src/lib.rs` (`publish` command)
   - `git show c200e676:.github/scripts/{release_signing.py,release_publication.py,release_components.py}`
2. **Design & Implement:**
   - **Unify signed payload structure onto `package-root`:** Replace legacy flat file lists (`core`/`runtime`/`helpers/{0,1,2}`) in `publish` and `remote_release` with signed `package-root` inventories (`generation.meta`, `wrapper`, `bin/codex`, `0..N` companions in `bin/`, `codex-package.json`, `codex-path/`, `codex-resources/`, `browser/{open,manual}/curl`, plus optional `manager` and optional TUI helper).
   - **Clean suffix-free names:** Emit/verify `codex-release` (manifest), `update-index` / `update-index-base`, `asset-map.json`, `codex-update-hold`, `codex-rollback-wrapper-guard`, `download-size` without `-v1`/`-v3`/`-v4` or `core`/`held_core_sha256` naming.
   - **OpenSSL Ed25519 verification & key state transitions:**
     - Bootstrap trust seed at `$HOME/.local/lib/codex/wrapper/release-public-key.pem` (bound by pinned key SHA-256 on initial `--signed` enrollment).
     - Verify `release.manifest` + `release.sig` + optional `release-authority.sig` rotation via OpenSSL.
     - Keep Stage 1 `ReleasePublicKey::LOCAL_ZERO` fully supported for `--upstream` and `--path`; prevent silent downgrade from signed keys to unsigned remote channels or corrupting a mixed `current`/`previous` `--rollback` pair (reject or require explicit `--upstream` opt-in).
   - **CLI additions:**
     - `codex update --signed [--version ... | --remote <HTTPS_URL> | --path <DIR>]` (requires `--signed` or `wrapper.toml` `signed_update = true`; default `codex update` keeps Stage 1 `--upstream`).
     - `codex-release-builder publish --generation <DIR> --release-sequence <N> --release-base <HTTPS> --private-key <PEM> --openssl <BIN> --output <DIR>`.
   - **Optional-asset degradation rule (`SPEC.md` §4.2):**
     - Fresh candidate staging/signing still requires all declared assets in `release.manifest`.
     - For an **already-installed** generation, a missing/broken optional `manager` or `tui` asset degrades only when Manager/TUI is invoked or reported in `codex doctor`; ordinary bare `codex`, `codex doctor`, `codex update`, and `codex update --rollback` remain fully operational.
3. **Exit Gate (Must Pass Before Phase 2.3):**
   - Full `codex-release-builder build` -> `publish` -> `codex update --signed --path` / mock HTTPS `--remote` test covering: initial trust seed, key rotation (`release-authority.sig`), tampered signature/manifest/mode rejection, sequence rollback protection, `LOCAL_ZERO` <-> signed boundary behavior, and missing optional Manager non-bricking recovery.
   - All Stage 1 `codex update --upstream` / `--path` / `--rollback` tests remain green offline with zero `release-public-key.pem`.

---

### Phase 2.3 — Optional `codex-manager` & `crates/runtime-binding` Reattachment
**Goal:** Port shared server coordination, execution lease (FD 36), and standalone `codex-manager` (`profile`, `task`, `notify`, `usage`, `rotation`, `launch`) cleanly behind Phase 2.1's `wrapper.toml` boundary.

1. **Inspect reference code (read-only):**
   - `git show c200e676:crates/runtime-binding/Cargo.toml` and `src/{lib,native,host_config}.rs` + `git show ebb0b519:crates/runtime-binding/src/native.rs` (`PackageRoot` diff ONLY — preserve `c200e676`'s security guards!)
   - `git show c200e676:crates/manager/Cargo.toml` and all `src/*.rs` + `git show ebb0b519:crates/manager/src/window.rs` (`--ei` diff ONLY!)
   - `git show c200e676:crates/core/src/{execution_lease.rs,shared_layout.rs,shared_server.rs,native_task.rs,manager_config.rs,manager_dispatch.rs}`
2. **Design & Implement:**
   - **Workspace & CI boundary update:** Re-add `crates/runtime-binding` and `crates/manager` to root `Cargo.toml`. Keep `crates/wrapper/Cargo.toml` free of `serde`/`serde_json`/`tungstenite` (depending at most on lightweight `codex-runtime-binding` if/when needed, which depends only on `libc = "=0.2.186"`). Update `.github/workflows/wrapper.yml` dependency check explicitly to distinguish bare `crates/wrapper` (`codex`) deps from optional `crates/manager` deps (`serde`, `serde_json`, `tungstenite`), rather than allowing silent dependency creep into `codex`.
   - **Runtime-binding unification:** Merge `c200e676`'s `writer_free_until`/`writer_directories`/error-code checks with `package-root` roots (`~/.local/share/codex/wrapper/{servers,catalogs}` and `generations/<id>/bin/codex`); finalize `settings_identity` domain string; keep `thread_unload_delay_secs = 0` enabled for shared-server reuse namespaces.
   - **Manager rebrand & clean IPC contracts:**
     - Cut ALL `c200e676` `codex termux` callbacks/help text in `crates/manager` to `codex wrapper`, and `CODEX_TERMUX_CORE_*` -> `CODEX_TERMUX_WRAPPER_*` (`CODEX_TERMUX_WRAPPER_API = "codex-manager-wrapper"`, artifact marker `codex-manager-artifact`, probe `wrapper_api=codex-manager-wrapper`).
     - Clean suffix-free Manager formats & IPC endpoints (`__prepare-runtime`, `__prepare-launch`, `__task-connection`, `__task-reentry`, `__terminal-*`, `codex-manager-profile`, `codex-manager-default-profile`, `notifications/config`, `codex-manager-notify`).
     - Generate notify hook `["codex", "wrapper", "notify", "emit", "Stop"]`. Evaluate explicit removal of `requirements.toml` generation unless shared SQLite topology strictly still requires setting `sqlite_home` for cross-profile sessions (note Stage 1 `runtime_patch` already preserves `/dev/fd/34/requirements.toml` remapping if needed!).
   - **Wire Manager / `shared_server` / `execution_lease` strictly behind opt-in activation:**
     - Bare `codex` still starts no `shared_server`, opens no FD 36 lease, and spawns zero `codex-manager` processes.
     - Active `manager` module (via `wrapper.toml`, `--with manager`, or `codex wrapper launch|profile|task|notify|usage|rotation`) resolves Manager executable (`<gen>/manager` or configured path), probes/validates API compatibility, prepares FD 34 fragment + FD 36 lease + `shared_layout` / `shared_server` (`setsid()`, `SO_PEERCRED`), and attaches `codex wrapper doctor` optional Manager diagnostics when present.
3. **Exit Gate (Must Pass Before Phase 2.4):**
   - Full `codex-runtime-binding` + `codex-manager` test suite (ported from `c200e676` to `package-root` / `bin/codex` / `codex wrapper`) passes under isolated `$HOME`/`$TMPDIR`.
   - Bare `codex --version` and bare `codex` run cleanly even when `manager` binary is deleted, corrupted, or disabled in `wrapper.toml`.
   - Active `--with manager` tests verify multi-profile `CODEX_HOME` isolation, shared server reuse & snapshot namespace isolation (`FD 34`), FD 36 lease inheritance, and Termux AM notification dry-run/JSON formatting.

---

### Phase 2.4 — Optional Enhanced TUI (`frontend/tui`) & Upstream Upgrade Immunity
**Goal:** Reintroduce opt-in source-patched TUI conveniences (`/switch`, quota switching, permission picker adaptations, clipboard write confirmation, foreground terminal FD binding) without ever tying `codex update --upstream` to TUI version pins.

1. **Inspect reference code (read-only):**
   - `git show c200e676:frontend/tui/` (`README.md`, `apply.py`, `native.patch`, `check_command_boundary.py`, `check_upstream_features.py`, all `src/*.rs` files including `termux_reentry.rs`, `account_observation.rs`, `provider_presentation.rs`, `termux_clipboard.rs`, `termux_terminal.rs`, `termux_backend_catalog.rs`)
   - `git show c200e676:crates/core/src/tui.rs` & `c200e676:crates/release-builder/src/source*` (`syn` permission rewriter + `native_route.patch`)
2. **Design & Implement:**
   - **Rebase TUI patches onto current upstream (`>= 0.169.0`):** `c200e676:frontend/tui/native.patch` still carries `# codex-upstream-base 086ccc833bc9cf01024ad2dd6af7dbd421b6bf9a` (`0.167.0`). Rebase `native.patch` + `source/syntax` onto current upstream source and update `frontend/tui` tests cleanly.
   - **Fix `upstream_package_version` coupling bug:** Replace `c200e676:tui.rs`'s hardcoded exact `upstream_package_version` requirement with capability/API probe matching (`--termux-backend-capabilities` -> `codex-termux-resolved-startup`, `CODEX_TERMUX_BACKEND_STARTUP=resolved`). When `tui` is enabled in `wrapper.toml` but incompatible or absent for a newly installed `codex update --upstream` version, fail clearly at `--with tui` OR fall back cleanly per explicit user setting without breaking `bin/codex` launch or `codex update`.
   - **Cut over all callbacks in `frontend/tui`:** `codex termux ...` -> `codex wrapper ...`, `CODEX_PROFILE_CORE` -> `CODEX_PROFILE_WRAPPER`, `CODEX_TERMUX_CORE_*` -> `CODEX_TERMUX_WRAPPER_*`; retain `c200e676`'s `termux_reentry.rs` + `CODEX_CHAT_MODEL_REENTRY_PID` guards.
   - **ELF patching:** Run `codex-release-builder`'s adaptive `termux-fd-remap` + `termux-uds-fd35` over the built optional TUI binary (stored cleanly inside `package-root` / optional TUI path rather than legacy `helpers/2`).
   - **Separate builder/CI job:** Keep `syn` / `proc-macro2` / `quote` / upstream Rust checkout cleanly isolated in an optional source-build workflow/binary so standard `crates/wrapper` and `crates/release-builder build` stay lightweight.
3. **Exit Gate (Must Pass Before Phase 2.5):**
   - `frontend/tui` Python checks (`test_apply.py`, `test_command_boundary.py`, `test_upstream_features.py`) pass on the rebased upstream source.
   - End-to-end tests verify: bare `codex` runs unpatched `bin/codex` cleanly; `codex --with tui` passes `termux-am-socket` clipboard confirmation, `/switch`, permission picker full-access mapping, and foreground terminal FD bind; `codex update --upstream` onto a newer mock upstream release never fails solely due to missing enhanced TUI.

---

### Phase 2.5 — Optional `providers/chatgpt` & `transport/tunnel-client`
**Goal:** Reattach ChatGPT Provider (`codex-chatgpt-provider`, `codex-termux-mcp-worker`) and Go tunnel client (`tunnel-client`) under `codex wrapper provider ...` with complete continuity for `@TermuxBridge`.

1. **Inspect reference code (read-only):**
   - `git show c200e676:providers/chatgpt/` (`Cargo.toml`, `README.md`, `LINUX_APP.md`, `qualification.schema.json`, `scripts/*`, `src/*.rs`)
   - `git show c200e676:transport/tunnel-client/` + `git show c200e676:scripts/build_tunnel_client.py`
   - `git show c200e676:crates/core/src/provider.rs` + `git show c200e676:crates/release-builder/src/provider.rs` + `git show c200e676:.github/scripts/{provider_signing,provider_publication}.py`
2. **Design & Implement:**
   - **Port hardcoded paths & config in `providers/chatgpt/`:**
     - `src/main.rs`, `src/operator_native.rs`: replace `.local/share/codex/core`, `.local/lib/codex/core/generations`, and `program.file_name() == "runtime"` (`operator_native.rs:422`) with `.local/share/codex/wrapper`, `.local/lib/codex/wrapper/generations`, and `bin/codex`.
     - `src/config.rs`: adopt `execution.wrapper` cleanly (explicit migration guidance if `~/.config/codex/providers/chatgpt.toml` still specifies `execution.core` on the live host).
     - Internal callbacks: `core termux __terminal-auto-show-v1` -> `wrapper __terminal-auto-show`, `__app-server-{catalog,default}` -> `wrapper ...`, env scrub `CODEX_TERMUX_WRAPPER_*`.
   - **Wrapper CLI routing (`codex wrapper provider ...`):**
     - Expose `codex wrapper provider install|update|status|rollback|chatgpt ...` behind Phase 2.1/2.2/2.3 modules (using suffix-free `codex-provider-payload`, `codex-provider-state`, `provider-index`, `termux-codex-operator-work`, `operator-work`, `work-route-transfer`, `codex-chatgpt-catalog`, `app-model-catalog`, and `config`/`catalog` records without retaining `v1`/`v2`/`v3`/`provider14`/`provider21`/`migrate-inference` backward compatibility).
   - **Independent Provider CI:** Restore a dedicated Provider + `tunnel-client` CI workflow/job (Go + NDK Clang + Rust `1.98+` + Node) triggered only on `providers/chatgpt/**`, `transport/tunnel-client/**`, or `crates/runtime-binding/**`, keeping `Wrapper CI` isolated.
   - **Live service migration checklist (explicit operator step, never automatic):** Verify disposable `$HOME` PRoot/CDP/MCP readback first, prepare `~/.config/codex/providers/chatgpt.toml` (`execution.wrapper`) and a clean Provider state directory for the suffix-free schema, then restart `codex-chatgpt-*` services intentionally without disturbing live `@TermuxBridge` tunnel identity.
3. **Exit Gate (Phase 2.5 Complete; see Phase 2.6 below):**
   - `cargo test` in `providers/chatgpt/` + `codex-release-builder` provider packaging + Ed25519 sign/verify readback + `qualification.schema.json` validation pass in disposable roots.
   - Go `tunnel-client` cross-compile (`android/arm64`) succeeds cleanly.
   - Live `@TermuxBridge` / existing running services remain untouched until explicit post-qualification cutover.

---

## 4. Phase 2.6 — Continuous Upstream Adoption Controller (Planned, Not Implemented)

**User request, 2026-11-16:** Treat recurring official-upstream adoption as a
Stage 2 delivery work item. It must **extend** the single Stage 1
`codex update --upstream` / release-builder owner, not create another CLI,
version selector, updater, daemon, account dependency, or signature authority.
The ordinary standalone update and rollback remain usable independently of this
automation and of every optional module.

**Prerequisites:** close the actual-device Stage 1 operating-acceptance gaps and
the Phase 2.1 FD 34 snapshot gate; coordinate with signed-channel Phase 2.2
before enabling publication. Design can proceed alongside optional modules;
base upstream **qualification and publication must never wait for TUI, Manager
or Provider**. The automation is not a license to skip any active gate.

### Implementation work order

1. **Discover (read-only):** scheduled (e.g. daily) and manual-dispatch
   official stable release discovery via the supported OpenAI release metadata.
   Resolve an exact version, metadata/descriptor digest, archive URL, archive
   SHA-256, source SHA, wrapper/release-builder SHA, toolchain/NDK and binary
   patch policy. Reject prereleases, missing official digests, divergent
   descriptors, unknown schema, downgrades and changed inputs; deduplicate
   on the complete immutable input tuple. Never infer a target from a CI job
   title, cached tag, mutable `latest` alias or a previous failed run.
2. **Prepare (least privilege):** fetch and digest-check the *complete official*
   `codex-package-aarch64-unknown-linux-musl.tar.gz` including companions and
   resources; invoke the existing rule-based `termux-fd-remap` /
   `termux-uds-fd35` builder, rejecting unsupported site layout with
   actionable evidence. Use pinned toolchains and prepared exact commit; no
   automatic source patching, blind ELF offset heuristics, or fallback to an
   unadapted official binary.
3. **Qualify (read-only):** test the standalone base lane before any optional
   composition: unit/negative tests, Android AArch64 packaging, Ubuntu ARM+Bionic
   smoke, installer rollback faults, real Android/Termux isolated HOME/PREFIX/
   CODEX_HOME/TMPDIR execution with the new official archive, doctor, fast
   help/version, app-server/remote-control boundaries, upgrade and rollback.
   Fail closed when the physical-device gate is required but unavailable.
   Additional TUI/Manager/Provider lanes qualify their own exact artifacts;
   their failure is not a veto of a valid independently usable base.
4. **Publish/promote (isolated authority):** separate unprivileged discovery
   and validation from the signer/publisher. Give PR and scheduler jobs
   `contents: read`; grant content-write and signing keys only to an
   environment-gated immutable-input promotion job, not to pull requests,
   arbitrary uploaded fixtures or self-hosted device smoke. Verify published
   archive bytes, manifest/index signatures (when enabled), SHA-256 and
   canonical download URLs by readback. Use distinct base/full qualification
   provenance and never mistake a skipped release step for a published product.
5. **Update/recovery:** leave the installed generation, sessions, original
   service identities, auth homes and previous/rollback pointers untouched by
   CI. Activation is a separate controlled device operation. On discovery,
   patch, qualification, download or publication failure retain the last
   admitted version, report the exact failed input/stage and retry only with a
   reconciled original receipt. Reuse unchanged successful expensive proofs
   only when source/input/toolchain/test contract are identical; never reuse a
   failed or incomplete gate.

**Exit criteria:** exercise no-new-release, same-input duplicate schedule,
changed official digest, malformed descriptor, absent companion, unexpected
ELF relocation, failed download, interrupted installation, stale proof,
signed-index verification failure, optional-module unavailable, device offline,
failed readback and successful newer release. The workflow must publish a
traceable accepted tuple, and a failed candidate must not change an active
device or existing release pointer. Record exact CI run IDs and physical
device receipts in `CURRENT.md`. No scheduled publication is currently
approved or running.

---

## 5. CI Verification Environment — Reviewed Options (2026-11-16)

| Environment | What it truly proves | Gap / cost | Decision |
| --- | --- | --- | --- |
| Current `ubuntu-24.04-arm` with pinned Android `linker64` + Bionic libs | Native AArch64 Bionic ELF loading, installer/bootstrap boundaries, isolated file and config behavior, deterministic faults | Linux kernel and synthetic Termux `/data/data` layout; not Android kernel, SELinux, Termux app lifecycle or genuine UI | **Retain as fast, mandatory PR gate** |
| Android emulator on x86 GitHub runners (e.g. AVD with hardware acceleration) | Real Android OS, package installation, selected app and UI paths | AArch64 upstream binaries require a matching guest architecture; default fast x86_64 emulator is not an AArch64 runtime, ARM translation support is not guaranteed. Higher setup/time cost; must prove Termux APK + shell route before reliance | Optional exploratory lane, never sole ARM64 admission |
| Managed physical-device testing service (e.g. Firebase Test Lab) | Android/SELinux/real device signals, UI and APK instrumentation | Expects app APK + instrumentation/Robo harness; not a drop-in generic shell or Termux CLI runner. Service quotas, credentials and packaging work | Feasibility prototype only |
| Dedicated physical AArch64 Android device with isolated Termux test installation via controlled ADB/relay | Actual Termux UID, kernel/SELinux/path constraints, PTY/TUI, daemon/remote-control, real official archive upgrades/rollback | Hardware upkeep and trusted runner isolation; ordinary public-PR code cannot be executed on a phone carrying personal accounts | **Preferred release-admission tier**; restricted exact-SHA artifacts and explicitly authorized jobs |
| Existing personal handset as a manual acceptance target | Highest fidelity to currently deployed environment | Auth, live services, account ownership and state risk; uptime cannot be assumed | Manual acceptance only, never unattended PR runner |

**Actual device gap observed on 2026-11-16:** In a genuinely short
isolated Termux `CODEX_HOME`, official `0.169.0` upgrades/rollback and
Wrapper diagnostics passed, but both direct app-server and
`app-server daemon start` failed with `SUN_LEN`; default `codex` PTY
also failed, whereas `--no-daemon` entered an unauthenticated PTY screen.
The existing pre-cutover Wrapper executable reproduced the daemon failure
under the same isolated conditions. The physical promotion gate therefore
**must test default PTY and daemon lifecycle**, not treat `--no-daemon`
fallback, help/status or a passed Linux+Bionic smoke as equivalent. Isolate
the upstream-vs-FD35/Termux cause before granting operating admission.

**Recommended topology:** keep the present 3 GitHub jobs
(`checks` + `android-package` + `android-smoke`) for every PR.
Add a **restricted, dedicated Android/Termux physical-device gate** for
pre-release / promotion: pull only checksum-pinned artifacts from a passed
exact-source build into owned isolated directories, run deterministic smoke,
record OS API, architecture, candidate SHA, archive digest, exit codes,
timeout, stderr, and rollback equality. No workflow secrets or ChatGPT
accounts on the test device; no arbitrary fork PR code, no implicit activation
of the user's installed `codex`, and no dependence on device availability
for ordinary source PR CI. If a physical device is unavailable, the promotion
gate remains **unverified**, not automatically green.

Investigate emulator/device-farm options only against the same acceptance
matrix; do not add an expensive duplicate CI job merely to report another
`aarch64` cross-compile. The existing Ubuntu/Bionic test is an excellent
ABI compatibility layer, not the real-device operating acceptance by itself.

---

## 6. Pre-Stage-2 Checklist (Already Completed on `rewrite/wrapper`)

- Unified naming (`wrapper`, `codex-wrapper`, `codex-local-generation`, `codex-activation-state`, `package-root`, `termux-fd-remap`, `termux-uds-fd35`) across code, tests, [`install.sh`](../../install.sh), [`.github/workflows/wrapper.yml`](../../.github/workflows/wrapper.yml), and documentation.
- Removed legacy `99b` / `R10` / `helpers/{0,1}` / `RootCodeModeHost` / `upstream_doctor` / version-pinned tests.
- Passed local checks (`ruby` Psych YAML, `sh -n` / `shellcheck`, `cargo fmt`, `clippy`, `cargo test --workspace --locked`, isolated AArch64 mock-ELF E2E) and GitHub Actions `Wrapper CI` (`checks`, `android-package`, `android-smoke`).
