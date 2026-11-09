# Native sessions, model transport, and ChatGPT Bridge contracts

**Status:** Normative detail of [SPEC.md](../../SPEC.md).
The platform's Native runtime owns execution, approvals, history and writer
authority. This document defines feature-specific interactions and acceptance
criteria; observed implementation status belongs to
[CURRENT.md](../../CURRENT.md).

## Bridge lifecycle and interoperability

### ChatGPT-Codex bridge architecture and work boundaries

Sections4.5–4.6 define the target ownership and user-flow contracts. Provider
adapts source/transport and external admission; Native owns the conversation,
effective route/context, execution and committed outcome. Existing work-control
records are implementation evidence to review, not an independent task authority.
Core provides runtime trust/lifecycle; frontend and optional Manager consume
Native state and submit intent. No second executor, transcript database, scheduler
or planning model is introduced. CURRENT distinguishes this design from shipped
behavior and names incomplete interfaces. Historical audits do not freeze the
earlier boundary allocation.

Historical architecture reviews inform, but do not determine, the current
contract; [AUDIT.md](../../AUDIT.md) records past alternatives. Earlier
initiative labels are traceability references, not product completion levels.
[CURRENT.md](../../CURRENT.md) owns implemented, qualified, published and
activated status. The following
contracts include the delivered bind/unbind/join foundation and the broader
entrance-equivalence target; API delivery alone does not qualify every target
scenario. See CURRENT's acceptance matrix and work order for the remaining scope.

#### Work recording scope and entry selection

The workflow names are fixed as **standalone — 단독 개발** and
**integrated — 통합 개발**. Standalone means ChatGPT standalone development;
integrated means the common task/tool/control/history/orchestration workflow.
Entrypoint, model and inference initiation do not redefine these modes.
Provider remains the component name, not a workflow mode. **MCP** and
**Function** are tool-delivery routes, not workflow modes. Function is the
user-facing name of the existing `local_function` transport through AppHost;
`local_function` remains its internal protocol identifier. Neither transport
selects standalone or integrated, creates a Native work by itself, or changes
resource ownership. Do not retain direct/provider or current/managed as workflow
aliases.
CURRENT records implementation and qualification; this contract does not claim
that the installed MCP already exposes a threadless local-tool surface.

- ChatGPT local tools return actual file/command results to the initiating Chat
  without creating Native threads/turns or copying user/assistant messages into
  Codex conversation history. Reusing the installed Core executable does not
  require a Native conversation, a model catalog or Codex model authentication;
  that API fact alone does not qualify a backend as standalone development.
- Integrated workflow owns all participation in Codex tasks, Native tools and
  orchestration: common instructions, work/turn admission, permissions,
  execution/control, public history, CLI projection and completion. A ChatGPT
  response attaching to existing Codex work uses integrated workflow too.
- Codex CLI entry and ChatGPT entry into integrated mode use that same common
  work admission/execution/event/result lifecycle. Differences in incoming
  transport and final display destination do not create a second development
  workflow. Standalone never serves as a bypass into Native work/tools.
- A new unbound ChatGPT task defaults to local tools. A Codex-origin input or a
  verified selection of an existing Codex task retains its Native work. One
  initial selection persists across ordinary follow-ups; repository paths,
  titles, incidental mentions and a global last-work selection cannot establish
  that relationship. Several tasks in one Chat need distinct selected contexts.
- Users may select or change the scope through explicit instructions or a
  supported setting/UI. Do not require another confirmation of an already clear
  instruction. Natural-language initial selection is model-assisted behavior,
  not a guarantee that unexpressed intent will always be classified correctly.
  A standalone lock must be enforced by server configuration or a verified
  user-selected profile/UI: a model argument alone cannot lift that lock.
- A scope transition affects subsequent operations and hands over necessary
  instructions/results without replaying previous effects or retroactively
  importing/deleting history. Existing jobs, approvals and controllers remain
  owned until their actual transition/Stop boundary settles.

The Provider owns scope admission, retained selection, authorized access and
uncertain-delivery reconciliation. Core/upstream own actual file/process I/O;
Native owns conversation execution only for selected Codex work. MCP and renderer
local functions use those same operation semantics and return to the original
caller. Keep one implementation per responsibility; two advertised profiles do
not justify two providers, executors or conversation stores.

A single MCP with explicit task contexts is the selected implementation design.
Linux-app actual-model calls establish conversation correlation, including across
MCP/app restart, but the same anonymous session also spans distinct saved branches.
Therefore a conversation-wide last-selected mode/work cannot be the dispatch key.
Retain the existing work/controller for Native calls; local operations carry their
own authorized workspace context. Correlation can find eligible contexts and
present defaults, never silently choose between unrelated tasks or branches.
Public enforcement/read-write/quality acceptance remains required before delivery.
Fixed local-tools/Codex-work profiles remain optional explicit entry/standalone-lock
surfaces over the same implementation. Making both visible does not itself
guarantee correct automatic selection. Do not vary registered tool lists on each
turn to hide an unresolved scope decision.

Host-provided anonymous session metadata may correlate calls only within its
verified lifetime and authenticated connection scope. It is not the Chat URL's
conversation ID, original message/branch provenance, authorization or user scope
selection. An actual model tool call and a manually issued Linux SDK tools/call
have different metadata: the latter need not contain a conversation session.
Missing/conflicting metadata requires a verified explicit context or fixed
profile, never inference from another Chat's last selection. Keep authenticated
connection scope separate from host subject/organization/session hints. A branch
edit cannot automatically inherit another branch's latest selection from this
anonymous ID. CURRENT records observed clients/lifetimes; new clients, account
changes and unresolved fork semantics need their own qualification. Retain the
separate original-source verification contract.

Native work allocation and task execution are separate operations. The declarations
and pre-effect validation share action-specific argument requirements: allocation
(create/resume/fork), join, queue, chat_input and source_join require a stable
nonempty call_id. Allocation does not accept task/model/effort/collaboration-mode
arguments; send actual user instructions on start/join instead of silently ignoring
them. Assignment budgets/parent/dependencies require an explicit role; ordinary
work need not invent an assignment budget. Expose these conditions in the schema
and field descriptions. Missing identifiers and misplaced task fields return
cause-specific pre-effect errors with correction guidance. Reads do not gain an
unused call_id requirement. An argument rejection does not justify changing the
user's integrated intent to standalone development.

Already-running ChatGPT inference can attach to Native Core without a separate
Provider-owned ChatGPT AppHost. Host status/ensure concerns that optional app,
not Core readiness or the validity of work-allocation arguments. Describe this
boundary in the exposed tools. A model-reported fallback is a failed workflow
qualification even when its standalone file edit/test succeeds.

The integrated MCP work list may mark each row `chat_correlated` when its call
carries host `_meta["openai/session"]`. Derive the private association from the
persistent authenticated service owner and session/subject/organization tuple;
never store raw host identifiers in that association. Record it only after a
successful explicit create/resume/fork/join/ownership selection or accepted
attachment. Inspecting work alone does not select it. Missing metadata leaves
the marker absent; another Chat/account/service has no borrowed last selection.
Several rows may be correlated, including across branches. The flag is a discovery
hint only, never a controller, branch selection, source receipt or default dispatch.
Keep metadata outside model arguments and reject malformed consumed metadata before
effects. A failed optional association write must be reported alongside the
original successful Native outcome, without resubmitting or hiding that outcome.
MCP restart/credential rotation retain associations within the same journal owner;
local functions retain their verified explicit lane/work identity and do not
invent Chat metadata. This adds no conversation transcript or workflow alias.

Threadless I/O must preserve authorized read/write development, relevant workspace
instructions and applicable approval/reviewer behavior. Upstream fs RPCs do not
inherit a cwd path grant, and command/exec process IDs are connection-local process
controls, not durable idempotency keys. Never expose raw APIs as equivalent to
Native tool admission without qualifying these differences. Reuse the existing
operation reconciliation responsibility: uncertain mutation delivery must not
trigger automatic redispatch. Necessary operation/control records are not a
second Chat transcript store. Stop, streaming, PTY, disconnect and reconnection
must be qualified on the actual path. Local-tool completion produces no Native
work-completion event; do not implement this by merely suppressing notifications
while still creating unwanted conversations.

Standalone local admission uses `execution.standalone_permission_profile`, an
explicit Core permissions-profile ID chosen by the operator, never a model tool
argument. Absence disables local execution. Core validates that profile and its
managed requirements; cwd is a working directory, not an additional path grant.
Commands, file edits/patches and tests use `command/exec` under that same profile;
raw fs RPCs are not exposed. This is a standing configured grant, not Native
turn approval/reviewer parity: do not claim a Native approval event occurred.
The local context returns the workspace, permission profile and instructions to
read applicable workspace guidance before effects. No model or effort is chosen
by these tools and no independent inference is submitted.

`local_context`, `local_execute` and `local_process` share one Rust operation
backend. Context IDs bind the service owner and configured execution scope;
commands require that explicit context and stable call_id. Execution captures
complete stdout/stderr as private operation output, with byte-offset paging and
base64 for exact bytes. A returned process ID confirms admission, not OS spawn
or exit. Dedicated connections isolate each command's stdin/PTY/Stop and lifetime
without starting a separate Core server per command. Deferred Core command replies
are not cut off by the ordinary short RPC observation deadline. Output loss or
unknown transport delivery is explicit, never successful completion or an
automatic command replay. Exact retries return the original process/result even
across service restart; an unsettled old operation cannot restart itself.

Process controls have their own stable call_id and preserve original Core replies.
A terminate acknowledgement is not exit confirmation; only the original exec
result confirms completion. Validate the streamed command response shape before
marking exit; an invalid response remains unknown with its original reply retained. An early control can be explicitly rejected before
Core has registered the command; this is not a successful Stop. Connection loss
fences further controls on that handle and does not borrow a replacement
connection. Provider shutdown releases its own command connections only, not
shared Core servers or unrelated Native jobs.

`execution.workflow_lock=standalone|integrated` optionally fixes the server's
workflow surface for its whole lifetime. Standalone requires an explicit local
permission profile, excludes Native work/tools and does not recover/rotate their
controllers. Integrated excludes local tools. Neutral host/project/Chat inspection
remains available. The same admission guard rejects calls hidden by the profile;
model arguments and host correlation cannot lift it. Workflow locks govern Bridge
tool/inference admission; Core's selected permissions separately govern programs
and file access inside commands. Commands intentionally starting Native development
or Codex orchestration belong to integrated work. An unlocked service exposes
both configured surfaces and retains explicit contexts without global last-work
dispatch. Integrated model-owned local-function catalogs exclude standalone tools;
a standalone function consumer must use the same local backend and grant, never
pretend to be an integrated Native turn.

These optional settings preserve serialization of prior configurations when absent.
Rollback to a pre-standalone Provider requires the matched prior configuration
(or removal of the two new execution settings while stopped), as well as the
existing explicit inference-state downgrade. Preserve local operation/output
records: an older Provider must not convert them into Native conversation work.

#### Entrance equivalence and continuous development binding

**Primary admission correction.** Bridge-native participation is the primary
execution path. The source-follow admission/queue contracts below describe the
explicit source verification/recovery consumer; they must
not force ordinary Bridge requests to wait for saved-Chat crawling. That
replacement is delivered at CURRENT's recorded qualification scope. Source history
facts, identity checks and no-duplicate-inference requirements remain mandatory;
arbitrary external host provenance is not inferred from Bridge arguments.

For a selected Codex development work, CLI and ChatGPT entrances must identify
the same original input, actual execution and public record. Native history and
active TUI display must each be verified. Source observation alone does not
establish native tool execution or request-start instruction equivalence.

The initiating ChatGPT host owns an ordinary Android/web user request. Provider's
local Electron renderer cannot intercept another client's request-start path.
A supported project instruction/connection can guide the model; neither proves
that tools, developer instructions or a work controller are continuously applied.
No unsupported host hook may be assumed to make this guarantee.

**Execution meaning and terminology.** Standalone means ChatGPT standalone development
without joining Codex work, invoking its Native tool workflow or orchestrating
through it. Integrated means the common Codex work/tool/orchestration workflow,
whether the request enters in CLI or ChatGPT and whether reasoning uses an
already-active Chat response or a Provider-submitted model request. Attaching an
active Chat to existing Codex work is integrated. The workflow mode
does not by itself mean an independent child agent.
Independent delegation requires a separate reasoning context, a parent/assignment
relationship and original result return. No current/managed compatibility names
are introduced.

Historical pre-migration code used `Selection.reasoning=direct|provider` and
`Reasoning::{Direct,Provider}` for attached versus submitted inference. The current
source uses `inference=attached|submitted` / `Inference::{Attached,Submitted}` at
the scheduling/result consumers. Neither version makes integrated work a
standalone workflow. Native work controls always select integrated
workflow; do not add a repeated workflow selector where the tool/work identity
already establishes it. Attached admits the existing original response without
another primary submission; submitted starts a genuine model request.
Native auxiliary requests remain separate from development-work admission. The
upstream `thread_source=thread_title` and guardian metadata route to the existing
AppHost response lifecycle, preserving their original schema/result/cancellation.
They do not register a work, require codex_attach/finish, inherit a development
project, or change the primary workflow. Prompt words are not a request-kind
classifier. Keep absent/false auxiliary metadata out of existing request digests;
original primary/reviewer replay identities must remain valid. Qualify the actual
TUI-generated title response/name and absence of extra Operator work, including
cold server shutdown with all owned Native turns settled.
New work records use v3 with `selection.inference`; the runtime accepts no old
reasoning field or value aliases. Change declarations, callers and offline
migration together. `migrate-inference --target attached-submitted` converts
v1/v2 work, pending-index and transfer selections while preserving original
transcripts/receipts. Explicit targets provider21/provider14 prepare matched old
code/state rollback; this does not add old runtime parsers.
Delivery must pair the new executable/state with generated clients and refreshed
registered MCP declarations. Compare actual local/server/host schemas before
activation; stale hosts must not be concealed by accepting old inference aliases.
Do not change model `reasoning.effort` / `model_reasoning_effort`, which are a
different public upstream contract. Migration uses the existing journal owner
lease, immutable backup, whole-input prevalidation and restartable exact writes;
never reinterpret an old `reasoning=direct` Native record as standalone work or
retain aliases as a substitute for completing migration. Preserve original work,
history and matched code/state rollback artifacts.

Within integrated workflow, an already-active response can be admitted without a
second primary inference, and a genuine new CLI input can submit a new model
request. Both use the same selected work/controller, context acceptance,
permissions and Native event/result handling. Reserve the actual reasoning owner
before native turn/start and keep original tool-result return bindings. Validate
the exact turn/source and scoped owner when changing inference initiation;
preserve the development binding. Ordinary entrance changes require no manual
workflow-mode transfer. Attaching after a host response starts does not prove
that its original request-start instructions were identical; qualify that boundary.

Both initiation entrances are required for Provider-started execution: a native
CLI request, and a current Chat's explicit request to Provider. In the latter,
the current Chat keeps its original control/result return while the Provider
model's public response and actual native tools are recorded on the selected
native work. Preserve related managed work and its complete original result;
this is part of first-batch cross-entrance qualification, not deferred solely
because independent-agent extensions come later. A source continuation is not
permission to submit the same question again; an explicit Provider execution or
delegation request does intentionally start model inference.

Independent delegation is provenance/control information, not a required third
mode or separate screen. Record the actual child context, source, parent, status
and original result when they exist. A Provider-started primary response needs
no fabricated parent/child relation. Pro internal model work, sequential requests,
parallel tools and exposed independent host children require different evidence.

MCP and local_function are tool/result transports, a different axis. Existing
attached-response inference presently requires MCP; submitted inference can use
MCP or explicitly selected app-local functions. Both Native paths are integrated
workflow. Local functions need no remote Chat-side tunnel;
external ChatGPT-to-MCP does. Native Responses rounds, host tool continuations,
new primary user requests and independent child runs are counted separately.
Existing execution.mode=model_adapter|operator remains unchanged. Work schema v3
requires the explicit matched-code/state migration described above; transport
selection does not justify retaining duplicate responsibility.

**Identity and connection lifetime.** A work_id selects one existing native
thread and execution scope. Development responsibility, native thread/turn,
Chat conversation/branch, host request and tool invocation have distinct identity.
A Chat can address several selected works; a work can retain successive or
explicitly related source Chats. Neither one-Chat/one-thread nor one-Chat/one-agent
is a global cardinality rule. At most one admitted controlling execution writes
one work at a time. Multiple independent works may share the same canonical
repository and physical working tree; controller exclusivity is per work, not
per workspace. Their actual effects follow the resource-aware concurrency
contract below.

A persistent development binding is established only for an explicit/verified
existing development work and authorized source. It identifies account reference,
conversation, verified ancestry anchor, native home/workspace/thread and connection
state in the existing Provider work journal. It survives reasoning-route changes,
ordinary completion, reconnect and context renewal. Changing attached/submitted inference must
not silently discard the retained source connection. Explicit unbind ends future automatic
attachment; it cannot erase admitted input, unresolved effects or historical
source references. Stop fences execution; it is not implicit deletion/unbinding.
General Chat, source reading and unrelated development create no native work.
Titles and repository paths alone never select ownership.

A logical source request is keyed by account, conversation and user message ID.
A changing branch leaf is not part of that deduplication key. Reserve the queue
call ID with its verified original account once, persist it before enqueue, then
use that exact ID through consumption/recovery. Offline control-state migration
preserves already-reserved pre-account queue IDs; runtime never guesses a legacy
ID or submits a second queue item to repair a mismatch. A pending source with
missing reservation metadata requires explicit migration. Active new account-scoped
source execution must settle before a Provider14 format downgrade. Verify branch
ancestry/prefix separately; preserve observed branch and source-parent references.
Equal text with different original IDs remains two inputs. Store source-to-native
client/turn/item mappings and original transport-call bindings, not a duplicate
conversation. Native IDs and controller generations cannot replace source IDs.
Do not assign guessed source IDs to old records that lack this provenance.
The immutable per-work/native-turn source admission stores the verified account,
original user ID/fingerprint and ancestry anchor, queue client ID and actual
native home/workspace. `codex_history.source_admissions` returns correlations for
its returned turns; `codex_status.source_admission` identifies the active/latest
source. These are input-admission facts, not evidence that the model joined or
executed tools. Native client ID and original text fingerprint must both match
before admission is recorded. Historical records with absent provenance stay absent.

AppHost also retains per-native-turn source execution references for both entrances.
The actual dispatched ChatGPT user IDs/text fingerprints, parent ID and requested
model are distinct from the observed response model in public message metadata.
Imported context users are labelled as submitted context, not invented original
user inputs. Each projected native public item refers to its original ChatGPT
message ID and channel; streaming suffix items retain the same source reference.
`codex_history.source_executions` returns these references for its returned turns;
`codex_status.source_execution` returns the latest turn's reference. Records contain
identifiers/fingerprints, not another transcript, and survive lane advancement.
These are Provider source observations and projection references, not native
commit receipts. Check canonical history items for actual commit and the running
TUI separately for display; a recorded reference alone proves neither boundary.
Missing source model metadata remains unknown; a selected CLI model cannot fill it.
For submitted input, Rust computes the fingerprint from the prepared native
message. JavaScript associates it with the actual wire user ID only after checking
that the wire content equals that exact prepared message. An unrecognized or
changed wire user is retained as an unverified reference with no fingerprint.
Snapshots return identifiers/fingerprints, not another copy of the submitted
context. Source correlation must not consume the public snapshot byte allowance
again in proportion to the full imported input.
Conflicting identity or known model observations fail explicitly. Gateway logical
call IDs and native call IDs remain labelled separately from platform message IDs.
For native_tool requests, the source execution reference additionally maps the
actual ChatGPT tool-request message ID to its logical call ID and native call ID.
McpBridge accepts a correlation only when the observed token and complete tool
arguments match that same Broker invocation. Distinct platform retry messages may
refer to one native invocation; correlation never enqueues or reexecutes a call.
Unmatched/rejected host calls cannot be attributed to an earlier native effect.
Raw source arguments and tool capabilities are not copied into the reference.
Canonical native items remain the execution/result authority when saved Chat
tool-result bodies are unavailable. Other host controls are not native tool effects.

A source-observation pass retrieves the selected saved conversation once, including
any actual upstream history pages, then selects the source turn from that complete
bounded snapshot. Internal200-message delivery pages must not refetch the same
conversation or combine changing snapshots. Public paginated Chat reads retain
their existing cursor and prefix validation. Renderer/account-scoped saved-read
status distinguishes actual wire requests, successful replies, HTTP429 replies,
and local cooldown/in-flight rejections. Counters describe only the current
renderer lifetime; neither a tool-call count nor a local cooldown rejection is
an additional HTTP request. No inference or private content is recorded by them.

Continuous development entry must use Termux Bridge's admitted request as the
primary input to Provider and the existing Native work. Fixed-timer retrieval
of the full saved Chat must not be the primary admission or execution path.
Native context acceptance, permissions, tool invocation/results, progress and
completion remain under the same work/turn owner, with no duplicate inference.
An authoritative original Chat message/branch ID or final response requires
trusted host provenance. Distinguish host-supplied metadata from model-supplied
text/IDs; an MCP tools/call argument alone does not establish original identity.
Source lookup or notification-based observation is a separate, explicit source
verification/recovery mechanism, not a requirement for ordinary Bridge-native
development. A passive host notification prototype was examined and withdrawn
in favor of this primary path. Preserve original
records and identify missing source provenance; never relabel caller-supplied
completion text as a subsequently generated, unobserved ChatGPT public final.
The Bridge-first path must qualify exact input, context, tool/result,
completion and display semantics before replacing the current source-follow
path; existing source equivalence is not claimed while that work is unfinished.

