# Operating state and product completion

Reviewed2026-10-18. Implementation authority: rewrite/rust-core. Publication: main's
accepted exact-source caller. AGENTS.md owns development rules; SPEC.md owns the
product contract. This file owns current state, remaining work, order and proof.
Superseded trial narratives remain in Git, not as additional current baselines.

## Operating baseline

| Component | Current authority / observed state |
| --- | --- |
| Upstream | Qualified0.163.0, exact source83086188ae0614bd882818ff1aa23f3194962012; recipe release/upstream.json |
| Public and user Full | local-relay-hosted-0-163-0-54f926437ad5-s60; product source54f926437ad5093bb11e8d53c3502b810e514071; published2026-10-17T12:42:39Z |
| Exact Base reused by Full60 | local-base-relay-hosted-0-163-0-b18cc2a2f5f4-s58, unchanged byte for byte; previous user Full57 retained |
| TUI | Native source1c413cf, qualification39027733603;48 maintained native UI tests,94 upstream feature checks and lint |
| Installed optional Chat provider | Immutable payload45072df8448589d3523e, source7bb202e; local-function returns; unchanged during source integration |
| Integrated provider source | providers/chatgpt on rewrite/rust-core, selectively ported from experiment ab41733 plus preserved user1m edits; separate optional offline gate, no operating activation |
| Approved provider implementation | Native Rust adapter under providers/chatgpt, minimal app-side JS and optional viewer assets; Rust conversion not started, current runtime remains JS/Bun + Python |
| Mutable Chat model catalog | All5 entries currently report context_window=max_context_window=1,000,000; auto_compact_token_limit=null |
| Provider source edits | experiment/web-termux has existing uncommitted profile/test changes from32,768 to1,000,000. Preserved unchanged there; their intended1m values are included in the integrated provider source. No operating activation. |

Full60 source acceptance39033120658, independent Core39033121990 and publication
39033383447 pass. Publication binds build, actual Android execution, signing,
immutable Release, complete public readback/disposable update and non-forced CAS
promotion. Main pinb0702ba selects54f; promotionb5e1d9f publishes60. Main's reusable
workflow implementation remains its accepted b18 input. Documentation-only changes
do not change the installed product version or require a release.

Read-only device proof chat-full60-operating-diagnosis.json verifies installed
signature/all8 payload hashes/modes and527 provider package files, retaining the
activation state and user jobs. The mutable model catalog was later inspected
separately; a generated setting is not an immutable source/package change.
Actual deployed Core/Manager functionality is distinguished below from approved
architecture changes. No provider project/configuration facility described as a
target is already implemented merely because this document defines it.

## Components and approved architecture

Names are **Core, Manager, TUI, Provider**. TUI means the upstream-based UI extension,
not Manager's private screen. Current source/artifact paths containing manager-tui
retain their compatibility spelling; moving directories/renaming signed markers
is implementation work, not a documentation side effect.

| Component | Owns | Excludes |
| --- | --- | --- |
| Core | Qualified Termux execution, environment/FDs, home/configuration/socket binding, signed installation/update/rollback | Account/window/quota/model policies |
| Manager | Profiles, quota, notifications/windows, task discovery/handoff policy | Another executor, canonical history, model transport |
| TUI | Upstream UI hooks, native transcript rendering, optional Manager/provider selections | Another backend supervisor; hardcoded provider capability authority |
| Provider | Independent Rust adapter: Chat capabilities/configuration, app-auth context, Responses/tool transport, context/cancel/retry and host lifetime; minimum app-side JS bridge | Native execution/reviewer, Core selection or canonical conversation DB |
| Upstream | Actual inference/tools/reviewer, threads, history, compaction and writer locks | Our release trust and Android window identity |

Every Termux Codex run passes through Core. The normal signed entrypoint stays;
optional dispatch may call Manager for preparation and return an explicit execution
request to Core. Manager is never an unverified alternative launcher. Basic Core
execution must require no Manager query, fragment or enhanced TUI. Keep one
installation authority, no permanent supervisor and no second transcript store.

