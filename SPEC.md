# Codex for Termux - Product specification

**Status:** Normative, release-independent product requirements.

This specification describes **what the product must do**, who owns each
decision, and how acceptance is judged. It is not an implementation plan,
a release ledger, a test report, or a claim about the currently installed
version. Named upstream/Termux protocol identifiers may be versioned because
their **wire compatibility is itself a requirement**; no Codex release number
or test-run ID determines the meaning of this specification.

The current implementation, release sources, verified evidence and outstanding
work are reported in [CURRENT.md](CURRENT.md); work priorities and the Pro
handoff live in [PRO_HANDOFF.md](PRO_HANDOFF.md). Development behavior is
defined in [AGENTS.md](AGENTS.md). Version-specific prior adaptations and
historical acceptance are recorded separately in
[release history](docs/history/GENERATION_ADMISSION.md). This specification
must not be edited merely because a new upstream release is adopted or a
new test passes; only a deliberate change to required product behavior does so.

**Normative detailed contracts (part of this specification):**

- [User experiences and optional Manager interfaces](docs/spec/USER_INTERFACES.md)
- [Native sessions, ChatGPT Bridge and provider interfaces](docs/spec/BRIDGE.md)
- [Artifact qualification, updates, rollback and runtime compatibility](docs/spec/DELIVERY.md)

An annex may describe an optional or not-yet-implemented capability. It still
defines its required behavior **if selected**; CURRENT.md, not the
specification, records implementation and verification status. An obsolete
historical policy is not revived merely by mentioning it in a reference.

## Continuous upstream upgrades

This product is a Termux wrapper for upstream Codex. Continuously following new
official stable releases is a permanent requirement, including every release
after the current upgrade target. Upgrade adoption is settled; engineering work
determines how to absorb upstream changes with minimal compatibility friction
and repeatable automation. Preserve upstream features and behavior, retain only
necessary Termux adaptations and remove local replacements made obsolete by
upstream. Adapt this wrapper to evolving upstream interfaces.

Qualification determines when an update can be activated. A compatibility
failure is work to repair the adaptation and existing release pipeline, not a
reason to freeze development on an older version. Preserve the operating and
rollback generations during that repair. Extend the existing official-release
discovery, source adaptation, qualification, signing, promotion and update path;
do not add another version selector or release controller. The current target
recorded in CURRENT.md does not limit this ongoing commitment to that version.

The common release builder owns source adaptation for both hosted and local
executors. Locate supported operations through Rust syntax and their call/data
relationships, preserving unrelated source and upstream additions; private
function names, local binding names, formatting and file relocation alone are
not compatibility failures. Prefer upstream configuration/APIs and remove
superseded adaptations. Generated protocol outputs come from the adapted type
definitions through upstream's generator, not a previous version's binary diff.
Ambiguous or changed contracts remain explicit failures rather than guessed edits.
The upstream generator owns `app-server-protocol/schema`; frontend scope admits
those outputs and captures the compiled-source proof after generation. Format
and lint compare that same prepared tree, and the existing generator archive
retains the complete generated payload.
Native and optional TUI integration patches each bind their original official
base in the patch itself. Executors acquire that exact base; the common Rust
producer uses Git's three-way content/rename merge with a temporary index.
Conflicts leave the checkout and real index unchanged. Frontend assembly owns
payload collision checks and copying, not an independent patch engine. Do not
backdate source files to force compiler-cache reuse; preserve Cargo's normal
freshness checks and reuse qualified artifacts at their existing admission boundary.


An upgrade candidate binds its official source, wrapper source and component
build inputs once. Build results and qualification references are outputs;
registering them must not change source inputs and repeat their qualification.
Reuse independently admitted artifacts with identical production inputs, and
rerun changed verification without rebuilding unchanged executables. Keep source
admission, signing, immutable public readback and non-forced promotion as separate
authority boundaries. The existing controller implements this flow; do not put
a second release manager above it.

The controller registers an accepted composition in main's `release-frontend-v1`
with its exact `release-source-v1` source. The pinned main caller passes that result
to the existing producer; the producer checks the qualified run, source recipe,
artifact and proof. This result never rewrites development build inputs or starts
their qualification again. Base can advance independently while a new frontend
is still qualifying; Full remains on its previous accepted composition until its
new frontend result is admitted. Historical embedded frontend references remain
readable for retained sources; new results use the composition record.

