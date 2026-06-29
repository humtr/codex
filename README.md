# Codex Termux Wrapper

Codex Termux Wrapper installs and runs the official `@openai/codex` linux-arm64 package inside Termux. It keeps upstream Codex behavior intact wherever possible and adds only the Termux compatibility layer needed to launch the upstream runtime reliably.

Release packages intentionally contain only runtime/install code. Source checkouts may include repository-only tests under `tests/` for development validation; release packages exclude them.

Build release packages with `bash tools/package-release.sh`. The release package is allowlist-based and excludes repository-only development material such as `tests/`, `.github/`, and `docs/`.

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

Public one-line install from GitHub:

```sh
curl -fsSL https://raw.githubusercontent.com/humtr/codex/main/install.sh | bash
```

This downloads the bootstrap installer, fetches the wrapper source, installs the
official upstream `@openai/codex` linux-arm64 package, patches it for Termux,
and installs the managed `codex` launcher.

For a private repository, the first `install.sh` download also needs GitHub
authentication because local saved config cannot be read until after the script
starts. If GitHub CLI is already authenticated, use:

```sh
GITHUB_TOKEN="$(gh auth token)" bash -c 'curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" https://raw.githubusercontent.com/humtr/codex/main/install.sh | bash'
```

If `~/.config/codex-termux/wrapper-source.env` already contains a saved
read-only PAT, use:

```sh
bash -lc '. ~/.config/codex-termux/wrapper-source.env; curl -fsSL -H "Authorization: Bearer $CODEX_TERMUX_WRAPPER_TOKEN" "https://raw.githubusercontent.com/$CODEX_TERMUX_WRAPPER_REPO/$CODEX_TERMUX_WRAPPER_REF/install.sh" | bash'
```

If you already have a source checkout, run the local installer:

```sh
bash bin/install-local.sh
```

`bash install.sh` still works from a checkout for convenience; it prints the
local checkout path and delegates to `bin/install-local.sh`.

After installation, use the managed launcher:

```sh
codex
```

## Release package

Create the distributable wrapper package from a source checkout with:

```sh
bash tools/package-release.sh
```

The package is built from an explicit allowlist. It contains only the runtime/install surface required by the wrapper: `README.md`, `install.sh`, `bin/`, `lib/`, selected runtime tools, `tools/codex_termux/`, and `config/`. Repository-only development files such as `tests/`, `.github/`, `docs/`, `.agents/`, git hooks, and version-update helpers are intentionally excluded.

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
codex <upstream args...>
```

Runs the managed upstream Codex runtime. The wrapper may repair, update, or roll
back the managed runtime before launch. Bare `codex` launches the most recently
selected profile, or `default` when no recent profile has been recorded.

Top-level Codex arguments are reserved for upstream Codex. Wrapper-specific
operations live under the `codex termux` namespace:

```sh
codex termux help
```

Prints the wrapper command surface.

```sh
codex termux install [VERSION]
```

Refreshes wrapper support from the configured wrapper source when available,
downloads a fresh upstream package, patches a runtime bundle, activates it, and
updates the verified rollback baseline.

```sh
CODEX_TERMUX_WRAPPER_REPO=OWNER/REPO \
CODEX_TERMUX_WRAPPER_REF=main \
CODEX_TERMUX_WRAPPER_TOKEN=github_pat_... \
codex termux update
```

The wrapper token lookup order is `CODEX_TERMUX_WRAPPER_TOKEN`, `GITHUB_TOKEN`,
then `gh auth token`. The token should be a fine-grained PAT limited to the
wrapper repository with `Contents: read-only`. Without a configured repository,
the current installed wrapper source is used.

Turn-completion notification behavior can be configured in:

```sh
~/.local/share/codex/termux/notify/config.env
```

Example:

```sh
CODEX_TERMUX_NOTIFY_CONTENT_CHARS=0
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES=1
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top
```

Use `CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top`, `middle`, or `bottom` to place the toast. The default is `top`.
Set `CODEX_TERMUX_NOTIFY_CONTENT_CHARS=0` to pass the full assistant message to the Android notification content.
Use `codex termux notify --hooks PreToolUse` if you also want a notification when a tool call starts, or `codex termux notify --hooks all` to enable every supported hook position. The default hook set is `Stop`.

```sh
codex termux notify --hooks all --toast-gravity top
```

Writes `~/.local/share/codex/termux/notify/config.env` and regenerates the hook configuration immediately.

```sh
codex termux update [VERSION]
```

Same as `codex termux install`: refreshes configured wrapper support, downloads
the selected or latest upstream package, patches a runtime bundle, activates it,
and updates the verified rollback baseline.

```sh
codex termux install support
```

Refreshes configured wrapper support files and the public launcher without
changing the active runtime.

```sh
codex termux install upstream
codex termux install upstream 0.142.3
```

Downloads the selected or latest upstream package and installs it as a patched runtime with the current installed support layer.

```sh
codex termux install rebuild
```

Refreshes configured wrapper support and rebuilds the patched runtime from the
cached raw package without fetching upstream Codex.

```sh
codex termux repair
```

Diagnoses the managed installation and applies the narrowest available repair.
It refreshes support when support files or the launcher are damaged, repairs
metadata when the runtime is healthy, and rebuilds from cached raw when the
active runtime is damaged. It does not update to a fresh wrapper/runtime by
default.

```sh
codex termux use
codex termux use --list
codex termux use <selection>
```

Lists cached and remote runtimes, then promotes the selected runtime. Selection accepts menu numbers and available runtime versions.

```sh
codex termux session
```

Interactive curses-based TUI picker to resume any discovered Codex session across any target profile.

```sh
codex termux doctor
codex termux doctor --json
```

Runs wrapper-only diagnostics for launcher, runtime resources, resolver, CA, DNS
patch, state, and registry metadata. The upstream `doctor` command remains an
upstream top-level command and is no longer combined with wrapper diagnostics.

```sh
codex termux profile
codex termux profile <name>
```

Lists profiles or launches the runtime with a selected profile.

```sh
codex termux version
```

Prints upstream Codex, active runtime, and wrapper version rows.

```sh
codex termux remove
```

Removes the managed launcher/runtime and restores launcher backups when present. State is kept for backups.

## Profiles

Profiles are `CODEX_HOME` switches.

### Default profile

The `default` profile is upstream Codex's normal default behavior.

- The wrapper does not create or manage `~/.codex`.
- The wrapper does not force `CODEX_HOME` for `default`.
- Upstream Codex creates or uses its own default home as needed.

`codex termux profile default` explicitly selects the upstream default profile. Bare `codex` uses the most recently selected profile, which is `default` until another profile is selected.

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
