# Provider configuration, context and remote projects

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


The inference-context/compaction and cross-provider handoff rules in this section
describe the existing model adapter. The opt-in operator contract above governs
Chat conversation renewal; shared configuration, identity and safety rules still
apply. Native compaction and operator Chat renewal are not one mandatory cycle.

## Configuration authority and precedence

The Provider's single user-owned configuration is
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

## App host and authenticated identity

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

## Response projection and continuation

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

The effective selectable model catalog consists of models and thinking options
actually offered to the authenticated ChatGPT app account. App discovery is
account-scoped and submits no inference. Configuration can label or map an
actually offered route and correct its Native effort translation; it cannot grant
entitlement, expose an unavailable route, rename an app slug into another app ID,
or change the server's Pro lane. User additions do not replace the app's whole list.
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
Configured model entries augment the bootstrap compatibility catalog before app
discovery. After discovery only entries whose actual host model and thinking
option remain offered are selectable. Matching entries may correct the Native
effort translation; unrelated offered routes remain available. Validate supplied
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

## Checkpoint and user-role integrity

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

## App admission and preparation

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

## Project and account binding

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

## Provider-scoped model discovery

The stored Provider ID `chatgpt_desktop` and `chatgpt-desktop/` model IDs are
compatibility identities; the user-facing transport name is **ChatGPT Linux app**.
A model ID, server slug, Provider ID, app account and Native execution home have
separate meanings. Existing thread records are not rewritten for display naming.

`/model`, its inline arguments, reasoning choices and current-provider inventory
use only the selected Provider. OpenAI keeps its upstream catalog, order and
selection behavior and never queries the ChatGPT app merely to list its models.
An explicit `/provider` choice may discover that destination's models before a
settled same-thread reentry. A foreign model passed to `/model` fails without
changing Provider. Other Providers retain their own catalog owner.

For ChatGPT, the existing authenticated `host models` control supplies all
currently offered server models and reasoning options, using `with_app_options`.
Configuration may label or map an offered route; it cannot grant entitlement,
restore `isAvailable=false`, invent an unoffered route or change a Pro lane.
Configured defaults and retained observations are not the current account's
selectable catalog. Discovery errors clear selectable stale choices and preserve
the current thread, selected model/effort and input. No other Provider supplies
a fallback. A model disappearing from the server remains in Native history but
is unavailable for a new selection or reentry.

Fresh discovery uses the existing immutable catalog snapshot producer, with
account identity checked after the app read. TUI consumes that exact private
snapshot and carries it through Core's ordinary catalog binding for a Provider
reentry or cold saved-thread restoration. A new CLI may project an older
service's fresh reply only when its existing journal observation matches the
reply's authenticated account and observation time. Retained/configured-only
responses, changed observations and unknown reply schemas remain failures;
compatibility is a capability contract rather than equal package versions. This never edits shared profiles, authentication, an active sibling's
catalog, the Native writer or old signed generations. Selection is revalidated
before applying Native's session-only model settings. Before accepting a new
ChatGPT input, the existing input barrier checks the selected model and effort
against a fresh account-bound catalog; unavailable choices or a model change
during preparation retain the draft. Steering an already admitted running turn
keeps that turn's own contract. Account changes refresh the current inventory
through the same discovery path.