Frontend proof admission checks the nonzero successful summary against unique
actual PASS records and requires every focus scope declared by the qualified
workflow. Added upstream tests do not invalidate that proof; missing scopes or
incomplete execution do. The schema1 focused_tests field remains readable for
retained input/parser consumers, not as the current artifact's exact test count.

The hosted executor can restore a completed raw build only by its exact existing
production-input fingerprint. The common verifier checks the complete raw
inventory, binary digest, official source/version and dependency audit before
transfer. The candidate builder retains its static AArch64 ELF check when it
consumes the runtime. A failed cache check fails the job; it does not silently rebuild or
admit the artifact. Cache reuse skips compilation, not source admission, Android
qualification, signing or public readback. Public signed Base retention remains
the durable runtime reuse boundary. Each executor owns its compiler profile;
the shared schema writer inherits that profile and never substitutes another
component's code-generation settings.

Cross-workflow delivery uses the raw runtime artifact from a successful owned
Core qualification of the exact selected source. Branch-scoped caches are only
an optimization, not the handoff to main publication. Discover the qualified
run through the shared exact-run admission rules, download its unexpired unique
runtime artifact and use the common raw verifier before transfer. A missing or
expired artifact requires normal production; a failed advertised download or
verification fails the job. Artifact reuse skips native compilation and musl
dependency preparation; Android, signing and public admission remain required.

Local and hosted builds derive the same official source tuple. Explicit executor
toolchain profiles retain their own qualification; they must not independently
pin different upstream versions. Core-only admission and activation never acquire
Manager, the enhanced TUI or Provider. Configured optional compositions have their
own integration gates; live connector refresh applies to an installed Bridge,
not to a Core-only installation with no Provider.
## 1. Product definition

The product provides one public `codex` entrypoint that runs the official
upstream Codex CLI correctly on supported Termux environments.

It has four components in one repository: mandatory **Core** for minimal Termux
compatibility and signed runtime lifecycle; optional **Manager** behind
`codex termux` for account/task/window conveniences; optional upstream-based **TUI**
for UI integration; and optional **Provider** for Chat transport/model capabilities.
The minimum execution is upstream plus Core. Tools, approvals, native conversation
records, compaction and writer locks remain upstream-owned. Section4.6 defines the
boundaries; CURRENT distinguishes current signed composition constraints from
source qualification of independently optional components.

Every declared optional artifact must qualify before entering a signed generation.
A missing optional capability cannot disable independently valid Core execution.

This is a clean rewrite. Legacy source is historical evidence, not an
implementation dependency or migration base.

## 2. Product priorities

In descending order:

1. preserve a working, recoverable upstream Codex execution path;
2. never damage system resolver, auth, profile, session, or installed runtime
   state;
3. preserve upstream process behavior at the execution boundary;
4. make installation, update, activation, and rollback crash-safe;
5. keep Core small, fast to build, and fast to execute;
6. add Manager conveniences without giving Manager ownership of Core state;
7. optimize implementation velocity without weakening acceptance gates.

## 3. Public command contract

The launcher classifies only an exact first argument of `update`, `doctor`, or
`termux`. Every invocation with one or more arguments that is not one of those
Core/Manager routes is passed to upstream Codex unchanged after the existing
Termux launch planning. The exact zero-argument bare `codex` invocation has one
narrow Core preflight exception: only when stdin, stdout, and stderr are all
terminals may Core perform the bounded signed-stable update discovery described
below before launching upstream. `update` is a Core-owned safety boundary: the
installed wrapper must never execute the upstream distribution updater, because
that updater can install an unadapted runtime on Termux. The wrapper release
pipeline obtains the official upstream package, applies the accepted Termux
patch, qualifies it, binds the matching Core artifact, and publishes a signed Core-plus-generation bundle; the
installed Core obtains and activates only that signed adapted bundle.

