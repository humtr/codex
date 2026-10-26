# Operating state and product completion

Reviewed 2026-10-26 UTC. `rewrite/rust-core` owns implementation; `main` pins
accepted exact sources for signed publication. SPEC owns product contracts,
README user instructions, and this file the current operating state. Superseded
incident/release narratives remain in Git; original private evidence remains in
`~/.cache/codex/acceptance`.

## Ordered implementation priorities

The user amended this order on2026-10-25: implement the separate TZ source change
first, then combine its native build/qualification with the entrance-continuity
repair. Older deferred setup, packaging and architecture work does not precede them.

| Order | Required outcome | Status |
| --- | --- | --- |
| 1 — source first | Core supplies Android TZif to the runtime and TUI; absolute session-picker dates use that same local timezone | Source, focused checks and combined current-Core/native-TUI execution passed, including shared Seoul TZif and KST completion display; final acceptance and delivery remain |
| 2 — next runtime repair and combined qualification | Continue the same bound work from Codex or its opened ChatGPT conversation, return both entrances' public turns to the same native history/TUI, and make subsequent Codex requests succeed. Include duplicate progress/final presentation | Native continuity, restart, retained-chain eviction/Stop, all63 affected TUI tests and disposable signed install/TLS/update/rollback passed. Actual Android→same native TUI/history→successful native follow-up passed, with no reinference. The subsequent single-read source settlement correction passed full Provider/JS gates, fault checks and actual-server SDK→same native TUI→native follow-up after Provider replacement. All5 original finals match; real web native-tool routing, uncontrolled concurrent entrances and exact-source delivery remain |
| 3 — next after2 | Retain common ChatGPT connection/work logic and branch by local/MCP transport, separating transport initialization, checks, configuration and lifecycle | Agreed implementation target; defer the refactor until1 and2 are complete |

The user also authorized a later Provider budget-extension change, to join the
combined implementation/build batch. `max_native_calls` currently accumulates per
assigned work across turns;75 was this work's supplied budget, not a hardcoded
default. Existing claim supports time extension but rejects a changed native-call
cap. Add explicit owner-authorized extension while preserving consumed calls and
parent/ownership boundaries; do not reset usage or edit live journals. For the
current Chat-first source assignment, the user chose a fresh work with1000 native
calls and requested actual consumption afterward. Preserve the exhausted75-call
work and report admitted calls separately from successful executions. Its source implementation is now under Codex qualification below; it is not
an installed runtime change.

Budget policy clarification:1000 was the user's deliberately generous temporary
allowance, not a chosen product default. Neither75 nor1000 is a global hardcoded
limit. Selected assignments currently require all finite budget fields; ordinary
unassigned native work does not acquire that assignment cap. Increasing the cap
does not settle whether that requirement is appropriate. Usage recording must
remain independent of policy; platform limits, approval, Stop and logical-call
deduplication remain separate.
Do not describe the1000-call extension proof as acceptance of mandatory fixed
budgets or silently turn1000 into a default.
The user's subsequent proposal is to let the initially delegated model estimate
its allowance, then reassess progress/remaining work at exhaustion and either
grant a justified increment within any owner-set total limit or summarize the
remaining work. Treat this as a review checkpoint in the same conversation;
exhaustion must return a structured unexecuted-call result for reassessment,
not cancel the model stream. No separate reviewer model or fixed extension
multiplier was requested. This policy discussion is not yet a runtime change.

The exhausted source-editor work `431ed8daca18eed9d52ee241a758f1e217172b53af2afdca`
was natively stopped with no remaining jobs; its75 admitted calls remain recorded.
The same saved Chat now owns implementation work
`9c0c3147f9d38168b72dfc19b2418f03c563f3882a95c591`, native thread
`01a17171-124d-72f0-91da-8ae97307ec75`, in
`$TMPDIR/codex-originality-chat-nzbslj9a/source`. The actual assignment journal
confirms1000 native calls and a4-hour elapsed cap. The source-edit batch completed
with91 admitted calls and0 pending calls; its16-file candidate includes the
inherited9-file diff. Preserve that workspace for Codex review. Initial static
review found contradictory checkpoint/visibility tests and a formerly successful
model-downshift route changed to rejection; the candidate is not admitted.
`chat-source-initial-review.json` and the complete original candidate diff are
retained in the evidence directory below. Compilation, tests, signing, deployment
and activation were not authorized for that Chat batch. Original instructions, saved-message readback and
the initial usage observation are retained in
`~/.cache/codex/acceptance/independence-review-20261025/`.

### Chat-first source audit and Codex supplementation — in progress