Existing no-Manager and damaged-optional-file paths pass. Remaining debt concerns
an available Manager's preparation/exec failure and valid-envelope but invalid
upstream settings. The source now uses a bounded versioned Manager launch preparation/handoff
instead of inspecting a preference filename or choosing a window-specific command.
The operating Full60 still uses the previous dispatch until a qualified release.
The optional config fragment remains opaque. The
fragment validates bytes/header/size, not upstream TOML/types. The explicit preparation endpoint now owns shared-profile projection; the separate
configuration lookup is read-only. Safe fallback must know the
same account/home and that no work started; a post-exec/uncertain failure is never
permission to replay. Invalid saved defaults now report the unfulfillable account choice instead of
launching the native default. These source changes are not installed yet.

Keep shared_server, signed-runtime/PID/socket checks, writer/client retention and
opaque catalog isolation. Actual0.163 retains a startup model manager; model/list
cannot replace a catalog. Same home/different catalog bytes must stay isolated.
Do not remove ensure_catalog or move this into an upstream engine patch merely to
make Core look smaller. Generic execution contracts and future policy-only changes
must leave Core/base inputs unchanged after the interface work is admitted.

## Product completion scope and work order

This is the single ordered product backlog, revised for the user's Rust Provider
decision. Active legacy Provider runtime/tests already live under providers/chatgpt;
there is no Rust Provider package yet. The final target is one independently built
Rust adapter in that directory, not another Core module. Current JS/Bun/Python is
the migration reference; only the app-side JS needed by the qualified function
transport and optional viewer assets remain in the product after cutover.
Remaining experimental technical evidence stays on experiment/web-termux until
ported; its historical Core/UI/release snapshots must not replace current
authority. Product completion requires exact-input tests and visible native
behavior. A source merge, successful menu, HTTP response, language rewrite or
preserved output file alone is insufficient.

| Phase | Owner and work | Exit evidence / dependency |
| --- | --- | --- |
| 0. Close current TUI regression | TUI: correct the overlength account-ID fixture; complete the existing compact /switch settings work | Actual48 focused native tests,94 upstream checks/lint,24/32/40/80-column rendering, selection/tab preservation and affected Android UI proof. A failed candidate is not releasable. |
| 1. Core independence | Core/Manager/TUI: finish explicit direct execution and optional-setting isolation; retain versioned preparation/handoff and explicit projection | Same-home normal→direct→normal; absent/damaged/malformed/hung Manager; invalid upstream settings; exact account identity; no duplicate effects or configuration changes. No real Chat required. |
| 2. Rust Provider foundation and transport migration | Provider: independent Cargo package, one adapter process; Rust Responses/parser/SSE/tools, CDP, context correlation and owned-host/cancel/reconnect; minimal versioned app JS bridge | Port relevant experimental drivers alongside code; deterministic legacy/Rust contract comparison without submitting to both; original call IDs, streaming/errors, warm deltas, checkpoints, capability mismatch, unknown dispatch and sibling isolation; actual Android binary/app-bridge proof. |
| 3. Provider settings and generic TUI | Rust Provider: one config/validator, show/effective/edit/set/validate and catalog/profile projection; TUI reads capabilities instead of hardcoded Chat families/efforts | File-only use without Manager; convenience surfaces share the validator; invalid edit/no-op/default precedence and1m preservation; no public command advertised before implementation. |
| 4. Context and native↔Chat handoff | Rust Provider/TUI/upstream APIs: repair unsupported_desktop_route at its actual previous-model/opaque-history boundary; reduce reply→compaction→new-Chat cycles; separate thresholds/wire limits | Fresh/existing long threads, both directions, readable canonical transfer, saved opaque checkpoints, hash/downshift, manual/automatic/interrupted/repeated compaction, cold resume, in-place Chat model/effort. No alias-only repair, lost history or blind replay. |
| 5. Project-backed history | Rust Provider: configured ChatGPT project; resolve/reuse or create when absent; bind account/workspace/project IDs before new conversations | Read-only host discovery, no-inference create/reuse/place/readback, then budgeted tools. Duplicate/inaccessible/deleted projects, concurrency, uncertainty and restart; no silent root-level fallback or old-chat deletion. |
| 6. Complete development and lifetime | Rust Provider/TUI: search/discovered MCP, JSON/freeform/batches, questions, reviewer, errors→repair→tests, parallel work, cancellation/reconnect/latest resume | Exact copied-repository trial, visible initial/intermediate/final plus cold transcript, one effect per original call, acknowledged stop state, surviving sibling, actual observer loss, long work/reboot. |
| 7. Cost/output and alternative transport | Rust Provider/TUI: lazy tool catalogs, warm latency, public markup handling; complete independent retained MCP original-call return | Count user objects/tool returns/starts/HTTP/setup/summary/retries separately; no new user prompt per tool; exact connector ACK and actual observed allowance only. No silently enabled unqualified MCP fallback. |
| 8. Complete experiment integration and retire legacy | rewrite/rust-core: audit branch-specific work, port remaining applicable evidence/drivers, preserve user edits/provenance/licenses; retire superseded product-path JS/Python/Bun after Rust equivalence | One maintained provider authority, all supported paths accepted, no historical Core/TUI/release overwrite, no dual-runtime automatic dispatch. Preserve unique source/evidence before branch/worktree closure. |
| 9. Release and activation | Producer/Core: authenticated optional Rust/bridge inventory, compatibility, install/update/reinstall/rollback/recovery; Base+Full composition | Base unchanged by provider-only inputs; Full retains admitted Manager/TUI; opt-in Chat never forces app/rootfs/login on ordinary launch/update. Sign/readback/disposable update/CAS before bounded activation; validate old-state migration and explicit rollback. |