| Command | Owner | Required behavior |
| --- | --- | --- |
| `codex` | Core then upstream | on an all-TTY zero-argument launch, perform only the bounded signed-stable advisory check/prompt below, then execute the active upstream runtime; otherwise execute upstream immediately |
| `codex [UPSTREAM_ARGS...]` | Core | for every non-empty upstream argument vector, execute upstream with original arguments without startup update discovery or prompting |
| `codex --version`, `codex -V` | upstream | print exactly the upstream version output |
| `codex update` | Core | resolve the signed stable wrapper release channel, discover newer official upstream metadata, request its missing hosted build and wait for signed publication; explicit source/transport overrides retain the separately qualified unified acquisition behavior |
| `codex update --help` | Core | print the wrapper-owned update usage without invoking upstream or changing state |
| `codex update --force` | Core | retry exactly the authenticated held public release sequence for this invocation while bypassing only the local rollback-hold comparison; a valid matching hold is required and signature, digest, release-sequence, candidate-probe, and activation checks remain unchanged |
| `codex update --build-local` | Core | resolve the exact official upstream stable metadata/archive, build and probe one local-derived generation, sign with the explicitly configured authorized key or a fresh temporary Ed25519 key, bind to the authenticated public baseline, activate without changing `update_key`, and perform no public upload/promotion |
| `codex update [INVALID_ARGS...]` | Core | reject unsupported updater options or combined selectors without invoking upstream or changing state |
| `codex update --local <DIRECTORY>` | Core | verify, stage, probe, and activate one compatible signed official/operator bundle through ordinary public-authority admission; it is not a local-derived import surface |
| `codex update --remote <HTTPS_BASE_URL>` | Core | acquire one immutable signed Core-plus-generation bundle and activate it through the local coordinated path |
| `codex update --rollback` | Core | atomically activate the retained previous complete signed generation; record a public update hold only when rolling back from an authenticated public generation, while rollback from a local-derived current creates no public hold or rollback guard |
| `codex doctor [OPTIONS]` | Core | combine upstream and Termux diagnostics |
| `codex doctor --color` | Core | explicitly request colored human diagnostics on a TTY, including when an outer Termux wrapper supplied `NO_COLOR` |
| `codex termux [COMMAND]` | Manager boundary | invoke the Manager artifact or report it unavailable |

`codex version` is not introduced. Wrapper/Core/Manager version rows must not
be appended to upstream `--version` or `-V` output.
## 4. Architecture and ownership

### 4.1 Core

Core exclusively owns:

- public command dispatch;
- official upstream artifact qualification;
- Termux runtime adaptation and launch environment;
- FD 33 and FD 34 preparation;
- final upstream process execution;
- sandbox capability enforcement;
- immutable generation construction and integrity metadata;
- update, activation, last-known-good selection, and rollback;
- read-only Core diagnosis;
- composition of upstream and Termux doctor results;
- the typed boundary through which Manager requests Core operations.

Core must remain usable when Node.js, Manager, network access, update services,
or optional Termux APIs are unavailable.

### 4.2 Manager

Manager owns:

- `codex termux` command UX;
- profile selection and presentation;
- notification configuration and delivery;
- Manager-local state and UI;
- profile execution handoff to the validated Core entrypoint.

Manager's profile UX and declared profile state are standalone Codex product
capabilities, assessed through the public `codex termux` path. Their completeness
is determined by how they support the official upstream runtime and user tasks,
not by features in a separate convenience product.
Conversation persistence, discovery, selection and resume belong to upstream,
as established by the shared-conversation contract below; Manager does not own
a second session index or picker.

The Manager executable is built separately from Core and is an optional signed
generation asset. Core does not compile it, discover it from `PATH`, or probe
it during ordinary launch. Release-builder owns the bounded artifact probe;
Core consumes only the signed generation and its digest-bound Manager path.

Manager must not directly write Core generations, pointers, manifests, locks,
runtime state, resolver data, or activation journals. A Manager request that
would mutate Core state must use a versioned, runtime-validated Core contract.
The Manager may create and update only its declared state and an explicitly
created profile-home directory, plus the no-replace shared configuration-profile
links in a validated selected upstream home described below. It must not copy,
parse, summarize, or rewrite
authentication files, cookies, OAuth material, arbitrary upstream state, or
session transcript content. Profile execution changes `CODEX_HOME` only in the
child environment and returns through the stable Core entrypoint; it never
changes the caller's environment or persistent Core state. Manager commands
that need an update, rollback, doctor, or other Core operation must request the
corresponding Core-owned command through that entrypoint and must not
reimplement the operation.

