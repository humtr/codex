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
  `codex doctor [--json]`, and `codex wrapper status [--json]`.
- **Stage 2 (decoupled optional modules, deferred):** optional Manager, enhanced `TUI`,
  `Provider` bridge, and Ed25519 `--signed` generation trust, preserved on
  `origin/main` and `rewrite/rust-core` for later opt-in reattachment once Stage 1
  is established. See [current status](CURRENT.md).

## Requirements on Termux

Stage 1 targets Android AArch64 (`aarch64-linux-android`, Android API 24+) inside
Termux and relies only on base Termux packages:

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

Useful installer options (`./install.sh --help`):

- `--prefix <DIR>`: override `$PREFIX` (defaults to `$PREFIX` or `/data/data/com.termux/files/usr`).
- `--release <TAG>`: download `codex-wrapper-aarch64-linux-android.tar.gz` and `.sha256` from a specific GitHub Release tag (defaults to `wrapper` rather than `/releases/latest`).
- `--version <X.Y.Z>`: bootstrap a specific upstream Codex release version during install.
- `--skip-upstream`: install `$PREFIX/bin/codex` without running `codex update --upstream` immediately.

## Managing Upstream Codex Generations

After `codex` is installed:

```sh
# Fetch, verify (HTTPS + OpenAI npm SHA-256), patch adaptively, and activate latest upstream Codex
codex update --upstream

# Install a specific upstream version
codex update --upstream --version <X.Y.Z>

# Install from a local upstream tarball or raw AArch64 codex binary
codex update --path /path/to/codex-aarch64-unknown-linux-musl.tar.gz

# Roll back to the immediately preceding activated generation
codex update --rollback
```

Every generation is isolated under `~/.local/lib/codex/wrapper/generations/<id>/`
with a `previous` pointer for single-step rollback. State owned by the wrapper
(`resolv.conf`, wrapper `config.toml`, timezone TZif cache) lives under
`~/.local/share/codex/wrapper/` and `~/.cache/codex/timezones/`.

- `--signed`, `--build-local`, and `--local` are disabled in Stage 1 and fail explicitly with exit status `2`.
- Upstream updates in Stage 1 fetch directly from OpenAI GitHub Releases + npm registry SHA-256 metadata over TLS 1.2+.

## Diagnostics and Upstream Passthrough

```sh
# Check both upstream health checks and Termux wrapper checks (FDs 33/34/35, TLS, TZif, generation)
codex doctor
codex doctor --json

# Inspect active wrapper generation and layout
codex wrapper status
codex wrapper status --json

# Direct upstream subcommands supported seamlessly under Termux runtime prep
codex app-server daemon start
codex app-server daemon version
codex remote-control
```

Android kernels lack Linux Landlock / unprivileged user namespaces, so the Stage 1
wrapper sets `sandbox_mode = "danger-full-access"` in its wrapper-owned config and
rejects incompatible `--sandbox read-only|workspace-write` or `codex sandbox linux`
invocations cleanly with exit code `2`.

## What the Stage 1 Wrapper Handles at Launch

Before `exec`ing upstream's static musl `codex` binary (plus optional
`codex-code-mode-host` and `codex-exec-server` helpers patched adaptively at install
time), the wrapper prepares the Android/Termux environment:

1. **DNS (`FD 33`):** materializes `~/.local/share/codex/wrapper/resolv.conf` when needed and redirects `/etc/resolv.conf` reads to `FD 33`.
2. **Wrapper config (`FD 34`):** lazily ensures `~/.local/share/codex/wrapper/config/config.toml` (`sandbox_mode = "danger-full-access"`, `mcp_oauth_credentials_store = "file"`) and redirects `/etc/codex` reads to `FD 34`.
3. **Temp root & UDS directory (`FD 35` + `TMPDIR`):** exports `TMPDIR` (`$PREFIX/tmp`) and binds `FD 35` so static musl temp/socket paths do not hit read-only `/tmp`.
4. **Timezone (`TZ`):** writes/reuses a valid `TZif` under `$XDG_CACHE_HOME/codex/timezones` (or `~/.cache/codex/timezones`) from Android `persist.sys.timezone` / `$TZ`.
5. **TLS & environment fence:** injects Termux `SSL_CERT_FILE` / `SSL_CERT_DIR`, disables upstream daemon auto-updates (`~/.codex/app-server-daemon/settings.json` -> `{"updater":{"autoUpdateEnabled":false}}` on normal launches), and neutralizes unreachable host X11/Wayland/WSL browser env vars.

Fast inspection flags (`--version`, `-V`, `--help`, `-h`) bypass settings writes cleanly.

## CI and Building From Source

### Wrapper CI (`.github/workflows/wrapper.yml`)

Every push or pull request on `main`, `wrapper`, or `rewrite/wrapper` runs:

1. **`checks` (`ubuntu-24.04`):** YAML syntax check, `sh -n install.sh`, Stage 1 workspace/`Cargo.lock` boundary check (`codex`, `codex-release-builder`, `libc`), markdown link check across all `*.md` files, `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`.
2. **`android-package` (`ubuntu-24.04`):** cross-compiles `codex` and `codex-release-builder` for `aarch64-linux-android` (API 24, Android NDK `27.3.13750724`), verifies `ARM aarch64` ELF headers and `/system/bin/linker64`, bundles `codex-wrapper-aarch64-linux-android.tar.gz` + `.sha256` + `SHA256SUMS` + `install.sh`, and uploads the workflow artifact.
3. **`android-smoke` (`ubuntu-24.04-arm`):** provisions pinned Android Runtime APEX `linker64` and Bionic libraries, installs the packaged bundle via `install.sh --skip-upstream`, and executes the real Android binary (`codex update --help`, `codex wrapper status --json`, `codex --sandbox read-only --version`, `codex sandbox linux true`).
4. **`release` (tag `wrapper`, `wrapper-*`, or `v*`, or manual `workflow_dispatch` with `publish_release=true`):** publishes the verified bundle, `.sha256`, `install.sh`, and `SHA256SUMS` to GitHub Releases.

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
- [AGENTS.md](AGENTS.md) — Agent and engineering working principles.
- [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md) and [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) — CLI routing and delivery contracts.
- [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md) — Stage 2 optional ChatGPT bridge architecture (preserved on `origin/main` and `rewrite/rust-core`).
- [docs/history/GENERATION_ADMISSION.md](docs/history/GENERATION_ADMISSION.md) — Historical generation admission architecture record.