Finish the already started TUI defect at0, then Core independence at1 before new
provider features. Phase2 establishes the Rust production path; implement new
settings/context/project policy in Rust, avoiding a second feature implementation
in Python/JS. Phase3 supplies the configuration/capability basis for4/5. Port the
two known compaction fixes and their negative controls in2; investigate the larger
observed loop and native transition in4. Read-only project/app discovery may happen
early to expose host limitations, without adding a parallel implementation path.
Phase8 is a final integration audit, not a reason to defer relevant tests until
after their implementation. Run deterministic regressions throughout; real-host
trials follow acceptance of those boundaries. Explain the necessary live model,
submission/retry budget and state mutation scope before each trial. No real Chat
is needed for this documentation revision. Pro is excluded from automatic tests;
GPT-6 comes first, GPT-5.6 remains deferred. Rust migration reduces implementation
and runtime dependency fragmentation; it does not itself resolve host allowances,
app/API instability, project support or private reasoning availability.

### Rust migration scope and preservation

The package target is providers/chatgpt/Cargo.toml and src/; no crate or product
binary exists there yet. Keep it outside the Core/Manager/TUI dependency graph.
Use existing upstream provider configuration and one bounded local adapter; no
new global launcher, second supervisor or general plugin registry is required.
Rust owns HTTP/SSE/protocol conversion, schema/tool IDs, config/capabilities,
context/compaction state, observations, CDP/control pipes, process identity/leases,
timeouts/cancellation and uncertain-result reconciliation. App-side JS is limited
to discovering qualified app capabilities, invoking app-local operations and
delivering events/results; its wire version/bounds are tested with Rust. Optional
viewer assets remain browser code. This is a boundary design, not a finding that
the desktop app as a whole is written in one language.

Within phase2, first establish protocol/state types and fixtures, then move
configuration projection and host/process control from the Python helpers, CDP
and pipe ownership from desktop_cdp/native_chromium_host, and request/SSE/tool
conversion from the adapter and pinned helper dependency. Move local-function
context/cancel/return orchestration next, retaining only app-local operations in
the script. Port retained MCP under the same native-execution boundary; keep it
disabled as an automatic fallback until its separate acceptance. A Rust entrypoint
wrapping all existing Bun/Python modules is scaffolding, not completed migration.

Start from the actual current contract and fixtures, not a mechanical syntax
translation or a wholesale third-party implementation. Independently structure
our Rust adapter; preserve attribution/licenses for reused or translated helpers.
Existing Responses/parser/SSE/tool helpers are reused dependency code today, so
do not rename them into a false claim of original authorship. Remove that runtime
dependency only after equivalent Rust acceptance. Development/CI Python or shell
may remain; product execution must not require Bun/Python solely for the adapter
after cutover. The desktop app/proot or optional viewer may retain their own
qualified dependencies; a Rust binary does not remove those requirements.

