# Codex for Termux — product contract

This is the normative, implementation-independent top-level product contract.
It defines required outcomes and ownership, not the current build, deployment
generation, work order, test report or location of other documents. Functional
annexes refine these requirements without weakening or superseding them.
Changing a source file, version, test result or document location alone does not
change this contract.

## 1. Product and public promise

<a id="1-product-definition"></a>
**P01 — Supported upstream Codex.** The product provides one public `codex`
entrypoint running the official upstream Codex CLI correctly in supported
Android Termux environments, with only necessary adaptations. Upstream tools,
approvals, native history, compaction and execution remain available,
recoverable and authoritative.

<a id="2-product-priorities"></a>
**P02 — Priorities.** Preserve upstream execution and user data; then security,
isolation, signed-runtime integrity and recovery; then compatible upstream
behavior and crash-safe lifecycle; then optional conveniences. Simpler internals
never justify weaker user, safety or admission guarantees.

**P03 — Independent composition.** Core and the qualified upstream runtime
form the minimum product. Manager, enhanced terminal UI and model/Chat Provider
are optional capabilities with separate execution, qualification and
availability. Their failure must not prevent independently valid Core launch,
diagnosis, update or rollback. Selected optional artifacts still require full
admission. Core/Full are the public composition names; required legacy stored
formats remain readable. Component inputs, artifact digests, module revisions,
composition revisions and global signed sequences are distinct identities. Reuse
requires exact production inputs and applicable qualification; existing signed
generations and rollback assets are immutable.

## 2. Commands and observable behavior

<a id="3-public-command-contract"></a>
**P04 — Public routing.** Core recognizes only exact first arguments `update`,
`doctor`, `termux` as its owned command routes. Other nonempty argument
vectors reach upstream without reinterpretation. `codex --version` and
`codex -V` print upstream's exact version; no `codex version` is added.
Only a bare, all-terminal invocation may perform a bounded signed-stable
update advisory before launching upstream; other launches do not.

**P05 — Core commands.** `codex update` uses authenticated, qualified
Termux-adapted releases, never the unadapted upstream installer. Its
`--help`, `--force`, `--build-local`, `--local`, `--remote`, and
`--rollback` modes preserve distinct validation, provenance and mutation
contracts. Unknown/conflicting selectors fail without state mutation.
`codex doctor` combines upstream and Termux diagnosis without modifying the
runtime. Core itself supplies offline `codex termux version`; other supported
`codex termux` operations invoke optional Manager or report it unavailable.
Explicit Core/Full update selection validates authenticated inventory and keeps
rollback authority; Core-only updates never require matching Manager or TUI.

**P06 — Optional user capabilities.** Manager profile, task, window and
notification operations alter only their authorized experience. TUI integrates
upstream interaction; it is never a second executor. Provider supplies a
selected model/Chat transport, not a Native work identity. Invalid or
unavailable choices report specific failures without silently changing
account, model, mode or capability. The selected Provider owns the model
catalog: OpenAI retains upstream selection, while ChatGPT uses authenticated
server/account availability and options. Stored Provider and model identities
remain distinct and compatible; an unavailable historical model is not rewritten.

## 3. Responsibility and authority

<a id="4-architecture-and-ownership"></a>
<a id="41-core"></a>
**P07 — Core authority.** Core alone owns public dispatch, qualified upstream
launch, Termux environment/capability binding, signed-generation integrity,
activation, update, rollback and Core diagnosis. Basic launch cannot depend on
Manager, Provider app, optional UI or network availability.

<a id="42-manager"></a>
**P08 — Manager and UI authority.** Manager owns its declared preferences,
profile preparation and optional conveniences, invoking Core through an
authenticated versioned boundary. It must not write Core generations, locks,
activation or upstream authentication/transcripts. UI presents interaction
without asserting Native writer or release authority. Notification clicks and
optional automatic display share one exact-thread terminal path: focus a valid
existing terminal or construct one observation/resume surface once. Display
never starts a model turn, steals a Writer or bypasses Android Activity policy.