For Bridge/attached-response progress, record every public progress message for the bound
task verbatim before publishing it in ChatGPT. On joining, also record any public
progress already shown for that task. Do not replace a longer public message with
a shorter Native status or expose hidden reasoning. A native-call report can
satisfy this requirement for that same message; do not emit an extra copy through
codex_report. The report declaration, attached-response context and Provider-driven
control instructions must share one progress policy. This is model guidance,
not automatic observation of arbitrary host messages; compare actual original
commentary with Native records separately.

For Bridge/attached-response completion, prepare the full public final answer before calling
codex_finish, submit that complete text, then return the same text unchanged in
ChatGPT after successful settlement. This instruction must be available in the
finish tool declaration, attached-response context attachment and Provider-driven control
instructions from one definition. It imposes no reduction of reasoning, tool use,
answer detail or public progress. A model instruction is not trusted host
provenance: qualify the actual later final separately, and report divergence
instead of claiming equality or adding a saved-history timer. Source-joined and
Provider model-adapter routes retain their observed-original completion contract.

Public assistant-authored report, finish, native-call report and handoff text must
not contain private service/controller/return values currently held by the accepting
control boundary. Reject such new text with private_control_in_public_text before
reserving its effect/receipt, consuming a native-call budget or emitting text.
The error contains no private value. The model may remove only those control
values and correct the same unreserved call_id; preserve all other public content.
This is exact known-value admission, not a pattern-based rewrite of examples,
public work IDs or arbitrary original user input/tool results. Previously admitted
exact receipts remain recoverable without re-emission; retained private original
history is not rewritten. Each accepting boundary validates the capabilities it
owns; this does not promise detection of unknown third-party credentials.

**Native execution rendezvous.** Native execution and its TUI must use the same
selected execution home/runtime/system configuration and complete resolved model
catalog. The original catalog is configuration input, not a projection of UI-known
model fields. Preserve it through configuration loading and pass that snapshot to
Core without losing unrecognized fields, renaming fields or inserting UI defaults.
Native typed model metadata remains the UI consumer's view, not the backend's
catalog authority. No second executor is created simply for display. Qualify
create-before-TUI and TUI-before-create separately. A stored thread ID alone does
not identify its live event stream. Existing recorded owners remain pinned and
fenced; do not rewrite history or transfer authority implicitly. Observers of an
existing work use its actual recorded owner.

For a Core-owned local shared backend, verified live-work attachment uses the
admitted Native work's authentication rather than demanding a new local OpenAI
login. Native still enforces actual credential, model and policy requirements.
Configured new work follows the invocation's resolved provider requirement;
account/read describes the backend default and may differ from that selection.
A new ChatGPT work configured with requires_openai_auth=false needs no separate
OpenAI login, while a newly selected OpenAI-auth provider retains its requirement.
With a validated independently authenticated Provider installed, the Core-owned
local TUI may defer OpenAI onboarding so the user can reach `/provider`. An
account/read error with Native-owned `data.reason=accountAuthenticationUnavailable`
is retained as unavailable account presentation and shown to the user at startup
or reconnection; it does not become a successful account response. Native marks
rejected refresh credentials or unauthorized account discovery this way;
requirements, routing and unclassified legacy errors retain their failure
semantics. Native model/permission admission and actual request
authentication remain mandatory. Transport, model-catalog and managed-policy
errors do not become account observations. Remote, embedded and unqualified
backends retain account-read policy. Existing login controls remain available.
This changes the local startup UI decision,
without fabricated accounts, credential copies or weaker request authentication.

Native qualification defaults to the complete build/test/compatibility/lint gate.
Explicit native-api or frontend-compatibility runs qualify only their named
scope and never satisfy full frontend publication or its in-flight proof. A
compatibility-only run may execute a prior identified frontend only when the
owned workflow/run metadata, upstream, target, build settings and executable
bytes match, and the complete prepared production diff is unchanged. App-server
integration-test sources are the permitted difference; changed production input
requires a new build. Keep this proof scoped rather than replaying accepted
builds and regressions or presenting a partial green run as full acceptance.

Qualified frontend preparation includes upstream Config source
preservation as well as TUI code. Artifact evidence must retain both; format/lint
comparison must cover the actual qualified source inventory. Additional Native
compatibility work follows sections4.5–4.6: demonstrate the interface gap, update
the declared source/behavior and qualification scope together, and keep execution
and policy with Native. A formerly narrow patch inventory is not proof that an
outer workaround has the right responsibility. Lint must not introduce changes
beyond the assembled input.

**Bridge admission API.** `codex_sessions join` selects an explicitly named
existing, completed Native work and receives `task` plus a required stable
`call_id`. The common Native start transition reserves one attached-response generation;
it does not retrieve saved Chat or submit Provider inference. The authenticated
Bridge caller receives the generation controller and uses `codex_attach` to read
and accept the actual current Native context before effects. A running, stopped,
unreconciled work or outstanding source admission cannot be silently taken over.
Exact retries return the retained admission result; changed arguments conflict,
and an uncertain start requires reconciliation rather than another start.
`task` is Bridge-submitted input, not independently verified original Chat text.
Supplied progress/final use the common report/finish controls and retain that
meaning; neither is labelled as an unseen host message.

`source_sync` and `source_join` are explicitly requested saved-source recovery /
verification controls. They retain original-ID/branch/context verification and
never run in the background. `synchronize` is solely the Native input barrier
used by CLI: it waits for an admitted attached/source execution, without saved-Chat
retrieval. Completion observers consume Native lifecycle events/recovery reads
only. A development binding does not enable a periodic remote history reader.
The CLI barrier applies to starting a next turn, not to steering an already
observed running Native turn. Active-turn steering uses upstream's current writer
and canonical turn ID; if a prepared steer loses that turn before submission,
keep the draft rather than
silently convert it into an unsynchronized new turn. Unreconciled writer/effect
state is never cleared by a TUI shortcut.
Native may cancel and reissue a Provider request on the same canonical turn
when accepting Steer. A successor on that same known turn waits for the canceled
request's Operator task to retire before reserving its AppHost slot. Retirement
is not remote Stop or effect settlement: all existing admission and confirmed-Stop
checks still apply afterwards. An uncanceled concurrent request remains busy;
Stop/controller fences and cancellation of the waiting successor remain effective.
After a confirmed remote Stop, Provider may admit that
continuation only when the previously submitted Native input is preserved as an
exact prefix (excluding transport-only additional_tools declarations), a new
non-context user input is appended, and any offered Native call has its result.
Same-turn Steer retains its owned remote conversation and source references.
The observed per-lane network dispatch counter, compared with its value at the
original submission, determines whether that submitted prefix is already in Chat;
only new input is sent after an observed dispatch. An undispatched request keeps
the previous import cursor. This starts no Native turn and clears no broker
effects/controllers. Exact retries,
changed prior input, missing tool returns and unknown Stop remain fenced. Older
lane records without the submitted-input boundary retain their refusal until a
new admitted request records that boundary.
MCP observes that same confirmed remote Stop before marking its transport round
complete. It revokes the old tool lease and permits another input only without
pending settlement, offered calls or unsettled broker effects. It preserves old
receipts and effect claims; a Stop request alone never completes the round.

Implementation and remaining qualification order:

1. Inspect the authenticated MCP envelope at mcp.rs::rpc/tools/call and the
   mcp_stdio forwarding boundary. Establish which source fields are actually
   host-produced; preserve model arguments as arguments, not trusted host input.
   Do not require a saved-source GET merely to execute an authorized Bridge task.
2. Share Operator::session_action(start|join), native_admission and attach/context
   receipt with the existing controller-generation rules. Register a
   Bridge task on the explicitly selected existing Native work, then return its
   current context. Reserve exactly one turn before publishing tools. New work
   and independent worker creation remain explicit choices. Drain any old source
   owner at its verified Native boundary before granting a new writer.
3. Reuse Broker/Operator tools, progress and completion. Provider-origin Bridge
   requests use the normal Native->Provider model stream and original result
   return. Attached-response requests use the invoking model, with no new primary
   inference. Preserve complete output; distinguish a submitted completion from
   an unobserved later Chat final until a qualified host event supplies it.
4. Keep watch_completion restricted to Native lifecycle observation and ordinary
   join free of any saved-source read prerequisite. Retain only necessary Native
   lifecycle observation and explicit source verification/recovery consumers;
   update their schemas, callers, tests and docs together. No second route may
   conceal the obsolete admission path.
5. Qualify repeated Bridge-origin prompts on one Native work, context acceptance,
   exact retry/result, Stop and approvals with zero idle saved-Chat requests.
   Then qualify actual Android/Web request metadata and full public output/TUI
   correspondence separately. Model omission cannot be detected from an input
   the system never receives; report that visibility boundary without inventing
   a task, polling all chats or implying that execution occurred.

Attached native tool inference retains its existing app-free capability. Complete
saved-source observation currently requires the authenticated app reader; absence
reports source_observation_unavailable rather than disabling unrelated native
tools or claiming complete source synchronization. A context delivered as a tool
result does not acquire developer authority or replace ChatGPT host policy.
Shared public execution does not imply identical hidden/system prompts across
hosts; native execution/approval policy remains enforced by the runtime.

**Single source admission and current-response participation.** Extend existing
codex_sessions with bind, unbind and source_join, shared by the existing CLI/control
facades. bind validates the selected source/work; unbind releases that future
connection at a reconciled boundary. The existing owned source is registered
in a versioned per-work development binding, independent of reasoning selection.
Its enable flag controls explicitly requested source recovery and survives service restart.
Source ancestry stays owned by the AppHost lane; only its verified owned successor
can advance the binding to a new conversation. bind/unbind require the current
controller and exact native history/context cursors at a completed native boundary;
bind also verifies conversation_id and the complete saved source_cursor. A repeated
old mutation receipt never overwrites a newer binding. Binding mutation receipts
are scoped to work/action/call ID and exact non-controller inputs, so an ordinary
service/controller rotation can retrieve the same non-secret receipt. A new
mutation still verifies the current controller and settled boundaries. Other
control mutation receipt scopes retain their existing authority semantics.
Native and Chat histories
are preserved. Initial binding to an unrelated/unowned Chat needs separate verified
source adoption; this must fail explicitly until that admission is implemented.
source_join admits the existing Chat response to
its verified original user request and existing native work. It reuses native
admission, tools/approvals, attach/context acceptance and original result return.
It must not submit the original prompt to ChatGPT again, fabricate a replacement
task/summary or create another work. If source observation already admitted the
native turn, source_join must reuse that admission/turn instead of starting another.
A native Responses/tool coordination round is allowed without a new primary
ChatGPT request.
The source_join facade returns context_required plus the exact native history, execution
instructions and context receipt. Echo that receipt as accept to receive the live
source-native tool lease; receipt changes require rereading actual context. It is
bound to source admission, native generation, current history/context and policy.
If original-input observation/admission is still pending, source_join returns waiting
with the actual observation cause and retry time. An older completed turn does
not prove that the new response ended. Retry the same explicit source_join; claim, resume
and legacy attach acceptance are not recovery paths for source-owned execution.
Source participation records catalog discovery, context acceptance and actual
native-call observations separately. A consumed source with no participation is
execution_not_joined even if its original final was mirrored successfully. The
transport's per-native-turn source lease is a reference to existing broker state;
it is never sufficient to revive execution after service loss or Stop. Inability to join an existing admitted turn must remain explicit,
not be hidden by a second native turn.

Preserve the existing source-verified native_catalog/catalog_readback and
source_discovery continuation as the minimal participating path for retained MCP
Chats. When it already satisfies this source admission, no additional join call
is required. The new source/work-addressed facade must share that admission and
token lifecycle; it must not create another executor/token manager or revive old
native_tool execution capabilities. Qualify the repaired observer and that actual
existing path for explicit recovery without imposing it on ordinary Bridge admission.

Source-joined completion reuses the existing AppHost/McpBridge source-follow
adapter and native_catalog/native_tool return, including when the reasoning
origin is the active external response. Its admission is not a new Provider app
submission. source_join returns admission/tool readiness without waiting for the final.
The original Chat public final then settles the native response round. Do not
require codex_finish on this path: the Chat may await that tool result before
producing its public final, so waiting for that final inside finish creates a
cycle. Preserve codex_finish's existing caller-supplied native final/completion
semantics for Bridge/Direct control consumers. Do not label that
control text as the subsequently generated original Chat final. Keep the source
identity/admission and native executor common, not two substitute transcript paths.

MCP does not inherently supply trusted conversation/message identity. Explicit
source IDs require account/source verification. Otherwise a bound source may be
resolved only when one unambiguous unclaimed original input exists under the
verified anchor. Multiple concurrent inputs/branches require source selection;
never attribute an old response to the latest user message merely by recency.
A client can retain an older selected branch after CLI input creates a newer
sibling. Report that verified conflict as source selection required; a later
read cooldown must not erase it. Explicit source_message_id selection may connect
a sibling to the same idle native work only when its selected ancestry contains
an exact previously committed public final of that work/account/conversation.
Derive eligible anchors from canonical native items and their existing source
execution references; never guess by text, timestamp, title or an arbitrary common
ancestor. Admit the first unconsumed original after the latest eligible ancestor,
verify the selected message ID, and retain both native history and saved branches.
The joined model must accept current native context, including CLI work absent
from its saved branch, before source tools are released. That divergence persists
across an interrupted or completed source response: each later source participant
must accept its own current context before effects. A Provider-started continuation
can resolve the divergence only after it has received the complete current native
context and completed; an uncertain or interrupted submission does not resolve it.
Unknown ancestry or
multiple unselected sources remain explicit conflicts. Selection grants no
controller, approval, extra inference or replay of earlier effects.
Reading the source acquires no controller. Existing work ownership, native scope,
controller generation/receipt and native permission decisions remain enforced.
A participant cannot approve a native human/reviewer question by accepting a
connection receipt. App-owned response execution cannot be acquired through
legacy controller attach/accept, report or finish, including Direct source-follow.
Source context acceptance authorizes its existing source tool lease; it does not
set the legacy controller's accepted flag or let a caller replace the real final. Uncertain admission is reconciled through the original queue,
client ID, turn and receipts without redispatch.

CLI sequence: native input admission -> reserve source request -> one new primary
app submission with selected native model/effort/instructions/tools -> bind actual
source UUID -> original native tool/results and public progress/final -> commit
and TUI consumption. Tool continuations may cause additional host requests; they
remain part of that logical request, not unreported independent agents.

ChatGPT Bridge sequence: host starts its user response -> model selects the
existing work -> join admits Bridge task on that settled work -> codex_attach
returns current context/instructions -> model accepts receipt -> the same model
uses native tools and reports full public progress -> original tool results return
on the same invocation -> submitted final completes the Native turn -> live TUI /
replay. No additional Provider primary inference is requested. Exact remote-host
message identity and a later host final remain distinct unverified boundaries
until actually supplied/observed; Bridge input does not invent their provenance.

Explicit original-source recovery sequence: source_sync/source_join -> verify
account/original input/branch -> reuse source-native admission and Broker lease ->
original active response can use native tools -> observe original public final ->
Native/TUI commit. An ordinary queue can start inference; only positively admitted
source-follow rounds have no-reinference semantics. chat_input deliberately starts
new saved-Chat input, not synchronization of an existing response.

**Observation, partial responses and recovery.** A Native completion observer
belongs to an admitted execution and retires when that Native round settles.
It uses Native events/recovery reads, never a periodic saved-Chat request. Status
reports native_observer_running separately from execution and the last explicitly
requested source verification. A last source observation is not a heartbeat or
proof of physical UI display. A binding alone starts no remote read, model or
Native task, including after service restart.

Model autonomy selects tools and Bridge participation; deterministic code validates
received work/control, context and effect identity. If the model never calls Bridge
and no supported host input event arrives, that request is outside the system's
visibility. Report that capability boundary; do not poll other clients' saved
history to pretend mandatory participation. During explicit source verification,
an observed unjoined response retains execution_not_joined; a later source_join
may reuse its actual active admission. An ended response is not automatically
resubmitted or resumed by a different model. Mandatory pre-response joining across
Android/web requires independently proven host support.

Preserve interrupted/cancelled input and all available partial public output as
such, then advance to a verified next source in order. Absence of a successful
final before the next user input is not a reason to permanently lose observation,
nor permission to fabricate a successful final. Before advancing an interrupted
source, confirm the exact native turn is failed/interrupted and its effects have
settled; keep the source discovery connection while retiring only that execution.
Unadmitted calls receive an explicit not-executed interruption receipt. Replaying
the failed request returns the same partial output and failure, never success.
Unsupported attachment/content
must retain source references and an explicit unavailable/blocked state. Implement
lossless supported types before claiming that type accepted; never replace the
original with a summary to pass fidelity qualification.

Known transient saved-read/CDP failures enter cause-specific retry_wait and host
readiness revalidation. Unknown/auth/account/branch failures enter an observable
blocked state with recovery conditions. An explicit retry revalidates those conditions without a Provider restart; do not
install a background reader or duplicate an already-admitted execution. A read timeout
never authorizes resubmitting a mutation. Saved GET429 keeps the existing shared
account-bound cooldown: honor Retry-After, otherwise at least60s exponential to
300s on consecutive429s; other transient reads back off. Requests during cooldown
perform no GET. Account changes do not inherit another account's cache. Successful
retrieval resets the consecutive counter. Native event/approval handling continues
while source reads wait. Do not restart a live host merely because a probe failed.

**Event and screen consistency.** Preserve the original public user, commentary,
tool call/result and full final with source/type/visibility references. Native
tools are produced/approved/recorded by upstream; host tools outside Codex are
labelled source observations with available original references. Hidden reasoning
is never exported. Redacted/unsupported content is explicit and cannot qualify
as complete content. Public source observation, queue ACK, native consumption,
native commit and screen consumption/physical display are different boundaries.

Use the actual native notification/history/replay path and bind the displayed
thread to its verified execution home/backend. Recover dropped/lagged events
through canonical history and existing TUI mechanisms with item/turn deduplication;
no second TUI transcript store or Provider-generated synthetic native effects.
The existing source-identity presentation may fold only a completed physical
message's progress display, retaining expandable full text, final and tools.
Do not change model/effort, prompt fidelity or answer completeness for display.

CLI/native and external source admission share one work's serialization boundary.
Uncontrolled external hosts may start requests before local admission; conflicting
branches must remain explicit and await source selection rather than be silently
linearized or rerun. Stop fences new effects immediately; native turn/jobs,
approval and host-stream cancellation confirmations remain separate. An external
stream lacking cancellation proof reports remote_stop_unconfirmed. Reconnect,
account/model changes and compaction preserve source identity, original usage and
unsettled receipts. Derive model/effort from the settings timeline and correlated
requests; no silent lowering or retrospective attribution. Preserve ordered-prefix
validation instead of resetting it to admit mismatched history.

**Language and component boundary.** Rust owns development binding/source
admission, native work/control/approval routing, policy, durable budgets/receipts,
source validation and recovery. JS remains for renderer-only React/module/API
access, stream callbacks, local-function delivery and synchronous checks/reservation
immediately before uncertain dispatch. Account/owner/Stop/duplicate checks at that
last host boundary remain necessary across time/process boundaries. Do not move
those to asynchronous per-event CDP round trips. Do not put native work selection,
connection lifecycle or a new scheduler in JS. Full Rust replacement requires a
proven alternative host interface; JSON encoding alone is not that interface.
Core stays independent of optional Provider/Manager/TUI. Upstream runtime remains
the canonical executor/history authority; Manager owns optional convenience only.

**Agent extension.** The first implementation/acceptance batch must cover same
source, actual execution and both screens across entrances. Independent delegation
follows that foundation. Existing Provider assignment.parent is a logical work
relation, not proof of upstream parent_thread_id or /subagents membership.
Prefer actual native spawn/agent lifecycle for native children. External model
workers require separately verified reasoning/source identity and result return;
current thread/start/fork APIs inspected in AUDIT do not register an external
child merely by writing a parent ID. Never edit rollout/DB ancestry directly.
Parent records carry delegation/status/result/source references; each child retains
its own complete public execution record and accessible original context. One Chat
may reference such a tree without a global one-Chat/one-parent mapping rule.
Independent-worker implementation includes proving actual separate execution and
its parent/result relationship, then designing and implementing assignment/context,
tool and approval authority, lifecycle/Stop/recovery, usage and result acceptance.
It is not satisfied by displaying parent/child labels alone. Capability discovery
in Pro is an investigative aid: examine exposed interfaces/events and retest the
identified mechanism with the ordinary model before declaring it Pro-only or
generally supported. Missing observations establish neither conclusion.
Same-Chat independent host children require demonstrated run/parent IDs, event and
result APIs plus individual history access; sequential same-Chat requests are not
that proof. A native/UI protocol extension requires its own explicit contract,
source compatibility and full acceptance before presenting external native ancestry.

