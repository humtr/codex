# Common interfaces and lifecycle

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


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