Repeated shutdown investigation: mobile ADB now supplies a system exit record
for `com.termux` PID31170 at2026-10-26 11:21:06.316 KST, reason3
`LOW_MEMORY`. Android's configured timezone is `Asia/Seoul`. The resumed build's
last observation was11:21:04.880589 KST, about1.435 seconds earlier; its supervisor
and compiler subsequently disappeared without an exit receipt. This links that
interruption to an Android-recorded low-memory Termux termination. The original
user-supplied record and correlation remain in `user-adb-termux-exit-info.txt`
and `source-admission-adb-exit-correlation.json`. It does not establish the cause
of all earlier exits, a TUI leak, duplicate compilers, a fixed3GiB limit or the
exact pressure/threshold that selected Termux. The reported77MB RSS is the last
sample of the Termux app process, not the aggregate child/compiler usage or an
exact death-time sample; see the [Android ApplicationExitInfo contract](https://developer.android.com/reference/android/app/ApplicationExitInfo).
The detached status still said `running` despite both recorded PIDs being absent;
`source-admission-native-tests-resumed-disappearance.json` preserves that fact.
Termux itself could not read system exit-info; ADB supplied the missing boundary.
The user requested one operation at a time. Provider regression/static/release
checks and native restart qualification completed first. Then a single directly
attached TUI test build completed normally (exit0,10m31s), with one compiler job,
debug information disabled and incremental output disabled. Musl-loader-aware
argument inspection found one TUI compiler and no overlapping Core/Provider/TUI
compiler. Observed compiler VmHWM reached3219392KiB; this is compiler memory,
not the running product TUI. This successful run mitigates build-time pressure
but is not a guarantee against future system low-memory kills.
The new executable lists5881 tests. All63 selected regressions passed, none
failed/ignored (`source-admission-native-tests-current-focused.log`), including
bound-input ordering/cancel/failure with retained attachments, disclosure, complete
history, image preparation and TZif/local-time formatting. Exact new executable
and inventory are retained outside the mutable compiler target.
The initial Provider check with mismatched default debug settings was explicitly
interrupted (exit130), not an unexplained exit; its log is retained separately
and is not accepted validation.
The2.7GiB observation belongs to Rust's TUI test compiler, not the running TUI.
Read-only ownership inspection found one matching Core daemon with one loaded
thread, three distinct tunnel destinations with separate child servers, and the
ChatGPT application's own server. Stale PID files included reused PIDs; compare
the actual executable before counting them as live owners. A62-second observation
without another workload showed Provider/tunnel memory flat, Core PSS+swap about
1.1MiB higher and the app-owned Codex server about0.6MiB higher. This does not
establish or exclude a longer-running/request-driven leak. Original per-process
samples and ownership evidence remain in `shutdown-*.json`; no live process was
terminated during this inspection. The earlier local TUI compilation and Provider
qualification overlapped, increasing build-time resource load independently of
whether the product has a leak.

The resumed native compiler completed code generation and returned exit101 with
missing Zig UBSan runtime symbols at link time; that run was not an unexplained
process disappearance. The isolated linker now includes the unmodified UBSan
runtime built from pinned Zig0.14.0, retaining its checks. This is a development
host-tool correction, not a product workaround or native TUI acceptance. Original
failure and monitored process status are `progress-tui-monitored-build.*`.
The native TUI test executable subsequently linked successfully with the pinned
Zig UBSan and compiler runtimes. Actual renderer disclosure/mouse expansion and
TZif child tests passed; the long full-history test passed with the existing CI
`CODEX_PROFILE_CORE=''` environment and an8MiB test-thread stack. Its earlier
2MiB test-thread stack overflow remains recorded; no product guard or assertion
was removed. Evidence is `progress-native-render-disclosure.log`,
`progress-native-timezone.log`, `progress-native-complete-history-ci-env.log` and
`progress-tui-complete-runtime-link.log`. These focused tests do not establish
signed release acceptance. The complete native CLI subsequently built successfully
under a detached supervisor: `progress-native-cli-resumed-status.json` records
exit0. The earlier CLI build lost its tool session and process
without a recorded exit status; do not relabel it as OOM or a compiler error.
The complete Provider gate passed195 Rust tests (30 explicitly armed device/live
tests remain ignored by that source gate), clippy with warnings denied, release
build, Bun and Python checks. Actual Android operator qualification passed both
entrance continuity, original tool-result retry, Stop/restart and failure paths.
Its selected-control batch also proved1000-call budget extension preserves the
original start and1 consumed call, with no Chat inference. Evidence is
`bound-chat-resumed-provider-gate.log`, `resumed-native-operator.json` and
`resumed-native-budget-controls.json`. Original Chat source-editor usage remains
91/1000; the budget proof's1 call is a separate disposable assignment.

The original completed16-file Chat candidate and root's preceding diff are
preserved in the private independence-review evidence directory. Codex imported
the candidate for supplementation, not release admission. Its public JSON
checkpoint had no format identity, rejected its own valid encoding in a test,
and coupled ACK with full reimport while dropping continuation policy recovery.
The environment projection and test disagreed; a constant-false visibility helper
also changed account/new-input identity. The model-downshift test replaced a
supported outcome with rejection, and unrelated-thread handoff checks were removed.

Supplemented source uses a namespaced, versioned public JSON continuation record:
`format=codex-chatgpt-public-continuation`, version1 and nonempty summary. Unknown,
duplicate, foreign and unsupported fields/versions are refused, while whitespace
and key ordering are ordinary JSON. The official Codex Compaction string field is
preserved; this is neither OpenAI-encrypted context nor issuer authentication.
The private offered-checkpoint journal owns ACK admission. ACK commits the native
cursor past the resident summary once, keeps the Chat and reasserts ordinary
policy/tools. Source metadata controls environment/handoff presentation; user
text never acquires developer authority. Public user/assistant/tool prefix
integrity remains checked while native developer/environment updates are admitted
and forwarded separately. Ordinary user wrappers cannot select models. Real native injection/re-entry
exposed a further blocker in the Chat candidate: Codex removes internal source
metadata before sending to a third-party Provider. Requiring the handoff kind
therefore fails with opaque_context_unavailable despite passing pure tests.
The wire evidence and failed probe remain under pc-c9006df2 in TMPDIR.

Issuance now authenticates the complete public handoff and its native message
ID using domain-separated HMAC with the existing private journal owner. This
survives the actual metadata-stripping boundary. A public self-hash is removed;
only an issued exact message can supply foreign-history projection or the
necessary old-model compactor route. Copied text/new message ID, changed route,
other owner and unissued wrappers fail verification without role elevation.
CLI initialization may create only the existing owner marker, with no host/login
or second history. Issuance reads that owner while a live service holds its writer
lease. TUI uses the auth definition's exact state; production Service admission
uses that journal's owner, and Request carries only verified public data, never
the secret. Actual Android native handoff, native-model↔Chat-provider round-trip,
fork, warm/cold child resume and nested fork passed using synthetic Chat replies
and the installed runtime. They start no real Chat inference. Production CLI
checkpoint→HTTP Service admission also passed with internal metadata absent;
changing the issued native ID fails admission. Removing MAC verification or the
production runtime's issuer wiring makes the targeted regressions fail. Original
native records and fault results remain in the evidence directory.

A further caller defect was found while auditing completeness: upstream
`thread_read(includeTurns=true)` hydrates only bounded display history for
paginated threads. The TUI handoff now has a separate complete-public-history
operation using ascending Full-detail turn pages, with cursor/duplicate-turn
failure detection. Both initial and pre-injection confirmation reads use it.
Provider issuance rejects explicit Summary/NotLoaded views, preserving the
legacy Full default. Removing that admission check fails its regression. The
actual native long-paginated-history test now checks all256 ordered fixture
items through the new operation. Final source assembly against pristine pinned
upstream succeeds. The current native producer/test passed all256 ordered items
in the combined TZ/reverse-entry batch, with the exact harness environment and
stack settings recorded below; the earlier small handoff probes have narrower scope.

Caller review also rejected an engine patch in optional TUI, which does not
produce the actual daemon request, and a current-thread selection lookup, whose
fields are explicitly not per-request telemetry. Their plumbing is removed;
their discarded draft is retained privately for review evidence.


Package verification now derives the exact allowed inventory from the parsed
signed manifest: new six-file packages omit the retired adapter and upstream
summary-template notices. Immutable eight-file rollback packages still require
all eight assets; the installed generations inspected all use that layout.
The obsolete bridge-patch whitespace attribute was removed with its retired
consumer. No historical release is edited.

A further originality audit found that the copied upstream summary preamble was
used at runtime for text-compaction acknowledgment, despite the earlier notice
calling it a fixture. That runtime/template dependency is now removed. Text
compaction produces the complete unchanged model summary with a Provider-owned
versioned round marker; native Core owns any additional preamble. The context
codec creates the same typed checkpoint response for AppHost and operator entry,
checks the exact issued body at a native message boundary and rejects an old
round, modified summary or unchanged history. Only compaction control output is
buffered; ordinary answer/tool streaming remains unchanged. The source template
and its Provider-only notice/license are removed from new packages; installed
and historical signed eight-file packages remain intact. Library checks pass122 cases with2 armed ignores; the full Provider Rust suite
passes200 reported cases (including its self-spawned child), with30 explicitly
armed ignores. Actual native manual text compaction and remote V2 checkpoint
both passed on the installed manifest-matched runtime: each uses three synthetic
model requests, commits once, preserves the same thread and continues without
reimporting the resident summary (`owned-text-native-compaction-results.json`).
The text trace contains one complete summary with its issued marker; the V2
trace preserves the exact public JSON item. The first attempt used an unadapted
TUI binary and failed configuration before any model request; its log and fixture
are retained separately, not counted as product proof. Python publication and
candidate gates passed7+6 cases. The Core workspace tests and warnings-denied
Clippy gate now pass. This also corrected a stale literal seven-file count by
deriving accepted counts from the authoritative inventories. The current Core
passed374 workspace tests plus its separately armed real-Termux read-only launch
smoke. Actual native automatic and interrupted compaction also passed: partial
interrupted summaries never commit, and the same thread recovers. Removing the
issued marker in disposable producer source made actual native continuation fail
with `native_context_prefix_changed`; that source was restored
(`owned-text-native-producer-mutation.json`).
The actual MCP and local operator paths passed with the current Core/Provider and
the real CLI synchronization consumer (`owned-text-native-operator.json` and
`owned-text-native-local-operator.json`). The latest combined continuity proof
(`owned-text-native-continuity.json`, fixture `$TMPDIR/ct-cg50xfpo`) passed5 external
inputs and7 native turns, one actual native effect with retained retry, source
admission before a new native draft, Provider restart on the same saved Chat,
rotated local controller, Stop and live/resumed mouse disclosure. It made2 primary
submissions and reinferred no saved answer. Saved Chat and host Stop are synthetic;
ordinary Android/browser input and actual ChatGPT server Stop remain unqualified.
Disposable signed package acceptance passed Base/Full installation, local and
owned-TLS remote delivery, authenticated stable-index update and rollback
(`owned-text-provider-package.json`). The fixture now copies the admitted
generation's launcher mode, and download verification derives the complete exact
asset set from its signed manifest instead of assuming the old package size.
Earlier fixture failures are retained separately. No live install has changed.
The final optimized Provider build and all-targets warnings-denied Clippy passed
(`resumed-final-provider-static-release.log`). The current JavaScript gate passed89
cases and Python gates passed224 cases across18/113/11/6/76 suites, including
publication, frontend source assembly and qualification inputs
(`resumed-final-js-python.log`). The embedded-asset inventory regression now uses
the active native model JSON rather than the removed summary template; it retains
the same generated-target exclusion and complete-asset assertions.
Current source/diff and the optimized executable are preserved outside mutable
targets. Manifest-matched inactive fixtures reclaimed7.268GiB with verified
compressed restore objects and `resumed-final-cleanup-*.jsonl`; original native
histories, source, screen frames and results remain. Reusable Core/Provider/TUI
compiler caches total about13.3GiB, below the15GiB development budget. The unknown
pre-existing `providers/chatgpt/target` was preserved.

The existing settled current-reasoning claim can now increase total native-call
and elapsed budgets while retaining concurrency/app caps, original start and
consumed calls. Child extension also checks the parent's native-call allowance.
Its Android operator qualification now passed the explicit1000-call extension
with the actual native consumer: the same thread, original start and1 previously
admitted call were retained. This disposable assignment is separate from the
source-editor work's91/1000 usage. Managed host-ledger extension
remains rejected. No live budget journal was edited.

The connector refresh gate previously checked names only. It now compares actual
live MCP `inputSchema` against registered action `params`, including queue enums
and context cursors, and refuses readiness on disagreement. Historical registered
metadata already includes context_cursor in some records, while the web exposed
schema omitted it: do not collapse registered metadata, live tools/list and the
model's cached schema into one observation. Actual refreshed web use remains to
be qualified after combined delivery; no live connector has been refreshed here.
Read-only resumption checks find the existing default Provider and app host live
and ready, with no saved-read failure. Its actual local MCP catalog has17 tools
and includes `queue`, but not the new `synchronize` action: this is the existing
installed service, not activation of the qualified candidate. The pending Android
trial's separate Provider now reports `stale_process_owner`; no model request or
old prompt was replayed. The user confirmed only the Codex login changed; the
Android ChatGPT account remains unchanged. A new isolated current-source trial
in `$TMPDIR/ca-e9ma06cq` verified that same app identity and completed one actual
native→ChatGPT→native answer. The user's actual Android input then entered that
same native history and live TUI, and its native follow-up completed in the same
saved Chat/thread. All3 complete original answers match saved-source readback;
the2 candidate primary submissions plus1 Android submission generated exactly3
turns, with no reinference or native tool execution. Per-turn records retain
GPT-6/none throughout. Evidence is `resumed-actual-android-continuity.json` and the
retained fixture. The original driver's final read hit429 after all3 turns had
completed; a later read reconciled that assertion without resending any input.
Its original failed aggregate result remains separate from the verified result.
The new host reuses access/ID credentials only,
with an empty required refresh-token field and no copied browser profile.
Initial trial setup omitted that required empty field and used a Provider
installation root different from the TUI's root. Those fixture errors were
corrected; the latter failed with `native_thread_owner_unavailable` before any
app dispatch, and its original native record remains. No history was rewritten
or failed source prompt retransmitted. One trial-service Stop exceeded its15s
confirmation bound; later pidfd/syscall observation confirmed that owner's exit.
Termux Bridge tools are not exposed in this Codex turn;
local catalog success does not establish external connector/schema readiness.

Actual Android import latency remains material: its saved assistant message's
creation timestamp precedes native completion by450.3 seconds. This is not an
exact server-completion timestamp. Repeated HTTP429 saved-read cooldowns were
observed; source inspection found a second GET during `follow_settle` after the
same answer had already been read successfully. A rate-limiting fixture makes
that second GET block completion despite the complete source answer. Source now
retains one transient raw snapshot per observing app lane, consumes it only when
the current account/scope, lane/round, final ID and entire final body match, and
releases it on settlement, Stop or a new round. Missing/mismatched snapshots use
the existing paced read and reject changed final text. The Rust consumer supplies
the complete final body already retained in its public projection. This adds no
model request, source text change or persistent secondary history.
The boundary tests also exposed stale renderer-scope admission across an account
switch; settlement now validates the currently selected scope before hydration.
The complete JavaScript suite passes92 cases; the Provider suite passed200 reported
cases with30 explicitly armed ignores, all-targets Clippy with warnings denied
and the optimized build passed (`source-settlement-provider-gate.log`). Disposable
faults restoring the duplicate GET, accepting changed text, reusing an earlier
round or ignoring the current app account each failed the source/SDK boundary
tests. Omitting the final body from the Rust producer also failed the real
AppHost/McpBridge exchange; all isolated sources were restored, with no live
source mutation (`source-settlement-*-mutation*.json` and retained fault logs).
The new Provider then replaced only the completed owned trial service, reusing
its state/endpoint and all3 existing native turns. A separate actual app SDK input
and native follow-up passed in the same Chat/thread, producing turns4/5 with
exactly2 additional model requests and no source-answer reinference. All5 complete
saved-source finals match canonical native history, and per-turn GPT-6/none is
unchanged (`source-settlement-real-continuity.json`). The original actual Android
trial is retained; this supplementary stage used an SDK caller, not a second
physical Android submission. The previous immutable auth CLI remains the actual
consumer of the same rotated listener credential; its protocol is unchanged.
These
findings do not establish that every429 is caused by this duplicate GET or reveal
the upstream's account-wide rate threshold. No operating installation changed.
The supplementary SDK input's original result was also confirmed complete
without retransmission. Both owned trial services were stopped only after their
work completed; the final teardown observed no loaded native threads and then
confirmed both isolated runtime exits through pinned kernel handles. It did not
query background-job pages for an unloaded thread; the qualified histories record
zero native tool effects. `post-proof-teardown.json` preserves that distinction.
The trial's manifest-matched payloads were archived with verified restore objects
through `scripts/clean_development.py`; original Android/SDK/native histories,
source and failure records remain (`source-settlement-fixture-cleanup.jsonl`).
Compiler caches remain about13.3GiB; the unowned pre-existing Provider target
remains untouched. Current source/diff and optimized executables are retained
outside mutable compiler outputs. The trial TUI and its polling host are now
closed; this result does not activate the user's operating installation.

Automatic bound-Chat reverse propagation and same-source duplicate display are
implemented and qualified on actual native Core/TUI/Provider with synthetic saved
Chat. Each external input receives its own native turn and fresh tool authority;
the source fixture includes active native execution, not just completed text
import. Actual ordinary Android/browser input after the429 correction, current
connector tool routing and public exact-source delivery remain open. The combined
native runtime has been built and exercised in disposable roots, with no live
installation, activation or publication.

Current audit evidence includes the green workspace full gate, ordinary Provider
Rust suite and clippy, Bun bridge/schema tests and affected Python checks. The
latest focused handoff7 tests, real CLI/Service issuer admission and frontend11
Python tests pass; the current ordinary Provider suite has200 reported passes
(including its self-spawned child) and30 explicit
device/live ignores. The final current-source Android native-model/Chat-provider
round-trip also passed with synthetic replies. Targeted faults are exercised in
disposable source only. `audit-batch-result.json` and the reviewed full source
diff are retained; owned partial assembly and fault sources were reclaimed.
Do not present synthetic Chat replies, source assembly, schema readiness checks
or issued-handoff model round-trips as actual external-web reverse propagation,
refreshed web queue use or signed delivery.

### Bound ChatGPT follow-ups and Codex re-entry — next runtime repair

The completed source batch reported91/1000 native calls; the canonical work
journal confirms91 calls, completed phase and0 pending calls. Completion had not
been configured to notify the original Codex receiver through its queue.
The user's actual web-Chat queue test now proves a separate capability exposure
defect: its `codex_sessions` action enum rejects `queue` before server dispatch,
and its advertised claim arguments omit the server-required `context_cursor`.
Both failures happened before enqueue. Fix the server/registered web schema
agreement at its producer/registration boundary; do not weaken the server's
ownership or cursor validation. Include this in the entrance-continuity repair.

The same exact notification and stable call ID were then enqueued through the
installed local CLI to original TUI thread `01a16d35-ed95-78a0-b268-344263df33e5`
in the explicit szob execution home. Receipt
`01a1718e-0e9a-7793-a5b7-e2fde3e9e18a` confirms queued delivery. The notification was subsequently received as a new
user turn and independently found as the same exact text in the target native
canonical transcript at2026-10-25T05:37:50.639Z, turn
`01a17190-b1f1-7070-994b-bca208937d5f`. Local CLI→queue→Codex consumption is
qualified for this receiver; physical screen display was not observed. The
web-schema failure and automatic bound-Chat reverse entry remain distinct. Do not
resend the consumed notification. `queue-test-summary.json` and the original web
schema error, local receipt and candidate review remain in the evidence directory
above. No source repair, build, installation or service restart was performed by
this queue experiment.

Reported native thread `01a170ac-abaa-7e42-8346-742c32ebfaea` is linked to saved
Chat `6add5bed-5598-83e8-8e39-a6dd4123e9a1`. Four Codex-origin GPT-6/xhigh
requests completed. Four subsequent user inputs made directly in that Chat and
their answers are present in its294-message saved branch but absent from the
native public conversation. Reading the full saved branch is not proof of native
delivery. The Chat's attempts to resume/create/claim work did not establish the
requested automatic return; invalid arguments/writer guards must not be bypassed.

The last Codex turn failed at2026-10-25 11:23:37 KST with
`native_context_prefix_changed`, not a demonstrated network disconnect. AppHost
rejects this at ordered native-prefix admission before a new app submission;
the retained lane has4 app dispatches and that exact last_error. Settings events
show GPT-6/xhigh→GPT-6-Pro/none at11:11:25 KST, after the four successful turns.
The failed turn contains new model-switch/collaboration developer messages.
The full failing outgoing Responses input was not captured, so the first changed
prefix item and causal contribution of model/owner change are not established.
Test direct-Chat input and model change independently; do not attribute the guard
failure to the user or infer it was caused solely by entering a Chat prompt.

Code boundary: app_bridge.js deliberately passes unowned, settled ordinary Chat
streams through. AppHost exchanges observe their own native requests, not an idle
subscription to all later inputs in the bound Chat. Existing saved-chat input,
queue and takeover controls are distinct explicit operations. Add continuation
for the already-bound conversation while preserving unrelated ordinary Chats.
The user confirmed the direct entrance is an ordinary web browser or ChatGPT
Android app, not the wrapper's hosted renderer. A renderer-only send hook cannot
repair that entrance. Installed SDK inspection confirms its prepare operation
also runs for unsent drafts, and existing-stream resume is separate from a new
completion (`/f/conversation/resume`, conversation_id and offset). Saved-source
observation and correlation to the same native owner must cover the external
client; SDK method existence alone does not qualify the continuation or tool lease.

Implementation direction: keep one verified native thread/work/conversation and
controller binding across entrances. Observe actual bound-Chat input/public output,
refresh per-turn native tool authority, and reconcile it through the native owner
without another primary model call, new work or replay of completed tools. Use
Core-owned history APIs rather than editing session DB/rollout files. The current
source uses native queue admission and verifies the consumed user item's
clientId/turn before observing the existing saved answer; raw-item injection
alone cannot provide live TUI turn projection. Qualify native persistence, live
item/turn projection and subsequent request context together.
Use stable source-message identity and acknowledged cursors for deduplication and
lost-receipt reconciliation. Update app/native context cursors only for verified
commits so imported source turns are not echoed back to Chat. Preserve prefix,
writer, controller and uncertain-delivery guards; legitimate model/context changes
need explicit admission, not blind prefix resets. Serialize concurrent inputs
through the existing native ownership/queue boundary. Bound-entry transitions
should perform required protocol admission internally rather than making the user
manually create/resume/claim work for each follow-up.

Duplicate presentation is confirmed in the same record. The first native final's
3799 characters are all already present in ordered preceding public commentary;
the last final repeats3995 of3999 characters. Its native final ID matches the
McpBridge full-final projection of that same saved physical app message. Four
successful native turns correspond to4 app dispatches: the observed duplication
is not evidence of another model submission. Provider13 intentionally preserves
the entire final after prior MCP fragments; keep that correction and all original
history. Prefer grouping/updating/collapsing the progress of the same source
message when its complete final arrives. Distinct useful reports remain distinct;
text similarity alone must not delete messages. Validate source identity across
virtual rounds, reconnect, replay and reload before choosing the exact display
mechanism. Do not fix repetition by dropping the final body, showing only its
suffix, or changing canonical history to a lossy summary.

The user selected option1: show progress and tools in source order while running;
after receiving the complete final, fold only progress from that exact source
message, with its full text available to expand. Keep the entire final visible
and independent reports/tool results in place. Preserve response fidelity and
completeness: change only
presentation, not prompts, model/effort, generation, model-visible context or
canonical records. Do not summarize, trim the final, request another answer or
merge messages by text similarity. Group only verified identical source-message
identity, keep distinct reports/tools available, and verify full final content
plus live/reloaded display. This selection does not suspend the separate
reverse-entry repair.

Native TUI qualification advanced: the actual musl test executable passed its
real renderer/mouse disclosure regression (same-source progress only, expand to
original code, complete final/independent report/tool preservation), the actual
timestamp formatter with KST TZif and New York DST in fresh children, and complete
public-history pagination with256 ordered entries. The latter first exposed the
unoptimized test harness's2MiB stack limit and an inherited live-profile Core
entry flag; it passed with an8MiB test stack and the workflow's existing empty
`CODEX_PROFILE_CORE`, without altering product permission checks or assertions.
Logs are
`progress-native-render-disclosure.log`, `progress-native-timezone.log` and
`progress-native-complete-history-ci-env.log` in the existing evidence directory.
Removing physical-source separation in disposable Provider source made the
regression select an independent report for folding and fail. The original
source was restored; fault diff/output remain in
`progress-false-source-identity-fault.*`. No model text or prompts were altered.

The complete native CLI and actual Provider journal CLI now passed live and
resumed owned-screen disclosure, including real PTY mouse input. Folded progress
occupies its original position; expanding reveals its original text, with the
complete final, independent report and native command/result still visible.
Native canonical history is unchanged before/after TUI resume. The source
fixture now prepares its profile in the same canonical registry as public
`prepare`; the former arbitrary registry was rejected by the TUI's existing
symlink check. The driver waits for a loaded native thread and rendered draft
before Enter, avoiding startup/paste coalescing rather than retrying input.

`progress-live-disclosure-ready.json` used an older retained Core executable:
its UTC timestamps exposed that input mismatch. Preserve that result's narrower
scope. `next-proof-core-current` was then rebuilt from the current source;
`progress-combined-current-core.json` passed the full source fixture with it:
5 external inputs,7 native turns, one actual tool effect, retained receipt retry,
native reentry, Stop and live/resumed mouse disclosure, with2 primary submissions
and zero reinference of saved answers. Actual runtime and TUI child environments
both referenced the same valid Seoul TZif; the screen displayed23:36 KST.
Child observation is `progress-combined-tzif-child-observation.json`; original
native records and folded/expanded frames remain in its named disposable root.
These are native product execution with synthetic saved Chat input, not actual
browser/Android Chat input or physical touchscreen observation. Inline physical
terminal scrollback is retained; the folding proof covers the owned viewport.

Actual-server qualification then used a disposable authenticated host, current
Core and the new native CLI in `$TMPDIR/cw-srka4brz`. Original user state was not
activated or replaced. The original host refused Stop because3 native works
require reconciliation; its empty renderer lane list is not sufficient to stop
that owner. Preserve it. The disposable host reused valid access/ID tokens only,
without copying the refresh token; the original host/account remained available.
Native → a separate app SDK input → native passed in one saved Chat/thread:
3 saved user/assistant pairs,2 candidate primary dispatches, no saved-answer
reinference and no native tool execution. Its native history/source comparisons
and `real-continuity-result.json` remain in that root. A CLI spelling mistake
was rejected locally before the SDK input; no model request was resent.

The user then entered the supplied fourth prompt through the Android app. Its
original successful answer exists in saved Chat and its user input was consumed
by native turn4, but native output failed with `app_round_identity_unconfirmed`.
Saved-read429 occurred684ms after native turn admission. McpBridge read the saved
source before recording its new round; a failed read left the previous round
authoritative, and common recovery relabeled that failure as identity loss.
Injecting one429 reproduced that exact error. This is a confirmed cause for the
bounded Android failure, not a diagnosis of the older unexplained process exits.
No fifth/native-followup model request was sent. Preserve the4-request source,
failed native turn and `android-failure-correlation.json`; no source message was
resent, native history edited or original work stopped to conceal the failure.

The source fix prepares/persists the source round before retrieval and keeps
the previous discovery capability private until source revalidation succeeds.
Cooldown therefore retains the same round; Stop fences both pending discovery
and execution. The existing complete-answer/reentry regression now injects429,
waits through backoff and proves discovery is unavailable until source validation.
A separate active-source Stop regression proves pending discovery wakes fenced
without another primary request. Both pass. Common observer exhaustion now
retains its originating static transport code instead of a generic wrapper.
Logs are `actual-android-429-regression-before.log`,
`actual-android-429-regression-source-lease.log` and
`actual-android-429-stop-regression-fixed.log` in the existing evidence directory.
Affected Rust tests passed196 with30 explicitly armed tests ignored by that source
gate; clippy with warnings denied and release build passed. Disposable mutations
reintroducing the early read and omitting the pending-discovery Stop fence each
failed their targeted regression (`source429-mutation-results.json`). Actual
current-Core/new-TUI/Provider execution passed5 external inputs/7 turns, one actual
native effect, receipt retry, Stop and live/resumed disclosure with synthetic source
(`source429-native-continuity.json`). Archived `next-proof-provider-source429` and
its service executable own that exact source scope. Later synchronization work
below requires its own qualification.
A fresh actual-server test in `$TMPDIR/cf-_zgv56er` completed its first native
request once, using Chat6ade2a38-cb14-83ee-bb06-13740001855c. The requested second
Android marker has not been confirmed; retain that same Chat/thread rather than
repeating the first prompt. Its pending observation is not successful app proof.
Two disposable service
Stops exceeded the15s confirmation bound and exited later; this is retained as
an observation/teardown boundary, not forced termination or an OOM diagnosis.

A separate native admission defect is confirmed in `source429-overlap-diagnostic.log`:
typing a new native question while a saved external reply is active admits the
new user before the original final, causing `native_context_prefix_changed`.
The stronger producer-to-consumer qualification rejects the previous client at
that earlier admission boundary (`source-admission-old-client-red.log`, fixture
`$TMPDIR/ct-5rhtptkv`), even with the corrected429 Provider. Do not weaken prefix
checks, reset history, truncate finals or reinfer them to conceal this defect.
Provider now owns the one-shot `codex_sessions synchronize` source observation
and queues only existing saved inputs. The saved-source lane owns external-turn origin before
native queue admission and retains it through actual native completion; Operator
reads that existing journal instead of duplicating origin in the strict work record. Ordinary
native steering remains native behavior. A capable TUI waits before recording its
new draft and reuses the existing image-input preparation lifecycle, now named
input preparation, retaining attachments, mentions and cancellation. Source
pending/read-backoff causes and retry timing come from Provider; hard failures
restore the draft. Canceling that unadmitted draft does not interrupt the already
admitted external turn. Local CLI discovery shares authenticated listener publication
between production and the native fixture. A verified older tools/list action enum
or exact older CLI rejection retains explicit legacy submission; other failures
must not become success. Current source tests passed before the final admission
rechecks; the final full checks passed. Actual native overlap execution passed in
`source-admission-native-continuity.json` (`$TMPDIR/ct-c0v0yu3n`):5 external inputs,
7 turns, one native effect and retained retry, exact original finals, delayed new
native draft, subsequent native question, Stop and live/resumed mouse disclosure.
It uses actual Core/TUI/Provider CLI but synthetic saved Chat input. The retired
consumer failed that same admission contract. Fresh TUI compilation and all63
affected unit tests now passed. The retained retired-consumer failure remains the
actual native admission regression evidence.
The detached native CLI build records process exit/resource samples in
`source-admission-native-cli-status.json`; no installation or activation occurred.
Latest Provider checks are `source-admission-tests-settled.log` (22 result groups,
196 reported passes including the self-spawned child;30 explicitly armed ignored),
`source-admission-clippy-settled.log` and `source-admission-release-settled.log`.
The native driver now reaches the authenticated synchronization CLI rather than
assuming a synthetic response. Final image preparation snapshots attachments
before the source check, avoids an empty blocking image worker, and then waits
before native admission. Its pristine assembly and compiler inputs match
(`source-admission-pristine-assembly-snapshot-first.*`).
The restart defect is confirmed: McpBridge's owner epoch changes at Provider
construction, but the previous external-input check called that an account change
and prevented the next native draft even on the same settled Chat. The original
failure remains in `source-restart-same-account-regression.log`, with disposable
source at `$TMPDIR/source-restart-review-8g_5607r`. Source now distinguishes owner
generation from account identity, verifies the completed native turn/writer/jobs,
and adopts the same saved Chat only after account/project/anchor validation.
Old execution capabilities stay retired; only the bound discovery lineage is
restored. Explicit Stop fences that lineage even before it enters the new live
map. Startup restores observation of completed bound MCP work without launching
Core or loading a thread; local CLI controller publication uses the same rotated
private record as the live operator. Pending/unknown work stays fenced.
Current Provider regression tests passed (197 reported passes, including the
self-spawned child;30 explicitly armed ignored cases,22 result groups), and Clippy
passed with warnings denied. Actual Android Core/TUI/Provider CLI restart
qualification passed in `source-reattach-native-continuity.json` at
`$TMPDIR/ct-0zdij3hi`: same saved Chat/native history, automatic external-source
observation after service replacement, rotated local controller,5 external inputs,
7 turns, one actual native effect with retained retry, delayed native draft,
subsequent native question, Stop and live/resumed source disclosure. No saved
answer was reinferred. The saved Chat and server Stop are synthetic; this does
not qualify an ordinary Android/browser client or actual ChatGPT server Stop.
The optimized Provider build passed (exit0). Final JavaScript tests passed89
cases, and affected Provider/Python qualification checks passed6+76 cases.
Disposable mutations that reactivated retired execution or bypassed the shared
read policy both failed their targeted contracts; both isolated sources were
restored (`source-reattach-mutation-results.json`). Actual native executable/input
copies are retained before cache cleanup. The inactive accepted and missing-dependency fixtures reclaimed about2.65GiB
of manifest-matched payloads through `scripts/clean_development.py`, with
compressed restore objects and `source-reattach-fixture-cleanup.jsonl` plus
`source-reattach-import-fixture-cleanup.jsonl`; source/native records remain.
After TUI qualification, reusable compiler caches total about13.0GiB.
This source is not installed or released. Ordinary Android/browser retake, actual
connector tool routing and exact-source signed frontend/public delivery remain.
The long-session audit confirmed that payload-cache eviction incorrectly removed
source discovery authority, including an evicted settled predecessor. The new
Broker registry separates current-owner discovery admission from bounded payload
retention, reads only retired source links from the existing private journal,
and validates a reattached chain without loading every payload into memory.
Fresh execution remains live-owner-only. Stop revokes cached/evicted discovery,
retains pending durable revocations across a failed journal access, and retries
that persistence without restoring authority. Individual stopped-token retirement
uses the same lane revocation implementation. Close now reports persistence
failures instead of swallowing them. A300-source-input/300-other-turn regression
and a repaired-journal Stop retry reproduce both failures before correction;
current library tests passed121 cases with2 explicitly armed ignores, and Clippy
passed (`source-retention-*.log`). Both faults were reintroduced into disposable
source and detected (`source-retention-mutation-results.json`). The subsequent
full Provider suite and current combined native continuity passed with this
registry and the compaction-origin correction (`owned-text-checkpoint-full-tests.log`
and `owned-text-native-continuity.json`).
Earlier incomplete fixture-asset copies that failed compilation are not product
reproductions.
The adoption GET now uses the existing shared account read policy instead of
bypassing cooldown/in-flight exclusion and leaking raw request failures. Ordinary
adoption and bound recovery share the same cause-specific401/403/404/429/5xx
mapping and Retry-After behavior. `source-reattach-read-policy-red.log` records
8 failures before correction; the corrected complete app-bridge file passes72
cases in `source-reattach-read-policy-green.log`. This is a local fixture proof,
not an actual ChatGPT request or Android retake.
The native workflow now includes the affected input/image preparation, disclosure,
timezone and complete-history regressions. The freshly built inventory lists5881 cases and confirms exactly63 selected
cases, all passed, matching the workflow and release qualification count. The existing accepted frontend pin
still names the older publication and must be replaced only through successful
exact-source frontend qualification/publication before combined release admission.
This pre-admission check does not claim an atomic transaction with an uncontrolled
browser input arriving after the check. Restart reattachment and real connector
tool routing also remain separate qualification boundaries.

Provider source tests now cover a completed saved web answer entering the same
lane without another primary request, exact-text reentry, distinct source IDs for
identical prompts, and new per-turn discovery while old execution tokens remain
retired. Existing source-read backoff is honored. These tests use synthetic saved
Chat replies. The initial `scripts/qualify_provider_continuity.py` also passed on actual Android
Core/runtime/TUI: two equal-text saved inputs with distinct source IDs, a third
active response with one real native command, retained MCP result retry, complete
source answers in canonical history/live TUI and a following native prompt all
stayed on one native thread (five turns, two primary submissions). Actual Operator
discovery initially bypassed the new common Broker readback; that caller is fixed
and the native proof passed afterward. The saved Chat source remains synthetic;
ordinary browser/Android Chat input and uncontrolled concurrent admission remain
unqualified; subsequent same-Chat restart proof is recorded above. Evidence for
this initial batch is `bound-chat-native-active-continuity.json` and its owned
fixture under the existing independence-review directory.
Do not describe reverse propagation as deployed or complete.

The display implementation correlates physical source IDs in Provider state and
reads them through a model-free CLI for TUI disclosure. The actual Android native
runtime preserved all correlated IDs in canonical response items; complete finals,
separate reports and tool execution also passed. Native renderer/disclosure tests
passed in the isolated official musl compiler workspace; Android host-tool
adaptation is not release acceptance. The subsequent input-preparation changes
passed compilation and all63 affected tests in that same disposable workspace.
Installed executables remain unchanged.

Acceptance must exercise the user's actual existing-TUI/opened-Chat path:
Codex→Chat→Codex, direct input in that bound Chat→same live Codex TUI/history→a
successful Codex follow-up, repeated alternation with/without tools, unchanged
model and separately GPT-6→Pro, restart/resume, concurrent input, Stop, duplicate
delivery and complete final text. Verify actual canonical records and TUI display,
source-message correspondence, one execution per logical tool, no additional
primary inference for importing an existing answer, and preserved model/effort,
project/account/controller identity. Earlier Codex-only follow-up or explicit
saved-chat-input proofs do not close this automatic reverse-entry boundary.

### Timezone source batch — native qualified, delivery pending

The same conversation's TZ proposal is implemented in source: Core supplies
Android timezone data to the musl runtime; existing Local-based TUI formatting
consumes it. Recorded actual TUI probes show `TZ=Asia/Seoul` retaining UTC in the musl runtime, while `KST-9` and extracted
Android TZif paths work. Seoul and New York TZif probes from the Android2026c
APEX database produced the expected distinct reset dates/times without a TUI
rewrite. Android-native chrono success alone does not qualify the musl runtime.

Respect explicit working POSIX/path TZ settings; resolve IANA names, or Android's
system timezone when unset, through verified Android tzdata and pass the selected
TZif to child runtime launches. Prefer the active Android timezone data over an
older system copy; validate available source discovery rather than hardcoding
this device's path. Cover the existing Core environment plan and relevant bare,
Manager/TUI and shared-server child paths. Validate names, offsets/bounds, TZif
format, private cache publication, data-update invalidation and unavailable or
invalid inputs. Diagnose failures without blocking ordinary Core launch. Do not
force KST or a fixed offset on all users, add +9 to server timestamps, rewrite
stored UTC, or require shell configuration/Manager for correct bare execution.
Explicit UTC user-facing displays such as resume_picker absolute dates need a
separate small upstream-TUI patch under native.patch; internal UTC records remain
unchanged. Required proof includes unset/IANA/POSIX/path TZ, Seoul, a DST region
across transitions and date boundaries, timezone-data updates, failure cases and
actual installed musl `/status`/resume display. Full acceptance/delivery are pending.

`crates/core/src/timezone.rs` now owns Android property/source discovery, IANA
selection, bounded index/TZif validation and private atomic cache publication.
The common `apply_child_env_plan_and_fence` consumer applies it for qualified
bare, optional-TUI and newly launched shared-server children. Explicit empty,
POSIX and path settings are preserved; failures report their cause and leave TZ
unchanged. The optional frontend's `native.patch` converts resume-picker absolute
timestamps with chrono Local, consuming the same TZif without another parser.
Stored UTC and relative-duration calculation are unchanged. No existing process
was restarted, no shell/profile changed, and no runtime was installed or released.

Focused Core checks executed5 tests successfully on Android, including actual
child environment delivery for Seoul/New York, POSIX/empty TZ, unset TZ using
the system property, and cause-specific nonblocking failure for an unknown zone.
A disposable-source mutation removing common-launch normalization fails that child-delivery regression; the live source is intact.
The original mutation output is in
`~/.cache/codex/acceptance/timezone-source-20261025/missing-launch-normalization-mutation.log`.
Core clippy passed with warnings denied, frontend Python checks passed11 tests,
and formatting/docs checks passed. Local Core checks used Rust1.99.1 with
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0`
and the reusable `~/.cache/codex/check-target` (about3GiB). Exact release-toolchain
and full acceptance remain deferred. Temporary source copies were removed after
preserving the mutation log. The pre-existing untracked Provider target is untouched.
The complete native patch assembled from pristine pinned inputs and the changed
resume-picker source was formatted. Its new real-formatter regression covers a
TZif date rollover and New York DST rollback in isolated processes, and is
required by the native feature gate. That formatter test now passed in actual
musl execution and in the fresh63-case focused gate; the Core launch mutation
above detects missing timezone delivery. The expanded-details test derives local
timestamps while retaining its metadata checks. The current full Core/Provider
gates, musl frontend build and disposable combined same-TZif KST display passed.
The formatter-specific mutation now also passed: its exact production formatter
and existing child-process test were extracted into disposable source and run
with the same musl compiler/dependencies. The unchanged producer passed; replacing
Local with UTC failed the Seoul rollover expectation (exit101), then the isolated
source was restored. This narrow proof does not rebuild the whole TUI; that
producer's full-TUI test passed above. Evidence is
`resumed-picker-timezone-mutation.json` and its baseline/fault logs. The initial
harness omitted an imported clock-test dependency; that compilation failure was
retained separately and corrected without modifying product or test assertions.
Installed `/status`/resume observation, exact-source release qualification,
signing and delivery remain separate work.

### Later: common Provider with transport-specific responsibility

Retain one common app connection, identity/project/conversation implementation
and native work lifecycle authority. Branch by local-function/MCP transport;
separate transport admission, initialization, result handling and shutdown so
optional use does not require the other external transport. Preserve ordinary
Chat passthrough and native execution/approval/history ownership. Keep tunnel
build/version/update/rollback independent and install it with selected MCP setup.
Module boundaries, process separation and package boundaries are separate design
decisions; do not duplicate the common engine merely to produce two executables.
The proposed Core-required/optional-layer/all-selected-Full composition remains
related future work. Do not make this refactor a prerequisite for the
entrance-continuity repair.

Private investigation evidence is in
`~/.cache/codex/acceptance/cross-entry-followup-20261025/`: original native snapshot,
fully paginated saved Chat, correlated journals/settings, incident-summary and
duplicate-presentation-analysis. The entrance-continuity investigation makes no
history, account, live hook or service changes and does not qualify that repair.
The TZ source batch above remains separate from installed runtime activation.

## Tool-call interoperability: Provider10 delivered

The research session01a16cf1-847a-7b30-9868-28c74e5cef1b selected
native gpt-6.1-sol/high at2026-10-24T09:15:34Z, but its09:15:53Z follow-up
failed with HTTP400: a retained68-character call_id exceeded64 characters.
Selection succeeded; history admission failed.

Responsibility/data flow: gateway_calls produces portable native Call IDs while
retaining the app host ID for ACK/results. Broker dispatch IDs also use64-hex
digests because Operator directly correlates those IDs with native results;
logical retry keys and the journal format are unchanged. Native and Chat continue
through the existing upstream transcript/executor, without an upstream patch,
transport proxy, migration flag or second history store. The user explicitly
excludes retroactive repair; old oversized historical IDs are left intact.

The first773c937 candidate missed the direct Operator consumer: one command
executed, but its result failed with mcp_result_invalid. It was rejected before
signing; its effect and failed native turn remain preserved without replay.
Source6e26cee includes both the Broker and gateway corrections. Local full gates
(369 Rust/141 Python), Provider gates (182 Rust plus Bun/Python), targeted faults
and direct diff/producer-consumer review pass. Mutations detect old Broker IDs,
unbounded app IDs and oversized batch IDs; disposable source is restored.
Independent-agent and GPT Pro reviews were not performed.

Signed Provider10 `chatgpt-6e26cee6d342-s10`, source6e26cee, is fully publicly
read back, Android-consumed, stable-promoted and activated for the ID repair.
Provider13 below is current, with Provider12 retained for rollback.
Binary SHA256 is `c742fdd1ba66ba4315d8bb3fe839601b70ad633223785eee6ab7bc2dc1d6a64a`.
Source40096385599, Provider/Android40096385723 and signing40097313205 pass.
The12 public assets, installed compositions, signed package lifecycle, exact
hosted native execution and public install/use/rollback pass. Non-forced stable
promotion changes only Provider index/signature/source; installed Full107 and
its frontend/runtime bytes remain unchanged.

After activation, three real primary turns pass on one new UUID: Chat reads one
owned file through one native command; gpt-6.1-sol/high answers from its result;
Chat then quotes that native answer. The tool ID is64 bytes, prior canonical
turns survive and the workspace is unchanged. Both Chat turns select none effort.
Own work Stop confirms native and host stop. This actual logged-in-app/native
inference proof is separate from synthetic Android admission tests.17 protected
config/auth/launcher inputs, eight pre-existing work identities/phases,18
registered native server process identities,16 MCP tools and fresh tunnel
polling are positively read back. Physical second-client display was not observed.

Host-only Stop initially refused an old reconciliation_required record. Its
thread was unloaded on all18 verified servers, its last native turn failed,
and it had no driver/pending calls or active app lane. Normal Provider maintenance
shutdown then released its package lease; that record was not repaired or marked
completed/stopped. A separate observer overcounted unregistered runtime PID4843,
which exited without captured command provenance. Its purpose/cause is unknown;
the incidental all-runtime-PID survival condition is rejected. Original failures
remain evidence. Read-only verification covers the registered native owners,
protected inputs, work inventory, signed selection, actual service/stdio images,
app readiness and tunnel polling; no native server was stopped by the update.

Evidence: `~/.cache/codex/acceptance/call-id-compat-kdt41vdr/`, especially
`delivery-final/{device-activation-proof,live-device-smoke,consumer-proof}.json`,
package/composition records, targeted-fault logs and the original failed observers.
Implementation, qualification, signed publication and device activation are complete.

## Project selection and app recovery: Provider12 delivered

Provider12 `chatgpt-5d2204a6c664-s12`, source5d2204a6, is signed, publicly
read back, Android-consumed, activated and stable-promoted. It is now Provider13's
rollback. At that delivery Core was Full107 with the qualified c55a5c9 frontend;
the current device is Full112. Source
40122894013, Provider/Android40122894169 and signing40123806220 pass. Public
promotion0eadaced changes only Provider index/signature/source, without forcing
history or modifying Base/Full heads.

The newly reported thread01a16ee2-04c0-7a91-afa5-9de91bde8189 executed12
native calls with64-byte IDs and all Broker results settled. Its final public
answer is retained locally and in the authenticated saved Chat. The app logged
404 `Resume stream unavailable` at17:11:00Z and successful polling recovery at
17:14:45Z (attempt40). Provider10 missed that recovered completion, timed out at
17:24:29Z and rejected the next input with `lane_reconciliation_required`.

Responsibility/data flow: the existing bridge now observes the app submission's
higher-level `onCompletion` as well as raw stream completion, using the same lane
epoch and cancellation fences. Failed/interrupted completion remains failure;
displayed text alone cannot settle a turn. The SDK single-tool-result helper
discards completion callbacks, so its separate submission branch/capability gate
is removed. User input and local tool results share the existing submission
implementation. First transport-error and resume metadata reaches content-free
Provider logs; no timeout extension, replacement inference or upstream patch
was added. Direct affected-flow review and four disposable faults detect missing
completion, the obsolete helper, retain-mode selection and late old completion.
Disposable source is restored; independent-agent/GPT Pro review was not performed.

A final same-epoch approval-stream audit reproduced a late original submission
callback settling the newer host reply. The additional source correction shares
one completion observer between ordinary input/results and delegated approval,
and fences it by both epoch and stream serial. The failing original audit and
its passing regression are retained. Provider12 includes this correction and
retains the verified project and original creation without another project.

The initiating Chat transport loss remains unestablished. The404 belongs to the
app's own resume endpoint, which the Provider new-dispatch hook does not intercept.
A separate local native app-server websocket1006/pong timeout does not prove why
Chat's first stream was lost. The missed completion is a verified repository
code defect; the original initiating fault is not attributed without evidence.

The project request previously used `history.mode=retain` and invoked no creation.
`projects list|use` CLI and the `codex_projects` MCP control now use the common
account-bound create/reuse/readback resolver, then atomically save project mode,
name and exact ID. Remote failure preserves local policy; concurrent config edits
preserve the remote binding for retry without duplicate creation. Creation locking
moves from individual AppHosts into that resolver. Existing conversations retain
their destination. Only the new project tool declares a new write effect; other
qualified tool annotations remain unchanged. Broad setup UX remains deferred.

The actual logged-in app created one owned private `Codex` project under
Provider11; Provider12 reuses that same verified destination. Readback and
a second use of its exact ID pass without inference or another creation, and
history policy is now project. Four real primary turns on one new native UUID
pass: Chat performs one native file read, Chat answers a follow-up, native
gpt-6.1-sol/high continues, then Chat returns. Both saved Chat conversations are
positively read back in that project. The one tool ID is64 bytes, previous turns
and workspace bytes survive, and own native/host Stop are confirmed. This was
normal live inference; missing-terminal recovery is independently exercised by
source faults with the pinned-app fixture, not claimed as an induced live outage.
Local184 Rust/77 Bun plus Python/lint/release and the armed actual Android native
command/patch/same-thread return fixture pass; its app/model replies are synthetic.
The exact hosted executable, compositions, signed package lifecycle with owned
TLS,12 public assets and disposable public install/use/rollback/stable update pass.
The first Provider12 live fixture submitted0 primary prompts: model re-entry
showed the native read-only/other-app ownership guard. Its terminal and native
records are preserved, with no inference replay or lock bypass. A new fixture
waited for the frontend's actual ready composer before model input and completed
the four turns. The original lock holder is unverified; no Core code or timeout
was changed, and the first failure is not accepted proof.

For activation, the reported failed work's native Stop and zero jobs are confirmed,
but its legacy app Stop remains unconfirmed. Its saved committed final is freshly
read back. Normal Provider maintenance shutdown fences MCP and closes its owned
app; the failed record remains reconciliation_required, never relabeled completed
or replayed.19 registered native server process identities survive. All17 protected
inputs are unchanged during the final activation; the earlier authorized Provider
history update is verified separately. The sole intended policy change is Provider
history selection. The same app identity,17-tool MCP inventory, shared generated
routing and final ready/idle app state are verified. Physical second-client UI
was not observed.

Evidence: `~/.cache/codex/acceptance/post-answer-recovery-20261024/`, especially
`delivery12/{final-device-proof,project-selection-proof,live-project-membership,
live-device-smoke,stable-consumer-proof}.json`, original project creation in
`delivery/project-selection-proof.json`, original failed observers and
mutation logs. Completed inactive fixture payloads are archived through the
existing cleaner with restore records; current/rollback artifacts and the unknown
repository target are preserved. Reusable compiler caches remain below15GiB.
KST is enabled in the user's ~/.bashrc and login ~/.bash_profile. Both fresh
interactive and login shells report TZ=Asia/Seoul and KST/+0900; originals or
absence records are retained privately. Running processes keep their environment;
canonical UTC records remain UTC. This proves the shell setting, not the musl
runtime's interpretation of that IANA name. The later reported TUI failure and
committed timezone implementation are covered by Priority2 above.

## Remote MCP polling: connection recovery correction activated

The reported saved Chat6add1a09-b21c-83ec-9c38-ed5ae0cad0ff called
`native_catalog` at2026-10-25 KST05:50:28 and05:52:32, after its last successful
poll at05:46:11 and repeated pre-header timeouts from05:46:46. The prompt was
submitted at05:50:18. No new command reached the local dispatcher during the
reported tool attempts. Local MCP still returned17 tools. The first physical
network interruption and the failed historical socket's ALPN/reuse were not
captured; temporal correlation alone does not prove their precise cause.

Responsibility/data flow: the external OpenAI tunnel-client owns control-plane
HTTP connections; Provider still owns MCP/native work. On the actual Android
runtime's HTTP/2 path, a silent loss of the established TCP connection left
later polls reusing that connection across request deadlines. Request cancellation
retired a stream without detecting the stranded physical connection. A real TLS/
HTTP2 fault test through NewTunnelServiceClient reproduces that recovery defect:
only the established connection discards traffic, while the same listener remains
reachable on a fresh connection. Original code fails two request deadlines; the
corrected transport recovers. This confirms the reachable recovery defect, not
an unobserved historical packet-loss mechanism or every possible server outage.

The maintained dependency source is `~/prj/tunnel-client`, branch
`fix/http2-poll-liveness`, commitbb78162ebfdf81e9984e0c469f58c8d552f35180,
based on the previously installed0.0.14 source0f870e5. Its control-plane HTTP/2
configuration probes an idle connection after half the configured poll wait and
closes an unanswered PING within the existing deadline guardrail. Defaults are
15s plus5s. Existing HTTP/2 settings are retained. There is no new retry route,
model replay, remote tunnel, timeout extension or Provider/Core source change.
The same regression fails when a disposable Go overlay removes the correction;
a healthy long-poll test also passes. All affected control-plane, MCP client,
transport and runtime-health package groups pass. Native Android `/tmp` fixture
failures remain in the original log; the unchanged Unix-socket tests pass with
only their test processes using a PRoot binding of TMPDIR to `/tmp`. The TLS/
HTTP2 regression itself runs directly on Android. Independent-agent/GPT Pro
review was not performed; the production diff and caller flow were reviewed locally.

The qualified Android image is selected only by this Bridge service's existing
TUNNEL_CLIENT_BIN configuration, under
`~/.local/lib/codex/transport/tunnel-client/generations/bb78162ebfdf81e9984e0c469f58c8d552f35180/`.
This is a local dependency correction, not a new signed Core/Provider/public
release. The original global executable and the other Tdev tunnel images remain
unchanged. Sole tunnel ownership, fresh advancing polls, unchanged Provider owner
and all25 pre-activation registered native owners are observed. The only service
configuration change is that executable reference. Old failed records are retained.

The exact original prompt, `Project: Chatgpt 작성하고 prj/codex 전수조사해.`,
was exercised through the original szob Codex TUI, Core Full112/source5d2204a6,
`prj/codex`, ChatGPT gpt-6/xhigh. One baseline request completed30 native commands,
created/selected one private Chatgpt project and returned its saved final. After
activation, one further original-prompt request completed21 native commands and
its final; the existing project was reused, its history policy retained and the
new saved conversation's exact project ID verified. Both own native/host Stops
confirm zero jobs. No poll error occurred during the accepted fixed-runtime proof.
The failed initial post-fix driver submitted0 prompts because model selection was
not confirmed; it is retained separately and is not accepted transport evidence.
Manual TUI selection then confirmed the requested model before the sole prompt.

The original tunnel health flag still means "a poll succeeded once", not current
freshness. Its documented limitation remains; acceptance checks advancing poll
metrics and real remote requests. A separate baseline finding is retained: its
saved final has4460 characters, its Provider public final has2584 raw characters,
and the native final item has2565 characters and starts at section5. The fixed-transport repeat also has4497 saved characters but only1413 in its
native final item. Full saved/native text equivalence is not claimed for either original run.
The Provider correction now gives each virtual round stable distinct public-item
IDs and publishes the complete committed final when its prefix was emitted in an
earlier round, including a completion with no added text. AppHost and McpBridge
share the committed-final predicate; source IDs still own prefix validation and
result routing. No extra model submission or tool replay is introduced. The new
retained-MCP regression covers growing and phase-only finals through real HTTP
tool returns; both the original ID collision and a disposable suffix-only
mutation fail its assertions. No synthetic model result replaces either original
record; historical finals remain unchanged.

Provider13 `chatgpt-bc355945a420-s13`, sourcebc355945, is now signed, publicly
read back, Android-consumed, activated and stable-promoted. Provider12 is rollback;
Core Full112 and its frontend remain unchanged. Source40145473855,
Provider/Android40145473830 and signing40146073560 pass. Local Provider acceptance
passes185 Rust/77 Bun plus Python checks. The explicit Android retained-MCP
fixture passes3 real native effects and same-thread native/Chat return. Exact
hosted execution, installed compositions, signed package lifecycle,12 public assets,
disposable public install/use/rollback and stable update pass. Non-forced main
288cd109 changes only Provider index/signature/source; Base/Full heads are intact.

The fresh original-prompt TUI trial uses the same szob home, workspace, Full112,
ChatGPT GPT-6/xhigh and one primary prompt. Thread01a1706e-fe2a-7821-ad4b-c900ddad7a0a
completes40 native commands, one app dispatch and one final. All237 saved messages
are paginated with the original cursor. The entire visible saved final4575 chars
matches the native4304 chars exactly after the existing production presentation
projection; this includes section1 through the conclusion.
Hidden/private content is not claimed readable. The existing Chatgpt project and
exact project ID are reused with history.mode=project. Own native/host Stop both
confirm zero jobs, and the own TUI's exit0 is observed. Actual Android/PTY execution
is proven; a physical second-client display was not observed.

Activation preserves17 protected inputs, the other2 registered native owners,
work identities, login and the scoped HTTP2 tunnel selection. A verified empty
Provider-owned catalog had inherited the previous Provider's package lease. Its
kernel identity,0 loaded threads and0 child jobs were rechecked before retiring
that one idle owner; no saved conversation was removed. Preventing that lease
inheritance is separate maintenance debt, not an established cause of this MCP
outage. Failed observer/setup records remain: a runtime was supplied instead of
the raw TUI fixture input (0 native steps), a consumer-package backup temporarily
moved its proof paths, and the first Chat observer assumed a single50-message page.
Corrected inputs/paths/pagination preserve the original results and acceptance.
Owned fixture payloads are archived with restore records; unknown repository
`providers/chatgpt/target/` is untouched and reusable compiler caches remain below15GiB.

The reproduced audit independently completed Core/workspace tests and Clippy.
Doctor also reported77 unindexed rollout files,73 stale DB paths and1 duplicate
thread ID, plus an unknown terminal-title item. These are preserved findings;
no data loss, outage causality or repair is inferred. Original history/DB/config
remain intact. The audit's PATH-level Bun absence does not invalidate the release
gate: its qualified native Bun executable ran all77 checks. Independent patch
review by another agent or GPT Pro was not performed.

Upstream dependency review on2026-10-25: latest0.0.18 was published
2026-10-24 05:25:58 KST; selected0.0.14 was released roughly7 weeks earlier,
4 releases/162 commits behind. Both implementations advertise wire2026-08-25;
legacy operation remains supported, and this actual MCP route passes. The exact
silent-HTTP2 fault also fails official0.0.18/Go1.27.3, while the same health-ping
patch makes both loss recovery and healthy waiting pass. This distinguishes the
confirmed recovery defect from an unproven protocol mismatch. Latest-source trial
changes are restored;0.0.18 has not been activated. Startup/readiness/routing
improvements still justify refreshing the maintained baseline after qualification.

Recommendation, not implemented installer behavior: track/build/version/rollback
tunnel-client as an independent dependency; install its tested Termux artifact
alongside optional MCP setup. Core and local_function must not require this tunnel.
Use tested pins rather than an unqualified latest image at launch. Current service
configuration remains the explicit opt-in described by SPEC. Upstream sources:
https://github.com/openai/tunnel-client/releases and tagged docs/protocol.md.

Evidence: `~/.cache/codex/acceptance/mcp-root-cause-20261025/`, including
`final-device-proof.json`, `halfopen-{original-test,fixed-test,mutation}.log`,
accepted package logs, exact baseline/manual TUI plans, canonical native records,
original saved finals and Stop results. `delivery13/` adds exact-text-equivalence,
public/stable consumer, activation and final-device proof, with original canonical
records. `tunnel-version-assessment.json` and `upstream18-{halfopen*,compatibility-proof.json}`
preserve the latest-source comparison and its red/green test. Earlier negative observers and the
original incident evidence remain in `mcp-session-incident-20261025/`. Reread
AGENTS.md before a resumed review/edit; preserve both repositories' current diffs,
unknown repository target, credentials, original conversations and running jobs.

## Reasoning-effort incident and redo boundary

The original thread's `thread_settings_applied` records establish the actual
selection change on2026-10-24: high→medium at02:30:32.045Z, then low at
02:30:32.055Z. The user restored high at02:41:02.946Z. These are KST11:30:32
and11:41:02. Current configuration and the earlier high `turn_context` did not
disprove that change; the earlier denial used the wrong evidence. The preserved
17 sampling-configuration log records in the low-selection interval still report
High for the already-running turn. They establish that recorded request scope,
not the absence of a TUI selection change or unrecorded requests' settings.

Upstream0.164.0 binds both Shift+Down and Alt+comma to immediate effort reduction,
accepts key repeats, and emits the user's exact lowest-level warning from that
shortcut handler. It updates TUI state, then sends `thread/settings/update`;
Core persists each resulting snapshot. Two such submissions precede the two
recorded reductions. This explains a reachable accidental navigation/selection
path without a model change, config rewrite or Provider round. Actual installed
Android TUI reproduction yields high→medium→low and the identical warning.
The precise original key is not recoverable: retained TUI logs start after that
event and there is no original input recording. Do not attribute that key to the
user, the assistant or a terminal defect as an established fact. Other producers
checked include accepted model migration, the usage model-switch prompt and
explicit model selection; none is established as this event's source.

The user identified accidental shortcut input as likely and explicitly cancelled
keymap prevention. The temporary opt-out in all6 existing execution paths
(4 distinct config files) was removed; each file again exactly matches its
pre-change bytes. README prevention guidance was also removed. No model/effort
was forced, no user login or credential was changed, and no running user job or
TUI was terminated. The shortcuts retain their original behavior.

Actual Android Core/installed Full106 TUI, isolated signed fixture, inert auth and
loopback Responses passed the following producer-to-consumer checks. Each case's
own canonical thread and emitted follow-up request were checked independently:

| Input/configuration | Canonical turn efforts | Result |
|---|---|---|
| Default Shift+Down, three presses | high, low | Two reductions and exact lowest-level warning reproduced |
| Disabled shortcuts, Shift+Down | high, high | Navigation does not reduce effort |
| Disabled shortcuts, Alt+comma | high, high | Alternate reduction binding disabled |
| Explicit F6 remap | high, low | User-configured shortcuts still work |
| Disabled shortcuts, explicit `/model --reasoning=medium` | high, medium | Manual model settings remain available |

The successful fixture made15 local synthetic Responses requests across5 primary
threads,0 real-model dispatches. Original failed fixtures remain preserved: one
exceeded the Unix-socket path budget; one omitted the config-level endpoint and
received401 from the official endpoint with an inert fixture key; two subsequent
observers wrongly assumed a single request/thread despite native auxiliary work.
The corrected observer uses each primary thread's canonical completion and
effort, alongside the emitted request. No user auth was copied. These failed
attempts are not accepted proof and are not hidden by a longer timeout or weaker
product assertion.

Retained change: an AGENTS rule distinguishing settings history from current
config and request evidence, addressing the incorrect earlier diagnosis.
There is no remaining keymap/configuration behavior change or temporary product
code. The isolated keymap controls above are reproducibility evidence, not an
active prevention policy. Local docs gate and final diff review passed; Iron
Laws, independent-agent and GPT Pro review were not performed.

Evidence: `~/.cache/codex/acceptance/bridge-incident-20261024/redo-high/` contains
`effort-settings-timeline.json`, `sampling-effort-summary.json`,
`effort-shortcut-android.json`, the diagnostic driver and original logs, and
`effort-keymap-{application,reversion}.json`. The earlier13-file unshipped continuity candidate
was preserved as `superseded-candidate.patch`/`superseded-source` and reverted to
HEAD for the requested redo; none of its checks admits it as the final repair.
The continuity repairs and their actual delivery boundaries are recorded below;
the shortcut investigation does not itself qualify those repairs.

## Bidirectional continuity: repair and qualification

Responsibility/data flow: Core owns canonical turns, settings and native execution;
the frontend restores saved model metadata before backend startup and preserves
saved settings across in-app resume. Provider owns the effective tool catalog,
work/controller reconciliation and its positively identified app stream. The
existing upstream native queue owns deferred user messages. There is no second
scheduler, transcript or execution route.

The requested redo replaced the superseded candidate. Current Provider source is
aeaefa7; frontend source is c55a5c9, whose recipe is unchanged by the later Provider
fixes. Behavior changes repair resume, declaration updates, completion observation,
hook ownership, transport recovery and queue admission; these are not described
as behavior-preserving refactoring.

| Reported boundary | Cause and resulting behavior |
| --- | --- |
| `native_context_prefix_changed` | Cold resume restored the Provider definition without its model catalog, changing the developer prefix. The existing startup path now restores saved Chat metadata before the models manager starts. Explicit catalogs/model overrides remain authoritative; prefix validation remains intact. |
| Warm `/resume` selected the wrong model | Model-picker re-entry's generated CLI flags became sticky overrides for another saved thread. The existing coordinator now distinguishes explicit selection from saved resume; saved settings are restored at Core-owned Chat/native boundaries. Unknown/custom-provider resume keeps upstream behavior. |
| `ambiguous_tool_declaration` | Historical revisions were treated as one conflicting declaration batch. Each batch is validated independently; chronological incremental updates replace older declarations and an explicit complete current catalog takes precedence. Same-batch conflicts remain errors. UUID/position heuristics were removed. |
| Completed work blocked the workspace | Completion depended on a later status poll; a late exchange finisher could restore running. One rule observes the actual completed native turn, idle app lane and no owned jobs, with matching owner/turn identity. Its terminal-event observer, status and admission share that rule. Replaced watchers are cancelled; no successful Stop is fabricated. |
| Ordinary messages in the same Chat were intercepted | Conversation ID alone retained hook ownership after settlement. Hooks now require positive stream/request/lane ownership. Ordinary user/tool streams pass through; owned permission replies, late requests and Stop fences remain protected. SDK pending submission is reserved before invocation, including synchronous continuation. |
| Managed-to-current follow-up retained old ownership | Settled claim preserves its transfer boundary; transfer detaches only passive owned hooks. Missing app ownership releases idempotently without launching an app. List supplies the existing work's home and source conversation so the caller can reuse it. Ordinary Chat text alone does not steal an active writer. |
| MCP/service replacement broke observation | One stdio framing process verifies the current authenticated service binding per RPC. A closed native websocket is discarded for the next new request. Uncertain mutations are never replayed. Device stdio invokes the admitted immutable Provider directly; the tunnel no longer restarts merely because Provider PID changes. |
| Queue messages were test-only | `codex_sessions action=queue` and `work queue` use the existing native queue. Explicit thread/text/stable call ID are required; lost receipts reconcile from upstream queue/canonical history without resend. Explicit execution home supports an unloaded saved thread. ACK proves enqueue, not consumption. |

Added: saved-catalog restoration at the existing startup boundary, the internal
selection/resume distinction, one native completion observer, current-binding
stdio recovery and user queue admission. Removed: duplicate declaration schema,
UUID/position guesses, old inline re-entry construction, startup-cached stdio
authentication wrapper, closed-client reuse, conversation-ID-only ownership and
the redundant immediate completion RPC. The transition proof now uses the actual
supplied Provider executable instead of a synthetic queue service; its warm-resume
observer checks the actual resumed UUID in the re-entered frontend process.
The obsolete synthetic fixture marker has no remaining consumer and is removed.
Retained adapters cross actual Core/app/stdio/native process boundaries. No
temporary product path or removal condition remains.

Actual Android proof uses the hosted frontend and exact hosted Provider binary:

- Same-UUID native→Chat→cold resume→native and in-app warm saved-Chat resume,
  preserving existing turns, saved effort and prefix metadata, without Manager.
- Two native execution accounts and a changed workspace; Provider/native owner
  replacement; Stop; first/existing-turn failure with readable history and native
  recovery; mode-only Plan retains selected high effort against a none default.
- Persisted completed before any status poll, then same-thread settled
  claim→transfer→accept→current native file write/final in the attached TUI, with
  no new managed submission for the current-Chat follow-up.
- Warm and unloaded queue consumption, lost committed/queued receipt recovery,
  owner-unavailable receipt readback and changed-input rejection, without resend.
- Exact hosted executable performs native command, custom patch and final through
  the existing MCP→Core path. The first observer incorrectly compared contextual
  envelopes; its failed turn is preserved. The original effect occurred once;
  distinct follow-up patch/final completed without replaying that command.

These native/managed fixtures use inert auth and local synthetic Responses/app
replies, not real Chat reasoning. The actual logged-in Linux app/SDK construction
probe additionally passes same-identity/profile checks with zero conversation
POSTs; injected SDK-start failure is not real Stop proof. Its owned app context
was closed and services restored. Physical simultaneous display in a second
ChatGPT client was not observed.

The original `Session terminated` source remains unverified. Tunnel logs do prove
an earlier stdio exit/restart and later polling disruption; they do not prove the
native task terminated or the tunnel stayed continuously connected. Original
native completion was22:41:30Z (next-day KST07:41:30); the separate Chat audit
finished around22:50Z. Saved creation times and later replay display are not new
native execution timestamps. Persisted Chat history and live second-client UI
delivery remain distinct observations.

Local Provider/Python/Rust gates and direct affected-flow review pass. Hosted
Source40069962639, Core40070000629 and Provider/Android40069962667 pass foraeaefa7.
Exact-bound Source40073657638 and Core40073658822 also pass. Source fault
mutations detect chronological first-wins declarations, missing
completion observation with zero status polls, stale startup stdio binding,
closed native cache, conversation-ID hook capture and app acquisition during an
absent-owner release. Disposable mutations were restored. Native40069297349
passes build,55 focused tests, saved-catalog mutation,94 retained upstream tests
and lint. `native-admission.json` verifies its source/recipe/artifact and retained
proof. Iron Laws, independent-agent and GPT Pro reviews
were not performed: none is installed as a gate, direct execution is requested
and no Pro facility is provisioned.

Failed observers and real guards remain evidence, not accepted successes: the
CI Unix-socket fixture needed explicit0600 under umask022; product socket checks
were retained. A mutation driver initially chose a stale Cargo artifact and was
corrected to the emitted executable. The queue proof no longer prepares over a
private fixture profile. No skip/xfail or weaker acceptance criteria were added.

Evidence: `~/.cache/codex/acceptance/bridge-incident-20261024/redo-high/`, especially
`warm-chat-final-android.json`, `exact-queue-cold-android.json`,
`account-bidirectional-android.json`, `exact-release-native.json`,
`actual-app-hook-probe3.log`, the `*-mutation.*` records, original failures and
the final gate logs. Read AGENTS.md again after recovery before editing/reviewing;
inspect this section, the actual diff and unresolved boundaries, not a summary
claim that newly added code is established architecture.

## Current status and next action

Current inventory selects signed Provider13 `chatgpt-bc355945a420-s13`, source
bc355945, with Provider12 for rollback. Its exact-user-path/public/stable delivery
proof is above. The new ordered priorities at the top own the next work; completed
Codex-origin trials do not close the reported direct-Chat continuation failure.
Current activation-state readback selects Core Full113/sourcebc355945, previous
Full112. This selection readback is separate from the earlier Full112 execution
proof; it does not retroactively qualify another generation or change a running
client's pinned runtime. This investigation performs no activation.

Provider12's earlier batch proved its service/stdio images, app identity/readiness,
17 MCP tools,19 registered native owners and account-local config/auth preservation.
Its routing/project/four-primary-turn proof is retained at that scope. It is not
proof that arbitrary direct Chat input automatically returns to a live Codex TUI.
Reported failed works remain recorded for reconciliation.

### Earlier activation boundaries

The first activation attempt correctly refused an inherited package lease and
left selection8 unchanged; services were restored. The exact native catalog
holder had zero loaded threads and zero child processes. Its metadata was saved
before PID-bound graceful shutdown and kernel exit confirmation, then a fresh
exclusive-lease check preceded activation. `host stop` separately refused
`host_active_work`; that guard was not bypassed. Inherited package leases remain
the documented maintenance boundary; no active native work or unverified owner
was stopped.

Signed Core Full107 `local-relay-hosted-0-164-0-deaf199f7667-s107`, sourcedeaf199,
upstream0.164, is published, fully publicly read back, disposable-update qualified,
stable-promoted and activated. Full106 is rollback. Publication40073946157 passes
the existing exact-source build/Android smoke/signing/readback/consumer/CAS path.
The separate actual-device public consumer also passes; its installed frontend
equals the admitted c55a5c9 bytes with only the existing54 Termux substitutions.
Core remains byte-identical to Full106; the frontend is replaced. Device activation
preserves15 protected files, all three Provider service PIDs and27 pre-existing
native processes, with zero observed exits. No user client/job was stopped.
Existing clients still run their pinned executables; a newly opened frontend
uses107. Installed selection is not a hot swap of an already-open TUI.

Account/workspace IDs, installation and tunnel choices belong to config/runtime
discovery. No production account/plugin-ID whitelist was introduced. App identity
comes from the authenticated app scope; native admission follows the uniquely
verified thread home/workspace. `history.account_change` remains configurable
(`new_chat` default, `refuse` optional). Settled new input can bind the selected
account; pending/unknown input/results cannot migrate between accounts. Fixtures
cover identity/workspace changes; no physical second-account login switch is
claimed. Another workspace's Secure MCP still requires its administrator
credential and matching Linux app login, as the user explicitly acknowledged.
No additional registration, tunnel or login change is part of this maintenance.

For upstream0.164, retain inline model/reasoning/speed/permission arguments,
saved-thread settings/compressed-history resume, repeated CLI overrides and
compaction corrections. Reviewer eligibility is unchanged; the discarded
Untrusted experiment stays removed. Original incident and earlier Full106/
Provider8 real-model evidence remain under
`~/.cache/codex/acceptance/account-independent-20261023T145152Z/`; they do not
establish a changed boundary's acceptance. Retained Provider9 additionally
completes one actual logged-in-app first prompt from an isolated execution home,
returning the exact marker through installed Full106. The empty workspace and
inert native auth remain unchanged; its own work Stop and backend kernel exit
are confirmed. Installed Full107 then passes ordinary cold resume without profile,
model or catalog flags and in-app warm resume from a different native thread,
with two distinct real Chat prompts. The same UUID retains all3 canonical turns,
each recorded Chat effort is none, native return preserves them and the workspace
stays empty. Managed completed is observed through list before any Provider status
poll. Own Stop and backend kernel exits are confirmed. Key records are
`redo-high/live-fresh-provider9/live-first-prompt-result.json`,
`live-full-resume/{result,saved-read-proof}.json`, `full-consumer-proof.json`
and `full-device-proof.json`.

Saved-source observation distinguishes explicit Stop from ordinary completion.
The first proof explicitly stopped its owned work before cold resume. The existing
`Phase::Stopped` contract starts a successor Chat for a new native input and keeps
the old source in `previous_chats`; both are positively read back. The two107
cold/warm finals are in the same current Chat; the first fresh final is in its
intact predecessor. The initial observer wrongly expected all3 finals in one Chat;
its mismatch and corrected boundary are retained. No prefix guard was bypassed
or native history replaced.

The additional actual native-only cold-loss proof passes without intervening work
Stop or Provider restart. It confirms completed work and an idle settled app lane,
closes only the owned TUI/idle backend, then plainly resumes the saved Chat and
submits one distinct next prompt. Work ID, native UUID and saved conversation ID
all remain identical. Both assistant finals are read from that same Chat; all5
canonical turns survive, native return preserves them and only the final cleanup
stops its owned work. All5 real Chat requests completed; no request was replayed.
`live-uninterrupted-cold-2/result.json` records this boundary. The first attempt
submitted0 model prompts: its read observer crossed the re-entry socket rename.
That original failure is retained; only read observation retries the demonstrated
socket replacement, without broader error swallowing or a longer timeout.

Remaining boundaries: physical live display in a simultaneous second ChatGPT
client and the original remote `Session terminated` cause are unverified. Local
stdio replacement, saved Chat persistence and native completion do not establish
those external client observations. No further source/runtime repair is pending
for the qualified boundaries above. The existing inherited-package-lease
maintenance requirement and workspace-scoped Secure MCP prerequisites remain.
Unknown `providers/chatgpt/target/` is preserved.
Reusable compiler caches remain below15GiB; manifest-matched inactive fixture
payloads are archived with restore records, retaining native records/source.
This completion batch reclaims2,722,209,792 allocated fixture bytes; it does not
prune user state, the unknown target or operating current/rollback generations.


## Operating baseline

| Plane | Verified state |
| --- | --- |
| Implementation | Provider12 verified project selection and stream-bound recovered submission completion, with portable IDs and retained continuity/queue repair; cold/warm saved-Chat frontendc55a5c9 remains qualified and installed |
| Operating Core | Current signed Full112/source5d2204a6, previous Full107, upstream0.164.0; original user TUI and fixed-transport proof use Full112 |
| Public/selected Provider | Sequence12/source5d2204a6, previous11, signed/read back/Android-consumed/promoted/activated; native defaults/auth retained |
| Host/profile | App26.1007.21434 under the Rust-owned private-pipe host; authenticated readiness and actual fresh inference accepted, original login/history retained |
| MCP | Local17-tool inventory and the original TUI/project/audit request pass; HTTP/2 stranded-connection recovery corrected in the scoped local tunnel-client buildbb78162. First physical interruption remains unestablished; separate native-final text boundary retained above |
| Retired source | Experiment refs/history, unique user changes, original snapshots, licenses and restore records remain preserved |
| Retained dependencies | Immutable c3642b19a87cdace158f JS/Bun/Python installation, rootfs, optional noVNC, current/rollback payloads and original proofs remain |

The following retained baseline records describe earlier qualified deliveries;
they do not supersede the current selections above. Read the original
`binding-release-20261023T030234Z` before maintaining its tunnel/transport scope.

Device evidence: `~/.cache/codex/acceptance/device-runtime-update-full93-provider-s3`.
`device-update-review.json` and `followup-full94/result.json` accept the actual
Core/Provider selections, kernel owner plus immutable Provider hash, same-account host readiness, authenticated
MCP initialize/list/read (16 tools), existing tunnel readiness and8 CLI/config
smokes. All ordinary account config/auth bytes remain unchanged. Only the known
generated shared ChatGPT profile was explicitly rebound; account-local links and
native defaults remain intact. The native socket budget was checked at84 bytes.

The idle legacy desktop was closed only after all5 retained works were stopped,
no active lane/control owner, and no UI draft/stream were verified. Its exact
source profile/history remain in the original location. The new guest preserves
1,865 app-profile entries and1,758 guest native login/history entries; IPC/tmp/
writer-lock directories were excluded from the copy. Guest `.codex/auth.json`
is required in addition to Chromium's `profile`; omitting it caused the preserved
first app_auth_context_unavailable observation. The corrected same-account read
passes without re-login or model work. The initial cold app_scope_unavailable,
relay-rejected Browser.close (no effect) and unsupported `--profile features`
observer command remain original failures; they were not retried as effects.
Supported `--profile chatgpt mcp list` loads the generated profile successfully.

H source admission subsequently promoted Full94 automatically. The final93→94
update uses that new immutable stable input, not a replay of the completed86→93
cycle. The same-account host stays ready and account config/auth hashes remain
unchanged. Previous93 and the original86/88 backup remain.

The permanent MCP wrapper is `~/.local/state/codex/providers/chatgpt/mcp-stdio`;
the existing codex-provider-rust-trial alias now invokes it. Old experiment
launchers, sources, rootfs/noVNC and original conversations remain preserved;
no live/unknown user state was pruned. Native model inference and physical viewer
usability were not retested. Do not rerun the completed update/install/migration
or old qualification roots. A future update must inspect current selection,
package leases, actual owners and pending work first.

### Model configuration and session authority

The current primary Codex thread is01a165ad-9447-7ea1-a85c-5a7592af7bf9.
Its canonical turn_context confirms gpt-6.1-sol/high; ordinary config is unpinned.
Quota-driven profile switches and supervisor resumption are excluded from product
failure/recovery costs. The user's latest decision removes forced ordinary model/
effort defaults. All6 default/account config paths (4 distinct files; api/awrk
already alias the default file) now omit top-level model/model_reasoning_effort
and the unsupported `[models.new_thread]` table. That table was an erroneous prior
edit and caused the reported ignored-setting warning. Actual installed Core
`features list` succeeds for all4 distinct configurations without unknown-setting
warnings;0 model requests. Other parsed settings, account boundaries and existing
symlinks are preserved. Evidence/backups:
`~/.cache/codex/acceptance/model-config-correction-20261022T220051Z`.
Only qualification supervisor launches explicitly select Sol6.1/high, through
`~/.local/state/codex/g-qualification-sol61.sh`. ChatGPT trial models are separate. Runtime
and resumed/explicit session choices may differ from ordinary defaults; inspect
canonical turn_context before claiming the executing model. Do not add a global
model pin or auto-upgrade/model-routing policy during handoff.

Core's `~/.local/share/codex/core/config/config.toml` is generated system-layer
output, reconstructed by render_core_config/system_config::prepare. It is not a
user-editable common-preferences file. Account `CODEX_HOME/config.toml` files are
user settings and are not overwritten by that generation. A future common user
preferences surface would need an explicit input/merge contract, not manual edits
to generated output or symlinking all account settings. No such feature is added.

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
| Provider | Rust config/catalog, model/tool adaptation and shared Chat/account/CDP/host/lane management; common interfaces implemented and qualified at the bounded F integration scope | Native execution/approval authority, canonical history or Core runtime selection |

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

| Stage | Disposition |
| --- | --- |
| A–D | Shared native correctness and bounded real development/comparison accepted at their exact inputs |
| E | Functional, ownership, context, project, host and idempotency decisions complete |
| F1–F9 | Common Rust bridge, delegation, reading, continuity and host integration boundedly accepted |
| G | Exact-source capability review, corrected native-final delivery, signing/readback, actual Android consumers and non-forced stable promotion accepted |
| H | Unique work preserved, duplicate source/worktrees retired, affected Provider/full gates independently reviewed, source committed/pushed e0f0927 |
| Device update | Initial G/H activation: Full94 / Provider sequence3. Binding correction: Full95 / Provider sequence4, same-account host, authenticated16-tool MCP and remote polling. Empty-session correction: Full96 / previous95, same Provider4/services/config/auth and exact qualified TUI hash;0 real model requests |

### G product qualification

G's corrected source skips only verified stale/nonruntime historical bindings;
selected-owner identity, private metadata and peer validation stay strict. The
original reviewer returned allow and its program executed exactly once, with3/3
original native returns, before the old source turn failed without a final. That
failure stays failed. The separate same-thread report-only turn used1 actual
GPT-6-thinking request,0 native tools,23.933s; CLI/native/saved final matched.
Both Stops, no jobs, candidate/native owner exit, bridge/input-observer cleanup
and original remote16-tool/read restoration were independently verified.

Useful exact-source proof (all roots below are private acceptance archives):

| Latest proof | Accepted scope and limits |
| --- | --- |
| `phase-g-source-installed-6aa2280` | Hosted Provider workflow39850738497 and source workflow39850738448 pass on exact6aa2280. Seven-file artifact/source readback and4 actual Android probes pass. Private installed composition84 steps plus signed-package113 steps pass; independent audit matches197 exits,185 noncredential stdout/113 stderr hashes and4 signatures. Twelve credential bodies are omitted.464.935s,0 Chat/native-account requests; same-primary canonical notification acknowledged. Archive manifest verifies513 original files |
| `phase-g-native-discovery-fix-5da0254d` | Correction's local Provider gate:167 Rust,162 JavaScript,111 Python, fmt/Clippy/release;28 opt-in Rust tests explicitly not run by that gate. Local Android reused-PID regression plus broader synthetic operator/cold-resume62.773s and managed lifecycle17.977s pass. The latter has5 synthetic inputs/78 observations/RSS growth3320KiB; its one cold native_socket_unavailable observation is retained.0 real Chat requests |
| `phase-g-native-discovery-fix-5da0254d/hosted-owner-reuse-6aa2280` | The exact hosted5ef3536c binary passes Android Status/Stop with an unrelated reused PID; unsafe metadata still rejects native_binding_invalid. PID bytes/mode restored, native owner/dispatch counts unchanged, stopped/no jobs, service exited and reaped.0 model/native inputs |
| `phase-g-native-discovery-fix-5da0254d/hosted-sessions-list-read-assertion-miss/reconciliation.json` | Original MCP list response succeeds and contains all3 works, including the retained stopped work54386f7c on thread01a16542. The harness incorrectly matched `id` instead of the documented `work_id`. Its failed assertion remains; offline audit accepts the original response without another call. The earlier claim that the work was omitted is withdrawn; there is no demonstrated list-discovery defect |
| `phase-g-source-installed-5da0254d` | Prior8d5d2056 hosted/private admission: source workflows,4 Android probes,84 composition/113 signed-package steps; no Chat. Exact earlier inputs and1387 archived hashes remain useful; they do not replace6aa2280 admission |
| `phase-g-tui-reentry-5da0254d` | Same original native thread retains prior turns and the explicit interruption, then completes one distinct follow-up on hosted8d5d2056.14 tests,5 new original calls/results; cohort10/10, no missing/replayed calls. Native final/original CLI/saved report match. One new GPT-6 request, cohort2, no automatic source summary.165.183s; supervisor-reconciled segments, not uninterrupted execution |
| `phase-g-review-timeout-handoff-5da0254d` and `phase-g-profile-switch-review-5da0254d` | grv.ns2g9obl completed its one development task and same-thread Chat handoff:11/11 original results,10 tests, unchanged protected files. Source generation1→successor2 and29 saved successor messages independently read; assistant finish matches native final. Original harness had a generation assumption and matched prompt text as completion; corrected audits preserve that distinction. Reviewer timed out before its program effect; no verdict is claimed for that run |
| `phase-g-review-policy-preparation-5da0254d` | gj.hnoqg7k6 completed its distinct task/effect with1 Chat request,3/3 native results and exact CLI/native/saved final. No reviewer ran: the new executable did not match the prompt rule. Keep this limitation and do not repeat the effect |
| `phase-g-native-review-owner-fault-5da0254d` | gk.hmaf439i used2 actual GPT-6 requests; unbound independent reviewer returned allow and the program executed once. All3 original calls/results returned; protected files unchanged. Old Provider5da0254d then hit the historical-PID discovery error and the source native turn ended systemError without a final. Reconciled native/host Stop, no jobs, candidate exit and original remote16-tool/read restoration pass. The original failed source result is still failed |
| `phase-g-actual-06062c04`, `phase-g-tui-first-and-failed-reentry-06062c04`, `phase-g-operator-cold-fix-local` | Earlier affected managed/direct development, first original-TUI task and real cold-owner failure/recovery evidence remain scoped to their inputs. Cold-owner/frontend fixes were admitted through5da0254d; accepted tasks are not repeated |


Corrected public delivery is archived under `phase-g-public-delivery-6aa2280`:
immutable release chatgpt-6aa228034346-s3; signing run39865112885;12 public assets
read back;10 immutable and4 stable Android consumer commands pass; non-forced
Provider-only promotion main6e89e76.0 model requests. The failed main-registration
workflow39864984102 remains preserved and is not a signing failure of the payload.

Final native proof is `phase-g-final-followup-6aa2280` (154 original files).
Cancelled preturn preparation and zero-request stale-observer cleanup remain
separate immutable archives. Never arm/replay gf continuation5 or an old run.py.
No hours-long reliability, actual allowance debit, hidden reasoning access,
unlimited Chat capacity or universal performance winner is inferred.

### H preservation and verification

`phase-h-retirement-6aa2280` preserves both source trees, unique user edits,
34 retired repository files, refs/licenses/provenance, tests and independent
review. Restore instructions identify exact worktree refs and patches. Active
Rust source, embedded app/viewer assets, explicit local-function fallback,
app-bridge/CDP tests and reusable qualification tooling remain.

Provider/full pass counts are in the baseline above. A quota-driven profile
switch interrupted the original foreground full check; its no-exit log remains
interrupted. Verified141 Python/fmt and170 Core passes were reused. The remaining
40 Core cases (plus1 ignored), other154 Rust cases and Clippy completed exit0.
No passed Core case was replayed. H's exact reviewed diff and retained evidence
hashes were checked again before committing, without rerunning accepted tests.

## Proposed ChatGPT control through MCP

The implemented and boundedly qualified control workflow lets ChatGPT discover
and manage local Codex work through MCP. Codex remains the execution, approval
and canonical recording system; Provider transports/control-adapts that path.
There is no second planning agent or transcript database. Termux Bridge names
the MCP connection, not another product component.

### Agreed bridge structure and E work

Entrance and reasoning context are independent: native CLI/TUI or ChatGPT can
control a work item, and inference can stay native or use a replaceable Chat.
One active controller owns each work binding; independent children/reviewers
retain independent contexts. General delegation and direct entry use the same
native execution/history/approval path. Defaults remain explicit in Provider
config; neither an entrance nor a tool annotation authorizes replay/fallback.

### Same-thread continuity and Chat context handoff

The primary native thread retains durable history while Chat conversations are
replaceable reasoning contexts. Summaries combine the accepted summary, confirmed
history since its boundary and current instructions/work state. Fence old control,
settle/reconcile calls and jobs, record the handoff, then require the successor's
exact current receipt. Failed/uncertain transitions preserve a recoverable boundary.
Origin/work/original CLI return binding survives. No summary covers later events
implicitly. Native compaction and Chat renewal remain separate operations.

Ordinary model selection reuses readable native history without a mandatory fresh
summary. An explicit source summary is a separate capability. Never decode/drop
foreign opaque checkpoints or replay tools to manufacture continuity.

### Idempotency preliminary research and phased work

E/F common operation, exact duplicate/result recovery and restart logic are
accepted at their bounded scopes. The Rust16-tool catalog applies12 true/4 false
idempotency hints; other annotation fields stay unchanged. The observed ChatGPT
backend omits idempotentHint; its internal consumption and causal performance
benefit are unknown. Logical call IDs, actual effects and original receipts—not
hints or JSON-RPC IDs—establish exactly-once/recovery proof.

### Linux app and host capability research

E research and F implementation cover authenticated app identity/project/read/
input, exact owner/CDP/private-pipe management, shared host controls and optional
viewer handling. Rust owns production host lifetime; retained CDP helpers are
qualification dependencies. App binaries/rootfs/login are separately provisioned,
not bundled into the Provider package or inferred from its model catalog.

## Context and easy configuration

Provider config: `$XDG_CONFIG_HOME/codex/providers/chatgpt.toml`, default
`~/.config/codex/providers/chatgpt.toml`. Rust owns configuration/catalog data;
Core/Manager/TUI do not maintain a second model-family list. GPT-5.6 is excluded;
Pro stays manual-only and excluded from automated trials; the Instant alias is
hidden. Listing a model establishes neither entitlement nor inference readiness.

`codex provider chatgpt config show/effective/validate/set/unset/edit` uses atomic
validated edits. `prepare --home PATH` explicitly publishes immutable native
catalog/profile routing for the next startup, preserving account config/auth.
Host routes/history are read at request admission; existing bound native catalogs
do not mutate. `status` is read-only; a failed readiness check does not authorize
restarting a live owner. Default native/user model config remains unpinned.

The user1,000,000-token setting is policy, not measured host capacity. Native
compaction/hash triggers, HTTP byte bounds and true Chat remaining context differ.
No independently verified remaining-context sensor exists. Summary acknowledgment
requires the next canonical input; identical text in an unchanged prefix is not
acknowledgment. Public ocx1 checkpoints contain readable context, not private
reasoning or decrypted foreign state.

## ChatGPT projects and history

Project/account/destination identity is explicit. Reuse the exact owned private
project; ambiguous/inaccessible destinations fail. Reconcile an uncertain create
before retry. Safe-boundary changes affect new lanes; preserve existing chats,
files/instructions/memory/access settings. Independent native reviewers remain
outside the main configured project. F destination/reading and G actual project-
guided development/handoff are accepted at their recorded inputs. Conflicting
instruction priority is not inferred. Broad setup UX remains deferred.

## Confirmed operating Chat blocker

Retained Full60 native→Chat history failed unsupported_desktop_route when an old
model's compaction reached the Chat endpoint, and includes an unreadable opaque
checkpoint. Preserve that conversation. Readable-history model selection and later
actual TUI return/re-entry pass; they do not make the old opaque input readable.
Use supported upstream readable handoff at a settled boundary. Original negatives:
chat-full60-operating-diagnosis.json / chat-full60-cross-provider-admission.json.
Known box/MemoryCite formatting fixes have bounded stream-safe proof; they do not
reclassify the historical failed trial.

## Deferred setup and repository work

These are follow-ups, not reopened G/H gates or authorization to start their
implementation during the documentation task.

### Shared minimum setup and login

Use one implementation behind an explicit setup entrypoint and fresh interactive
post-install invocation. Collect only missing inputs for the selected entrance;
reuse configuration, accounts and login. Native login, Linux app login and remote
MCP authentication remain distinct. Use existing installers/host/login controls
and model-free readiness checks. Preserve ordinary launch behavior and unpinned
native model settings. No new setup command exists yet. The manual procedure is
in the [Provider guide](providers/chatgpt/README.md#first-time-setup).

### ChatGPT entry and task management

Existing tools support listing/observing work, original result retrieval,
supported continuation and confirmed Stop. Users can select the installed Termux
Bridge MCP connection and ask short natural-language tasks; `reasoning=current`
and `reasoning=managed` are assistant-facing routing fields. Connection selection
and natural-language routing do not establish a universal automatic default.

The proposed convenience is a Termux Bridge plugin with one reusable workflow
skill for both routes, followed only as needed by a compact project/work panel.
It should reuse existing Provider controls and native history/approval, expose
readiness, bound workspace, reasoning source, work/thread identity, pending human
requests and original results, and reconcile uncertain operations without resend.
No bundled skill, UI resource, project picker or dashboard is implemented. Exact
packaging/selection behavior and actual client support need bounded proof before
becoming a public contract. See the [current/proposed split](providers/chatgpt/README.md#chatgpt-entry-and-task-management).

### Repository separation and cleanup

The user wants a clean development repository without accumulated PR/Actions
history; implementation is explicitly deferred. The intended layout is an old
repository retained as an archive, a fresh private development repository keeping
the `codex` name/source, and a separately named public distribution repository.
`codex-archive` and `codex-dist` are example names, not created destinations.
Preserve Git source/commit/tag provenance and the independent implementation and
publication histories; a fresh GitHub PR/Actions history does not require
squashing source history or deleting the old repository.

Only this owner's device is currently in use; the user accepts a brief install/
update outage. Zero-downtime migration is not required. Reusing the old repository
name removes its redirect, so explicitly reconnect bootstrap, Base/Full and
Provider indices, Release asset URLs, publication origin validators, source pins,
documentation and CI. Git source transfer does not transfer Release attachments,
Actions settings/secrets, app permissions or other repository metadata. Configure
scoped cross-repository publication rights; keep signing/auth credentials private.
Retain immutable original artifacts, signatures, proofs and rollback sources.
Future qualification must check fresh install and this device's Core/Provider
update paths against the new public distribution, with applicable signed release
and Android acceptance. Existing local execution can continue during the outage.
No rename, repository creation, visibility change, source move or endpoint change
has been performed for this task.

## Development artifact retention

Use TMPDIR for disposable roots and XDG cache for compiler outputs. Prune only
verified owned inactive payloads through `scripts/clean_development.py`, after
preserving source/diff/results/native records and current/rollback artifacts.
It verifies fixture key, exact manifest and absent process references; `gp.*/c`
uses the same guards. No age/name inference; unknown repo target stays untouched.

Latest two completed fixture batches archived/pruned12 native payload copies,
1,756,069,888 allocated bytes; original evidence/restore records remain. Last
compiler-cache observation was about4.7GiB, below15GiB. Retain needed executables
outside mutable outputs before pruning idle debug builds. Disposable qualification
used debug information=0, incremental=0, jobs=2. Accepted binaries need no rebuild
merely because their copies were archived. Pinned/rootfs/profile/user data is not
generated-fixture cleanup material.

### Verified execution pitfalls — check before continuing G/H

This is the user's requested pre-execution reference, reconciled against original
failure records on2026-10-22. Read the relevant row before forming a new command;
use the existing implementation/guard rather than reconstructing its procedure.
A failed prerequisite stops its dependent action. Never replay a completed effect
to fix an observer, packaging or reporting failure.

| Pitfall and original evidence | Required pre-execution check |
| --- | --- |
| Wrong signing ref: `phase-g-post-device-20261022T081337Z/dispatcher-registration/plan.json` explicitly says main always fails before signing. This session repeated it in39864984102 | Read the selected workflow body and compare remote rewrite/rust-core HEAD with the accepted source. Dispatch the implementation revision; main is registration/promotion only. Correct source run39865112885 already passed; never rerun its signing |
| Long native HOME: chat-entry.11uglt0w made a116-byte socket and consumed a Chat before native preparation failed; see its retained original acceptance failure | Call existing scripts/qualify_provider_chat_entry.py:check_native_socket_budget before fixture creation or model input. Budget is107 bytes including the actual absolute HOME layout. Retained gp.i3b9_52r/c/h is103; the copied gpub/consumer layout would be112. The operating HOME was checked at84 bytes |
| Downloaded Actions file modes differ from signed payload modes: this session's first raw-package check failed on Cargo.lock | Keep the raw download. Reconstruct a separate package with existing provider_publication.restore, then verify signature/inventory/modes. The earlier consume_public.py already uses this path; do not rebuild/re-sign or chmod the original evidence |
| Dependent shell commands continued after an earlier check failed: this session attempted consume.py after stage audit failed | Run dependent steps sequentially with checked exit/result predicates. Do not put the consumer, promotion or deletion after an unchecked shell command. Stage/result proof must exist and pass before its dependent action |
| Wrong runtime/interpreter/protocol and premature stdio EOF: gf continuation1–5 preparation records,0 model requests | Reuse real Termux sh/env paths, protocol2025-11-25 and request→response sequencing before EOF. Fixture PREFIX is a product fixture, not the interpreter authority. Check imports/assets and the actual tunnel child target before submission |
| Stale PID/URL/lease or incomplete cleanup: retained-owner discovery fault, cancelled gf continuation5, and gfinal.w8_8wm0n stale observer | Compare kernel start/executable, selected owner, work/history and pending calls. Verify native/host Stop separately; absent lane is an independently proven condition, not a fabricated ACK. Audit both bridge and input observer; release only an exact owned inactive hook and restore its original functions |
| False final/continuity assertions: grr generation assumptions, prompt-marker match and historical `id` vs `work_id` list check | Read the actual response schema. Require assistant final content and matching original CLI/native/saved records; do not match the prompt. Preserve original failed results and generation/controller boundaries. No model replay for an offline observer fix |
| Wrong receiving thread or model/config authority: prior-primary bindings and unsupported models.new_thread settings | Bind final notification to actual CODEX_THREAD_ID and verify canonical receipt/turn_context. Ordinary configs stay unpinned; only this development supervisor selects gpt-6.1-sol/high. Generated Core config is output, not the common preference file |
| Stale promotion parent and binary Contents API conversion: prior Full90 CAS rejection and binary-readback-red.log | Read the latest main only after prior mutations settle, preserve independent histories and use non-forced promotion. Reconcile unknown mutation once before any retry. Read signatures through canonical Git blobs/raw bytes, not the text Contents API |
| Retiring unique/user/current data as generic cleanup | Before H removal, preserve experiment refs, exact source/user1m patch and regression, licenses, original proofs and pinned/current/rollback dependencies. Check process references. Use clean_development.py only for owned generated payloads; never prune unknown target contents or an active/unknown worktree |
| Provider update stays busy after its services exit: binding-release-20261023T030234Z | Inspect kernel FD/lock holders as well as executable paths. A native catalog can inherit the Provider directory lease. Preserve metadata and prove no loaded threads, active jobs or children before gracefully stopping that exact idle owner; verify exclusive lease availability before a new update intent. Keep an unchanged-selection rejection distinct from an uncertain mutation |


## Verification and publication

Reuse green proof only for unchanged actual inputs/environment/scope. H changes
no compiled Rust or embedded runtime assets, so applicable accepted Android proof
is reused for those inputs. Provider/full source-removal gates pass separately.
Runtime/state/security/installer changes and releases still require affected/full
acceptance plus applicable Android execution proof. Failed/zero-test required
gates remain unfinished. Original private transcripts/account bindings stay private.

Public assets/history are immutable; release signing/readback/Android consumers
and non-forced CAS promotion retain exact-source bindings. Implementation and
publication histories stay independent. Public Provider source6aa2280 remains
accepted after H; H retirement does not relabel that release as a different source.
Device activation is a distinct user-authorized action with its own results.

### Remaining acceptance and delivery

This matrix records accepted G/H scopes and separately deferred follow-ups.
The A–H work is complete at those scopes. Future actual Chat trials need a declared scope/expected
submissions first. Do not turn completed fixtures/live proof into mandatory reruns
without changed inputs, failures or an unresolved boundary.
G applies to the paths/components retained after F integration, informed by D's
historical route evidence; replacement may retire an old
path-specific gate with its feature. Shared safety, supported upstream behavior,
protected state and signed lifecycle gates cannot be retired as path optimization.

| Area | Evidence already retained | Accepted disposition and separate follow-up |
| --- | --- | --- |
| Design and functional definition | E complete; F1–F9 bounded implementation/integration reviewed above | G retained-capability review accepted at6aa2280; E decisions remain closed |
| Idempotency contract and hints | Common operation/duplicate/recovery logic qualified; Rust12 true/4 false values applied on candidate40 with other hints unchanged; actual native23/23 plus retained loss/restart/summary faults | G packaged-path review retains these invariants. Backend consumption, causal speedup and billing remain unmeasured |
| Linux app/host management | E/F shared host CLI/MCP, private-pipe acquisition/Stop, endpoint reuse/RFB; G3 actual fresh-profile Android host tests and preserved-profile physical reboot recovery | G exact-input/ownership review accepted; current final followed fresh preflight; later device migration verifies the same authenticated account. New provisioning and physical viewer polish remain separate |
| Common bridge and usage routing | F shared managed MCP/local and current direct/general MCP; G four installed usage combinations and later affected managed/direct development | G exact-input review and distinct admitted-candidate native-final follow-up pass. No completed development or comparison is repeated |
| Coordinated agents and orchestration | F actual review/correction/dependencies, isolated writers/conflict and tree Stop; G independent real allow/effect plus separately accepted project/handoff development | Distinct same-thread native-final follow-up and retained composition/resource review pass. Original interrupted/failed cohorts stay failed; H retires duplicate source |
| Shared saved-Chat reading | F discovery/read of unrelated/work-linked sources, duplicate titles/branches/redactions and full ranges; G independently read29 successor messages and matched the assistant native final | G capability/readback review passes; current saved assistant final matches native/CLI. Hidden reasoning remains unavailable |
| Direct entry and delegated return | Earlier direct two-turn and F general delegation/question/final; G installed entrances and affected managed/direct development with original CLI/native/saved finals | G final review reuses these exact accepted scopes. Changed or uncovered future routes require targeted proof |
| User continuity and control | F existing-Chat input/current cursor/acknowledged transfers; G original-TUI same-thread return/re-entry, source generation1→successor2 and original Stop/history reconciliation | Distinct same-thread report-only final passes after the correction, with0 new tools or summaries. Ordinary readable-history model selection requests no fresh summary |
| Host lifecycle/visibility | F scoped Stop/failure fixtures; G actual3 owned fresh-profile host lifecycle tests, physical reboot/request-free recovery, private composition/restart/rollback and20 retained synthetic turns | G exact-owner, pending-call and Stop review passes. Current native/host Stop and restored remote16/read are independently verified; hours-long reliability remains unmeasured |
| Context and TUI | C/C2 actual development,5 same-Chat turns and actual successor development; F transfers; G actual project-guided handoff, original-TUI return/re-entry on hosted5da0254d, plus Android cold-owner and retained-turn fixtures | Installed TUI return/re-entry is closed at that scope. G final review retains applicable context/interruption guarantees. Old unreadable Full60 opaque history stays unsupported; capacity/compaction research remains deferred |
| Tools/approval/question | Full native catalog and retained function/custom/tool-return fixtures, actual command/patch/test development and original question answers; G independent real reviewer allow and exactly-once approved effect | G native-final follow-up and full-surface retained-proof mapping pass; no supported tool is omitted. Physical question presentation remains a separate observation; quoted executable prefix matching retains its upstream limitation |
| Existing MCP return | Actual R11/C/C2/F original call/result/final evidence, G current16-tool/read readiness and corrected list response with3 works; all3 approval-task native results returned | Hosted6aa2280 final delivery and route/Stop reconciliation pass. The old systemError source turn is preserved; program/reviewer/effect were never replayed |
| Cost/presentation | Stream-safe markup projection; D2/D3/F9 timing/request/byte/resource samples; G actual original CLI/native/saved reports and final-only same-primary notice acknowledgments | G result/presentation audit passes. Actual allowance debit and a universal performance winner remain unmeasured |
| Core update information/prompt | User-confirmed85→86 success; Core bytes unchanged; release-date/other display details observed as unknown | Low priority with the next substantive Core change: [deferred Core follow-up](#deferred-core-follow-up), including component information and a prompt on each eligible bare launch; preserve upstream/noninteractive behavior |
| ChatGPT project setup/management | Authenticated project operations and F exact destination/context contracts; G actual project-guided source/handoff and unbound independent reviewer | G retained project/identity mapping passes. Untested conflicting-instruction priority is not inferred; broad setup UX remains lower priority |
| Viewer and desktop | E app/host management research; F Rust private RFB/HTTP/control fixtures and actual owned fresh-profile RFB; existing notification/terminal Linux viewer opener; old desktop font/icon observations | noVNC usability retains lower priority: [development host management and viewer usability](#deferred-linux-chatgpt-development-host-management). Physical current viewer/keyboard/scroll/login claims require separate observation; old observations do not qualify a changed viewer. This polish is outside declared G and the viewer remains optional |
| Packaging/composition | Publicc65d3f8 Base89/Full90 and Provider release sequence 2 signing/readback and actual18-command Android consumer pass; latest6aa2280 hosted source/artifact,84 composition/113 private signed-package steps and Android owner regression pass | G corrected6aa2280 Provider release sequence 3 signing, complete12-asset public readback, Android10 immutable/4 stable consumer commands and non-forced main6e89e76 promotion pass. Old public assets remain immutable; activation is separate |
| Experiment retirement/publication | Unique source/user1m changes and immutable original proof preserved; earlier public delivery is accepted, latest correction is publicly admitted | G runtime/public admission accepted. H preserved unique source/user1m/licenses/refs and retired34 source files plus2 inactive worktrees; Provider/full verification and independent review pass. Pinned rootfs/viewer/current/rollback payloads and explicit Rust fallback remain |


### Deferred Core follow-up

With the next substantive Core change, improve installed/target component versions,
source/date information and missing metadata presentation. On every eligible TTY
zero-argument bare launch with a verified update, offer current→target details and
an update choice; decline/timeout must not hide the next eligible prompt. First
amend SPEC's display-acquisition/six-hour snooze contract. Arguments/non-TTY streams
must bypass discovery/prompts/advisory writes and preserve upstream behavior.
This remains deferred; runtime updating does not implement it.

<a id="deferred-linux-chatgpt-development-host-management"></a>

### Linux ChatGPT app management and deferred viewer work

Use Provider's `host status/ensure/open/stop`, `view`, saved-Chat controls and
shared authenticated facade. Rust ownership/status/Stop and optional RFB/viewer
have bounded proof. Accepted Android intent/HTTP responses do not establish
physical visibility, touch, Korean IME, scrolling, keyboard or login usability.
noVNC mobile polish, broad project setup UX, actual capacity/compaction policy,
optional TypeScript and app-free alternatives remain separate later work.

Native quoted-executable prefix-rule matching retains its upstream limitation;
reviewer allow/effect proof used a plain owned program path. Do not rewrite model
commands in Provider to hide that boundary. Actual costs, hidden reasoning and
long-duration reliability remain unmeasured. No optional follow-up reopens G/H.
