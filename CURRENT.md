# Operating state and product completion

Reviewed 2026-10-18. `rewrite/rust-core` owns implementation; `main` pins an accepted
exact source for publication. SPEC owns product behavior and trust boundaries,
README user instructions, this file current state and unfinished work. Superseded
audit narratives and plans remain in Git.

## Operating baseline

| Component | Current authority |
| --- | --- |
| Upstream | Qualified0.163.0, source83086188ae0614bd882818ff1aa23f3194962012; release/upstream.json |
| Public/user Full | local-relay-hosted-0-163-0-54f926437ad5-s60; source54f926437ad5093bb11e8d53c3502b810e514071; published2026-10-17T12:42:39Z |
| Base used by Full60 | local-base-relay-hosted-0-163-0-b18cc2a2f5f4-s58, reused byte for byte; previous user Full57 retained |
| Published TUI | Source1c413cf, qualification39027733603 |
| Retained prior TUI candidate | Source310f0f7e5ce421e61e2c3158406f17e743584557, qualified39107948384; old binding invalidated by changed source recipe, artifact retained/not activated |
| Installed Chat adapter | Immutable c3642b19a87cdace158f; sourceBase9a5436a plus preserved user1m setting; JS/Bun/Python; app-local function returns |
| Rust Provider candidate | Independent Responses/CDP/tool-return/config/catalog/owned-host/lane implementation; actual authenticated Rust host and bounded direct AI development qualified; not installed publicly |
| Experiment | experiment/web-termux at08e9856; user's uncommitted1m profile/test edits preserved, branch/worktree not closed |

Full60 acceptance39033120658, Core39033121990 and publication39033383447 passed.
Main pinb0702ba selects source54f; promotionb5e1d9f publishes Full60. The reusable
main caller retains its acceptedb18 input. The Base fingerprint is
43e0b888ec06af28e2e98d55cdd30adb9f01e5ef9d3e8f691cfab7c6e2a97a3c.
The pilot inventory was read against its actual runtime binding:526 immutable
files, zero mismatches. These observations do not activate the Rust candidate.
Source release/upstream.json now binds frontend15da596 via successful39129327503:
50 focused native tests,94 retained upstream-feature checks and lint passed.
The downloaded exact artifact/recipe was independently verified and an actual
Android no-Manager /model round trip passed. This does not activate a public
generation. Automatic source commits may invalidate obsolete bindings while
retaining published Base/Full and prior immutable artifacts.

The provisioned app remains26.1007.21434 with Chromium155.0.8059.40. The user
reestablished login after revocation/logout. Authenticated request construction
now passes against the expected private account with zero Chat submissions,
including after Electron creates auxiliary index-page targets. Read-only DOM
selection admits the sole main Chat/Work surface; multiple real main surfaces
still refuse. Its observed
app-initial-ed5807835d97.js/app-shared-2d77298f0100.js capabilities define this
bridge's qualified host boundary. App binaries, rootfs, login and account data are
not bundled into the repository or implied by a model listing.

## Components and approved architecture

Every Termux Codex execution passes through Core. Minimum operation is upstream
plus Core; Manager, TUI and Provider are independent optional components, not a
mandatory launch chain.

| Component | Owns | Does not own |
| --- | --- | --- |
| Upstream | Inference/tool/reviewer engine, canonical transcript, threads, turns, compaction, writer locks | Signed generation lifecycle or Android window policy |
| Core | Qualified executable/environment/FDs, home/shared-layout/server binding, install/update/rollback/provenance | Account/window/quota/model policy or Provider transport |
| Manager | Profiles, quota/rotation, notifications/windows/title/tmux and task discovery/handoff policy | Another executor/history store or runtime pointer authority |
| TUI | Upstream rendering and optional /switch, permissions and /model integration | Provider transport/host supervisor or another model catalog |
| Provider | Rust config/catalog, Responses/local-function mapping, Chat account/CDP/host/lane lifetime | Native tool execution/reviewer, canonical history or Core runtime selection |

Core links no Manager/Provider implementation. `runtime-binding` is its small
execution-record contract consumed by optional tools. Core-only operation bypasses
optional preparation/config/TUI but retains execution-home/shared storage/runtime
validation. Per-launch effective settings and server FD34 snapshots are immutable;
optional fragments are admitted by the actual upstream loader before dispatch.
Malformed/hung optional components preserve Core defaults and confirmed home.
No fallback after handoff may replay work or change an explicitly selected account.