The Manager artifact is optional and independently qualified. Its absence,
incompatibility, or failure must not make ordinary upstream launch, Core
doctor, Core update, or Core rollback unavailable. Manager v1 has no external network,
package-manager, OpenSSL, bwrap, resolver, or generation-discovery authority.
Profile registration creates only Manager metadata and a private new account
home. Selection affects only the child invocation; execution readiness and
shared conversation-path preparation belong to Core before upstream launch.
Optional shared configuration-profile projection belongs to Manager preparation.

An installed generation's unavailable Manager is reported at the Manager
boundary and by doctor; it does not invalidate the independently usable Core
runtime. Ordinary dispatch checks the declared Manager path for a regular,
owner-executable file without probing it or requiring OpenSSL. Installed-release
verification still authenticates the signed inventory, descriptor and all
Core/runtime assets; a failed optional Manager file check is excluded from that
verification result and local-build carry forward, without disabling Core update
or rollback. This exception applies only to already-installed generations.
Fresh candidate admission, staging and activation
retain complete Manager verification whenever the candidate declares one. No
generation metadata, signed inventory or user state is rewritten to express absence.

Manager is a separate Rust executable. No Manager runtime or dependency may
become a prerequisite for ordinary upstream launch, Core doctor, update, or rollback.

### 4.3 Shared contracts

Core and Manager may share only explicit versioned data contracts. Compile-time
types are insufficient; every external or cross-layer payload is validated at
runtime. Unknown incompatible schema versions fail without mutation.

### 4.4 Product completeness against upstream

The completeness boundary is the public `codex` and `codex termux` product.
Core supplies the supported official runtime, Termux adaptation, process and
shared-server execution, generation lifecycle and diagnosis. It must preserve
accepted upstream command/configuration behavior and account isolation; optional
convenience failures must not become prerequisites for ordinary execution.
Candidate publication and activation still require the complete signed inventory
and qualification; optionality does not permit admitting an invalid candidate.

Manager supplies standalone user-facing configuration and selection conveniences
above that boundary. A useful convenience must produce a distinct user result,
not another implementation of a Core command. Profile selection must clearly
distinguish the identity supplied to the upstream child from saved selection
history; history alone is not evidence of the identity used by a later ordinary
launch. Authentication, conversation persistence and thread lifecycle remain
upstream-owned. Session conveniences, if selected in a later contract, must use
upstream-supported discovery/resume rather than a second transcript index.

Notification policy, upstream hook rendering and Termux delivery belong to Manager.
Core combines its execution defaults with a bounded configuration fragment from
the admitted Manager; it does not parse Manager notification records or event names.
Delivery failure must
not fail an upstream turn. Public-path proof must establish the intended event
and delivery behavior; a configuration record or generated command alone is not
proof of the user's completed task. Legacy functionality is evaluated against
these user results and ownership boundaries, not as an automatic feature backlog.

### 4.5 Responsibility and compatibility admission criteria

These are target architecture contracts. CURRENT.md owns implemented and proven
scope. An existing file, service, guard or boundary does not acquire authority by
surviving earlier work. Reassess ownership against the complete requested flow,
including failure and recovery, before implementing another local fix.

1. **One owner per fact and mutation.** Native owns conversation/turn identity,
   execution, approvals, writer authority and committed history. A model route
   is a selected inference source, not the identity of the user's task. A remote
   Chat, transport connection, process and installed generation each have their
   own lifetime; changing one does not implicitly replace the others.
2. **Demonstrated interface sufficiency.** Use qualified upstream interfaces
   first. When they cannot deliver the authorized behavior, name the exact
   missing operation/invariant and compare supported composition, a minimal
   Native compatibility extension and replacement. A source guard proves current
   behavior, not that the product must retain it. Compatibility work must keep
   execution and policy decisions at their authoritative boundary; no second
   tool executor, writer registry or transcript authority is introduced.
