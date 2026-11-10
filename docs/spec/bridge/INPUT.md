# Input preparation and execution settings

> Normative module of [docs/spec/BRIDGE.md](../BRIDGE.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

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
The [Linux app operating guide](../../../providers/chatgpt/LINUX_APP.md#플러그인-생성과-교체)
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
