# Product state and completion roadmap

SPEC.md and its linked normative annexes own behavior and component boundaries.
AGENTS.md owns development principles
and must be reread after recovery. This file distinguishes source implementation,
automated proof, actual Android execution, connected-client proof, public signed
deployment and device activation. None substitutes for the next boundary.

## Stage 1: Standalone Wrapper (`rewrite/wrapper`, 2026-11-16)

### Status and scope
Stage 1 decouples `crates/wrapper` (`codex`) and `crates/release-builder` (`codex-release-builder`) into a standalone, zero-addon Termux compatibility wrapper aligned with [SPEC.md](SPEC.md) Sections 1–5 (`Stage 1` vs `Stage 2` architecture).
- `Cargo.toml` contains only `crates/wrapper` and `crates/release-builder`, and `Cargo.lock` locks only `codex`, `codex-release-builder`, and `libc` (`0.2.186`). Zero non-wrapper addon crates, providers, TUI patches, or legacy CI/qualification scripts remain tracked on `rewrite/wrapper`.
- Active Stage 1 CLI surface: `codex update [--upstream] [--version <MAJOR.MINOR.PATCH>] | --path <FILE_OR_DIR> | --rollback | --help`, `codex wrapper status [--json]`, `codex doctor [--color | --json]`, and direct upstream passthrough including `codex app-server daemon ...` and `codex remote-control`. Removed flags (`--build-local`, `--local`, `--signed`, `--remote`) exit with status `2`; `codex-release-builder` exposes only `build` (`fetch`/`prepare`/`publish` exit `2`).
- Adaptive Termux binary patching (`crates/release-builder/src/runtime_patch.rs`): applies unified equal-length path remaps (`termux-fd-remap`, FD 33/34) plus adaptive UDS FD 35 relocation (`termux-uds-fd35`) across `0..N` matching static AArch64 ELFs under `bin/*` (`bin/codex`, optional `bin/codex-code-mode-host`, optional `bin/codex-exec-server`), preserves static AArch64 `codex-path/rg` when present (otherwise installs wrapper script), preserves `codex-resources/*`, and writes a single `codex-local-generation` (`package-root`) `generation.meta` with `wrapper_api_identity = wrapper-api`, `persistent_schema_identity = schema`, `creation_metadata = codex-wrapper`, and `termux-browser-open` / `termux-browser-manual` under `browser/{open,manual}/curl`, without `manager_artifact_digest`, `upstream_doctor`, or legacy fallback readers (`v1`/`v2`/`v3`, `99b`, `R10` helper bridge, or `compat/` layouts).
- Standalone runtime setup & state: fast `--version`/`-V`/`--help`/`-h` / `help` path bypassing `daemon_settings::ensure` (which manages only `$CODEX_HOME/app-server-daemon/settings.json` `autoUpdateEnabled: false`); lock-free FD 33 `resolv.conf` cache hit; lazy FD 34 `config.toml` (`# codex-termux-wrapper`, `sandbox_mode = "danger-full-access"`, `mcp_oauth_credentials_store = "file"`); `timezone::configure` (`TZ` TZif); FD 35 temp root; `codex-activation-state`, `codex-activation-journal`, and `downloads/` under `$HOME/.local/share/codex/wrapper/` (serialized via directory `flock` on the state root); AArch64 `SYS_renameat2` (`276`) atomic `rename_noreplace`; TLS/TMPDIR/BROWSER/X11-Wayland-WSL environment fence; `codex doctor --json` (`schema_version: 1`, `layout: "package-root"`); and `--sandbox` mode guard.