3. **Owner of selection and persistence.** Entrances submit user intent.
   Native validates and commits each thread's effective inference route and
   model settings, preserving policy, history and other threads. Provider
   supplies discovered/configured routes and capabilities. Frontend, Manager
   and Provider do not independently persist conflicting selections or model
   a route change as their own conversation restart.
4. **Minimal Core.** Core supplies qualified runtime/environment binding and
   generation/update authority. It does not interpret model catalogs, select an
   inference account or acquire an optional host for ordinary Native execution.
   Optional-component interaction occurs only on an explicitly used component
   path and must not make ordinary Core launch depend on that component.
5. **Derived state and real boundaries.** Consumers derive Native execution and
   completion from Native events/receipts and verified live ownership. Provider
   may persist cross-process authorization, exact retry admission, source
   correlation and unfinished external I/O that Native does not own. Do not
   mirror Native phases as another independently advanced task lifecycle.
   Necessary revalidation across process, time, concurrency and authority
   boundaries remains; cosmetic reduction of guard counts is not simplification.
6. **Configuration and lifecycle.** Account/workspace choices, paths and transport
   instances are configuration or verified discovery. Shared implementation has
   no account-name branches or embedded account paths. One configured supervisor
   owns a service lifetime; Provider owns its optional app lifetime. No temporary
   desktop PID is a boot prerequisite for Native/MCP access.
7. **Proof before replacement.** Establish each old path's consumer and required
   behavior. KEEP necessary contracts, MERGE duplicate decisions, MOVE misplaced
   authority, DELETE replaced machinery in the same qualified change. A temporary
   adapter names its actual missing Native capability and removal condition.
   Compare runtime cost and maintenance with the same inputs; do not replace
   measurements with class/file counts.
8. **Whole-flow admission.** Source success, Native execution, host observation,
   live client display, signed delivery and activation are distinct evidence.
   Qualification exercises cold start, ordinary input, settings transitions,
   compaction, reconnect, interrupted/uncertain delivery and recovery at the real
   producer/consumer boundary. An ignored/zero-test gate is incomplete.

Account homes remain upstream execution identities. Termux socket/environment
adaptation is not permission to relocate authentication or own Native storage.
The accepted sharing policy continues to use upstream's canonical formats and
locks. Revised route selection must not weaken administrator policy or imply
authority to use an unavailable account/model. Explicit failures retain the
conversation, draft and observed results; there is no silent model substitution.

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
An authoritative completion with no usable output retains available results
and reports that outcome without permanently disabling the work. Native owns
an explicitly requested fork of a verified committed checkpoint; a child does
not inherit its parent's execution capability or silently replay uncertain work.

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
## 5. Termux runtime contract

The first supported release target is `aarch64-linux-android` on a supported
Termux installation. Release users must receive a prebuilt Core and must not be
required to install Rust, Clang, or Cargo.

Core must derive runtime paths from the actual environment and must not embed a
single app data path as product authority.

Before final upstream execution Core must:

- construct the qualified runtime environment without leaking package-manager
  or preload variables;
- preserve stdin, stdout, stderr, TTY behavior, signals, and upstream exit
  status;
- open the selected resolver source read-only and make it available on FD 33;
- make the process-local managed configuration directory available on FD 34;
- expose the qualified temporary directory read-only on FD 35 for the bounded UDS adaptation;
- ensure those descriptors survive the final `exec` boundary;
- use the selected official runtime and compatibility tool paths;
- report unsupported Linux sandbox requests clearly.

Core must never create, rewrite, chmod, repair, or delete
`$PREFIX/etc/resolv.conf`. Resolver diagnosis is read-only.

Linux namespace/bwrap sandboxing is not a Termux capability of this product.
Core must not claim that `read-only` or `workspace-write` Linux sandbox modes
are enforced. Ordinary supported launch uses the explicitly selected upstream
no-sandbox policy; unsupported sandbox requests fail clearly rather than
silently weakening the request.

Core must preserve the accepted upstream argv byte-for-byte, including an empty
bare-launch vector. It must not synthesize `-c`, `--enable`, `--disable`, or
`--search`. The Termux no-sandbox default is supplied through the existing
Core-owned system `config.toml` on FD 34 as
`sandbox_mode = "danger-full-access"`, which is also visible to a qualified
shared background server. User-supplied upstream options remain unchanged;
explicit unsupported sandbox requests still fail before runtime I/O. Hiding a
shared-server warning or disabling the server does not satisfy this contract.

