# Codex Termux Wrapper

Codex Termux Wrapper installs and runs the official `@openai/codex` linux-arm64 package inside Termux. It keeps upstream Codex behavior intact wherever possible and adds only the Termux compatibility layer needed to launch the upstream runtime reliably.

Release packages intentionally contain only runtime/install code. Source checkouts may include repository-only tests under `tests/` for development validation; release packages exclude them.

## Philosophy

The wrapper is not a fork of upstream Codex behavior. It is a thin Termux launcher/runtime manager.

- Preserve upstream Codex defaults wherever possible.
- Patch only the parts that do not work in Termux.
- Keep wrapper-specific state under explicit Termux wrapper paths.
- Avoid compatibility aliases, migrations, or legacy internal names.
- Treat custom profiles as explicit `CODEX_HOME` switches, not as a separate product layer.

## What the wrapper changes

The wrapper does four things:

1. Installs the official upstream `@openai/codex` linux-arm64 package.
2. Patches the raw binary into a Termux-compatible runtime.
3. Installs Termux-compatible `bwrap` and `rg` shims beside the runtime.
4. Exposes a managed `codex` launcher.

The wrapper does not replace upstream Codex commands. Unknown commands and normal bare execution are passed through to the active runtime after the wrapper has ensured that the runtime is ready.

## Install

```sh
bash install.sh
```

After installation, use the managed launcher:

```sh
codex
```

## Runtime model

The wrapper uses these terms consistently:

- **upstream package**: the official `@openai/codex` linux-arm64 package downloaded from npm.
- **raw binary**: the upstream `codex` executable before Termux patching.
- **runtime**: the patched executable that can run in Termux.
- **runtime bundle**: the runtime plus its support files and shims.
- **active runtime**: the runtime currently selected by the managed launcher.
- **verified runtime**: the rollback baseline kept after a successful activation.

The runtime patch remaps the binary's Termux-incompatible system paths through inherited file descriptors. The launcher opens the Termux resolver source on fd 33 and the managed Codex system config directory on fd 34 before running the runtime. These descriptors are runtime patch contracts, not configurable user options.

## Commands

```sh
codex
```

Runs the managed upstream Codex runtime. The wrapper may repair, update, or roll back the managed runtime before launch. Bare `codex` launches the most recently selected profile, or `default` when no recent profile has been recorded.

```sh
codex install
```

Refreshes wrapper support from the install source, downloads a fresh upstream package, patches a runtime bundle, activates it, and updates the verified rollback baseline.

For fresh wrapper commands, the installer can use a release archive when configured:

```sh
CODEX_TERMUX_WRAPPER_RELEASE_URL=https://example.invalid/codex-termux.tar.gz codex install
```

`CODEX_TERMUX_WRAPPER_RELEASE_SHA256` may be set to pin the archive checksum. Without release settings, the current install source is used and stored under the managed support directory for later `codex install` and `codex rebuild` calls.

Turn-completion notification behavior can be configured in:

```sh
~/.local/share/codex/termux/notify/config.env
```

Example:

```sh
CODEX_TERMUX_NOTIFY_CONTENT_CHARS=0
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES=1
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=middle
```

Use `CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top`, `middle`, or `bottom` to place the toast. Set `CODEX_TERMUX_NOTIFY_CONTENT_CHARS=0` to pass the full assistant message to the Android notification content.

```sh
codex update
```

Downloads the selected or latest upstream package and builds it with the current installed wrapper.

```sh
codex rebuild
```

Refreshes wrapper support from the install source and rebuilds the runtime from the cached raw package without fetching upstream Codex.

```sh
codex repair
```

Rebuilds the runtime from the cached raw package with the current installed wrapper without network access.

```sh
codex use
codex use --list
codex use <selection>
```

Lists cached and remote runtimes, then promotes the selected runtime. Selection accepts menu numbers and available runtime versions.

```sh
codex session
```

Interactive curses-based TUI picker to resume any discovered Codex session across any target profile.

```sh
codex doctor
```

Runs upstream `codex doctor` first, prints a separator, then runs the wrapper doctor checks.

```sh
codex doctor <args>
```

Passes arguments directly to upstream `codex doctor`.

```sh
codex profile
codex profile <name>
```

Lists profiles or launches the runtime with a selected profile.

```sh
codex version
```

Prints upstream Codex and the active runtime creation date.

```sh
codex remove
```

Removes the managed launcher/runtime and restores launcher backups when present. State is kept for backups.

## Profiles

Profiles are `CODEX_HOME` switches.

### Default profile

The `default` profile is upstream Codex's normal default behavior.

- The wrapper does not create or manage `~/.codex`.
- The wrapper does not force `CODEX_HOME` for `default`.
- Upstream Codex creates or uses its own default home as needed.

`codex profile default` explicitly selects the upstream default profile. Bare `codex` uses the most recently selected profile, which is `default` until another profile is selected.

### Custom profiles

Custom profiles live under:

```text
~/.codex-profiles/<name>
```

When a custom profile exists, the wrapper runs upstream Codex with:

```text
CODEX_HOME=~/.codex-profiles/<name>
```

When a custom profile does not exist, the wrapper prompts before creating it:

```text
profile 'work' does not exist. Create it? [y/N]
```

- `y` or `Y` creates the profile directory and launches the runtime.
- `n`, `N`, Enter, or Esc cancels.
- Non-interactive sessions do not create missing profiles.

Custom profile plugin directories are not shared. If upstream Codex creates plugins or other state, it does so inside that profile's `CODEX_HOME`.

## Managed paths

Runtime files live under:

```text
~/.local/lib/codex/termux
```

Wrapper state lives under:

```text
~/.local/share/codex/termux
```

Custom profiles live under:

```text
~/.codex-profiles
```

The upstream default Codex home remains:

```text
~/.codex
```

## Non-goals

This package intentionally does not provide:

- legacy wrapper compatibility;
- aliases for old internal names;
- migration from old development layouts;
- shared plugin wiring between profiles;
- a replacement command surface for upstream Codex;
- a sandbox implementation beyond Termux compatibility shims.