### Verified boundaries — post-cutover (`wrapper` naming & suffix cleanup, 2026-11-16)
- **Workspace format, lint, tests & release binaries (`aarch64` Termux host):**
  - `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `(umask 0022 && cargo test --workspace --locked)` exited `0` (`codex`: `25 passed; 0 failed`, `codex-release-builder`: `6 passed; 0 failed` including unit & `runtime_patch::tests`, `0` doc-test failures; unit/synthetic tests cover behavior directly without hardcoded version pins).
  - `cargo build --release --locked -p codex -p codex-release-builder` built `target/release/codex` (`1,302,024` bytes) and `target/release/codex-release-builder` (`760,488` bytes).
- **Installer, workflow, dependency & documentation checks:**
  - `ruby` (`4.0.9` with `libyaml` `0.2.5-5`) YAML validation (`Psych.parse_file` and `YAML.safe_load`) on `.github/workflows/wrapper.yml`, `sh -n install.sh`, `shellcheck -S style install.sh` (plus CI `shellcheck install.sh`), `Cargo.toml`/`Cargo.lock` dependency check (`codex`, `codex-release-builder`, `libc`), and markdown relative-link check passed.
- **GitHub Actions `Wrapper CI` & `wrapper` GitHub Pre-release (`rewrite/wrapper`):**
  - Push runs (`316d0c09` `43781905741`, `bc816505` `43784796608`, `c0abf7a6` `43786603428`) and manual `workflow_dispatch` release dispatch (`bc816505` `43784904077`, `publish_release=true`, `release_tag=wrapper`, `prerelease=true`) passed all active jobs (`Build and package aarch64-linux-android bundle`, `Workspace lint, tests, and docs`, `Smoke-test Android Bionic binary and install.sh`, and `Publish wrapper GitHub Release`).
  - Published prerelease `wrapper` (<https://github.com/humtr/codex/releases/tag/wrapper>) containing `codex-wrapper-aarch64-linux-android.tar.gz`, `.sha256`, `install.sh`, and `SHA256SUMS` (refreshed via `workflow_dispatch` so packaged `README.md`/`SPEC.md` match current `rewrite/wrapper`).
- **Isolated synthetic `$HOME`/`$PREFIX` E2E smoke test (disposable temp roots on Android AArch64, with system `curl`/`openssl`/`gzip`/`tar`/`sh` and TLS cert copied into synthetic `$PREFIX`):**
  - `install.sh --binary target/release/codex --prefix "$SYN_PREFIX" --skip-upstream` installed `$SYN_PREFIX/bin/codex`, `codex update --help` and `codex wrapper --help` exited `0`, uninitialized `codex wrapper status --json` exited `1` (`"status":"uninitialized"`, `"addons":"none"`), and `--sandbox read-only --version` and `sandbox linux true` exited `2`.
  - Using minimal static AArch64 mock binaries containing `/etc/resolv.conf` and `/etc/codex/config.toml` sites, `codex update --path pkg-1.0.0.tar.gz` patched and activated generation `local-*` (`generation.meta` `codex-local-generation`, `patch_policy_id: termux-fd-remap`, `wrapper-api`, `schema`, `codex-wrapper`, `termux-browser-open`/`manual`; `activation-state` `format=codex-activation-state`; FD 34 `config.toml` `# codex-termux-wrapper`), `codex --version` printed `codex-cli 1.0.0`, `codex wrapper status --json` reported `"layout":"package-root"`, `codex doctor --json` exited `0` (`"schema_version":1`, `"layout":"package-root"`), upgrade to `2.0.0` + `codex update --rollback` restored `codex-cli 1.0.0`, and `codex-release-builder build` -> `codex update --path <built-gen>` activated `"has_code_mode_host":false` (`codex-cli 3.0.0`).
- **Not yet completed post-cutover:**
  - Live remote `curl | sh` smoke test from `https://github.com/humtr/codex/releases/download/wrapper/install.sh` on Termux (only offline synthetic `install.sh --binary` + CI Bionic `install.sh --skip-upstream` tested so far).
  - Live-device `/data/data/com.termux/files/usr/bin/codex` replacement (left untouched on the prior binary during workspace cleanup) and external remote client pairing over `codex remote-control`.
  - Promotion of `rewrite/wrapper` to `main` (explicitly deferred until final sign-off).

### Pre-cutover baseline evidence (before `minimal-wrapper` -> `wrapper` cleanup, 2026-11-16)
The checks below were executed on `rewrite/minimal-wrapper` prior to removing `-v1`/`-v2`/`-v3`/`99b`/legacy-compat code paths and renaming CI/installer assets from `minimal-wrapper` to `wrapper`; they serve as prior baseline evidence against real upstream `Codex 0.169.0` and live device services rather than post-cutover proof of the renamed artifacts:
- **Pre-cutover GitHub Actions & `minimal-wrapper` prerelease:** `.github/workflows/minimal-wrapper.yml` passed on commit `e592ade4` (runs `43728935017`, `43758752360`) and `b84b9110` (`43759096273`), publishing prerelease `minimal-wrapper` (`codex-minimal-wrapper-aarch64-linux-android.tar.gz`, `874,663` bytes), and passed isolated `curl | sh` bootstrap of upstream `Codex 0.169.0` and Linux host CI tests.
- **Pre-cutover live Termux device verification (`1,326,448`-byte prior binary SHA-256 `b6611c3a...` with real `0.169.0`):** verified bare PTY TUI launch, `app-server --managed-daemon` (`fd/33`–`fd/35`, `TZ` TZif, `SSL_CERT_*`, `TMPDIR`, `settings.json`), `codex doctor`, `codex exec`, and full `codex remote-control start --json` / `pair --json` / `stop --json` + `app-server daemon disable-remote-control` / `start` restore while leaving running `codex-chatgpt-provider`, `codex-chatgpt-host`, and `codex-chatgpt-tunnel` services undisturbed.
- **Pre-cutover isolated upstream lifecycle tests:** verified real upstream `0.167.0 -> 0.169.0` `--upstream` update, `--rollback`, and fast `--version` zero-write behavior.