The same Core-owned system configuration supplies the Termux default
`thread_unload_delay_secs = 0`. Upstream unloads a thread only after its last
subscriber disconnects and the thread is inactive; a running turn remains
protected by native lifecycle checks. Closing an idle TUI must therefore release
its native writer without the upstream default 60-second cache delay, allowing
immediate same-ID resume through another account's server. Core never removes
or steals a writer lock, copies a transcript, or overrides explicit higher-layer
user configuration or CLI arguments. Authentication remains per account.
This native setting is read when a shared server starts. Already-running servers
retain their previous delay until normal restart; generation activation starts
the new runtime's server without terminating other active clients or work.

Shared-server reuse binds the complete immutable FD-34 settings snapshot under
the existing namespace coordinator lock. Equal settings may reuse the server;
changed Manager hooks or other effective bytes select a matching configuration
namespace without rewriting a live server's files or restarting its active work.
New launch/resume uses the newly selected snapshot. Existing threads retain their
actual upstream configuration until an upstream-supported transition applies it.
Do not add CLI overrides or mutate sibling settings to force a policy refresh.

#### Optional Manager configuration preparation

Optional launch policy is queried only from the admitted Manager through
`__prepare-launch-v1 CAPABILITY -- ORIGINAL_ARGS`. CAPABILITY is `local-frontend`
only when Core can offer that qualified interactive frontend; otherwise `runtime`.
The bounded, no-work query returns exactly `codex-manager-launch-v1\n` followed by
`current\n`, `handoff\n` or `blocked\n`. Manager alone reads account preferences
and decides whether its convenience launcher is needed. Core reads no Manager
preference filename or window recursion marker for that decision. `blocked` stops
without executing another account; unavailable optional preparation falls back
only before handoff, with the unchanged current execution home and arguments.
`handoff` replaces Core with the same admitted Manager's `__execute-launch-v1`
using the original capability/arguments and existing authenticated Core handoff.
There is no post-handoff retry or alternate-account fallback. Manager's explicit
launcher revalidates its selected account before requesting Core execution.

The explicit `__prepare-runtime-v1 CONFIG_DIRECTORY` operation performs best-effort
shared profile projection and returns the configuration envelope below. Core calls
this operation before execution. `__runtime-config-v1` remains a read-only lookup:
it returns the same envelope without creating profile links or changing account
files. Both require the admitted Core handoff and share the same bounds. Neither
starts jobs or performs inference. A malformed saved account preference is an
execution error; it must not fall back to another account. An absent preference
continues to use the native default.

Before final runtime execution, Core may call the admitted generation's Manager
with `__prepare-runtime-v1 CONFIG_DIRECTORY`. This private endpoint requires the
existing Core handoff, never starts a runtime or performs inference/network work,
and receives no upstream workload arguments. It returns UTF-8 bytes beginning with
`codex-manager-runtime-config-v1\n`, followed by the Manager-owned upstream TOML
fragment. The entire response is limited to 8 KiB (inside the existing 16-KiB
shared-server configuration bound) and the call to two seconds;
stdin and stderr are not inherited. Core checks the envelope, byte bound, UTF-8
and absence of NUL, and composes the fragment after its execution defaults into an immutable private
per-launch snapshot supplied through FD34. The shared base directory contains
Core defaults only. Equal complete settings reuse a snapshot; different settings
never rewrite one another. Upstream still owns TOML parsing and configuration
precedence. No account config.toml or credential is rewritten.

Before dispatch, validate optional combined settings using the exact admitted
upstream executable with an inert configuration-loading command, empty private
HOME/CODEX_HOME and working directory, no account credentials, bounded time and
no inference. Cache success only for that immutable snapshot and runtime identity.
If combined syntax/types fail, validate Core defaults and continue on those
defaults only if they pass. Never retry a workload or silently change accounts.

