# Codex Termux Wrapper

Codex Termux Wrapper installs and runs the official `@openai/codex` linux-arm64 package inside Termux. It preserves upstream Codex behavior wherever possible and adds only the compatibility layer required for reliable execution on Android/Termux.

## Source layout

The implementation is organized by runtime role rather than by product name or language:

```text
bin/          public install entrypoints
shell/        shell orchestration and process/FD glue
src/wrapper/  Python policy and domain implementation
libexec/      internal programs installed with the manager
native/       native launcher sources
lib/          public compatibility facade
tools/        development, release, and compatibility entrypoints
```

`codex-termux` remains the external product and compatibility name. Public commands, release names, `CODEX_TERMUX_*` variables, `~/.config/codex-termux`, and `lib/codex-termux.sh` remain stable. Historical paths under `lib/codex-termux/`, `tools/codex_termux/`, and selected `tools/` files are temporary one-release facades; new implementation belongs only in the role-oriented directories.

## Philosophy

The wrapper is not a fork of upstream Codex behavior. It is a thin Termux launcher and runtime manager.

- Preserve upstream Codex defaults wherever possible.
- Patch only the paths and process contracts that do not work in Termux.
- Keep wrapper state under explicit managed paths.
- Keep public compatibility while preventing duplicate internal policy ownership.
- Treat custom profiles as explicit `CODEX_HOME` switches.

## What the wrapper changes

The wrapper:

1. Installs the official upstream `@openai/codex` linux-arm64 package.
2. Patches the raw binary into a Termux-compatible runtime.
3. Installs Termux-compatible `bwrap` and `rg` shims beside the runtime.
4. Exposes a managed `codex` launcher.
5. Manages immutable support and source-snapshot artifacts with rollback pointers.

Unknown top-level commands and normal bare execution are passed through to the active upstream runtime after readiness checks.

## Install

Public one-line install from GitHub:

```sh
curl -fsSL https://raw.githubusercontent.com/humtr/codex/main/install.sh | bash
```

For a private repository with GitHub CLI authentication:

```sh
GITHUB_TOKEN="$(gh auth token)" bash -c 'curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" https://raw.githubusercontent.com/humtr/codex/main/install.sh | bash'
```

When `~/.config/codex-termux/wrapper-source.env` already contains a read-only token:

```sh
bash -lc '. ~/.config/codex-termux/wrapper-source.env; curl -fsSL -H "Authorization: Bearer $CODEX_TERMUX_WRAPPER_TOKEN" "https://raw.githubusercontent.com/$CODEX_TERMUX_WRAPPER_REPO/$CODEX_TERMUX_WRAPPER_REF/install.sh" | bash'
```

From a source checkout:

```sh
bash bin/install-local.sh
```

`bash install.sh` remains a convenience facade that delegates to `bin/install-local.sh`.

## Release package

Create the distributable archive with:

```sh
bash tools/package-release.sh
```

The allowlist-based package contains `README.md`, `install.sh`, `bin/`, `lib/`, `shell/`, `src/`, `libexec/`, `native/`, `config/`, the contract manifest, and the temporary compatibility entrypoints required for upgrades. It excludes tests, Git metadata, workflows, documentation-only directories, developer hooks, `.gitignore`, and Python bytecode.

## Runtime and support model

- **upstream package**: the official npm package.
- **raw binary**: upstream `codex` before Termux patching.
- **runtime**: the patched executable that runs in Termux.
- **active runtime**: the runtime selected by `current`.
- **verified runtime**: the runtime rollback baseline selected by `verified`.
- **manager**: the active wrapper support artifact.
- **verified manager**: the previous known-good support artifact.
- **source snapshot**: the immutable source artifact corresponding to the active manager.

Runtime patching remaps the resolver and Codex system configuration through inherited file descriptors. FD 33 is the resolver file and FD 34 is the managed system-config directory. These are fixed runtime contracts.

Support installation publishes immutable artifacts and switches pointers only after validation:

```text
~/.local/lib/codex/termux/
├── support-store/
│   └── support-<version>-<commit>-<nonce>/
├── source-store/
│   └── source-<version>-<commit>-<nonce>/
├── manager -> support-store/<active>
├── verified-manager -> support-store/<previous-good>
├── source-snapshot -> source-store/<active>
├── verified-source-snapshot -> source-store/<previous-good>
├── current
├── verified
├── raw
└── store/
```