### Naming Cutover (`minimal-wrapper` -> `wrapper`) and Promotion to `main`
The unified `wrapper` naming and suffix-free internal formats are now implemented in source, CI, installer, and normative specs on `rewrite/wrapper` (renamed locally from `rewrite/minimal-wrapper`) ahead of promotion to `main`:
- **CI & release artifacts:** `.github/workflows/wrapper.yml` (`Wrapper CI`, concurrency group `wrapper-${{ github.ref }}`, artifact `wrapper-android`, release tags `wrapper`, `wrapper-*`, `v*`), `install.sh` (`DEFAULT_RELEASE_TAG='wrapper'`, `codex-wrapper-aarch64-linux-android.tar.gz` + `.sha256`), and `README.md` URLs are cut over; promotion to `main` remains deferred until merge.
- **Unified formats:** `codex-local-generation`, `package-root`, `termux-fd-remap`, `termux-uds-fd35`, `wrapper-api`, `schema`, `codex-wrapper`, `termux-browser-open`, `termux-browser-manual`, `codex-activation-state`, `codex-activation-journal`, `downloads/`, `# codex-termux-wrapper`, and `doctor --json` `schema_version: 1`.

### Stage 1 operational requalification (2026-11-16, actual Termux device)

A **separate owned disposable** checkout and HOME/PREFIX/CODEX_HOME/TMPDIR
were used on an actual Android AArch64 Termux device (Android API 37); the
installed `$PREFIX/bin/codex`, credentials, profiles, existing Provider/Bridge
services, and production generation pointers were **not** modified.

- Checked out `work/wrapper-stage1-operational-20261116` at
  `a224f223` for the native release build: `cargo build --release --locked`
  compiled `codex` and `codex-release-builder`; `install.sh --binary
  ... --skip-upstream` installed the binary into an isolated PREFIX. Upstream
  **real official archives** were verified and adapted by the candidate
  Wrapper: `0.167.0 -> 0.169.0`, `codex --version` showed both exact
  versions, `codex update --rollback` returned to `0.167.0`, and a repeat
  `0.169.0` update succeeded. The generation layout was `package-root`.
- Physical device CLI evidence: bare help and `exec --help`,
  `app-server daemon --help`, `remote-control --help` passed;
  unsupported Linux `--sandbox read-only` / `sandbox linux` returned
  exit `2`; `codex wrapper status --json` reported active `0.169.0`
  and `addons=none`. Wrapper section of `codex doctor --json` was
  `healthy` with both runtime and Code Mode host healthy; *combined*
  doctor was `unhealthy` because the intentionally empty isolated
  `CODEX_HOME` has **no authentication**, not because Wrapper generation
  failed.
- Installer negative proof ran on the same actual Termux device using the
  **GitHub-updated installer** (after SHA-256 checked-copy backup replaced
  hard-link backup): offline forced failure **after binary swap** restored
  the previous bytes exactly (`cmp`), the failed *first install* removed
  the candidate, and successful `--skip-upstream` committed cleanly.
  Rejected symlink/stale-lock tests are included in the Android Bionic CI job.
- **Open release gate:** an isolated `codex app-server daemon start`
  against the adapted `0.169.0` returned `1`:
  `path must be shorter than SUN_LEN` from the managed daemon. This
  reproduced with a short physical `CODEX_HOME` (42 bytes; advertised
  control socket 85 bytes), short TMPDIR, and real Termux PREFIX;
  `daemon stop` returned `notRunning` without changing an existing
  service. The exact failed producer path / upstream-vs-adaptation root
  cause is **not established**. Help passthrough does not qualify daemon
  lifecycle. Actual remote-control start/pair/stop and external client
  pairing on this final source are also unverified; no authorized user
  account was copied into the isolated fixture.