The current config-v1 execution record is the only codec. Unsupported records
permit no mutation; preserve their processes/data. Historical experimental codec
emulation and staged reader/writer migration were rejected. Core retains necessary
shared server/config isolation; removal of catalog namespaces is not required to
achieve optional-component independence.

Public naming is TUI. Existing frontend/manager-tui paths and signed markers retain
their spelling until a qualified migration. Catalog model/effort/context data
belongs to Provider configuration; neither Manager nor Core maintains its own list.

## Product completion scope and work order

| Order | Work | Current result and next gate |
| --- | --- | --- |
| 1 | Correct source/contracts, settings and /switch presentation | One-line narrow rows, config/catalog CLI and source audit corrected; native TUI50 focused +94 retained upstream tests/lint passed |
| 2 | Core independence | Core-only, optional absent/broken/hung/config isolation and signed Android57-step proof passed; final composed release still required |
| 3 | Rust vertical path | Actual native commands/custom patch/results/transcript and authenticated Rust-owned app passed, without Bun/Python host |
| 4 | Shared config/catalog and TUI | Mutable data authority, generated command-auth profile/catalog and same-provider model/effort source passed; actual admitted TUI composition/activation pending |
| 5 | Native context transitions | Manual/automatic/interrupted compact, one-time checkpoint/warm deltas and actual Android bidirectional API re-entry passed, including real Chat summary; final TUI artifact and pending-result/cold-resume/downshift admission pending |
| 6 | ChatGPT project destination | Actual create/reuse/project-bound inference and saved membership passed; instruction priority and guardian/sibling isolation need proof |
| 7 | Lifetime and real development | Actual ~/prj/ai analysis→failure→patch→tests→review and same-thread recovery passed; native reviewer allow/deny, concurrent lanes and isolated stop pass without Chat; native blocking question/answer passes at the app-server API; actual question TUI and host crash/reboot remain |
| 8 | Independent MCP and efficiency | Native three-tool retained HTTP fixture passes. Dedicated tunnel/app installation, discovery and immutable runtime recovery pass alongside Tdev. Actual app submission reached host confirmation; no native MCP effect yet. Audit repairs project-memory admission, live preflight, confirmation waiting/correlation and counters; final live return remains pending |
| 9 | Retire experiment/runtime | Preserve useful unique proof/drivers/user edits, finish one Rust authority and optional viewer/provisioning; no product Bun/Python solely for adapter |
| 10 | Package/release/activate | Authenticated optional artifact inventory, final composition/install/update/rollback, signing/public readback/disposable proof/non-forced promotion, then bounded activation |

The user's required AI trial was run directly in ~/prj/ai, not a copied project or
worktree, after the Rust no-MCP execution/context/project path was established.
The model performed the actual source work. Future trials must likewise inspect
AGENTS/Git and preserve uncommitted/untracked content before mutation; Git history
alone does not preserve it. Supervisor review and applicable tests remain mandatory.
Do not substitute the old adapter or a supervisor-made patch for model development.

Complete independent source work while hosted checks run; admit releases in their
dependency order. Do not stop after each individual file change or invent a new
planner/checkpoint hierarchy. Existing user permission and verified remote ownership
remain valid; no fresh user input is presently required for ordinary source work.

## Context and easy configuration

The Rust authority is $XDG_CONFIG_HOME/codex/providers/chatgpt.toml, default
~/.config/codex/providers/chatgpt.toml. It owns settings and catalog.models.
Initial routes are default-models.toml data, not hardcoded Rust model families.
GPT-5.6 is excluded; retain GPT-6, manual-only Pro and hidden Instant alias.
Listing/editing a model is not entitlement or execution proof.

The candidate CLI supports models, catalog, config show/effective/validate/set/
unset/edit and prepare --home PATH. Related fields commit atomically; cooperating
writers serialize, stale editors refuse overwrite, invalid/no-op edits preserve
original content. Effective output reports file/default/per-model origins and
runtime_application=not_queried. The installed JS adapter does not consume this file.

Provider reads host routes/history config at request admission. Catalog snapshots
and generated upstream startup settings remain immutable: prepare again for the
next startup rather than editing a bound server. Per-model context overrides file
settings; explicit native invocation overrides win. /model is primary; generated
upstream --profile chatgpt is the fallback. Manager is not required. No bearer is
stored in the profile. `token`/prepare may lazily start the same Rust executable
against a provisioned rootfs; concurrent startup serializes. `status` is read-only.
A failed readiness check never authorizes restarting an existing live owner.

