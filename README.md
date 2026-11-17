# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

**Keeping up with upstream is a permanent product requirement.** This wrapper
continues to adopt official stable Codex releases and their features. We adapt
the Termux layer, remove local code superseded by upstream and improve the
existing automated upgrade path. Compatibility failures require repair; the
current working release remains available until the next is qualified. This
applies to future releases as well as the current target. See the
[continuous upstream upgrade contract](SPEC.md#continuous-upstream-upgrades).

The product has one public command, `codex`, built in two decoupled stages:

- **Stage 1 (active standalone Wrapper):** a standalone Rust Wrapper (`crates/wrapper`)
  and adaptive builder (`crates/release-builder`) with zero external crate dependencies
  beyond `libc` that provides Termux runtime compatibility (FD 33/34/35, wrapper `config.toml`,
  timezone TZif, TLS certs, browser helpers, sandbox guard, `app-server daemon` &
  `remote-control` passthrough), `codex update [--upstream] [--version <M.m.p>] | --path <FILE_OR_DIR> | --rollback`,
  `codex doctor [--color | --json]`, and `codex wrapper status [--json]`.
- **Stage 2 (decoupled optional modules, deferred):** optional Manager, enhanced `TUI`,
  `Provider` bridge, and Ed25519 `--signed` Wrapper/addon bundle verification, preserved on
  `rewrite/rust-core` (with `0.167.2` release/index manifests on `origin/main`) for later opt-in reattachment once Stage 1
  is established. See [current status](CURRENT.md) and the [Stage 2 reintegration plan](docs/plan/STAGE2_PLAN.md).

## Requirements on Termux

Stage 1 targets Android AArch64 (`aarch64-linux-android`, Android API 24+) inside
Termux and relies only on base Termux packages (plus base `Essential` `procps` `ps`
needed by upstream `app-server daemon` PID tracking):

```sh
pkg install curl openssl-tool gzip
```

(`clang`/`rust` are only needed if you build the wrapper from source on-device.)

## Installing the Stage 1 Wrapper

The standalone installer [install.sh](install.sh) stages the `codex` binary into
`$PREFIX/bin/codex`, runs a live `/system/bin/linker64` sanity check when on Android,
moves it atomically into place, and bootstraps an upstream generation if none is
active yet.

### Option 1: Install from a prebuilt release tarball or local checkout

If you unpacked a release tarball containing `bin/codex` and `SHA256SUMS`, or built
the repo locally:

```sh
# Unpacked tarball directory (verifies SHA256SUMS automatically)
./install.sh

# Or point directly at a local wrapper binary
./install.sh --binary ./target/release/codex
```

### Option 2: Build locally from source in Termux

```sh
./install.sh --build
```

This runs `cargo build --release --locked -p codex` and installs
`target/release/codex` into `$PREFIX/bin/codex`.

### Option 3: Install directly from a CI-published GitHub Release

Once a `wrapper` GitHub Release is published:

```sh
curl -fsSL https://github.com/humtr/codex/releases/download/wrapper/install.sh | sh
```

This installs the standalone `codex` wrapper binary, verifies its SHA-256
digest, and bootstraps an active adapted upstream Codex generation; see
[current release and verification status](CURRENT.md#verified-boundaries--post-cutover-wrapper-naming--suffix-cleanup-2026-11-16).

Useful installer options (`./install.sh --help`):

- `--prefix <DIR>`: override `$PREFIX` (defaults to `$PREFIX` or `/data/data/com.termux/files/usr`).
- `--release <TAG>`: download `codex-wrapper-aarch64-linux-android.tar.gz` and `.sha256` from a specific GitHub Release tag (defaults to `wrapper` rather than `/releases/latest`).
- `--version <X.Y.Z>`: bootstrap a specific upstream Codex release version during install.
- `--skip-upstream`: install `$PREFIX/bin/codex` without running `codex update --upstream` immediately.

## Installer interruption and recovery

`install.sh` creates an exclusive `$PREFIX/bin/.codex-install.lock` containing a
synced owner identity (PID, process start ticks, boot ID) and a durable
`codex-install-v1` receipt with the previous/new executable SHA-256, the
pre-install activation-state digest and the commit phase. The exact previous
executable is separately SHA-256-verified and synchronized **before** the
atomic executable rename. The upstream activation journal remains independent.

A caught failure before generation activation restores the exact prior executable
(or removes an unsuccessful first install). A failed updater that has already
changed the authoritative generation is **not** rolled back over that generation:
the installer retains its transaction, candidate and backup for reconciliation.
`--skip-upstream` commits the executable without triggering the updater.

After SIGKILL or a crash, a new install can automatically settle only a
verified pre-updater transaction. For explicit recovery without attempting a
new install, run `install.sh --prefix <PREFIX> --recover`. A completed activation
can be adopted without repeating the updater **only** when the child/group are
proved quiescent and the active generation's metadata and running Wrapper match
the recorded SHA-256. Live jobs, PID reuse, absent ownership records, partial
or conflicting metadata, and uncertain activation fail closed with a specific
diagnostic. Inspect the retained lock/backup and owned updater before any manual
intervention; never remove a lock merely because `kill -0` failed. An
uninspectable transaction requires operator-led reconciliation, not an automatic
second updater. This protocol synchronizes files and directories but cannot
claim testing of an actual physical power cut or firmware storage guarantees.

## Managing Upstream Codex Generations

`install.sh` replaces the Wrapper executable. `codex update` uses that installed
Wrapper to download, verify, patch and activate upstream Codex; it does not update
the Wrapper executable. The same Wrapper can handle new upstream releases while
its adaptation rules remain sufficient. A compatibility defect beyond those rules
requires a Wrapper fix and a new Wrapper installation.

After `codex` is installed:

```sh
# Fetch, verify (HTTPS + official OpenAI release SHA-256), patch adaptively, and activate latest upstream Codex
codex update --upstream

# Install a specific upstream version
codex update --upstream --version <X.Y.Z>

# Install from an official upstream package .tar.gz or a prepared generation directory
codex update --path /path/to/codex-package-aarch64-unknown-linux-musl.tar.gz

# Roll back to the immediately preceding activated generation
codex update --rollback

# Inspect official release inputs without downloading or changing installed state
codex update --check --json
```

Every generation is isolated under `~/.local/lib/codex/wrapper/generations/<id>/`
with a `previous` pointer for single-step rollback. State owned by the wrapper
(`resolv.conf`, wrapper `config.toml`, timezone TZif cache) lives under
`~/.local/share/codex/wrapper/` and `~/.cache/codex/timezones/`.

- `--signed`, `--remote`, `--build-local`, and `--local` are disabled in Stage 1 and fail explicitly with exit status `2`.
- Upstream updates in Stage 1 resolve the latest stable release via GitHub Releases (`openai/codex/releases/latest`) or a requested release via `https://releases.openai.com/codex/releases/<version>/release.json`, and verify the official `codex-package-aarch64-unknown-linux-musl.tar.gz` SHA-256 digest over TLS 1.2+.
- Latest discovery is cross-checked against the exact-version official descriptor.
  `--check --json` reports the version, canonical archive/descriptor URLs and
  SHA-256 digests without installing, recovering journals or pruning. Ordinary
  unpinned discovery rejects an unintended downgrade; explicit `--version` and
  rollback remain available.
- Candidate admission checks exact version output and bounded `features list`
  in a fresh temporary home. Full daemon/PTY/tool/rollback qualification is a
  separate reusable test, not an account or personal-device check on every update.

Binary adaptation happens when a generation is prepared; the patched bytes remain
in that generation. Each launch still prepares process-local FD 33/34/35 and the
Termux environment. Unchanged Wrapper system config is reused without rewriting
it. Its `thread_unload_delay_secs = 0` default applies to every `CODEX_HOME`, with
explicit user settings retaining their upstream precedence.

## Optional module configuration (Stage 2.1 boundary)

Stage 1 upstream execution remains the default: no Manager, enhanced TUI, or
ChatGPT Provider binaries are bundled or started by this wrapper. The Stage 2.1
extension boundary implements strict `wrapper.toml` at
`~/.local/share/codex/wrapper/wrapper.toml`, multi-flag `codex wrapper setup`,
and conditional FD 34 immutable fragment snapshots (`config/snapshots/<settings_id>/`
with `flock(LOCK_SH)` and `features list` validation when modules supply fragments), with:

```sh
codex wrapper setup show
codex wrapper setup --enable manager
codex wrapper setup --disable manager
codex wrapper setup --signed-update false
codex wrapper setup --bare
codex wrapper --bare -- --version
codex wrapper --with tui -- --version
```

An explicit `--with` request fails if its module is unavailable. An unavailable
module selected in persistent settings emits a warning and ordinary `codex`
continues with standalone upstream, so optional modules cannot disable the base
runtime. `--bare` bypasses enabled
addon modules for one launch while preserving upstream arguments (`wrapper setup
--bare` disables addon modules while retaining `signed_update`). Setting
`signed_update = true` records the opt-in but does **not** activate a signed
channel until Phase 2.2; ordinary `codex update` remains independent, and
actual Manager/TUI/Provider addon reattachment remains deferred to Phases 2.3–2.5.
The no-addon launch adds no dependency, addon process, lock, snapshot write, or extra config probe.

## Diagnostics and Upstream Passthrough

```sh
# Check both upstream health checks and Termux wrapper checks (FDs 33/34/35, TLS, TZif, generation)
codex doctor
codex doctor --color
codex doctor --json

# Inspect active wrapper generation and layout
codex wrapper status
codex wrapper status --json

# Direct upstream subcommands supported seamlessly under Termux runtime prep
codex app-server daemon start
codex app-server daemon version
codex remote-control
```

Upstream keeps its selected daemon package separate from the CLI generation.
After updating or re-adapting the CLI, select its complete adapted package when
you also want to update an existing daemon:

```sh
codex app-server daemon update --from-cli
codex app-server daemon start
codex app-server daemon version
```

The upstream confirmation covers replacement and a possible restart of running
work. Wrapper does not change that selection automatically. Use `--from-cli`
to retain the Termux adaptation; the plain upstream daemon updater downloads an
official package without Wrapper adaptation.

Closing a TUI does not necessarily stop its background server. A running server
keeps its selected executable and startup settings until its normal lifecycle
applies a replacement. Idle writer release requires the last subscriber to leave
and the turn to be inactive; the global unload default does not interrupt work or
merge authentication between homes.

Android kernels lack Linux Landlock / unprivileged user namespaces, so the Stage 1
wrapper sets `sandbox_mode = "danger-full-access"` in its wrapper-owned config and
rejects incompatible `--sandbox read-only|workspace-write` or `codex sandbox linux`
invocations cleanly with exit code `2`.

## What the Stage 1 Wrapper Handles at Launch

Before `exec`ing upstream's static musl `codex` binary (plus optional
`codex-code-mode-host` and `codex-exec-server` helpers patched adaptively at install
time), the wrapper prepares the Android/Termux environment:

1. **DNS (`FD 33`):** materializes `~/.local/share/codex/wrapper/resolv.conf` when needed and redirects `/etc/resolv.conf` reads to `FD 33`.
2. **Wrapper config (`FD 34`):** lazily ensures `~/.local/share/codex/wrapper/config/config.toml` (`sandbox_mode = "danger-full-access"`, `mcp_oauth_credentials_store = "file"`, `thread_unload_delay_secs = 0`) and redirects `/etc/codex` reads to `FD 34`.
3. **Temp root & UDS directory (`FD 35` + `TMPDIR`):** requires inherited `PREFIX` and `TMPDIR`, exports `TMPDIR`/`TMP`/`TEMP`/`SQLITE_TMPDIR`, and binds `FD 35` so static musl temp/socket paths do not hit read-only `/tmp`.
4. **Timezone (`TZ`):** writes/reuses a valid `TZif` under `$XDG_CACHE_HOME/codex/timezones` (or `~/.cache/codex/timezones`) from Android `persist.sys.timezone` / `$TZ`.
5. **TLS & environment fence:** injects Termux `SSL_CERT_FILE` / `SSL_CERT_DIR`, disables upstream daemon auto-updates (`$CODEX_HOME/app-server-daemon/settings.json` defaulting to `~/.codex/app-server-daemon/settings.json` -> `{"updater":{"autoUpdateEnabled":false}}` on normal launches), and neutralizes unreachable host X11/Wayland/WSL browser env vars.

Fast inspection flags (`--version`, `-V`, `--help`, `-h`, and top-level `help`) bypass settings writes cleanly.

## CI and Building From Source

### Wrapper CI (`.github/workflows/wrapper.yml`)

Every push or pull request on `main`, `wrapper`, or `rewrite/wrapper` runs:

1. **`checks` (`ubuntu-24.04-arm`):** YAML syntax check, `sh -n install.sh` (plus `shellcheck install.sh` when available), Stage 1 workspace/`Cargo.lock` boundary check (`codex`, `codex-release-builder`, `libc`), markdown link check across all `*.md` files, `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`, including execution of the emitted AArch64 `realpath` hook.
2. **`android-package` (`ubuntu-24.04`):** cross-compiles `codex` and `codex-release-builder` for `aarch64-linux-android` (API 24, Android NDK `27.3.13750724`), verifies `ARM aarch64` ELF headers and `/system/bin/linker64`, bundles `codex-wrapper-aarch64-linux-android.tar.gz` + `.sha256` + `SHA256SUMS` + `install.sh`, and uploads the workflow artifact.
3. **`android-smoke` (`ubuntu-24.04-arm`):** provisions pinned Android Runtime APEX `linker64` and Bionic libraries, installs the packaged bundle via `install.sh --skip-upstream`, and executes the real Android binary (`codex update --help`, `codex wrapper status --json`, `codex --sandbox read-only --version`, `codex sandbox linux true`).
4. **`release` (tag `wrapper`, `wrapper-*`, or `v*`, or manual `workflow_dispatch` with `publish_release=true`):** publishes the verified bundle, `.sha256`, `install.sh`, and `SHA256SUMS` to GitHub Releases.

The manual upstream canary tests the installed adaptation logic against exact
official inputs. It neither repairs that logic automatically nor gates local
`codex update` through an online CI result. CI with a Bionic loader on Linux and
actual Android execution remain distinct evidence boundaries. Current releases
use checksum verification; Ed25519 verification of distributed Wrapper/addon
bundles is planned in Phase 2.2, not active Stage 1 behavior.

### Local development checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked -p codex -p codex-release-builder
```

## Repository Documentation

- [SPEC.md](SPEC.md) — Normative product contract and Stage 1 / Stage 2 boundaries.
- [CURRENT.md](CURRENT.md) — Verified build, live-device, and isolated runtime evidence.
- [docs/plan/STAGE2_PLAN.md](docs/plan/STAGE2_PLAN.md) — Stage 2 detailed reintegration procedure, lineage warnings, and verification gates.
- [AGENTS.md](AGENTS.md) — Agent and engineering working principles.
- [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md) and [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) — CLI routing and delivery contracts.
- [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md) — Stage 2 optional ChatGPT bridge architecture (source preserved on `rewrite/rust-core`).
- [docs/history/GENERATION_ADMISSION.md](docs/history/GENERATION_ADMISSION.md) — Historical generation admission architecture record.
- [docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md) — Archived pre-rewrite `rewrite/rust-core` (`0.167.0`) development evidence.
