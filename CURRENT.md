# Operating state and product completion

Reviewed 2026-10-18. `rewrite/rust-core` owns implementation; `main` pins an accepted
exact source for publication. SPEC owns product behavior and trust boundaries,
README user instructions, this file current state and unfinished work. Superseded
audit narratives and plans remain in Git.

## Operating baseline

| Component | Current authority |
| --- | --- |
| Upstream | Qualified0.163.0, source83086188ae0614bd882818ff1aa23f3194962012; release/upstream.json |
| Installed Full | Actual activation-state selects local-relay-hosted-0-163-0-acf1bdb8f531-s86; upstream0.163.0; previous Full85. User confirms85→86 update succeeded; installed launcher and both generations have identical Core bytes |
| Earlier public Full proof | local-relay-hosted-0-163-0-54f926437ad5-s60; source54f926437ad5093bb11e8d53c3502b810e514071; published2026-10-17T12:42:39Z |
| Base used by Full60 | local-base-relay-hosted-0-163-0-b18cc2a2f5f4-s58, reused byte for byte; previous user Full57 retained |
| Earlier Full60 TUI proof | Source1c413cf, qualification39027733603; Full76 helper was a different signed inventory entry |
| Retained prior TUI candidate | Source310f0f7e5ce421e61e2c3158406f17e743584557, qualified39107948384; old binding invalidated by changed source recipe, artifact retained/not activated |
| Installed Chat adapter | Immutable c3642b19a87cdace158f; sourceBase9a5436a plus preserved user1m setting; JS/Bun/Python; app-local function returns |
| Rust Provider candidate | Recoverable Phase A source acf1bdb8f5316b0f0a0d5e6d4c0893aaa8455dbf; unsigned Android run39173039235 and disposable snapshot restart/rollback accepted; existing authenticated host/AI proof retained; not installed publicly |
| Experiment | experiment/web-termux at08e9856; user's uncommitted1m profile/test edits preserved, branch/worktree not closed |

Full60 acceptance39033120658, Core39033121990 and publication39033383447 passed.
Main pinb0702ba selects source54f; promotionb5e1d9f publishes Full60. The reusable
main caller retains its acceptedb18 input. The Base fingerprint is
43e0b888ec06af28e2e98d55cdd30adb9f01e5ef9d3e8f691cfab7c6e2a97a3c.
The pilot inventory was read against its actual runtime binding:526 immutable
files, zero mismatches. These observations do not activate the Rust candidate.
Source release/upstream.json now binds frontend15da596 via successful39129327503:
50 focused native tests,94 retained upstream-feature checks and lint passed.
The downloaded exact artifact/recipe was independently verified and an actual
Android no-Manager /model round trip passed. This does not activate a public
generation. Automatic source commits may invalidate obsolete bindings while
retaining published Base/Full and prior immutable artifacts.

The provisioned app remains26.1007.21434 with Chromium155.0.8059.40. The user
reestablished login after revocation/logout. Authenticated request construction
now passes against the expected private account with zero Chat submissions,
including after Electron creates auxiliary index-page targets. Read-only DOM
selection admits the sole main Chat/Work surface; multiple real main surfaces
still refuse. Its observed
app-initial-ed5807835d97.js/app-shared-2d77298f0100.js capabilities define this
bridge's qualified host boundary. App binaries, rootfs, login and account data are
not bundled into the repository or implied by a model listing.

## Documentation authority and review scope

The seven tracked Markdown documents were checked for the same-thread/context
design: AGENTS, SPEC, CURRENT, root README, TUI README, Provider README and NOTICE.
SPEC retains the current and opt-in implementation contracts; this file owns status,
one work sequence, outstanding acceptance matrix and bounded proof index. README
files describe their component and link here rather than carrying competing plans.
NOTICE owns provenance. AGENTS and the license/prompt fixtures remain unchanged.

SPEC separates the opt-in operator contract from current model-adapter policy.
The correction retains one primary Codex thread, puts incremental Chat handoff in
B–C, reopens the native integration choice and limits E's adapter-specific gates
to D's retained paths. Phase A recovery evidence remains exact-source evidence;
the unqualified B draft is not that baseline. AGENTS, NOTICE, licenses and prompt
fixtures need no change. The deferred Core update-display/prompt work remains low
priority for the next substantive Core change.

This review changes documentation only. No runtime requalification, B execution,
Chat submission, public release or device activation is claimed. Earlier
documentation-agenda-audit.json and documentation-audit-migration-example.log
retain their historical scope, including the then-observed Full76; they do not
describe today's installed generation. Completed fixtures/live proofs are not a
new backlog. Historical evidence remains in Git/private acceptance logs.

## Components and approved architecture

Every Termux Codex execution passes through Core. Minimum operation is upstream
plus Core; Manager, TUI and Provider are independent optional components, not a
mandatory launch chain.

| Component | Owns | Does not own |
| --- | --- | --- |
| Upstream | Inference/tool/reviewer engine, canonical transcript, threads, turns, compaction, writer locks | Signed generation lifecycle or Android window policy |
| Core | Qualified executable/environment/FDs, home/shared-layout/server binding, install/update/rollback/provenance | Account/window/quota/model policy or Provider transport |
| Manager | Profiles, quota/rotation, notifications/windows/title/tmux and task discovery/handoff policy | Another executor/history store or runtime pointer authority |
| TUI | Upstream rendering and optional /switch, permissions and /model integration | Provider transport/host supervisor or another model catalog |
| Provider | Rust config/catalog, Responses/local-function mapping, Chat account/CDP/host/lane lifetime | Native tool execution/reviewer, canonical history or Core runtime selection |

