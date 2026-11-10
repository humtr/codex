# 4.6 Component responsibilities and user-flow contracts

> Normative module of [SPEC.md](../../../SPEC.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

### 4.6 Component responsibilities and user-flow contracts

This table allocates responsibilities, not mandatory new processes, modules or
forwarding layers. Existing implementations are retained only where they satisfy
the contract. A specific Native API extension is not accepted merely by this
table; its feasibility, exact scope and affected callers require qualification.

| Responsibility | Decision and mutation owner | Producers/consumers and required boundary |
| --- | --- | --- |
| Qualified runtime and generations | Core/release-builder | Verify, launch, retain, update and roll back admitted artifacts; ordinary Native execution is independent of optional components |
| Conversation, turns, tools, approvals and writer control | Native | Both entrances use the same authoritative execution/history path; real Native effects and original returns remain bound |
| Effective model/inference route and committed context | Native | Provider produces route/capability metadata; entrances request selection; Native commits and exposes settings to all consumers |
| Chat transport, account session and owned app | Provider | Bind the configured source/account, deliver input/results and observe the owned app; no Native execution or second transcript |
| External request admission and correlation | Provider at the transport boundary, Native at execution admission | Retain exact retry identity, scoped capabilities, source references and unfinished external I/O; derive Native completion rather than advance a shadow phase |
| Process/connection availability | Configured supervisor and transport owner | Shared lifecycle implementation consumes instance configuration; validate local readiness and actual remote reachability separately |
| Presentation and user intent | Frontend/Chat host; optional Manager conveniences | Render authoritative events and send supported intents; no independent model-policy, app supervisor or recovery engine |

**Attached Native permissions.** `codex_sessions create` optionally accepts
`approval_policy` (`on-request` or `never`) and `approvals_reviewer` (`user` or
`auto_review`). Omission inherits Native configuration. An explicit selection
uses the existing Native `thread/start` approval/reviewer fields with Termux's
`danger-full-access` sandbox; Native enforces managed constraints and owns the
effective settings. The creation reply exposes Native's confirmed initial
permissions, not another mutable Provider policy record. These fields are
rejected on other actions: start/join/reconnect do not change existing work's
permissions. Exact allocation retries preserve the complete selected input.
Both policies use the same Native tools, approvals, effects and history path.
Auto Review failure never selects `never`; offering Full Access changes no
operational default. Compare bounded actual development runs before recommending
a default, and obtain the user's decision before changing that default.

**Session and inference.** A coherent selected task normally retains one Native
thread across model/entrance changes. Historical source provider/model remains
provenance, not an unconditional prohibition on a later authorized route.
Transition admission validates the destination route, credentials, capability and
administrator policy before committing effective settings. Existing calls and
writers settle or are explicitly reconciled; old control cannot produce late
effects. Failure preserves recoverable prior state and never silently forks.
Without a requested route change, resume retains the thread's saved effective
route instead of silently adopting a new process default. With an explicit
destination, a historical provider definition/login is not required merely to
read its permitted history; requirements on destination use and history access
remain enforced. If decoding or summarizing old context actually requires that
source, expose that dependency separately. Do not treat past provenance as an
active inference credential or silently replace the chosen model.
Provider control validates Native thread identity, execution home and workspace
independently of its selected model route. Reading or controlling already admitted
work must not require the Provider's inference route. Admission of a primary
inference request to Provider still verifies that Native selected that route;
moving this check must not admit stale requests from another route.
Resolve a live thread through its verified Native rendezvous before preparing a
server. A newly started thread can have a live writer before its rollout exists.
Ownerless work uses the common default control server; Native alone resolves saved
route metadata and explicit destination selection. Provider must not choose a
second server from historical provider affinity. Retain execution-home and
writer-authority checks.
Native resume checkpoints a live persistent thread through its conversation
owner before reading the stored view, even before the first turn. The checkpoint
includes pending metadata as well as canonical history; low-level file persistence
alone cannot establish a readable paginated view. Live read/archive use the same
owner. This starts no inference and preserves writer and observers. Storage
failure is reported without replacing that writer;
ephemeral threads are not made persistent as a side effect of resume.
Provider catalog and transport overrides belong to new configured work or
explicit Provider inference selection. Generic work observation/control retains
the route and metadata; default-I/O preparation does not inject a Provider catalog.
Resuming/forking with explicit attached/submitted inference selects that destination
through Native without starting a task. Without this selection, allocation retains
the saved route. Start/join rebinds selected Provider inference through Native at
turn admission, retaining same-route model settings unless explicitly overridden.
This refreshes the current transport even if the Provider's endpoint changed.
Reuse the verified live writer and require its advertised transition capability
before sending route fields to an older server. Attached inference still starts
no second primary Chat request. Unsupported updates are rejected before user input.
Existing Native settings events/persistence are preferred over a Provider route
ledger. Native owns the transition and observer binding even if a qualified
implementation must reconnect a backend. Clients do not independently restart
backends and guess that a matching UUID proves continuity.

The transition has one Native admission/commit boundary. Prepare the destination
provider, model metadata and request client together, then revalidate settled
execution and publish the effective settings through the existing event/history
path. Observer subscriptions must remain attached or have a qualified rebind
that recovers committed intervening events. Selection itself submits no primary
inference, replays no user prompt and does not stop an unrelated thread. A failed
preparation leaves the previous selection and draft usable; an uncertain commit
is recovered from Native rather than guessed or retried as a new user turn.
The maintained Native interface extends `thread/settings/update` with optional
`modelProvider` and `config`. The latter is admitted only with a route selection
and accepts destination `model_providers` and `model_catalog_json`, not executor
or permission overrides. Omitted catalog preserves same-route metadata and clears
the former route's catalog when changing routes, without reopening that former
file. Managed catalog requirements retain precedence. Managed policy is revalidated; unreadable requirements block input. A malformed
mutable defaults file alone does not invalidate a live thread's already committed
route or require that unrelated defaults be repaired before continuing it.
`initialize.modelProviderUpdates` explicitly advertises support; absence means
unsupported even when the upstream version matches. Clients must not interpret
an older server's ignored unknown fields as successful route selection.
Selection is serialized with Native input and requires a settled turn, including
reservations and realtime work; background terminals are preserved. A successful
explicit route change through warm `thread/resume` uses the same admission and
commit operation, preserving observers and jobs rather than shutting down a cache
entry. A successful route reply follows materialization, the settings event and
rollout flush. An append/materialization/flush failure after selection
is reported as unconfirmed persistence and must be reconciled from Native state.
Configuration definitions remain configuration, not credentials copied into history.
The route's request client/catalog can change without replacing the tool executor.
Preserve background terminals/jobs, permissions, approvals and original result
bindings. Thread-idle alone does not prove there are no owned jobs. Recreating a
whole session is not an accepted transition if it loses that state.

**Entrance and inference initiation.** Codex-origin submitted
inference and an already-active ChatGPT response attached through Bridge differ
only where their actual source/transport requires it. They share execution and
context admission. Attaching does not submit another primary model request.
The active Chat's actual model/effort remains a host-owned observation; Native
does not retroactively select or replace that running inference. Record known
source settings as source facts and keep unknown values unknown, separately from
the route committed for subsequent Native-submitted inference.
Initial standalone-Chat import creates/selects Native work once, labels imported
context with its source and reuses that work. One Chat may contain independent
tasks; an account or conversation hint alone does not select or authorize one.

MCP supplies tool discovery/calls/results. It is not assumed to provide every
host user-input or final-response event. Model instructions to call report/finish
are not independently observed host output. Complete automatic entrance
equivalence requires a proven host integration for the necessary input/context/
output events; it cannot be manufactured by polling unrelated saved Chats,
inventing provenance or requiring a special user prompt. Primary Chat inference
must not be blocked merely because Provider's optional Linux app is unavailable.
When a task actually requires an unavailable dependency, preserve its input and
expose the specific failed boundary rather than reset its session or create work
again. Platform/model capability and authorization limits remain explicit.

**Context and compaction.** Native owns canonical context replacement and its
automatic/manual lifecycle. The selected model may generate its summary through
the inference adapter. Provider carries a source-labelled public projection when
the next host cannot consume Native opaque state; projection is not a second
semantic compactor and never deletes or fabricates original history. Initial
Chat import, model transition and Chat renewal reuse that projection responsibility.
Native opaque compaction, public summaries, current policy and tool results keep
their actual types and authority. Images/files need a qualified transfer or
reference path; text-only acceptance is not full multimodal interoperability.

Projection does not decrypt an opaque checkpoint or make full history smaller.
Prefer an already accepted portable Native context plus its subsequent committed
items; otherwise transfer qualified public history when it fits. If neither fits,
Native must obtain one portable compaction through its compaction lifecycle before
the transition. Do not start another Provider summary agent or recursively
summarize on every model change. Preserve opaque originals and source records;
returning to a compatible source must retain their qualified continuation path.
An empty public summary, a public-items API or a Provider JSON checkpoint does
not by itself prove that a portable equivalent of opaque state exists. Export
and compaction interoperability require actual source, context and model proof.
Model-visible context is real request data with token cost, not an OS
environment variable or promoted system authority.

A managed Chat can be replaced while its Native thread remains. A confirmed
Native compaction supplies a new context boundary; a managed Chat's resident
window must be rebound/recreated when required to consume that boundary.
Conversely, remote-context renewal does not force Native compaction. User-owned
Chats are not silently replaced. No remote remaining-context sensor or timer
threshold is inferred from Native token settings. Never wait for a compaction
that depends on the same unfinished MCP call; continue at a settled boundary.

**Availability, provisioning and updates.** Native/MCP admission depends on its
execution/control service and reachable transport, not on starting a Chat app.
Submitted Chat inference acquires its required app/account through Provider.
Configuration and provisioned assets name instances; service code has no account
branches. Preserve login profiles and observe actual identity/readiness.
Cold start, process restart, auth renewal and connector refresh use the same
lifecycle path; no parallel watchdog, account repair script or hidden retry queue
is added. App failure must not shut down unrelated Native/MCP capabilities.

**App failure and subsequent input.** CDP connectivity, target/session presence,
renderer scope, authentication, project verification and model-input readiness
are separate observations. A ready host or a successful catalog read does not
prove that a later input or approval-review request can reach its intended target.
CDP owns command/response correlation and connection/target lifetime; Provider
owns external operation delivery and recovery. A CDP timeout or lost connection
does not prove that an app operation was cancelled or that it did not execute.

Recovery distinguishes failed preparation before dispatch, uncertain Lane open,
an opened Lane without model input, uncertain model submission, and lost stream
or final-result observation. Retain the original operation/Lane/round identity
and recover its outcome through the authoritative app/Native observations. Empty
local counters alone do not establish non-delivery across the app boundary.
Never automatically resubmit a possibly delivered model input or Native effect.
Confirmed non-delivery may resume preparation; uncertain delivery remains fenced
until its actual outcome is reconciled. Re-observation that establishes safe
continuation must update the owning state and unblock that same work, rather
than leave a permanent generic reconciliation failure. An unresolved outcome
reports the specific missing fact and available recovery action.

**Observation and completion.** A physical stream ending, a Native Responses
round handing off tools, an app inference completing and a Native task settling
are different facts. Adapters preserve those meanings through their protocol
projection; a shared `done` label is not sufficient completion evidence. A
snapshot without public output does not prove an empty task, failed inference
or non-execution. Hidden reasoning remains private and is not a substitute for
the missing public completion evidence.

Native owns accepted input, canonical turn/call identity, effects and committed
results. Provider owns correlation and settlement of the corresponding external
inference request. Physical streams/reconnections observe that request; they do
not independently advance task status. A valid new Steer input may produce a
successor request on the same Native turn. Correlate each observation with its
original request; delayed events cannot complete or overwrite its successor.
Retain distinct identifiers where their lifetimes differ, without independently
persisting the same decision at several layers.

The Responses/SSE subscriber is an observer, not the authority to terminate an
already admitted app request. Its disconnection does not itself request remote
Stop, revoke a Native call or permit retransmission. Keep consuming and settling
the original bounded operation and its tool receipts after observer loss; a
subsequent observation recovers only evidence the original owner actually
retained. Explicit Stop, service shutdown, and a Native-admitted replacement
have distinct authority: only a confirmed active-turn Steer with appended
canonical user input may retire its preceding inference request, and a
successor cannot dispatch until the previous owned Stop is settled. A failed
observation is neither remote completion nor permission to cancel or replay.

When Native accepts additional input before an outstanding MCP tool return,
the retained return may carry that input as separate `native_input` context
alongside the unchanged `result`. It is Native-admitted user/context data, not
a new tool execution or elevated instruction. Exact retries retain both values.
Tool handoff is explicitly marked in the adapter observation; it does not emit
the physical app's completion terminal. A confirmed tool return is committed
independently of whether subsequent inference produces a usable final answer.
Stop may confirm the whole original lane after fencing its tool leases and
observing its physical stop, including after observer restart or a rejected
successor round. This lane-scoped receipt is distinct from a matching inference
round's outcome; it never admits another request or settles Native effects.
AppHost consumes the confirmed Stop reply itself; a later snapshot or inference
round match is not required to acknowledge that owned-lane Stop. If the reply is
uncertain, observe the same lane and preserve the actual failure cause. Inference
counters remain correlated with their own round and do not define Stop identity.

Passive renderer-hook disposal ends local observation, not the remote request.
It may detach retained lanes only after each has begun local cancellation and
has no pending submission, active stream or approval. Unconfirmed remote outcomes
and Native effects remain in their original journals with execution fenced;
disposal neither resolves them nor admits replay. This lets a quiescent observer
upgrade/reconnect without making one unknown request own the whole app lifetime.
The bridge revision guard admits new inference only against its matching code.
Retiring an existing owned lane (Stop, release or passive dispose) uses ownership
and its existing cancellation/quiescence contract across revisions; it must not
initialize replacement hooks or admit execution as part of retirement.

Use the existing operation owner to recover partial/lost observations within
the admitted deadline and budget. The app SDK first owns transport resume and
its saved-completion recovery. If these exhaust with a transport failure but
the exact account/response is still streaming, the Provider may reobserve that
response through the SDK resume operation at a verified offset. This never
resends user input. A server streaming indicator permits reobservation but does
not prove model progress; a terminal status must be reconciled with the actual
saved result, not described as a recovered stream. Old callbacks and uncertain
identity/offset never advance a newer request. A committed Native tool result remains a
known result even if its app receipt is uncertain: settle its original return
without re-executing the tool. Reconciliation fences genuinely unresolved
operations, authority or effects and exposes the evidence/action needed to
settle them. It must have a supported exit when safe continuation is established. Confirmed
failures, invalid identities/calls and exhausted budgets remain explicit; no
fabricated success, unlimited observation or automatic mutation replay follows.
A resume-stage HTTP404, like HTTP413, does not prove the admitted server response
has stopped. The existing bounded same-response reobservation may proceed only
with its matching owned token/offset, unchanged account and independently verified
IS_STREAMING status. A terminal/missing conversation never authorizes resubmission.
An authoritative completion with no usable output retains available results
and reports that outcome without permanently disabling the work. Native owns
an explicitly requested fork of a verified committed checkpoint; a child does
not inherit its parent's execution capability or silently replay uncertain work.

Stop confirmation must remain observable after an execution lease retires.
Reobserve the same idempotent owned Stop when an ordinary execution snapshot
cannot confirm it; this must not send another remote cancellation or input.
Native-confirmed Stop with no owned jobs settles that work's observed process
session reservations independently of remote Chat Stop acknowledgment. Unknown
remote progress keeps its own work/input admission fenced; it cannot retain a
terminated Native process lease or block independent Native tools. Non-session
unknown effects still require their own qualified non-execution/settlement proof.
A Native command's observed process exit can settle its matching reservation
even if the model stopped before polling it. Match its original call/process
identity and preserve the original running-tool return and failed exit. The
qualified Native adapter must verify its producer's exit semantics: a terminal
item alone is insufficient when an error/timeout can synthesize that state.
An unknown-failure or timeout status can be synthesized before the owning
process terminates; neither independently proves kernel-observed process exit.
Do not fabricate a successful reply or replay the command. A missing thread/
cache entry or departed observer also does not prove termination.

Native input admission must consume that recovered state. Preserve the
thread/work, history, selected route/model, approvals, writer and original input
identity. TUI consumes a draft only according to confirmed Native admission;
its retained text is not proof that the server never accepted that input. Decide
whether an input can be resubmitted from that admission contract and original
delivery evidence, not from the error label. TUI has no parallel recovery state
machine and Core has no Provider repair executor. Approval-review target failure
does not grant approval or prove general Native execution failure.

One source adapter owns the qualified app interface and its supported app build;
model discovery, selection, input/results and maintenance reuse that binding.
An unavailable renderer scope is an adapter/readiness failure, not a reason to
add another desktop launcher or retry an uncertain request. An external desktop
descriptor and Provider's privately owned CDP pipe are different bindings. The
existing connector-refresh operation must work through a verified current owned
binding, retaining full registered/live input-schema comparison. Do not duplicate
the refresh API or borrow an unrelated app/profile to make that gate pass.
Trusted local maintenance may borrow the current app's CDP client through the
Provider's authenticated loopback listener. It does not acquire a missing app,
export its raw pipe, add an MCP action or create another desktop owner. Bind each
command to the observed app generation and reject a changed generation before
dispatch. The existing connector-refresh script consumes this binding for an
internally owned app and retains its supported disposable-relay binding.
For a descriptor-backed app already acquired by Provider, maintenance borrows
that verified current CDP client too; it neither opens a competing relay observer
nor acquires a missing app. Descriptor generation changes invalidate the borrow.

The route's catalog producer owns conversion to the qualified Native schema.
Validate generated catalogs against their actual Native consumer before publishing
them; preserve complete input metadata through frontend/Core. Do not repair a
missing required field with UI defaults, weaken Native parsing, or overwrite an
unrelated catalog. Live model discovery determines available choices; prompt
wording, a hardcoded account/model list or guessed effort must not select a route.
Control/auxiliary requests use their real protocol purpose, not a prompt-text
classifier. An unavailable capability has a specific observable failure and
retains the original input; it is not a successful response from another model.

Package selection, generated launch/auth references and active service images
must be reconciled by their owning component. A successful package update must
not be presented as complete operational readiness while configured consumers
still bind an obsolete or unavailable image. Retain active jobs/generations and
report pending activation when a safe boundary has not been reached. Invocation
binding must resolve an admitted selected generation without duplicating pins
across profiles/services; its startup cost and locking need actual measurement.

**Replacement order.** Before runtime implementation, settle the route/identity,
Native admission/observer, public-context and service/host lifecycle contracts
together. Compare a Native-owned transition with corrected supported reentry;
do not assume either candidate is implemented. Repeated forks, a pair of
synchronized Native transcripts and a mandatory proxy for ordinary OpenAI work
are not the default solution. Move wrong authority and remove obsolete state
machines in the implementing slice, retaining real cross-boundary guarantees.

**Required user flows.** These are acceptance targets, not claims about currently
available host events or Native APIs. Each row must exercise real context,
inference where required, Native effects, durable results and the receiving client.

| User flow | Session/context behavior | Required observation |
| --- | --- | --- |
| Codex with OpenAI → ChatGPT → OpenAI | Keep one Native thread; commit each selected route; carry a qualified public projection when opaque state is incompatible | Both directions, warm/cold resume, exact prompt/result and once-only effects; no repeated forks |
| Codex with ChatGPT → OpenAI → ChatGPT | The same transition/context contract, irrespective of the first provider | Settings and actual request route agree; Native history and live TUI both continue |
| ChatGPT attached through Bridge → Codex and back | Reuse selected Native work; context must be accepted before Native effects; attaching starts no second primary inference | Real host input/output provenance, original tool-result return and Native writer/observer handoff; model reports alone do not pass |
| Standalone Chat → first integrated work → Chat/Codex continuation | Create/select one Native thread, import available source context once as labelled source data, retain its binding | No fabricated past Native execution; reentry reuses the work and independently observes the source where the host permits |
| Native automatic/manual compaction while using ChatGPT | Native commits the replacement; Provider delivers it to a renewed/rebound managed Chat at a settled boundary | Actual source context changes and following inference; an ACK that merely resets a local cursor does not prove remote-window renewal |
| Cold start, account change, reconnect or component update | Reuse configuration/lifecycle authorities and original admissions; retain active jobs and uncertain returns | Actual supervised owner, authenticated route, refreshed connector schemas and new Native roundtrip; failures retain input and do not replay effects |
| Codex ChatGPT input fails at app/CDP preparation or observation, then user continues | Retain the same Native work and draft/admission identity; Provider re-observes the original Lane/round and settles recoverable state | Pre-dispatch timeout, lost Lane-open reply, lost submit ACK, same-work reinput, missing Composer versus missing Target, and TUI/Native/MCP observations; no duplicate submissions/effects, no approval bypass, specific block only while necessary |

`/fork` remains the user's explicit way to branch work. It is not automatic
recovery for a routine provider change. If a source cannot supply required context
or events, report the exact missing capability and continue independent supported
work; do not mark that flow complete or quietly weaken the acceptance target.
