# Codex for Termux

Termux compatibility layer for the upstream Codex CLI. The public command is
`codex`. The product continuously adopts qualified stable upstream releases
while keeping native execution, approvals, history and rollback under Core's
actual authority. Optional components do not become prerequisites for Core.

| Component | Purpose | Independence |
| --- | --- | --- |
| **Core** | Signed runtime entrypoint, installation, update, rollback, diagnosis | Required; upstream Codex remains the executor |
| **Manager** | Optional account, terminal/window, quota and notification conveniences | `codex termux` |
| **TUI** | Terminal UI with optional Manager and provider integration | Source: `frontend/tui`; not a Manager subprocess |
| **ChatGPT Provider** | Optional model transport, account/app and ChatGPT Bridge | Separate package, app and MCP bindings |

See the [product requirements](SPEC.md), [current verified state](CURRENT.md),
[active acceptance work](HANDOFF.md) and
[documentation reading policy](docs/DOCUMENTATION.md). Superseded release,
audit and handoff records remain accessible in Git history, not in current
user instructions.

## Source and installed releases

The development recipe selects upstream **0.167.1**. The independently checked
installation is **0.167.1 Full S165**, with 0.167.0 Full S163 retained as previous
generation. Core S164 is a separate signed release, not evidence of installation.
[CURRENT](CURRENT.md) owns exact source, signed inventory and CI evidence;
[HANDOFF](HANDOFF.md) lists only unfinished acceptance. Source changes and
commands described here become installed behavior only through separately
qualified, signed and activated generations. Existing signed releases stay
immutable.

## Everyday usage

From an existing project directory and an installed authorized generation:

```sh
codex
```

- `codex login` and `codex login status` manage upstream Native Codex login,
  **not** a Linux ChatGPT app session.
- `codex resume` continues an upstream Native conversation and
  `codex exec "TASK"` uses its noninteractive entrypoint.
- `codex termux` opens Manager features when installed.
- This source adds Core-owned `codex termux version`, `--details` and `--json`
  for offline verified inventory, even without Manager. Older installed
  generations may not provide this command.
- `CODEX_TERMUX_CORE_ONLY=1 codex` invokes the standalone Core route for
  that run, skipping optional Manager/TUI hooks. It does not bypass Core.
- `codex doctor` is the diagnostic entrypoint. Existing Native sessions and
  installed rollback generations should be preserved during troubleshooting.

A current-Chat ChatGPT response can use **Termux Bridge** attached Native
reasoning; separately requested managed Chat work uses Provider/AppHost and
its own login/project configuration. The model source is not the Native work
identity. A configured Provider is optional: no existing Native subscription
login, app account or approval policy should be silently changed.

## Installation, Provider, and rollback

For a **fresh** installation, use the repository's reviewed, signed
[online installer](install-online.sh) and the current
[installation contract](docs/spec/delivery/INSTALL.md). Do not reuse an old pinned installer
URL from a historical snapshot; check the exact accepted release and its
signature/manifest before use. The Full variant combines qualified Core/Runtime, Manager and TUI.
Core uses a qualified Runtime Bundle without requiring Manager or Enhanced TUI.
Historical Base names and index fields remain compatible. Provider and Linux
ChatGPT app/login are **separate**
setup. Existing installations use Core-owned `codex update`, not a fresh
bootstrap overwrite. This source supports `codex update --core` and
`codex update --full`; plain update preserves the authenticated installed edition.
A Core selection preserves the previous Full generation and still enforces
anti-rollback. `codex update --rollback` is an explicit rollback
operation; verify the current and retained generation and settle active work
before performing an update or rollback. An app/service restart is not implied
by source compilation or package selection.

[Provider installation and account-bound configuration](providers/chatgpt/README.md)
and [existing Linux ChatGPT app setup](providers/chatgpt/LINUX_APP.md)
are described separately. The enhanced TUI's source assembly and
qualification are in [frontend/tui/README.md](frontend/tui/README.md).

## Local development and constraints

Begin with [AGENTS.md](AGENTS.md), the
[documentation map](docs/DOCUMENTATION.md), and the
[Termux path/ownership preflight](docs/TERMUX_CONSTRAINTS.md).
Every live TUI test must start at the real Core dispatcher and use the same
Native writer that the selected Provider can actually observe. Do not set a
long fixture's directory as the runtime Unix socket `TMPDIR`.
The maximum filesystem socket path is **107 encoded bytes**; see the
canonical [socket contract](docs/spec/delivery/TERMUX_COMPAT.md).
Keep per-work scratch/evidence outputs independent and never touch Android
sdcard. This repository's current development branch is
`rewrite/rust-core`; check its actual HEAD before acting. `main` retains the
separate publication/approval role. Temporary integration branches do not
become parallel long-lived development sources.

New Provider producer packages use `provider-chatgpt-v...`; published
`provider-gpt-v...` remains readable for historical rollback. Native TUI
source is `frontend/tui`; public compatibility identifiers
`termux-manager-tui-v1`, `manager-tui-native` and accepted builder aliases
remain unchanged to protect signed generations.
