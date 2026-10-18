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
| Optional Chat provider code | Immutable payload45072df8448589d3523e, source7bb202e; local-function returns; separately installed experiment |
| Mutable Chat model catalog | All5 entries currently report context_window=max_context_window=1,000,000; auto_compact_token_limit=null |
| Provider source edits | experiment/web-termux has existing uncommitted profile/test changes from32,768 to1,000,000. Preserve them; this review neither commits nor activates them. |

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
| Provider | Chat model capabilities/configuration, auth context, transport and host lifetime | Native execution/reviewer, Core selection or canonical conversation DB |
| Upstream | Actual inference/tools/reviewer, threads, history, compaction and writer locks | Our release trust and Android window identity |

Every Termux Codex run passes through Core. The normal signed entrypoint stays;
optional dispatch may call Manager for preparation and return an explicit execution
request to Core. Manager is never an unverified alternative launcher. Basic Core
execution must require no Manager query, fragment or enhanced TUI. Keep one
installation authority, no permanent supervisor and no second transcript store.

Existing no-Manager and damaged-optional-file paths pass. Remaining debt concerns
an available Manager's preparation/exec failure and valid-envelope but invalid
upstream settings. Core currently checks a Manager preference filename, invokes
window-specific dispatch and queries an opaque optional config fragment. The
fragment validates bytes/header/size, not upstream TOML/types. Projection of shared
profiles is also a side effect of the Manager query. Safe fallback must know the
same account/home and that no work started; a post-exec/uncertain failure is never
permission to replay. Invalid saved default currently falls back to native default;
the approved target instead reports an unfulfillable saved account choice.

Keep shared_server, signed-runtime/PID/socket checks, writer/client retention and
opaque catalog isolation. Actual0.163 retains a startup model manager; model/list
cannot replace a catalog. Same home/different catalog bytes must stay isolated.
Do not remove ensure_catalog or move this into an upstream engine patch merely to
make Core look smaller. Generic execution contracts and future policy-only changes
must leave Core/base inputs unchanged after the interface work is admitted.

## Product completion scope and work order

This is the single ordered product backlog. Experimental technical evidence stays
on experiment/web-termux until ported; its historical Core/UI/release snapshots
must not replace current authority. Product completion requires the required
capabilities below, exact-input tests and visible native behavior. A source merge,
a successful menu, HTTP response or preserved output file alone is insufficient.

| Phase | Owner and work | Exit evidence / dependency |
| --- | --- | --- |
| 1. Core independence | Core/Manager/TUI: explicit direct execution, optional-setting isolation, versioned preparation/execution boundary, no preference/window-policy interpretation in Core, projection separated from lookup | Same-home normal→direct→normal; absent/damaged/malformed/hung Manager; exact account identity; zero duplicated tool effects; existing jobs/configuration preserved. No real Chat required. |
| 2. Provider settings and generic TUI | Provider validates one user-owned config; show/effective/edit/set/validate; generated catalog/profile projection; TUI reads provider capabilities instead of hardcoded Chat families/efforts | File-only use without Manager; optional TUI/Manager convenience uses same validator. Invalid edit/no-op/reinstall/rollback/default precedence tested. No extra slash command or global launcher is assumed. |
| 3. Context and native↔Chat handoff | Provider/TUI/upstream APIs: preserve1m override, diagnose successive-prompt reply→compaction→new-Chat/summary cycles, warm-delta reuse, separate compaction thresholds and wire bounds, readable canonical state transfer, failure atomicity | Fresh and existing long threads, switch both ways, saved opaque compaction, comp-hash/downshift, manual/automatic/mid-turn/failed/interrupted/repeated compaction, cold resume and chosen model/effort. Checkpoints apply once; no unnecessary lane retirement, dropped history or blind prompt replay. |
| 4. Project-backed history | Provider: configured ChatGPT project; resolve/reuse or create when absent; bind account/workspace/project IDs; create new owned chats there | Read-only host discovery, no-inference create/reuse/place/readback; then scoped live tool roundtrip. Duplicate/inaccessible/deleted projects, concurrency, uncertain results and app restart pass; no silent root-level fallback. |
| 5. Complete development and lifetime | Provider/TUI: native search/discovered tools, JSON/freeform/batches, questions, reviewer, errors→repair→tests, parallel work, cancel/reconnect/latest resume | Exact copied-repository trial, user-visible initial/intermediate/final and persisted transcript, one actual effect per call, original-ID returns, surviving sibling on cancel, real-host observer-loss, hours-long work and phone reboot. |
| 6. Cost/output and alternative transport | Provider/TUI: warm deltas/lazy tool catalogs, latency, public markup handling; complete independent retained MCP original-call return | Separate user-message objects/tool returns/model starts/HTTP/summary/retry counters; no extra user prompt per tool. Confirm exact host call/connector acknowledgment and report its cost; no automatic approval. Both transports share Core execution/transcript ownership. |
| 7. Selective source integration | Current rewrite/rust-core: port active provider modules/dependencies/licenses and regression drivers; separate runtime from research/qualification; preserve unique history and existing edits | No resurrection of old Core/shared-profile/quota/UI/publication code. Port all remaining requirements or mark unshipped optional behavior explicitly; no enabled-but-unqualified fallback. Required supported product paths pass together. |
| 8. Release and activation | Producer/Core: authenticated optional payload inventory, compatibility, install/update/reinstall/rollback and recovery; Base+Full composition | Base excludes Manager/TUI/Provider; Full retains its qualified UI/Manager; Chat opt-in never forces app/rootfs/login on ordinary launch/update. Sign/readback/disposable device proof/CAS before bounded device activation; close experiment after migration and accepted proof. |

Phase1 precedes feature implementation to avoid moving new debt twice. Phase2 sets
the configuration/capability basis for3/4; inspect project host capabilities early
without mutation so unsupported app behavior is found before implementation.
Run cheap deterministic regressions during each phase, not just at the end. A
necessary code move may happen before phase7; stable feature admission does not.
Real-host project/tool/lifetime trials follow deterministic acceptance. Pro is
excluded from automatic testing; GPT-6 is first, GPT-5.6 remains deferred until its
continuation failures are resolved. Do not rewrite all provider code in Rust merely
to unify language or combine separate processes to reduce their count.

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
origin is shown. Generated models.json/profile files and immutable Python/JS code
are not user-editable authorities. No editing multiple account homes or source
constants to change a provider preference. Preserve overrides across reinstall/
rollback and diagnose an older payload's unsupported setting without dropping it.

Provide effective value/source, validate-before-save and atomic edit/set through
the provider's existing invocation. TUI/Manager may offer a convenient view, using
the same validator; neither is required for direct file use. Settings inspection
submits0 messages. Apply at a safe boundary: context/catalog changes select the
matching backend, history/project changes affect new lanes, native sibling settings
and in-flight calls stay unchanged. Exact command names/schema are finalized with
phase2, not advertised as working commands now.

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
