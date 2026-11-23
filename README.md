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
  `codex doctor [--color | --json]`, and `codex wrapper status [--verify] [--json]`.
- **Stage 2 (decoupled standalone tools & services):** lightweight `SHA256SUMS.sig`
  Ed25519 signed releases, standalone `codex-ext` (`cdx`) for profiles/notifications/switching,
  independent live MCP servers (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`),
  multi-lane `chatgpt-provider` + `host/chatgpt-desktop`, `transport/tunnel-client`,
  and optional durable orchestration (`@TermuxOrch`), all separate from the
  `codex` binary. See the
  [architecture and ownership map](docs/ARCHITECTURE.md),
  [current evidence](CURRENT.md) and [roadmap](docs/ROADMAP.md).

For related `@TermuxOrch` Works, pass the same `project_scope` such as
`humtr/codex`. Set `project_name: "Wrapper"` on the first Work if you want a
shorter display name. Add `@branch` to the scope for a separate branch project.
Set `session_prefixes` to customize the role prefixes for each Work.
[Project and session naming](docs/spec/ORCHESTRATION.md#3-project-organization-and-consistent-session-naming)

The account-free ChatGPT Desktop boot probe uses the AppHost supervisor from
the matching `chatgpt-provider` build. Use
`manager.py probe --provider-bin /path/to/chatgpt-provider` to test a local build
with a temporary profile. See the [AppHost contract](docs/spec/CHATGPT_PROVIDER.md).

## Requirements on Termux

Stage 1 targets Android AArch64 (`aarch64-linux-android`, Android API 24+) inside
Termux and relies only on base Termux packages (plus base `Essential` `procps` `ps`
needed by upstream `app-server daemon` PID tracking):

```sh
pkg install curl gzip
```

`install.sh` uses `sha256sum` from Termux `coreutils` and `openssl` (`openssl-tool`)
to verify Ed25519-signed GitHub Release `SHA256SUMS.sig` manifests. Wrapper and
the release builder use built-in SHA-256, so ordinary `codex update --upstream`
runs without `openssl-tool`.

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

The stable `wrapper` release contains the standalone Stage 1 binary:

```sh
curl -fsSL https://github.com/humtr/codex/releases/download/wrapper/install.sh | sh
```

This downloads and verifies the Ed25519-signed `SHA256SUMS` + `SHA256SUMS.sig`
manifest against the pinned Stage 2 public key ([release-public-key.pem](release/release-public-key.pem))
with monotonic `sequence` anti-rollback, verifies the release archive and
extracted `bin/codex` SHA-256, installs `$PREFIX/bin/codex`, and bootstraps an
active adapted upstream Codex generation; see
[current operating state](CURRENT.md#current-live-operating-snapshot).

Useful installer options (`./install.sh --help`):

- `--prefix <DIR>`: override `$PREFIX` (defaults to `$PREFIX` or `/data/data/com.termux/files/usr`).
- `--release <TAG>`: download signed `SHA256SUMS`, `SHA256SUMS.sig`, and `codex-wrapper-aarch64-linux-android.tar.gz` from a specific GitHub Release tag (defaults to `wrapper` rather than `/releases/latest`).
- `--verify-key <PEM>` / `--key-sha256 <HEX>` / `--require-signed`: override or strictly pin the Ed25519 public key used to verify `SHA256SUMS.sig`.
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
- Interactive updates display download progress (`downloaded / total`), Termux
  patching, and daemon restart progress on one terminal row.
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

On a bare `codex` launch with all three streams attached to terminals, Wrapper
checks for a newer official stable Codex release using a 15-minute local cache
(`~/.local/share/codex/wrapper/update-check.cache`) and a five-second deadline.
Press `y` or `Y` to update immediately. Press `n`, `N`, or Enter to decline.
Declining or an update failure defers the offer for the remaining cache lifetime
and opens the active Codex. Set `CODEX_DISABLE_UPDATE_PROMPT=1` to skip the startup check.
An explicitly set `CODEX_TERMUX_UPDATE_VERSION` also skips it.

If a regular file or directory occupies `$HOME/.codex/packages/standalone/current`,
update and rollback fail without replacing it or changing the active generation.
If an interrupted update leaves its result uncertain, use `codex wrapper status`
to recover and inspect the selection before deciding whether to roll back again.

### Managed daemon binaries and profiles

Each profile with a running managed daemon has its own PID and control socket.
Once aligned, all profile links resolve to one active executable. A new
generation changes the target while keeping each profile's logical launch path.

```text
$CODEX_HOME/packages/app-server-daemon/current/bin/codex
  -> $HOME/.codex/packages/standalone/current/bin/codex
  -> $HOME/.codex/packages/standalone/releases/<id>/bin/codex
```

On an eligible first TUI launch or explicit daemon start in a fresh home,
Wrapper can create the default package link. Upstream decides when to start
the process, so the link can exist during sign-in or with
`features.daemon_auto_start = false`. Help, version, queries, `--no-daemon`,
`--remote`, and direct `app-server daemon update --from-cli` skip this link setup.

A successful `codex update`, including an already-current result, `--path`,
or `--rollback`, synchronizes eligible initialized homes. Wrapper looks in
the invocation's `$CODEX_HOME`, `$HOME/.codex`, and owned Manager profile homes
under `$HOME/.local/share/codex/manager/profiles/*/home`. It aligns the
package link in a stopped home. It keeps the PID of a matching running daemon
when no migration is needed. A verified running daemon is restarted when its
executable or version differs from the active generation, or when its legacy
PID path needs migration.
Restarting can interrupt active or queued work. Completion output summarizes
restarted, link-aligned, already-current, deferred, and failed homes by profile
name; if `codex update` runs inside a session hosted by a daemon that needs a
restart, Wrapper aligns its link and defers restarting that ancestor daemon
with a warning instead of terminating the running update. Check stderr warnings
for per-home details and use `codex app-server daemon version` in that profile
to verify the running version.

Use `codex update` for the shared Termux package. The upstream TUI `/daemon`
menu and `codex app-server daemon update` can select a separate package for
one home. The public-release option can select a package without Termux
adaptation. The `--from-cli` option copies the full adapted package to that
home's `packages/app-server-daemon/releases/` and selects it for the same
home's daemon. A subsequent Wrapper update can reselect the shared executable.
Dedicated package files and the shared rollback generation can remain on disk.

### Standalone companion (`codex-ext` / `cdx`)

Stage 1 `install.sh` installs only `$PREFIX/bin/codex` and leaves profile
CRUD, notifications, and in-session switching completely out of the standalone
Wrapper. Those features live in the independent Stage 2 companion crate
[crates/codex-ext](crates/codex-ext/Cargo.toml) (`$PREFIX/bin/codex-ext` +
`$PREFIX/bin/cdx` symlink, superseding the retired `contrib/codex-account`
script while remaining 100% compatible with existing
`~/.local/share/codex/manager/profiles/<NAME>/home` directories):

```sh
# Build locally from source (independent workspace; zero impact on crates/wrapper)
cargo build --release --locked --manifest-path crates/codex-ext/Cargo.toml
install -m 0755 crates/codex-ext/target/release/codex-ext "$PREFIX/bin/codex-ext"
ln -sf codex-ext "$PREFIX/bin/cdx"

cdx                                      # launch Codex in default profile (non-PTY, lossless OSC 52)
cdx use szob                             # launch Codex in named OpenAI OAuth profile
cdx profile list                         # list profiles, auth presence, and daemon status
cdx profile create work                  # create a new isolated OpenAI OAuth profile
cdx notify set --enable --events Stop    # configure Termux:API notifications (config.toml Stop / hooks.json extras)
# Inside Codex TUI (`!` shell):
!cdx switch szob                         # from a cdx-launched TUI, waits for its own shell turn and resumes the same thread
!cdx provider chatgpt                    # waits for its own shell turn, then opens a new ChatGPT thread in this window
```

Per-account V2 config (`-p <name>`) reads `<name>.config.toml` from the selected
account's `$CODEX_HOME`. With `CODEX_SHARED_HOME`, `installation_id` and
`config.toml` are shared by default; `codex-ext` automatically includes
`hooks.json` in `CODEX_SHARED_EXTRA_FILES` while preserving any additional
colon-separated basenames you configure. Credentials such as `auth.json`,
database files, and lock files are reserved and cannot be opted in. Session
index merges preserve separate entries for renamed threads.

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

If [daemon synchronization](#managed-daemon-binaries-and-profiles) reports a
start failure after a confirmed stop, resolve the reported error and run
`codex app-server daemon start` from that profile.

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

1. **`checks` (`ubuntu-24.04-arm`):** YAML syntax, `sh -n` and `shellcheck` for `install.sh`, workspace and lockfile scope, Git-tracked Markdown links and heading anchors, Rust formatting, clippy, and workspace tests including the emitted AArch64 `realpath` hook.
2. **`android-package` (`ubuntu-24.04`):** cross-compiles `codex` and `codex-release-builder` for `aarch64-linux-android` (API 24, Android NDK `27.3.13750724`), verifies `ARM aarch64` ELF headers and `/system/bin/linker64`, bundles `codex-wrapper-aarch64-linux-android.tar.gz` + `.sha256` + `SHA256SUMS` + `install.sh`, and uploads the workflow artifact.
3. **`android-smoke` (`ubuntu-24.04-arm`):** provisions pinned Android Runtime APEX `linker64` and Bionic libraries, installs the packaged bundle via `install.sh --skip-upstream`, and executes the real Android binary (`codex update --help`, `codex wrapper status --json`, `codex --sandbox read-only --version`, `codex sandbox linux true`).
4. **`release` (tag `wrapper`, `wrapper-*`, or `v*`, or manual `workflow_dispatch` with `publish_release=true`):** requires official upstream runtime qualification before signing `SHA256SUMS` + `SHA256SUMS.sig` (Ed25519 with monotonic `sequence`) and publishing the verified bundle, `.sha256`, `install.sh`, `SHA256SUMS`, and `SHA256SUMS.sig`. Publication is serialized per tag and succeeds only after readback verifies the peeled tag commit, notes, prerelease setting, Ed25519 signature, and uploaded asset bytes. A mismatched annotated tag requires manual reconciliation.

Run the read-only upstream canary on `main` with the existing workflow.

```sh
gh workflow run wrapper.yml --ref main -f qualify_upstream=true -f publish_release=false
```

It discovers the latest official stable input by default. Set `upstream_version`
for an exact version and `previous_version` to qualify a cross-version upgrade
and rollback. Read the `wrapper-upstream-evidence` artifact for the input and
qualification reports.

Local `codex update` continues to qualify official OpenAI releases independently
with built-in SHA-256 (no signing key or `openssl-tool` required). CI with a
Bionic loader on Linux and actual Android execution remain distinct evidence
boundaries.

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
- [docs/ROADMAP.md](docs/ROADMAP.md) — Stage 2 decoupled slice summary, follow-up verification criteria, and deferred Phase 2.6 canary notes.
- [AGENTS.md](AGENTS.md) — Agent and engineering working principles.
- [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) and [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md) — Detailed delivery, recovery, Ed25519 signing, CLI, and `codex-ext` (`cdx`) contracts.
- [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md) — Stage 2 3-MCP (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`) and `chatgpt-provider` overview contract.
- [docs/spec/MCP_SHELL.md](docs/spec/MCP_SHELL.md), [docs/spec/MCP_CODEX.md](docs/spec/MCP_CODEX.md), [docs/spec/MCP_CHATGPT.md](docs/spec/MCP_CHATGPT.md), [docs/spec/CHATGPT_PROVIDER.md](docs/spec/CHATGPT_PROVIDER.md), [docs/spec/SUPERVISION.md](docs/spec/SUPERVISION.md), and [docs/spec/ORCHESTRATION.md](docs/spec/ORCHESTRATION.md) — Normative annexes for each independent Stage 2 component, opt-in `runit` 3-state supervision, and Slice 7 durable orchestration (`@TermuxOrch`).
- [docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md) — Archived 2026-11-16 – 2026-11-19 Stage 1 milestones and retired work orders.
- [docs/history/GENERATION_ADMISSION.md](docs/history/GENERATION_ADMISSION.md) and [docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md) — Archived pre-rewrite admission records and `rewrite/rust-core` (`0.167.x`) evidence.