The user1,000,000-token setting is preserved policy, not measured Chat capacity or
a qualified ceiling. Do not impose an arbitrary32k minimum. Native token triggers,
model/hash compaction, HTTP byte bounds and true host capacity differ. An unset
explicit threshold delegates to upstream, not an all-compaction disable switch.

Responses Lite declarations are transport metadata, not canonical conversation
items; compaction may remove/reinsert them without changing the accepted prefix.
Compaction is recognized from actual request metadata/explicit V2 triggers, never
user prose. Summary-only policy and replies stay transient until upstream commits
the exact offered checkpoint. Retire a settled lane once after acknowledgment;
identical summary text in an unchanged old prefix is not acknowledgment. Reassert
normal native policy after control operations. V2 ocx1 encodes public readable
summary, not private reasoning and not decrypted foreign OpenAI context.

Source `checkpoint` serializes a complete idle public native snapshot as readable
external user context, with no credentials, app startup or inference. Upstream
persists it with its existing inject API, preserving original items/thread ID and
the old provider's ability to read them after an entry failure. Provider admission
accepts only an explicit handoff in the same thread or native-declared fork/parent
lineage before foreign opaque input can be
projected from public history; current policy/context settings and later input stay.
Unknown public item types refuse; reasoning/internal metadata are excluded.
The catalog no longer advertises a fabricated opaque compaction compatibility hash;
token/window compaction remains active. Source TUI waits asynchronously for the
source Chat Provider's readable summary before reverse entry, retaining the source
on failure/stop/event loss. These source changes are not an activated composition.
The real summary trial used two app model starts in33.9s, observed four user-role
objects, no tool return/Pro/retry and unknown Chat allowance debits. Native provider
responses in that trial were local fixtures; actual Chat inference and both native
process re-entries were real. It preserved the exact continuation marker, three
explicit native user messages, original public history and foreign opaque input.
One separate compaction turn is not another explicit native user request. The
current native user/context journal remains upstream's authority.

Public output includes initial/intermediate/final reports and actual native tools.
The app's final channel alone is not a final answer: finished_partial_completion
with end_turn=true can precede a tool. Stream once and promote an open item only
when successful end-of-turn is established. response.failed preserves native error
reasons; static last_error records contain no request/credential text. Known MemoryCite tags and box containers now have a stream-safe Markdown
projection, preserving useful text, optional titles, unknown markup and literal
code. Every UTF-8 prefix and native warm/cache/tool-result paths pass without
extra app submissions. Raw app/native tool data is not rewritten. Nested JSON
argument overhead and actual app sampling of markup remain separate work.

## ChatGPT projects and history

Retain history by default. Project mode uses history.mode, history.project_name,
optional exact history.project_id and history.create_if_missing (true by default).
This is a remote ChatGPT project, not CWD, Manager profile or API project.
The bridge uses the installed app's authenticated list/read/create operations;
no account bearer is exported. Current admitted destinations are owned, private, unshared
projects; their existing memory scope is preserved. Shared destinations refuse.

Bindings include account/user/workspace and exact project ID. Reuse an unambiguous
name, or select its configured ID. Duplicate/deleted/inaccessible destinations do
not create a replacement or silently fall back to root chats. Serialize creation;
record intent before dispatch. Reconcile uncertain delivery before another create;
a definite rejection is separate. Use app defaults rather than nullable memory/
sharing options. Destination changes affect new lanes; model/effort changes reuse
settled conversations. Do not move/delete existing chats or create one per tool.

Actual metadata create/reuse/readback spent zero Chat submissions. The native
project trial verified the declared ID before streams and read back persisted
conversation membership after inference. Project instructions/files/memory still
require priority and sibling/guardian isolation proof; do not change personalization,
sharing, native reviewer or MCP permissions silently. Grouping reduces clutter,
not stored history or established allowance units. Temporary/archive/delete modes
are not admitted and must never mass-delete history.

## Confirmed operating Chat blocker

Installed Full60 long native→Chat transitions failed400 unsupported_desktop_route
because ModelDownshift compaction sent the old gpt-6.1-sol model to the Chat
endpoint. Two requests were rejected before any Chat submission. The retained
conversation also has an unreadable upstream opaque checkpoint. An alias rename
or the1m setting does not repair either boundary. Keep the conversation.

Source Rust manual compaction and fresh native Chat now have separate real proofs;
actual Android TUI transfer now passes against an unsigned candidate in a signed
disposable composition, but final hosted artifact admission is unfinished. Obtain an upstream-owned readable
handoff at a settled boundary before routing it to another Provider; never decode,
drop or relabel opaque state or replay tools. Evidence: chat-full60-operating-
diagnosis.json and chat-full60-cross-provider-admission.json in the acceptance root.
Fresh startup is not blanket proof of all long conversation transitions.