Port tests around request admission/model routing, original-ID function return,
stream ordering/partial failure, large packets, hidden instructions versus public
transcript, prefix/delta/checkpoint reuse, concurrent lanes, ownership and shutdown.
Fixture-based differential testing never submits a live task to both adapters.
Do not use legacy-only passes as Rust proof or enable fallback on an uncertain
send. One launch selects one admitted payload. During development keep the live
provider and current credentials/profiles/conversations unchanged; migration uses
disposable roots first. Version persistent mappings/config where needed, diagnose
unsupported older state and preserve user1m overrides. Never discard pending tool
results or rebuild a conversation just to change implementation language.

Integration means the maintained Provider lives on rewrite/rust-core, not in
crates/core. The runtime/license/test port from ab41733 is already committed;
remaining branch work and user profile/test edits still need final preservation
review. Keep source provenance and useful drivers in the owning source/test paths,
proof artifacts in XDG cache and historical narrative in Git. Final branch closure
requires an explicit reviewed integration result with no unintended Core/UI/release
changes and no unique unpreserved work. Do not erase the experimental checkout as
an incidental cleanup. Keep no second active roadmap or production runtime after
cutover; old signed generations remain explicit rollback targets.

Adapt the independent provider gate to cargo format/test/lint/build plus minimal
bridge contract tests and actual Android execution. Optional JS bridge tests need
not impose Bun on the shipped adapter. Core/Base jobs stay package-scoped; shared
lockfile/toolchain changes still get checks for their actual consumers. Preserve
source fingerprints so provider-only policy updates do not invalidate Base. The
release gate includes direct --profile use without Manager/TUI, generic /model
selection without restart, absent/incompatible provider behavior and preservation
of native siblings. Packaging must authenticate the Rust executable, app script
and required assets as one compatible payload, with tested activation/rollback
through the existing signed authority; it must not create a second updater.

Implementation is in progress: /switch now uses Policy/Accounts/Custom tabs and
upstream single-line rows, with real-render tests at24/32/40/80 columns. Native
qualification39088954804 for sourceb55ea78 failed:47/48 focused tests pass, while
the new render fixture fails admission before rendering. Its supposed valid
account ID has65 characters, beyond the64-character contract. Correct the fixture
to the valid boundary and rerun exact native acceptance; do not relax production
validation or claim the render test passed. No new TUI is published or installed.
The controller cleared stale frontend output pins
while the changed recipe awaits successful qualification; public Full60 is
unchanged. The Core launch/config/account source changes pass the full local gate
(135 Python/355 Rust, format/lint/build), followed by the affected66 Manager
profile tests and lint after the final argument guard. The explicitly ignored
read-only Android smoke also passes. Actual signed disposable Android proof passes
saved/inherited accounts, FD36 leases, catalog/native coexistence and exact-byte
reuse, active/idle profile safety, rename/delete data preservation, invalid-account
rejection and Manager-absent native execution. It uses qualified TUI1c413cf;
new quota UI acceptance is separate. Proofs: product-phase1-complete-check.log,
manager-final-profile-gate.log, product-preparation-android.{json,log},
product-android-read-only-smoke.log. No operating activation or real Chat.
Provider integration runs97 JS and28 Python tests without Chat; hosted run
39089428990 passes for exact source63c64cd. This is legacy implementation evidence,
not Rust acceptance. This selected
runtime gate excludes unrelated historical SIWC/probe suites. The source fixes
successful-but-uncommitted compaction poisoning the warm prefix: old source fails
with desktop_context_replaced, candidate keeps one conversation and sends only
one new user delta. A second regression prevents an identical older summary in
the prefix from falsely acknowledging another compaction: only actual canonical
history replacement consumes the checkpoint. The old source falsely acknowledges
it; the candidate refuses and accepts the actual replacement once. No extra model
request, transcript store or service is added. Proof: checkpoint-ack-negative.json.
Content-free observations distinguish append/replay, changed
instructions/tools and host-acknowledged conversation counts. This does not prove
the cause of the user's entire repeated-compaction cycle or allowance deductions.
Provider-only changes now select the independent Provider gate instead of
rebuilding unchanged Core/Manager in source acceptance. The accepted-predecessor
requirement and conservative unknown/mixed-input fallback remain; Base has no Bun
dependency. This does not admit an optional provider merely through Core acceptance.
Existing operating provider and running jobs remain unchanged. Private proofs:
context-prefix-negative.log, context-local-regression.log, integrated-provider-gate.log.

