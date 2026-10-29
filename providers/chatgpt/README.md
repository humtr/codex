# ChatGPT Provider / Codex bridge

This optional Rust component lives on `rewrite/rust-core`. Core launches the
verified upstream runtime; Provider owns Chat connection, model configuration,
Responses/tool-return mapping and host lifetime. Upstream Codex executes tools,
applies its approval/reviewer policy and owns canonical history. Neither Core nor
Manager links Provider implementation. Manager and enhanced TUI are not required
for the upstream configuration route.

[CURRENT](../../CURRENT.md#operating-baseline) owns the selected generations,
signed delivery evidence and actual running owners. Provider is the component
name; `chatgpt` is its CLI identifier. A release sequence is package metadata,
not part of that component name. Source qualification, package selection,
service activation, actual inference and physical UI observation are distinct.

## Bridge design and implementation status

Provider contains the ChatGPT–Codex bridge: model/native-tool adapters and shared
session, connection and host management. MCP is a tool/result protocol used by
that bridge; the app displays its MCP connection as **Termux Bridge**. ChatGPT
supplies judgment; native Codex executes, approves and records. `operator` remains
a code/config/API label for native-work controls, not another reasoning agent.

The [retained control contracts](../../SPEC.md#e-selected-common-interfaces-and-lifecycle)
remain in force. Earlier F/G/H qualification and delivery apply at their recorded
scope; [CURRENT](../../CURRENT.md#evidence-map-and-limits) separates those results
from remaining entrance-continuity boundaries. [AUDIT](../../AUDIT.md) preserves
the original independent Chat/native-work audit; superseded proposals are
distinguished from the current SPEC and CURRENT plan.
Provider15 delivers persistent binding and Bridge-native join for an active Chat
response at the qualification scope in CURRENT. Saved-source recovery is explicit;
ordinary Bridge work does not crawl Chat. Source recovery does not make all host
tools Native execution or establish unobserved original host message identities.

Provider18 retains inherited handles, including Core's payload selection lease,
in its own process and marks non-stdio handles close-on-exec before starting
children. Native/app children receive only explicitly exported descriptors; they
must not keep an incidental Provider selection lock alive. Running Provider
processes retain their pinned immutable bytes. The development Core now allows
selection changes during execution under its existing activation lock; older
installed Cores retain their prior selection-blocking behavior until upgraded.
CURRENT distinguishes source qualification from operating activation.

All tool declarations share one Rust annotation policy: readOnlyHint=true,
destructiveHint=false and openWorldHint=false, including codex_projects. These
hints do not replace native permissions or change actual read/write capability.
Only idempotentHint varies with the qualified operation contract. The
[idempotency study](../../CURRENT.md#implemented-and-qualified-at-recorded-scope)
remains historical research; E settled and implemented the common operation/
duplicate/result-recovery contract and per-tool targets. F qualified
and applied them at the recorded scope. The Rust policy selects true for fresh source/native reads and
keyed execution/report/finish/handoff/answer/attach operations; mixed host/session/
summary/Stop/project tools retain false. Historical JavaScript declarations stay in the
preserved experiment/installation. Hint changes alone have no demonstrated
performance benefit. In the observed ChatGPT backend tools/list response,
idempotentHint is omitted even though the Rust service emits it; internal
consumption is unknown. Other hints must remain unchanged.

The [idempotency implementation](../../CURRENT.md#implemented-and-qualified-at-recorded-scope)
shares in-flight duplicate results and durable mutation/finish/registration/round
receipts, with per-work admission and Stop fencing. Other hints remain unchanged.
Older signed generations retain their own evidence and rollback inventory; see
the [operating baseline](../../CURRENT.md#operating-baseline).

## First-time setup

This is the current manual procedure. There is no installed Provider `setup`
command yet. Core/Full installation is documented in the
[main guide](../../README.md#fresh-install). Existing configured devices should
use their current configuration; these examples are for first-time setup.

### Choose the entrypoint and execution account

| Entry | Primary reasoning | Linux app needed? | Remote MCP needed? |
| --- | --- | --- | --- |
| Terminal with ChatGPT | Provider response | Yes, with app login | Only for MCP inner returns; local_function does not need it |
| ChatGPT direct | Active response develops directly (`reasoning=direct`) | No for primary direct work | Yes |
| ChatGPT general delegation | Provider-started response (`reasoning=provider`) | Yes, with app login and project configuration | Yes for the requesting Chat |

Native Codex login, Linux app login and the ChatGPT MCP connection are separate
boundaries. Direct current-Chat work does not require a native subscription login
or a Linux app for primary reasoning; separately configured app reviewers do.
Never copy tokens between them or into chat prompts.

Install/select the signed Provider, then inspect its status/catalog:

```sh
codex provider install chatgpt
codex provider status chatgpt
codex provider chatgpt models
```

Package selection is not proof of app login, service readiness or a working tunnel.
For a fresh default execution account, set these example variables. Replace the
project path with your existing project. For a named Manager account, replace
`codex_provider_home` with that account's existing registered home instead;
`codex termux profile current` shows its account ID, not its filesystem path.
The registered `work` account home is
`~/.local/share/codex/manager/profiles/work/home`. An explicit `CODEX_HOME` wins
over a saved Manager default; do not silently substitute another account.

```sh
codex_provider_home="$HOME/.codex"
codex_project=/absolute/path/to/your-project
(umask 077; mkdir -p "$codex_provider_home")
codex provider chatgpt config set \
  execution.core "$PREFIX/bin/codex" \
  execution.home "$codex_provider_home" \
  execution.workspace "$codex_project"
codex provider chatgpt config validate
```

These fields select defaults for explicitly created native work. Terminal Chat
requests follow the actual native session's account and working directory,
including after account switches; no Provider configuration change is needed.

The Linux app account is read from its current authenticated state. Account IDs,
workspace IDs and connector IDs are not compiled into Provider. Configure the
common account-switch policy through `history.account_change`: `new_chat` (default)
uses a fresh remote chat for a new native user input after the previous round is
settled; `refuse` requires the original app account. Pending/unknown requests and
native tool results keep their original identity and are never replayed into
another account. Connector branding remains `transport.connector_name`; tunnel
IDs, credential references and remote organization/workspace associations belong
to the tunnel configuration. Adding another ChatGPT account does not require a
code change. Each account still needs its own plugin registration and valid
Secure MCP Tunnel association/permissions.

```sh
codex provider chatgpt config set history.account_change new_chat
```

The config file remains the source of policy; select `refuse` there if switching
accounts should require a separate native conversation. Account values are read
from the logged-in app. A tunnel's organization/workspace associations are remote
configuration, and its ID/credential references stay in the `tunnel-client`
profile. Keep existing associations when adding another configured account.
Do not edit an existing work binding underneath running work; reconcile and
settle it before a deliberate configuration change/restart.

### Direct ChatGPT connection without a Linux app

With the native binding above, run the service in one Termux terminal:

```sh
codex provider chatgpt serve
```

Keep it running (or use your existing owned service supervisor). In another
terminal, use `status` and `work list` to observe it. `mcp-stdio` is the local
MCP command for a tunnel/transport; it requires an existing authenticated service:

```sh
codex provider chatgpt status
codex provider chatgpt work list
codex provider chatgpt mcp-stdio
```

The last command speaks MCP JSON-RPC on stdin/stdout; it is not an interactive
chat command. Configure your actual transport to invoke it, with the same HOME,
PREFIX and Provider state. In ChatGPT web, add a custom MCP server named
**Termux Bridge** through Plugins. Choose your transport's reachable Server URL
or configured Secure MCP Tunnel, complete authentication/connection consent,
install the resulting plugin and verify tool discovery. Select it with the `@`
picker in a conversation. The local loopback URL is not a public connector URL.
Account/workspace availability still applies; see the
[official OpenAI connection instructions](https://developers.openai.com/api/docs/guides/custom-mcp-server).

To enqueue a message for an existing Codex thread, use
`codex provider chatgpt work queue --thread-id UUID --text 'message' --call-id UNIQUE_ID`
or `codex_sessions` with `action: "queue"` and those fields in snake_case.
The receiving thread keeps its selected model and effort, so this also works
when it uses a Codex model. The queue acknowledgment confirms enqueueing;
inspect native history for consumption and the resulting turn. Reuse the same
call ID and arguments after a lost response. An uncertain enqueue is reconciled
against upstream queue/history and is never blindly sent again. Use
`--execution-home PATH` when selecting an explicit account or an unloaded thread.

Remote tunnel provisioning/credentials and connection registration currently
remain manual. Existing `tunnel-client runtimes connect/status` can supervise a
configured local MCP command, but Provider does not supply a universal remote
endpoint or create a remote account. The installed device's permanent wrapper
and remote alias in CURRENT are local operating state, not fresh-install defaults.
This device's local runtime now uses [runit services](#device-runit-services);
do not start a second tmux runtime against the same remote tunnel.

Once connected, use [direct development](../../README.md#chatgpt-with-direct-local-development).
The assistant selects `reasoning=direct` and completes the native attach/receipt/
execution/final protocol through advertised tools. No Linux rootfs is required
for this primary path.

### Linux app and login for app-backed reasoning

This branch additionally needs the official app in an already provisioned Linux
rootfs and the Termux `proot`/`Xvnc` executables. Their packages are `proot` and
`tigervnc`. An optional viewer needs compatible noVNC assets. **The signed Provider
package does not provision the app/rootfs or noVNC assets.** The historical
experiment installer is retired; it is not a supported fresh-install shortcut.

After you have provisioned those assets, set their actual absolute paths:

```sh
codex provider chatgpt config set \
  host.rootfs /absolute/path/to/provisioned-rootfs \
  host.viewer_assets /absolute/path/to/compatible-novnc-assets
codex provider chatgpt prepare --home "$codex_provider_home"
codex provider chatgpt host ensure
codex provider chatgpt view
```

`prepare` publishes generated routing and can start the service, without model
work. `host ensure` acquires the app. `view` opens its optional viewer. Complete
the official app login in that UI, then inspect:

```sh
codex provider chatgpt host status
```

The app must report ready in the intended account. Opening a window or receiving
a successful Android intent alone does not establish that. A cold app can be
unready while initializing; inspect its current state rather than repeatedly
restarting it. Do not copy the existing device's login files into another user's
setup. Native `codex login` is not a substitute for this app login.

Terminal use can now select `/model` in Full or run `codex --profile chatgpt`.
The built-in local_function return avoids a separate MCP connection for this
terminal Provider path. If you explicitly choose MCP returns, configure and verify
that connection before sending work. General ChatGPT delegation additionally
needs the remote connection described above and an explicit project below.

### Provider Chat project

Create or reuse the requested owned private project and select it for new work:

```sh
codex provider chatgpt projects list
codex provider chatgpt projects use --name "Codex"
```

`use` verifies the project through the authenticated app, creates it when absent,
then saves `history.mode=project`, its name and exact ID in the existing Provider
configuration. It submits no model request. Supply `--project-id ID` to choose
among duplicate names. The `codex_projects` MCP tool exposes the same operation
when an assistant receives an explicit project creation/use request.
An uncertain creation or conflicting local configuration edit preserves the
remote binding for reconciliation; retry the same selection without creating a
duplicate. Direct `config set` remains available for policy-only changes and
does not itself create a remote project.
Do not change an active work's destination underneath it. General delegation
selects `reasoning=provider`; the Provider task requires app readiness and
its own work/receipt/budget controls. See the
[entrypoint example](../../README.md#chatgpt-delegating-provider-development).

### Small shared setup flow to implement later

The requested follow-up is one shared minimum-setup implementation, callable
explicitly and after a fresh interactive install. It should fill only missing
inputs, preserve existing settings/login, install/select components through their
existing authorities, open the necessary official login UI and verify readiness
without sending a model task. Native, app and connection authentication must
remain distinct. Asset/remote-service inputs still need explicit sources; an
already working manually migrated device is not a fresh provisioning recipe.
Offer standard termux-services/runit registration for the selected entrance:
Provider and the configured OpenAI Secure Tunnel runtime for remote entry,
and the Provider-owned host keeper when eager app readiness is wanted. Reuse
existing credentials/remote registration; include private bounded logs,
dependency/readiness checks and persistent disable/restore operations. The
runtime check must establish Android compatibility before enabling the tunnel;
an executable name or a successful version print alone does not establish it.
The [service and tunnel targets](../../SPEC.md#explicit-device-service-integration)
make the patched Android tunnel individually selectable and part of future Full.
Normal installation consumes validated prebuilt binaries; independent tunnel
update/rollback remains available. Release and explicit source-build/recovery
reuse one wrapper-owned official-source/patch/build recipe, without another
repository checkout or ordinary-install compiler requirement. Core-only and local-function paths do not thereby require a remote
tunnel. These dependency controls are not shipped commands.
The owner's additional runit-root recovery layer is optional, not a prerequisite. This service
registration is an explicit device action here, not an implemented setup command.
This document does not introduce a new command or change installed behavior.

### Optional package selection

Public Core/Full90 and later expose these commands without Manager or enhanced
TUI. [CURRENT](../../CURRENT.md#operating-baseline) owns this device's selected
release; other installations must first update Core to a compatible signed generation.

```sh
codex provider install chatgpt
codex provider update chatgpt
codex provider install chatgpt --remote https://github.com/humtr/codex/releases/download/chatgpt-VERSION/
codex provider install chatgpt --local /absolute/signed-package
codex provider status chatgpt
codex provider chatgpt models
codex provider chatgpt prepare
codex provider rollback chatgpt
```

Install verifies an independent six-file Android payload and its detached
Ed25519 manifest using Core's admitted update authority. It stages privately,
checks every hash/size/mode and probes the executable without model work before
selecting an immutable generation. Status reports selection, not app/login/model
readiness. Explicit stable install/update authenticate the independent signed
Provider index before package acquisition; remote installation also accepts an
explicit immutable HTTPS directory. Downloads are bounded, do not retry effects
and never start during ordinary Core launch/update. Flat remote license assets
map back to the signed inventory. Failed admission preserves the selected generation. A running payload's
immutable executable remains pinned across selection changes; the installer never stops
jobs. Rollback retains the sequence high water and the previous admitted verifier
key, including across later authority changes. Neither command deletes older
payloads, downloads a desktop/rootfs, logs in or submits model work.
Historical signed eight-file payloads remain accepted with their complete
original inventories for rollback.

Older Providers through17 can pass their package lease into native/app children,
leaving selection busy after the Provider exits. Provider18 marks inherited
non-stdio descriptors close-on-exec at entry: the Provider keeps its lease, while
children receive only explicitly exported descriptors. Upgrading from older code
can still encounter an already inherited lock. Diagnose its kernel FD owner and
actual work; preserve unknown or active owners. Only an explicitly owned, settled
test backend may be retired as part of its qualified maintenance action. Ordinary
install never terminates Native work or removes a lock to force selection.
An unchanged-selection rejection and an uncertain mutation need different
reconciliation; neither permits replay or killing unrelated runtimes.

An explicit completed-work join/start admits a verified replacement Native
backend before reserving its new Native turn. It verifies scope, retained calls
and jobs and rotates the controller/token at that boundary; ordinary status does
not transfer ownership. This avoids an asynchronous exchange mistaking the already
recorded new turn for an old request on another owner. Failed or uncertain turns
remain in history and require explicit reconciliation, not automatic resubmission.

Selection does not rewrite existing generated profiles. Explicit `prepare` binds
the selected immutable executable to the generated native profile; existing native
config/auth/model defaults remain protected. Retain profile-pinned generations
through H. Provider work supplies native routing directly without republishing
shared or per-home profiles; its catalog snapshots stay in its own state.
Shared `work`, `chats`, `host`, `config`, `serve` and other Provider
arguments pass through `codex provider chatgpt` to that executable. The original
direct binary remains usable for pinned profiles and owned services.

Release producers use `codex-release-builder provider-package` with explicit
binary/assets/source-input inventory, exact base commit, generation/sequence,
private Ed25519 key, OpenSSL and absent output directory. The signed inventory
contains the binary, Cargo.lock, NOTICE, two dependency licenses and source-inputs.json;
it excludes desktop software, credentials, conversations and user configuration.
Local fixture signing and lifecycle evidence do not establish public delivery or
authorize device activation. See [the package contract](../../SPEC.md#optional-provider-packaging-and-selection)
and [current G acceptance](../../CURRENT.md#g-product-qualification).

Development retention uses [CURRENT's cleanup/restore procedure](../../CURRENT.md#development-artifact-retention).
The G composition driver archives verified executable copies after success;
`--keep-payloads` retains them for debugging. Journals, source, results and
workspaces remain. Restore archived original payloads before reusing a historical
fixture. Compiler caches are disposable; accepted/current/rollback candidate
artifacts stay outside them. No installed runtime or Chat state is cleaned by
this development helper.

| Usage | Path |
| --- | --- |
| Codex development | Native Codex -> Linux app API -> Provider work Chat -> MCP/local tools -> same native work |
| ChatGPT direct development | Active ChatGPT response -> MCP -> native work; active ChatGPT response supplies intelligence |
| ChatGPT general development | Active ChatGPT response -> MCP request/return -> the same Provider Codex path, including its app/session management |

MCP/local selects the inner tool return for an app-Provider work Chat. General
development still enters through MCP; ordinary direct ChatGPT local injection is
not an implemented equivalent. The Linux app is retained for useful authenticated
session reuse/replacement. Replacing it with browser/DOM is not the next objective.
Legacy service-wide `execution.mode` alone does not select the combined routing;
the shared work interface binds the selected reasoning source per work.
Stable native work, one current controller, related Chat reuse, grounded handoff,
original question/result return and model-free status are integration boundaries.

The [ChatGPT-operated Codex workflow](../../CURRENT.md#chatgpt-entry-and-task-management)
shares native Responses dispatch, approvals, events and history across managed,
direct and general-delegation entrances. The work interface selects its reasoning
source and transport per work; a service-wide `execution.mode` value alone does
not choose the combined route. Built-in defaults remain model_adapter/local_function;
this device explicitly selects MCP in Provider-owned configuration.

The primary native thread retains long-term history while Chat contexts can be
replaced. Successors require grounded summary/current receipt and old-controller
fencing. Original questions, results and CLI return bindings survive handoff.
A native compaction/reset does not create or attach a successor Chat. No uncertain
mid-task switch or silent legacy fallback is allowed.

Saved-Chat discovery/read works from both entrances for work-linked or unrelated
accessible conversations. Preserve message order, source and branch identity;
page the full requested range and expose unavailable content. Reading starts no
model task, resumes no work and transfers no control. General delegation and
[agent/orchestration](../../SPEC.md#agent-and-orchestration-extension-direction)
reuse this common bridge and independent native roles/contexts. Their actual,
synthetic and recovered-result scopes are accepted in
[CURRENT](../../CURRENT.md#audit-verification-and-remaining-acceptance); a universal quality,
latency, billing or capacity advantage remains unproven.

E/F design/integration, G product/public delivery and H preservation/retirement
have bounded acceptance at their [recorded scopes](../../CURRENT.md#product-completion-scope-and-work-order).
Original failed/interrupted cohorts and exact comparison inputs remain preserved.
Detailed context-capacity policy, optional SIWC/app-free research and viewer/setup
polish stay separate follow-ups. Linux-app removal is not a prerequisite.

## ChatGPT entry and task management

### What can be used now

Connect once using the manual procedure above, then select Termux Bridge from
the ChatGPT plugin picker. The [main guide](../../README.md#usage-by-entrypoint)
uses short task requests for direct work, managed delegation and existing-work
management. API fields and controller receipts are the assistant's concern;
users identify the project/task and the desired reasoning source.

| User intent | Existing control path |
| --- | --- |
| Work from this Chat | `codex_sessions` with direct reasoning, native attach/receipt, tools/report/finish |
| Delegate to a separate work Chat | Provider-started work with explicit app/project readiness and distinct assignment; retrieve its original final |
| List or inspect work | `codex_sessions` list and `codex_status` |
| Retrieve an existing result | Events/wait and public history on that work; no new task |
| Continue a settled task | Reconcile and resume/start the related work at its supported boundary |
| Stop a selected task | `codex_stop`; use tree Stop only when that whole scope is requested |

The only reasoning values are `direct` and `provider`: direct uses the active
Chat response; provider starts/manages a model response, including sequential
follow-ups in the same saved Chat. Neither proves an independent child model.
Previous names are not accepted as aliases. Existing installation state requires
explicit offline format migration with its originals retained. MCP/local_function describe tool/result transport. A plain queue
can start new inference; the existing no-reinference follow path additionally
requires verified saved-source admission. chat_input deliberately starts a new
saved-Chat user request and must not mirror an already active original prompt.

Provider15 exposes bind/unbind/join; see CURRENT for exact qualification and
remaining source-identity gaps. Earlier immutable Provider14 lacks these controls.
An existing Provider14 journal must be converted while its service is stopped:
`migrate-reasoning --state /absolute/state --backup /absolute/new-backup.json --target direct-provider`
uses the new verified executable and preserves original control records. The
installed CLI form is `codex provider chatgpt migrate-reasoning ...` after package
selection. It starts no service or inference. Before rolling back to Provider14,
use the new executable with `--target provider14` while stopped, then select the
old package. Pending/unknown execution must reconcile first. Runtime aliases are
not used to read old state. Installation changes package selection; existing
service executable pins and generated profiles need their explicit update too.
Ordinary `join` admits Bridge-submitted input directly on a selected completed
Native work, without saved-source crawling. Accept current context via
`codex_attach`; progress, actual tools/results and submitted final use the common
Native controls. Pending saved-source verification belongs to explicitly requested
`source_sync`/`source_join`, whose exact branch/account checks and context obligation
remain enforced. Repeated source participants, including after interruption, must
accept their current Native context before effects. A Provider continuation imports
full Native context and clears source divergence only after completion.

The [entrance contract](../../SPEC.md#entrance-equivalence-and-continuous-development-binding)
requires one actual execution and preserved response quality. General Android/web
requests start outside Provider's renderer: model guidance can encourage Bridge
use, but original host input/message identity and the later host public final must
be established from real metadata/events. Model-supplied arguments/completion do
not establish those source facts. No native tools are fabricated for unjoined host
output. Saved-source verification/recovery is explicit; no whole-Chat timer reader
runs in ordinary development. The offline notification prototype was withdrawn
and never activated. CURRENT separates delivered behavior, Native/TUI
qualification and remaining actual Android/Web boundaries.

The assistant should report the returned work/thread identity with progress and
results so follow-ups can address the same work. Resolve ambiguous targets before
mutation. Prefer original native events/final items over a fresh summary task.
Do not treat an unready app as blocking primary current-Chat reasoning; managed
work and app-backed reviewers have their own readiness requirements.

A supported ChatGPT project instruction may hold a chosen absolute workspace
and preference for "this Chat" or "a separate work Chat". That is contextual
guidance: it does not enable a connection, validate a local path or assign a
controller. Native project instructions still come from the real workspace.
ChatGPT project availability and connection use must be checked in the actual
client; configuring Provider's `history.project_name` does not install user-facing
instructions or bind the requesting Chat to that project.
See the [official project guide](https://learn.chatgpt.com/docs/projects).

### Proposed convenience, not shipped behavior

The recommended small next step is to package the existing MCP app with **one
shared workflow skill** under Termux Bridge. It would carry both direct and
managed procedures, including discovery, finite work budgets, native attachment,
project instructions, original question/result delivery and uncertain-call
reconciliation. Users would select the plugin and state the task; route choice
would remain explicit or come from a displayed/confirmed preference. Automatic
skill selection is not guaranteed on every request. This repository currently
ships neither that skill nor a combined plugin package.

Official OpenAI documentation describes
[plugins bundling reusable skills and app connections](https://learn.chatgpt.com/docs/migrate-custom-gpts).
This is a packaging option for the current Provider, not another executor or a
second history authority. Exact account access and real direct/delegated
selection still need verification before delivery.

If text requests still make work hard to manage, a later **compact work panel**
could offer these controls over the same service:

| Panel area | Proposed content/actions |
| --- | --- |
| Connection and readiness | Device/service, bound account/workspace, tunnel observation, and app readiness only for routes that need it |
| Task entry | Choose this Chat or a separate work Chat; show the selected workspace/project and task before submission |
| Existing work | Work/thread identity, phase, pending human input and original result; inspect, continue or Stop the selected scope |

Official [MCP Apps UI support](https://developers.openai.com/plugins/build/app-quickstart)
makes an embedded ChatGPT panel possible. No such UI resources or panel exist in
the current tools-only service. A panel would reuse existing Provider controls,
route direct reasoning through the active Chat, preserve native approvals and
human answers, and distinguish requested Stop from confirmed interruption.
It must not create another transcript store, retry uncertain effects or convert
ordinary status reads into app acquisition/model requests. The proposed order is
skill packaging, then optional panel work; implementation remains deferred.

## Opt-in operator controls

The signed Provider exposes common `work`, `chats` and `host` CLI commands over
the same authenticated MCP service. `host status` observes service
and app state without acquiring a missing host. App/source controls can run
without a native execution binding; native work requires the declared
Core/home/workspace. `chats list` supports project/title/time filters and an
account/filter-bound `list_cursor`; `chats read` returns stable IDs, parents,
source cursors and explicit unavailable content. Range enumeration (`complete`)
and readable content (`content_complete`) are separate.

New native work may select `reasoning=direct` (origin Chat/MCP) or
`reasoning=provider` (saved project Chat, explicit return path). Related starts
retain the native work. `work events` retrieves its original public results;
`work wait --max-wait-seconds N` waits locally up to300 seconds for a genuine
question, completion or pending handle without a supervising model. Explicit
evaluation supplies the current controller, full history cursor, decision and
findings text. Managed completion is its original native final and completed
status; an explicit `codex_finish` tool item is required only for current
reasoning. Follow history pagination before declaring a final absent. At verified
completion, events/wait exposes `completed_result` with the current native turn's
original final items even when the supplied cursor already covers them. A final
need not be the last history item. Reuse returned cursors for incremental waits.
Explicit
`work chat-input`/`chat-result` use complete saved-source cursors and durable
message IDs; uncertain input is observed without resend. A complete saved read
also provides an `input_cursor` at its public final when only a finished hidden
final suffix follows it. Input admission retains the original cursor and binds
delivery to that public final before sending, because the app can remove the
hidden terminal on follow-up. Result receipts expose that binding; old receipts
without it retain their original strict parent check. `work transfer` and
`transfer-accept` fence a completed source and require the target receipt before
changing its route. Actual combined routes/agents/fault/efficiency evidence and
selected hint application retain their bounded evidence. G public delivery and
H retirement are complete; see CURRENT for the actual device baseline.

The qualified source candidate waits on local owned-stream state during an
active input. Completion still requires the saved original message. A failed
retrieval returns `complete=false` with `observation_unavailable`; it does not
cancel or resend development. HTTP429 starts a shared account cooldown of at
least60s, increasing up to300s on repeated limits and honoring `Retry-After`.
`host status` exposes static read/backoff diagnostics. Cooldown observations make
no saved-Chat HTTP request. Bounded actual Android pacing proof is recorded in
CURRENT; it does not establish whole-F acceptance.

The explicit configuration `execution.mode="operator"` requires absolute `execution.core`,
`execution.home` and `execution.workspace`. `transport.return_path="mcp"` exposes
the controls through the Chat-side connector; `"local_function"` uses the same
Operator Host through app local functions without that connector or tunnel. D2
implements this local binding and compares all four control/transport combinations
from Codex. All four matched six-task real-development/handoff runs pass; detailed timing,
request counts, independent audits and retained failures are in CURRENT. ChatGPT-first
local discovery is deferred. The default remains `model_adapter`; binding/transport
changes need a service restart. An unbound operator configuration is invalid. Current G/H acceptance and the
operating device are recorded in CURRENT; fresh setup remains the procedure
above. `mcp-stdio` passes the private local bearer;
never put that credential in Chat prompts or a remote tool argument.

| Control | Behavior |
| --- | --- |
| codex_sessions | List/create/resume independent work; fork children; start genuine user tasks; request settled native compaction; claim reconciled recovery or explicit fenced takeover |
| codex_attach | Return current instructions/history/summary through the tool result; echo its receipt to accept the offered controller |
| codex_tools / codex_execute | Discover complete native function/custom/search schemas and execute with exact logical retries |
| codex_history | Read public native items and public input context since their verified source cursors |
| codex_report / codex_finish | Stream reports into native CLI/history and confirm final native settlement |
| codex_status / codex_stop | Observe native activity and host freshness separately; fence and interrupt the owned turn/jobs, reporting confirmations |
| codex_answer | Forward the user's supplied native question answer; never grant approvals |
| codex_handoff | Record a source-bound incremental summary, fence the old controller and optionally request a successor Chat in the same native thread |
| codex_native_summary | Supply an upstream-requested inference summary without rotating Chat |

`native_catalog/native_tool` remain isolated reviewer transports. Configured native
reviewers retain independent contexts and real approvals. Chat-first startup and
primary execution need no desktop; Codex-origin Chat start and app-backed reviewers
still acquire the explicitly provisioned app lazily. An optional viewer is not
started by operator bootstrap. Real consent/attachment smoke and ten-cycle TMPDIR
development, actual successor renewal, AI-project development and active recovery
have bounded actual proof; C2 additionally verifies saved-Chat multi-turn continuity
and successor development. Synthetic bridge proof retains its separate scope.
The current first-start
correction forwards only the actual current user request. Origin/history remain
in control attachment on the selected transport; successor and native-summary starts use explicit non-user
input. Actual app admission and server acceptance are separate checks.
App permission/authentication waits appear in the original CLI/history and
`codex_status`; an app error or an explicit final without native completion
requires reconciliation. The existing `reveal --thread ID` command opens only
the bound owned Chat after identity checks, without inference or consent.
In opt-in operator mode, delegated `approve --thread ID` first invokes the signed app's
real approval action without a window. Both this path and the visible fallback
prefer **Allow … for this conversation**; only if that offered scope is absent do
they use **Allow once**. Success never triggers a second approval. Only a known
pre-dispatch capability rejection permits fallback; uncertain results stop for
observation. Native tool/reviewer approval policy is unchanged.
`approve-ui --thread ID` opens the bound private viewer and Chat, then actually
clicks the preferred option in its exact pending message card. Cold UI readiness
is observed only on this path, with a bound and no fixed startup delay; the app
action does not wait for the viewer. It exercises the
same fallback used by `approve` when the app action is unavailable. Account,
controller, duplicate/stop and server-offered target fences apply to both paths.
Forms/passkeys/authentication or ambiguous UI controls remain pending for manual
interaction. Conversation permission expires with this Chat, not the Codex thread.
Both the background action and real UI conversation option have bounded actual
read-only native execution/report/final proof, recorded in CURRENT.md. Those
approval trials alone do not qualify long development or successor Chat renewal;
the separately retained C/C2 and later direct-entry trials establish only their
own development/handoff scopes.

The native public-history reader pages upstream items, skips reasoning and active
items, and refuses explicit safety bounds rather than silently truncating. Current
bounds are 16MiB/native frame, 64MiB/complete history or rollout read and4096
public-item pages. They are
not Chat context limits or hours-long development qualification. Context-only
input messages missing from the public-items API use an owner-private, thread-
matched, read-only native rollout projection; reasoning/internal metadata is
excluded. History and context have separate cursors. Attach receipts cover both;
handoff and recovery require their source cursors. Later committed instructions
return with the original MCP tool result. Input queued by upstream during an
active turn becomes observable when upstream commits it on a subsequent round.

For an existing completed work, `codex_sessions join` takes its explicit
`work_id`, the Bridge-submitted `task` and a stable `call_id`. It reuses the
Native start transition with Direct reasoning, returns the generation controller,
and performs no saved-Chat retrieval or new Provider inference. Read and accept
current context through `codex_attach` before tools; use `codex_report` and
`codex_finish` for full submitted progress/final. Exact retries reuse their
receipt. Busy, stopped and unreconciled works are rejected, not taken over.
Submitted text is not falsely labelled as an unobserved ChatGPT original.
Record each full public progress message verbatim before showing it in ChatGPT;
on joining, also record progress already shown for the same task. The Direct
attachment's `progress` policy and report declaration share one definition.
A native tool's report can carry that same message without a second report call.
This preserves public detail and does not request hidden reasoning. Actual host
commentary still requires independent comparison; the policy is model guidance.
Prepare the complete public answer before `codex_finish`, then return that same
text unchanged in ChatGPT after success. The tool declaration and attachment
provide one common completion instruction; its presence does not prove that a
host obeyed it. Compare the actual later public final separately.
New public reports, finals, native-call reports and handoff summaries containing
known current private control values are rejected before admission with
`private_control_in_public_text`. Correct only those values under the same
unreserved call ID. Public work IDs, example code and other public text remain
unchanged; existing exact receipts remain recoverable. Original user/tool text
and retained private history are not rewritten.

Before a new native prompt, a compatible TUI uses `work synchronize --thread-id`
as a Native-only barrier for an already admitted Direct/source execution. It
retrieves no saved Chat. The draft retains attachments/mentions and is restored
on cancellation or a hard failure. Ordinary development has no periodic remote
history reader, including after restart or binding.

`work source-sync` and `work source-join` explicitly request saved-source
verification/recovery. They retain original-ID/account/branch verification and
source context acceptance; their saved reads obey cooldowns. They are separate
from ordinary Bridge admission. Actual Android/Web original identity and the
later Chat public final require their own qualification; CURRENT records those
remaining boundaries.

## Configuration and commands

The executable is `codex-chatgpt-provider`. The selected signed package exposes
these commands through `codex provider chatgpt ...`. Its settings and catalog have
one authority:
`$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, default
`~/.config/codex/providers/chatgpt.toml`.
Broader ChatGPT setup/management UX is deferred; explicit project selection and
configuration/operations will remain its authority. Scope and priority are in
[CURRENT.md](../../CURRENT.md#operating-baseline).

- `models` reads `catalog.models`; `catalog` generates native model JSON.
- `config show|effective|validate|edit` inspects or edits settings. Effective output
  reports file/default/per-model origins and `runtime_application=not_queried`.
- `config set FIELD VALUE [FIELD VALUE ...]` validates dependent values in one
  atomic transaction; `config unset FIELD ...` restores defaults. Cooperating
  writers serialize, stale editors refuse overwrite, and invalid edits preserve
  the old file. `config edit` uses `VISUAL`/`EDITOR`.
- `prepare [--home PATH] [--state PATH]` generates an immutable catalog and shared
  command-auth profile for an existing selected home, preserving local overrides,
  base settings and login. It may authenticate/start the configured Rust service;
  it submits no Chat or project creation. No bearer is written in the profile.

Provider16 adds `service.startup=standalone|supervised`. Standalone
is the default and retains lazy service startup. Supervised preparation/token
requests wait for authenticated foreground-service readiness, for at most45s,
and never start a detached Provider. Status stays read-only. Configure the mode
through `config set service.startup supervised` only after selecting code that
supports it; Provider15 and earlier do not. Setting it does not enable runit or
transfer an existing process into supervision. Preserve the old configuration
before maintenance, settle work and stop the prior owner, then start the chosen
supervisor. Before an old-code rollback, restore its compatible configuration
while stopped. This device uses supervised startup with Provider16; CURRENT
owns exact delivery/activation proof. No shared setup command is implied.

Initial catalog data is in `default-models.toml`. This seed includes GPT-6 and
explicitly selected Pro; the hidden Instant alias retains existing bindings.
The source input implementation preserves a prepared Chat's actual host model
and effort without filtering by model generation. `work chat-input` accepts
optional `--model` and `--effort`; a Provider `work start` can also change Native
settings, persisting under the Native API's normal semantics. Existing Native
resume preserves saved settings and fork copies the source settings explicitly.
These changes are not yet installed; CURRENT identifies qualified delivery.
Model/effort/context entries are editable without rebuilding unless the host
protocol changes. The current implementation's `host models` reads the app's
available picker choices and adds configured models. Discovery and
Native profile preparation have different purposes: standalone `models`/`catalog`
inspect offline configured data. Refresh the actual app list with
`codex provider chatgpt host models`, then run `codex provider chatgpt prepare`
at an idle startup boundary to publish those choices for Native selection.
Preparation reads retained metadata without acquiring the app; without an
observation it uses explicitly identified configured-only data. Use
`host models --refresh false` to inspect that nonacquiring view. Actual app-model
execution verifies the current account before admitting the observed route.
Listing a route does not prove entitlement. Automated qualification uses Pro
only for a separately authorized test.

The source input implementation keeps common app-to-Native effort names in
`default-efforts.toml`. Additional/corrected mappings can be supplied through
`[catalog.effort_map]` in the Provider config, without copying model definitions:

```toml
[catalog.effort_map]
custom_app_effort = "ultra"
```

The exact app token is retained for transmission. An existing model-specific
route has precedence over this general mapping; unknown mappings remain unknown.
Only values representable by the Native effort protocol are accepted.
Bootstrap model/effort defaults are data in `default-selection.toml`; user
`model.default` and `model.reasoning_effort` settings override them.

Per-model context fields include `window_tokens` and `auto_compact_token_limit`;
explicit upstream invocation overrides win. The user's1,000,000-token override is
preserved policy, not measured Chat capacity. An unset threshold delegates to
upstream and does not disable model/hash-triggered compaction. Saving settings does
not mutate a bound native catalog; prepare for the next startup. Provider reads
compatible host-route/history changes at request admission.

`/model` is the primary enhanced-TUI selection surface; upstream `--profile chatgpt`
is the configuration fallback, distinct from Manager login accounts. Current exact
source/device composition proof, accepted actual TUI re-entry and separate device
activation are in
[CURRENT](../../CURRENT.md#audit-verification-and-remaining-acceptance).

## Service and observation

| Provider command | Effect |
| --- | --- |
| `serve [--descriptor PATH] [--state PATH] [--port PORT]` | Run the loopback Responses/control service; app-backed model work uses a provisioned owned host or explicit qualification relay, while direct Chat-first native work needs no app host |
| `token [--state PATH]` | Authenticate and optionally start that same configured executable; never paste its credential output into chat/logs |
| `status [--state PATH] [--thread ID]` | Read local process/last lane observation; never starts, queries, resumes or cancels Chat |
| `reveal --thread ID [--state PATH]` | Select the exact bound saved Chat conversation through an existing service; no model message/approval/viewer startup |
| `approve --thread ID [--state PATH]` | Delegated approval: app action first, known unsupported action falls back to UI; prefer conversation scope, otherwise once |
| `approve-ui --thread ID [--state PATH]` | Open the bound viewer/Chat and click the same preferred approval scope; never retry uncertain delivery |
| `stop [--state PATH]` | Request PID-bound graceful Provider service shutdown; preserve journal/app profile/config and report unconfirmed shutdown accurately |
| `view [--state PATH]` | Open the optional Rust viewer against configured noVNC assets/private RFB; may start service, never submit Chat |
| `mcp-stdio [--state PATH]` | Forward bounded multiplexed MCP JSON-RPC to an already-running verified Provider for an existing tunnel |

Run qualification services from private immutable binary snapshots, not Cargo
outputs replaced by later builds. Bridge-source revision, process/account/workspace
identity and exclusive app-home lease bind the owner. Failed readiness of a live
owner never authorizes restarting it. Startups serialize; restart retains the
journal's loopback port and rotates its local credential. An occupied saved port
fails before app startup. Content-free startup errors stay private.

`host.rootfs` selects an already provisioned official-app rootfs. Rust uses PRoot,
Xvnc and private Chromium pipes, with app home under Provider state and temporary
display resources. No existing profile is copied; no rootfs/app install or login is
implicit. `--descriptor` is an explicit qualification relay, not a production
Bun/Python dependency or automatic fallback. Ordinary Base/Full startup/update must
not download or log into the app. Third-party app binaries/auth/user data are not
bundled here. Korean fonts and pinned icons of the existing qualification desktop
are not proof of a changed Rust viewer or new user login.

Status separates endpoint liveness, last host observation, pending native calls,
host confirmation and stop requested/acknowledged/unknown. An old journal or a
content-free SSE keepalive proves neither current model progress nor task
completion. Provider service shutdown does not prove a native tool stopped or undo
its effects; native task control has a separate upstream authority. Physical CLI
progress and confirmed task Stop remain explicit acceptance requirements.
Service shutdown is confirmed by the pinned kernel process handle; losing the
recorded executable path during teardown leaves stop unconfirmed until actual exit.

The optional viewer requires `host.viewer_assets`; an owned lazy operator host
registers it after the app actually starts. An external relay can explicitly use
`host.viewer_socket` for its verified private RFB socket. It
uses a separate temporary capability. Missing assets do not block inference.
Mobile keyboard/scroll/Esc/fullscreen controls require physical acceptance beyond
HTTP/WebSocket/RFB fixtures. Revealing a conversation does not prove the visible
Android screen changed. Do not restart a live owner to apply host settings.

For an explicitly configured installed Provider, open its optional viewer with:

```sh
codex provider chatgpt view --state /absolute/private/provider-state
```

The command uses the configured viewer assets/private RFB capability. It may
start the Provider service and submits no model task; intent success does not
confirm physical visibility. Existing notifications and their pinned installed
helpers remain unchanged by source retirement. noVNC visual/touch/IME polish
retains its lower priority under [app management and viewer work](../../CURRENT.md#linux-chatgpt-app-management-and-deferred-viewer-work).

The [recorded app/host capability evidence](../../CURRENT.md#evidence-map-and-limits)
covers authenticated Chat/project APIs, session reuse/input, process ownership,
connection/readiness/recovery, configuration and shared management surfaces.
F implements the selected common management interfaces; H preserves reusable
qualification helpers and retires duplicate launch policy. Viewer usability stays
a separate physical validation scope.

## Device runit services

The user explicitly opted the existing device into the shared runit directory
`$PREFIX/var/service/`, beside the unchanged tdev services. Component ownership
does not change: Provider implements the bridge/MCP service; OpenAI's existing
`tunnel-client` transports MCP; Provider owns the Linux app/Xvnc/private pipes.
No new tunnel implementation or remote tunnel is introduced.

| Registered service | Local configuration |
| --- | --- |
| `codex-chatgpt-provider` | Runs `codex provider chatgpt serve` in the foreground with the existing state and explicit service environment |
| `codex-chatgpt-tunnel` | Runs the existing tunnel profile through `tunnel-client run`; startup waits for authenticated Provider readiness |
| `codex-chatgpt-host` | Every15s observes host status and invokes `host ensure` only for confirmed absence; preserves present/unready/unknown owners |

Service helpers and private rotated logs live under
`~/.local/state/codex/providers/chatgpt/runit/`. Each service has a runit `log/run`
using `svlogd`, capped at1MB per file with5 retained rotations. The tunnel profile's
pre-existing private application log is separate. Helpers select the existing
Core/account/workspace/state; inherited interactive account/profile switches do
not retarget them. No credential value is stored in a run script. The tunnel
profile retains its existing credential file reference. Shared private
`runit/environment` supplies `CODEX_SERVICE_HOME`, `CODEX_SERVICE_PREFIX`,
`CODEX_SERVICE_CORE`, `CODEX_PROVIDER_STATE`, `CODEX_PROVIDER_BIN`,
`TUNNEL_CLIENT_BIN`, `TUNNEL_PROFILE_DIR` and `TUNNEL_PROFILE_NAME`. The observer and stdio wrapper
uses an explicitly selected admitted immutable Provider image, avoiding repeated
Core activation-lock contention; rebind it during deliberate package/service
maintenance. Provider's `execution.home` and `execution.workspace` select native
execution, rather than a hardcoded account name in the service scripts.

The tunnel executable must be Android-compatible. This device's global
`tunnel-client` is native0.0.14, while this Codex service now pins the wrapper-built
Android0.0.18-termux.0ad5d2db16a9 HTTP/2 recovery image. Its old scoped image
is retained for rollback; unrelated consumers are preserved. Neither is updated by the current
Codex package updater. Upstream tracking/Termux release maintenance and wrapper
dependency integration remain separate work in
[CURRENT](../../CURRENT.md#product-completion-scope-and-work-order); the current
scoped pin is not an automatic update channel. Inactive Linux images were archived
outside executable paths. Do not infer compatibility from the filename or modify a system
resolver to accommodate a Linux binary. The MCP wrapper keeps stderr separate
from its JSON-RPC stdout; runit service logs may merge their own stderr.

Provider restart rotates its local bearer. The stdio process verifies the current
service endpoint and bearer per RPC; loss returns an explicit error and a later
request can use the replacement service. The tunnel remains running across a
Provider PID change. Rebinding its admitted immutable stdio image requires one
deliberate transport restart. It does not create a remote tunnel, retry a model
request or replay native effects. Service Stop and native task Stop remain
different operations. Inspect original work before any maintenance/recovery.

Runit restarts exited processes. The host keeper reacquires a confirmed missing
host; it does not repair an arbitrary hung/partially failed app/display by killing
it. Inspect such unknown state and settle work before deliberate host/service
repair. Logs/readiness never establish physical viewer usability. Xvnc is acquired
through Provider; no independent Xvnc or browser/desktop launcher is installed.

During maintenance, serialize service startup and profile preparation. `prepare`
can start a detached Provider when the supervisor has not yet bound the endpoint.
Start runit alone, verify that its supervised PID matches the actual authenticated
Provider status, then prepare the generated profile. If preparing before enabling
runit, stop that prepared idle service first. Concurrent starts can leave runit
reporting `provider_already_running` while the detached service owns the endpoint.
Do not resolve it by killing unrelated Native sessions or replaying work.

The old local tmux runtime was explicitly stopped before runit activation. Its
remote alias remains, but `tunnel-client runtimes` local metadata describes that
retired tmux lifetime and is not authoritative for these services. Use `sv status`,
Provider status and the configured runtime's current local health endpoint.
Local health/ready responses and remote control-plane health are separate checks.

Start/restore with `sv-enable codex-chatgpt-provider`, optionally
`sv-enable codex-chatgpt-host`, then `sv-enable codex-chatgpt-tunnel`.
For persistent maintenance disable tunnel, host, then Provider using `sv-disable`.
Stopping only a process will trigger desired-up recovery. Standard
termux-services/runit is sufficient; the owner's optional `service-daemon`
root-recovery layer adds root-loss repair and is installed from
`~/prj/termux-services`, not this repository. Neither layer survives a fully killed
Termux UID without an external Boot/login opportunity. Future minimal setup
should register the appropriate services explicitly; no general installer or
login automation is delivered by this device configuration.

## Existing result transports

`transport.return_path` chooses `local_function` (built-in default) or explicit
`mcp`. This is a result-transport setting within the existing model adapter, not
selection of the ChatGPT-operated workflow. Return path/connector changes
require settled service restart; uncertain dispatch never triggers fallback.

Local functions return actual native results on their original app calls.
The MCP path exposes `native_catalog` and `native_tool` under a per-turn capability,
projects native calls through the same execution/history path and returns results
on the original MCP invocation. It does not expose general session/context control.
Neutral discovery describes capabilities; catalog access is read-only metadata and
execution advertises possible state changes. Exact logical IDs/inputs/results bind
retries independently of JSON-RPC IDs. A detached MCP waiter is not an execution
cancel; the owning Responses request retains its own cancellation semantics.

Set `transport.connector_name` for the installed connector; the default is
`Termux Bridge`. Historical `Codex Native2` naming is not required. Tunnel IDs,
clients and targets remain independent from Tdev. A changed tunnel alias may still
run its old stdio child: reconnect after settlement and verify actual executable/
state argv instead of relying on readiness metadata. No replay follows uncertainty.
The current MCP display name is `Termux Bridge`; earlier `Termux Codex Provider`
references identify historical trials. Its installed tool list must
match the selected operating mode. Refresh only the verified trial connection
and read back its actions before spending Chat requests on a changed MCP schema.

Host approval/authentication retains the exact request and shows a waiting report.
Use the signed app's actual consent control; existing explicit bounded delegation
may authorize selecting it, never fabricating consent. A correlated host permission
reply is counted separately and does not retire the retained MCP capability. A
stopped/restarted owner never silently reactivates old pending work.

The bounded actual R11 MCP trial completed two commands/effect+verification and
public/final native output, with one user object and two app dispatches including
one real Allow reply. This is not full-control or long-development qualification.
App dispatches, user-role objects, tool returns and charged Chat message limits
are different quantities; the last remains unknown. Neither same-call returns nor
project grouping establishes a free allowance or absent remote history.

## Context, projects and transcripts

The existing adapter imports public native history on cold admission and sends
needed deltas/results while warm; import is not itself model summarization. Native
Codex requests inference-context compaction, ChatGPT supplies the summary through
Provider, and upstream commits its active-context replacement. The Provider
acknowledges that replacement once, retains the owned Chat and restores normal
policy/tools without resending the resident summary. The original durable native
history remains. There is
no independently verified Chat remaining-context sensor. This coupled lifecycle
belongs to the model adapter; operator Chat renewal follows the
[same-thread incremental handoff design](../../CURRENT.md#implemented-and-qualified-at-recorded-scope).

The existing adapter tracks warm-prefix deltas and native checkpoint acknowledgment.
Summary policy/results are transient until upstream commits the offered checkpoint;
identical summary text in unchanged old input is not acknowledgment. The public
checkpoint is JSON with `format=codex-chatgpt-public-continuation`, version1 and
a nonempty summary. Whitespace and key order are insignificant; unknown/duplicate
fields and unsupported versions are rejected. It neither decodes OpenAI's foreign
opaque reasoning nor reads retired `ocx1:` records.
Native compaction and model/hash transitions remain upstream lifecycle operations.

`checkpoint --thread ID --from-model ID --model ID --effort EFFORT [--state PATH]`
reads a complete settled public snapshot from stdin and emits an external-context
message. TUI loads all Full-detail turn pages, independently of display limits;
explicit Summary/NotLoaded snapshots are refused. The command starts no
app/inference and writes no native history. It may initialize the private owner
marker before first use. The existing owner authenticates this public message and
its native ID because Codex strips internal metadata on the Provider wire. TUI
passes its auth definition's exact state and uses the upstream API to persist it,
preserving original history/thread and source readability. Same-thread or declared
fork/parent lineage and current policy/later input are validated. Historical calls
are quoted observations, never executable replay. A synthetic API/PTY handoff does
not establish all real-Chat TUI/downshift/cold-resume transitions.

Public initial/intermediate/final reports and real tools use native output events.
When the app appends to an older message after a later report, native output uses
a separate continuation item; completed native history is not reopened. Stop
requires terminal proof before clearing stale local stream state. A cached
committed response with no pending calls can prove host settlement after context
replacement; an idle label alone cannot.
Native Stop loads an exact owned `notLoaded` thread with verified scope/routing
solely to inspect/terminate jobs, without a new user turn or model request.
Known MemoryCite/box presentation converts to append-only terminal Markdown while
literal code, unknown markup and exact tool data remain unchanged. No formatting
model request or private reasoning export occurs. A rejected single gateway envelope
may receive at most three original-call error corrections before any native effect;
uncertain delivery/identity/compaction errors never use that path. Native function
argument validation stays native; correction model starts remain real usage.

Independent native reviewer lanes preserve upstream decision schemas and exclude
the primary project; self-review is not equivalent. Existing allow/deny fixtures
have synthetic replies, not qualified real Chat judgment. A question fixture also
returns the actual answer on the original call; physical question UI is separate.

Project mode is explicit, for example:

```sh
codex provider chatgpt config set history.mode project history.project_name "Codex"
```

`history.project_id` disambiguates an exact owned private/unshared project;
`history.create_if_missing` controls creation. Preserve existing memory scope,
conversations and personalization. Duplicate/inaccessible destinations fail, and
uncertain create is reconciled before retry. Destination changes affect new lanes.
Project grouping reduces clutter, not stored history or established message costs.
Archive/delete/temporary retention is not admitted. Full contracts are in
[SPEC](../../SPEC.md#approved-provider-configuration-context-and-remote-project-behavior).

## Verification and migration

Historical live G, D3 and direct-entry drivers no longer select a particular
device's temporary roots, experimental app installation or tunnel alias. They
require explicit existing inputs, validated before preparation. Input names below
have the prefix `CODEX_QUALIFICATION_`; path inputs must resolve to existing paths.
These are development-driver inputs, not general user setup options.

| Driver | Explicit input names |
| --- | --- |
| G prepare | `PRIOR_ROOT`, `CONNECTOR_REFERENCE`, `BUN`, `TUNNEL_PROFILE`, `TUNNEL_ALIAS` |
| G cold continuation | `PRIOR_BATCH` |
| Direct-entry prepare | `D2_ROOT`, `DESKTOP_SESSION`, `CONNECTOR_REFERENCE`, `TUNNEL_PROFILE`, `TUNNEL_ALIAS`, `FRONTEND` |
| D3 prepare | `DESKTOP_SESSION`, `CONNECTOR_REFERENCE`, `FRONTEND`, `BUN`, `PRIOR_ENVIRONMENT`, `TUNNEL_ALIAS`, `TUNNEL_ENVIRONMENT`, `TUNNEL_PROFILE`; `STOPPED_WORK_PROOF` when its CLI option is absent |

`TUNNEL_ALIAS` is a string. `TUNNEL_PROFILE` is the explicit private JSON profile
containing `control_plane`; `TUNNEL_ENVIRONMENT` is a private JSON string map or an
object containing that map as `tunnel_environment`. Keep these inputs private.
D3 subsequent tunnel operations require the same explicit environment/profile;
its admitted case record determines the alias for fault recovery. App refresh
uses the recorded connector and desktop paths and verifies exact connector
ownership before app access. A missing input never falls back to a live device.
Accepted historical fixtures/results remain closed; changing these drivers does
not authorize replaying them. New work needs its own explicit intent, inputs and
reconciled service state. These legacy tmux-era drivers do not manage the current
runit services.

Intentional fixed values remain: protocol/ABI and socket limits, supported app
API bindings and asset identities, bounded test scenarios, editable Provider
defaults, and the current signed publication origins. Repository separation is
deferred; changing those trust origins is a separate coordinated migration.

```sh
BUN=/absolute/path/to/qualified/bun scripts/check.sh provider
```

This runs Rust fmt/tests/clippy/release plus app-bridge/CDP and Python package/
qualification regression suites in disposable roots without Chat submissions.
Automated qualification never
samples Pro or GPT-5.6; live trials first declare purpose, scope and expected
submissions. Missing Bun is an unfinished
app-bridge/qualification gate, not a Core or production Provider dependency.
Source, actual Android/app proof, signed delivery and device activation each
retain their own evidence; package selection alone does not establish readiness.

The Android qualification job retains an unsigned snapshot for30 days, including
the executable, exact source/run/toolchain identity, dependency lock and licenses.
Before copying its executable into a private immutable qualification location,
read back every declared file against its inventory and source commit:

```sh
python3 -B providers/chatgpt/candidate.py verify \
  --artifact /absolute/path/to/provider-android --source . \
  --source-sha EXACT_40_HEX_COMMIT
```

The verifier rejects missing/extra/nonregular files, changed content and source
assets. Its unsigned inventory establishes consistency with the selected build
record, not publisher authentication or device acceptance. Preserve that record
with the build/test and disposable restart/rollback proof. It never installs,
starts a host or changes the operating Provider.

The existing no-MCP Rust path already passed bounded direct `~/prj/ai` development
and supervisor verification. Future materially different paths need their own real
trial there under its AGENTS/Git baseline, without a temporary worktree or a
supervisor-written substitute patch. Preserve unrelated user edits; exercise
analysis, failure/correction, tests and latest-state resume. The single
[acceptance matrix](../../CURRENT.md#audit-verification-and-remaining-acceptance) owns open
gates. That no-MCP proof alone does not qualify MCP controls; separately recorded
C/C2, D2/D3 and direct-entry proof apply at their exact inputs/scopes. F common
integration/general delegation and G's declared product/delivery scope are
accepted with their recorded limits. Release sequence3 neither repeats earlier
development nor replaces an old failed result with a passing one.

The experimental installer/runtime source is retired from the active tree after
accepted Rust replacement. Exact source, user1m changes/tests and licenses are
preserved in experiment history and private retirement evidence. Preserve pinned
installed payloads, credentials and conversations through an explicit device
migration; source retirement alone does not activate a replacement. Use the
authenticated Provider package interface for new installations. Reusable
qualification tooling remains. See [NOTICE](NOTICE.md) for provenance and licenses.