## Verification and publication

Private acceptance root: ~/.cache/codex/acceptance/development-audit-1t4txdwt.
Logs/state there may contain private trial transcripts/account bindings; do not
publish them wholesale. Repository records contain only safe scope/results.

| Proof | Scope |
| --- | --- |
| core-settings-android.json |57 signed Android steps; actual config RPC/FD34 normal→direct→normal with live sibling, optional syntax/type/envelope/hung failures, TUI without Manager; no Chat |
| provider-context-project-full.log |56 Rust/101 Bun/28 Python, fmt/clippy/release build before streaming repair |
| provider-owned-readiness-full.log |58 Rust/102 Bun/28 Python, fmt/clippy/release build; startup target/auth-loading waits and immediate ambiguity refusal |
| provider-native-fixture-final.log | Actual Codex0.163 generated profile/auth/catalog, two commands/custom patch/validation, three effects; synthetic app replies, no Chat |
| provider-phase-native-android.log | Actual native partial-output/final promotion and manual checkpoint→same-thread continuation; synthetic app replies, no Chat |
| provider-readable-handoff-native.log | Actual Android inject/read/next-turn fixture preserves opaque input, same ID and public turns; hidden user-context projection, no inference at injection, synthetic app replies/no Chat |
| provider-bidirectional-handoff-native.log | Three actual Android fixtures: manual compact, public handoff, source→Chat→source across process restarts; readable return, same ID, no Chat |
| provider-native-fork-initial.log / provider-native-fork-fixed.log | Actual native fork exposed an inherited-marker thread mismatch; declared native lineage now preserves public context for child warm/cold resume and nested fork, with three independent lanes and unchanged parent; no Chat |
| provider-handoff-live-native.log / provider-handoff-live-state | Real GPT-6 response and readable summary across native API provider re-entry, two app dispatches/four user objects, source responses synthetic; not yet actual TUI/device activation |
| provider-native-mcp-discovery.log / provider-native-mcp-discovery-fixed.log | Actual native deferred search first exposed a prefix defect: upstream drops function-only name from a search item. After canonical normalization, discovery→one MCP effect→commands/custom patch/validation→final passed; six fixture dispatches, five returns, no Chat |
| provider-gateway-correction.log / provider-native-correction-regressions.log | Rejected gateway batch returns a structured error on its original ID; no partial native execution/new user input. Actual correction→search→MCP→patch/tests passes with six Responses requests/seven fixture app dispatches, five native returns/one error return. Three-correction limit, lost reply/no resend and account-change refusal pass; no Chat |
| provider-native-reviewer-final.log / provider-native-reviewer-composed.log | Actual Android danger-full-access + on-request + auto_review; two reviewer decisions per case, distinct primary/reviewer gateways, schema-valid allow/deny, three allowed effects versus one permitted patch and zero denied-command effects; synthetic app/no Chat |
| provider-reviewer-full.log |66 regular Rust/102 Bun/28 Python plus fmt/clippy/release build; actual native companion run separately passes five executor/reviewer/MCP and four context/fork fixtures, no Chat |
| provider-lane-observation.log | Content-free read-only lane/CLI inspection distinguishes running, native tool-result wait and acknowledged stop without app queries or service startup; concurrent sibling completion/parent stop leaves sibling result and exact retries intact; no Chat |
| provider-tui-handoff-android.json / provider-tui-handoff-android-slash.log | Actual Android PTY/Core/disposable signed composition with728cf26 candidate: /model native→Chat (no inference), Chat turn→one synthetic source summary→native turn, same ID/public turns/readable return and unchanged permission identity. Real Provider checkpoint command, synthetic Responses; candidate failed hosted test compilation so this is not stable artifact admission |
| provider-guardian-isolation-full.log / provider-reviewer-project-isolation.log |69 regular Rust/102 Bun/28 Python, fmt/clippy/release build; actual native guardian_review metadata keeps reviewer opens outside the primary configured project for allow and deny. Existing project-bound review lane refuses before app access. No Chat judgment sampled |
| provider-presentation-full.log / provider-presentation-native-prefix.log |72 regular Rust/102 Bun/28 Python plus fmt/clippy/release; every UTF-8 presentation prefix stays append-only, literal code/native results preserved, rendered native prefix/cache reuses two app submissions |
| provider-native-question.log / provider-presentation-native-composed.log | Actual Android blocking native question answers Alpha on original host call; one native user turn, two synthetic app dispatches, no user resubmission. All five native executor/reviewer/MCP plus five context/fork/question fixtures pass. Physical question TUI not observed |
| provider-android-executable-smoke.json | Hosted39127309252/source45a500e Android API24/Rust1.98 candidate inventory matches and executes help/models/config validate on Android without config/app/state writes or Chat. Unsigned, not stable install admission |
| provider-explicit-stop-android.log / provider-explicit-stop-full.log | PID-bound explicit service stop works on Android, rejects a forged start-time record without stopping the real owner, preserves journal/profile and supports same-endpoint restart;72 Rust/102 Bun/28 Python plus fmt/clippy/release; no Chat |
| provider-tui-handoff-final-android.json | Actual same-thread PTY round trip with ff2682d executable8d84f1dd…, real checkpoint encoding, four synthetic Responses including exactly one reverse summary; no Chat/operating mutation. Exact composed input digests retained; hosted full TUI acceptance still finishing |
| provider-viewer-wire.log / provider-viewer-owned-android-final.log | Actual Rust HTTP/WebSocket/private RFB; capability versus Responses auth, origin/traversal/symlink boundaries, controls and peer shutdown. Fresh official-app profile boots/serves actual RFB from the same service, concurrent auth coalesces, service/viewer credentials rotate on restart; no existing profile or Chat, physical UI not observed |
| provider-viewer-qualified-serial.log / provider-viewer-cdp-auth-readonly.log |74 regular Rust/102 Bun/28 Python plus fmt/clippy/release; WebSocket stack unified at0.29, actual authenticated relay capability/identity preflight stays green without hooks or Chat. An earlier parallel-Cargo run hit generated executable ETXTBSY; serialized full gate passes |
| provider-context-seven-android.log / provider-context-seven-lint.log | Seven actual Android context fixtures pass: automatic token-trigger summary occurs once, and an interrupted summary commits no checkpoint, preserves the latest native work and resumes in the same thread. Three/four synthetic app dispatches respectively; no Chat submissions; clippy passes |
| provider-terminal-identity.log / provider-terminal-identity-full.log / provider-terminal-identity-native.log | Account change during model work refuses before executable calls/completion; restoring the original account reconciles the original round with one total app dispatch.75 regular Rust/102 Bun/28 Python, fmt/clippy/release and five native executor/reviewer/MCP plus seven context fixtures pass; no Chat |
| tui-admission-39129327503.log / provider-tui-accepted-no-manager.json | Successful final TUI source15da596 matches the current recipe and exact8d84f1dd binary. Current Core e83083 with Manager absent passes actual Android native↔Chat /model, same thread/permissions, four synthetic responses/one source summary and real Provider checkpoint; no Chat or operating activation |
| provider-mcp-native-return.log / provider-mcp-composed-regressions.log | Rust HTTP MCP returns actual Android command/patch/verification results on three retained calls with one synthetic app submission. Exact retry, changed arguments, batch atomicity, deferred-catalog wake ordering, sibling stop and active input/policy/route refusal pass; no Chat. Real connector and complete source gates remain separate |
| provider-mcp-qualified-full.log / provider-mcp-complete-native.log / provider-mcp-final-lint.log |83 regular Rust/103 Bun/28 Python, fmt/clippy/release and13 actual Android executor/reviewer/MCP/context fixtures pass. One synthetic app submission returns three actual native effects on original MCP HTTP calls. Stdio multiplex/cancel/fragment preservation, service shutdown, pending owner-generation refusal and settled cold re-entry pass; no Chat |
| provider-panel-target-full.log / provider-mcp-multiple-index-preflight.log |84 regular Rust/103 Bun/30 Python, fmt/clippy/release pass. Real reauthenticated app with auxiliary index targets passes zero-inference MCP preparation, hidden automatic environment/user-literal preservation and hook cleanup. Connector inference ACK remains separate |
| provider-mcp-host-wait-full.log / provider-mcp-preflight-review.log |88 regular Rust/105 Bun/30 Python, fmt/clippy/release pass. Immutable versus replaced process owner, observed error-path counts, retained confirmation resolve/cancel and reply correlation regressions pass. Corrected actual-app request preparation passes with zero posts; real host approval/return remains separate |
| provider-sse-idle-regression-before-corrected.log / provider-sse-idle-android-fixed.log / provider-sse-idle-full.log |Actual native 2.5s SSE-idle regression fails with comment-only keepalive; data pings complete four six-second silent fixture waits/three native effects, zero Chat. Wire ping/cancel proof plus89 regular Rust/105 Bun/30 Python, fmt/clippy/release pass. Source-only fix, not an activated public runtime |
| provider-sse-android-composed.log |14 actual Android native regressions pass against the current installed0.163 runtime: retained MCP three-tool return, six executor/search/correction/reviewer/silent-wait fixtures and seven context/fork/question/automatic-interrupted compaction fixtures. All app replies synthetic; zero Chat |
| desktop-panel-device.json / desktop-panel-icon-taps.json / provider-panel-official-icons-python.log | Official installed ChatGPT/Chromium icons, no copied/redrawn artwork. Cold browser launch3.425s; existing-window helper0.139–0.241s. Six actual X pointer taps minimize/restore both exact windows without duplicates (observed0.142–0.424s); physical Android touch not separately observed. Final icon-source revision passes30 Python tests; only tint2 config reloaded, app/profile/inference preserved |
| provider-mcp-app-auth-admission.log | Retained earlier failed diagnosis, superseded by the authenticated preflight above. An old settled fixture was first preserved, then released after same-account/no-live-stream/no-tool-call checks without deleting its chat. Relay ownership/pre-send guard placement were corrected; missing auth refused before submission. No Chat or permanent Provider activation; this failure is not connector ACK |
| provider-mcp-tunnel-admission.json | Installed Termux Codex connector/tunnel/tool schemas match; real Rust MCP stdio tools/list and local health pass. External poll fails401 tunnel_use_forbidden even with recorded owner scope after process-only DNS relay. Zero Chat starts. Trial Provider/tunnel alias/proxy stopped and generated trial root removed; original GUI/profile/remote tunnel and unrelated services preserved. Historical failed attempt; superseded by the recovered runtime-key reference and separate installed Provider connector below |
| provider-tui-no-manager-current-core.json / core-current-composition-build.log | Current Core rebuilt from source (e83083f6…), Manager absent, same-thread /model native↔Chat round trip and permissions preserved with four synthetic Responses/no Chat. Earlier debug Core was stale and fell back to ordinary TUI; driver now offers --build-core and checks the actual frontend process |
| provider-native-failure-reason-final.log | Actual Android Codex reports the exact injected Provider failure, not generic EOF; no Chat |
| provider-auth-bootstrap-android.log | Two concurrent auth calls share one Rust service, endpoint retained/credential rotated on restart, readonly status; empty app profile, no Chat |
| provider-project-discovery-android.log / provider-project-create-minimal-body.log | Actual authenticated project discovery/create/reuse/readback; no Chat |
| provider-live-project-native.log | Actual GPT-6 in configured project, three exact native effects, four app dispatches/two user objects/three tool returns, saved membership readback |
| ai-development-proof.json | Direct real AI repository development, same native thread across recovery and independent validation; details below |
| provider-owned-auth-native.log / owned-auth-transition.json | Actual authenticated Rust-owned app/private pipe/native tools, four dispatches and three effects; original profile inode restored, no credential copy |
| provider-restored-auth-readonly.log | Existing controller/login restored to the same account; no hooks/inference |

