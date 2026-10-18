# ChatGPT Provider experiment

Reviewed2026-10-18. This branch contains a selectable ChatGPT adapter for local
Termux Codex. It is not a stable bundled provider. Production [SPEC](https://github.com/humtr/codex/blob/rewrite/rust-core/SPEC.md)
owns current contracts; production [CURRENT](https://github.com/humtr/codex/blob/rewrite/rust-core/CURRENT.md#product-completion-scope-and-work-order)
owns the single ordered completion plan. This branch [CURRENT](../../CURRENT.md)
owns exact provider state, proof and limitations. Historical investigation and
superseded setup instructions remain in Git rather than this user entrypoint.

## Actual ChatGPT in native Termux Codex

The installed optional adapter uses the signed Linux app's Chat mode and login.
It exposes local /v1/responses wire format; it is neither the official Responses
service nor evidence of API entitlement. Core/upstream remain the sole native
executor, reviewer and canonical history owner. Manager account login and signed
Chat app login are distinct. The app/display/Scope/CDP dependencies are real even
when no user-visible browser Send action is used.

/model is the primary optional selection and upstream --profile chatgpt is its
configuration fallback. Do not resume a long native thread expecting lossless
Chat transfer yet: previous-model compaction currently fails unsupported_desktop_route,
and opaque checkpoints require readable handoff. Fresh Chat avoids that specific
transition but current Full60 real inference was not requalified by diagnosis.
The user's1,000,000-token catalog override is retained, not verified host capacity.

Installed code45072df8448589d3523e/source7bb202e uses local-function results:
Core approves/runs each advertised tool, and exact output returns on its original
host call without a new Chat user message per result. Independent MCP return is
an additional source/offline-qualified transport, not an installed live capability.
The actual host currently reaches confirm_action before native tool execution.
Count user objects, tool messages, starts, HTTP and allowance deductions separately.
No Codex-quota experiment establishes Chat message-limit savings.

The current source also records bounded, content-free warm-context/host-creation
observations. A successful summary instruction is transient until native history
accepts its checkpoint; an uncommitted summary must not poison the next prefix.
These observations do not establish allowance deductions. Active runtime sources
are being integrated as the optional providers/chatgpt component on rewrite/rust-core;
this experimental checkout is not the production Core or release authority.

## Ownership and dependencies

| Component | Owner |
| --- | --- |
| Runtime execution, tools, reviewer, history and locks | Qualified upstream + Core |
| Accounts/quota/windows/task policy | Optional Manager |
| Upstream terminal UI and optional selections | Independent TUI; existing frontend/manager-tui is a compatibility path |
| App discovery, auth context, lanes, submissions/returns, cancellation and host lifetime | Repository-owned desktop_* modules and lifecycle code |
| Parser/SSE/queue/tool mapping/turn identity/summary helpers | Pinned [miuuyy6.1.5/source92a356f](https://github.com/miuuyy/codex-chatgpt-web/tree/92a356fac2292e3af5a97ab7ba634edd8d38621e), desktop-bridge-615.patch, dependency source/licenses |
| Independent retained MCP | Repository-owned desktop_mcp_* with shared DesktopToolSession and Core execution |

Do not claim the Responses implementation is wholly original or rename away
attribution. Older browser/full-mode code retained in dependency source is not
wired into the installed adapter merely because it is present. Browser6.1.8/6.1.9
and SIWC/public Responses are separate historical research, not active fallbacks.
Connector display names, including the old Codex Native2 probe name, are not
protocol requirements or hardcoded current branding.

## Installed authority

~/.local/share/codex-web-experiment/native-provider/runtime.json selects the
immutable payload under ~/.local/lib/codex-web-experiment/. Its .installation.json
binds source/dependencies; generated models.json and registered upstream profiles
are settings outputs. Keep previous payloads for rollback. Signed desktop app login
and profile data remain outside immutable code. Never print runtime tokens/auth.
Full60 updates do not replace or qualify this experimental installation.

install_desktop_provider.py packages26 reachable runtime source/assets plus pinned
dependencies/licenses. It can reuse the verified installed dependencies. This is
an explicit development installer, not the stable production update mechanism;
activation refuses a live adapter and must not stop user work to make installation
pass. Runtime install/reinstall/rollback trials require an explained bounded scope.
Do not run installation or inference merely to read settings or inspect a project.

## Approved configuration and project target

One user-owned file, $XDG_CONFIG_HOME/codex/providers/chatgpt.toml (default
~/.config/codex/providers/chatgpt.toml), should drive context/compaction/model/effort
and remote project placement. Effective/show/validate/edit/set plus optional
TUI/Manager views are planned, not implemented commands. All use one provider
validator; Manager/TUI are not prerequisites for file-based use. Generated catalogs,
per-home aliases and immutable Python/JS are not additional editing authorities.
Preserve1m explicit override and keep native token triggers, encoded/decoded byte
limits and actual host capacity distinct. Do not silently truncate or disable
compaction needed for readable state transfer.

New owned chats should use the configured ChatGPT project and create it when
absent; reuse a confirmed account/workspace-scoped ID. Current local transport
retains normal chats with no project binding. Read-only app-operation discovery
must precede no-inference create/reuse/place/readback, then a budgeted model/tool
trial. Project grouping affects instructions/files/memory and sidebar organization;
it does not itself reduce message count or total history. Preserve existing chats,
explicit retention settings and native reviewer boundaries. Do not silently change
sharing/personalization or claim temporary+project is supported.
See [official projects](https://learn.chatgpt.com/docs/projects?surface=app);
this documents user-facing behavior, not an adapter API.

## Checks without real Chat

From the repository root, supply the qualified installed Bun path as BUN:

```sh
"$BUN" test ./experiments/web-provider/*.test.mjs
PYTHONDONTWRITEBYTECODE=1 python -B -m unittest discover \
  -s experiments/web-provider -p 'test_*.py'
```

Do not use unscoped bun test: chatgpt_contract.test.ts is a dependency-checkout
fixture whose imports target that checkout's src/, not this standalone suite.

A deterministic real-Core surface trial uses current production validators and
an unused private output directory, with0 real Chat submissions:

```sh
CODEX_CHAT_CORE_PROOF_SOURCE="$HOME/prj/codex" python -B \
  experiments/web-provider/desktop_surface_probe.py \
  --output "$HOME/.cache/codex/acceptance/NEW-UNUSED-PROOF-DIRECTORY"
```

The development variants exercise actual repair/tests/manual or automatic/fault
compaction and cold continuation in disposable roots. Inspect --help and CURRENT
for exact gate coverage. Driver-answered questions are not physical user proof.
Tests must retain actual tool effects, native report persistence/rendering and
latest-state resume; a successful server reply/file/counter is insufficient.

Live flags submit actual Chat. Explain necessity, exact input identity, model,
expected user/setup/summary/retry count and bound before use. No automatic Pro,
account-drain or speculative repeat after uncertain delivery. Native thread
cancellation is distinct from terminating an already-started shell process.

## Integration and release

Port active provider implementation, dependencies/licenses and required tests onto
current rewrite/rust-core after the approved responsibility work. Do not merge old
Core/Manager/quota/TUI/release snapshots over current code. Separate runtime,
qualification and historical research; preserve existing source edits and Git
history. TUI is independent from Manager, while every Termux execution uses Core.
Provider native --profile operation must remain usable without either optional UI.

Stable admission needs generic configuration, readable native↔Chat handoff, project
placement, long-work/lifetime and visible transcript proof, independent MCP if
shipped/enabled, authenticated payload/update/rollback and public/device acceptance.
Keep Base independent, Full's qualified features and provider opt-in; no mandatory
rootfs/app installation or login during ordinary launch/update. Source integration,
public publication and user-device activation are distinct gates.