<a id="43-shared-contracts"></a>
**P09 — Native and Provider.** Native owns thread/turn/work identity,
execution, approvals, history, compaction and writer authority. Provider owns
optional transport, authenticated app/account/host binding and unfinished
external I/O. Only Native receipts establish Native effects. Chat, transport,
process, host and Native work have distinct lifetimes. ChatGPT inference
authentication, local service authorization, Native execution home and original
conversation ownership are independently verified; a caller-supplied account
label is never an ownership proof.

<a id="44-product-completeness-against-upstream"></a>
**P10 — Explicit boundaries.** Cross-component state uses versioned, runtime-
validated contracts with a single owner for every decision and mutation.
Unsupported identity, ownership, schema or route refuses safely. Source
presence, selected UI and readiness signals do not prove execution. Reuse
qualified upstream interfaces; do not duplicate controllers, executors,
transcript stores, approval gates or release authorities.

## 4. Input, continuity and concurrency

<a id="45-responsibility-and-compatibility-admission-criteria"></a>
<a id="46-component-responsibilities-and-user-flow-contracts"></a>
**P11 — Exact intent.** CLI and ChatGPT entrances preserve original user
input, effective model/effort, Native work/thread identity, tool returns and
history across admission. Attaching an already-running Chat response does
not submit it a second time. Explicit new inference retains its own identity,
policy, account and cost boundary.

**P12 — Continuity and idempotence.** Accepted inputs, approvals, effects and
results must survive observer disconnection, interrupted app response, transport
limits, ambiguous acknowledgment and recovery without loss or duplicate effects.
Unknown outcomes require original-request reconciliation, not silent replay,
identity reset or a fallback model. Stop/handoff must explicitly fence or
transfer ownership and preserve authorized pending work and history. Automatic orphan
settlement requires positive completion, original results and absent effective
writers under the original lock; uncertainty remains recoverable. For process
effects, completion covers descendants that can continue the effect; parent exit
alone is insufficient. Account or
profile switching distinguishes a verified owner, confirmed free writer and
unknown ownership without replacing the original thread.

**P13 — Independent work.** Eligible independent tasks must remain possible
without unsafe concurrent writes, approval bypass or multiple Native writers.
A resource conflict produces a bounded actionable state, never silent
corruption or indefinite denial of unrelated work. Observation does not
substitute for completed execution proof. The independent Termux MCP shell
remains available without Native resource claims or Provider inference, uses
the common MCP policy and original-call idempotence, and owns no parallel AI,
Native history or approval engine.

## 5. Installation, upstream evolution and safety

<a id="continuous-upstream-upgrades"></a>
**P14 — Continuous official upgrades.** Qualified new upstream stable
releases must remain adoptable; retain upstream additions and remove obsolete
adaptations. Source preparation, candidate qualification, authenticated
artifact reuse, signing, public promotion and activation remain separate,
exact-input-bound decisions. Failures require repair, not weakened gates.

<a id="5-termux-runtime-contract"></a>
**P15 — Termux boundaries.** Real device execution respects Android/Termux
capabilities, Unix-socket path budgets, private temporary resources and actual
Core/Provider/Native writer identity. Unsupported sandbox/host actions fail
transparently. Fixtures or optional components may not alter resolver,
credentials, unrelated jobs or signed state as a workaround.

<a id="optional-manager-configuration-preparation"></a>
<a id="termux-permission-selection"></a>
**P16 — Authorized generation lifecycle.** Only qualified, complete,
authenticated signed generations enter public activation; updates and rollback
preserve a usable previous generation and user state through interruption.
Explicit local-derived generation modes do not silently publish or rewrite
trust keys. Required old signed formats remain readable for rollback.
Services and accounts change only under their owners and authorization.

## 6. Acceptance and contract preservation

<a id="detailed-feature-and-release-acceptance"></a>
**P17 — Whole-flow acceptance.** A required user flow is qualified at each
real owner boundary it crosses. Source/unit, synthetic integration, Native,
live-model/app, visible UI, signed publication and device activation are
distinct proofs. Verify material failure and recovery paths; mock results,
skipped gates and historical receipts never imply full acceptance.

**P18 — Stable contract.** Preserve all approved observable requirements in
functional normative annexes when refactoring implementation or documents.
Changing public behavior or authority requires a deliberate normative
decision and affected-boundary proof. Undelivered optional behavior belongs in
the current-state record, not as an implicit relaxation of product contracts.
