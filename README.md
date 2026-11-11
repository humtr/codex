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

## Actual status — source is not deployment

The development source currently selects official upstream **0.167.1**
([recipe](release/upstream.json)). The most recently verified public signed
Full publication is Codex **0.167.0 Full163**
([release](https://github.com/humtr/codex/releases/tag/full-v0.167.0-c19b61b-s163)).
[PR #29](https://github.com/humtr/codex/pull/29) reports Full163 installed
and Full159/Full153 retained; no new device readback was performed by this
documentation update. The PR's Core compatibility-link repair has now been
merged into the development source, but **was not included in signed Full163**.

At `f9dfc3f`, [Source Acceptance](https://github.com/humtr/codex/actions/runs/42942784769)
and [Core qualification](https://github.com/humtr/codex/actions/runs/42942787296)
passed; [TUI Native qualification](https://github.com/humtr/codex/actions/runs/42945712023)
**failed**. The PR #29 head separately passed source and Core qualification,
not exact post-merge acceptance. No CI receipt here proves installed GPT-6
same-thread TUI continuity.

After a user-performed complete Termux restart, TermuxBridge remained
reachable but a newly admitted Native `functions.exec_command` still returned
`native_resource_busy` with `executed=false` despite an empty per-work
active-effect list. This **P0 wrapper execution availability** issue is
unresolved; neither a successful Bridge handshake nor a Termux reboot proves
execution recovery. See [CURRENT](CURRENT.md) for dated evidence and
[HANDOFF](HANDOFF.md) for the next authorized acceptance work.

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
signature/manifest before use. The Full variant includes Core, Manager and TUI;
Base includes Core. Provider and Linux ChatGPT app/login are **separate**
setup. Existing installations use Core-owned `codex update`, not a fresh
bootstrap overwrite. `codex update --rollback` is an explicit rollback
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
`integration/codex-native-20261110`; check its actual HEAD before acting.

New Provider producer packages use `provider-chatgpt-v...`; published
`provider-gpt-v...` remains readable for historical rollback. Native TUI
source is `frontend/tui`; public compatibility identifiers
`termux-manager-tui-v1`, `manager-tui-native` and accepted builder aliases
remain unchanged to protect signed generations.
