# Codex Termux Standalone Upstream Wrapper Spec

## Goal

Install the official upstream Codex Linux ARM64 npm package as a Termux-managed
runtime. `$PREFIX/bin/codex` is the managed launcher.

The wrapper preserves the upstream package resource layout, patches the static
musl DNS resolver path, and keeps existing Codex auth/config state user-owned.
It installs a managed Termux `bwrap` compatibility launcher in
`$PREFIX/bin/bwrap` so upstream Codex selects a system bubblewrap path that can
execute commands on Android without creating Linux namespaces.

## Managed Paths

- `~/.local/lib/codex/native/raw/`
- `~/.local/lib/codex/native/runtime/`
- `~/.local/share/codex/native/state.json`
- `~/.local/share/codex/native/registry.json`
- `$PREFIX/bin/codex`
- `$PREFIX/bin/bwrap`

Existing non-managed `codex` launchers are backed up under
`~/.local/share/codex/native/backups/` before replacement.

## Public Commands

- `codex` runs upstream Codex.
- A prompt-like first argument is routed as `exec`.
- `setup`, `update`, `doctor`, `version`, `help`, `use`, `profile`, and
  `remove` are wrapper lifecycle commands.
- `--` forces upstream passthrough.
- `doctor --upstream` runs upstream `doctor`.
- Plain upstream execution checks the npm `linux-arm64` dist-tag at most once
  every six hours and updates before continuing when a newer runtime exists.
- `use` lists cached runtime snapshots from `~/.local/share/codex/native/store/`
  and promotes the selected snapshot without changing auth/config state.
- `profile` lists profile directories under `~/.codex-profiles/` and launches
  upstream Codex with `CODEX_HOME` set to the selected profile. `default` maps
  to `~/.codex`.
- `$PREFIX/bin/bwrap` is a managed compatibility launcher. It advertises the
  bwrap flags Codex probes for, ignores namespace/mount setup that Android
  cannot perform, applies execution-relevant env/cwd options, and execs the
  command after `--`.

## Update Source

The default package source is `@openai/codex@linux-arm64`, which follows the npm
`linux-arm64` dist-tag. Explicit versions such as `0.137.0` are normalized to
`@openai/codex@0.137.0-linux-arm64`.

## Non-goals

This wrapper does not make Android support Linux bubblewrap namespace
isolation. The managed `$PREFIX/bin/bwrap` compatibility launcher is a
deliberate no-namespace execution path for Termux; it preserves Codex command
execution but does not provide the isolation guarantees of real bubblewrap on
Linux.