Hosted exact source5e0d396 passed source39117103778, Provider39117103775 and Core/
runtime39117140293. TUI728cf26 built but39122849434 failed a test-only AbsolutePathBuf assignment;
ff2682d fixed compilation;39126312868 passed50 focused +94 retained upstream
checks but failed a test-only format lint.15da596 fixes that lint;39129327503
passed and its exact artifact passed the no-Manager Android TUI trial above.
Neither failed prior candidate is admitted by that later success.
TUI310f0f7 passed39107948384 with48 focused and94 retained
upstream checks/lint. Later source edits require matching gates; these are not
proof of a new signed public composition. Ignored Android/live tests count only
when separately armed and actually run. Failed/zero-test required gates remain open.

### Direct AI development acceptance

Baseline humtr/ai main1f958e5 was clean; its full verifier passed48 checks before
model edits. GPT-6 High independently found that session filters following a
reference were ignored, wrote a failing regression and observed its failure. The
first attempt stopped on the Provider's partial-final phase defect before its
implementation patch reached native execution. That failure and state were kept.

After the Provider repair, the same native thread resumed, read current Git/test
state, patched lib/ai_resource.py, passed the new regression/full verifier and
reviewed its diff. The supervisor did not implement the AI patch. Independent
review reproduced failure on baseline code in memory and success on the model's
patch, then independently ran all49 checks/166 TUI cases, zero warnings/failures.
AI commit43abdc8 is pushed to verified owned humtr/ai; installed AI runtime unchanged.

