# Fixed annotations and idempotency work

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


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

Native effect diagnosis distinguishes the Provider's work mirror from the actual
process-shared claim ledger. A missing mirror entry never proves shared settlement.
The existing status response exposes bounded, content-free resource scope and
owner-process observations from that locked ledger; failed snapshots remain
unavailable rather than an empty success. Status creates no ledger, removes no
claim and does not infer completion from a departed process. Settlement requires
original Native evidence of the matching process's completed OS wait, or positive
proof from the original dispatch authority that the matching effect never started.
A termination request, Stop acknowledgment, empty job list, unloaded thread,
missing history item, synthetic timeout/unknown exit status or legacy completion
boolean is not that proof. A startup rejection may follow process creation;
its error wording alone cannot establish non-dispatch. Hook execution is a separate
effect boundary: a backend that never started does not establish that an earlier
applicable hook had no effect. Known disjoint resources may progress concurrently;
an opaque shell's unknown effects are not confined merely by its working directory.

Keep completion provenance with the existing original call/session, Native owner,
thread/turn, input fingerprint and receipt. Execution stdout and model text cannot
create that authority. Preserve original results, including unsuccessful exits;
new evidence may supplement a matching immutable receipt but cannot overwrite it
or promote an unproven legacy boolean. A changed result or uncertain identity stays
unsettled. A genuine nonzero OS exit is still terminal evidence when its producer
positively distinguishes it from a synthetic status. A new producer's explicit
unconfirmed completion field cannot fall back to legacy text/exit inference;
legacy omission and new unconfirmed evidence are different states. When one call
makes multiple backend attempts, each attempt must be positively settled before
that call grants terminal authority. A late earlier wait cannot replace the final
attempt's exit. A late witness must preserve the original terminal payload and
call/session/turn identity; it adds evidence without a second public command.
Persist its original unconfirmed receipt and late witness through the same bounded
output projection as canonical command history. This storage projection does not
rewrite the original function result or public live output.

A successful local continuation may separately report `session_running` only
after its own final, unchanged per-tool hook boundary is confirmed. Bind this
nonterminal observation to that continuation call and the actual returned session.
It can settle only the continuation invocation reservation in the existing ledger;
it does not prove OS exit or release the original process claim. Do not accept it
as a terminal command-completion receipt. Explicit null, changed hooks, unknown
sessions and failed continuations retain their original unresolved reservations.

Native bounded execution facts must reflect the selected executor environment and
actual hook dispatch. An unrelated synchronous input-notification hook does not
invalidate a shell proof, and remains enabled for its own matcher. Applicable
PreToolUse, PostToolUse or PermissionRequest handlers invalidate that proof;
match canonical names and permission aliases through the existing hook engine.
Unknown hook sources, dynamic executor/environment overlays, other hook events
and asynchronous work retained across configuration changes remain unbounded.
This admission observation never disables a hook, changes approval policy or
changes the original tool invocation. Missing proof remains opaque. This rule also
covers bounded file patches and ordinary image/clock reads: their hooks can add
unbounded effects. Private execution schema 2 records hook-free tool identities,
including actual matcher aliases, separately; a patch hook does not disable an
unrelated proven command. Legacy schema 1 remains readable for request and receipt
continuity, but cannot grant new bounded admission without current hook proof.
Native's special ToolSearch payload has no tool hooks and remains read-only
metadata discovery, including while an unrelated effect is unconfirmed.

Persisted bounded file claims carry the classifier's scope-proof version. An
older, unknown or partially marked claim remains readable and retains its original
fingerprint, call identity and receipt, but new admission treats its scope as
opaque. Original completion may still settle it. A previous reader may ignore
and drop an additive file-proof field; that round trip must degrade conservatively,
not promote an unproved claim. Queued historical ordinary reads require the same
turn's current-format hook proof before dispatch; Native ToolSearch remains the
known hook-free metadata payload. This does not retroactively arbitrate effects
that an already-running older Provider never recorded. Parallel qualification
therefore applies to the exact participating Provider/Runtime generations; a
mixed-binary activation must account for that older unrecorded-effect boundary.
No source compatibility check authorizes stopping an operating Writer or rewriting
its claim ledger.

Both submitted and attached paths must carry the existing Core-verified Native
execution and canonical workspace facts into their Broker turn. Keep that scope
internal to the verified owner path; remote requests, serialized caller fields,
account labels or a claimed working directory cannot create this authority.
Transport must preserve it until Broker admission, then omit it from app/remote
payloads. A missing proof remains conservative rather than an invented scope.

Settle an original call using its durable admission and reservation identity,
including after a result commit or process restart. The next sampling turn's
hook/execution facts cannot reclassify that earlier reservation. Updating result
receipts and next-turn facts must preserve this distinction under the existing
lock and recovery contract. Only positively recorded unreserved reads may avoid
shared-ledger settlement; missing legacy evidence retains conservative recovery.