Core links no Manager/Provider implementation. `runtime-binding` is its small
execution-record contract consumed by optional tools. Core-only operation bypasses
optional preparation/config/TUI but retains execution-home/shared storage/runtime
validation. Per-launch effective settings and server FD34 snapshots are immutable;
optional fragments are admitted by the actual upstream loader before dispatch.
Malformed/hung optional components preserve Core defaults and confirmed home.
No fallback after handoff may replay work or change an explicitly selected account.

The current config-v1 execution record is the only codec. Unsupported records
permit no mutation; preserve their processes/data. Historical experimental codec
emulation and staged reader/writer migration were rejected. Core retains necessary
shared server/config isolation; removal of catalog namespaces is not required to
achieve optional-component independence.

Public naming is TUI. Existing frontend/manager-tui paths and signed markers retain
their spelling until a qualified migration. Catalog model/effort/context data
belongs to Provider configuration; neither Manager nor Core maintains its own list.

## Proposed ChatGPT control through MCP

Status: Phase B implementation and synthetic-host Android qualification authorized.
The partial implementation is paused while its design/documents are realigned:
the primary Codex thread persists; only the Chat execution context is replaced.
This review authorizes documentation work, not implementation resumption. C real
Chat trials, D default/fallback selection and E publication/operating activation
remain separate decisions/gates.

This proposal supersedes the earlier interpretation of dual entry as just a
ChatGPT-first bootstrap for the existing per-inference tool loop. The user wants
ChatGPT to operate local Codex through MCP: discover tools, inspect/manage work
and sessions, execute development, choose context/checkpoint/handoff operations,
publish public reports and finish the task. Codex is the local execution/recording
system; the Provider is its transport/control adapter, not a second planning agent.
The documentation review alone approved no implementation/default/activation change;
the subsequent user decision authorizes B. SPEC now includes the opt-in operator
contract. The model-adapter/local-function path remains the current default.

Two entrances converge on the same control workflow:

- Codex entrance: Provider applies selected model/effort, starts the Chat task and
  keeps the original native request connected to that task's output/completion.
- ChatGPT entrance: the user starts Chat directly; MCP opens/attaches local work.
  No recursive Chat initiation or second Codex model is required for primary work.
- Common path: ChatGPT controls local Codex through MCP, receives real execution
  results on original tool calls, emits reports and explicitly finishes. Native
  events/records remain available from either entrance; only a Codex-origin request
  has an original CLI response to settle. Another CLI may observe the same work.

### Origin and return routing without another Chat user turn

The user did not request an origin marker in the user prompt. Persist the binding
in the existing Provider journal: origin, a stable run/request identity, confirmed
execution home/account, native thread/turn associations, outstanding calls and the
local reply sink. These are proposed fields/semantics, not an implemented schema.
Keep the same primary thread and run identity across model-request rounds, Chat
conversation handoffs and restart; Chat renewal never forks the primary thread.
thread_id, response_id and JSON-RPC id alone are not the whole task identity.

If ChatGPT needs origin/control context, return a compact model-visible record on
an existing non-user return path: the original local-function return or MCP result,
preferably the already-needed attach/catalog response. This reuses the demonstrated
same-call return mechanism; the new origin/context payload itself is not qualified.
Do not append a user-role environment block, resend previous answers, or post a
follow-up user message merely to supply routing information. If context is needed
before a first return exists, establish that bootstrap explicitly; do not invent a
server push into a nonexistent tool call. Avoid a separate model round just for
origin when an existing result can carry it.

Responses-compatible wire format is our local adapter interface, not proof that
arbitrary Responses metadata reaches ChatGPT. MCP content/structuredContent is
model-visible; _meta is UI-only. Keep private paths, credentials and the actual
reply channel local. Report/final calls resolve through the saved binding, not a
model-selected URL or guessed destination. Model-visible does not mean absent
from Chat history, free of context cost, or uncharged by Chat message limits.
Reference: https://developers.openai.com/plugins/reference#tool-results

### Responsibility and observable behavior

| Owner | Proposed responsibility | Remains outside it |
| --- | --- | --- |
| ChatGPT | Development decisions, discovery, session/context strategy, public reports and completion requests through MCP | Fabricating execution receipts or declaring an unconfirmed stop |
| Provider | Model/effort initiation for Codex entry, bound MCP/API controls, exact-call correlation, return routing, event projection and host lifetime | A competing planner, shell executor, canonical history store or silent account selection |
| Upstream Codex | Actual tools, configured approvals/reviewer, writer ownership, authoritative native records and execution/control primitives | Chat transport and UI-entry routing policy |
| Core | Verified runtime/environment/server binding and generation lifecycle | New ChatGPT control policy or Provider dependencies |
| Optional Manager/TUI | Profile/window selection and displaying/controlling the bound native work | A second work owner or duplicate transcript authority |

Inventory the full requested Codex control surface before freezing MCP tools.
Expose supported native operations, including necessary context/session controls;
existing native_catalog/native_tool alone do not constitute this capability. A
missing general native-tool/control API may require a Provider/TUI integration or
small upstream adapter; do not move orchestration into Termux Core to bypass it.
The installed0.163 experimental schema includes command/exec, thread/shellCommand,
thread/inject_items, thread/compact/start and turn/interrupt. Schema existence is
not runtime qualification: command/exec has no thread, thread/shellCommand is a
user command with different policy, and history injection alone does not update
a live TUI. Prefer reuse of actual native execution and existing Responses/event
projection over arbitrary rollout-file edits or an unverified second executor.
Reference: https://learn.chatgpt.com/docs/app-server