**Acceptance and migration.** AUDIT scenarios1–10 are mandatory, with1–9 the first
entrance batch and10 explicit delegation. Distinguish source/doubles, real native,
actual Android/web model execution, saved readback, current PTY/reopened TUI and
physical UI observation. Include joining omitted/late, interrupted/overlapping
inputs, lost effect replies, account/model changes, approval/Stop and compaction.
Count primary submissions, continuation requests and tool effects independently.
No new primary inference for importing/reading the original source is permitted.
Unknown IDs/content/capabilities remain unfinished, not weakened PASS results.
Old work/selection/history/rollback artifacts and user auth/profiles remain usable.
Existing active work records use the v2 codec and the old direct/provider
inference-selection field; migrate this separately from public workflow mode. The
normal runtime does not read v1 work records or old selection aliases. Explicit
offline migration preserves originals; rollback to Provider14 requires inverse
format conversion before the old process starts. Lane control state additionally
persists the source queue reservation; offline migration preserves old reserved
IDs without changing user text, native records or historical MCP receipts. New connection
metadata uses versioned work-keyed records in the same Journal/owner lease, found
from the existing work index without repeated whole-receipt scans or a second
persistent index. Do not append unknown fields to deny_unknown_fields records.
Use existing operation intent/per-work admission for cross-record/native
reconciliation and advance source anchors only after confirmed native commits.
Qualify rollback startup/readback with the retained Provider on disposable new
idle state; it may lack new connection capability but must still read old work.
Fence/reconcile active or unknown new execution before downgrade. Legacy absence
never permits guessed migration, auto takeover or replay of reconciliation-required work.
An explicitly admitted start/join or claim may reload an unloaded persisted thread
on its verified Native owner before querying live jobs or starting a turn. Preserve
its public history/context across resume and revalidate scope and the work/controller
generation. A read-only status/history request must not load a thread. Unknown
writers, changed history/context and resume failure remain explicit failures; no
user input, tool effect or model request is replayed to recover loading.
Resume and the requested start/claim share one durable logical-call intent;
an existing intent from another invocation never authorizes redispatch.
After Native becomes idle following a supplied compaction summary, the input
barrier permits the next request even while the summary checkpoint is retained.
The actual next Native request must acknowledge that checkpoint before execution;
waiting for that request at the input barrier would deadlock its own validation.

#### Common interfaces and lifecycle

The retained interfaces below coexist with the entrance-equivalence target above;
CURRENT distinguishes their implemented/qualified scope from unfinished connection
behavior. New binding/join work must preserve these existing control contracts. Retain legacy execution.mode/config/schema names
and unchanged defaults. A logical interface below is not a new required daemon,
Core dependency or second execution/history authority.

**Work and entrances.** A stable work_id selects one native thread and workspace.
An assignment has its own task identity, role, parent/dependencies and return
destination; a Chat may serve successive related assignments, and an assignment
may span acknowledged successor Chats. A controller generation grants exactly one
current writer/control lease per work. Native tool IDs/turn IDs and saved Chat
IDs/branches remain distinct and are returned as source references. Display titles
are labels, never routing keys. Original progress/question/final destinations are
bound at submission and survive Chat replacement; a detached observer does not
change the destination or acquire a writer.

The common work service supports list/select, create/attach, submit/follow-up,
history/status, answer, finish, handoff/transfer and Stop. CLI and MCP are facades
over the same service. Extend existing codex_sessions/control contracts rather
than build a second scheduler/executor. Route selection is per work: reasoning in
the current Chat or in an app-managed Chat. The existing service-wide mode remains
a compatibility input alongside the per-work combined routing. MCP/local is selected before
managed execution and cannot change on uncertain delivery. General delegation
always enters through MCP and binds the current Chat's question/result return;
the inner managed path is the same as Codex-origin work. Attached-response
inference does not start another primary Chat; it is still integrated workflow.
Child/reviewer inference and any app acquisition are
explicit dependent assignments with separately counted cost.

The Provider resolves the native thread/work binding before dispatching each
Responses request: submitted inference reuses model-adapter/AppHost/McpBridge
mechanisms; attached-Chat inference reuses the native-control Host without opening
a primary Chat. Both interfaces are registered in one optional Provider service.
Codex-origin Responses discover the unique loaded thread across Core's verified
execution homes and derive the workspace from native thread metadata. Provider
execution.home/workspace are defaults for explicit work creation, not restrictions
on the account or workspace of an incoming Codex session. Native account changes,
cold resume and working-directory changes revalidate that binding on each request.
Recognized legacy rendezvous records whose process stat is absent or terminal
cannot block another verified owner. Android may hide a reused foreign PID's
directory while reporting its stat absent. Discovery preserves those records and
never connects to or signals that PID; unknown codecs and unsafe records refuse.
A genuinely new native turn may reconcile a settled Codex-origin work to its new
owner/workspace while rotating its old controller. Ambiguous writers, pending
effects and stale controllers retain their existing refusals. Explicit Chat-owned
work remains bound to its selected execution scope until reconciled transfer.
Existing operator-driven managed execution remains a compatibility route until
changed-path comparison and explicit retirement review; its name is not the
architecture. Managed MCP is the selected new opt-in preference and managed
local is an explicit before-start alternative. Bare/installed legacy defaults
remain unchanged by common-route integration.
Do not add another execution.mode permutation for these roles: keep that setting
as the legacy default for unbound requests. Within integrated workflow,
new work selects attached or submitted inference through the common work service; the assignment's route
remains fixed until an acknowledged transfer. Existing create/resume/fork/start
inputs keep their meaning when the new selection is absent. The common CLI is
the optional Provider's candidate work/chats/host facade; independent release qualification admits its public
codex dispatch/packaging separately without making Manager mandatory.

One renderer connection/hook owner is shared, with transport declarations per
lane/work rather than a service-wide mode toggle. Request ownership requires
its lane metadata/signature, an already sent
batch, an observed host-permission response or an active owned stream. A saved
conversation ID alone does not own a new ordinary Chat user request after that
stream settles. Such input and its ordinary stream pass through unchanged;
late owned requests remain fenced by Stop and duplicate-delivery rules.
The Provider must remove the existing
global app-bridge mode before admitting mixed MCP/local lanes concurrently;
validate capabilities before inference, pass nonowned Chat requests through
unchanged and never replace a busy owner's hooks. A capability conflict is an
observable admission failure, not permission to restart the app or switch paths.
Native Pool lookup includes qualified runtime binding, execution home, canonical
workspace and routing configuration. Independent child workspaces must not be
routed through a fixed-workspace Pool by changing only a displayed cwd. Preserve
Core's server/config isolation and native child/history semantics.
During Provider discovery, a structurally valid historical record whose PID now
belongs to a different executable is not a native server candidate. It must not
block an independently verified current thread. Never connect to that process or
repair/delete its record. Selected execution bindings still require exact
PID/start/image and peer/socket validation; malformed records and ambiguous live
native writers remain errors. A discovery exclusion does not authorize takeover,
model/tool replay or acceptance of a changed selected owner.
The same exclusion applies before namespace decoding to the two known historical
owner shapes (home/runtime, optionally followed by `catalog-v1`) only when private
owner/PID reads and the kernel prove the recorded process is absent or now has a
different executable. Preserve those records without interpreting their old codec.
A matching live owner, unreadable/unsafe owner or PID, unknown owner shape and any
malformed current config-v1 record remain errors. A bounded explicit device repair
may separately archive a verified inactive historical record, preserving its
complete contents and restore path; ordinary discovery performs no such mutation.
An unavailable rendezvous does not establish that its Native process or writer
has ended. Discovery may continue observing other verified candidates, but a
unique loaded-thread match survives that socket failure only when every unavailable
candidate has the same execution home and the existing shared Native writer-lock
proof identifies that exact matched process as the thread's writer. Keep scanning
for duplicate live matches; they remain ambiguous. Unavailable other homes, absent
writer proof, malformed records, unsafe sockets and changed peers remain failures.
No match plus an unavailable candidate preserves the original socket error rather
than declaring the thread absent or preparing another owner.

The work owner may inspect, stop and supply genuine follow-up input. Its managed
worker holds the current native execution controller; a coordinator cannot issue
competing native tool writes while delegation is active. Changing reasoning
ownership uses the explicit reconciled transfer contract, not two simultaneous
controllers. CLI uses stable work handles; raw private transport IDs are optional
diagnostic data rather than required user input.

The common work interface extends codex_sessions with explicit reasoning selection at creation, genuine
follow-up submission and source-bound event/result retrieval. Discovery declares
the supported arguments/capabilities; old servers reject unsupported selection
before starting work. Add codex_chats for list/read only and codex_host for the
shared ensure/status/open/stop actions. Chat input and native controller transfer
remain explicit work operations rather than effects hidden inside read. New
mixed management tools retain false idempotentHint until all their actions have
the settled scope/receipt contract; new pure observations follow the read policy.
Bounded event retrieval returns fresh events/cursors and durable terminal
receipts, without restarting worker inference. Neither a cached pending status
nor a client timeout marks the work complete.
Before upstream persists a fresh thread's first turn, paged items may report
that history is not supported yet. Event retrieval must explicitly report that
observation as unavailable, retain the caller's cursor and return pending without
resubmitting work. A completed work status alone cannot certify final delivery
through an unavailable event history. Other native errors remain errors.
At verified work completion, events/wait also returns `completed_result` with
the current native turn ID and its original public final-answer items, selected
from that same native history. Native item creation order does not imply that
the final answer is the last item. This focused view remains available when the
caller's cursor already covers those items; it creates no new history or result.
Wait cannot certify completed delivery while this current-turn final is missing.
Callers reuse the returned cursor for incremental events instead of repeatedly
retrieving all earlier tool output to locate a final answer.

The candidate `work` CLI and MCP share `events` and bounded `wait`
(`max_wait_seconds` 0–300). Waiting observes local native events, returning the
original work's question, terminal result or pending handle. It neither starts
another model nor certifies receipt by a disconnected caller. `chat_input`
requires an explicitly selected saved conversation, complete `source_cursor`,
related task, stable `call_id` and finite app/time budget; `chat_result` observes
its stable `input_id`. Durable input intent precedes submission. Lost replies
and restarts reconcile the saved original message UUID/parent and an unambiguous
public final descendant; they never authorize retransmission. Unavailable output
or an ambiguous source cannot certify a completed result.

`chat_input` represents an ordinary user submission to that selected Chat. Its
new conversational content is exactly one visible user message containing the
supplied task, without added developer instructions, environment/context bootstrap,
tool-result imports or an independently delegated Native task. Existing Chat
context and ordinary host behavior remain in force. Identity, branch, delivery
budget and model-selection metadata are request controls, not prompt content.
This user-emulation operation is a specialized capability, not the default
entrance for all model-driven development. An agent uses it when the workflow
actually requires a user-like Chat submission or client-path qualification.

Model and reasoning-effort selection is independent of ordinary-Chat delivery.
For development workflow submissions, support explicit invocation selection over
configured defaults, using the existing validated model catalog. A model acting
within an authorized workflow may choose
an invocation setting when the user has not fixed it, including for a test; this
must respect user model/cost constraints and does not introduce an automatic
selection agent. Workflow code must not hardcode a model or disable thinking
merely to preserve ordinary-Chat submission. Resolve and expose the actual
selection and its origin without rewriting shared configuration or mutating
process-wide environment between requests. Invalid or unavailable explicit
selections fail before dispatch rather than silently downgrading. Requested
inheritance of an existing app selection requires verified settings; a previous
response's model label does not establish the currently selected effort. For
user emulation, resolve the target owned Chat's effective selection only after
its conversation metadata query, branch history and applicable model catalog are
ready. Hydrating message nodes alone does not establish that readiness. Retain
the existing exact-message, stable message-ID and durable admission guarantees;
an app helper that trims input, adds default hints or omits lifecycle controls is
not an interchangeable replacement solely because it reads current settings.
Revalidate identity, branch and selected settings across asynchronous preparation.
Omitting model/effort on a lower-level send means host defaulting, not inheritance:
the qualified ordinary Chat without a custom override resolves that omission to
auto/no explicit effort; applicable host overrides retain their own semantics.
Caller omission and fields populated by the host SDK are distinct.

User-emulation selection, explicit/configured workflow selection and host-default
selection have distinct semantics but may share the existing send implementation.
Do not infer the selection of a different Android/web client from Electron-local
picker state. Only verified synchronized/persisted settings can be reconstructed;
an unsubmitted selection on another client remains unknown without evidence.
Original settings before a new invocation and settings admitted for that invocation
must remain distinguishable from response model metadata. Returning a tool result
or approval is a continuation with its own actual caller, not another user input.

Input interfaces follow their actual consumers: chat_input supplies user text to
Chat; queue supplies input to an explicit Native thread; start invokes Native work
with its selected Direct/Provider route; a tool return resumes its original
invocation. Context/instruction assembly remains the model adapter's responsibility.
Reuse these interfaces instead of expanding chat_input into a generic message,
environment or tool-result injection API. A new input type requires a demonstrated
producer/consumer whose semantics these existing interfaces cannot represent.
Distinguish user emulation from workflow-configured or structured input at the
caller contract, without requiring another transport or forcing development
requests through chat_input. Model/effort controls alone add no conversational
instructions; new instruction/context or tool-result messages change the input
semantics and stay with their appropriate execution interface.

#### Input preparation and execution settings

These are the approved execution contracts. Source qualification and installed
product state are recorded separately in CURRENT.md. Keep one execution
authority per destination: ChatGPT interprets its conversation/picker/catalog;
Native Codex interprets its thread settings and turn overrides. Rust admits the
owned task, validates user constraints and preserves exact input/return identity.
The renderer reads/prepares app state and invokes its existing send capabilities;
it does not choose Native ownership or assemble another development policy.

1. Input purpose, settings source, reasoning route and return transport are
   separate. User emulation is specialized chat_input; development uses existing
   Native/Provider execution owners. direct/provider and mcp/local_function do
   not acquire new meanings or parallel orchestrators.
2. Existing settings survive ordinary continuation, resume and a mode-only
   change. Explicit user/workflow model selection wins; an existing Native thread
   otherwise supplies its acknowledged model/effort; config initializes a new
   task or an explicitly selected configured route. Config is not a recurring
   override of a resumed thread. Existing Chat inheritance uses that exact Chat's
   prepared settings, not an unrelated Native thread's or app-global default.
3. The caller's exact user text remains exact. Context/instruction/tool-result
   assembly retains its existing owner. Reading app settings adds no message,
   hidden prompt, summary, tool invocation or inference.
4. Resolve before dispatch and persist the admitted settings in the existing
   input intent. Omission, explicit none, inherited effort and host-default
   routing have different meanings. Validate the model/effort pair; do not clamp,
   guess a missing effort or silently switch to an easier route.
   For an unmodified Chat selection, the ready app selector and its model
   catalog own the host pair: the Native conversion catalog need not represent
   every host pair. Retain that exact pair, and leave the Native effort unknown
   when it cannot be mapped. An explicit override requiring an unknown inherited
   Native effort must fail until the caller supplies a compatible effort.
5. A request override is not necessarily temporary: Native model/effort overrides
   apply to subsequent turns under the upstream API, and Chat submission may
   affect saved conversation settings. Preserve the real destination semantics
   and report the acknowledged selection. Do not advertise one-turn isolation
   or restore a previous value without an explicit operation and concurrency proof.
6. Read-only preparation may await normal metadata/catalog readiness. Revalidate
   account, branch, selection and control ownership at the existing admission/send
   boundary. A missing/moved source fails with its cause before dispatch; a sent
   or uncertain intent is observed, never re-prepared and resent under new settings.
7. Already-running Direct Chat responses retain their host-selected settings.
   Registering them in Native history does not start a replacement inference or
   change their model. Tool/approval returns retain their original request binding.

Public request draft, simplified after review: keep the existing actions and
add optional `model`/`effort` choices only where an inference is actually started.
Example additional arguments:

```json
{"model":"chatgpt-desktop/gpt-6","effort":"xhigh"}
```

The action and its existing owned destination already identify the base settings
source: chat_input uses its selected Chat, an existing Native work uses its
thread, and fresh Native creation uses config. Do not expose a second chat/native/
config/host_default selector or duplicate the destination in another object.
The earlier model_settings/source request proposal is superseded before any
implementation; it has no compatibility consumer. Settings origins remain
internal facts returned with the effective request, not another routing decision.
An agent may choose an explicit option within the authorized workflow and user
model/cost constraints; no automatic selection agent or fixed test model is added.
Changing only the model preserves an inherited effort only if the resulting pair
is valid; otherwise require a suitable explicit choice or return the incompatibility.
The observed host-default/auto path remains an experiment, not a new public input
mode without a production caller. Do not interpret omitted options as requesting
auto. Where an actual caller requires host defaulting, first specify its distinct
semantics and model/cost limits rather than adding it as an implicit fallback.
Pro requires an explicit user or authorized workflow choice; automated
qualification does not select it without a separately authorized test.

These choices contain execution metadata, not environment-variable or prompt
injection. For Chat source, use the app selector after identity/branch, conversation
metadata query, history and catalog admission. Resolve host slug/effort without
guessing an ambiguous inverse catalog alias; retained canonical route metadata is
used only when actually established. For Native source, use thread/read and the
existing turn/start/settings contracts; keep Responses request model validation.
The minimal app capability belongs in existing chat_adopt/preparation and submit,
not a forwarding-only settings engine or a second renderer policy implementation.

Return/persist the effective model/effort and host mapping with per-field origins
(explicit, chat, native or config) in the existing intent/result. They are consumed
by retry reconciliation, route validation and the caller's display/acceptance.
If future callers admit host-default requests, they retain unresolved defaulting
status until the actual request/response is observed; auto is not a proved final model. Existing
account/branch/cursor/owner records already bind provenance: add no duplicate
signatures, hashes or capability tokens solely for settings traceability.

Reuse current exact message-ID generation, prefix admission, pending-lane fencing,
budget ledger, result reconciliation, Stop and original-source reading. Remove
chat_input's unconditional config selection when replacing it; do not retain a
silent fallback branch. Native create/resume/fork must distinguish initialization,
inherited restoration and explicit provider/model transition instead of sending
config defaults for every action. Inspect existing restoration paths before
changing them; unchanged valid Native restoration must remain unchanged.

New unsubmitted calls use the new default policy only after implementation and
versioned delivery. Existing saved intents retain their original fingerprint,
message ID and phase; do not infer historical settings that were never recorded
or modify their admitted content. Old unknown/submitted calls remain result-only
reconciliation. Rollback preserves their journals and must not dispatch them again.

Required implementation proof: exact multiline/CRLF input; Chat selection versus
conflicting global config; metadata-ready versus unready restoration; Native
resume/fork/mode-only preservation; explicit supported/unsupported pairs and
omitted versus none; changed identity/branch/selection during preparation;
concurrent different inputs; same-call retries across config change/restart and
unknown delivery; Stop and original tool/approval continuation; same-work
ChatGPT/Native history and running TUI. Mutate the preparation/default-selection
fault in disposable source and prove rejection. Use actual app requests plus
original result readback for model/effort; synthetic/Native unit tests alone do
not qualify Android UI synchronization or an already-running model change.

Saved-input completion reconciles the exact admitted user UUID and parent,
then a unique saved descendant branch. The app may remove a hidden final terminal
when storing a follow-up. Before submission, derive a delivery cursor at the
preceding public final from the same verified complete source read, skipping only
finished, end-turn hidden assistant final nodes. Submit beneath that public final
and persist both the original source cursor and delivery cursor. Reconciliation
requires the exact delivery parent and unchanged delivery prefix; expose both
parents and the resolution. This needs no extra source request or hidden content.
Old receipts lacking that binding retain their original strict check; never infer
or rewrite a missing binding after execution. Other changed parents remain errors.
A hidden assistant final terminal may
follow the public final: preserve its unavailable placeholder and source cursor,
and identify the preceding public final as the delivered result. Skip only
finished, end-turn assistant nodes explicitly marked hidden in the final channel;
never treat analysis, unfinished/unsupported content or commentary without a
public end-turn final as completion. Read no hidden content. Ambiguous branches
remain errors; a cancelled input without that public final
is not accepted as successful output.

At a completed idle native boundary, `transfer` takes exact native history and
context cursors, selected reasoning/return path and a grounded public summary.
It fences the source and offers a generation-bound receipt. `transfer_accept`
must echo that receipt and unchanged source cursors before the next start may
use the target route. Offering is distinct from destination acknowledgment and
from successful subsequent development. The native thread, canonical record,
assignment and workspace are retained; source summaries are contextual data.

The initial client coverage is the owned Linux ChatGPT app and ChatGPT web with
the same authorized connector/account. Direct primary work requires no separately
acquired Linux host; delegated/managed work and app-backed source reading retain
their actual app dependencies. Bind the current RPC/native invocation as the
return channel without assuming MCP supplies a trustworthy Chat ID. A selected
saved Chat ID additionally needs account/source verification. Native questions
remain upstream approval/input requests; the coordinator forwards real user
answers and cannot approve them itself.

A disconnected origin retains the result and delivery state independently of
work completion. Retry/retrieve the same logical invocation where the client
supports it; never fabricate a new user message or claim an idle web Chat was
automatically resumed. App-backed continuation needs the selected original Chat,
verified ownership and correlated source receipt; qualification must demonstrate it. A client
that cannot accept background delivery shows pending delivery and a source-bound
retrieval handle. This retains long-running delegated work without per-turn
supervisor inference or an invented MCP/Work quota route.

**Saved context and project selection.** New managed workflows use saved Chats in
an explicitly selected compatible private project; Qualification fixtures select their own
project/configuration rather than changing installed history defaults. Resolve a
configured project ID first, otherwise show exact owned name matches and require
selection on ambiguity. Project creation is an explicit configured action with
durable intent/readback, never an inference-triggered blind retry. Reuse the same
related work Chat across ordinary follow-up turns. A completed native turn alone
does not renew Chat. Replace it only through acknowledged handoff, an explicitly
selected new source or a verified incompatible context/model/account boundary.
Do not impose an arbitrary turns-per-Chat limit while detailed capacity policy
is deferred. Keep legacy temporary mode available under its declared limitations;
it cannot establish saved-source continuity/completeness acceptance. Source Chat
loss does not erase the native canonical record or permit uncertain replay.
Titles describe project/task/role for users; stable IDs handle duplicate titles.
Title readback compares the complete conversation graph and message values,
excluding only each message's volatile `update_time`. Preserve raw before/after
evidence separately; changes to text, metadata, IDs or branches remain failures.