An empty `write_stdin` observation may retain exactly one original bounded
same-work/session claim when the current Native turn proves that its applicable
exec/continuation hooks are absent. Both Broker and shared-ledger admission verify
the same original scope; the poll does not revalidate pre-launch output snapshots
or replace the original call identity. Nonempty stdin and continuations without
applicable hook proof cannot inherit a bounded claim merely because the session
is already owned. Before dispatch, the existing admission locks must reject any
new conflict or durably widen the effective blocking scope. A refusal leaves the
previous scope intact. Widening preserves the original input/fingerprint/result
and is monotonic until positive settlement: a later empty poll, aborted admission
or restart cannot narrow or discard it. Keep the continuation invocation and the
original process reservations distinct: the original process's late wait cannot
erase an unresolved continuation or its hook effects. A confirmed nonterminal
continuation leaves the widened original process scope in force.
Unknown or foreign sessions, ambiguous
ownership and missing/legacy scope proof retain conservative admission. A running
poll preserves the claim. A matching original terminal result
or Native command-completion item releases it only with the completed-wait evidence
and original call/session/turn binding above. Public terminal status alone adds no
execution authority. Parallel execution remains observable through continuations.
Native may commit a terminal command item before its last output is collected.
Before releasing that original claim, retain its exact call/session, Native
PID/start/thread and original turn evidence on the existing receipt. An empty
hook-free observation may then retrieve that same uncollected Runtime output,
including in a later turn of the same owner. It does not synthesize a result or
recreate a process claim. Ambiguous/reused session identity, a changed owner,
pending or unknown observations, a collected terminal result and missing legacy
proof cannot grant this observation exception. Repeated original call IDs still
return their immutable original receipts; a new call cannot reuse an expired proof.
After Provider restart or payload-cache eviction, only the currently admitted,
account/branch-verified source-discovery chain may supply cold MCP receipts for
this observation. Read the entire original-to-current chain, including later
collectors and session reuse; restoring just an old completion would revive an
expired proof. These reads do not cache or reactivate retired execution tokens,
restore a Controller lease, change a call ID or rewrite an original result.
Missing, stopped, foreign or unsettled chain entries remain unavailable.

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
or failed loading remains unconfirmed. Stop can settle a positively recorded,
never-offered call through its original admission receipt. An already offered call
missing from asynchronously persisted history remains uncertain; it cannot become
non-dispatched merely because Stop completed. Preserve its original identity and
result for later observation without a replacement submission.
A read-only completion observer never loads or resumes an unloaded thread. It may
use the original source-bound receipt/history chain and qualified Writer absence,
checking every still-relevant effect and reusing already proven original settlement.
Historical tools are not rejected merely for being non-shell, and unknown or
unsettled effects cannot be ignored merely because their turn is old. Incomplete
history, unconfirmed persistence, missing receipts or ambiguous ownership leave
completion unavailable. An append/flush acknowledgment failure is unknown delivery:
a later fresh read of the complete original Native row may recover positive
evidence when its call/input/session/turn/Owner binding is verified. Record that
evidence durably on the existing original receipt before releasing its claim;
neither the failed acknowledgment nor a missing row establishes non-dispatch.
Assignment evaluation requires the current
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
| codex_attach | Offered generation plus accepted source receipt; repeated acceptance observes current context, cannot acquire a different generation. A starting_native generation without a confirmed terminal failure returns pending_native_context; exact Native history-not-ready observation returns pending_native_history. Both are pending reads with accepted=false and no receipt; observe/retry without restarting the task or executing before its real context is available |
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

For direct attached inference, a confirmed failed or interrupted Native turn
ends that turn's result-producing lifetime without ending its development work.
The existing Native observer may fence that same work into reconciliation only
after verifying its exact execution owner, controller generation and latest turn,
no owned background jobs, and committed original call/output pairs plus the
execution-settlement proof above for every offered result that is still missing.
Match the admitted tool identity and exact arguments/input, including a
continuation's own call ID; a public command item may retain the original exec ID.
Hold the work's existing Native admission while verifying and retiring this
generation. Recover only its positively committed original public Native tool
results into the existing Broker receipts. A persistence request or warning-only
flush failure cannot certify that an absent call was never dispatched.
Preserve a result already committed by the normal callback; Native's full stored
output and its model-input truncation are different projections. A late callback
from the retired turn cannot overwrite either result or acquire the next turn.
The original work/thread/history remain intact; explicit same-work claim/start
accepts the next intent without replaying the failed turn. Running and successfully
completed turns do not authorize this retirement. A separately owned AppHost or
source inference is not retired through this direct-attached recovery boundary.
This same observation applies when Native accepted the turn but failed before
Provider request ingestion. A known starting_native turn's confirmed terminal
failure enters the existing reconciliation boundary; attach cannot keep reporting
pending context solely because no Provider stream began. A missing Native turn
receipt, a different latest turn, or unsettled owned effects remain unconfirmed.

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

[qualification.schema.json](../../../providers/chatgpt/qualification.schema.json) defines
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