The context/handoff policy below distinguishes Chat renewal from native inference
compaction. A control command targeting an active turn can require a deferred or
settled transition, not a recursive wait while that turn awaits its MCP result.
Independent review and child agents need their own real contexts/transports;
do not replace auto_review
with self-review or silently weaken the configured approval mode. If a reviewer
still uses the Linux app, that capability retains the dependency even when the
primary ChatGPT-first path does not.

Execution/output events must stream to an attached CLI and durable native history.
Public report/final MCP calls enter that same output path; an optional host-text
observer can add token-level streaming. MCP alone does not copy all Chat UI prose.
An actual command receipt is recorded by native execution, not retroactively
invented by ChatGPT. Completion resolves once only after outstanding calls settle.
Neither a report stored on disk nor an HTTP200 proves the user saw live progress.

Do not adopt blind continuation after observer loss. Show actual native activity,
last-observed host state/freshness, unknown versus idle, and stop requested versus
confirmed. MCP-only observation cannot establish whether ChatGPT is thinking or
idle between calls, nor guarantee its Stop button stops local work. Explicit Stop
must fence new actions and confirm the affected native turn/jobs. Resume reads the
latest history and unresolved original calls; it must not replay uncertain effects.
One active controller owns each work binding even if both UIs observe it. Separate
work can run concurrently; a second entrance cannot silently become a second writer.

### Same-thread continuity and Chat context handoff

The primary Codex thread keeps the long-term canonical execution/history record.
Chat conversations are replaceable reasoning contexts; use each for substantial
MCP development, then prepare its successor in that same native thread. Independent
children/reviewers retain their own native contexts; they are not replacements for
the primary thread. Origin, work identity and original CLI reply binding survive.

ChatGPT supplies the next summary from the previous accepted summary, confirmed
native history since its source boundary and current actual work state. Include
current instructions/goals, decisions, changed files, tests, remaining issues and
references for fetching details through MCP. Local code validates/persists through
supported upstream recording, retaining full original history. The Provider journal
keeps transport/controller state and references, not a second transcript/summary DB.
Reconcile against original evidence so repeated summaries do not silently lose work.

Bind handoff to a definite native-history boundary and controller generation. Fence
new effects from the old Chat, settle or reconcile pending calls/approvals/jobs,
record the prepared handoff, then confirm the successor's attachment/receipt before
granting control. Attachment rereads later instructions/events and unresolved calls;
an acknowledged summary does not cover new events automatically. A failed or
uncertain transition preserves the last recoverable boundary without executing
effects twice. Handoff may span native turns, but does not complete the user's
overall task or discard its original return binding.

Native inference compaction and Chat renewal are separate operations. Keep native
compaction only where the chosen execution integration needs it; native token/hash
triggers must not dictate Chat renewal. No independently verified host-capacity
sensor exists. Initial/cold history import is projection, not automatic model
summarization; the old30k/32k or preserved1m settings are not measured Chat limits.
Use task milestones, observed context growth and headroom rather than a fixed MCP
call count; call output sizes differ. Do not renew on every native transport round.

Before B implementation resumes, compare Responses dispatch reuse with a narrow
upstream tool/control integration. The first preserves existing native execution
but may retain unwanted inference-budget/compaction coupling. The second must
still preserve full native tools, approvals, history, live events and writer locks.
Select from concrete source/runtime evidence; no second executor or Core policy.
The partial native client/config/dependency edits are uncommitted draft work, not
an accepted API or recovery candidate; the first compile check has an unresolved
catalog-error conversion. Its `new_context`/native-compact handoff assumption is
withdrawn: neither operation proves successor Chat creation, summary transfer or
attachment. Automatic Chat creation/reattachment, bootstrap delivery, failure
recovery and any remaining app/bridge dependency require their own B/C proof.

### Timing, fallback and proposed revised order

Phase A has frozen a useful, recoverable Rust baseline. Qualify this alternative
before expanding orchestration that it may replace. Finishing every existing
optimization and publishing a final legacy-shaped Provider first risks duplicate
development; replacing the working path immediately
risks losing its broader native/AI-development proof. A full public release is not
a prerequisite for the new bounded experiment. Base/Full automation and unrelated
Core/Manager fixes remain independent and may ship with their own acceptance.

