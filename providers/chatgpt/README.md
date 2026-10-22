# ChatGPT Provider / Codex bridge

This optional Rust component lives on `rewrite/rust-core`. Core launches the
verified upstream runtime; Provider owns Chat connection, model configuration,
Responses/tool-return mapping and host lifetime. Upstream Codex executes tools,
applies its approval/reviewer policy and owns canonical history. Neither Core nor
Manager links Provider implementation. Manager and enhanced TUI are not required
for the upstream configuration route.

[CURRENT](../../CURRENT.md#operating-baseline) distinguishes accepted source,
signed publication, selected packages and actual running owners. The accepted
public generation is `chatgpt-6aa228034346-s3`, with release sequence3. Provider
is the component name; `chatgpt` is its CLI identifier. The sequence is release
metadata, not part of the component name.
This device selects Full93 and this signed Provider generation. Its shared native
profile binds the immutable Rust executable; same-account owned-host readiness,
local16-tool MCP and the existing tunnel have been verified without model work.

## Bridge design and implementation status

Provider contains the ChatGPT–Codex bridge: model/native-tool adapters and shared
session, connection and host management. MCP is a tool/result protocol used by
that bridge; the app displays its MCP connection as **Termux Bridge**. ChatGPT
supplies judgment; native Codex executes, approves and records. `operator` remains
a code/config/API label for native-work controls, not another reasoning agent.

E's [selected contracts](../../SPEC.md#e-selected-common-interfaces-and-lifecycle)
and [capability findings](../../CURRENT.md#linux-app-and-host-capability-research)
are settled. F1–F9 common integration and G's declared runtime/public delivery
scope are accepted. H source/worktree retirement and Provider/full verification
are complete and committed. Original actual, synthetic, interrupted and failed
proofs retain their own inputs and limits; see
[CURRENT](../../CURRENT.md#remaining-acceptance-and-delivery).

All annotation values except idempotentHint are intentionally fixed. The
[idempotency study](../../CURRENT.md#idempotency-preliminary-research-and-phased-work)
remains historical research; E settled and implemented the common operation/
duplicate/result-recovery contract and per-tool targets. F qualified
and applied them at the recorded scope. The Rust policy selects true for fresh source/native reads and
keyed execution/report/finish/handoff/answer/attach operations; mixed host/session/
summary/Stop tools retain false. Historical JavaScript declarations stay in the
preserved experiment/installation. Hint changes alone have no demonstrated
performance benefit. In the observed ChatGPT backend tools/list response,
idempotentHint is omitted even though the Rust service emits it; internal
consumption is unknown. Other hints must remain unchanged.

The [idempotency implementation](../../CURRENT.md#idempotency-preliminary-research-and-phased-work)
shares in-flight duplicate results and durable mutation/finish/registration/round
receipts, with per-work admission and Stop fencing. Other hints remain unchanged.
G admits source6aa2280 after exact-source qualification, retained-capability review,
signing, complete12-asset public readback and14 actual Android immutable/stable
consumer commands. The earlier approval task's source final remains failed;
a distinct same-thread report-only turn closes the corrected native-final path
without repeating its reviewer/program/effect. Earlier public generations keep
their own evidence. Package selection and operating activation are separate actions.

### Optional package selection

Public Core/Full90 and later expose these commands without Manager or enhanced
TUI. This device has been explicitly updated to Full93; other installations must
first update Core to a compatible signed generation.

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

Install verifies an independent eight-file Android payload and its detached
Ed25519 manifest using Core's admitted update authority. It stages privately,
checks every hash/size/mode and probes the executable without model work before
selecting an immutable generation. Status reports selection, not app/login/model
readiness. Explicit stable install/update authenticate the independent signed
Provider index before package acquisition; remote installation also accepts an
explicit immutable HTTPS directory. Downloads are bounded, do not retry effects
and never start during ordinary Core launch/update. Flat remote license assets
map back to the signed inventory. Failed admission preserves the selected generation. A running payload
lease or immutable executable blocks selection changes; the installer never stops
jobs. Rollback retains the sequence high water and the previous admitted verifier
key, including across later authority changes. Neither command deletes older
payloads, downloads a desktop/rootfs, logs in or submits model work.

Selection does not rewrite existing generated profiles. Explicit `prepare` binds
the selected immutable executable to the generated native profile; existing native
config/auth/model defaults remain protected. Retain profile-pinned generations
through H. Managed work supplies native routing directly without republishing
shared or per-home profiles; its catalog snapshots stay in its own state.
Shared `work`, `chats`, `host`, `config`, `serve` and other Provider
arguments pass through `codex provider chatgpt` to that executable. The original
direct binary remains usable for pinned profiles and owned services.

Release producers use `codex-release-builder provider-package` with explicit
binary/assets/source-input inventory, exact base commit, generation/sequence,
private Ed25519 key, OpenSSL and absent output directory. The signed inventory
contains the binary, Cargo.lock, NOTICE, four licenses and source-inputs.json;
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
| Codex development | Native Codex -> Linux app API -> managed work Chat -> MCP/local tools -> same native work |
| ChatGPT direct development | Current Chat -> MCP -> native work; current Chat supplies intelligence |
| ChatGPT general development | Current Chat -> MCP request/return -> the SAME managed Codex path, including its app/session management |

MCP/local selects the inner tool return for an app-managed work Chat. General
development still enters through MCP; ordinary direct ChatGPT local injection is
not an implemented equivalent. The Linux app is retained for useful authenticated
session reuse/replacement. Replacing it with browser/DOM is not the next objective.
Legacy service-wide `execution.mode` alone does not select the combined routing;
the shared work interface binds the selected reasoning source per work.
Stable native work, one current controller, related Chat reuse, grounded handoff,
original question/result return and model-free status are integration boundaries.

The [ChatGPT-operated Codex workflow](../../CURRENT.md#proposed-chatgpt-control-through-mcp)
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
[CURRENT](../../CURRENT.md#remaining-acceptance-and-delivery); a universal quality,
latency, billing or capacity advantage remains unproven.

E/F design/integration, G product/public delivery and H preservation/retirement
are complete at their [recorded scopes](../../CURRENT.md#product-completion-scope-and-work-order).
Original failed/interrupted cohorts and exact comparison inputs remain preserved.
Detailed context-capacity policy, optional SIWC/app-free research and viewer/setup
polish stay separate follow-ups. Linux-app removal is not a prerequisite.

## Opt-in operator controls

The signed Provider exposes common `work`, `chats` and `host` CLI commands over
the same authenticated MCP service. `host status` observes service
and app state without acquiring a missing host. App/source controls can run
without a native execution binding; native work requires the declared
Core/home/workspace. `chats list` supports project/title/time filters and an
account/filter-bound `list_cursor`; `chats read` returns stable IDs, parents,
source cursors and explicit unavailable content. Range enumeration (`complete`)
and readable content (`content_complete`) are separate.

New native work may select `reasoning=current` (origin Chat/MCP) or
`reasoning=managed` (saved project Chat, explicit return path). Related starts
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
selected hint application are recorded in the bounded F final review; G owns
remaining product qualification, activation and public delivery.

The qualified source candidate waits on local owned-stream state during an
active input. Completion still requires the saved original message. A failed
retrieval returns `complete=false` with `observation_unavailable`; it does not
cancel or resend development. HTTP429 starts a shared account cooldown of at
least60s, increasing up to300s on repeated limits and honoring `Retry-After`.
`host status` exposes static read/backoff diagnostics. Cooldown observations make
no saved-Chat HTTP request. Bounded actual Android pacing proof is recorded in
CURRENT; it does not establish whole-F acceptance.

The candidate adds `execution.mode="operator"` with absolute `execution.core`,
`execution.home` and `execution.workspace`. `transport.return_path="mcp"` exposes
the controls through the Chat-side connector; `"local_function"` uses the same
Operator Host through app local functions without that connector or tunnel. D2
implements this local binding and compares all four control/transport combinations
from Codex. All four matched six-task real-development/handoff runs pass; detailed timing,
request counts, independent audits and retained failures are in CURRENT. ChatGPT-first
local discovery is deferred. The default remains `model_adapter`; binding/transport
changes need a service restart. The target direction does not make an unbound default
operator configuration valid or activate it on the device. Use a disposable
qualification home until G accepts the remaining host/product and delivery gates. `mcp-stdio` passes the private local bearer;
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

## Candidate configuration and commands

The source executable is `codex-chatgpt-provider`; these are candidate developer
commands, not commands promised by the current signed installation. Its settings
and catalog have one authority:
`$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, default
`~/.config/codex/providers/chatgpt.toml`. The installed JS adapter does not read it.
User-facing ChatGPT project setup/management is deferred; existing project
configuration/operations will remain its authority. Scope and priority are in
[CURRENT.md](../../CURRENT.md#chatgpt-projects-and-history).

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

Initial catalog data is in `default-models.toml`, not a Rust model-family list.
GPT-6 and manual-only Pro are retained; GPT-5.6 is excluded. A hidden Instant alias
is compatibility data. Model/effort/context entries are editable without rebuilding
unless the host protocol changes. Listing a route does not prove entitlement.

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
[CURRENT](../../CURRENT.md#remaining-acceptance-and-delivery).

## Service and observation

| Candidate command | Effect |
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

E's [app/host capability research](../../CURRENT.md#linux-app-and-host-capability-research)
covers authenticated Chat/project APIs, session reuse/input, process ownership,
connection/readiness/recovery, configuration and shared management surfaces.
F implements the selected common management interfaces; H preserves reusable
qualification helpers and retires duplicate launch policy. Viewer usability stays
a separate physical validation scope.

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
Provider, upstream commits its active-context replacement, then Provider retires
and rebuilds the Chat lane. The original durable native history remains. There is
no independently verified Chat remaining-context sensor. This coupled lifecycle
belongs to the model adapter; operator Chat renewal follows the
[same-thread incremental handoff design](../../CURRENT.md#same-thread-continuity-and-chat-context-handoff).

The existing adapter tracks warm-prefix deltas and native checkpoint acknowledgment.
Summary policy/results are transient until upstream commits the offered checkpoint;
identical summary text in unchanged old input is not acknowledgment. Public `ocx1:`
checkpoints do not decode or share OpenAI's foreign opaque reasoning format.
Native compaction and model/hash transitions remain upstream lifecycle operations.

`checkpoint --thread ID --from-model ID --model ID --effort EFFORT` reads a complete
idle public snapshot from stdin and emits an external-context message. It starts no
app/inference and writes no native history. TUI uses the upstream API to persist it,
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
codex-chatgpt-provider config set history.mode project history.project_name "Codex"
```

`history.project_id` disambiguates an exact owned private/unshared project;
`history.create_if_missing` controls creation. Preserve existing memory scope,
conversations and personalization. Duplicate/inaccessible destinations fail, and
uncertain create is reconciled before retry. Destination changes affect new lanes.
Project grouping reduces clutter, not stored history or established message costs.
Archive/delete/temporary retention is not admitted. Full contracts are in
[SPEC](../../SPEC.md#approved-provider-configuration-context-and-remote-project-behavior).

## Verification and migration

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
[acceptance matrix](../../CURRENT.md#remaining-acceptance-and-delivery) owns open
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