- **Operating verdict:** standalone installation, official-upstream
  update/rollback, adaptation, and recovery boundaries pass on a physical
  device, but **full Stage 1 operating acceptance is not complete** while
  managed daemon lifecycle remains broken and connected-client proof
  remains outstanding. Do not represent a synthetic Bionic CI job or the
  auth-free isolated doctor as proof of the missing stages.
- `install.sh` failure rollback is caught-exit/signal safe, not a
  cross-power-loss transaction; after SIGKILL/power loss, an owned
  `.codex.previous.<pid>` may remain behind a stale install lock and
  requires inspected manual recovery. This is explicitly documented in
  `README.md` and independently exercised by the new CI test fixture.

The Stage 2 recurring official-upstream adoption controller and CI
environment comparison are now documented in `docs/plan/STAGE2_PLAN.md`;
no scheduler, automatic publication or production runtime deployment occurred.

### Stage 2.1 — Optional settings and CLI admission groundwork (2026-11-16)

Implemented via [PR #31](https://github.com/humtr/codex/pull/31), without
attaching a Manager, enhanced TUI, Provider or additional runtime dependency:
- Bounded, fail-closed `wrapper.toml` parser under the Wrapper state root;
  symlink-safe read, atomic serialized setup updates, and safe first-run
  creation of missing state directories.
- `codex wrapper setup [show|--bare|--enable|--disable|--signed-update]`,
  invocation-local `--bare` bypass and `--with` module validation.
  Configured but unavailable modules fail before upstream execution.
- `codex wrapper status` reports configured module selection. Bare upstream
  `--version` / `--help` retain the zero-addon/zero-write fast path.
  The stored `signed_update` opt-in is not used by Stage 1 update transport.

**GitHub source evidence:** PR #31 HEAD `8ed4703f4d559790b9c0737259500b188705c147`,
[Wrapper CI run 43789749536](https://github.com/humtr/codex/actions/runs/43789749536):
workspace format/Clippy/tests/docs, AArch64 Android package build and
real Android/Bionic installer/CLI smoke passed. No live device installation,
service restart, signed publication or connected-client proof occurred.

**Remaining Stage 2.1 exit gate:** immutable FD 34 fragment snapshots and
their bounded `features list` validation, with a qualified module producer
and lifetime/cleanup tests. Phases 2.2–2.5 remain unstarted.

### Stage 2 Plan (Documented — Phases 2.1–2.5 pending qualification)
See [docs/plan/STAGE2_PLAN.md](docs/plan/STAGE2_PLAN.md) for the verified phase-by-phase reintegration procedure, `c200e676` vs `ebb0b519` regression protections, suffix-removal schema rules, live-device safety checks, and verification gates. Summary sequence:
1. **`wrapper.toml` & Generic Extension Contract (Phase 2.1):**
   - Introduce `~/.local/share/codex/wrapper/wrapper.toml`, `codex wrapper setup`, `codex wrapper --bare`, and `codex wrapper --with <MODULES>`, plus conditional FD 34 system configuration snapshots (`config/snapshots/<id>/`) while keeping bare `codex` / `codex --version` zero-write and free of addon crates or IPC.
2. **`--signed` Ed25519 Channel Verification & Publication (Phase 2.2):**
   - Wire Ed25519 signed manifest/key verification (`release.manifest`, `release.sig`, `release-authority.sig`, `release-public-key.pem`) and `publish` over unified `package-root` trees into `codex-release-builder` and `codex update --signed` (`update-index-base` / `update-index` / `codex-release`), preserving bare `LOCAL_ZERO` `--upstream` updates and degrading missing optional assets at the addon boundary.
3. **Optional `codex-manager` & `runtime-binding` Reattachment (Phase 2.3):**
   - Port `crates/runtime-binding` and `crates/manager` from `rewrite/rust-core` (`c200e676`, plus `ebb0b519` `--ei` and `bin/codex` adapters) behind `wrapper-api` (`codex wrapper launch|profile|task|notify|usage|rotation`), rebranding all `codex termux` callbacks and `CORE` env vars to `codex wrapper` and preserving dependency isolation on bare `crates/wrapper`.
4. **Optional Enhanced TUI (`frontend/tui`, Phase 2.4):**
   - Rebase `frontend/tui` from `c200e676` (`086ccc8...` / `0.167.0`) onto current upstream (`>= 0.169.0`), decouple capability probing from `upstream_package_version`, apply adaptive `runtime_patch`, and attach opt-in via `wrapper.toml` / `--with tui`.
5. **Optional `chatgpt` Provider & Tunnel Reattachment (Phase 2.5):**
   - Port `providers/chatgpt` and `transport/tunnel-client` from `c200e676` to `wrapper` roots / `bin/codex` / `execution.wrapper` behind `codex wrapper provider ...` with an independent Provider CI gate and non-disruptive live `@TermuxBridge` cutover.

### Decoupled Modules Index & Stage 2 Baselines
All decoupled Stage 2 components are absent from `rewrite/wrapper` and preserved in Git history:
- **Reference git baselines:**
  - `rewrite/rust-core` (`c200e676`): **authoritative preserved Stage 2 source tree** (`crates/{core,manager,release-builder,runtime-binding}`, `frontend/tui/`, `providers/chatgpt/`, `transport/tunnel-client/`, `.github/scripts/`).
  - Pre-split `ebb0b519` (in `rewrite/wrapper` history): contains useful `core -> wrapper` transition adapters (`window.rs --ei`, `PackageRoot` `bin/codex` acceptance, `CODEX_PROFILE_WRAPPER`), but **silently reverts ~70+ `c200e676` security/completeness fixes** — never restore wholesale (see [docs/plan/STAGE2_PLAN.md](docs/plan/STAGE2_PLAN.md)).
  - `origin/main` (`f9c75f55`): 19 release-index/caller/workflow files for the `0.167.2` public lane (`release-source-v1`, `release-intent-v1`, `release-frontend-v1`, `provider-source-v1`, `provider-index-v1(.sig)`, `update-index-v1(.sig)`, `update-index-base-v1(.sig)`, `.github/`, `AGENTS.md`, `README.md`); contains no source crates.
- **Removed from `rewrite/wrapper` (recoverable via `git show c200e676:<path>`):**
  - Workspace & addon trees: `crates/manager/`, `crates/runtime-binding/`, `frontend/tui/`, `providers/chatgpt/`, `transport/tunnel-client/`, `bootstrap/`, `install-online.sh`, `release/`, `scripts/`, legacy `.github/` workflows/scripts, `docs/development/`, `AUDIT.md`, `PRO_HANDOFF.md`, `WORK_SCOPE_REVIEW.md`.
  - Pre-split wrapper/builder modules removed from `crates/core/src/` (or `ebb0b519:crates/wrapper/src/`): `execution_lease.rs`, `local_source.rs`, `manager_config.rs`, `manager_dispatch.rs`, `tui.rs` / `manager_tui.rs`, `manual_update.rs`, `native_task.rs`, `provider.rs`, `release_display.rs`, `remote_release.rs`, `rollback_guard.rs`, `shared_layout.rs`, `shared_server.rs`, `system_config.rs`, `component_metadata.rs`, `current_alias.rs`, `termux_version.rs`, `update_edition.rs`; and removed from `crates/release-builder/`: `src/fixtures/`, `src/native_*`, `src/provider.rs`, `src/runtime.rs`, `src/source*`.

## Linux app plugin replacement (2026-11-09)

On 2026-11-09, the Linux app exposed an installed, ACTIVE `@TermuxBridge`
replacement connector using the then-existing authenticated MCP tunnel.
It was initially created as `Termux Bridge 새 연결`; readback during that
documentation task already returned `@TermuxBridge`. No further plugin creation
or rename was issued by that task. The previous connector returned HTTP 404 in
the selected app account; its deletion or legacy rename was not established.
Account-specific connector and tunnel identifiers stay in private evidence,
not public source history.

All 17 registered/live/candidate input schemas agree, installation and ACTIVE
connection were read back, and the actual connector's sessions-list read passed.
Private current references and proof are in
`~/.cache/codex/linux-plugin-update-b0hcdkmp/current-connection/`; creation and the
earlier successful app-cache refresh remain in its parent directory. This task
submitted no model request or new Native work and replaced no runtime/service.
The naming contract and operating steps are in
[BRIDGE](docs/spec/BRIDGE.md) and the
Linux app guide (`providers/chatgpt/LINUX_APP.md#플러그인-생성과-교체`, preserved on `origin/main` and `rewrite/rust-core`).

## Archived pre-rewrite evidence

Historical pre-rewrite development and qualification records from `rewrite/rust-core` (`0.167.0` `Full153`/`Base152`, `source-acceptance.yml`, `PRO_HANDOFF`) previously appended to this file have been archived unchanged (for historical provenance only, non-normative) in [docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md).