There were26 app dispatches across two explicit user tasks,24 tool returns, no Pro
or automatic replay. First attempt had2 user objects (one visible). Cold resume
imported history and reported29 user objects, not29 new visible user requests.
Warm returns and cold history import differ; neither count establishes allowance
deductions. Initial/intermediate/final reports and actual commands/patches/tests
were observed. This is one bounded maintenance proof, not hours-long/reboot/all-tool
qualification. Unsupported box components remained in its final transcript.

### Remaining acceptance and delivery

1. App-wire concurrent lanes/stop/sibling results now pass; read-only thread
   status reports last journal phase/counters even without the local endpoint,
   without claiming fresh app progress. Finish actual startup/identity loading,
   host-loss/cancellation/concurrency/reboot qualification. Authenticated Rust boot/inference now works; its reversible
   device test restored the old controller and original home directory inode.
   Explicit PID-bound Provider service stop now passes without altering native
   history; permanent operating migration and fresh login/provisioning remain. Optional
   Rust viewer now reuses the service/private RFB with configured noVNC assets;
   actual fresh-profile RFB and restart/revocation pass, physical UI remains.
2. Automatic/interrupted native compaction now passes with synthetic responses;
   qualify model-downshift compaction and actual TUI
   native↔Chat transfer with pending results/cold resume. An actual Android PTY
   /model round trip now passes with real checkpoint encoding and synthetic
   Responses against the final admitted artifact; real-Chat TUI proof remains. Bidirectional
   API re-entry and actual Chat summary also pass within their narrower scope.