The2026-10-18 implementation authorization also includes tidying /switch rotation
settings: group policy/account controls, retain all capabilities and render every
selectable item on one line at narrow terminal widths. Actual rendering/navigation
proof is required; removing descriptions alone is insufficient. This optional TUI
change does not move quota policy into Core or alter Base inputs.

## Context and easy configuration

The user's1,000,000-token setting is a preserved local policy override, not measured
Chat capacity. It addresses an observed successive-prompt cycle of reply,
compaction, new Chat conversation and summary/context transfer. Investigate the
trigger separately from checkpoint application, lane retirement, actual Chat
creation and prefix retransmission; actual frequency still needs measurement
against the current override. Warm context and an acknowledged checkpoint must
not be reset/applied repeatedly. Start with deterministic no-Chat regressions,
then explain any required live sample budget. A large number does not lift the
HTTP wire/decompression ceiling, guarantee host memory, or disable model-downshift
and comp-hash compaction. Current auto_compact_token_limit=null is not a promise
that automatic compaction never runs. Historical32k proofs do not qualify1m use.

Approved configuration target (not implemented):
`$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, defaulting to
`~/.config/codex/providers/chatgpt.toml`. Editable settings cover native context
window, compaction threshold/headroom, transport limits, default model/effort and
remote history/project destination. Invocation overrides win, then per-model
settings, then shared defaults; native upstream choices remain explicit and their
origin is shown. Generated models.json/profile files and immutable provider code
are not user-editable authorities. No editing multiple account homes or source
constants to change a provider preference. Preserve overrides across reinstall/
rollback and diagnose an older payload's unsupported setting without dropping it.

Provide effective value/source, validate-before-save and atomic edit/set through
the provider's existing invocation. TUI/Manager may offer a convenient view, using
the same validator; neither is required for direct file use. Settings inspection
submits0 messages. Apply at a safe boundary: context/catalog changes select the
matching backend, history/project changes affect new lanes, native sibling settings
and in-flight calls stay unchanged. Exact command names/schema are finalized with
phase3, not advertised as working commands now.

## ChatGPT projects and history

Use a configured remote ChatGPT project for newly created provider conversations;
create it if absent, reuse it thereafter. This is distinct from local CWD, Manager
login profiles and API projects. Resolve in the authenticated Chat account/workspace,
with exact ID after an unambiguous name lookup. Project creation/placement must be
confirmed before first user submission, serialized and reconciled on uncertainty.
Project changes do not move existing user conversations or replay pending tools.
Retain per-native-thread lane ownership and remote-ID mapping without making a
second canonical transcript. Same-Chat model/effort changes should reuse the lane.

Grouping does not reduce message allowance use or total stored history. Retain
history by default; temporary/archive/delete modes need explicit configuration
and their own acceptance, never automatic deletion of user chats. Project shared
instructions/files/memory can alter behavior: verify instruction precedence and
reviewer/tool isolation; never change global personalization or sharing silently.
Host project mode and temporary Chat are not assumed combinable.

Official documentation establishes the user-facing project capability, not a
project API in our installed signed-app transport. Current local-function source
uses retained conversations with no project selection, while an older text
transport requests temporary chats. Neither implements configured placement.
First discover actual app operations read-only, then no-inference project readback,
then budgeted native execution. Prefer app-owned authenticated operations; an owned
UI fallback may be qualified if necessary, but never extract account tokens or
invent an endpoint. References: [projects](https://learn.chatgpt.com/docs/projects?surface=app),
[project selection](https://learn.chatgpt.com/docs/reference/slash-commands).

## Confirmed operating Chat blocker

Full60 itself is installed correctly. Two2026-10-17 attempts to continue an existing
long native conversation using Chat GPT-6 (Extra High then High) fail400
unsupported_desktop_route before Chat submission. Native logs show pre-turn
ModelDownshift compaction requests previous model gpt-6.1-sol through the selected
Chat endpoint. The installed adapter refuses that non-Chat route; lifetime proof
has2 arrivals,2 admission failures,0 admitted requests and0 user submissions.
The transition snapshot also contains an opaque upstream compaction checkpoint,
not a readable ocx1 summary. Its presence is stored-state evidence, not a captured
failed wire body. Renaming the model would leave a second unreadable-context
boundary; an actual no-inference HTTP regression proves both rejection paths.
Do not remove the guard, invent a summary or silently discard history.

Fresh Chat does not have the same previous-native-model transition, but current
Full60 fresh real inference has not been requalified in this diagnosis. Do not
say it has no problems. The later1m setting does not itself establish a repair.
Proofs: chat-full60-operating-diagnosis.json,
chat-full60-cross-provider-admission.json and chat-full60-diagnosis-python.log.

## Delivered native product and retained proof

- Full60 quota-aware /switch is delivered. Cached5h/weekly rows, freshness/errors,
  account pool,5h/weekly/combined/remaining/custom ranking, reserves, order/weights,
  cooldowns, exclusions and session-off are implemented; rotation is off by
  default. Fresh identity/quota/model/effort/permissions and writer release gate
  automatic handoff. No original-prompt/tool replay. Native quota recovery uses
  one checkpoint continuation, proactive idle uses none. Unknown/busy work pauses.
  Chat allowance is not a Codex-quota rotation target.
- Full60 checks:135 repository/publication/frontend Python and353 workspace Rust
  tests, fmt/Clippy; native48+94. quota-qualified-device-matrix.json binds Android
  PTYs, one command effect, preserved settings/UUID and incompatible-permission
  pause. Real account read-only isolation preserves19 processes with0 inference.
  quota-release-public-full-device.json and quota-public-published-manager-ui.json
  prove public57→60/no-op/rollback/forward and published-Manager handoff. Their
  deterministic model is not a real account depletion or Chat allowance test.
- Base/full automation is delivered: independent signed heads, shared immutable
  GitHub Release assets, exact base references, latest-successful-base target for
  failed-Full repair, public readback/disposable update proof and non-forced CAS.
  Release dates/edition/base are producer-authenticated and rendered by Core
  without Manager. Base input fingerprint for Full60 is
  43e0b888ec06af28e2e98d55cdd30adb9f01e5ef9d3e8f691cfab7c6e2a97a3c.
  Preserve current/previous and every referenced base; no date-only asset cleanup.
- Retained core-independence-final.json:30 standalone build/install/update/rollback
  steps. integrated-lifecycle.json and integrated-profiles.json cover resolved
  catalogs, profile/writer/lease paths and Manager absence. Read-only60907ee review
  re-runs20 affected tests successfully; valid-envelope invalid-TOML/schema and
  post-exec Manager failure are not qualified by those successes.
- Actual Android fresh/resume/btw permission proof retains dangerFullAccess and
  independent approval/reviewer, with native commands/guardian and no bwrap.
  Full60 named/legacy fixtures and exact native backend inputs remain the baseline.
- Window return and optional tmux are delivered. Native-origin Termux returns to
  invoking terminal; stock APK uses one dedicated registered terminal then reuse.
  No notification launches/resumes work. Bare/tmux binding, same-window /switch
  and /title field/order propagation have physical user confirmation. Single tap
  consumes a notification. Stock-window-proof38137172382 separately qualifies the
  unmodified APK path. Existing unregistered old tmux origins cannot be inferred;
  live bare↔tmux process migration remains unsupported; nested tmux is refused.
- Question notifications contain structured question text, one per conversation,
  rather than duplicate generic reminders. notification-consolidation-device.json
  records actual Termux:API replacement/grouping and preserved user state. Existing
  proof is not a fresh physical-tap observation for new changes.

## Provider evidence and limits

Installed local-function transport already returns native tool results without a
new Chat user-message per result. Default local transport is separate from the
independent retained MCP path, which still encounters host confirm_action before
Core execution. Catalog refresh success is not original-call result acceptance.
Do not call the reused protocol dependency an independently written Responses
service: parser/SSE/queue/tool mapping/identity/summary helpers derive from pinned
miuuyy6.1.5/source92a356f with licenses and maintained patch. Our transport/lifetime
integration is repository-owned; connector display names are not fixed branding.

Retained signed-app/stable54 proof has2 co-submitted user objects,2 local tool
returns,3 starts,0 Pro/retry and initial/intermediate/final native reports restored
in cold TUI. Source7bb202e passes103 Bun/45 provider Python tests and packaged
admission. Local text setup is hidden where appropriate without changing current
user literals or instruction authority. Warm deltas avoid resending known public
replies/bootstrap, not a guarantee of shared cache across providers.

Deterministic app-port/Core proofs cover search/discovered MCP, freeform/JSON error
recovery, overlapping tool batches/polls, driver-answered questions and observation
loss/reconnect without replay. Actual earlier signed-app trials establish bounded
autonomous repair/tests and automatic/interrupted compaction. The272s copied-ai
trial is useful behavior evidence but lacks unchanged-start-input hashes and is
not exact-byte release admission. Historical tests used earlier32k catalogs and
must not be reused as1m/project/new-Core-layout acceptance.

Remaining: physical question interaction, actual-host output loss/recovery,
hours-long long-context maintenance, remote history/project placement, restart/
reboot, generic model capabilities, full lifecycle/package compatibility and
independent MCP acknowledgments. Public markup such as MemoryCite/unsupported box
must not leak control syntax or be rendered as executable HTML; preserve useful
text and native tool/transcript semantics. Public reports are supported; private
chain-of-thought is not a required output or fabricated transcript.

Count real user-message objects, tool returns, model starts, setup/summary/retry,
HTTP requests and actual observed plan deductions separately. No previous Codex
quota comparison establishes Chat message-limit behavior. Official SIWC/public
Responses research is a separate optional track; app-route acceptance grants no
API entitlement. Alternative browser/CAPTCHA/GUI research must not be revived
without a new technical basis or become a mandatory product path.

## Verification, publication and document authority

For each phase use meaningful deterministic boundaries first; runtime/state/
installer/security changes and release candidates require full acceptance and
actual affected Android execution. Reuse green proof only for identical relevant
inputs/environment/scope. Explain live Chat necessity, model and expected user/
summary/retry sampling before a bounded trial; no automatic Pro or account-drain
experiment. Host read-only discovery and settings validation need0 inference.

The completion matrix must prove native/base without all optional components;
Manager without enhanced TUI; target TUI/provider selection without Manager;
provider via --profile without enhanced UI; Full plus opt-in Chat; supported
upgrade/reinstall/rollback; old/new protocol mismatch; no account or writer
confusion. Show real native progress, exact work effects, actual stop state,
latest-state resume and remote project membership. A heartbeat/file/server ACK is
not sufficient. Signed release publication and bounded device activation remain
separate; preserve existing jobs, credentials and user conversations throughout.

README is the working user entrypoint; SPEC distinguishes deployed contracts and
approved target; this file is the sole product work order. Experimental CURRENT
retains exact technical proof until selective integration and links back here.
Do not maintain competing roadmap copies or append superseded operating diaries.
Historical contracts and immutable assets remain compatible; existing identifiers
are not renamed by terminology edits. AGENTS.md is retained unchanged.

## Related repositories and explicitly separate work

AI wrapper, Termux APK, Tdev, AGY/OpenCode hooks and the broad home/usr cleanup have
their own authorities; they are not this product's implementation prerequisites.
Retained AI formal source e8d49c3 and follow-up1f958e5, Termux humtr/termux origin
extension, window-device-88b0a09.json and ai-formal-source.json remain useful cross-
repository proof; this review does not assert a new audit of their live state.
The Tdev progress/control proposal is a separate-session document, not a required
provider redesign. Source-owned temporary tests stay in TMPDIR, compiler/proof
artifacts in XDG cache, runtime/user data in their approved persistent roots.
