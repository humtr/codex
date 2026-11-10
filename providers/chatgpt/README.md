# ChatGPT Provider — Codex bridge

Optional Provider component for ChatGPT model/source transport, account and
app lifetime, and result observation. **Native Core and upstream Codex own
work/turn identity, tools, approvals, original committed history and effective
thread route**; Provider never becomes a second Native executor.

[Current evidence](../../CURRENT.md) ·
[Bridge product contract](../../docs/spec/BRIDGE.md) ·
[Component documentation policy](../../docs/DOCUMENTATION.md) ·
[Linux app operations](LINUX_APP.md) ·
[Historical full guide](../../docs/legacy/2026-11-10/providers/chatgpt/README.md)

## Verified scope and remaining acceptance

The source revision `5384d938` passed
[Provider qualification #42758329834](https://github.com/humtr/codex/actions/runs/42758329834),
but this does not qualify the installed `/provider` → GPT-6 two-turn flow.
The isolated candidate TUI selected GPT-6 and then failed on the first
real input with `native_thread_owner_unavailable` because that Core and
the installed Provider used **different Native owner bindings**.
SSE cut, HTTP 413, original-result recovery and independent concurrent
work remain unfinished. Operating Full159, rollback Full153 and Provider
account/service bindings were not modified.

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
screen and session setup. [Bridge protocol details](../../docs/spec/BRIDGE.md)
cover Native inputs, transport and approvals; [current acceptance plan](../../HANDOFF.md)
defines remaining work. Historical commands and long, dated incident
reports are isolated under [legacy](../../docs/legacy/README.md), not
current service instructions.