**Reading and input.** Provide one account-bound conversation service with CLI
chats list/read and MCP codex_chats list/read facades, independent of native work.
List supports project/title/time filters, opaque pagination and stable IDs; title
matches may return several candidates. Read selects conversation ID and branch
tip, then a public message range with a cursor bound to account, conversation,
branch and source prefix. Return ordered IDs, parent/source references, public
roles/content, continuation cursor, observed time and completeness/unavailable
reasons. Never represent a truncated/unsupported message as a complete read or
export hidden reasoning, credentials or private tool capabilities, including live
controller/return tokens present in a visible tool result. Mark redacted ranges
and preserve their source identity; source import cannot create a control grant.
Source changes invalidate the affected cursor; a later appended message does not
invalidate an unchanged covered prefix. Partial reads are explicitly partial.
Reading creates no native work, model request, input or controller. Context import
records selected source/range separately from user instructions in native history.

Follow-up submission requires an explicitly selected Chat/branch and logical
request ID. Reuse a related, settled, same-account Chat through the app API when
its identity/capabilities are verified; do not require it to have been created by
this bridge. Preserve existing branches/drafts and never silently adopt an active
unowned stream. Active owned work uses native follow-up/steering semantics where
supported; otherwise show queued/waiting state. An older settled Chat may receive
explicit user-authorized input without taking native control. Direct/managed
transfer additionally requires reconciled calls/jobs, current history/context
coverage, old-controller fencing and successor acceptance. Input, read and
takeover have distinct receipts; sending input alone cannot grant control.
Work listing exposes the bound execution home and saved source conversation ID
so a follow-up can select the existing work. A claim at a completed idle boundary
preserves that completed boundary while rotating its controller. Route transfer
then uses the existing history/context coverage and destination-accept handshake.
Transferring a settled managed lane detaches its passive app hooks without
submitting a Stop to an ordinary Chat stream or creating a replacement Chat.
Releasing an absent owned host/bridge is an idempotent no-op and must not acquire
an app after a service restart. This proves release of ownership only; missing
host observations still cannot establish stream or native-process termination.

**Agents and coordination.** The initiating Codex or current Chat reasoning context
coordinates by default; an extra managed coordinator is an explicit choice with
separate model cost. Provider assignment parents are not native child ancestry.
The approved agent extension above defines that boundary. Reuse actual native child/reviewer threads and record assignments,
findings, corrections and integration in native histories. Required orchestration workflows:
implementation -> independent review -> correction -> accepted integrated result;
and two independent implementation assignments followed by review/integration.
The coordinator releases dependent tasks after checking their results, not merely
after stream silence. Review findings identify evidence and unresolved issues;
the implementer returns corrections and the coordinator decides acceptance.

Parallel assignments may use the same repository and physical worktree,
including disjoint simultaneous writes and version-checked same-file edits,
under the shared-repository Native concurrency contract below. Disposable
worktrees are optional, not mandatory. Coordinators integrate conflicting
candidate changes using verified resource/version preconditions and record
actual integrated checks. Parallel requests specify a finite concurrency
and inference/resource budget; admission respects native/host capacity and exposes
queued tasks. Do not invent a fixed agent roster or run extra reviewers as an
unpriced background policy. Serial execution remains available. Stop(task) fences
only that assignment; Stop(work tree) enumerates/cancels admitted descendants and
reports each native/job/host confirmation. Partial failure leaves independent
tasks/results inspectable; unknown children prevent whole-work success. These
are product capabilities, not authorization to spawn development collaborators.

**Shared-repository Native concurrency (required, including future orchestration).**
A workspace is a verified working directory and authority boundary, not a
repository-wide writer mutex. Independent top-level works, an orchestrator,
reviewers and child workers may share the same repository, physical working tree,
directory or nested paths. Work creation, attachment, reading, reasoning,
status, reporting and coordination do not acquire a workspace write lease.
Separate Git worktrees are an optional isolation choice, not a condition for
parallel work. A work's sole current controller remains exclusive to that work;
it is not the sole controller of the workspace.

Upstream owns Native execution, approval, thread history and original results.
Provider owns resource admission and effect settlement only for integrated
works and tool calls it actually mediates; Manager offers optional task and
orchestration conveniences. Core and release-builder own Termux compatibility
and authenticated installation, not Native concurrency policy. A mediated
tool boundary must enforce actual write scope and safe publication. Provider
claims do not protect ordinary upstream CLI/TUI or outside editors: those
paths require a proven supported upstream or scoped execution boundary, not
optional coordination code injected into upstream ToolRegistry. Retain
workspace and permission checks and authorized write capabilities; mere
workspace overlap cannot deny work creation.

**Conflict classes and required outcomes.** Apply every relevant row to a
compound operation. The strongest actual conflicting resource rule wins; absence
of a known conflict is not proof that an opaque command is harmless.

| Case | Concurrent situation | Required behavior |
| --- | --- | --- |
| NC-01 | Different work IDs in one repository, worktree or cwd; independent roots or parent/children | Admit both; per-work controller, history, return destination and budget remain distinct |
| NC-02 | Read/read, including orchestrator inspection, source browsing, reviews and status | Run concurrently, regardless of equal or nested paths |
| NC-03 | Live read during an unrelated write | Allow; label the view live, not an immutable snapshot |
| NC-04 | Read requiring repeatable results during any write | Pin a revision/snapshot or verify a consistent set of content versions; retry or report unavailable consistency if neither succeeds, without blocking the entire repository |
| NC-05 | Known writes to distinct files in the same directory, module or worktree | Run concurrently with independently enforced write sets |
| NC-06 | Known writes to separate portions of one file | Permit parallel preparation and version-checked, nonoverlapping merge so both edits survive; order final publication, never permit two blind writes |
| NC-07 | Same file, overlapping hunks or incompatible final content | Report conflict and require reconciliation; neither writer silently overwrites the other |
| NC-08 | Same proposed content/effect and identical scoped operation identity and payload | Reuse the settled result only for the same authorized operation; a distinct call must validate current versions, even if its content matches |
| NC-09 | Concurrent creation of the same name, removal versus edit, rename versus old/new-path access | Coordinate the actual path and affected parent directory entries; reject or reconcile conflicting effects |
| NC-10 | Sibling creates/updates beneath a common directory | Permit independent names; coordinate the colliding directory entry, not the whole directory merely because its metadata changes |
| NC-11 | Directory rename/delete, recursive changes, glob writes and changed permissions/ownership/attributes | Cover the actual descendant set and metadata effects; re-evaluate against concurrent uses |
| NC-12 | Symlinks, hardlinks, relative paths, `..`, bind mounts and aliases | Compare verified physical targets, open inode identities and directory entries; revalidate at effect time against alias/path-swap bypasses |
| NC-13 | Nested repositories, submodules and distinct worktrees of one repository | Treat overlapping files as shared resources and Git metadata according to its physical owner, not only textual paths |
| NC-14 | Git working-file edits concurrent with `git status`, diff or read-only log queries | Permit; indicate a live view unless the caller requests a pinned snapshot |
| NC-15 | Distinct worktree indexes/HEADs and unrelated Git refs | Permit independent operations where actual metadata/resources do not overlap |
| NC-16 | Shared Git index, refs, locks, branch checkout, reset, merge, rebase or commit publication | Coordinate the specific index/ref/affected files; validate expected HEAD and index, and never include unrelated staged or working changes in a commit |
| NC-17 | Independent builds, tests, code generation and temporary outputs | Run concurrently with per-work output directories and source-version evidence; do not serialize a repository just because two builds are running |
| NC-18 | Shared compiler cache, package database, generated destination or artifact name | Respect that store's own concurrency semantics or arbitrate that named output; distinguish cache lock from source-tree lock |
| NC-19 | `TMPDIR/aaa` and `TMPDIR/bbb` for unrelated tasks | Admit both; a task owns only its assigned subtree, never `TMPDIR` or another task's subtree |
| NC-20 | An intentional shared temp path, cleanup or recursive deletion | Coordinate the exact shared target and any live descendants; do not remove unknown or other-owned outputs |
| NC-21 | Independent reads/writes in different repositories with a common path ancestor | Admit except for actual common files, directory entries or external resource effects |
| NC-22 | Environment HOME/config, persistent credentials, app host, installed runtime and shared sockets | Apply their existing distinct security, process and publication leases, not an incidental cwd/repository mutex |
| NC-23 | Network publication, release tags, deployments and other irreversible external side effects | Require the corresponding target-specific ownership, authorization and idempotency; filesystem disjointness grants no extra authority |
| NC-24 | Opaque shell command or tool with an unprovable mutation set | Stage it in an owned disposable view and verify/apply the resulting changes, or enforce an OS-level write scope; if neither is available, serialize only that effect against potentially conflicting writers |
| NC-25 | Unmanaged external CLI/editor changing the same files | Do not claim invisible writers are prevented; use filesystem/version checks and applicable OS leases, and surface uncertainty when a safe commit cannot be proven |
| NC-26 | Multiple inputs/tools within one Native work | Keep one current controller; admit independent effects according to the same resources and Native/host capacity, not one global workspace writer |
| NC-27 | Different Native works with `waiting_native`, `stop_requested` or `reconciliation_required` records | Do not treat the record itself as a workspace lock; retain only unresolved *specific effects/resources* until their ownership and termination are reconciled |
| NC-28 | Stop, crash, detached Chat, restarted host, stale PID or cancelled worker | Fence its future effects and reconcile observed processes, in-flight calls and resource leases; neither a timeout nor a terminal Chat alone proves a physical writer stopped |
| NC-29 | Multiple simultaneous conflicts, retries and partial application | Order overlapping resources deterministically, preserve idempotent receipts, surface wait/conflict owners and avoid partial-success claims |

Each NC-01 through NC-29 row is an independent, testable acceptance case.
Compound commands satisfy every applicable row; acceptance must demonstrate
admission of safe parallel effects and rejection or reconciliation of unsafe ones.

**Minimal resource-aware execution contract.** Resolve a proposed Native effect
to an access mode (read, bounded write or opaque), its verified physical resource
set, a work/operation identity and any required base versions. Read and write
authorization are distinct from scheduling. Reads with no requested consistency
pin may observe live changes; repeatable reviews and tests use a pinned view
when needed. For bounded writes, take only the necessary short resource leases,
validate preconditions immediately before the visible mutation and commit the
result with atomic replacement or an equivalently guarded application. Use
content/version comparison and a three-way merge for independently prepared
same-file changes when feasible; return a genuine conflict otherwise. An
operation touching multiple outputs must identify partial effects and restore
or reconcile them before claiming completion. Resource ordering must avoid
deadlock. Admission must expose the precise conflicting effect/resource and an
actionable wait, join, handoff or isolation option, without leaking unrelated
private content.

Resource identity distinguishes file contents/inodes, parent/name directory
entries, path or inode metadata, Git index/refs, generated output/cache and
external publication targets. One operation can touch several such identities.
An idempotency identity is scoped to its authorized work/operation and exact
payload; neither matching text nor a reused caller-provided string transfers
another work's result or authority.

A model's intended file list or a tool annotation alone does not enforce a
write boundary. Arbitrary commands may create files, follow links, mutate Git
metadata, call out to other programs or write outside cwd. Such effects require
actual native/OS mediation, an isolated staging view with validated publication,
or a bounded fallback lease while the uncertain command runs. That fallback
may defer a *conflicting effect*, but must not prevent unrelated works from
starting, inspecting, preparing edits or using their own temp subtrees. No
unverified file-level concurrency guarantee is made for unmanaged writers.
For an ordinary single Native tool request without an explicit durable wait or
queue action, a conflicting resource lease must settle that original logical
request with a persisted `executed=false`, retryable conflict response **before**
the effect is forwarded. It must not silently execute later after the calling
observer has timed out or departed. A caller may inspect current effect status
and submit a new logical call after settlement; repeating the rejected call ID
must return the original rejection, not dispatch a delayed effect. Deliberate
durable queues have distinct explicit registration/consumption receipts.
If a shared working tree is already being changed, version-checked publication
must detect intervening mutations rather than overwrite them. A separate
worktree remains a useful opt-in for high-risk opaque effects, never mandatory
for routine independent edits in the same physical tree.

Allocate each new work an owned `$TMPDIR/codex-native/<work-id>/` subtree
(and distinct generated-output paths where necessary) without changing its
actual selected repository cwd. Resume retains referenced outputs and may use a
generation-scoped child directory; creation verifies exclusive ownership and
non-aliasing paths. Work-local commands receive the owned subtree as their
temporary root; no work claims `$TMPDIR` or the common `codex-native` parent.
Cleanup only removes proven inactive, owned subtrees. Generated build targets,
test scratch and temporary Git state must not alias another work's outputs
merely because they use the same repository. Cache sharing is permitted when
the cache's actual lock and publication protocol supports it. Existing Core
execution-home directory leases,
host stream ownership, authentication and installer/rollback fences are separate
and must not be weakened by workspace concurrency.

Orchestration is an optional future consumer of this shared Native work service,
not a prerequisite for the concurrency contract. Its coordinator can inspect
the same repository while multiple independent child workers read or edit
different or overlapping portions of the same worktree. Each child has its own
work/turn/operation identity, scoped authority and budget. Parent ownership
does not override individual mutation conflicts; stopping one child does not
cancel unrelated writers. The coordinator can review candidate changes and
integrate them with the same verified publication path. Same-file overlapping
edits may be prepared in parallel and integrated sequentially. A second
scheduler, compulsory extra model call or mandatory disposable Git worktree is
not introduced by this requirement.

**Compatibility, recovery and acceptance.** Replace Provider's
`native_workspace_writer_busy` ancestor/descendant work-record rejection with
effect/resource-aware admission, settlement and recovery for mediated Native
tool calls. Unmediated upstream execution is not thereby protected. Both
explicit and default workspace selection obey one policy. Existing unconfirmed work records remain inspectable
and their unsettled *effects* fenced; they cannot veto unrelated reads, creates
or disjoint edits indefinitely. Legacy records without an enforceable resource
claim must be reconciled against actual native/process effects before inferring
a writer, not assumed to own the entire workspace forever.

Qualify on real Native entrances and the same physical repository/worktree:
two readers; live and pinned read against a writer; two disjoint file writers;
same-file disjoint and overlapping hunks; create/delete/rename/metadata conflicts;
symlink/hardlink/`..` aliases and nested repositories; simultaneous Git index/ref
updates; builds with distinct and shared caches; sibling temp subtrees under a
busy parent; broad/opaque commands; external CLI changes; one parent with several
children and an observing coordinator; unknown/stopped/reconciliation work;
duplicate calls, Stop/restart, partial failure and publication races. Verify both
positive parallel admission and negative collision enforcement, actual content
and source revisions, original Native receipts and no loss of existing controller,
permission, host, account, installer or rollback protections. No simulated
success or task-phase-only proof substitutes for these executed paths.

Parallel assignment limits apply to siblings under an explicit parent work;
independent top-level works do not share an anonymous global queue. Each root
has its own declared budget; child budgets remain within the parent's caps and
same-parent uncertain work still reserves its slot until reconciled. Preserve
confirmed completed/stopped phases across shutdown, restart and owner discovery
while fencing/rotating old controllers. Other unsettled phases require explicit
reconciliation; a past native turn alone is not a running top-level assignment.

A settled current-reasoning work may receive an explicit elapsed/native-call budget extension
through `codex_sessions claim` with a stable call_id and exact history/context
cursors. Supply unchanged concurrency/app-start caps, a native-call cap no smaller
than its previous cap, and a finite elapsed cap no smaller than its previous cap,
measured from the original assignment start. An extension increases the total
allowance; it never resets used calls or starts a new budget period. Preserve that start,
all admitted-call counters, role, dependencies and native/work identity; never
reset consumed resources implicitly on follow-up or recovery. Child extension
also requires the current parent controller and stays within its budget. Refuse
extension while native jobs are unsettled. Managed reasoning refuses this
extension until its app-side budget ledger can be reconciled consistently; a
claim without budget retains its existing behavior and does not renew a budget. An explicit
claim may reload its recorded native thread when a fresh native observation
reports notLoaded; this starts no inference. Preserve the exact workspace and
routing, recheck scope/history/context after reload and record mutation intent
before sending it. A lost reload reply requires reconciliation, not replay.
Passive status/history do not load a thread merely to report readiness.

Registered web MCP actions must agree with the actual producer's `tools/list`
input schemas, including action enums and cursor arguments. Connector refresh
qualification compares those schemas, invalidates the app cache and exercises
an actual read through that connector. Matching tool names alone is insufficient;
a mismatch prevents readiness/model admission and reports the schema cause.
Queue registration acknowledgment remains distinct from native consumption.

