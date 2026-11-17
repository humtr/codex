# Product state and completion roadmap

SPEC.md and its linked normative annexes own behavior and component boundaries.
AGENTS.md owns development principles
and must be reread after recovery. This file distinguishes source implementation,
automated proof, actual Android execution, connected-client proof, public signed
deployment and device activation. None substitutes for the next boundary.

## Stage 1: Standalone Wrapper (`rewrite/wrapper`, 2026-11-16)

### Stage 1 P0 installer safety qualification (2026-11-17; isolated worktree)

Work branch `work/wrapper-p0-recovery-20261117` began at
`e484e833a6e733917b12318999ebe4e2938a8cca`, with only two preexisting
untracked user TOML files in the original repository. No live service, account,
installed Wrapper or activation generation was modified. In an isolated Termux
PREFIX, the original installer was SIGKILLed while its spawned updater remained
alive (parent PID 28456, child PID 28487); the staged executable replaced the old
SHA while the previous executable and empty install lock remained. The updated
installer uses synced installer transaction receipts and conservative recovery.
The owned-PREFIX synthetic process/filesystem fault suite
`.github/scripts/test_install_recovery.py` passed 14/14 on actual Termux with
Android linker present, including I1-I15 coverage (I6/I9 combined and I12/I13
combined), plus explicit missing updater receipt rejection. In a **second isolated** `HOME`, `CODEX_HOME`, `TMPDIR`, and copied
`PREFIX`, the already-installed 0.169.0 Wrapper acquired and verified the official
0.169.0 archive and activated `local-1794879214-17073-2` (exit 0); `codex
--version` returned `codex-cli 0.169.0` and a subsequent install without
`--skip-upstream` recognized the qualified matching active generation without
redownloading. `doctor --json` reported no isolated credentials as expected;
`daemon --help` and `remote-control --help` worked. Operational binary SHA
remained `715179a4be5518aedb7091dc148d3fe25df8a242338de7fc68780693423fa8c6`.
This is **not** a physical power-loss result or a complete P0 release
acceptance. Generation-journal and public Release checks remain separate gates.

### P0 activation journal fault qualification (2026-11-17; isolated worktree)

The existing `ActivationIo` state transition and `FsActivationIo` were retained.
Actual POSIX filesystem operations were interrupted **before and after** each of
8 activation I/O steps for 3 transitions (initial `before=None`, update and
rollback): **48 independent isolated state transitions**, each verified for
file combination, authoritative pointer, single and repeated recovery. Another
**10 recovery-I/O interruption points**, orphan combinations, conflict rejection
and a second writer lock check also passed. A real defect was identified in
`load_activated_generation`: it skipped recovery entirely when `activation-state`
was missing, even with a published initial journal. The loader now always invokes
the existing recovery routine; an isolated regression proves a published initial
journal is cleaned and then reports `NoCurrentGeneration` without guessing a
committed generation. A freshly built candidate Wrapper (`8265b72fbf15ea90e80ef7fa1bb53848ae91df01d37767a89c383b16e10de447`)
ran on actual Android in a completely isolated HOME/PREFIX/CODEX_HOME/TMPDIR:
a published initial journal with no pointer was recovered as uninitialized,
then official `0.169.0` installed and `codex --version` succeeded; an explicit
official `0.167.0` update followed by `codex update --rollback` restored exact
`0.169.0`. At this boundary, `cargo clippy --workspace --all-targets --locked
-- -D warnings` exited 0 and `cargo test --workspace --locked` passed 53
Wrapper + 8 release-builder tests with no failures. The faults are deterministic
userspace I/O interruptions, not a physical power-cut proof; GitHub CI checks
remain separate acceptance gates.

### P0 GitHub Release provenance qualification (2026-11-17; isolated worktree)

Read-only public GitHub API and exact asset download confirmed lightweight tag
`wrapper` points at `0519fc5c18f7f330f1391775afd6442f5513094d`, matching
Release body `Source commit`, and archive 886,125 bytes hashes to
`cccefd20e8affd743cad52b23872aaf47b27e79467a725cbea4161d4ecc6b567`,
matching the Release API asset digest. In contrast `targetCommitish`
was `bc816505e7eef4f5590a68e6b5bb5da91820ce59`, a **metadata-only drift**
in that published state. No public tag/Release was mutated. The release workflow
now invokes `.github/scripts/publish_wrapper_release.py` only from the
publication job, after source checkout; it performs exact Git commit/tag
validation (with annotated tag peeling), local SHA checks, per-asset download
and byte readback, note validation and final tag recheck. The publisher is
serialized per target tag and no error is ignored. The offline fake-GitHub
suite `.github/scripts/test_publish_wrapper_release.py` passed 16/16,
including tag/API failures, incorrect tags/sources/notes/bytes, partial upload,
annotated tags and concurrent tag drift. **At this earlier isolated checkpoint**,
no live public release had yet been published. The later authorized
source-bound publication and independent consumer acceptance are recorded below.

### Stage 1 P0 operational cutover and public Release acceptance (2026-11-17; actual Termux)

