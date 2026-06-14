# Codex Termux Standalone Upstream Wrapper Spec

## Goal

Install the official upstream Codex Linux ARM64 npm package as a Termux-managed
runtime. `$PREFIX/bin/codex` is the managed launcher.

The wrapper preserves the upstream package resource layout, patches the static
musl DNS resolver path, and keeps existing Codex auth/config state user-owned.
It installs a runtime-private Termux `bwrap` compatibility launcher and places
that path first only while running Codex, so upstream Codex can execute commands
on Android without creating Linux namespaces or adding a public `bwrap`.

## Managed Paths

- `~/.local/lib/codex/native/raw/`
- `~/.local/lib/codex/native/runtime/`
- `~/.local/share/codex/native/state.json`
- `~/.local/share/codex/native/registry.json`
- `$PREFIX/bin/codex`

Existing non-managed `codex` launchers are backed up under
`~/.local/share/codex/native/backups/` before replacement.

## Public Commands

- `codex` runs upstream Codex.
- A prompt-like first argument is routed as `exec`.
- `setup`, `update`, `doctor`, `version`, `help`, `use`, `profile`, and
  `remove` are wrapper lifecycle commands.
- `--` forces upstream passthrough.
- `doctor` without arguments runs upstream human doctor and then appends wrapper
  diagnostics. `doctor` with arguments passes them to upstream doctor unchanged.
- Plain upstream execution checks the npm `linux-arm64` dist-tag at most once
  every six hours while the installed runtime is current. Once a newer runtime
  is detected, interactive execution prompts every time until the runtime is
  updated. Pressing `1` keeps running the current patched runtime; pressing `0`
  updates, patches, and then continues on the latest runtime. Non-interactive
  execution never prompts. `CODEX_NATIVE_AUTO_UPDATE_MODE=force` restores forced
  updating, and `CODEX_NATIVE_AUTO_UPDATE=0` disables the check.
- Runtime drift is self-repaired from the cached raw vendor package when
  possible. This covers support-tool changes such as bwrap/rg shim updates
  without requiring another npm download.
- `use` lists cached runtime snapshots from `~/.local/share/codex/native/store/`
  and promotes the selected snapshot without changing auth/config state.
- `profile` lists profile directories under `~/.codex-profiles/` and launches
  upstream Codex with `CODEX_HOME` set to the selected profile. `default` maps
  to `~/.codex`. Named profiles link `plugins` to `~/.codex/plugins` when the
  profile does not already have its own `plugins` entry.
- Profile execution preserves the selected `CODEX_HOME/config.toml` byte for
  byte. Network and approval policy remain owned by upstream Codex and the user.
- `runtime/codex-path/bwrap` is a managed compatibility launcher selected
  through the Codex-only runtime PATH. It advertises the
  bwrap flags Codex probes for, ignores namespace/mount setup that Android
  cannot perform, applies execution-relevant env/cwd options, and execs the
  command after `--`. Normal execution is quiet; verbose compat diagnostics are
  enabled only with `CODEX_NATIVE_BWRAP_COMPAT_VERBOSE=1`.

## Runtime Registry

`state.json` and `registry.json` record the raw package, wrapper support
version, and promoted runtime as a tuple. The legacy `installs` list remains for
`codex use`, while the tuple fields make self-repair auditable:

- `raw`: upstream official package binary hashes.
- `wrapper`: local wrapper version and commit.
- `runtime`: runtime copies produced by a raw/wrapper tuple.
- `active_tuple_id`: the tuple currently promoted into the managed runtime.

## Accepted Termux Delta

The wrapper accepts the Termux compatibility delta that can be implemented
without replacing the official binary with an Android-native fork:

- Browser login uses `termux-open-url` when Termux provides it and `BROWSER` is
  not already set.
- `LD_LIBRARY_PATH` and `LD_PRELOAD` are sanitized before Codex runtime
  execution.
- `CODEX_SELF_EXE` is preserved as the managed runtime path instead of the raw
  upstream binary.
- Runtime updates rebuild from the official raw package through the local
  Termux patcher and record the raw/wrapper/runtime tuple.
- bwrap namespace failures are not surfaced during ordinary execution; the
  compatibility launcher provides command execution without pretending to offer
  Linux namespace isolation.
- The compatibility launcher does not provide filesystem namespace isolation,
  but upstream Codex network-off execution still applies a seccomp socket
  boundary. The wrapper preserves that policy and verifies off/on/reset behavior.
- Runtime binary mutation is limited to the byte-length-preserving resolver path
  rewrite. Each runtime records a build manifest and is checked before execution.
- Compatible cached runtimes are capped by `CODEX_NATIVE_RUNTIME_RETENTION`,
  which defaults to three.

## Update Source

The default package source is `@openai/codex@linux-arm64`, which follows the npm
`linux-arm64` dist-tag. Explicit versions such as `0.137.0` are normalized to
`@openai/codex@0.137.0-linux-arm64`.

## Non-goals

This wrapper does not make Android support Linux bubblewrap namespace
isolation. The runtime-private compatibility launcher is a deliberate
no-namespace execution path for Termux; it preserves Codex command execution
but does not provide the isolation guarantees of real bubblewrap on Linux.

This wrapper also does not claim Android-native Codex binary features that
require rebuilding the Rust project for Bionic/Termux, such as
`RUNPATH=$ORIGIN`, bundled `libc++_shared.so`, Android-native PTY/lock patches,
voice/realtime package pruning, or a Termux-built `codex-exec`. Those belong to
an Android-native fork/package strategy rather than this official-runtime
wrapper strategy.