3. Native tool search/MCP discovery now passes against an owned local MCP fixture;
   its tool executes once and appears with its result in the native transcript.
   This is native Codex MCP, not the separate Chat-side MCP return fallback.
   Actual native auto_review now invokes independent Provider reviewer lanes with
   schema-valid decisions: allow executes the owned commands, deny prevents both;
   the separately permitted patch remains. Replies are synthetic, not Chat judgment
   qualification. Native blocking question→actual answer→original host-call return now passes
   with one native user turn and no resubmission; question TUI remains. Finish host
   search, broader JSON/freeform/batch errors, real reviewer judgment and broader project instruction priority. Native
   guardian_review metadata now isolates reviewer lanes outside the configured
   project; actual Android allow/deny preserves primary membership and confirms
   both reviewer opens have no project. Sibling stop/result isolation passes.
   Native prefix-rule matching currently misses double-quoted executable paths:
   upstream shell-command/src/bash.rs accepts command_name only as a word. The
   prompt-rule fixture uses a plain owned path; do not generalize that proof to
   quoted paths or rewrite model commands in Provider to hide this upstream issue.
4. Rust MCP returns actual native command/patch/test results on retained HTTP
   fixture calls, sharing the common lane/executor/transcript. Local functions
   remain the default. The independent `Termux Codex Provider` ChatGPT development
   app is installed against a dedicated Rust trial tunnel; its IDs are in
   `chatgpt-connector-registration.json`. Tdev's runtime-key file reference is
   reused, while tunnel IDs, targets and clients remain separate. Tdev's production
   services and the legacy `Termux Codex` connector were not changed. Metadata,
   control-plane polling and ChatGPT discovery pass. A trial-only CONNECT relay
   handles container DNS through Android without resolver changes or TLS decryption.

   The MCP-onward audit found the trial was running from a mutable Cargo target:
   rebuilding unlinked its live image, causing `stale_process_owner`; lazy auth
   then refused the held writer as `provider_already_running`. This was not a
   missing global config: the service already used its own XDG MCP configuration.
   Zero active lanes were verified, the exact old process was pinned/signalled
   gracefully, and the same endpoint recovered from an immutable candidate. The
   active connector name was corrected to `Termux Codex Provider`. Process identity
   checks remain strict; never normalize away a deleted image to bypass them.
   `token` may bootstrap and is not a read-only diagnostic; use `status` for that.

   The previous environment fix's ignored live test bypassed the Rust projector
   and omitted one returned assertion field. Corrected actual-app pre-send proof
   now runs the real projector, observes one user object, preserves literal text,
   and keeps environment metadata in hidden system context with zero posts.
   This is request-construction evidence; physical app transcript acceptance is
   separate. The exact API24 artifact/source `1cb0a4d` from CI `39140919784` also
   passed hash-checked Android `models`/`catalog` and isolated configuration smoke;
   its receipt is under the acceptance cache's `provider-android-smoke-UqQrXy/`.
   These are executable smoke results, not stable installation admission.

   The first native MCP request failed before app submission because a private
   owned project used `project_v2` memory. Read-only sidebar/exact-resource proof
   confirmed matching owner, private sharing and no sharing subjects. Rejecting
   that memory scope was an extra unsupported restriction; admission now preserves
   the project's existing mode and retains ownership/privacy checks. A subsequent
   native attempt submitted one app request/one user object and reached the
   host's explicit allow/deny confirmation for this connector, with no native
   effects. The old error path lost these counts; its zero counters are not proof
   of zero Chat submissions or allowance use. Chat allowance debits remain unknown.

   Host-confirmation waiting now keeps the same request attached, emits one public
   waiting item and records a content-free block plus counters before failures.
   Correlate host responses by target message ID; no private user reply or action
   arguments are exported. Resolved prompts do not block forever; cancellation
   remains acknowledged/unconfirmed as appropriate. Native tools and the host's
   approval are never synthesized. Actual host resolution and retained native MCP
   tool return still require live proof before admission. The refreshed immutable
   candidate retained one live trial at host confirmation (one app dispatch/one
   user object, no native effect). Opening the viewer initially showed the app's
   home screen; the active project conversation had to be selected explicitly.
   By the time its actual Allow once dialog was displayed, native Codex had hit
   its SSE idle deadline and the Provider acknowledged its stop. No native effect
   occurred. Comment-only heartbeats did not reset upstream's parsed-event timer.
   Content-free data pings now preserve that transport deadline without claiming
   model progress or adding Chat/transcript items. Actual Android regression first
   reproduces the timeout, then survives four six-second silent fixture waits with
   a 2.5-second native idle limit, executes three native effects and completes with
   zero Chat submissions. Wire proof verifies retained wait/cancellation too.
   The original app/relay had exited between sessions; restart reused the same
   persistent profile without copying credentials or changing Tdev. Explicit
   viewer-to-native-lane selection remains an integration task; opening the app
   alone does not guarantee its pending consent is visible. Preserve stopped or
   unknown attempts instead of replaying them. Fixes `ea954ae` and `7f02db9` are
   pushed. The latter's immutable device candidate is running in the isolated MCP
   trial; one new app dispatch/one user object is at host confirmation with zero
   native effects. The actual latest project chat is selected and a fresh viewer
   notification requests the user's host consent. No automatic approval/retry is
   used. Full source validation passes; hosted qualification `39146010553` has
   passed its source gate and is still building the independent Android artifact.
