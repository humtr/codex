# ChatGPT Provider — Codex bridge

Optional Provider component for ChatGPT model/source transport, account and
app lifetime, and result observation. **Native Core and upstream Codex own
work/turn identity, tools, approvals, original committed history and effective
thread route**; Provider never becomes a second Native executor.

[Current evidence](../../CURRENT.md) ·
[Bridge product contract](../../docs/spec/INDEX.md) ·
[Component documentation policy](../../docs/DOCUMENTATION.md) ·
[Linux app operations](LINUX_APP.md) ·
[Functional contract index](../../docs/spec/INDEX.md)

## Qualification boundaries

[CURRENT](../../CURRENT.md) owns the exact verified source, installed generation
and CI results. Unit and synthetic transport tests do not establish live ChatGPT
inference, a same-thread TUI round trip or Android display acceptance. Source
updates also do not replace an operating Provider binary, account binding or
signed generation; those remain separate activation boundaries.

## Independent Termux MCP worker

The optional Android qualification retains two separately verified unsigned
candidates: `provider-android-candidate` and
`termux-mcp-worker-android-candidate`. The worker runs
`codex-termux-mcp-worker --state ABSOLUTE_PROVIDER_STATE`; its shell path needs
neither a running Provider nor Core. Existing Native tools still use their
original Provider service. These candidates do not authorize installation and
do not alter existing signed Provider inventories or the operating MCP wrapper.
For an independently installed worker, connector refresh reads its exact existing
wrapper through the separate `mcp_command` input described in
[Linux app operations](LINUX_APP.md). The Provider/Desktop binary remains a
different input. The [MCP shell contract](../../docs/spec/interfaces/MCP_SHELL.md)
owns receipt, lifecycle and publication boundaries.

## Choose the requested entrance

| Entrance | Source of primary reasoning | Prerequisite |
| --- | --- | --- |
| Native terminal using ChatGPT | Selected ChatGPT Provider through configured Core/TUI route | Provider package, authenticated Linux app and compatible host |
| Current ChatGPT reply continuing Native work | `inference=attached`; no second primary model request | Reachable Termux Bridge and selected original Native work |
| Codex requesting a new ChatGPT app turn | `inference=submitted` | Configured app/account/project, valid route and explicit Native admission |

Native Codex login, Linux app login, MCP connection and Provider account
identities are **different**. Do not copy credential tokens between them or
treat a connected app as permission to execute on any Native writer.

## First-time configuration (do not apply to an existing device blindly)

No general `codex provider setup` command is shipped. From a qualified
Core/Full install, **observe** current selection before changing it:

```sh
codex provider status chatgpt
codex provider chatgpt models
codex provider chatgpt config validate
```

For a new, separately authorized Provider package, selection is:

```sh
codex provider install chatgpt
```

The configured default Native account/workspace is explicit. Only when
creating a fresh setup, use the **actual** account's `CODEX_HOME` and an
existing project, and check that Core's signed launcher is installed:

```sh
codex provider chatgpt config set   execution.core "$PREFIX/bin/codex"   execution.home "/absolute/path/to/the/selected/CODEX_HOME"   execution.workspace "/absolute/path/to/the/existing/project"
codex provider chatgpt config validate
```

These are *examples*, not authorization to replace the operating wrlab
profile, signed Provider, app host or service. A Manager named account and
Native `--profile chatgpt` have distinct scope. Source selection, package
installation, authenticated app, service activation and actually completed
inference are separately verified boundaries.

## Identity, connection and recovery rules

- Start a real product TUI test through `$PREFIX/bin/codex`, **not** a raw
  frontend helper, and inspect the selected Core generation and actual
  Native writer. Keep configured Provider Core/HOME/execution-home/workspace
  consistent with that writer. An isolated Core requires an isolated,
  appropriately bound Provider or a zero-model fixture.
- Preflight private **short socket TMPDIR** independently of long scratch
  roots, as required by [Termux constraints](../../docs/TERMUX_CONSTRAINTS.md).
  Core owns the native executable and socket permissions.
- For connection loss, timeout, HTTP 413 and uncertain acknowledgements,
  preserve request identity and original Native receipts; never auto-resend an
  accepted user input or upgrade a failed turn to success.
- Source and exact retry use the existing configured operation/work scope.
  Native policy and auto-reviewer control must not be widened through
  Provider, browser automation or a second shell executor.
- An account change must respect configured policy and already accepted
  work. Existing credentials/sessions/journals are not disposable test data.

## Support references

[Linux app launch, login and plugin replacement](LINUX_APP.md) contains
screen and session setup. [Bridge protocol details](../../docs/spec/INDEX.md)
cover Native inputs, transport and approvals; [current acceptance plan](../../HANDOFF.md)
defines remaining work. Historical commands and long, dated incident
completed reports are recoverable through Git history, not
current service instructions.

## Provider-specific model selection

The TUI displays this transport as **ChatGPT Linux app** while preserving the
existing `chatgpt_desktop` Provider ID and saved `chatgpt-desktop/` model IDs.
Select it with `/provider`; `/model` then reads the current authenticated app
catalog, including newly offered server models and their available reasoning
levels. OpenAI's `/model` keeps the upstream catalog and does not contact the app.
A model belonging to another Provider is rejected by `/model`; use `/provider`
for an intentional route change.

The read-only `host models` response includes account-verified native metadata
and its immutable private snapshot. A new CLI can consume an older running
service's fresh response by checking its exact account-bound observation and
using the same catalog projector. This compatibility does not require a service
restart or a shared version number. An old CLI without the response capability
reports live discovery unavailable; configured defaults and other accounts are
never fallbacks. Updating source alone does not replace an operating Provider
binary or signed TUI installation.