The single proposed implementation sequence and decision gates are in
[Product completion scope and work order](#product-completion-scope-and-work-order).
They do not change current defaults or authorize the new implementation by themselves.

A baseline checkpoint is not a demand to polish every old feature. Fix shared
correctness defects first. Defer new Provider-side planning/context heuristics,
transport-specific micro-optimization and broad cosmetic work until the alternative
comparison, unless required for the baseline. Do not defer required failure/stop
visibility, exact-call correctness or account/data preservation as cosmetics.

Retain the existing Rust model-adapter/local-function path as an explicit fallback
candidate while qualifying the new operator path. Existing MCP return is a
transport on that same adapter, not the new full-control workflow. Do not overload
transport.return_path to pretend execution policy and result transport are the
same setting; final naming/schema waits for design. Keep only deliberately
supported combinations, not a general cross-product routing framework.
Fallback retention is not a permanent promise to maintain two orchestration
implementations. Keep it only if it offers distinct useful coverage, such as work
without an available Chat-side MCP connector, at acceptable shared-test cost.
Remove obsolete path-specific policy after replacement proof and an explicit
product decision; keep common codecs/execution/history instead of duplicating them.

No automatic path switch after uncertain dispatch, active tools or an unresolved
writer. Choose before starting; change paths only with settled/reconciled state
and an explicit handoff. Rollback preserves checkpoints/history; it never replays
a task. Installed JS/Bun/Python remains the protected current installation and
migration reference, not a newly supported fallback implementation. Never silently
select it because the Rust/new MCP route fails.

### Required proof before choosing the new default

- Zero-Chat fixtures: origin/context travels through non-user returns; no extra
  user-role environment/previous-answer block. Origin and reply binding survive
  reconnect/handoff without exposing private return destinations.
- Actual Android: discover tools, run multiple command/patch/test rounds, replace
  Chat with a source-bound incremental handoff in the same primary Codex thread,
  retain original native history and stream public
  initial/intermediate/final reports to the original CLI. A no-observer Chat entry
  still records execution without creating a dummy CLI-origin request.
- Interrupt and disconnect during model wait, execution, approval, checkpoint and
  handoff. Verify observed stale/unknown state, the exact stopped work, remaining
  owned jobs, result reconciliation and no new effects after confirmed Stop.
- Two entry clients observe/control one task without duplicate writers; independent
  tasks/reviewers remain isolated. Prove restart, unchanged primary thread/return
  binding, summary receipt plus post-boundary events, and safe failed/double/stale
  controller attachment. Synthetic Chat replacement is B evidence; actual Chat
  creation/continuation is C evidence. Qualify native compaction/cross-provider
  return only where the selected path still needs it; R11 proves neither handoff.
- A matched real development task, including direct ~/prj/ai under its AGENTS and
  preserved user changes, must show analysis, actual failure/correction, tests and
  same-task resume. Supervisor evaluates the real diff/tests and physical CLI
  transcript, not just Provider delivery logs. Existing AI proof is retained as
  baseline evidence, not reassigned to the new workflow.
  Compare repeated fixtures with equivalent inputs for latency/call overhead;
  use sequential legitimate AI changes for real development, not simultaneous
  writers or repeating an already-completed patch. Do not infer efficiency from
  the unmatched earlier AI-development and two-command MCP trials.
- Reuse prior acceptance only at its exact scope. Non-user tool returns are proved;
  general task controls, new origin payload, automatic real-time transcript and
  charged Chat message savings are not. No Pro/GPT-5.6 trial or silent retry.

Phase B has a paused partial implementation. Resume with the integration choice
above, then qualify the common control path on actual Android with a synthetic
host. No B completion, real Chat trial, new default, automatic fallback or runtime
activation is yet claimed.

## Product completion scope and work order

This is the single work sequence. A's bounded baseline is accepted. B is authorized
with partial implementation paused for design alignment; C real Chat trials require
their declared scope/submission budget, and D default/fallback selection remains
pending. E retains final product acceptance; independent Core/Manager/Base/Full
work need not wait for the Provider experiment.

| Phase | Work | Gate |
| --- | --- | --- |
| A | Stabilize shared Rust identity/auth, exact-call returns, native execution/history, visible failure/stop/recovery, config and candidate packaging | Bounded recoverable checkpoint accepted at acf1bdb, with exact source/artifact/proof and disposable standalone snapshot install/restart/rollback. Remaining product/lifecycle gates stay in E; no public activation |
| B | Choose native integration, expose Codex control to ChatGPT and connect both entrances | Authorized; draft paused. Actual Android synthetic host: non-user bootstrap, full native tools, live public reports, repeated MCP work and same-thread incremental Chat handoff, failure/restart/stop reconciliation and original final routing |
| C | Real ChatGPT control, conversation renewal and comparison | Explain trial scope/expected submissions first, including summary and successor Chat starts; real automatic creation/attachment/receipt and continued work on the same native thread; matched fixtures and direct AI development, physical CLI progress/stop/recovery and independent supervisor review |
| D | Select default and useful fallback | Compare quality, continuity, latency, dependencies and maintenance. Existing Rust local functions stay the current default until qualified replacement; no uncertain mid-task fallback |
| E | Finish shared product, publish and activate | Close applicable matrix gates for D's retained paths/components, without expanding replaced adapter policies; independent composition, package/provisioning/install/update/rollback, signing/readback/disposable update/non-forced promotion. Device activation is separate |
| F | Close superseded experiment and code | Preserve unique work/user1m edits/licenses/proof and admitted rollback generations; remove replaced implementation policy only after accepted replacement |

Completed implementation is not a new backlog: Core independence/config isolation,
validated Provider config/catalog, one-line /switch rows, source same-provider model
selection, native execution/context fixtures, configured-project operations and
bounded AI/MCP live proofs are retained below. Full publication and operating
Provider migration remain separate. Detailed open qualification belongs only to
the acceptance matrix; the proof index records completed evidence at its real scope.

Phase A's shared correction closes a false service-stop acknowledgment: after
SIGTERM, losing the recorded executable path no longer counts as process exit.
Only the pinned kernel handle confirms shutdown; a surviving owner returns bounded
`provider_stop_unconfirmed` and preserves its journal. The actual Android regression
failed on the prior implementation and passes on the repaired source and downloaded
NDK executable. This is service shutdown, not confirmation that native tool jobs or
Chat work stopped. Existing exact-call/history/recovery proof retains its old scope.

The standalone candidate now has one create-new package/readback helper, including
source-bound lock/notices/licenses and a complete seven-file inventory. Hosted
qualification39173039235 passes at exact sourceacf1bdb; executable SHA-256 is
bd6e6068364791f02fad5bc53637e7dfc50b0609d87848403e17da4e5211a32c. The exact
artifact and proof are retained privately beyond the30-day hosted retention.
Actual Android c463→acf1bdb→restart→c463 snapshot transitions preserve the owner,
endpoint, disposable auth/config/app data and rollback profile while rotating local
credentials and rejecting a second writer. A saved-port collision preserves the
record. This qualifies bounded standalone candidate recovery, not a signed product
installer, real host-loss/reboot, physical UI or long-running task recovery.

Local `full` acceptance passes358 Rust and135 Python checks; Provider passes98 Rust,
108 Bun and35 Python checks, fmt/clippy/release. Source contract39173039249 and both
hosted Provider jobs pass. Unarmed live/physical gates remain in the matrix below.
Added qualification uses disposable roots and a synthetic host. The candidate
lifecycle trial made zero host requests/Chat submissions; there was no public
signing or operating activation. The later user decision authorizes B independently
of this baseline; C–E retain their separate trial/default/delivery gates.

This design correction does not invalidate Phase A's exact-source service-stop,
identity, correlation or candidate-recovery proof. Preserve acf1bdb as the recovery
baseline instead of folding the unfinished B draft into it. Those passing gates
are not evidence for B's same-thread incremental handoff or changed dependency
closure. Requalify affected inputs when B changes them. Do not expand old adapter
context/rotation policy simply to finish A; keep shared correctness and defer
path-specific hardening to D's retained scope. No Phase A implementation change is
required by this documentation correction.

The user's required AI trial was run directly in ~/prj/ai, not a copied project or
worktree, after the Rust no-MCP execution/context/project path was established.
The model performed the actual source work. Future trials must likewise inspect
AGENTS/Git and preserve uncommitted/untracked content before mutation; Git history
alone does not preserve it. Supervisor review and applicable tests remain mandatory.
Do not substitute the old adapter or a supervisor-made patch for model development.

Complete independent source work while hosted checks run; admit releases in their
dependency order. Do not stop after each individual file change or invent a new
planner/checkpoint hierarchy. Existing user permission and verified remote ownership
remain valid for authorized baseline and Phase B work. They do not authorize
undeclared real Chat trials, a new default or operating activation.

### Deferred Core follow-up

User-requested, low-priority work to bundle with the next substantive Core change;
do not implement it now or make it a prerequisite for the Provider work order.
The85→86 update succeeded, but the user observed `unknown` in the right-hand
release date/other details. Core bytes were unchanged across those releases.

- Improve update information so upstream Codex, Core, Manager and applicable TUI
  each have identifiable installed/target versions or source revisions and release
  dates. Make unchanged or absent optional components and unavailable metadata
  understandable. Diagnose the missing display data; use authenticated metadata
  and its actual date meaning, never infer a release date from installation/file
  time or imply every component changed because the Full sequence increased.
  Choose a concise, consistent presentation for explicit update and startup prompt
  when implementing; exact layout and metadata mechanics remain design choices.
- On every eligible all-TTY, zero-argument bare `codex` launch with a verified
  available update, show the useful current→target component information and ask
  whether to update. A prior decline/timeout must not hide that same update on the
  next eligible launch. Missing optional component details must not hide a verified
  update or prevent its prompt. Keep discovery/display acquisition bounded,
  continue normal launch on decline, timeout or failed discovery, and retain
  rollback/trust guards.
- Preserve all upstream commands/options and raw execution behavior. Any arguments
  or any non-terminal standard stream bypass startup discovery, prompts and
  advisory writes entirely; preserve stdin, argv, streams, exit status and signals.
  No optional Manager/Provider dependency or unattended activation is introduced.

Before implementation, amend SPEC's current display-acquisition and six-hour
generation-snooze rules. This records future work, not changed shipped/default
behavior. Verification belongs to the eventual affected Core change.

## Context and easy configuration

The Rust authority is $XDG_CONFIG_HOME/codex/providers/chatgpt.toml, default
~/.config/codex/providers/chatgpt.toml. It owns settings and catalog.models.
Initial routes are default-models.toml data, not hardcoded Rust model families.
GPT-5.6 is excluded; retain GPT-6, manual-only Pro and hidden Instant alias.
Listing/editing a model is not entitlement or execution proof.

The candidate CLI supports models, catalog, config show/effective/validate/set/
unset/edit and prepare --home PATH. Related fields commit atomically; cooperating
writers serialize, stale editors refuse overwrite, invalid/no-op edits preserve
original content. Effective output reports file/default/per-model origins and
runtime_application=not_queried. The installed JS adapter does not consume this file.

Provider reads host routes/history config at request admission. Catalog snapshots
and generated upstream startup settings remain immutable: prepare again for the
next startup rather than editing a bound server. Per-model context overrides file
settings; explicit native invocation overrides win. /model is primary; generated
upstream --profile chatgpt is the fallback. Manager is not required. No bearer is
stored in the profile. `token`/prepare may lazily start the same Rust executable
against a provisioned rootfs; concurrent startup serializes. `status` is read-only.
A failed readiness check never authorizes restarting an existing live owner.

The user1,000,000-token setting is preserved policy, not measured Chat capacity or
a qualified ceiling. Do not impose an arbitrary32k minimum. Native token triggers,
model/hash compaction, HTTP byte bounds and true host capacity differ. An unset
explicit threshold delegates to upstream, not an all-compaction disable switch.

The following describes the existing model adapter, not B's Chat-renewal policy.
Cold admission projects/imports public native history; a warm lane sends needed
deltas/results. Neither step necessarily asks a model to summarize. Native Codex
requests inference-context compaction on its own lifecycle triggers, asks its
configured model endpoint for a summary and commits the accepted replacement to
active inference history while retaining the original durable rollout. The Chat
Provider supplies that summary through ChatGPT and retires/rebuilds its old lane
only after native acknowledgment. This couples native compaction to Chat renewal;
it is not an independent measurement of actual Chat remaining context. B replaces
that coupling with the [same-thread handoff policy](#same-thread-continuity-and-chat-context-handoff).

Responses Lite declarations are transport metadata, not canonical conversation
items; compaction may remove/reinsert them without changing the accepted prefix.
Compaction is recognized from actual request metadata/explicit V2 triggers, never
user prose. Summary-only policy and replies stay transient until upstream commits
the exact offered checkpoint. Retire a settled lane once after acknowledgment;
identical summary text in an unchanged old prefix is not acknowledgment. Reassert
normal native policy after control operations. V2 ocx1 encodes public readable
summary, not private reasoning and not decrypted foreign OpenAI context.

Source `checkpoint` serializes a complete idle public native snapshot as readable
external user context, with no credentials, app startup or inference. Upstream
persists it with its existing inject API, preserving original items/thread ID and
the old provider's ability to read them after an entry failure. Provider admission
accepts only an explicit handoff in the same thread or native-declared fork/parent
lineage before foreign opaque input can be
projected from public history; current policy/context settings and later input stay.
Unknown public item types refuse; reasoning/internal metadata are excluded.
The catalog no longer advertises a fabricated opaque compaction compatibility hash;
token/window compaction remains active. Source TUI waits asynchronously for the
source Chat Provider's readable summary before reverse entry, retaining the source
on failure/stop/event loss. These source changes are not an activated composition.
The real summary trial used two app model starts in33.9s, observed four user-role
objects, no tool return/Pro/retry and unknown Chat allowance debits. Native provider
responses in that trial were local fixtures; actual Chat inference and both native
process re-entries were real. It preserved the exact continuation marker, three
explicit native user messages, original public history and foreign opaque input.
One separate compaction turn is not another explicit native user request. The
current native user/context journal remains upstream's authority.

Public output includes initial/intermediate/final reports and actual native tools.
The app's final channel alone is not a final answer: finished_partial_completion
with end_turn=true can precede a tool. Stream once and promote an open item only
when successful end-of-turn is established. response.failed preserves native error
reasons; static last_error records contain no request/credential text. Known MemoryCite tags and box containers now have a stream-safe Markdown
projection, preserving useful text, optional titles, unknown markup and literal
code. Every UTF-8 prefix and native warm/cache/tool-result paths pass without
extra app submissions. Raw app/native tool data is not rewritten. Nested JSON
argument overhead and actual app sampling of markup remain separate work.

## ChatGPT projects and history

Retain history by default. Project mode uses history.mode, history.project_name,
optional exact history.project_id and history.create_if_missing (true by default).
This is a remote ChatGPT project, not CWD, Manager profile or API project.
The bridge uses the installed app's authenticated list/read/create operations;
no account bearer is exported. Current admitted destinations are owned, private, unshared
projects; their existing memory scope is preserved. Shared destinations refuse.

Bindings include account/user/workspace and exact project ID. Reuse an unambiguous
name, or select its configured ID. Duplicate/deleted/inaccessible destinations do
not create a replacement or silently fall back to root chats. Serialize creation;
record intent before dispatch. Reconcile uncertain delivery before another create;
a definite rejection is separate. Use app defaults rather than nullable memory/
sharing options. Destination changes affect new lanes; model/effort changes reuse
settled conversations. Do not move/delete existing chats or create one per tool.

Actual metadata create/reuse/readback spent zero Chat submissions. The native
project trial verified the declared ID before streams and read back persisted
conversation membership after inference. Project instructions/files/memory still
require priority and sibling/guardian isolation proof; do not change personalization,
sharing, native reviewer or MCP permissions silently. Grouping reduces clutter,
not stored history or established allowance units. Temporary/archive/delete modes
are not admitted and must never mass-delete history.

## Confirmed operating Chat blocker

Installed Full60 long native→Chat transitions failed400 unsupported_desktop_route
because ModelDownshift compaction sent the old gpt-6.1-sol model to the Chat
endpoint. Two requests were rejected before any Chat submission. The retained
conversation also has an unreadable upstream opaque checkpoint. An alias rename
or the1m setting does not repair either boundary. Keep the conversation.

Source Rust manual compaction and fresh native Chat now have separate real proofs;
actual Android TUI transfer now passes against an unsigned candidate in a signed
disposable composition, but final hosted artifact admission is unfinished. Obtain an upstream-owned readable
handoff at a settled boundary before routing it to another Provider; never decode,
drop or relabel opaque state or replay tools. Evidence: chat-full60-operating-
diagnosis.json and chat-full60-cross-provider-admission.json in the acceptance root.
Fresh startup is not blanket proof of all long conversation transitions.

## Verification and publication

Private acceptance root: ~/.cache/codex/acceptance/development-audit-1t4txdwt.
Logs/state there may contain private trial transcripts/account bindings; do not
publish them wholesale. Repository records contain only safe scope/results.

The index below retains useful latest/negative proof, not a chronological backlog.
Older source-run counts and superseded candidates stay in Git and the private
acceptance root. A synthetic host exercises native product code but does not
qualify actual Chat judgment, physical UI or charged Chat message allowance.

| Proof | Scope |
| --- | --- |
| core-settings-android.json |57 signed Android steps: actual config RPC/FD34 normal→Core-only→normal with live sibling, optional malformed/hung settings and TUI without Manager; no Chat |
| tui-admission-39129327503.log / provider-tui-accepted-no-manager.json / provider-tui-no-manager-current-core.json | Exact TUI source15da596, 50 focused+94 retained upstream tests/lint and binary8d84f1dd; actual Android same-thread native↔Chat /model, permissions, real checkpoint encoding and four synthetic Responses with one source summary. Current Core e83083f6 built explicitly, Manager absent; no Chat or operating activation |
| provider-native-question.log / provider-context-seven-android.log / provider-bidirectional-handoff-native.log | Actual native manual/automatic/interrupted compact, public handoff, API re-entry, fork/nested child warm/cold resume and question→actual answer on original call. Foreign opaque items preserved; synthetic Chat replies, no inference at injection |
| provider-handoff-live-native.log / provider-handoff-live-state | Real GPT-6 response and readable summary across native API provider re-entry, two app starts/four user objects; source responses synthetic. Not physical real-Chat TUI transition proof |
| provider-native-mcp-discovery-fixed.log / provider-native-correction-regressions.log | Actual native deferred search and exact canonical tool names, MCP effect, rejected-gateway correction, commands/patch/tests/final. No partial rejected-batch effects/user resubmission; synthetic host, no Chat |
| provider-native-reviewer-composed.log / provider-reviewer-project-isolation.log | Independent native guardian lanes, schema-valid allow/deny and actual allowed/prevented command effects; reviewers outside primary project with supplied context only. Synthetic replies, not real Chat reviewer judgment |
| provider-explicit-stop-android.log / provider-viewer-owned-android-final.log / provider-auth-bootstrap-android.log | PID-bound service shutdown/refusal of forged identity, same-endpoint restart, concurrent startup coalescing, owned fresh-profile HTTP/WebSocket/private RFB and capability revocation. No existing auth copy or Chat; physical fresh-profile viewer remains separate |
| provider-terminal-identity-native.log / provider-native-failure-reason-final.log | Account change before publishing tools/final refuses; restored identity reconciles original request without resubmit. Exact Provider failure reaches native UI. Synthetic host/no Chat |
| provider-sse-idle-regression-before-corrected.log / provider-sse-idle-android-fixed.log | Comment-only heartbeat fails actual native2.5s idle deadline; content-free data events pass four six-second waits/three effects without extra model/transcript items. Synthetic host/no Chat |
| provider-mcp-live-qualified.json / provider-mcp-real-native-v11-result.json | R11 at c463c69: actual signed Chat app→dedicated Rust tunnel→Android two exact commands/effect+verification→original MCP results→exact native final and public reports. One user object, two app dispatches including actual delegated Allow reply, two settled logical calls, no result resubmission/pending/replay; Chat debits unknown |
| provider-discovery-metadata-proof.json / provider-discovery-metadata-full.log / provider-discovery-metadata-android.log | e11162c source: neutral discovery/side-effect hints and correct native execution ownership; exact-call/capability checks retained. Rust97/Bun108/Python30, fmt/clippy/release plus14 actual Android MCP/executor/search/correction/reviewer/wait/context fixtures pass. Synthetic host, zero Chat; new descriptions' physical host-warning effect unqualified |
| provider-c463-android-smoke.json / provider-qualified-settled-release.log | Hosted39163366831 exact source/Android jobs; API24/AArch64 inventory/lock/source matches and actual device models/catalog/config validation makes no state writes/Chat starts. Unsigned/unactivated. Trial service/tunnel/proxy and settled hooks stopped/released, chats/auth/Tdev preserved |
| phase-a-baseline-proof.json / phase-a-candidate-lifecycle.json / phase-a-core-full.log / phase-a-provider-full.log | Sourceacf1bdb, hosted39173039235 and source contract39173039249; complete seven-file source-bound readback, actual Android standalone snapshot install/replacement/restart/rollback, exclusive owner/endpoint/credential and preservation checks, downloaded-binary unconfirmed-stop regression. Full358 Rust/135 Python; Provider98 Rust/108 Bun/35 Python; fmt/clippy/release pass. Zero Chat/host requests; unsigned/unactivated |
| provider-project-discovery-android.log / provider-project-create-minimal-body.log / provider-live-project-native.log | Actual authenticated project discovery/create/reuse/readback without inference; separately actual project-bound GPT-6 with three native effects, four app dispatches/two user objects/three returns and saved membership |
| ai-development-proof.json | Direct ~/prj/ai actual development, failure/repair, same-thread recovery and independent49-check/166-TUI validation; detailed scope below. Installed AI runtime unchanged |
| provider-owned-auth-native.log / owned-auth-transition.json / provider-restored-auth-readonly.log | Actual authenticated Rust-owned app/private pipes/native tools, four starts/three effects; original profile inode and controller restored. Subsequent readonly identity preflight makes no inference |
| installed-full76-inventory-readback.json / installed-payload-readback.json | Full76 active/Full60 previous; complete declared payload readback at recorded time. Inventory equality is not a fresh signature check or Rust Provider activation |
| desktop-panel-icon-taps.json / provider-font-physical-proof.json | Six actual X pointer minimize/restore actions, official installed app/browser icons and Korean glyphs observed. Pointer timing is not physical Android touch acceptance; qualification helpers are not Provider runtime dependencies |

Failed TUI728cf26 compilation and ff2682d lint candidates are not admitted by the
later15da596 success. Historical310f0f7 proof cannot authorize changed inputs.
Earlier MCP tunnel401, target selection, approval, private-content and retained-
capability failures remain private diagnosis; R11 supersedes them only within its
actual bounded scope. Runtime-key references, exact real tunnel child argv, owned
immutable service snapshot and renderer bridge revision were checked before submit.
Readiness, schema discovery and construction alone are not connector execution ACK.
Ignored Android/live gates count only when armed and actually run; failed/zero-test
required gates remain open.

### Direct AI development acceptance

Baseline humtr/ai main1f958e5 was clean; its full verifier passed48 checks before
model edits. GPT-6 High independently found that session filters following a
reference were ignored, wrote a failing regression and observed its failure. The
first attempt stopped on the Provider's partial-final phase defect before its
implementation patch reached native execution. That failure and state were kept.

After the Provider repair, the same native thread resumed, read current Git/test
state, patched lib/ai_resource.py, passed the new regression/full verifier and
reviewed its diff. The supervisor did not implement the AI patch. Independent
review reproduced failure on baseline code in memory and success on the model's
patch, then independently ran all49 checks/166 TUI cases, zero warnings/failures.
AI commit43abdc8 is pushed to verified owned humtr/ai; installed AI runtime unchanged.

There were26 app dispatches across two explicit user tasks,24 tool returns, no Pro
or automatic replay. First attempt had2 user objects (one visible). Cold resume
imported history and reported29 user objects, not29 new visible user requests.
Warm returns and cold history import differ; neither count establishes allowance
deductions. Initial/intermediate/final reports and actual commands/patches/tests
were observed. This is one bounded maintenance proof, not hours-long/reboot/all-tool
qualification. Unsupported box components appeared in that historical trial transcript; later
stream-safe formatting fixtures address known boxes but are not a rerun of it.

### Remaining acceptance and delivery

This matrix is the outstanding qualification authority, not a second work order.
Use the A–F sequence above. Actual Chat trials need a declared scope/expected
submissions first. Do not turn completed fixtures/live proof into mandatory reruns
without changed inputs, failures or an unresolved boundary.
E applies to the paths/components retained at D; replacement may retire an old
path-specific gate with its feature. Shared safety, supported upstream behavior,
protected state and signed lifecycle gates cannot be retired as path optimization.

| Area | Evidence already retained | Open gate and timing |
| --- | --- | --- |
| Host lifecycle/visibility | Authenticated Rust boot/native effects, synthetic concurrent lanes/sibling stop, readonly journal status, Phase A exact-owner/restart and kernel-confirmed service-stop proof | E: real host-loss/cancellation/concurrency/reboot and fresh login/provisioning. Journal liveness is not current model progress; service exit is not native-job stop confirmation |
| Context and TUI | Seven actual Android synthetic context cases, exact admitted no-Manager PTY /model round trip, separate real Chat API summary/re-entry | B: same-thread incremental Chat handoff, history boundary/current instructions, old-controller fencing, pending-result/failure/restart reconciliation and original CLI return. C: actual successor Chat creation/receipt and continued development. E: real-Chat TUI/downshift/cold-resume transitions for retained model-adapter routes |
| Tools/approval/question | Actual native discovery/correction, independent reviewer allow/deny effects and project isolation, blocking question result on original call | E: broader function/custom/batch and host search behavior, real reviewer judgment/project instruction priority, physical question UI. Quoted executable prefix matching remains an upstream limitation |
| Existing MCP return | Actual R11 effect/result/report/final; e11162c neutral description/annotation source+Android gate; Phase A retains endpoint/owner controls and exact-call regressions | E: new-description physical warning behavior remains unverified. B–C: full Codex control and two-entry origin/report/finish are new gates, not R11 features |
| Cost/presentation | Stream-safe MemoryCite/box projection, exact native results and warm prefixes | Count title-model starts, setup/summary/retries and source-specific overhead; defer path-only optimization until C–D unless shared correctness. No claim that counters measure Chat message debits |
| Core update information/prompt | User-confirmed85→86 success; Core bytes unchanged; release-date/other display details observed as unknown | Low priority with the next substantive Core change: [deferred Core follow-up](#deferred-core-follow-up), including component information and a prompt on each eligible bare launch; preserve upstream/noninteractive behavior |
| Viewer and desktop | Rust private RFB/HTTP/control fixtures; actual owned fresh-profile RFB; old desktop font/icon observations | E when shipped: physical current Rust viewer/keyboard/scroll/login. Old desktop observations do not qualify a changed viewer; viewer stays optional for inference |
| Packaging/composition | Independent hosted Android Provider build, source-bound complete inventory and Phase A disposable standalone snapshot lifecycle; TUI/Provider without Manager on actual Android | E: signed optional payload/provisioning and product installer lifecycle, Base alone, Manager without TUI, direct --profile and Full /model, absent/incompatible components, install/reinstall/update/rollback. No implicit desktop download/login |
| Experiment retirement/publication | Source/provenance and user1m changes preserved; existing Base/Full release process | E: build/smoke/sign/readback/disposable consumer/non-forced promotion then separate activation. F: remove superseded code only after unique-source/license/user-state review; no legacy runtime auto-fallback |

Native prefix-rule matching misses double-quoted executable paths in the reviewed
upstream shell-command/src/bash.rs command_name parser. The allow/deny fixture uses
a plain owned program path; do not generalize it to quoted paths or rewrite model
commands in Provider to conceal that limitation.

R11 used the independent Termux Codex Provider connector/dedicated tunnel. Tdev's
key-file reference was reused; tunnel IDs/clients/targets remained distinct and
Tdev was unchanged. Process-only DNS relay preserved end-to-end TLS and resolver
files. Readiness metadata can diverge from an already-running tunnel child:
reconnect an owned changed alias only after settlement and inspect actual argv.
Trial service/tunnel/proxy and settled hooks were stopped/released; GUI/login,
native journals and saved conversations remain. No permanent migration followed.

Base/Full automation retains independent signed heads, immutable GitHub Releases,
exact Base references and latest-successful-Base repair target. A successful new
Base remains independently usable when Full fails; repairing the add-on targets
that latest successful Base, not the previous Full's older Base. Keep current/
previous and referenced bases. Core owns signed edition/date/base display without
Manager. Source integration, public publication and device activation differ.

Published Manager window/notification/title/tmux and Full60 quota-switch proofs
retain their original scopes: one-tap notifications never launch/resume Codex;
stock APK uses one registered dedicated window; live bare↔tmux migration/nesting
is not supported. Rotation defaults off and waits for settled tools/writer state;
Chat limits are not a Codex-account rotation target. The new composition must
preserve these features. Existing physical observations are not new device proof.