Manager, source snapshot, generated hook configuration, and the public launcher are one support activation transaction. A failure restores all four from the recorded transaction data.

## Commands

```sh
codex
codex <upstream args...>
```

Runs the active upstream runtime. Bare `codex` uses the most recently selected profile, or `default` when none has been recorded.

Wrapper operations are under the `codex termux` namespace:

```sh
codex termux help
codex termux install [VERSION]
codex termux update [VERSION]
codex termux install support
codex termux install upstream [VERSION]
codex termux install rebuild
codex termux repair
codex termux use [--list|SELECTION]
codex termux session
codex termux doctor [--json]
codex termux profile [NAME|current|status]
codex termux version
codex termux remove
```

`install` and `update` refresh wrapper support, install the selected upstream package, patch a runtime, and activate it. `install support` changes only support and the launcher. `install upstream` installs a fresh upstream package with the current support layer. `install rebuild` rebuilds from the cached raw package without network access.

`repair` diagnoses the installation and applies the narrowest action: support refresh, metadata repair, cached rebuild, or verified rollback.

`doctor --json` reports runtime/state health and, for role-oriented managers, validates the support manifest, immutable manager/source stores, active and verified support pointers, and required `shell`, `src/wrapper`, and `libexec` files.

## Wrapper source configuration

Example update from a configured repository:

```sh
CODEX_TERMUX_WRAPPER_REPO=OWNER/REPO \
CODEX_TERMUX_WRAPPER_REF=main \
CODEX_TERMUX_WRAPPER_TOKEN=github_pat_... \
codex termux update
```

Token lookup order is `CODEX_TERMUX_WRAPPER_TOKEN`, `GITHUB_TOKEN`, then `gh auth token`. Use a fine-grained token limited to repository contents read access.

## Notifications

Notification settings live at:

```text
~/.local/share/codex/termux/notify/config.env
```

Example:

```sh
CODEX_TERMUX_NOTIFY_CONTENT_CHARS=0
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES=1
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top
CODEX_TERMUX_NOTIFY_TOAST_DURATION=long
```

`CODEX_TERMUX_NOTIFY_TOAST_DURATION` accepts `short` or `long`. During the compatibility release, `CODEX_TERMUX_NOTIFY_TOAST_SHORT` is also read when the new key is absent.

Configure hooks with:

```sh
codex termux notify --hooks all --toast-gravity top
```

The notification implementation is owned by `src/wrapper/notification/` and executed through `libexec/notify`. Click actions are allowlisted to no action, opening Termux, or opening a validated tmux target. External commands use argument arrays rather than caller-supplied shell strings.

## Profiles

Profiles are `CODEX_HOME` switches. The `default` profile preserves upstream behavior and does not force or create `~/.codex`.

Custom profiles live under:

```text
~/.codex-profiles/<name>
```

When a custom profile is selected, the runtime receives:

```text
CODEX_HOME=~/.codex-profiles/<name>
```

Missing profiles are created only after interactive confirmation. Auth, plugin data, logs, and caches remain isolated by profile. The session picker can discover sessions across profiles and link the selected session into the target profile before invoking upstream `resume`.

## Managed paths

```text
Runtime and support: ~/.local/lib/codex/termux
Wrapper state:      ~/.local/share/codex/termux
Source config:      ~/.config/codex-termux
Custom profiles:    ~/.codex-profiles
Upstream default:   ~/.codex
```

## Validation

Portable validation:

```sh
bash tests/run-portable.sh
```

Termux/device validation:

```sh
CODEX_TERMUX_AUTO_UPDATE=0 bash tests/run-termux.sh
codex termux doctor --json
```

## Non-goals

This package does not provide:

- a replacement command surface for upstream Codex;
- a fork of the upstream runtime;
- configurable FD 33/34 contracts;
- migration to a different implementation language as part of this layout change;
- a replacement Termux:API APK;
- shared plugin wiring between profiles;
- a sandbox implementation beyond the required Termux compatibility shims.