- **Accepted source and CI:** `rewrite/wrapper` commit
  `990ac6036ac7995c297b9c88103bffd3d28269a6` passed ordinary
  [Wrapper CI run 44002875118](https://github.com/humtr/codex/actions/runs/44002875118),
  explicit official `0.169.0` qualification
  [44002563425](https://github.com/humtr/codex/actions/runs/44002563425),
  and publication with repeat official qualification
  [44006439993](https://github.com/humtr/codex/actions/runs/44006439993).
  The last run passed all lint/unit, NDK packaging, Android Bionic smoke,
  official-upstream qualification and GitHub Release publication jobs.
- **Actual operating entrypoint replacement:** `install.sh --binary <CI-verified
  aarch64-linux-android bin/codex> --skip-upstream` returned `0` on the
  real `$PREFIX/bin/codex` at 11:49 KST. Old executable SHA-256
  `715179a4be5518aedb7091dc148d3fe25df8a242338de7fc68780693423fa8c6`
  was preserved; new installed SHA-256
  `f5739302eff9b39a6883eca94f0ab4acfb8a79133f7493d392d8277828adeb3b`
  matches the exact CI artifact. The real `activation-state` SHA-256 remained
  `8c69ee4a3eae79b7ee8a4a357fb35d588717f036a56cba3f3891c71a58bc0515`;
  active `local-1794826650-14364-2` and previous `local-1794826619-14075-2`
  both remain unchanged on upstream `0.169.0`. No installer lock remains,
  live daemon/Provider processes were preserved, live `doctor` is `healthy`,
  and two existing untracked user profile links were preserved during the
  source-checkout fast-forward. Verified byte backups, both status snapshots,
  the installer stdout/exit proof and rollback instructions are under
  `$HOME/.local/share/codex/wrapper-recovery/20261117T0249Z-stage1-p0/`.
  **No active upstream generation, account, credential, service or profile
  was replaced or restarted by the cutover.**
- **Physical-device independent regression:** the owned-prefix installer
  interrupted-state suite passed `14/14`, and the mock-GitHub source/tag/byte
  failure suite `16/16`. The same now-installed Wrapper binary passed the
  fresh account-free `qualify_wrapper.py` run at
  `$PREFIX/tmp/codex-stage1-activation-20261117/qualified-e2e/report.json`:
  official `0.167.0` to `0.169.0` update, exact-generation rollback and
  restoration, all three adapted upstream executables, daemon start/control/stop,
  Native `initialize`/`config/read`/`command/exec` with independent file SHA,
  and an eight-second interactive PTY startup (`7,993` rendered bytes) passed.
  The device is Android API `37`/AArch64 with **4 KiB kernel pages**; the
  Wrapper and all three runtime ELFs separately passed 16 KiB geometry checks.
  An account-free test `doctor` reports healthy Wrapper/app-server but fails
  the intentionally absent credentials check; production `doctor` is healthy.
- **Actual public Release and consumer:** the authorized `wrapper` prerelease
  tag now peels to exact `990ac603` and `targetCommitish` matches it.
  The public archive SHA-256 is
  `37f2bfacfc4708370c3b88fd0279f1bad8f6930e54a8a600b863b6f37e4ace11`
  (890,307 bytes). All **four** real downloaded assets exactly matched CI
  bytes, including `install.sh` SHA-256
  `3ab6a9edcd0deae34572c7adf8d17f220e5ca881256e1bc12741c8679c98113e`.
  A **second** account-free device root actually executed the published
  `curl -fsSL <Release install.sh URL> | sh -s -- --prefix ... --version 0.169.0`
  consumer path. It installed the
  same Wrapper SHA, activated the official upstream `0.169.0`, started and
  stopped its own managed daemon, completed local Native RPC/file proof,
  and rendered the eight-second PTY (`8,038` bytes). Result `passed` at
  `$PREFIX/tmp/codex-stage1-activation-20261117/public-consumer/report.json`.
  No actual personal account was used for this publication-consumer probe.
- **Remaining distinct qualification boundaries:** physical power removal
  (as opposed to userspace SIGKILL and I/O faults), execution on a real 16 KiB
  kernel or other Android API/hardware, authenticated model inference *after*
  the new Wrapper replacement (a previous one-time Stage 1 inference proof
  exists above), and external remote-control pairing from a separate client.
  Neither test exercised a public signed Stage 2 addon, nor does this evidence
  authorize `main` merging or automatic operational daemon replacement.

### Status and scope
Stage 1 decouples `crates/wrapper` (`codex`) and `crates/release-builder` (`codex-release-builder`) into a standalone, zero-addon Termux compatibility wrapper aligned with [SPEC.md](SPEC.md) Sections 1–5 (`Stage 1` vs `Stage 2` architecture).
- `Cargo.toml` contains only `crates/wrapper` and `crates/release-builder`, and `Cargo.lock` locks only `codex`, `codex-release-builder`, and `libc` (`0.2.186`). Zero non-wrapper addon crates, providers, TUI patches, or legacy CI/qualification scripts remain tracked on `rewrite/wrapper`.
- Active Stage 1 CLI surface: `codex update [--upstream] [--version <MAJOR.MINOR.PATCH>] | --path <FILE_OR_DIR> | --rollback | --check [--version <MAJOR.MINOR.PATCH>] [--json] | --help`, `codex wrapper status [--json]`, `codex doctor [--color | --json]`, and direct upstream passthrough including `codex app-server daemon ...` and `codex remote-control`. Removed flags (`--build-local`, `--local`, `--signed`, `--remote`) exit with status `2`; `codex-release-builder` exposes only `build` (`fetch`/`prepare`/`publish` exit `2`).
- Adaptive Termux binary patching (`crates/release-builder/src/runtime_patch.rs`): applies unified equal-length path remaps (`termux-fd-remap`, FD 33/34) plus adaptive UDS FD 35 relocation (`termux-uds-fd35`, intercepting static musl `realpath` for `/proc/self/fd/35` paths/symlinks as well as `site1`/`site2`, across internal RX alignment gaps and safe trailing RX `PT_LOAD` extension) across `0..N` matching static AArch64 ELFs under `bin/*` (`bin/codex`, optional `bin/codex-code-mode-host`, optional `bin/codex-exec-server`), preserves static AArch64 `codex-path/rg` when present (otherwise installs wrapper script), preserves `codex-resources/*`, and writes a single `codex-local-generation` (`package-root`) `generation.meta` with `wrapper_api_identity = wrapper-api`, `persistent_schema_identity = schema`, `creation_metadata = codex-wrapper`, and `termux-browser-open` / `termux-browser-manual` under `browser/{open,manual}/curl`, without `manager_artifact_digest`, `upstream_doctor`, or legacy fallback readers (`v1`/`v2`/`v3`, `99b`, `R10` helper bridge, or `compat/` layouts).
- Standalone runtime setup & state: fast `--version`/`-V`/`--help`/`-h` / `help` path bypassing `daemon_settings::ensure` (which manages only `$CODEX_HOME/app-server-daemon/settings.json` `autoUpdateEnabled: false`); lock-free FD 33 `resolv.conf` cache hit; lazy FD 34 `config.toml` (`# codex-termux-wrapper`, `sandbox_mode = "danger-full-access"`, `mcp_oauth_credentials_store = "file"`, `thread_unload_delay_secs = 0`); `timezone::configure` (`TZ` TZif); FD 35 temp root; `codex-activation-state`, `codex-activation-journal`, and `downloads/` under `$HOME/.local/share/codex/wrapper/` (serialized via directory `flock` on the state root); AArch64 `SYS_renameat2` (`276`) atomic `rename_noreplace`; TLS/TMPDIR/BROWSER/X11-Wayland-WSL environment fence; `codex doctor --json` (`schema_version: 1`, `layout: "package-root"`); and `--sandbox` mode guard.

### Verified boundaries — post-cutover (`wrapper` naming & suffix cleanup, 2026-11-16)
- **Workspace format, lint, tests & release binaries (`aarch64` Termux host):**
  - `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `(umask 0022 && cargo test --workspace --locked)` exited `0` (`codex`: `25 passed; 0 failed`, `codex-release-builder`: `6 passed; 0 failed` including unit & `runtime_patch::tests`, `0` doc-test failures; unit/synthetic tests cover behavior directly without hardcoded version pins).
  - `cargo build --release --locked -p codex -p codex-release-builder` built `target/release/codex` (`1,302,024` bytes) and `target/release/codex-release-builder` (`760,488` bytes).
- **Installer, workflow, dependency & documentation checks:**
  - `ruby` (`4.0.9` with `libyaml` `0.2.5-5`) YAML validation (`Psych.parse_file` and `YAML.safe_load`) on `.github/workflows/wrapper.yml`, `sh -n install.sh`, `shellcheck -S style install.sh` (plus CI `shellcheck install.sh`), `Cargo.toml`/`Cargo.lock` dependency check (`codex`, `codex-release-builder`, `libc`), and markdown relative-link check passed.
- **GitHub Actions `Wrapper CI` & `wrapper` GitHub Pre-release (`rewrite/wrapper`):**
  - Push runs (`316d0c09` `43781905741`, `bc816505` `43784796608`, `c0abf7a6` `43786603428`) and manual `workflow_dispatch` release dispatch (`bc816505` `43784904077`, `publish_release=true`, `release_tag=wrapper`, `prerelease=true`) passed all active jobs (`Build and package aarch64-linux-android bundle`, `Workspace lint, tests, and docs`, `Smoke-test Android Bionic binary and install.sh`, and `Publish wrapper GitHub Release`).
  - Historical prerelease `wrapper` (<https://github.com/humtr/codex/releases/tag/wrapper>) at `bc816505` (`43784904077`) was subsequently refreshed at `0519fc5c` via `wrapper.yml` dispatch (`43903649299`, `publish_release=true`, `release_tag=wrapper`, `prerelease=true`; `codex-wrapper-aarch64-linux-android.tar.gz` `sha256:cccefd20e8affd743cad52b23872aaf47b27e79467a725cbea4161d4ecc6b567`). Source-only pushes do not auto-refresh release assets unless `publish_release=true` (or a release tag) is invoked.
- **Isolated synthetic `$HOME`/`$PREFIX` E2E smoke test (disposable temp roots on Android AArch64, with system `curl`/`openssl`/`gzip`/`tar`/`sh` and TLS cert copied into synthetic `$PREFIX`):**
  - `install.sh --binary target/release/codex --prefix "$SYN_PREFIX" --skip-upstream` installed `$SYN_PREFIX/bin/codex`, `codex update --help` and `codex wrapper --help` exited `0`, uninitialized `codex wrapper status --json` exited `1` (`"status":"uninitialized"`, `"addons":"none"`), and `--sandbox read-only --version` and `sandbox linux true` exited `2`.
  - Using minimal static AArch64 mock binaries containing `/etc/resolv.conf` and `/etc/codex/config.toml` sites, `codex update --path pkg-1.0.0.tar.gz` patched and activated generation `local-*` (`generation.meta` `codex-local-generation`, `patch_policy_id: termux-fd-remap`, `wrapper-api`, `schema`, `codex-wrapper`, `termux-browser-open`/`manual`; `activation-state` `format=codex-activation-state`; FD 34 `config.toml` `# codex-termux-wrapper`), `codex --version` printed `codex-cli 1.0.0`, `codex wrapper status --json` reported `"layout":"package-root"`, `codex doctor --json` exited `0` (`"schema_version":1`, `"layout":"package-root"`), upgrade to `2.0.0` + `codex update --rollback` restored `codex-cli 1.0.0`, and `codex-release-builder build` -> `codex update --path <built-gen>` activated `"has_code_mode_host":false` (`codex-cli 3.0.0`).
- **Publication and activation boundaries:**
  - Live public `curl | sh` installation was subsequently tested below. The published binary still predates the fixes; fresh accepted-source release assets have not been published.
  - Local executable/generation activation was subsequently authorized and completed below. External remote client pairing over `codex remote-control` remains unverified.
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
- **Open baseline record before Issue #33 fix (preserved):** Earlier isolated acceptance tests on `a224f223` / `f8284261` failed `app-server daemon start`, `app-server --listen unix://`, and default `codex` PTY with `SUN_LEN` (tracked in Issue #33).
- **Root-cause fix & isolated Android API 37 Termux requalification (2026-11-16, current source):**
  - **Root cause:** In upstream `0.169.0`, BOTH server bind (`short_socket_dir` calling `realpath("/proc/self/fd/35")`) AND daemon client connect (`std::fs::canonicalize(".../app-server-control.sock")`) invoked static musl `realpath`, expanding `/proc/self/fd/35` through procfs back to long physical `$TMPDIR`. In addition, `codex-exec-server` and `codex-code-mode-host` had smaller `.fini`–`.rodata` gaps (`192` and `332` bytes) than `codex` (`1,120` bytes), requiring multi-gap RX search and safe same-page trailing `RX PT_LOAD` extension. Additionally, upstream `0.169.0` daemon startup invokes `ps -p <pid> -o stat= -o lstart=`, which requires Termux base `procps` (`/usr/bin/ps`), whereas `/system/bin/ps` (`toybox`) fails on `/proc/sys/kernel/pid_max` (`EACCES`).
  - **Isolated Termux verification against cached official `Codex 0.169.0`:** Using owned isolated `$HOME`/`$PREFIX` (with base Termux tools including `procps` `ps`), `TMPDIR` of `118` bytes, `CODEX_HOME` of `130` bytes, and control socket path of `173` bytes:
    - `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `(umask 0022 && cargo test --workspace --locked)` (`codex`: `40 passed; 0 failed`, `codex-release-builder`: `6 passed; 0 failed`), and `cargo build --release --locked` passed.
    - `install.sh --binary ... --skip-upstream` installed cleanly; `codex update --path` adapted all three `0.169.0` binaries cleanly; `codex --version`, `codex-exec-server --version` (`0.169.0`), and `codex-code-mode-host --help` executed cleanly on the Android kernel.
    - `codex app-server daemon start` (`"status":"started"`), `daemon version` (`"status":"running"`, `0.169.0`), `daemon stop` (`"status":"stopped"`), `timeout 2s codex app-server --listen unix://<118B-TMPDIR>/direct-listen.sock` (listening, exit `124`), and default `codex` PTY (`TERM=xterm-256color`, `671` TUI bytes, zero `SUN_LEN`, with `codex doctor --json` confirming `app_server.status` ok & `termux_wrapper` `healthy`, overall `unhealthy` only for missing auth credentials) all passed cleanly without altering live `/data/data/com.termux/files/usr/bin/codex` or running services.
  - **Boundaries remaining at that baseline:** live activation, authenticated task completion, external client pairing and refreshed public-asset installation. Later one-time authentication and public installation results are recorded below.
- `install.sh` failure rollback is caught-exit/signal safe, not a
  cross-power-loss transaction; after SIGKILL/power loss, an owned
  `.codex.previous.<pid>` may remain behind a stale install lock and
  requires inspected manual recovery. This is explicitly documented in
  `README.md` and independently exercised by the new CI test fixture.

The later read-only upstream scheduler is recorded below. Automatic publication
and production runtime deployment remain separate and have not occurred.

### Independent standalone compatibility review (2026-11-16)

The priority is the minimal Wrapper's independent Termux execution and continued
official-upstream adoption, before addon attachment. Baseline `51f69a69` and
[Wrapper CI 43817848999](https://github.com/humtr/codex/actions/runs/43817848999)
were independently checked: all three checks succeeded; release publication was
skipped. This establishes neither refreshed public assets nor device activation.

- **Adaptive admission defect repaired:** UDS detection previously called the
  same ELF layout parser as patch application and converted parse failure to
  “UDS absent.” A valid static ELF with its section table stripped still
  contained the daemon UDS implementation but was successfully published without
  that adaptation. Detection now searches the input independently; an unsupported
  matching layout fails before publication for both main and companion binaries.
- **Standalone independence repaired:** a persistently enabled unavailable addon
  previously blocked ordinary upstream execution, contrary to SPEC Sections 1
  and 4.2. It now warns and continues standalone without changing the selection;
  explicit `--with` still rejects unavailable modules. A subprocess regression
  exercises actual upstream exec, not just dispatch classification.
- **Generated-code proof added:** AArch64 tests execute the emitted `realpath`
  hook with actual libc fallback/allocation in a disposable child. Both output
  buffer forms, FD 35 aliases, socket-style symlinks, ordinary symlink resolution,
  nonexistent paths, prefix collisions, null input and a closed FD 35 are tested.
  Workspace checks now run on `ubuntu-24.04-arm` to execute this gate in CI.
- **Local verification:** workspace format/Clippy/tests passed (`codex`: 40;
  release-builder: 8; zero failures), and native release builds passed. A fresh
  isolated API 37 / 4 KiB-page Termux fixture verified the official `0.169.0`
  archive against current official release metadata SHA-256 `c8d0b4ab...`:
  install/update, all three executables, daemon start/version/stop, actual Native
  `initialize`/`config/read`/`command/exec` with independently checked file contents,
  persisted-missing-addon standalone launch and explicit-addon refusal passed.
  TMPDIR/CODEX_HOME/control socket were 134/146/189 encoded bytes. A sized 30x100
  PTY with terminal-query replies rendered `OpenAI Codex` and stayed alive for
  eight seconds (4,669 output bytes, no `SUN_LEN`); this is startup rendering,
  not an authenticated task or physical UI proof.
- **Negative device proof:** a section-table-stripped official main ELF was
  rejected by `codex update --path`; activation-state bytes and active runtime
  SHA-256 stayed equal, and `--version` still ran. The initial fixture attempt
  omitted the copied resolver; it failed before activation and its failed receipt
  was retained, then the fixture was corrected. Evidence and reproducer remain
  in `$PREFIX/tmp/wrapper-base-review-20261116.rl4paotf/`. No fixture executables
  remain running. Installed `$PREFIX/bin/codex` stayed at SHA-256 `b6611c3a...`.

**Remaining base qualification at that review:** update-time automatic admission
then probed only `--version`; the follow-up below adds bounded configuration
admission and completes the requested one-time full current-environment tests. New musl code generation
or UDS markers can require adaptation work; this is not proof of arbitrary future
release compatibility. API 24/other Android versions and 16 KiB pages, authenticated
model/Code Mode completion, external remote-control pairing and refreshed release
asset installation remain unverified here. Next implement read-only official
release discovery and reusable isolated base qualification through the existing
updater/builder, with physical-device promotion evidence; optional modules and
signed channels must not delay that work.

### Standalone qualification and read-only upstream discovery follow-up (2026-11-16)

- The reported FD 34 race is repaired (details below), with the original failure
  reproduced before the fix and concurrent process/lifetime/recovery tests passing.
- `codex update --check [--version ...] [--json]` reuses the existing official
  resolver/transport, cross-checks latest against an exact descriptor, and emits
  exact archive/metadata/wrapper SHA-256 inputs without creating installed state,
  recovering activation journals, downloading a runtime or invoking retention.
  Draft/prerelease and divergent descriptors fail. Unpinned updates reject an
  unintended downgrade. A real official-latest check resolved `0.169.0` and left
  the owned HOME without `.local/`.
- Local format/Clippy/workspace tests passed (`codex`: 47; release-builder: 8; zero failures).
- Update admission now checks exact upstream version output and `features list`
  in a fresh account-free HOME/CODEX_HOME, with 10-second and 64-KiB stream bounds.
  The obsolete unbounded doctor probe was removed. This adds no ordinary-launch
  probes, account requirement, device allowlist or addon dependency.
- `.github/scripts/qualify_wrapper.py` is a reusable fresh-root test through the
  public installer/updater/launcher, not a second release resolver or adaptor.
  It records observed OS/page size, immutable source/artifact inputs, Native RPC
  file effects, daemon PID/executable identity, actual control connection, sized
  PTY rendering, diagnosis and exact-generation rollback/restoration. Shared
  read-only `.github/workflows/wrapper-verify.yml` retains the three existing
  check/package/Bionic jobs and supports explicit discovery/qualification and
  reuse only of identical completed proof. Publication stays in `wrapper.yml`.
- **Accepted CI source:** `fdcc1bdfeaee7c08cfff4454ec6bd327d4a72ddb` passed
  [ordinary CI](https://github.com/humtr/codex/actions/runs/43852129927) and
  [full official-upstream qualification](https://github.com/humtr/codex/actions/runs/43852762294),
  including an actual fresh `0.169.0` qualification and exact-input proof check.
  Both publication jobs were skipped. The first CI attempt exposed an empty
  `CARGO_ENCODED_RUSTFLAGS` overriding the 16-KiB linker flags; removing that
  override made the ELF alignment assertion pass. No assertion was weakened.
- **Read-only automation is active:** main commit `c49dcf6f` adds only
  `.github/workflows/wrapper-upstream.yml`, pinned to that accepted source,
  with `contents: read` and daily `06:23 UTC` discovery/qualification.
  [Manual execution through this default-branch caller](https://github.com/humtr/codex/actions/runs/43854267201)
  passed all three jobs, fresh qualification and exact-input validation.
  [A second identical-input execution](https://github.com/humtr/codex/actions/runs/43855791029)
  passed by restoring the exact proof, checking its complete inputs and skipping
  the repeated full runtime qualification as intended.
  The workflow is registered as active; a natural cron invocation has not yet
  been observed. Product-source merge, signed release inputs and publication
  authority were not changed.
- **Actual current-environment one-time proof:** isolated API 37 / 4 KiB Termux
  `0.167.0 -> 0.169.0`, all companion entrypoints, daemon start/version/stop,
  Native `command/exec` with checked file bytes, default 30x100 PTY rendering,
  rollback to the exact `0.167.0` generation and restoration of `0.169.0` passed.
  Wrapper doctor was healthy; intentionally account-free overall diagnosis was
  unhealthy. All selected ELFs had 16-KiB-compatible load alignment. This is
  geometry proof, not execution on a 16-KiB kernel or an older Android release.
- **Actual distribution-artifact proof:** the checksum-verified NDK/API-24
  Android bundle from CI `43852129927` also passed the same available-device
  installer, official update, all companion entrypoints, daemon/control identity,
  Native file effect, PTY and exact-generation rollback/restoration checks.
  Its report is `ndk-device/report.json` in the evidence root below; this closes
  the difference between a native Termux build and the CI-distributed binary.
- **Actual authenticated one-time proof:** the existing Codex ChatGPT login was
  used without copying credentials or supplying a Platform API key. An ephemeral
  public-Wrapper `--no-daemon exec` task completed a Native shell command and
  wrote the exact requested nonce file in an owned workspace. Exit was 0;
  authentication-file size/mtime and existing daemon-settings SHA-256 were
  unchanged. Evidence is in Termux's original
  `$PREFIX/tmp/wrapper-qualify-20261116.zfwbz818/` (`base-final/report.json` and
  `authenticated-report.json`). The first harness run's doctor-schema KeyError
  was retained; it was a harness assertion error after healthy Wrapper diagnosis,
  not a runtime failure. The corrected fresh-root run passed.
- **Public installation boundary:** actual HTTPS `curl | sh` from the `wrapper`
  release into a fresh owned HOME/PREFIX completed archive/binary checksum checks,
  default online `0.169.0` bootstrap, `--version` and Wrapper status successfully.
  The executed installer matched the reviewed SHA-256 `f1fcfa65071c86ce20959084f877594c2d9ff8e766d4d8932f3e131a2f81855a`.
  Its published old binary then failed daemon startup with `SUN_LEN` (exit 1).
  This is a public asset lag, not accepted-source runtime proof; receipts remain
  in `public-remote/report.json` alongside the successful CI-artifact device proof.
- **Existing daemon selection boundary:** upgrading that public installation to
  the accepted CI binary and re-adapting the same official version preserved
  upstream's old daemon selection, so a subsequent start still used the old
  executable. Upstream `app-server daemon update --from-cli --yes` then selected
  the complete adapted CLI package; start/version/stop passed, `/proc/<pid>/exe`
  and executable SHA-256 matched the active adapted generation, and the previous
  Wrapper generation remained present. No package files or pointers were repaired
  manually. The first failed upgrade receipt is retained alongside this successful
  explicit-selection receipt. Wrapper update alone does not replace or restart
  the independently selected upstream daemon.

The user's requested boundary is a one-time check of the available environment,
not a hardcoded recurring check of this phone, account, API level or private path.
No other Android/page-size environment or real external remote-control client
has been exercised here. These qualification runs preserved the installed Wrapper;
the later authorized local activation is recorded below. Public release refresh
remains separate from source qualification and local activation.

### Authorized local activation and profile handover (2026-11-16)

- The same thread was observed moving from the `janmori101` shared server to
  the `szob` shared server: the former stopped holding its kernel writer lock,
  and the latter acquired it. The prepared archive/unarchive helper reached
  read-only preflight only and was retired without either mutation. No lock file
  was deleted, and no shared server was killed to force the handover.
- After explicit authorization, `$PREFIX/bin/codex` was replaced through
  `install.sh --binary ... --skip-upstream` with the device-qualified NDK binary
  from source `fdcc1bdfeaee7c08cfff4454ec6bd327d4a72ddb`, SHA-256
  `df970540acb9560add8540fd629ee3df8a176b50a48161b97aec8c9f7f5fdbea`.
  The old signed `codex-activation-state-v3` installation was transitioned under
  the existing directory lock to the exact qualified standalone state and two
  byte-verified generation copies: active `local-1794826650-14364-2`, previous
  `local-1794826619-14075-2`, both official upstream `0.169.0`.
- All 15 pre-existing runtime directories, the previous executable and pointer
  state, Wrapper system config and public verification key were copied and
  verified in `$HOME/.local/share/codex/wrapper-recovery/20261116T121328Z-qualified-cutover/`.
  `receipt.json` and `RECOVERY.txt` describe the cutover and recovery boundary.
  Restoring the former full Wrapper requires its executable and state format
  together; ordinary standalone `codex update --rollback` selects the retained
  standalone generation. No account home, credential or conversation was copied.
- The installed entrypoint passed version/help/status and read-only official
  update inspection (`current`, `0.169.0`). Actual production-root Native
  initialize/config-read/command-exec wrote independently verified nonce bytes;
  selected authentication-file metadata remained unchanged. The first Native
  probe incorrectly removed the existing `CODEX_SQLITE_HOME` routing and timed
  out during local DB backfill initialization; its failed receipt is preserved.
  The corrected probe retained the real environment and passed without changing
  the timeout or starting another model task. Preparation-only directory-sync
  errors also remain recorded; no live binary/state had changed at those errors.
- Existing shared-server connections and active work were retained while the
  two participating profiles' default endpoints were moved to upstream-managed
  daemons launched through the newly installed Wrapper. New `janmori101` PID
  `17578` and `szob` PID `17776` both executed the complete adapted package at
  SHA-256 `69892b8420314c92c1119450887166dbcf6ac562fab0fe665c567409f990aac0`,
  matching the activated generation. Public daemon version/control connections
  and independent WebSocket peer-PID checks verified the new endpoints. The old
  servers remained alive for their existing clients; the current conversation
  still runs in its earlier `szob` server until that client closes. A subsequent
  connection uses the new default endpoint.
- The upstream config API persisted `thread_unload_delay_secs = 0` in these two
  profiles, retaining the immediate-idle-unload behavior previously supplied by
  Wrapper shared-server configuration. All other parsed configuration values
  were checked equal. No credential was copied or edited. The previous endpoint
  symlinks and their targets are recorded in the recovery root. Signed publication
  inputs, release assets, and the system resolver were not changed.
- A separate owned test thread, with **no model inference**, was started on the
  new first-profile server, unsubscribed, and observed releasing its kernel
  writer lock. The same thread ID resumed on the new second-profile server and
  acquired that server's writer ownership. Both servers reported an effective
  unload delay of zero. The upstream API then deleted only this owned test thread;
  its rollout and writer lock were confirmed absent. `server-handover.json`
  records the successful server and profile-handover verification.

### Stage 2.1 — Optional settings, CLI hooks, and FD 34 snapshot engine (2026-11-16)

Implemented beyond initial PR #31 without attaching Manager, enhanced TUI, Provider, or external crates beyond `libc`:
- Bounded, fail-closed `wrapper.toml` parser under the Wrapper state root (`crates/wrapper/src/extensions.rs` + `fs_util.rs`): target-correct AArch64 `O_NOFOLLOW` (`0o100000` on ARM/AArch64, `0o400000` elsewhere), `uid`/`dev`/`ino` validation in `read_bounded_regular`, dedicated `.wrapper-settings.lock` separate from state-root activation `flock`, and XDG-compatible `0o755` (umask-respecting) creation of missing `.local`/`.local/share` while enforcing `0o700` on `codex` and `wrapper`.
- `codex wrapper setup [show|--bare|--enable|--disable|--signed-update]` supports multi-flag updates in one invocation, `--bare` preserves `signed_update`, `codex wrapper status [--json]` reports `signed_update` and `addons`, invocation-local `--bare` and `--with` enforce fail-closed validation on missing modules, and bare `--version`/`--help` retain the zero-write fast path (zero `wrapper.toml`, `config/snapshots/`, or `app-server-daemon/settings.json` writes).
- Conditional FD 34 snapshot engine (`crates/wrapper/src/system_config.rs` + `UpstreamRuntimeHost` in `main.rs`): enforces `FRAGMENT_LIMIT` (`8192` B) and `CONFIG_LIMIT` (`16384` B), derives FNV-1a `"config-v1"` `settings_identity`, validates via cached `validated-<key>` 5s bounded `features list` probe, and holds `flock(LOCK_SH)` across FD 34 child execution. The reported concurrency defect is now repaired: prepare and prune hold the same configuration-directory coordinator, complete new records in unpublished staging, atomically publish the directory with its shared lease already held, and reject/preserve incomplete published state. A regression first failed on the old pruning path; all seven snapshot tests then passed on actual API 37 Termux, including four concurrent creator/pruner processes, interrupted publication and child-inherited FD 34 retention. Actual Manager producer attachment remains unqualified; bare execution bypasses this engine.

**Verification & open boundary:** PR #31 hardening + `system_config` + Issue #33 fixes are verified by workspace unit tests (`codex`: `40 passed; 0 failed`, `codex-release-builder`: `6 passed; 0 failed`) and isolated Termux API 37 runtime checks (multi-flag setup, `--bare` preserving `signed_update`, `(umask 0022)` `755`/`700` XDG modes, AArch64 `O_NOFOLLOW` symlink rejection, unavailable `--with manager` failure, zero-write bare launch). Actual addon module binaries/producers (`manager`, `tui`, `provider`) are not yet attached in Stage 1/2.1; cross-process shared-server integration of FD 34 snapshots will exercise with a real Manager producer in Phase 2.3. Phases 2.2–2.5 remain unstarted; Phase 2.6 read-only discovery and qualification are implemented separately.

### Global FD 34 unload policy, trust-boundary alignment, and prerelease refresh (2026-11-17)
- **Decision reference (non-normative):** `agy --conversation=fca978e5-08f9-420e-8195-44a1a89e05fa`.
  The public user decisions at steps `4074`, `4085`, `7547`, `7578` and `7592`
  establish standalone Wrapper first, independently attachable optional modules,
  on-device upstream patching, the global FD 34 default, and the Wrapper/addon
  distribution signing boundary. This reference records provenance; SPEC and its
  annexes own the requirements. Assistant proposals and earlier experiments are
  not additional acceptance evidence or permanent restrictions on upstream APIs.
- **Global FD 34 `thread_unload_delay_secs = 0` (`0519fc5c`):** Added `thread_unload_delay_secs = 0` directly to `TERMUX_WRAPPER_CONFIG_CONTENT` in `crates/wrapper/src/main.rs` so `/dev/fd/34/config.toml` applies immediate idle thread unload universally across all `CODEX_HOME` profiles without writing per-user `$CODEX_HOME/config.toml` files, removed duplicate emission in `crates/wrapper/src/system_config.rs`, and preserved user-layer overrides. Verified locally (`cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`: `codex` `47 passed; 0 failed`, `codex-release-builder` `8 passed; 0 failed`, isolated FD 34 `config/read` system-layer vs user-layer override check on `0.169.0`).
- **Normative & roadmap alignment:** Updated `SPEC.md`, `docs/spec/USER_INTERFACES.md`, `docs/spec/DELIVERY.md`, `docs/plan/STAGE2_PLAN.md`, `README.md`, and `CURRENT.md` so:
  - Phase 2.2 Ed25519 `--signed` verification targets self-built distributed Wrapper + optional addon bundles (`install.sh` + Wrapper self-update + addons), while official OpenAI upstream tarballs remain authenticated via OpenAI metadata + SHA-256 and locally patched into `generations/<id>/bin/codex` without remote CI re-signing;
  - `.github/workflows/wrapper-upstream.yml` on `main` (`fcefb8b3`) switched from daily `06:23 UTC` cron (`c49dcf6f`) to manual `workflow_dispatch`-only pinned at `0519fc5c`, confirming local `codex update --upstream` has zero CI schedule dependence.
- **CI, prerelease, and live device cutover (`0519fc5c`):**
  - `Wrapper CI` push (`43903430545`) and release dispatch (`43903649299`, `wrapper` tag, `prerelease=true`, bundle SHA-256 `cccefd20e8affd743cad52b23872aaf47b27e79467a725cbea4161d4ecc6b567`) passed all jobs; manual canary dispatch on `main` (`fcefb8b3`, run `43903798151`) passed all jobs.
  - Installed updated CI binary onto `/data/data/com.termux/files/usr/bin/codex` (`sha256: 715179a4be5518aedb7091dc148d3fe25df8a242338de7fc68780693423fa8c6`) via public `install.sh --skip-upstream`, refreshed `~/.local/share/codex/wrapper/config/config.toml`, removed `thread_unload_delay_secs = 0` from per-user `janmori101` and `szob` `config.toml` files (verified zero remaining per-user copies), and confirmed live read-only `config/read` across `default`, `janmori101`, and `szob` resolves `thread_unload_delay_secs = 0` solely from the FD 34 `system` layer.
- **Documentation reconciliation against that conversation:** separated installer,
  upstream updater, manual canary and existing-server lifecycles in the guide and
  contract. Removed remaining normative requirements for device-local/temporary
  signing of ordinary upstream generations and scoped optional Manager signatures
  to addon bundles. The retired signing design is recorded in release history.
  Phase 2.2 signature implementation remains pending. This reconciliation changes
  documentation only; it performs no new deployment, account edit or server restart.

### Stage 2 Plan (Documented — Phases 2.1–2.6 pending qualification)
See [docs/plan/STAGE2_PLAN.md](docs/plan/STAGE2_PLAN.md) for the verified phase-by-phase reintegration procedure, `c200e676` vs `ebb0b519` regression protections, suffix-removal schema rules, live-device safety checks, and verification gates. Summary sequence:
1. **`wrapper.toml` & Generic Extension Contract (Phase 2.1 — implemented in `crates/wrapper`; module producers in Phases 2.3–2.5):**
   - Implemented `~/.local/share/codex/wrapper/wrapper.toml`, `codex wrapper setup`, `codex wrapper --bare`, and `codex wrapper --with <MODULES>`, plus the conditional FD 34 system configuration snapshot engine (`config/snapshots/<settings_id>/`) while keeping bare `codex` / `codex --version` zero-write and free of addon crates or IPC.
2. **`--signed` Ed25519 Wrapper & Addon Distribution Verification (Phase 2.2):**
   - Wire Ed25519 signature verification (`release.manifest`, `release.sig`, `release-authority.sig`, `release-public-key.pem`) and `codex-release-builder publish` over **distributed Wrapper + optional self-built addon bundles** (plus `install.sh` / Wrapper self-update verification), while keeping official upstream tarballs verified by OpenAI SHA-256 and locally patched into `generations/<id>/bin/codex` on-device without remote CI signing. Preserve bare `LOCAL_ZERO` `--upstream` / `--path` updates and degrade missing optional addons at the addon boundary.
3. **Optional `codex-manager` & `runtime-binding` Reattachment (Phase 2.3):**
   - Port `crates/runtime-binding` and `crates/manager` from `rewrite/rust-core` (`c200e676`, plus `ebb0b519` `--ei` and `bin/codex` adapters) behind `wrapper-api` (`codex wrapper launch|profile|task|notify|usage|rotation`), rebranding all `codex termux` callbacks and `CORE` env vars to `codex wrapper` and preserving dependency isolation on bare `crates/wrapper`.
4. **Optional Enhanced TUI (`frontend/tui`, Phase 2.4):**
   - Rebase `frontend/tui` from `c200e676` (`086ccc8...` / `0.167.0`) onto current upstream (`>= 0.169.0`), decouple capability probing from `upstream_package_version`, apply adaptive `runtime_patch`, and attach opt-in via `wrapper.toml` / `--with tui`.
5. **Optional `chatgpt` Provider & Tunnel Reattachment (Phase 2.5):**
   - Port `providers/chatgpt` and `transport/tunnel-client` from `c200e676` to `wrapper` roots / `bin/codex` / `execution.wrapper` behind `codex wrapper provider ...` with an independent Provider CI gate and non-disruptive live `@TermuxBridge` cutover.

6. **Continuous Upstream Adoption (Phase 2.6):** Read-only official-stable
   discovery and base qualification are running through the pinned main caller;
   immutable digest/source/toolchain inputs, proof reuse and rollback are checked.
   Signing/publication, optional module qualification and device activation are
   separate pending work, independent of manual `codex update --upstream`.

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