Plugin replacement within an authorized task preserves the original display name
for the replacement. Retain the prior registration as `<original name> (레거시)`
and verify that rename before creating the replacement under the exact original
name. Resolve an existing legacy-name collision without overwriting another
plugin. An already created replacement keeps its identity and receives the
original name; do not create another copy. Display-name transfer never changes
either registration's identity, audience, credentials or outstanding calls.
Creation uses the verified current MCP service/tunnel accessible to the intended
app account, with installation, connection and schema/readiness verified
separately. A refresh error alone does not establish that a replacement or another
tunnel will resolve its cause.
The [Linux app operating guide](../../providers/chatgpt/LINUX_APP.md#플러그인-생성과-교체)
owns the operating steps and partial-failure recovery.

**Management.** Extend the optional Provider binary's existing status/view/reveal/
stop/serve mechanisms with one host lifecycle facade (ensure/status/open/stop),
shared by management CLI and MCP. Configuration stays in Provider config with an
explicit owner-bound state root. Reuse Rust OwnedHost/CDP/Remote/AppHost and native
Pool/Broker; development Python/Bun helpers supply capability evidence and import
compatibility, not another production manager. Validate PID/start/executable,
lease, account and app binding revision before reuse. A failed readiness probe
does not authorize restarting a live owner. Keep service, host, connector/tunnel,
native turn/jobs and optional viewer state separately observable with freshness.
Status/read/open do not launch inference; status must not lazily acquire an absent
host. Explicit ensure may start configured assets, without implicit downloading
or login. Open/reveal displays the selected existing owner/Chat. Stop names its
scope, preserves profile/Chats/user files and cannot kill an unrelated/shared host.
Reconnect/connector refresh restores transport only after inspecting pending
receipts; it does not resend tasks. MCP Broker owns admission and settlement for
McpBridge-mediated Native calls. Operator forwards those already-reserved calls
without reserving them again as external AppHost effects. Only local-function
AppHost calls use Operator external-effect admission/settlement; work budgets
and Native ownership validation still apply to both return paths. Missing/expired/changed app APIs report their
specific unavailable capability. Viewer/login interactions remain optional and
distinct from backend work; cosmetic noVNC changes are not a prerequisite.

**State, observation and proof.** Expose native and host status separately. Running,
waiting for tool/user/previous completion, reconciling, completed, stopped and
unknown carry an observed time/cause; silence or missing output is not completion.
Completion requires native final commitment, owned-call/job settlement and durable
completion state. Observe events locally without a supervising model turn; send
one terminal notice to the original live session, plus genuine user questions.
Notification delivery has its own acknowledgment and cannot determine work success.

Preserve source-bound history cursors. A qualified source reader may cache verified immutable prefixes and
fetch new pages, but must detect truncation/rewrite/branch changes, and must not
replace canonical history with a second transcript database. Qualification accepts
explicit candidate, workspace/home, fixture, account/host and connector inputs;
historical paths remain evidence, not implicit current defaults. Offline source
gates and paid/live trials are separate. New live batches declare submissions,
responses, bounds and expected cost first; audit diffs, checks, original CLI,
native records and saved Chats after the batch. Compare equal tasks and count
setup, coordinator/worker/reviewer, context transfer, observations, retries and
failed attempts. No quota or billing improvement is inferred from authentication.

#### Fixed annotations and idempotency work

The existing readOnlyHint=true, destructiveHint=false and openWorldHint=false
annotations are intentional immutable user requirements across all tool definitions,
including added common interfaces. The MCP base catalog and Native management
catalog use one declaration policy, without a per-tool read-only exception.
Do not change them or reopen their selection as a product-admission prerequisite.
Only idempotentHint varies by the actual operation contract and implementation.
The idempotency contract defines logical-request scope, duplicate/unknown-result
behavior and shared source logic; fault qualification precedes each declaration. Earlier
preliminary recommendations are research, not authority over the qualified policy
below. The hint neither adds a global queue nor proves host parallel scheduling.

The goal is consistent logical-request identity, duplicate handling and result/
completion recovery across the tool surface. Reads retain freshness. Effectful
retries must share the original operation or recover its receipt, reject changed
payload under the same identity and reconcile unknown outcomes without executing
again. A deliberately new operation may perform the same input again. Define
generation/ownership and retention boundaries explicitly; idempotence does not
guarantee identical read output or permission to cache status indefinitely.

Implement reuse/coalescing at the relevant operation boundary; do not introduce a
global serial queue that blocks unrelated work. Preserve native authority, work
capabilities and requested read/write behavior. An idempotentHint declaration alone
does not implement these guarantees or establish client-side retry/parallel policy.
CURRENT records exact candidate sources, limits, decisions and observed acceptance.

The implementation contract is scoped logical identity plus a payload
fingerprint and durable intent/result. Use existing call_id, work/controller
generation, native request/turn and Broker token identities. One operation's
concurrent observers share its dispatch/settlement; unrelated operations are not
serialized globally. Persist intent before dispatch and settled results before
acknowledgment. If dispatch/result is unknown, return reconciliation-required and
inspect the original native effect; never create a replacement operation silently.
Process restart invalidates live controller/host leases without deleting receipts.
Per-work lifecycle admission orders start/compact/claim/takeover and native-answer
forwarding against Stop. Stop fences first, drains an already admitted native
mutation before confirming interruption and blocks new controller acquisition
until settlement. It does not wait behind report/finish observers. A late native
request from a stopped in-flight start is not a new user recovery turn.
If the exact owned thread is `notLoaded`, Stop may load that same thread with
its verified workspace and routing configuration solely to observe and terminate
jobs. This submits no user turn or inference. Unavailable history, changed scope
or failed loading remains unconfirmed. Assignment evaluation requires the current
controller, exact result cursor, decision and findings text. Managed completion is
the native completed turn and original final, not an explicit `codex_finish` call
in its tool history. A final missing from one history page is not proof of absence.
Retain receipts for their work/generation; reject capacity exhaustion explicitly
instead of evicting an unresolved operation. Retirement/cleanup is explicit and
must preserve recovery/provenance needed by admitted work.

| Tool family | Identity and duplicate behavior |
| --- | --- |
| Catalog/history/status/session list | Fresh observations; source-bound paging, no effect receipt/result cache |
| native_tool/codex_execute | Existing token plus logical call_id and name/input/report fingerprint; one Broker invocation and original result, including detached observers |
| codex_report/codex_finish | Work-scoped call_id and text/final fingerprint with current-controller admission; receipt intent precedes the one message emission. IDs remain reserved across generations; a new intended report needs a new ID. Retry observes that item and resumes applicable completion settlement; final-item presence alone is not completed work |
| Session create/resume/fork | Existing caller call_id plus action/payload; durable creation intent/result; unknown native creation is reconciled, never dispatched again |
| Session start/compact/claim/takeover | Add optional call_id for compatible exact retries, scoped to work/action/controller. New common clients supply it; old unkeyed clients retain conservative fenced behavior. Unknown mutation requires reconciliation |
| codex_attach | Offered generation plus accepted source receipt; repeated acceptance observes current context, cannot acquire a different generation. A starting_native generation returns pending_native_context; exact Native history-not-ready observation returns pending_native_history. Both are pending reads with accepted=false and no receipt; observe/retry without restarting the task or executing before its real context is available |
| codex_answer | Work/controller plus native question request_id and answers fingerprint; one answer forwarding; unknown response is not permission to answer again |
| codex_native_summary | Common clients supply a stable optional call_id, scoped to work/controller; record the admitted native inference round and summary fingerprint before one delivery. Legacy unkeyed input remains scoped to its admitted round. A duplicate keyed reply cannot answer a later round; Chat replacement remains separate |
| codex_handoff | Existing work/call_id and full cursor/context/summary fingerprint; coalesce intent/generation transition, recover original receipt; uncertainty never creates another successor |
| codex_stop | Observe and converge current owned native/job/host state on every call. Do not cache an old stop confirmation or block Stop behind a waiting report |

An exact `codex_execute` retry may retrieve its already committed Broker result
without rediscovering a live Native socket. It still requires the same current
work/controller and name/input/report fingerprint, and any existing App identity
binding. This reads the retained result only: it grants no new execution,
submission, claim or controller ownership. The response marks fresh history and
context observation unavailable and supplies no new cursors; callers use the
existing history/status interfaces when fresh Native observation is available.
Missing or unsettled calls remain on the ordinary Native admission path.

Work records are authoritative; their index is derived/recoverable, not an
independent source of work existence. A virtual MCP round's durable settlement
intent must remain aligned with memory across Broker failure, including uncertain
commit. Do not roll memory back alone or admit a new round on ambiguous state.
Reconcile recorded Broker results before completing that intent, without replaying
tool effects or submitting another Chat request. Apply selected hint declarations
only after operation-scoped fault qualification; CURRENT records the completed Rust application
and its exact scope. Implementation alone does not justify declaring every
mixed-action tool idempotent.

The following declarations require actual operation-scoped fault qualification
before application in the Rust Provider. Source application follows the retained
effect/receipt/restart checks and still requires changed-candidate proof. It does not change the other immutable annotations or promise a speedup.
The original table omitted the common read/host tools: codex_chats follows the
same fresh-read contract, while codex_host retains false for its mixed surface.
The retained JavaScript adapter keeps its own declarations and exact-input proof.

| Target idempotentHint | Tools / reason |
| --- | --- |
| true, subject to operation-scoped fault proof | native_catalog, codex_history, codex_tools, codex_status and codex_chats: fresh observations; native_tool and codex_execute: scoped original Broker result; codex_report, codex_finish and codex_handoff: mandatory keyed intent/settlement; codex_answer: keyed native question; codex_attach: offered generation/source-receipt acceptance |
| false for the compatible mixed surface | codex_projects: list plus account/project selection without a generation-scoped operation key; codex_host: status plus ensure/open/stop with separate ownership/lifecycle effects; codex_sessions: list plus keyed and legacy unkeyed mutations; codex_native_summary: legacy unkeyed round-scoped input remains; codex_stop: fresh convergence over current work, without a request generation in its old input |

Known validation rejection occurs before intent/dispatch. For virtual MCP results,
Broker validation and lane-intent preparation share the same commit admission
section: a new rejected proposal leaves the previous lane authoritative, while a
failed/uncertain intent or Broker commit keeps the prepared lane fenced. A later
same-operation observer can settle confirmed results without replay. Existing
receipts preserve their context when result acknowledgment is committed. The v2
work index retains pending registration, repairs v1 omissions on first migration
and maintains a settled v1 mirror for controlled downgrade. Neither journal
format nor controller capability is permission to reactivate a live old writer.

The expected benefit is fewer duplicate effects, duplicate waits, avoidable
retries and unsettled completion errors, with less related load. This is an
unmeasured hypothesis; it does not establish lower model latency, network failure,
total token use or subscription debit. Independent qualification measures the
affected failure/overhead boundaries while verifying the other annotation
values remain unchanged.

#### Bridge qualification input and evidence contract

[qualification.schema.json](../../providers/chatgpt/qualification.schema.json) defines
termux-codex-bridge-qualification-v2. It is an input contract, not an executable
runner or authorization to launch live work. Qualification entrypoints accept an
explicit --inputs manifest without changing already recorded evidence.
No implicit historical root/TMPDIR/tunnel/previous-frontend path is a valid
new input. JSON schema validates shape; the consumer must also perform the
semantic checks below.

| Input | Required semantics |
| --- | --- |
| run_id, cases, routes | Unique owned run; explicit contract-enumerated cases and Codex/ChatGPT-attached/ChatGPT-submitted route list. Attached uses inference=attached/MCP; Codex/submitted use inference=submitted, with their declared inner return path. Earlier v1 manifests remain original evidence and are not silently accepted as new inputs |
| candidate | Base Git commit plus complete current source-input manifest/hash, including dirty/untracked build inputs; immutable binary, embedded bridge and effective config hashes. A commit ID alone cannot identify this working tree |
| native | Exact Core/frontend hashes and qualified upstream version/source; canonical isolated execution home and workspace, never the user's active home/project |
| fixture | Frozen project/tasks/expected checks/seed manifest, including task ordering and per-role ownership. Preparation creates separate short TMPDIR homes/workspaces, preserves socket path bounds and seals inputs before any model action |
| environment | Explicit TMPDIR/artifact roots, private host/account/connector descriptor references and app version/binding hashes where needed. Revalidate actual PID/start/image/lease/account and keyed readiness at use; saved ports/URLs/hashes alone are not readiness. Do not include credential values |
| limits | Offline/synthetic uses zero Chat submissions/responses; live declares finite submission, response/native-call/time/concurrency bounds and references existing authorization. No automatic Pro or GPT-5.6. Reuse prior scope/ownership approval rather than demand a fresh approval artifact |
| return | Original private destination reference and terminal-notice choice; no per-turn supervisor model review. Verify destination acknowledgment before detaching a live batch |

Here max_app_starts counts app completion submissions (stream request starts),
not GUI launches or model-free prepare/read operations. Track physical host starts
separately. max_model_responses counts observed response attempts/continuations;
unobservable backend usage and actual billing remain unknown, not zero. Budget
admission occurs before a new submission; exhaustion fences further submissions,
preserves current calls/jobs and reports actual stop/reconciliation. It never
creates another operation ID for an uncertain request. All referenced files have
canonical absolute paths and verified hashes; semantic preflight rejects symlink
aliases, overlapping live state and model/effort/approval drift before effects.

Prepare is model-free: inspect/hash configured inputs and ownership, validate
capabilities and produce a sealed resolved-input manifest. Do not acquire an app
for direct-primary cases unnecessarily. Launch is a distinct exclusive intent;
an existing launch/outcome is reconciled, never duplicated. Once launched, the
deterministic runner consumes prepared tasks/events/receipts without supervising
Codex model turns, and emits the final same-session notice. A real unanswered
user question may request attention; stream silence cannot advance the sequence.

Evidence uses the sealed input digest and per-work/task/role/source/controller/
turn/call identities, with UTC and monotonic start/end timestamps and durations.
Record native calls/results/patches/checks, original CLI and canonical finals,
saved Chat/message/branch readbacks, original destination and notification ack,
fault boundaries, duplicates, retries, unknown states, app submissions/observed
responses, host starts, CPU/RSS/bytes/context and all setup/failed-attempt costs.
Keep per-turn timing locally; one final batch audit compares actual diffs/checks
with the original outputs. A failed or zero-case required gate is unfinished.

Common work/per-lane routing and generation dispatch precede model-free
reading/input/return, host management, serial and isolated parallel agents,
changed-path local/synthetic proof, and bounded necessary live/efficiency trials.
Include idempotency recovery cases before applying target hints or claiming
combined acceptance. Product/Android lifecycle, signing, distribution and
activation are separate qualification boundaries; preserve unique work before
retiring an old path. Idempotency repairs need not await delivery/retirement.

The common Provider interface reuses native TUI/CLI thread/child navigation,
public progress, questions and cancel controls; the optional Provider
work/chats/host facade supplies selection,
source reading, scoped Stop and model-free status. A new bespoke agent dashboard
is not required for the selected workflows. Completion uses the existing
source-bound notification mechanism with an actual receiver acknowledgment.
These capabilities remain usable without enhanced TUI or Manager; physical
UI/composition and packaging retain separate proof.

#### Agent and orchestration extension direction

The entrance-equivalence contract owns the extension boundary and order. Earlier recorded
orchestration evidence qualifies Provider assignments/dependencies/evaluations
and isolated work at its recorded scope, not arbitrary external-agent membership
in the native /subagents tree or same-Chat independent host inference. Preserve
those existing capabilities and original costs while qualifying actual native
spawn/child lifecycle and external-result return separately. No additional model
or automatic development collaborator is required by this product contract.

### Optional managed Chat operator controls

The optional managed Chat operator controls require an explicit selected
execution home, Native work identity, owning user and transport capability.
Model adapters and MCP are separate choices from Native work control; when
a before-start app local-function transport is supported, its use must be
explicit and must preserve the same Native controller, receipt fencing,
upstream execution, canonical history and correlated original results.
Transport selection submits no extra primary inference, does not bypass
approvals and must not silently replay an uncertain request. Neither a
synthetic integration pass nor a configured mode is proof of safe live app
delivery. Optional paths remain replaceable only after their actual distinct
consumers and capabilities are qualified; Core acquires no Provider or operator
policy dependency.

Model connection, native-work control and shared connection/session management
are separate responsibilities, not mutually exclusive products. An adapter
already manages Chat connections; the presence of management code alone does not
define operator execution. One optional Provider may expose a Responses model
interface to Codex and an authenticated MCP work-control interface to ChatGPT,
sharing bindings, receipts and recovery. Current execution.mode remains a
service-wide setting; the combined target requires its own implementation and
proof. ChatGPT attached-response execution of selected Codex work uses the owned Termux
Codex wrapper's native app-server with no second primary model or execution-app
acquisition. Submitted inference uses the common app-backed model path. Both are
integrated workflow. Standalone tools follow the work-recording scope contract;
they do not automatically create Native work or start a managed model.
Existing controls already use the native server; neither reasoning route introduces
a new executor or counts an identical path twice. App-backed reviewers and automatic Chat
creation/renewal retain their own dependencies and must be accounted separately.

The target uses ordinary ChatGPT conversation intelligence/context without
substituting Codex/Work allowance or separate API billing as the solution.
Same-account MCP authentication does not select an inference quota. SIWC identity,
MCP service authorization, native transport access and model-request credentials
remain distinct even when linked to the same user. For selected Codex work,
preserve native approvals, execution and canonical recording across both entrances;
no second transcript authority or uncertain mid-task route switch is introduced.

Codex subscription login/auth.json is not a prerequisite for the Bridge's Native
tool, work-control, history or presentation path. The selected Native runtime and
its local Provider connection remain required; an execution home selects local
configuration/history, not an inference account. Attached-response inference uses
the current ChatGPT response; submitted inference uses configured host authentication.
Keep local service/MCP authorization and tool-specific external credentials intact.
Optional Codex account usage/rotation and explicitly selected Codex-hosted inference
retain their own authentication requirements; their unavailable account state must
not become a global gate on ordinary Bridge tools.

The optional Provider may expose one authenticated MCP control service with two
entrances. A Codex-origin native request starts the initial selected Chat model/effort;
a Chat-origin integrated-workflow request opens or attaches its selected work
without initiating Chat or another model. Both then use the same work binding and
controls. The Provider routes controls/events/results; upstream owns execution, approvals, writers,
canonical history, context commits and native task lifecycle. The Codex entrance
may bind those same controls through the selected local-function entrypoint. There is no Provider
shell executor, planning agent, second transcript database or self-review override.

Use upstream's actual tool dispatch, approval and recording path; native results
settle their original logical calls on the selected transport. The opt-in
implementation selects native Responses tool rounds and their existing event/
recording path, with the same operator control Host for MCP and local functions.
Chat lifetime is independent of those rounds and native compaction. Local-function
result continuation may submit another app request in the same saved Chat; a
transport round alone must not start a replacement saved Chat.
Preserve the complete
advertised/deferred tool surface and independent child/reviewer contexts. Within
selected Codex work, do not substitute threadless command/exec or user-shell
escape hatches for Native tool admission/recording. Threadless local tools are
a separate intentional execution scope, not a fallback for broken Native work.

The candidate configuration adds `execution.mode=model_adapter|operator`, plus
`execution.core`, `execution.home` and `execution.workspace`. Operator requires
all three absolute paths. `transport.return_path=mcp|local_function` selects the
Chat-side control/result transport; local functions call the same Operator Host
and do not require a Chat-side MCP connector or tunnel. The authenticated local
management interface remains available. Execution binding and
transport changes require a service restart. Bootstrap/control MCP requests require
the private service bearer supplied locally by `mcp-stdio`, then an explicit
per-work controller and receipt acceptance before effects. Chat-first work needs
no app host for startup/tools/control. Codex-origin Chat initiation and independent
native reviewers acquire the configured app lazily; app absence is an explicit
failure for those capabilities, not permission for another model/account/path.
The first Codex-origin Chat may receive the actual current user request as its
initial user message; origin, return binding and imported history still arrive by
control attachment through the selected transport. A successor uses an explicit
non-user bootstrap on a fresh owned Chat, never a fabricated tool call or another user context message. This wire
shape requires actual app qualification; local admission alone proves no delivery.
A known local rejection before any request starts records zero dispatches and may
confirm Stop only after the pending submission and stream have both settled.
An observed app failure must reach native CLI/error history without waiting for
a model-completion guess or silently replaying the request. An explicitly settled
Chat turn without the required native finish/summary/handoff is an incomplete
operator round: report that boundary and require reconciliation, never infer
successful completion or silently wait for more output. Upstream `systemError`
is a settled failed thread, not successful completion. Stop/recovery may accept
that explicit state only with no active turn/approval or owned background jobs.
An observed app permission/authentication request remains pending and appears in
the original CLI and read-only status. Reuse the existing host-request classifier;
do not equate a settled stream at that boundary with failure or task completion.
The explicit Provider reveal command may navigate to this exact owned Chat after
account/host identity validation, without submitting inference or answering its
consent. The app's real controls remain the permission boundary.

Session/control operations use the same bound upstream app-server and native
context tools. Provider may consume the existing Core catalog-server preparation
operation and execution-record codec; it validates the exact home, runtime, live
process and socket peer. It never chooses another execution account after failure.
The control surface must cover discovery, commands/custom patches/tests, native
history/status, thread creation/attachment/list/read and supported independent
child/fork operations, Chat context handoff, public reports/completion, explicit
interrupt and latest-state resume. Supported native control inventory and actual
execution determine the exposed schema; existing native_catalog/native_tool alone
are not the operator capability.

Keep the same primary Codex thread and stable work identity across native request
rounds, Chat conversation replacement and restart. A new Chat conversation must
not create/fork a replacement primary Codex thread. Explicit independent child
work retains its own native thread and permissions.
The existing private Provider journal holds origin, exact execution binding,
thread/turn links, original reply sink and unresolved calls. Return model-visible
origin/control context on the attach/catalog or other already-needed tool result.
Do not insert origin/return/environment blocks or prior answers as another Chat
user message. Keep private reply destinations, credentials and account identifiers
local. Authenticate bootstrap controls before they can create or claim work; a
work capability and exclusive controller then fence subsequent operations.

For Codex work, reports and final output enter a qualified native event/recording
path, render live in an attached CLI and persist through upstream recording.
A Chat-origin Codex task without a CLI has real native history and no fabricated CLI-origin request. Actual
execution receipts come from native tools. Completion is idempotent and only
confirmed after outstanding calls and the native turn settle. HTTP success or a
saved report alone is not CLI visibility proof.
For submitted inference, the Operator reconciles completion from the bound native
turn, no owned jobs and the settled app lane when native terminal events arrive.
Status and effect/resource admission use that same settlement proof; task
phase or overlapping workspace paths alone never imply a live writer, and
completion must not depend on a prior status read. Stop and newer native turns
fence old observers.
Qualification CLI observers use the work's exact execution home as well as its
verified backend socket. Isolated fixture homes explicitly select their generated
Provider before model submission and preflight the native account requirement;
they never copy credentials to satisfy an unrelated default login screen.

For Codex work, the Native thread is the durable continuity authority. Its
canonical compaction and context-export contract is in section4.6; renewing a
managed Chat does not create another Native thread or a parallel compaction
policy. The current explicit `codex_handoff` accepts a model-supplied grounded
summary and verified history/context cursors. This is a current caller to migrate,
not a reason to retain an independent semantic summary workflow in the target.
The replacement must preserve goals/current instructions, decisions, changes,
actual test/tool outcomes, pending work, source coverage and original returns.
Retrieve original details through the existing history interface where needed.
Replace the summary producer and all actual handoff/renewal consumers together;
do not remove their source-coverage and uncertain-effect protections.

The qualified upstream public-items API omits context-only injected messages.
For this version, the operator may supplement it by reading only public user/
developer/system messages from the exact native-reported rollout for that bound
thread. Require an owner-private regular file and matching thread identity;
exclude reasoning/assistant analysis, internal metadata and credentials. Keep this
read-only compatibility projection separate from the paged execution items, with
its own source cursor in history, attachment acknowledgment, handoff and recovery
coverage. Return later committed input changes with the original MCP tool result;
an upstream queued input is not committed merely because its injection RPC returned.
`codex_sessions action=queue` and `work queue` expose upstream `thread/queue/add`
for an explicit `thread_id`, `text` and stable `call_id`. They preserve its model
and effort and use the uniquely verified live owner across execution homes;
explicit `execution_home` selects a home, including an unloaded saved thread.
With no live owner, the configured execution home may prepare its existing native
backend to access saved work. No new thread, Chat conversation or local scheduler
is created. The enqueue response means queued, not a completed receiving turn.
Identical retries return the original receipt before looking up a current owner.
After uncertain delivery, reconcile the stable native client-message identity in
the upstream queue and canonical user items; an absent observation stays unknown
and must not trigger retransmission. Changed arguments for the same logical ID fail.
Never edit the rollout, infer missing content from timestamps, or save a second
transcript. A native API providing the same complete public context can replace
this version-specific reader after qualification.

Bind each handoff to the same work/thread, source-history boundary and controller
generation. Preserve the summary through an upstream-supported recording operation
without replacing/deleting original execution history or adding another transcript
database. The existing Provider journal keeps only routing/reconciliation state
and references. At transition, fence the old controller from new effects, settle
or explicitly reconcile outstanding approvals/jobs/calls, and confirm the new
Chat's attachment and receipt before giving it control. Read later native events
and user instructions again on attachment; a stale summary cannot override them.
Keep the original CLI reply binding and unresolved call identities. If preparation,
creation, attachment or acknowledgment fails, preserve recoverable state and report
the boundary reached; do not guess success or replay uncertain work.

Chat renewal does not itself require native inference-context compaction or a new
Codex thread. Native turns may change as execution requires without changing the
primary thread/work or completing its original task. Native compaction remains
upstream-owned when the selected execution integration actually requires it.
Do not interpret Native token/hash triggers as a remote remaining-capacity
sensor or reset Native context merely to rotate Chat. An accepted Native context
replacement is a real input to managed-Chat rebinding under section4.6; it is
distinct from an estimated token threshold. Initial history projection is not
itself model summarization. No verified Chat remaining-context
sensor or fixed call/token threshold is assumed; use task boundaries, observable
context growth and headroom, and report unknown capacity honestly. Native controls
must respect active turns; never wait recursively for a summary while that same
turn awaits the MCP call's result. Do not assume `new_context` or native compact
automatically performs this Chat handoff. Preserve foreign opaque state and use
supported native APIs rather than editing rollout files.

Status separates native activity, last host observation/freshness and unknown
remote state from idle. Missing output/observer heartbeat is not completion.
Reusing a saved Chat for the next native user turn requires positive observation
that the previous app submission and stream have settled, with no pending consent
and a successful terminal. A stream terminal alone is insufficient. Waiting for
that boundary retains Stop fencing and must not dispatch after Stop or replace
the Chat merely because its preceding response is still finishing.
Explicit Stop fences actions before requesting native/host interruption and reports
requested, confirmed or unknown results separately, including remaining owned jobs.
A service exit is not confirmation that native work stopped. Local app stream
abort/cancel and a client-side terminal are not server inference-stop proof.
For a managed lane, a pristine journal with zero app dispatches, user objects,
tool returns and public events, no conversation ID and no pending native calls
proves that lane never submitted host work; native Stop need not issue a ChatGPT
Stop for it. An `idle` label by itself is not that proof.
For an admitted request, host confirmation requires a normal stream completion
observed before local cancellation, or the app client's server stop operation
and a fresh matching-conversation COMPLETE/FAILURE stream-status observation.
Missing server identity, rejected/lost Stop or IS_STREAMING/unknown status retains
an unconfirmed host result while native admission remains fenced. A known
zero-dispatch rejection may settle locally after its pending stream/submission
has settled. Do not release an uncertain server round merely because its local
stream closed. Resume reads latest
native history and reconciles unresolved calls/results without replaying uncertain
effects. Another entrance may observe or explicitly take a settled handoff; it
cannot silently become a second controller/writer.
Graceful Provider shutdown releases a Chat lane only after confirmed host Stop and
an acknowledged release, then records that released/stop-confirmed boundary in the
existing journal before exit. Restart retains the work/thread and requires the
explicit latest-history claim; it may reuse that recorded Stop without addressing
the released Chat or starting inference. An absent lane without this recorded
confirmation remains unknown and must not manufacture a successful Stop.

Acceptance requires actual Android native tools/history and live CLI initial,
intermediate and final reports from a synthetic MCP host, both entrances, repeated
MCP work followed by same-thread Chat replacement, source-bound incremental
handoff, exact result/retry correlation, interruption and reconciliation. A saved
summary or ready MCP endpoint does not prove a new Chat was created/attached or
received it. Synthetic transition proof and actual Chat creation/continuation proof
are separate gates, including host dependencies and measured starts/submissions.
No real Chat submission, account migration, live installation change or new default
is implied by this specification. CURRENT owns detailed scope and proof.

### Approved next architecture: independent execution and optional integrations

This heading names the accepted independence design, not a claim that every item
is still unimplemented. CURRENT records source, execution and delivery gates.
The Provider sections describe the existing model-adapter path and its Rust
migration. They do not prescribe Provider-owned planning/context strategy for the
new ChatGPT-operated MCP proposal. Reusable execution/identity/history/stop rules
remain requirements for either path; replacing an adapter policy requires an
explicit contract update and proof before activation.

All Termux execution continues through Core and the
same authenticated installation/update authority. Manager is never an alternative
unsigned runtime launcher. Core may retain a small versioned optional-component
dispatch contract; it must not inspect Manager's preference filenames or interpret
account, window, quota or provider policies to make an execution decision.

`CODEX_TERMUX_CORE_ONLY=1 codex ...` selects the direct path when Manager is
installed but not wanted. It skips Manager preparation, launch policy, optional
TUI and Manager-provided configuration for that invocation. Core still performs
its signed-runtime, environment, sandbox, shared-state and server preparation.
Core consumes and removes the switch before starting upstream; it is not an
upstream setting and does not alter saved Manager preferences.

Manager prepares account/window policy and requests Core execution with the
resolved home and invocation settings. Core owns signed runtime/frontend selection,
required environment/FD preparation and opaque execution-configuration isolation.
A direct Core path must query no Manager and use no Manager fragment or enhanced
frontend, while retaining native storage, locks and server identity protections.
Do not reuse a backend with incompatible effective configuration or rewrite a
live sibling's defaults. Do not create a second execution or update supervisor.

Optional settings must be isolated from Core's required configuration; their
upstream syntax/types and application must be qualified before execution. Manager
configuration discovery must not also project files into an execution home.
Account preference validation and explicit profile projection remain Manager
operations. A saved account selection that Manager cannot fulfill returns blocked
and must not silently become another account. Unavailable optional preparation before
handoff retains the confirmed current execution home; it is not permission for
Manager to ignore a known invalid saved choice. Safe preparation
failure may continue only with the confirmed execution home and proof that no work
was dispatched. Failure after handoff or uncertain submission never triggers a
blind task restart. Explicit user configuration errors remain explicit errors.

Core, Manager, TUI and Provider are separate components of one product. Manager
owns policy and headless commands; TUI owns upstream UI integration and connects
to Manager only for its optional capabilities. Provider selection/settings must
not require Manager. Preserve separate executables/builds where upstream TUI
dependencies require them; do not link the upstream execution engine into Manager
to reduce artifact count. Generic model routing reads the installed provider
capability/configuration authority rather than duplicating Chat families/efforts
in Manager or TUI. Optional capability absence removes only the affected extension;
native UI and basic Core execution remain usable. TUI installation and launch
must not require Manager. Without Manager, its profile/quota/window commands
report that capability unavailable, while native UI and Provider selection work.
This composition requires its own execution proof before release.

The source candidate may expose `codex-chatgpt-provider serve --descriptor PATH
--state PATH [--port PORT]`, `status --state PATH` and command-backed `token
--state PATH`. These are Provider-owned operations, not Core dispatch commands.
Bind only loopback, keep endpoint credentials in private owner-bound state and
verify a fresh keyed readiness challenge before returning the local token.
With no explicit port, a restart reuses its private journal's published loopback
port, retaining generated profile URLs while rotating the local credential. An
occupied prior port fails before starting an app; it never silently selects a
different address or trusts the listener at that port. A malformed service record
is an error, not permission to reset its state.
Never export official-app credentials. Listener readiness is distinct from app
connection, model progress and production qualification. A saved unsupported
feature must fail explicitly before app submission, never silently lose behavior.
These commands do not qualify the remaining host/project/context lifecycle.
The generated command-backed `token` authentication and explicit `prepare` may
start this same Provider executable on demand, using only an already configured
official-app rootfs and the Provider's own persistent app home. Serialize concurrent
starts and return credentials only after authenticated readiness. Never restart a
live lease holder or bypass failed readiness of a recorded live process; preserve
pending lanes. No Chat is submitted by startup/authentication/preparation. `status`
remains read-only. Bound startup time and retain content-free errors in a private
startup log; no alternate launcher, Manager dependency or new supervisor is added.

#### Explicit device service integration

An owner may explicitly opt an already configured Provider and its existing
OpenAI Secure Tunnel runtime into Termux's runit services. This is device service
configuration, not implicit Core launch behavior or a bundled provisioning flow.
Provider owns the bridge/MCP service; the tunnel runtime transports its MCP
traffic. They remain separate processes. Use the existing `termux-services`
startup/root-recovery mechanism rather than tmux or another daemon manager.

**Startup ownership contract.** Provider configuration selects
`service.startup=standalone|supervised`, defaulting to standalone. The default is
omitted from persisted config serialization used by existing fingerprints.
The generic termux-services
layer owns the shared runit graph and its recovery, not Provider health or task
policy. When supervised service operation is selected, runit owns Provider
startup; preparation/authentication must use the verified supervised endpoint
or report bounded readiness failure, rather than race it with detached startup.
Standalone operation retains its existing lazy-start capability. Supervised
authentication/preparation waits within the existing45-second startup bound for
authenticated readiness and returns provider_supervised_start_timeout if absent;
it creates no detached service. Status remains read-only in both modes. Changing
the setting does not adopt, stop or replace an existing service: maintenance must
settle work, stop the old owner and enable the selected foreground supervisor.
Older Provider binaries reject the new service configuration section. Preserve
the previous configuration with the package/state rollback inputs, and restore
the compatible configuration while stopped before starting that old binary.
Establish the
selected lifecycle authority explicitly without guessing from a process name or
an unresponsive endpoint. Concurrent preparation and supervisor startup must
converge to one authenticated service under the selected authority, preserving
the Journal lease, active Native owners and pending work. No second supervisor
or Provider-specific recovery policy belongs in the generic root monitor.
CURRENT records implementation, qualification and delivery separately; the
serialized maintenance procedure remains applicable to installed older code.

Run Provider's existing `serve` command in the foreground under runit. A separate
opt-in host keeper may observe that service and use `host ensure` only when the
owned host is confirmed absent. Provider still owns Xvnc, the Linux app, private
pipes, profile lease and optional viewer. Do not supervise another Xvnc against
that profile, replace a live/unready owner or retry unknown work. Host acquisition
must use provisioned assets and retained login, without model input. A status
timeout or present-but-unready host remains unknown/pending, not permission to
restart it. Disabling the host keeper preserves lazy direct-Chat operation.

Run the existing configured tunnel runtime/profile in the foreground under its
own runit service, after authenticated local Provider readiness. Reuse its
remote tunnel and credential references; never create another remote tunnel or
expose credentials merely to change supervision. Local readiness does not prove
remote control-plane health. Stop the exact old runtime before starting its
replacement; do not run tmux and runit owners concurrently.

Select an Android-compatible tunnel image explicitly; an executable name/version
probe alone does not establish platform compatibility. Do not change a system
resolver to accommodate an incompatible Linux runtime. Keep service-selected
paths/profile in private common configuration; native execution home/workspace
remain Provider-config authority. Observation may use the admitted immutable
Provider image to avoid Core activation-lock contention, with explicit rebinding
during package/service maintenance. A failed observation preserves the current
transport. MCP stdio uses that explicitly admitted Provider image directly and
verifies the current service endpoint/credential per RPC. Provider PID/start
changes do not restart the tunnel or replay a request. Rebinding the immutable
stdio executable is a separate bounded maintenance action. MCP stdio stdout
remains reserved for protocol messages, separate from diagnostics.

**Optional component and tunnel distribution target (not yet implemented).**
Core is mandatory. Manager, enhanced TUI, Provider and the patched Termux tunnel
client are independently selectable optional capabilities. Full selects all of
these optional capabilities, including the prebuilt Android tunnel client;
individual selection must also work without installing Full. Remote MCP setup
resolves its tunnel dependency; a Core-only or local-function route has no new
remote-tunnel requirement. Installing binaries does not enable services, log into
an app, acquire credentials or create remote registrations implicitly.

Ordinary installation/update uses qualified prebuilt artifacts produced during
release preparation, with authenticated inventories and independent component
version/selection/rollback. It requires neither Go/clang nor another source
checkout on the user's device. Full may carry exact signed component packages
under the same installation authority; it must not become a second implementation
of their installers or couple independent tunnel updates to Core/Provider rebuilds.
Existing Base/Full releases retain their original inventories and interpretation;
CURRENT distinguishes that installed format from this future composition.

The wrapper repository owns one authoritative tunnel compatibility recipe: the
official source/version resolution, bounded Termux/mobile patches, Android build
and affected-path tests. Reuse it for release builds and an explicitly selected
source-build/recovery path. The latter acquires official source into temporary
storage, checks required Go/clang/build tools and qualifies the result without
requiring ~/prj/tunnel-client or a separate user-managed repository. Resolve latest
stable when requested; a source revision outside the validated recipe must fail
with its actual incompatibility rather than silently use an old build. Build-tool
setup is scoped to the selected build operation, never ordinary Core startup.
The additional tunnel checkout may remain a development experiment, but cannot
be the product's patch/build authority or a runtime dependency. Keep one copy of
each product patch and build rule in this repository. CURRENT records preparation,
source-build availability, signed distribution and activation separately.

Qualify platform execution, existing profile/credential compatibility, actual
MCP request/result delivery and the mobile half-open HTTP/2 regression before
switching the configured service image. Evaluate each local patch against the
selected upstream instead of blindly retaining it. Preserve remote registration,
credentials and unrelated global/shared tunnel consumers. Intentional replacement
stops the exact old service runtime, selects the validated image and verifies the
actual new process and remote delivery; failure restores the matched previous
selection without inference/effect replay. A successful build, version probe or
package install alone is not activation proof. Supervision never performs an
update on its own. CURRENT owns installed versions and the separate maintenance
versus future automatic-dependency integration milestones.

Runit may restart an exited service, but it must not replay native/Chat work,
submit prompts, grant approval, perform updates or change account/model settings.
The device's existing root recovery can recreate the registered supervisors.
It cannot recover a fully killed Termux UID without the next external Boot/login
opportunity. Intentional maintenance must first disable the relevant service;
otherwise desired-up supervision will start it again. Service enablement, dependency
checks, private bounded logs and explicit disable/restore instructions belong in
the future shared minimum-setup scope; automatic installation remains deferred.

`status --thread ID` may additionally inspect that native thread's private lane
journal without connecting to the app or starting a service. Report observed
transport phase, pending native call count, cached public event count, separate
dispatch/user-object/tool-return counters and a bounded static error code; never
export prompt/result text, account identifiers, host IDs or credentials. Distinguish
endpoint liveness from the last journal observation. Running/unknown/unconfirmed
stop state is not completion, and tools-ready means awaiting native execution or
results. If the local endpoint is unavailable, retain the observed lane state and
say its current remote activity is unknown. Inspection never resumes or cancels
work or infers Chat allowance deductions.
An explicit Provider `stop --state PATH` requests graceful shutdown of its verified
local service, including owned app cancellation/teardown, without removing its
journal, app profile or configuration. Use a PID-bound kernel handle and revalidate
the recorded process before signaling; never signal an unverified reused PID or
fall back to a broad process match. Bound acknowledgment time, report an unconfirmed
stop if the owner remains, and never automatically force-terminate the service.
Existing bounded teardown of that service's own child groups is unchanged.
An unavailable kernel signaling capability fails explicitly. Stopping this service
does not stop the native Codex engine or undo completed native tool effects.

The optional Rust-host viewer reuses that Provider service and its owned private
RFB Unix socket; no Bun/Python daemon or second app process is introduced. An
explicit host.viewer_assets setting selects already provisioned noVNC assets.
Absent/broken viewer assets must not prevent inference. Expose viewer HTTP and
WebSocket only on loopback behind a separate random capability, with no-store and
no-referrer headers and exact local-origin checks on WebSocket/control requests.
Reject traversal, symlinks and unexpected/oversized assets. The viewer capability
never authenticates Responses or general app/CDP operations. An explicit `view`
may start the Provider and open its private URL in Android through Termux; it
neither logs that URL nor submits Chat messages. Preserve mobile keyboard toggle,
manual keyboard, scroll/drag, Esc and fullscreen controls. Viewing is optional for
inference and does not establish successful fresh login or correct physical UI.

The retained Linux qualification desktop exposes compact pinned ChatGPT/browser
icons from the installed official packages. A single tap launches an absent application, minimizes its focused window,
or restores/activates an unfocused or minimized window. Existing-window actions
are immediate, not queued through the half-second controller loop. Match the
private desktop's exact app class and controller ancestry, never a window title
or another desktop/profile. Serialize duplicate launch taps, preserve authentication
and running work, and do not submit Chat or create a conversation by switching
windows. This qualification UI is not a Core dependency or a second Provider
execution path. Superseded desktop/adapter helpers retire with the experiment;
reusable qualification tools remain development dependencies and are not added
to the Rust Provider payload.

The Chat provider source lives under `providers/chatgpt`, outside crates/core,
Manager and TUI. Its approved implementation target is a native Rust adapter
process, usable through upstream configuration without Manager or enhanced TUI.
Rust owns configuration/capabilities, Responses HTTP/SSE and tool mapping,
conversation/context correlation, cancellation/reconnect/retry policy, CDP
communication and owned-host lifetime. The provider may use the signed app's
authenticated context; it does not replace that app's login or inference.
Its failure must not disable native Codex, account commands or window management.
When Electron retains several index-page targets after sign-in, select the sole
main Chat/Work surface through read-only DOM capability inspection. Do not select
by target order, activate/create a chat, or attach inference to an auxiliary
window. Even one initial index page must pass the visible-composer check before
attachment is retained. Window count, creation order and localized button labels
are not readiness evidence. Duplicate target identities or multiple eligible main surfaces refuse;
an unready surface remains a readiness failure. App account and bridge admission
still run after target selection.
Reuse existing upstream provider configuration and bounded local contracts; do
not introduce a speculative general plugin framework. Unify component discovery/
status and lifecycle presentation where appropriate, not credentials, transcript
storage, process ownership or unqualified update mechanisms.

Earlier JS/Bun and Python implementation paths are migration evidence, not
separate product authorities. Keep only the JavaScript required to execute inside the
app renderer for the current app-owned function transport, plus optional viewer
assets. Rust controls and validates that bridge through CDP; no token extraction,
second tool executor, duplicated configuration validator or task/retry/lifetime
policy belongs in the injected script. Define a versioned, bounded bridge contract
and reject incompatible app capabilities before an uncertain submission. This is
not a claim that the entire desktop app is Rust or JavaScript, nor a promise that
changing the provider's language bypasses the host's limits.

The optional Rust Provider package is providers/chatgpt/Cargo.toml with runtime code in src/
and separately inventoried app-side bridge/assets. It must not be a Core, Manager
or TUI dependency. Workspace membership must not make Base acquire/build the
provider or desktop app. Replace product-path Bun/Python dependencies only after
their behavior is qualified in Rust; development/qualification scripts may remain
Python or shell. Use one selected adapter per launch, not automatic dual-runtime
execution or fallback after uncertain dispatch. Old signed generations remain
explicit rollback targets. Preserve installation identities, login/profile data,
remote conversation associations, live jobs and current configuration overrides
through bounded migration; source integration alone never activates a replacement.

Once the supported Rust Provider path has individually qualified every
replaced consumer, retire duplicate source adapters, experimental installer/
profile writer and desktop/viewer launch policy with their unused callers.
The authenticated optional Provider package is the supported installation
interface; preserve a qualified renderer bridge, viewer assets, explicit
transport choices, Native configuration/approval/history and reusable
qualification tools. Preserve user changes, licenses, original experiment
proof, installed experiment consumers and immutable current/rollback inputs.
Source retirement never silently replaces a running app, profiles or service:
live migration remains a separate authorized, observable transition.

Native automatic reviewers have their own Provider lanes and remain outside the
configured ChatGPT project. Recognize upstream guardian_review thread metadata;
send only that native reviewer's supplied context, never the primary app lane's
conversation or implicit project instructions/files/memory. This transport
isolation does not determine the native approval policy or reviewer decision.
An existing project-bound lane cannot be relabeled as an isolated reviewer.

New independent MCP trials use the repository's `Termux Bridge` connector display
name, with display/connector identity configurable rather than hardcoded branding.
Historical `Codex Native2` references remain provenance only. Renaming never changes
an outstanding native tool call/result identity. Reuse authorized existing admin
and computer-use access when exercising this path; actual connector ACK remains
an independent gate from discovery and the local-function transport.

The Rust MCP return path is explicitly selected at service startup through
transport.return_path=mcp and a configurable transport.connector_name (default
Termux Bridge). Local functions remain the default; never switch transports after
uncertain dispatch. The same Provider service exposes a bounded streamable-HTTP
/mcp endpoint; it is not a second executor. Static discovery discloses no native
state. Discovery descriptions state capabilities, not instructions overriding the
host's approval or retry policy. native_catalog reads the selected native tool
catalog; native_tool dispatches native tools.
Exact-call validation and replay protection remain enforced by the Provider.
Upstream Codex owns tool execution, approvals and canonical history; Termux Core
only prepares and launches its verified runtime. The same binary's optional
mcp-stdio command verifies an already running owned Provider for each RPC and
forwards bounded multiplexed JSON-RPC for an existing tunnel. Each request uses
the same verified service snapshot for its process, port and credential; later
requests discover replacement generations without retaining stale endpoints.
An unavailable service produces a request error while stdio stays open, including
when the service is absent at bridge startup. No mutation is automatically replayed;
it never starts an app/service, repeats delivery or executes tools. EOF/request
cancellation detaches only that observer. A random per-app-turn capability authorizes native_catalog/native_tool;
each logical call ID binds exact tool/input/report, independently of JSON-RPC IDs.
Native execution/results still pass through the common Responses/lane journal.
Persist invocation intent and native results before publication/delivery. Peer
loss detaches its waiter, not the native action; an exact retry receives the same
result. Changed IDs/arguments, stopped turns and unresolved owner changes refuse.
Return native results to the retained original MCP invocation, never submit them
as a new Chat user/tool message. A retained app turn cannot silently change model,
effort or developer policy. Effective tool definitions belong to the current
request: explicit top-level declarations own the complete current set. In
Responses Lite, input declarations are chronological incremental updates or
one rebuilt catalog for a nonincremental window. Validate each batch independently
and apply later definitions by tool key; history is not one simultaneous catalog.
Each individual catalog must remain internally unambiguous. Discovery results
can extend that set, while explicit current declarations retain precedence;
pending calls retain their original tool identity and result binding. Historical
catalogs cannot re-advertise tools absent from a current catalog.
new user input waits for a settled boundary. Account/project/stop/transcript rules
are identical to the local-function path. Host confirmation/authentication must
surface as blocked state, never be bypassed. Rust wire fixtures alone do not admit
this route for live use; actual connector ACK is required before activation.
Pending host authentication/confirmation keeps the original request attached and
emits one public waiting report; status records the blocking reason and observed
submission counters. Poll the same invocation until the host resolves it or the
user cancels. A correlated host response clears only its own confirmation. Without
explicit user delegation, never click approval automatically. Bounded explicit
user delegation permits selecting the app's real Allow control for that authorized
operation; it never permits synthesizing consent or bypassing the host's controls.
For delegated app permission, `approve --thread ID` first uses the signed app's
own approval action without opening a window. Prefer its offered "for this
conversation" action; only when that scope is unavailable choose one-time Allow.
A successful conversation approval must never be followed by one-time approval.
Only a known capability rejection before any approval intent permits the visible
UI fallback. `approve-ui --thread ID` explicitly exercises that fallback: open the
bound private viewer/Chat and actually select/click the same preferred scope.
Both paths require the current bound account, an active operator lane and exactly
one unresolved confirmation for its configured connector. Use only server-offered
actions; the app constructs its own reply. Scope refers to this Chat and connector,
never account-wide/persistent permission or a successor Chat. Native approvals are
unchanged. Authentication, forms, passkeys and ambiguous targets remain pending.
Wait for cold UI rendering through bounded pre-intent readiness observations;
never queue a delayed approval merely because navigation started.
Fence approval intent before sending/clicking. An uncertain result never permits
another path or scope; reconcile its actual reply before proceeding. Opening a
window or returning HTTP success does not prove acceptance/native completion.
Optional viewer assets and a verified private RFB socket are needed for automated
visible fallback. Operator-owned lazy hosts register that viewer only after the
host actually starts. An external owned relay may explicitly configure
`host.viewer_socket`; it must pass the same private socket/identity checks.
Never submit another Chat user message merely to continue. Record
observed counters before reporting a host block or stream failure.
Loopback SSE keepalive uses a content-free data `ping` event: upstream's parsed
SSE idle deadline ignores comment-only heartbeats. This proves transport
liveness only, not model or native work progress. It adds no transcript item,
Chat submission or retry; ordinary observer cancellation still reaches the host.
An explicit host permission/authentication reply may use the signed app's separate
tool-role continuation. Admit only a response correlated to an observed pending
host action in this lane, including a target explicitly offered in that action's
server metadata which addresses the invoking message instead of the confirmation
message. Unrelated ancestors, ambiguous matches and a second response to the same
action refuse. A failed stream takes precedence over a historical host wait.
The permission stream's `done` does not retire its retained MCP capability.
Native round admission accepts the original dispatch plus exactly the observed
host replies; it never normalizes away actual request counts. Late callbacks from
a prior permission stream cannot terminate the current continuation.
Never fabricate a reply instead of the signed app's real consent control. Count
those actual host requests/reply objects separately from native tool returns and
user-role objects. They are not automatic retries or proof of allowance debits.
A retained renderer bridge is bound to the exact Provider bridge-source revision.
A different revision refuses admission without replacing hooks or pending work;
explicit settled qualification cleanup or a newly owned host establishes the new
revision. A new binary must never silently keep executing old renderer hooks.
`reveal --thread ID` selects only that native lane's actual saved app conversation,
using the existing owned service and current account binding. It neither starts a
service/viewer, submits Chat input nor approves tools. Missing/mismatched lanes or
accounts refuse; never pick a chat by title, sidebar order or a guessed remote ID.
Last-known host blocks do not make an unknown/stopped lane a live attached wait.
A confirmed host authentication/permission wait pauses the model-work deadline;
transport cancellation still requests and verifies stop. Human response time alone
must not expire a correctly attached request and force another Chat submission.
The provisioned Linux UI includes Korean-capable CJK fonts; typography is not an
account-language or personalization change.

Experiment integration targets rewrite/rust-core as a repository, not crates/core
as a code destination. Port exact reviewed provider/runtime sources, dependencies,
licenses and affected regression drivers onto current authority. Never replace
current Core, Manager, UI, release workflows or contracts with experiment snapshots.
Preserve source history and user changes; compare intent and current equivalents
before replaying a commit. Separate runtime modules from qualification/research
material. Do not ship historical probes, GUI trials or user app/login data as
runtime dependencies merely because they share an experiment directory.

The runtime/tests/license port and standalone Rust implementation are already
on rewrite/rust-core. Remaining lifecycle/package/activation gates and preservation
of unique experiment work still precede closure; source existence is not release
admission. Current implementation and proof status belong in CURRENT. Port useful
remaining qualification drivers alongside the affected Rust implementation, preserving their
provenance and independent inputs. Review branch-specific and uncommitted work
before the final integration/closure; retain unique evidence and attribute reused
or translated code. Do not overwrite current product history with a wholesale
experiment merge, delete unreviewed user work, or leave two active provider source
authorities. Rust admission, product acceptance and preservation of unique work
precede experiment closure. CURRENT.md owns the ordered migration work.

First qualify no-Manager execution and configuration isolation, then generic
Manager/frontend/provider composition. Before stable provider admission, establish
cross-provider readable-history transfer, fresh/config-profile inference, native
tools/reviewer, interruption/resume and observed native transcript behavior against
the exact release inputs. Report live Chat submissions and explain trial budgets
before execution; Pro is excluded from automatic qualification. Experimental
source integration is not stable publication or device activation.

The actual development acceptance uses the real `~/prj/ai` checkout directly
after the evaluated path can execute development. The existing Rust no-MCP path
has its bounded actual trial recorded in CURRENT; do not schedule that completed
trial as if it had never run or count it as proof of the proposed workflow. New
control paths require their own development acceptance. This user-approved
source-development trial does not use a temporary worktree, clone or project copy. Inspect that
repository's AGENTS/contracts and Git baseline, preserve unique uncommitted and
untracked source, and let the ChatGPT-backed native Codex perform a real sequence
of analysis, implementation, tests and failure repair. The supervising agent must
not substitute its own implementation for the evaluated model. Preserve unrelated
user work and operating authentication/data/runtime.
The qualification controller runs the bounded sequence without per-turn
supervisory model polling or quality intervention. After the terminal notice,
the supervisor independently reviews actual changes and applicable validation;
recording a future task or accepting the evaluated model's own success report is
insufficient. Earlier C/C2 reviews retain their actual assisted/unassisted scope;
their per-turn supervision does not become a future requirement.
Qualification observation has an explicit no-activity budget as well as the total
deadline. Rewriting a local snapshot or retaining an active-stream flag cannot
reset that budget or establish progress/completion. The absolute wall deadline
still applies across Android suspension; report wall and monotonic elapsed time
separately rather than assuming they are identical. After a known failure, retain
completed artifacts and bound further waiting. A fresh authoritative server
terminal observation may clear a stale local active-stream flag after Stop;
local abort alone cannot. Reject late starts on cancelled owned lanes before
changing their active state. Preserve the original Stop promise without resending
it; keep uncertain source state until independently reconciled.
A persisted settled response with its committed Completed event and no pending
calls can satisfy a later host Stop without recreating its disappeared app lane.
If that same round remains present during shutdown, its fresh owned snapshot
must show no pending input, stream, approval or error before release. The committed
Completed receipt also permits release after the receipt-only Stop path; absence
of that receipt preserves the original server Stop requirement.
An idle label alone, an offered tool, or an unfinished round cannot. Native Stop
still independently accounts for active turns and jobs before acknowledging the
whole work or tree.
Verify complete immutable-input hashes before work and final acceptance. During
local protocol observation, cached verification may reuse unchanged file identity,
size, modification time and change time; any metadata change triggers a new hash.
The original manifest is pinned and cannot be replaced during that run. Never
rehash a large unchanged CLI binary on every status/approval observation.
Native transcript, exact
effects, interruption/reconnection/latest-state continuation and meaningful test
results qualify the path; synthetic replies alone do not. A live budget and
separate submission/result/retry counters remain required; automated Pro is
excluded. Git protection is not permission to reset or discard existing changes.


### Optional Provider packaging and selection

In the deployed format Base excludes all optional components and Full composes
qualified Manager/frontend; Provider is a separately selected package. The future
selectable-component/Full target is defined under device service integration.
Provider remains opt-in outside Full and must not require signed desktop app/rootfs download
or login on ordinary update/startup. A stable provider needs a declared versioned
payload/dependency inventory, authenticated delivery and tested activation/rollback
before replacing its experimental installer. Preserve app authentication and user
state; importing provider source does not authorize redistributing third-party
desktop software. Choose the bounded packaging extension from its real reachable
payload; do not assume the current eight-file inventory already supports it.
The optional-component packaging contract selects an independent Provider payload, outside Base/Full generation
inventories. Core exposes `codex provider install chatgpt --local DIRECTORY` or
`codex provider install chatgpt --remote HTTPS_DIRECTORY_URL`,
`codex provider install chatgpt` / `codex provider update chatgpt` (explicit stable
optional-component discovery),
`codex provider status chatgpt`, `codex provider rollback chatgpt`, and
`codex provider chatgpt ARGS...`. These management routes need neither Manager
nor TUI. Ordinary Core execution never consults or starts an absent Provider.
Explicit Provider installation may enroll a payload; default manual update may
acquire only a previously selected Provider. Both consumers use the same verified
acquisition/selection implementation. Ordinary launches do not enroll or update it.
Remote installation uses a canonical HTTPS directory URL with a trailing slash.
Remote asset names flatten the manifest’s fixed `licenses/NAME` paths to
`licenses--NAME`; all other names stay unchanged. This permits immutable GitHub
Release delivery without a separate mirror or changes to Base/Full public heads.
Core downloads the bounded manifest/signature first, authenticates them against
its admitted authority before fetching payload files, enforces signed file and
aggregate bounds and reuses the same staging/selection checks as local install.
Controls have finite transfer deadlines; payload downloads fail after 120 seconds
without new bytes and continue while bytes are arriving, subject to signed size ceilings.
Failed or uncertain transfers are never retried implicitly and cannot select a
payload. Stable discovery reads the independent `provider-index-v1` and its
detached signature from the owned main branch. The bounded ordered stable index
binds provider/generation/sequence, immutable release directory and manifest digest;
Core authenticates it with the current admitted update authority and requires
all bindings to match the downloaded package. Base/Full heads remain independent.
Only the invocation-owned private acquisition is removed on exit; an
interrupted acquisition remains unselected for later owned-artifact cleanup.

New Provider payloads contain the Android/AArch64 executable, dependency lock,
NOTICE, two retained third-party licenses and an opaque source-input inventory.
New packages use a six-file signed inventory; Core continues to authenticate
immutable eight-file packages from earlier releases for existing selection and
rollback. New six-file packages require this updated Core verifier; existing
Core versions that accept only eight files must be upgraded first. Neither
signed layout permits undeclared assets. The ordered manifest
binds provider identity, generation, monotonically increasing release sequence,
base source commit, source-input digest, target/API/dependency contract and exact
file names/hashes/sizes/modes. Detached Ed25519 authentication uses Core's admitted
update authority, never a public key supplied by the candidate itself. Payloads
contain no Linux app/rootfs, login, profiles or model/user configuration. External
PRoot/Xvnc/explicit app rootfs are needed only for managed app work; viewer assets
are optional. Direct Chat-first native work retains its no-app boundary.

Install verifies the complete immutable inventory before and after private
staging, probes the executable without inference, then atomically selects it.
Selection retains current/previous identities, their admitted verifier keys and
a high-water release sequence. Rollback revalidates the admitted previous payload
and never lowers that high-water sequence. Equal identity/content is observable
without replacement; changed bytes under an existing identity are rejected.
Unknown/malformed state fails before mutation. Install/rollback serialize selection
writes through the existing Core activation lock and change only the atomic
selection record and absent immutable payload directories. Running services and
pinned profiles retain their original immutable generation; install/rollback never
stop services, replay work, rewrite profiles, delete older generations or overwrite
user data. New ordinary launches use the newly selected generation. Service restart
or explicit profile preparation is a separate user/configured lifecycle decision.

The shared store lease is retained for installed older Core consumers whose writers
still use that directory exclusively. New selection writers take the same shared
lease under the Core activation lock, so old running readers do not block them,
while an in-progress old exclusive writer is still excluded. This compatibility
exists while installed old Cores remain supported; it introduces no new lock/state
file. Launch revalidates signature/inventory under that shared lease before exec.
The Provider keeps inherited non-stdio descriptors, including the store lease, in
its own process
but marks them close-on-exec before any runtime or child startup. Only descriptors
explicitly exported by a child launch cross that boundary. Native backends, app
children and detached service children must not retain an incidental parent lease;
a live Provider keeps its immutable executable and does not block new selection.
Stopping an idle Provider must not require terminating an independent Native
backend solely to release such an inherited lock. This adds no environment/FD ABI
and does not change existing Core dispatch, payload verification or work authority.

Package selection and generated native-profile preparation are distinct. Existing
profiles/catalogs keep their immutable executable binding until explicit
`codex provider chatgpt prepare`; install/rollback report selection only. Retain
generations referenced by existing profiles and running work through verified retirement.
This preserves the current direct-executable auth/checkpoint contract and does not
silently replace the installed JS adapter or any user-selected model route.
A default manual update advances the selected Provider independently of a Core
rollback hold. Provider rollback retains its previous/high-water contract; a later
explicit `codex update` checks its forward channel again. No background Provider
update or service/profile restart follows from that selection.
Authenticated remote delivery/promotion and actual-device activation require their
separate signed-release and device proofs/authorization; local fixture signing does not establish either.
Once the Core interface work is admitted, Manager/provider-only changes must leave
base inputs unchanged. Upstream/backend changes still require the applicable
frontend and execution-protocol compatibility proof.

### Approved provider configuration, context and remote project behavior

The inference-context/compaction and cross-provider handoff rules in this section
describe the existing model adapter. The opt-in operator contract above governs
Chat conversation renewal; shared configuration, identity and safety rules still
apply. Native compaction and operator Chat renewal are not one mandatory cycle.

The Rust candidate's single user-owned provider configuration is
`$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, defaulting to
`~/.config/codex/providers/chatgpt.toml`. Its source implementation does not make
the installed JS adapter consume it or qualify a signed production interface. Provider owns validation, effective settings
and rendering
to native upstream configuration/catalog; generated profiles, models.json and
immutable installation modules are outputs, not additional editable authorities.
Explicit invocation overrides win, then per-model settings in this file, then
file-wide settings and declared defaults. Preserve ordinary upstream overrides
and show their effective origin rather than silently rewriting user configuration.
This configuration precedence resolves configured values; it does not supersede
the retained settings of an existing Chat or Native thread on ordinary resume.
Use the destination-owned preparation/override rules above for those requests.
Existing --profile chatgpt remains a generated upstream fallback. Provider's
candidate `prepare --home PATH` projects a native catalog and command-auth profile
for an explicitly selected existing Codex home. One private shared profile is a
generated output of Provider configuration; per-home links never override a user
file or different/dangling link. Manager may project the same shared registry to
new managed homes, but is not needed for Provider preparation. Native defaults
and unrelated config/auth remain unchanged. Managed native work renders its
per-thread routing directly and writes only immutable catalog snapshots in its
own state; it does not publish or replace the shared or per-home profile. Only
explicit `prepare` publishes those profile outputs. Routing restoration supplies
no model, provider-selection or reasoning defaults to
an existing thread: native persisted settings remain authoritative. Explicit
Chat profile/route selection supplies its configured initial settings. A
mode-only start reads the bound thread's current model and nullable effort;
changing between Plan and Default must not reset them to Provider defaults.
Generated catalog snapshots are immutable; settings edits apply to projections at the next explicit preparation,
and active backends retain their bound catalog. A model route reload in the
running Provider does not require regenerating metadata when its public contract
is unchanged. Native Codex
model defaults/account configuration are not modified to configure this provider.
Native work discovery first distinguishes execution homes from their bounded,
private owner records. Unsupported records for another home do not block this
home; this home's records still require complete namespace, settings and process
validation before use. Unreadable or unsafe owner records remain an error.

Expose show/effective, validate, edit and atomic field-setting operations through
the provider's existing invocation surface; optionally present the same operations
in TUI and Manager. Do not introduce another permanent launcher, require Manager
for file-based configuration or make a TUI implementation a second validator.
Validate before atomic commit; failed edits preserve the old configuration.
Show invalid configuration and unapplied changes clearly. No settings read/edit
may submit Chat or create a project. Unknown keys/types and inconsistent limits
must be diagnosed, not silently ignored. Avoid secret values in status output.

The candidate `host.rootfs` field selects an explicitly provisioned official-app
Linux rootfs. Rust owns a private persistent app home beneath provider state, an
exclusive profile lease, a temporary X server and Chromium FD3/FE transport.
Resolve `proot` and `Xvnc` from the Termux prefix, with fixed isolated arguments;
never interpret a configured shell command or bind the user's host home into the
guest. No implicit app/rootfs installation or profile copying occurs. Preserve
app home/authentication on shutdown; remove only owned temporary resources.
Read-only app identity, host status and catalog observations may inspect a unique
primary app page without a visible composer; they do not install writer hooks or
grant inference admission. Programmatic inference uses that same owned primary
target and validates its app Scope, authenticated identity and API capabilities
before opening a lane. Visible composer state does not establish those facts and
is not required on Home/project pages. A composer may disambiguate multiple app
windows; ambiguous targets still refuse. An unready startup launcher is detached
and rediscovered on the same CDP client during the existing bounded readiness
wait. Home/Plugins navigation alone is not loss of authentication.
A lost host with unsettled lanes is not permission to replay their requests.
Bind each lane to the app's currently authenticated user/account/workspace and
owned host generation using identifier-only app state, never a token dump. Check
that identity before warm submissions/results and again after a completed app
round, before publishing executable calls, committing a summary or declaring
completion. Account/host changes during inference must not authorize later native
effects. An account mismatch on pending/uncertain work or an old native turn
refuses before sending history or results; restoring the original account
reconciles that round without resubmission. A settled lane with no pending calls
may, under `history.account_change="new_chat"` (the default), use the app's newly
authenticated account at a genuinely new native user turn
containing a new user input. Open a fresh account-scoped remote conversation from
current canonical native input, resolve any project in that account, and retain
the old remote chat. `history.account_change="refuse"` retains the original-account
requirement. Account-dependent values belong to user/transport configuration;
code never adds branches for particular account, workspace, tunnel or connector IDs.
Revalidate the visible app scope when opening a new lane. A replaced login scope
or client retires only passive owned hooks before installing the current context;
active work keeps its hooks. Check the admitted identity at lane allocation and
immediately before the actual conversation POST, so a login change between those
steps cannot submit into the previous or newly selected account by accident.
A settled lane may also be rebuilt from current
native canonical input at a new native user turn after its app context disappears;
retain the old remote chat. A model/effort-only change is not a new user turn.
Pending/uncertain rounds and executable cached calls must not be replayed into a
replacement host. Report that reconciliation is required and preserve their state.
A cached settled final response can still be returned locally without inference.

A definitely rejected, single local gateway call may receive a structured tool
error on its original host call ID, allowing the model to correct an unadvertised
name or malformed gateway envelope. Validate its entire batch before publishing
any executable call; rejected batches have no partial native effects. This is a
tool-result continuation, never a new user message or a resend of the original
request. Permit at most three such correction continuations per Responses request,
record each rejection before returning it, and expose the rejection in the public
transcript. Additional model starts remain real usage and must be counted.
Uncertain delivery, identity/host/protocol failures, ambiguous multiple host calls,
compaction controls and replayed call IDs never enter this correction path.
Native function argument validation stays native; ordinary malformed arguments
are not rewritten by Provider. Exhaustion returns a specific terminal failure.

Public model text may translate known Chat presentation components for native
terminal Markdown: remove MemoryCite component tags and render box containers as
plain text with an optional title. Preserve their useful contents, unknown markup,
literal inline/fenced code and exact tool arguments/results. Hold incomplete known
tags while streaming so an emitted prefix never needs rewriting; retain malformed
or unfinished literal text at final settlement. This projection never requests
another model turn, exports private reasoning or changes native execution policy.

An app may append to an earlier public message after emitting another message.
Keep already completed native items immutable: publish the new suffix in a new
continuation item, retaining the source message's prefix and phase checks. Reuse
that continuation while it is current. A phase-only final promotion of an older
closed message publishes its full final text as a distinct final item; it cannot
rewrite earlier native history or lose the final answer. This is local projection,
with no new model request, tool replay or change to tool annotations.

Retained MCP returns can divide one physical app message across several native
Responses rounds. Give each round's public fragment its own stable native item
identity so later fragments cannot replace earlier canonical items. When a
message that began in an earlier round commits as the final answer, publish its
complete body as a distinct final item, including a phase-only completion with
no new text. Preserve the app source ID for prefix validation and result routing;
never submit another model request to assemble the final body.

The private relay descriptor remains an explicit qualification connection option,
not a fallback or required Bun/Python production dependency.
An explicit owned bridge disposal may restore only its own installed hooks after
all its lanes have been released. Refuse disposal while any lane remains; never
remove another owner's hooks/state or close the shared app as qualification cleanup.

The Rust configuration tool remains a candidate developer surface until product
admission. Its effective output distinguishes file values from defaults and
reports runtime application as not queried; saving a file does not establish
that an installed adapter consumes it. The source Rust server reloads this one
file at request admission. Models output distinguishes implemented candidate
transport from production qualification. A field
transaction may set several dependent values together, validated once before
commit. Unsetting a field restores its declared default. Concurrent tool edits
serialize; an external editor changed since opening must cause a conflict rather
than silently overwrite newer content. No-op changes preserve the existing file.
An editor failure or invalid buffer preserves the original. Use private bounded
files, reject symlink/nonregular destinations, and publish without hard links.
The project fields are history.mode, history.project_name,
history.project_id and history.create_if_missing; account/workspace-scoped bindings are
added only with qualified host discovery. Retained history remains the migration
default; choosing project mode requires a name and defaults creation to enabled.
Neither saving that choice nor listing models proves a host operation is available.

The effective selectable model catalog combines models and thinking options
actually offered by the authenticated ChatGPT app with entries the user adds in
provider configuration. App discovery is account-scoped, observes the app's
offered model/effort semantics, and submits no inference. An explicit configured
entry can add a model or a compatible route and can deliberately correct a
matching mapping; do not make user additions replace the app's whole list.
Keep the common app-to-Native effort translation as validated data, independent
of model-name branches. The currently observed app values `standard`,
`extended`, and `max` correspond to Native `medium`, `high`, and `xhigh` in
the qualified picker. Explicit configuration can supplement or correct that
mapping. Unknown or ambiguous choices remain explicit rather than being rounded
or silently downgraded; retaining an already selected host pair requires no
invented Native effort.
The common mapping lives in `catalog.effort_map`, keyed by the app's exact
thinking-effort token with a Native effort as its value. Omitted entries inherit
the bundled qualified mapping; explicitly supplied entries supplement or correct
it. An existing model route is more specific and retains precedence. A mapping
table can be configured without copying the bundled model definitions.
Expose account-bound app model discovery through `codex_host action=models`.
Its optional `refresh=false` reads retained options plus current config without
acquiring the app; an absent observation explicitly reports configured-only data.
Native `prepare` uses that nonacquiring read so authentication/profile setup stays
lazy. Refresh explicitly before preparation to publish the latest app choices.
An app-only configured default without admitted options fails preparation.
The app supplies its offered picker options, not every internal catalog entry;
Rust merges explicit configured model definitions and the common effort map.
New app-discovered model IDs are `chatgpt-desktop/app/<actual-host-slug>`.
Aggregate effort choices only for that exact host slug; family/version labels
are presentation data. This avoids giving an existing family alias a different
execution meaning when the app offers separate Instant/Thinking models.
Retain existing configured IDs for saved bindings, hidden unless explicitly
configured as selectable. They are not alternative names for new app IDs.
Configured model entries augment the bootstrap catalog before app discovery and
the app catalog after discovery. Matching model/effort entries correct the
existing route; unrelated offered routes remain available. Validate supplied
definitions before merging so malformed duplicates are not normalized away.
This operation starts no inference and does not change a selected model. A missing
app capability or unmapped effort is reported with its specific cause.
Validate conflicts and upstream effort representability instead of inventing a
mapping or advertising inaccessible options as confirmed available. Preserve the
existing configured catalog when migrating. Cached discovery must identify its
account and freshness; an unavailable host is reported as such and must not
prevent ordinary Core launch. Catalog refresh never changes an active request or
implicitly selects Pro; new selection is an explicit user/workflow choice.

The existing Provider journal retains the discovered, account-bound offered options
for Native preparation and request-route admission. Reapply the current validated
configuration when consuming that observation; cached merged configuration must
not override a later configuration edit. Discovery time is the time the
app state was observed, not a claim about the last upstream network fetch.
Native app-model requests use that admitted mapping and verify the current app
account before using it. An absent app-model catalog requires explicit or
request-driven discovery; known configured/legacy routes retain their existing
admission path. Reading metadata does not authorize model execution.

The model catalog is validated data in the same provider configuration, under
catalog.models, rather than a Rust list of model families or effort mappings.
Each entry declares its public ID/display name and effort-to-host-model route,
with an optional host thinking-effort value. Supply initial defaults as a data
resource; users can add or correct mappings through configuration without a
binary rebuild. The binary owns protocol/schema validation and lifecycle safety,
not the list of models. Listing a configured route proves neither account
entitlement nor successful execution. Actual transport acceptance and stable
package/activation admission are separate; their current results belong in CURRENT.
The later request to expose the app's available models supersedes the earlier
fixed GPT-5.6 exclusion; discovery does not imply entitlement to unavailable
models. Pro remains an explicit choice, never an automatic qualification request.
Retain existing configured routes and the hidden Instant alias as needed. A host API
or tool-return protocol change can still require a new binary/bridge version.
Use the selected model's effective catalog record at an idle boundary; changing
catalog data must not mutate an active lane or replay a pending request.

Editable context controls distinguish native context_window_tokens (also projected
to the appropriate upstream catalog fields), automatic-compaction trigger/headroom,
and encoded/decoded transport byte limits. Support model-specific values. The
user's current1,000,000-token override is preserved as an explicit override; it is
not a verified hosted Chat context capacity or proof that no compaction is needed.
Do not treat the user's 1m override as a measured maximum or impose an arbitrary
32k minimum. Validate positive upstream-compatible integer values separately from
the hard encoded/decoded transport ceilings. An absent token threshold delegates
to upstream defaults; it is not an automatic-compaction disable switch.
No proven transport/model limit is bypassed by inflating a catalog number. Native
compaction caused by model/hash changes is separate from token thresholds; a
disabled token-trigger setting must not promise to suppress every compaction.
Changing a startup catalog creates/reuses only the matching qualified execution
binding; do not hot-edit an immutable package or an active sibling's metadata.

The1m override was a workaround for the user's observed reply/compaction/new-Chat
summary-transfer cycle on successive prompts. Investigate and reduce that cycle,
not just token-trigger frequency. Distinguish compaction reason, checkpoint
acceptance, lane retirement, actual Chat creation and context retransmission.
A checkpoint is acknowledged only after canonical history replacement, not merely
because identical summary text already appears in the old prefix. An acknowledged
checkpoint must be applied once; warm requests must reuse valid
owned conversation/context and send only needed deltas. Do not suppress necessary
compaction, discard history, replay a submitted request or promise that stored
host history increases the model's real context capacity. Deterministic regressions
must cover repeated checkpoints, unchanged prefixes, compaction failure, restart
and model/effort changes before budgeted live qualification. Report message
submissions separately from tool returns and model starts.
For the selected duplicate-presentation behavior, retain chronological progress
and tool events during execution. Once the complete public final is committed,
the TUI folds only earlier progress from that exact physical source message;
each folded entry stays at its original position and can reveal its full text.
Keep the final body complete and distinct reports/tools in place. Provider-owned
presentation metadata correlates native public item IDs with physical source
identity without altering Responses text, native IDs, model-visible context or
canonical records. Missing/unavailable correlation must preserve the full visible
content. Model prompts, effort, generation and execution are unchanged. Replayed
and paginated native history must retain message identity for the same display
behavior; copying/exporting must preserve full source content. Terminal redraws
must preserve unrelated terminal output and reading position.

An ordinary web/Android follow-up in an already bound saved Chat is a new input
to the same native work. Read the exact public source branch after its last
confirmed native-delivered final; do not interpret drafts or arbitrary chats.
Native queue admission uses stable source-message identity, and a canonical
clientUserMessageId/turn match admits observation of that existing Chat response.
Do not submit that prompt to Chat again. A completed saved answer and an active
retained MCP response use the same native history/approval owner. Old tool
capabilities cannot execute new effects; refresh discovery only after the new
native turn has admitted its current tool catalog. Preserve original receipts on
old logical calls. A completed bound Chat may retain a discovery request while
its next source input awaits native admission; this grants no execution authority.
Admission or Stop/owner loss wakes that observer, within the existing retained MCP
observation bound. Unknown enqueue delivery is reconciled, never retransmitted.
Branch changes, account/owner loss and unsettled effects remain explicit failures.
Remote observation must honor source read-backoff without retransmitting model
requests. Serialize separate native/external inputs through native admission;
never substitute a queued ACK for a consumed native turn or live TUI delivery.

Recognize compaction from upstream request metadata and its explicit trailing
`compaction_trigger`, rather than guessing from user prose. Text-summary requests
return ordinary summary text; remote V2 returns one Provider-issued `compaction`
item containing a JSON public continuation record with format
`codex-chatgpt-public-continuation`, version1 and a nonempty summary. Field order
and JSON whitespace do not carry meaning; unknown/duplicate fields and unsupported
versions are rejected. A JSON object with only version/summary is not our record. It is public data, not encrypted reasoning or proof of
issuer identity; foreign opaque checkpoints remain unreadable. The Provider
journal binds its own offered record to the native pre-compaction prefix and
recognizes replacement only after the canonical native prefix changes. The
original native items and signed installation history are never rewritten.
Never execute tools during compaction. A transient summary prompt and reply
do not become the accepted native prefix. On accepted replacement, acknowledge the new native import prefix without
reimporting the already-resident summary or replaying completed tool events.
Reassert ordinary native policy and tools on continuation, preserving the owned
remote Chat and its pending tool/approval state. Chat replacement is a separate decision based on verified
remote availability, capacity or explicit user choice; native compaction alone
never releases a Chat. Reimported summary is contextual data and cannot erase
the Chat model's already retained history or increase its measured capacity.
Old `ocx1:` records are not decoded by the new ordinary path: recover public
context from canonical native records or report the unavailable context instead
of silently dropping it.

A user-authored control-looking tag, public summary, or ordinary handoff wrapper
never elevates the user's role, selects model/effort, or authorizes journal
replacement. Codex strips internal source metadata for third-party providers;
that metadata cannot identify an issued handoff on the actual wire. The local
checkpoint producer uses the existing private Provider journal owner to MAC the
complete public handoff and its native message ID with a separate HMAC-SHA256
domain. Native routing and public-history projection verify that issuance, plus
the same-thread or declared lineage checks. A pasted copy with another message
ID, edited history/route, another owner or self-supplied digest is ordinary data.
The owner secret is never emitted in the message or user configuration.

The CLI may create only the existing private owner marker before the first
handoff; issuance also works while the service holds its journal writer lease.
The TUI passes the exact state from the installed native auth definition through
`checkpoint --state`; runtime HTTP admission uses that same journal owner. A
verified handoff can correlate the selected old-model compactor route, but grants
no tools, permissions or controller lease. Native ownership, approvals and pending
calls still own execution. Never infer a running request's selected model solely
from current thread model/effort, which may reflect a later settings change.

Cross-provider entry may use a readable public-history handoff at an idle primary
thread. TUI reads the complete upstream public transcript and asks the installed
Provider to encode it; upstream alone persists the resulting contextual user
message through `thread/inject_items`. This is not a new user turn or inference,
another history store, a developer instruction, or an opaque checkpoint. Keep
the original native items and thread ID. The message uses upstream's ordinary
external-context wrapper and remains readable by the previous provider if entry
fails. Exclude reasoning and internal metadata; reject unsupported public items,
incomplete pagination, active work and mismatched thread identity. Imported tool
events are quoted historical observations, never calls to execute again.
The TUI's bounded display history is not a handoff snapshot. Paginated native
threads must be read through all ascending turn pages with Full item detail;
repeated cursors or duplicate turns fail the read. The Provider rejects explicit
Summary/NotLoaded item views rather than signing a truncated public history.

For native CLI entry in either model-adapter or operator execution mode, resuming
the same scoped thread may replace its native backend. A genuinely new,
canonically active native turn may rebind that work after verifying its execution
home, workspace, route, original receipts and absence of pending calls/background
jobs. Preserve its
history, counters and budgets; fence the former controller. A repeated old turn,
unknown effect or ambiguous owner still requires explicit reconciliation. An
explicit Bridge join of a completed work, or start of an otherwise admitted
completed/unstarted work, may select the verified replacement owner before
reserving its one new Native turn. It must first verify the same
execution home/workspace/route, retained history/receipts, no pending calls or live
writer; rotate the controller/lease at that completed boundary and persist the
selected owner before turn/start. A read/status observer cannot perform this
transition. The async model exchange must not need another owner rebind after
the returned turn ID has already been recorded. Existing request IDs and uncertain-
result rules still prevent resubmitting that original input. An observation
started before a successful rebind must not revoke
the newer binding merely because its saved snapshot names the previous process.
Apply completion/host observations only to their original native turn, generation
and owner. Managed admission and Stop fencing must be atomic with cancellation
registration. An old exchange finishing after confirmed Stop cannot restore a
running phase; a genuinely new user turn may start after that confirmed boundary.
An asynchronous driver-start failure or subsequent owner/controller fence must
terminate its original native response with an explicit error even if status has
already marked the work for reconciliation. Preserve unknown effects and original
receipts; this error never authorizes a retry, fallback or a completion claim.
After Provider restart, a validated owned MCP journal token may be retired without
reactivating its old capability or calls. Preserve its recorded receipts and the
pending-completion guard; unknown or unreadable tokens remain rejected. A native
owner replacement must not fail solely because the obsolete token is no longer
in the restarted Broker's live map. Durable retirement errors still propagate.

The bounded retained-turn payload cache is not the authority for saved-source
MCP discovery. Cache eviction must preserve an admitted source's discovery chain
and permit its settled predecessor to link to a newly admitted turn. Keep that
current-owner discovery admission separate from payload retention; read only
retired saved-source records from the private journal when a link is absent from
the cache. Fresh execution authority must still come from this Broker's live
turn. A restart admits no source until verified saved-Chat reattachment; Stop
revokes all of that lane's admitted discovery, including evicted links, and a
subsequent new primary must not restore an earlier stopped chain. Reattachment
validates the whole recorded chain while retaining bounded payload memory.

Settling a saved external answer must reuse the app's raw snapshot from its
successful public-source read when the same owner/account, lane/round, final ID
and complete final text still match. Hydrating the SDK must not require another
GET of that already validated answer. Retain at most one transient raw snapshot
per observing lane and release it on settlement, Stop or a new round. A missing
or mismatched snapshot requires a fresh account-paced read and exact final
validation; it cannot acknowledge changed text or cross an ownership boundary.

Only an explicit, validated handoff in the same thread or inherited through
upstream-declared fork/parent lineage can supply the readable replacement for
preceding foreign opaque input. Lineage metadata correlates quoted history; it
does not authenticate an app account or grant permissions. A child keeps its own
native ID and Provider lane, including after cold resume and nested forks.
Unrelated threads without lineage still refuse a mismatched handoff.
Preserve current system/developer
policy and contextual user settings; retain all later input. Never claim to decode
the old opaque state. Text compaction returns one complete Provider-owned summary frame, with the
unchanged public model summary followed by a versioned identifier for its issued
round. Buffer only compaction control output until that frame is complete;
ordinary public answer/tool streaming retains its original flow. The native Core
owns its additional summary preamble. Provider acknowledgment compares the exact
issued frame at a message boundary in committed replacement history and requires
the previous canonical prefix to have changed. It does not bundle, reproduce or
match an upstream natural-language summary template. An old/different round or
an edited/truncated summary cannot acknowledge the current text checkpoint.
The Provider's opaque compaction format is not compatible
with upstream OpenAI compaction: do not advertise a fabricated common compaction
hash. Hash absence does not disable token/window compaction. Ordinary reverse
entry resumes the same upstream-owned history without an additional source-summary
request. Existing readable context and canonical results remain in that thread;
native model-specific instruction updates remain upstream-owned. This does not
promise that every foreign opaque checkpoint is accepted or disable upstream's
conditional compaction. Keep explicit source-summary generation and its exact
completion/interruption handling available for a separately requested context
operation; ordinary model selection does not initiate it. If that explicit
operation fails or is interrupted, preserve its source Provider and history.

First admission may wait for the same owned app target and authentication to
finish loading. Bound this read-only wait; never create a target, change an
account, restart a live owner or infer merely to test readiness. Distinguish a
missing startup target from multiple matching app targets; ambiguity refuses
immediately. A local listener readiness proof is not model readiness.

Only the current ordinary native request is projected as a ChatGPT user message.
The upstream `<environment_context>` block is execution metadata: preserve it only
as context-only developer/system input, never as a user-role message. The visual
hidden metadata flag is insufficient on its own because the live ChatGPT surface
has displayed hidden user-role context. Preserve the original native request and
do not promote the contents of environment values into instructions. Source and
app-wire checks must assert the system/developer role and absence of that block
from user-role messages; transcript visibility must be checked separately from
request construction.

Public message phase is not determined by the app's channel name alone. A
`final` channel message with `finished_partial_completion`, even with
`end_turn=true`, remains commentary before its tool call. Stream its text once;
an unfinished message may promote from commentary to a final answer only after
`finished_successfully` plus `end_turn=true`. Preserve one item ID and text,
never duplicate prose or rewrite already closed output. Emit terminal Provider
failures as `response.failed` with a structured response error that native Codex
actually reads; a top-level `error` event alone is not sufficient. Retain only
static content-free failure codes in the private lane record.
An owned submission's higher-level completion callback also settles successful
app recovery by saved-history polling, which need not emit the original stream's
terminal callback. Observe both through the existing lane epoch and submission stream identity,
including delegated host-approval submissions; stale or locally cancelled
callbacks cannot settle a newer stream within the same lane. A displayed final alone does
not prove completion. Preserve failed/interrupted completion as failure, and
record content-free first transport-error/resume observations so a later recovery
does not erase the cause of the original disconnect. Never replay inference to
obtain a missing terminal or extend a timeout to hide incomplete observation.

The provider should accept a ChatGPT project destination, resolve it in the
currently authenticated Chat account/workspace and create it when absent, as
configured. This is a remote ChatGPT project, not the local CWD, a Manager login
profile or an OpenAI API project. Fields are `history.mode=project`,
`history.project_name`, `history.create_if_missing` and optional `history.project_id`
for an exact existing destination. The configured destination is the default for
new provider-owned conversations. Existing conversations are not moved/deleted
implicitly. A project/retention change applies to new lanes at a safe boundary;
model/effort changes should reuse an existing owned conversation when supported.

`codex provider chatgpt projects list` and `projects use --name NAME` expose
authenticated project discovery and create-or-reuse without inference. The same
`codex_projects` MCP control lets an explicit request to create/use a project
complete both operations: verify its exact owned destination, then atomically
save history.mode=project, project_name and project_id. A failed remote operation
must not report a configured destination; a conflicting local edit must preserve
the verified remote binding for reconciliation, without another creation.
Existing lanes retain their destination. Creation across app entrypoints shares
one resolution lock; uncertainty never authorizes another create.
Remaining setup/management UX must let the user inspect/change the configured
destination and verify conversation membership. Reuse the existing history
configuration and authenticated project operations as the single authority;
do not require optional Manager/TUI or add a second settings store. New Chat
contexts for the same Codex work must retain the chosen destination. Preserve
existing conversations and project context/access settings. This is a future
feature requirement beyond the explicit project controls above;
CURRENT.md owns its priority and remaining qualification.

Host account/workspace identity plus exact project ID binds remote placement.
Reuse an unambiguous owned/readable matching project; duplicate names, missing
permissions and inaccessible IDs require a useful error, not another account or
silent root-level chat. Serialize owned creation and reconcile uncertain results
before retrying. A definite server rejection is distinct from uncertain delivery;
only the latter blocks another creation until reconciled. Use the app's omitted
defaults instead of supplying nullable memory/sharing options. Verify project
membership before sending the first user request;
if creation succeeded but placement failed, report the partial state and retain
the known project for reconciliation. Do not create one project per tool result.
Retire/replace a local-function lane only when its pending tools/results are
settled. Scope any durable remote-ID mappings to native thread/lane ownership;
these are lifecycle bindings, never a second canonical transcript database.

Projects may add shared instructions/files/memory and therefore affect model
behavior. Do not silently change global personalization, shared-project access,
MCP permissions or the native reviewer policy. Verify tool declarations and
instruction priority with project context, including guardian child isolation.
An otherwise owned, private, unshared project retains its existing memory scope;
that scope alone does not make the configured destination incompatible. Never
change project memory settings merely to qualify the transport.
Temporary chat and retained project history are alternative modes; do not claim
their combination is available before the signed host proves it. Project grouping
reduces sidebar clutter, not message count, stored history size or allowance use.
Retain history by default. Any later archive/delete/temporary retention option must
be explicit, scoped and independently qualified; never mass-delete existing chats.

Official product documentation supports projects and shared project context, but
does not prove this signed-app adapter can create or select one using its local
function return transport. Read-only host capability/operation discovery precedes
implementation; then prove project create/reuse/readback without model sampling.
Verify the declared destination before starting inference and read back actual
conversation membership after the budgeted native tool round trip. A new remote
conversation may not exist on the server until its first admitted model request;
metadata-only preparation must not be reported as persisted conversation placement. Use the signed app's own auth and
supported app behavior; do not extract tokens or invent unsupported endpoints.
If the current app cannot express the operation, report the exact boundary and
qualify an owned UI fallback before any ambiguous submission; do not silently
abandon the configured project. Background documentation (not proof of a supported app operation):
[projects](https://learn.chatgpt.com/docs/projects?surface=app) and
[project selection](https://learn.chatgpt.com/docs/reference/slash-commands).

Settings and project acceptance cover fresh/config-profile launch, /model choice,
same-provider model/effort change, cold resume, account/workspace change, concurrent
lanes, duplicate names, deleted/inaccessible projects, uncertain creation and
configuration migration/reinstall/rollback. Inspect actual ChatGPT project
membership and native TUI reports/tool effects, not just HTTP success or counters.
Count setup, creation requests, user-message objects, tool returns, model starts,
retries and compaction separately; actual Chat allowance debits remain unknown
without suitable evidence. Project operations/settings must not spend inference.