Manager reads its own notification policy and renders the same existing native
completion callback and hook matchers. Missing configuration enables no automatic
hooks; malformed configuration does not become default notification enrollment.
Its system-layer fragment also supplies `approval_policy = "on-request"` and
`approvals_reviewer = "auto_review"` as convenience defaults. Upstream precedence
keeps account/project/config-profile/CLI choices authoritative, including explicit
user review or full access. Core neither selects this reviewer nor interprets
these fields; without Manager, ordinary upstream approval defaults remain usable.
The explicit preparation owns optional shared `*.config.toml` projection from the
private CONFIG_DIRECTORY/profiles registry into the selected upstream home.
Existing local files and dangling links take precedence, and projection does not
copy authentication or conversation data. Projection remains best effort.

An absent Manager causes no call. A failed, incompatible, oversized or timed-out
response is discarded with a concise warning, leaving the Core execution defaults
usable; it must not replay the workload or run an alternate executor. The legacy
`# codex-termux-notify-v1` file prefix remains solely as the ownership marker for
safe replacement of existing Core system configuration. Shared-server snapshots
consume the same complete immutable bytes without a second policy renderer. Managed-home
leases and shared conversation preparation remain Core-owned in this slice.

### Termux permission selection

Approve for me selects the upstream automatic reviewer for native approval
requests. It does not promise a model review for every command. Preserve
OnRequest eligibility and establish a specific missing reviewer invocation
before changing approval behavior. Do not restore Untrusted or broaden admission
merely because ordinary allowed commands showed no review.

For a qualified Termux runtime, the upstream TUI permission picker and its
permission shortcuts must preserve the supported no-sandbox execution policy.
`Ask for approval` selects `danger-full-access`, `on-request` and the user
reviewer. `Approve for me` selects `danger-full-access`, `on-request` and
`auto_review`. `Full Access` selects `danger-full-access` and `never`. Changing
reviewer must not select Linux workspace sandboxing or restart a shared server.
Qualified local frontend resume must explicitly carry the effective supported
Termux permission selection even when it came from Core's system defaults, so
saved Linux sandbox settings cannot override that selection. Preserve saved
approval policy and reviewer unless the user supplied a current replacement.
Fork and side conversations inherit the resulting actual parent selection.
Reject an effective explicitly restricted/custom policy before a new local
thread/turn request; never silently reinterpret it as unrestricted execution.
The Termux picker and shortcuts exclude the unsupported Read Only choice;
explicit CLI sandbox requests remain Core usage failures. User-supplied
named/custom profiles retain upstream semantics. This adaptation
must not impose a profile allowlist that silently falls back from an explicit
restricted request to unrestricted execution.
The UI must describe the actual no-sandbox permissions; auto review evaluates
only approval requests and does not imply that every unrestricted command is
reviewed. Read-only and explicitly requested workspace sandbox policies remain
unsupported; a compatibility adaptation must not reinterpret such explicit
requests as full access. Core does not own approval decisions or add synthetic
CLI overrides. Manager is not required for this capability.

A release-specific UI adaptation requires exact official source and artifact
qualification, bounded byte changes, actual TUI selection and effective native
thread-setting proof before release. A default-setting test or a direct API
request alone does not close the picker requirement. Existing installed
runtimes remain unchanged until signed publication and ordinary activation are
separately accepted.

Core never invokes, installs, downloads, or repairs `bwrap`. An explicit Linux
sandbox-policy rejection is a Core usage/policy failure with process status 2
and must occur before resolver/configuration descriptor setup, upstream
execution, generation construction, bootstrap publication, or activation-state
mutation. A failure of a development runner's own sandbox is infrastructure
evidence only and is not a product-path result.

## Detailed feature and release acceptance

The linked normative annexes define the observable contract for command and
terminal interactions, Bridge transport and conversation continuity, managed
profiles/notifications, release admission and recovery. A successful source
build does not establish installed operation; synthetic reviewers do not
establish live-model approval quality; a signed artifact alone does not establish
client, host or physical-screen effects.

A required user flow is complete only with evidence at every actual owner
boundary it crosses. Preserve original tool results and uncertain operations
instead of guessing from timeouts, resetting identities or duplicating effects.
Use existing qualified Native mechanisms where possible; release-specific
adaptations remain minimal, source-bound and individually qualified.
