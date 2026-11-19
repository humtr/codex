# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

**Keeping up with upstream is a permanent product requirement.** This wrapper
continues to adopt official stable Codex releases and their features. We adapt
the Termux layer, remove local code superseded by upstream and improve the
existing automated upgrade path. Compatibility failures require repair; the
current working release remains available until the next is qualified. This
applies to future releases as well as the current target. See the
[continuous upstream adoption invariant](SPEC.md#2-continuous-upstream-adoption).

The product has one public command, `codex`, built in two decoupled stages:

- **Stage 1 (active standalone Wrapper):** a standalone Rust Wrapper (`crates/wrapper`)
  and adaptive builder (`crates/release-builder`) with zero external crate dependencies
  beyond `libc` that provides Termux runtime compatibility (FD 33/34/35, wrapper `config.toml`,
  timezone TZif, TLS certs, browser helpers, sandbox guard, `app-server daemon` &
  `remote-control` passthrough), `codex update [--upstream] [--version <M.m.p>] [--force-rebuild] | --path <FILE_OR_DIR> | --rollback`,
  `codex doctor [--color | --json]`, and `codex wrapper status [--json]`.
- **Stage 2.1 extension interface (implemented, not attached):** `wrapper.toml`,
  `codex wrapper setup`, `--bare`, unavailable-module `--with` admission,
  and conditional FD 34 snapshot preparation. These do **not** launch optional
  modules; the production empty-fragment path bypasses the snapshot engine.
- **Stage 2.2–2.5 (deferred):** Ed25519 `--signed` Wrapper/addon
  verification, Manager, enhanced `TUI`, and Wrapper-managed `Provider` bridge.
  Earlier source is preserved on `rewrite/rust-core` for selective integration,
  not included in the Stage 1 binary. See the
  [architecture and ownership map](docs/ARCHITECTURE.md),
  [current evidence](CURRENT.md) and [roadmap](docs/ROADMAP.md).

## Requirements on Termux

Stage 1 targets Android AArch64 (`aarch64-linux-android`, Android API 24+) inside
Termux and relies only on base Termux packages (plus base `Essential` `procps` `ps`
needed by upstream `app-server daemon` PID tracking):

```sh
pkg install curl gzip
```

`install.sh` uses `sha256sum` from Termux `coreutils` to verify release tarballs.
Wrapper and the release builder use built-in SHA-256, so Stage 1 runs without
`openssl-tool`.

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

The currently published `wrapper` **prerelease** contains the accepted
standalone Stage 1 P0 binary (the public asset is already verified in CURRENT):

```sh
curl -fsSL https://github.com/humtr/codex/releases/download/wrapper/install.sh | sh
```

This installs the standalone `codex` wrapper binary, verifies its SHA-256
digest, and bootstraps an active adapted upstream Codex generation; see
[current operating state](CURRENT.md#current-live-operating-snapshot-2026-11-19).
The standalone release uses HTTPS plus SHA-256 checksum verification, **not
Stage 2.2 Ed25519-signed addon distribution**.

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

For an offline Wrapper-only upgrade, run `./install.sh --binary /path/to/new/codex`.
If the new Wrapper qualifies the active generation by Wrapper API, persistent
schema, and patch policy, the installer keeps both `current` and `previous`
without calling the updater or downloading an upstream archive.

After `codex` is installed:

```sh
# Fetch, verify (HTTPS + official OpenAI release SHA-256), patch adaptively, and activate latest upstream Codex
codex update --upstream

# Install a specific upstream version
codex update --upstream --version <X.Y.Z>

# Download, verify, and re-adapt a release even when it is already current
codex update --version <X.Y.Z> --force-rebuild

# Install from an official upstream package .tar.gz or a prepared generation directory
codex update --path /path/to/codex-package-aarch64-unknown-linux-musl.tar.gz

# Roll back to the immediately preceding activated generation
codex update --rollback

# Inspect official release inputs without downloading or changing installed state
codex update --check --json
```

Every generation is stored under `~/.codex/packages/standalone/releases/<id>/`
(normally named `<version>-<6hex>`, e.g. `0.169.1-a1b2c3`, with
`~/.codex/packages/standalone/current` pointing to the active release and a
`previous` pointer retained for single-step rollback). State owned by the wrapper
(`resolv.conf`, wrapper `config.toml`, timezone TZif cache, `activation-state`,
`downloads/`) lives under `~/.local/share/codex/wrapper/` and `~/.cache/codex/timezones/`.

- `--signed`, `--remote`, `--build-local`, and `--local` are disabled in Stage 1 and fail explicitly with exit status `2`.
- Official updates show live download progress (`downloaded / total`) and transient Termux patching section status on interactive TTYs.
- A compatible active generation with the requested version and official archive SHA-256 is reused by default. Use `--force-rebuild` with an official upstream update to re-adapt it explicitly.
- External launchers/addons may isolate `auth.json` per profile (`CODEX_HOME=<profile_home>`) while sharing sessions, SQLite databases (`CODEX_SQLITE_HOME`), and `config.toml`/skills via `CODEX_SHARED_HOME="$HOME/.codex"` without adding profile CRUD to the standalone Wrapper.
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
in that generation (`~/.codex/packages/standalone/releases/<version>-<6hex>/`).
Each launch still prepares process-local FD 33/34/35 and the
Termux environment. The Wrapper refreshes older managed system config on
launch and reuses unchanged files. The installed upstream Codex supplies
the thread unload default. Explicit `config.toml` and `-c` settings retain
upstream precedence, including an explicit zero.

On the first TUI launch eligible to use a managed daemon in a fresh
`$CODEX_HOME`, the Wrapper can prepare the default package selection by linking
`$CODEX_HOME/packages/app-server-daemon/current` to
`$HOME/.codex/packages/standalone/current`. Explicit daemon start commands
can prepare the same default. Codex decides whether to start a daemon, so the
link can appear even with `features.daemon_auto_start = false` or during sign-in.
Help, version, queries, `--no-daemon`, `--remote`, and direct `app-server daemon
update --from-cli` skip this preparation.

Homes using the link pick up a Wrapper update or rollback on the next daemon
start or restart. A running daemon keeps using its original executable.
Existing per-home selections and homes with prior daemon launch traces keep
their upstream-selected packages. A later explicit `app-server daemon update
--from-cli --yes` can pin a dedicated package independently of Wrapper updates.

If a regular file or directory occupies `$HOME/.codex/packages/standalone/current`,
update and rollback fail without replacing it or changing the active generation.
If an interrupted update leaves its result uncertain, use `codex wrapper status`
to recover and inspect the selection before deciding whether to roll back again.

### Optional account selector (`contrib/codex-account`)

Stage 1 `install.sh` installs only `$PREFIX/bin/codex` and leaves profile
registration/management out of the standalone Wrapper. For convenience when
running already-registered account homes (`~/.local/share/codex/manager/profiles/<ACCOUNT>/home`)
directly without `ai` or Stage 2 Manager, you can optionally copy
[contrib/codex-account](contrib/codex-account) to `~/bin/codex-account`
(preserving any existing file you already placed there):

```sh
mkdir -p ~/bin && cp -i contrib/codex-account ~/bin/codex-account && chmod 700 ~/bin/codex-account
```

Usage (`codex-account ACCOUNT [CODEX_ARG ...]`) consumes only `ACCOUNT`, sets
`CODEX_HOME`, `CODEX_SHARED_HOME="$HOME/.codex"`, and
`CODEX_SQLITE_HOME="$HOME/.codex"` for registered accounts (or unsets all three
for `default` / `home`), and passes all remaining arguments directly to
`$PREFIX/bin/codex` without requiring `--`:

```sh
contrib/codex-account --help                    # show the selector's own usage from a checkout
codex-account work                              # start Codex TUI in work home
codex-account personal resume <UUID>            # resume thread in personal home
codex-account work -p review exec "review this" # apply work home + review.config.toml
codex-account default                           # unset inherited profile env and use ~/.codex
codex-account work --help                       # pass --help through to Codex
```

Per-account V2 config (`-p <name>`) reads `<name>.config.toml` from the selected
account's `$CODEX_HOME`. With `CODEX_SHARED_HOME`, `installation_id` and
`config.toml` are shared by default. To share other config files already present
in the shared home, set `CODEX_SHARED_EXTRA_FILES` to their colon-separated
basenames. For example, `CODEX_SHARED_EXTRA_FILES=review.config.toml:team.config.toml`
opts in both files. Credentials such as `auth.json`, database files, and lock
files are reserved and cannot be opted in. Session index merges preserve separate
entries for renamed threads.

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

`--with` currently validates the module names then **always fails with
unavailable status (exit 1)**: no Manager/TUI/Provider executables are admitted
or dispatched by this source. An enabled but unavailable module stored in
persistent settings emits a warning and ordinary `codex` continues with
standalone upstream, so a saved opt-in is **not** evidence of running integration.
`--bare` bypasses enabled
addon modules for one launch while preserving upstream arguments (`wrapper setup
--bare` disables addon modules while retaining `signed_update` when the settings
are valid). If `wrapper.toml` is a malformed regular file, run
`codex wrapper setup --bare` alone to restore the bare defaults, including
`signed_update = false`. Setting
`signed_update = true` records the opt-in but does **not** activate a signed
channel until Phase 2.2; ordinary `codex update` remains independent, and
actual Manager/TUI/Provider addon reattachment remains deferred to Phases 2.3–2.5.
The no-addon launch adds no *addon* dependency, process, lock, snapshot write,
or extra config probe. Existing Wrapper generation retention and activation
recovery remain in force.

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

`codex doctor` is read-only. It inspects generation state and shared-home
bindings without repairing them, and uses disposable runtime preparation under
`$TMPDIR`. The output reports Wrapper health and embeds the upstream diagnosis.
An overall `healthy` result can include upstream warnings such as
`state.rollout_db_parity`.

Run Wrapper commands directly. Leading upstream options on commands such as
`codex -c k=v doctor` or `codex -c k=v wrapper status` return exit `2`.

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

The background server can outlive the TUI. A running daemon keeps its
startup settings and reads the updated Wrapper defaults on its next normal
start through that Wrapper. Cross-home resume depends on actual native
writer release and upstream resume checks. Authentication remains per home.

Android kernels lack Linux Landlock / unprivileged user namespaces, so the Stage 1
wrapper sets `sandbox_mode = "danger-full-access"` in its wrapper-owned config and
rejects incompatible `--sandbox read-only|workspace-write` or `codex sandbox linux`
invocations cleanly with exit code `2`.

## What the Stage 1 Wrapper Handles at Launch

Before `exec`ing upstream's static musl `codex` binary (plus optional
`codex-code-mode-host` and `codex-exec-server` helpers patched adaptively at install
time), the wrapper prepares the Android/Termux environment:

1. **DNS (`FD 33`):** materializes `~/.local/share/codex/wrapper/resolv.conf` when needed and redirects `/etc/resolv.conf` reads to `FD 33`.
2. **Wrapper config (`FD 34`)** lazily ensures `~/.local/share/codex/wrapper/config/config.toml` (`sandbox_mode = "danger-full-access"`, `mcp_oauth_credentials_store = "file"`) and redirects `/etc/codex` reads to FD 34.
3. **Temp root & UDS directory (`FD 35` + `TMPDIR`):** requires inherited `PREFIX` and `TMPDIR`, exports `TMPDIR`/`TMP`/`TEMP`/`SQLITE_TMPDIR`, and binds `FD 35` so static musl temp/socket paths do not hit read-only `/tmp`.
4. **Timezone (`TZ`):** writes/reuses a valid `TZif` under `$XDG_CACHE_HOME/codex/timezones` (or `~/.cache/codex/timezones`) from Android `persist.sys.timezone` / `$TZ`.
5. **TLS & environment fence:** injects Termux `SSL_CERT_FILE` / `SSL_CERT_DIR`, disables upstream daemon auto-updates (`$CODEX_HOME/app-server-daemon/settings.json` defaulting to `~/.codex/app-server-daemon/settings.json` -> `{"updater":{"autoUpdateEnabled":false}}` on normal launches), and neutralizes unreachable host X11/Wayland/WSL browser env vars.

Fast inspection flags (`--version`, `-V`, `--help`, `-h`, and top-level `help`) bypass settings writes cleanly.

## CI and Building From Source

### Wrapper CI (`.github/workflows/wrapper.yml`)

Every push or pull request on `main`, `wrapper`, or `rewrite/wrapper` runs:

1. **`checks` (`ubuntu-24.04-arm`):** YAML syntax, `sh -n` and `shellcheck` for `install.sh` and `contrib/codex-account`, workspace and lockfile scope, Git-tracked Markdown links and heading anchors, Rust formatting, clippy, and workspace tests including the emitted AArch64 `realpath` hook.
2. **`android-package` (`ubuntu-24.04`):** cross-compiles `codex` and `codex-release-builder` for `aarch64-linux-android` (API 24, Android NDK `27.3.13750724`), verifies `ARM aarch64` ELF headers and `/system/bin/linker64`, bundles `codex-wrapper-aarch64-linux-android.tar.gz` + `.sha256` + `SHA256SUMS` + `install.sh`, and uploads the workflow artifact.
3. **`android-smoke` (`ubuntu-24.04-arm`):** provisions pinned Android Runtime APEX `linker64` and Bionic libraries, installs the packaged bundle via `install.sh --skip-upstream`, and executes the real Android binary (`codex update --help`, `codex wrapper status --json`, `codex --sandbox read-only --version`, `codex sandbox linux true`).
4. **`release` (tag `wrapper`, `wrapper-*`, or `v*`, or manual `workflow_dispatch` with `publish_release=true`):** requires official upstream runtime qualification before publishing the verified bundle, `.sha256`, `install.sh`, and `SHA256SUMS`. Publication is serialized per tag and succeeds only after readback verifies the peeled tag commit, notes, prerelease setting, and uploaded asset bytes. A mismatched annotated tag requires manual reconciliation.

Manual verification can also request upstream qualification and supply
`previous_version` to test upgrade and rollback across two official releases.
Local `codex update` continues to qualify candidates independently. CI with a Bionic loader on Linux and
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

Start at the [documentation map](docs/INDEX.md) for document authority,
lifecycle rules, and status vocabulary; the
[architecture and ownership map](docs/ARCHITECTURE.md)
ties active commands to code and distinguishes integrated from deferred modules.

- [SPEC.md](SPEC.md) — Settled top-level product invariants.
- [CURRENT.md](CURRENT.md) — Current live operating state and recent verified evidence.
- [docs/ROADMAP.md](docs/ROADMAP.md) — Deferred Stage 2 reintegration roadmap, lineage warnings, and follow-up verification criteria.
- [AGENTS.md](AGENTS.md) — Agent and engineering working principles.
- [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) and [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md) — Detailed delivery, recovery, CLI, and conditional Stage 2 contracts.
- [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md) — Deferred Stage 2 ChatGPT Bridge/Provider contract (source preserved on `rewrite/rust-core`).
- [docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md) — Archived 2026-11-16 – 2026-11-19 Stage 1 milestones and retired work orders.
- [docs/history/GENERATION_ADMISSION.md](docs/history/GENERATION_ADMISSION.md) and [docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md) — Archived pre-rewrite admission records and `rewrite/rust-core` (`0.167.x`) evidence.