5. TUI also starts an auxiliary automatic-title model thread for an unnamed
   conversation; the terminal fixture names its owned thread to isolate the
   transfer proof. Qualify/count that cost and avoid unnecessary Chat starts.
   Reduce setup/context/argument overhead and adapt terminal markup without losing
   useful text, exposing hidden reasoning or increasing Chat requests. Count user
   objects/tool returns/model starts/setup/summary/retries separately; Chat debits
   remain unknown. No automated Pro or GPT-5.6 trials.
6. Provider CI now defines an independent Android/API24 NDK candidate after the
   offline gate, with exact source/lock/toolchain/digest inventory, no Core/Manager
   linkage or bundled app/auth. It is explicitly unsigned and not device-admitted.
   TUI/Provider without Manager now passes an actual Android same-thread round trip
   against current Core; final signed composition remains. Finish optional signed
   package/provisioning inventory, Base alone, Manager without TUI, direct --profile and Full opt-in
   /model; missing/incompatible artifacts and install/reinstall/update/rollback.
7. Preserve unique experiment probes, licenses and uncommitted1m edits before
   removing superseded runtime/closing its branch. Retain admitted signed rollback
   targets, not a silently selected legacy runtime.
8. Build/smoke/sign/read back immutable public assets; disposable update proof and
   non-forced stable CAS promotion, then separately activate the admitted device
   generation. Ordinary Base/Full never downloads/logs into the app implicitly.

Base/Full automation retains independent signed heads, immutable GitHub Releases,
exact Base references and latest-successful-Base repair target. A successful new
Base remains independently usable when Full fails; repairing the add-on targets
that latest successful Base, not the previous Full's older Base. Keep current/
previous and referenced bases. Core owns signed edition/date/base display without
Manager. Source integration, public publication and device activation differ.

Published Manager window/notification/title/tmux and Full60 quota-switch proofs
retain their original scopes: one-tap notifications never launch/resume Codex;
stock APK uses one registered dedicated window; live bare↔tmux migration/nesting
is not supported. Rotation defaults off and waits for settled tools/writer state;
Chat limits are not a Codex-account rotation target. The new composition must
preserve these features. Existing physical observations are not new device proof.
