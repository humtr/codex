# Operating state and next work

Implementation: rewrite/rust-core. Publication: main's immutable source caller.
AGENTS.md owns development rules; SPEC.md owns product behavior and boundaries.
This repository carries no external workflow skill or model configuration.

## Operating baseline

Signed production authority is [rewrite/rust-core CURRENT.md](https://github.com/humtr/codex/blob/rewrite/rust-core/CURRENT.md).
The current device/public generation is stable51 (CLI0.161.0); older installed
processes retain their loaded images. This branch owns opt-in provider experiments,
not a parallel production activation pointer or release authority.

## Implementation ownership and connector identity

The Chat provider is this repository's integration with reused miuuyy6.1.5
protocol helpers, not an independent Responses implementation. The installer
bundles its parser, event queue, SSE/errors, tool mapping, turn identity and
summary prefix; patched dependency source and licenses are retained. Our
signed-app transport, lifecycle and Core/Manager integration own the active path.
The full-mode MCP broker exists in dependency source but is not connected to it.
Tool-result continuation currently submits another Chat user message. Native
Core execution proof does not establish same-Chat-turn MCP continuation.

`Codex Native2` was copied from the upstream browser default into an old real
probe. It is not a required MCP wire name or a signed-app route dependency.
Historical browser entrypoints now require an explicit `--connector-name`,
forwarded unchanged to browser selection without a required brand/prefix.
Existing remote connections and installed payload are unchanged. Future work
must qualify native MCP continuation and genuine cancellation before claiming
reduced submissions; renaming is not that implementation.
Three offline selector/entrypoint tests and five affected browser-helper tests
pass. Actual installed6.1.5 `providerConfig` forwards four explicit names,
including non-Codex and Korean labels, without a prefix requirement. This is
selector-mapping proof, not live MCP attachment. No Chat message was submitted.

## ChatGPT internal transport and shared registration

Signed stable51 includes the qualified model menu and shared configuration path;
full source acceptance38557124149 and signed publication38557559491 pass. Native
qualification38552383479/source5f8ff12 passes39 actual TUI tests, lint and pristine
Core/dependency proof. Actual legacy/named model round trips preserve PID/PTY,
account home, native thread and permissions; installed new-home projection plus
real native commands passes without copied auth or base config changes.

Installed experimental provider0de70e3072a4b8e87a31 uses source46db05e. The signed
app's submit/cancel/state own separate temporary conversations per native thread
and guardian child; credentials stay in the app. Continuations explicitly link to
the lane's completed correlated assistant. Internal and GUI transports share the
qualified external-tool/context instructions. App startup waits for its actual
Scope before considering an unavailable-capability fallback. The app and display
remain dependencies; no debug TCP port or extracted first-party token is introduced.

Real GPT-6 overlap/cancel-sibling, two native clients/four commands and two actual
reviews pass. App acceptance timestamps measure overlap directly. Tiny Instant replies can
finish before the sibling starts; a bounded Medium trial proves actual overlap
and cancellation/sibling survival. Earlier non-overlap trials stay failed evidence. Manual
Medium compaction passes persisted tool-only state, four native commands and both
Core/whole-app restart/resume. GitHub catalog search plus the actual discovered
read-only repository call passes in wrlab. A controlled no-inference native catalog
comparison returns8 GitHub matches in wrlab and0 in szob; app login does not grant
another native account's connector tools. Failed/timeout trials remain evidence.

34 protocol/app/body tests (225 assertions) and15 desktop/profile tests pass for the current source.
Packing/admission/accounting changes are not yet installed. GPT-6 Instant/Medium/High/
Extra High actual tool loops pass; Pro inference is prohibited and was not sent.
GPT-5.6 remains deferred. Automatic/fault compaction under the new internal path,
multimodal input, hours-long load, physical reboot and phone layout remain separate
gates. Historical GUI qualification is not blanket internal-path acceptance.
Private proofs: ~/.cache/codex/acceptance/chat-internal-checkpoint-startup-wait.json,
chat-internal-discovered-tool-wrlab.json, native-account-catalog-comparison.json,
chat-final-concurrent-guardian.json, chat-final-app-acceptance-timing.json,
chat-final-medium-overlap-cancel.json and chat-final-installed-inventory.json.

## ChatGPT model errors: next work

Status: runtime repair unresolved; controlled diagnosis and adapter changes on
2026-10-14 are source-only. User-visible failure:
`stream disconnected before completion: error sending request`. Device stable51
and provider 0de70e3072a4b8e87a31/source 46db05e were inspected without changing
runtime, configuration, conversations or authentication. The installed protocol,
app transport, Responses server and profile source matched the maintained files
before this source change; installation remains unchanged. Candidate admission
repairs now handle identity/zstd bodies at16 MiB encoded/32 MiB decoded and reject
opaque/corrupt compaction before parser placeholder conversion. These changes
are not yet installed. The candidate now carries the upstream-resolved catalog
from the local frontend to Core's existing backend coordinator. Core's private
catalog operation passes actual Android request admission with12 tools, same-byte
server reuse, changed-byte separation, previous-thread readability and Manager
writer discovery. The self-contained probe no longer requires an untracked model
fixture or a newer checkout's WebSocket helper. Runtime remains unchanged;
release frontend38654727667 builds and both legacy/named-permission Android TUI
fixtures pass entry into Chat and return to native, including backend catalog
binding, same PID/PTY/thread/home, sibling defaults and preserved configuration.
The upstream `--profile chatgpt` fallback passes the same actual Android
round-trip probe with zero inference, alongside legacy/named permission cases.
The first CI fails before test execution on an error-type conversion in the new
test, fixed in a20c58d; complete native qualification38657053854 is pending.
Cross-provider compaction/resume and real Chat model-use remain unqualified.
Local affected acceptance passes77 Python tests and323 Rust tests, with one
existing ignored resolver smoke, plus fmt/Clippy/diff. The first full run was
invalidated by overlapping rebuild of its executable; the sequential rerun also
exposed a fixture-only107-byte socket overflow after its test counter grew.
The fixture path is now shorter. A final full test run passes; two redundant
borrows found by Clippy were corrected and lint passes separately. These are
source gates, not a signed release or completed model-use acceptance.
An additional installed-app read-only check found two app:// pages after cold
startup: the small launcher is first and lacks the main model/Scope context.
The source CDP path now positively selects the main app using the pinned model/
Chat-Work UI capabilities, detaches the launcher, and reuses the selected page.
Actual Android discovery and35 Bun tests/230 assertions pass without submission.
Only the previously installed desktop app was restarted after verifying its
controller/host/browser were dead; no binary, login or provider setting changed.
MCP plugin attachment is still unqualified; menu visibility is not tool-call proof.

Confirmed observations:

- At 13:26:26-13:26:27 UTC, the shared backend logs unknown model
  `chatgpt-desktop/gpt-6` and uses fallback metadata. Its fresh-thread request
  reaches loopback `/v1/responses`, which returns 400 `unsupported_desktop_request`.
  The error covers multiple admission checks; the exact rejected field is not
  captured. Do not assume a particular check failed from the generic code alone.
  Frontend picker visibility/provider selection does not prove that the backend
  loaded the intended model catalog and request capabilities.
  The no-inference `native_catalog_request_probe.py` now reproduces this boundary
  through the installed official Android backend in disposable homes: per-thread
  catalog override still produces unknown-model metadata and an unsupported routed
  `web_search` declaration (43,348 JSON characters in the owned fixture); the same
  catalog loaded at server startup produces an admitted request (16,894 characters).
  This proves a controlled mechanism, not the uncaptured historical rejected field.
  Actual wire bodies are42,543 and16,387 bytes respectively; the character counts
  above reserialize the owned JSON and are not the measured HTTP byte lengths.
  Production model lookup ignores the per-thread `ModelsManagerConfig.model_catalog`;
  frontend projection alone cannot repair the startup-scoped catalog.
- At 13:26:59 UTC, the existing long thread's pre-turn ModelDownshift compaction
  sends model `gpt-6.1-sol` to the newly selected loopback Chat endpoint. The HTTP
  send fails (`is_timeout=false`, `is_connect=false`) and the backend records
  `Failed to run pre-sampling compact`. Upstream previous-model compaction uses
  `with_model(previous_model)` while preserving the current provider; a provider
  transition therefore needs an explicit compatible compaction handoff.
- `desktop_responses_server.mjs` sets `maxRequestBodySize:2*1024*1024` before
  context staging. Two deliberately invalid local requests, which cannot reach
  Chat submission, return 400 at approximately 1 KiB and 413 above 2 MiB. The adapter
  proof file remains byte-identical. Active-history payload serialization before
  the failure totals 2,277,814 bytes since the last compact; this is not a measured
  HTTP body. The ceiling is a confirmed limitation and a plausible contributor
  to the send failure, not a proven explanation of that exact failed request.
  `desktop_server_admission_probe.py` reproduces both a header-only 413 and a
  BrokenPipeError while sending an oversized body. The missing historical body
  size remains unproved; the installed ceiling is unchanged. Candidate HTTP
  acceptance admits decoding of owned3 MiB identity/zstd packets, categorizes
  invalid encoding/compression/UTF-8 and bounds expanded data before Chat access.
  A33 MiB compressed expansion returns typed413; over16 MiB wire requests can
  still fail with BrokenPipe during an early413. No unlimited-size claim follows.
- The adapter is alive and its read-only readiness response verifies against
  its existing private binding. Zero admitted-request/submission counters do not
  mean zero HTTP arrivals: the observed 400 occurs before admission accounting.
  No Chat inference or chat-limit consumption experiment was performed. Nothing
  observed establishes a Chat message-limit, Cloudflare or login rejection.

Evidence remains private in `~/.codex/logs_2.sqlite` and the affected rollout's
metadata. Use the timestamps and error targets above to inspect narrowly; do not
publish prompts, tool results, credentials or raw conversation records.

Required follow-up, before claiming ordinary `/model` use is qualified:

1. Finish frontend build and actual TUI qualification of resolved-catalog startup.
   The minimal candidate uses the frontend's existing upstream config loader and
   Core's existing server/retirement implementation, without a TOML loader, new
   executor, dependency or proxy for native inference. A global merged catalog/
   model router was considered: it also changes native inference, auth and compact
   routing and would introduce an optional adapter dependency into that path.
   The bounded startup fix therefore precedes reload removal. Catalogs are keyed
   by exact bytes/home/runtime, not model/effort or a mutable source filename.
   Their private `catalogs` records leave the native `servers` rendezvous unchanged;
   current Manager discovers both, while older readers can ignore the new family.
   Keep per-account native inference and otherwise independent Chat login.
   Qualify the actual frontend selection/resume path, native siblings and retired
   catalog generations before activation. No pristine upstream backend is patched.
   Once startup is qualified, bypass re-entry for changes within the same Chat
   provider using upstream model/effort actions; keep the session-only guard
   independent so Chat slugs cannot become the native default. Cross-provider
   re-entry and its compaction handoff remain separate from this smaller saving.
2. Establish a compatible old-provider compaction/new-provider resume boundary
   for long histories, preserving canonical history, permissions and sibling
   routes. Do not bypass the defect by deleting history or silently truncating it.
   A new offline fixture proves the bundled parser accepts an opaque native
   `compaction.encrypted_content` and replaces it with OPAQUE_COMPACTION_NOTE.
   The adapter's existing guard catches opaque agent payloads, not this checkpoint
   loss. This is an additional repair gate: preserve/produce readable canonical
   context or explicitly refuse; a successful HTTP reply with a placeholder is
   not compatible resume. No real/private checkpoint was read or submitted.
3. Measure actual serialized request bytes and handle the body ceiling explicitly
   before app context staging. Distinguish oversized input, unsupported shape
   and network failures in the user-visible error; increasing the ceiling alone
   does not solve route/catalog or compaction incompatibility.
4. Qualify fresh selection, long native-to-Chat and Chat-to-native switching,
   pre-turn compaction, restart/resume, sibling isolation and preserved native
   tools on the actual installed Android path. Earlier direct-profile tool loops,
   manual internal compaction and inference-free menu round trips retain their
   narrow scope; they are not proof for these failing combinations.

Runtime/catalog/cross-provider compaction repair and real Chat inference remain
unfinished; bounded body decoding and pre-parser checkpoint rejection now have
source fixes and actual Android no-inference proof. Explain
trial necessity and expected submissions before performing real inference.
Chat message submission/limit accounting is a separate review, not a diagnosis
or resolution of these model-use failures.

## ChatGPT provider cost and debt reduction

The selected objective is to improve ChatGPT as a model inside local Codex with
the least maintenance and message-submission cost. tdev progress/control proposals
belong to a separate session; neither its runtime nor a new workflow controller,
display or autonomous agent is a prerequisite for this provider.

Current findings and source changes on 2026-10-14:

- An offline synthetic packet of 50,410 bytes (five tool definitions, 24 history
  messages and instructions) previously became27 stages:26 context ACKs and one
  work request. Ordered packing now produces2 stages: one context ACK and one
  work request. This evaluates the previous commit and current code on the identical
  fixture using only `desktopStages`; no Chat message was submitted and no
  quota was measured. Splitting every old message and issuing an empty tool-reset
  ACK separately are avoidable submission overhead, not a provider requirement.
- The signed app has `extraDeveloperInstructions`, `prependMessages` and a real
  tool-result continuation export (`bRt`/`Nwi`). These are investigation candidates,
  not acceptance. Its generated local function signatures are currently only
  Work/Codex `handoff` and are empty in Temporary Chat; `Jui` supplies no arbitrary
  local tool names. `toolOrigin` marks an SDK follow-up prompt, so changing the
  author label alone does not establish native tool continuation or quota savings.
- The installed payload already has one bundled wire bridge and one adapter.
  UI fallback still carries a second submission/retry/ownership implementation;
  noVNC/display control and numerous qualification sources are shipped alongside
  inference code. Keep necessary signed-app/X dependencies until independently
  qualified alternatives exist; a headless or token-extraction shortcut is unproved.

Source-only implementation packs whole schemas/history with one final work phase,
preserves oversized instruction/message fragments and explicit empty tool resets,
and refuses an oversized whole schema before submission. Existing warm deltas
and canonical history ownership remain. Fixed admission codes reveal no request
or error bodies; authenticated arrivals/rejections are distinct from admitted work.
Work submissions retain their counter; setup submissions are observed before ACK,
so an accepted setup with a failed ACK is still counted. Total observed submissions
separate app acceptance from weaker GUI observation; retries and completed ACKs
remain separate. None of these counters measures chat-limit deductions.

Remaining order; no runtime activation or real Chat submission performed:

1. Repair the recorded catalog/admission, provider-crossing compaction and body-size
   boundaries. Preserve ordinary upstream execution and one authoritative history.
   Do not hide incompatibility by dropping tool/context features, silently trimming
   history or forking pristine upstream Core.
2. Qualify implemented ACK packing/accounting through actual submission, preserving
   the first tool-list reset and warm deltas. Assess the app's native
   instruction/history parameters before assuming the old UI character limit is
   necessary internally. Preserve data roles, whole schemas and final-work separation.
   Offline replay/byte-boundary tests precede any real submit. Measure work, setup,
   tool-result, guardian, compact and retry submissions independently of quota.
3. Independently implement a minimal native-Chat MCP continuation, using miuuyy's
   full-mode flow as a reference rather than importing its broker/turn executor.
   Use AgentDock as the lifecycle reference, adapted to our exact Core turn APIs.
   Exercise the existing adapter/tunnel
   path, without tdev as executor or another agent. Chat-issued calls should become
   ordinary Core tool calls; actual Core results should satisfy the original MCP
   call while the same Chat turn continues. This is a transport replacement, not
   a script that invokes Codex recursively. Preserve all advertised native tools,
   reviewer/approval, cancellation, call identity and independent thread ownership.
   The app turn must survive an expected Responses tool boundary but stop on actual
   cancellation; account/plugin/Temporary Chat availability and MCP timeouts are
   explicit feasibility gates. Do not build a general broker before this tiny proof.
4. After acceptance, keep one inference transport, make GUI viewing optional and
   package reachable runtime dependencies separately from tests/history. Remove
   a replaced path only after its full affected acceptance. If native continuation
   is unavailable, retain the optimized text adapter with its explicit per-judgment
   submission ceiling rather than accumulating speculative compatibility layers.

The first real native-MCP proof would use one user message for two harmless Core
tool calls and a final answer, versus three work submissions through today's text
loop, plus any separately counted setup/review overhead. These are submission
expectations, not chat-limit deduction claims. Explain the exact candidate and
budget before real inference/provider changes; Pro inference remains prohibited.

Private no-inference proofs: `~/.cache/codex/acceptance/chat-stage-packing.json`,
`chat-catalog-request.json`, `chat-admission-bounded-repair.json`,
`chat-catalog-merged-offline.json` and `chat-existing-broker-offline.json`.
Current HTTP proof covers ten distinct admission/decode/checkpoint failures,
401 without auth, large identity/zstd requests, compressed expansion bounds and
zero admitted work/Chat submissions. Protocol/app/body tests pass34/225 assertions
and desktop/lifecycle tests pass15. This is not a real Chat
quality, ACK/cancellation observation or provider-switch qualification. User data,
credentials, active runtime and conversations are preserved.
`scripts/check.sh full` passes77 Python and323 Rust tests, formatting, Clippy and
diff checks; the separate Android HTTP/catalog proofs above cover the affected
real request paths without inference. An installed/live Chat gate remains open.

Historical reference proof: the installed6.1.5 broker was exercised over its real temporary Unix socket:
parallel exact-ID replay, one-time pending result resolution, completion fences,
observer abort with retained turn, explicit revoke/late-call rejection and sibling
survival pass. No Chat, tunnel or Core inference occurred; this proves reusable
broker mechanics, not a live connector or durable pending promises after restart.

## Chat On Steroids reference review

Review-only source inspection on2026-10-15: original repository
totec448-spec/chat-on-steroids, commit2d1c86efed698ed759e17efded4e12ca2d78b28e,
package2.1.37/MIT. GitHub identifies dikoweii/chatonsteroids as its fork, not an
independent design. The reference was cloned only under TMPDIR; no dependencies,
installer, service, extension, tests or Chat inference were executed. Its tests
were read as intended regression coverage, not reproduced acceptance.

The product is a browser ChatGPT-to-local-MCP executor with its own sessions,
not a Codex Responses model provider. Do not adopt its executor, durable history,
Goal helper, browser extension or desktop app as another operating authority.
Useful principles to implement independently in this repository:

- `src/main/session/correlation.ts` joins provider request IDs to exact observed
  conversations; active tab, timing and tool names are not owner evidence. One
  workflow request ID can contain many tool calls. Our binding additionally needs
  Core home/backend/thread/turn, remote conversation/generation and distinct call
  IDs; never use a workflow ID as per-call deduplication. This reference's browser
  evidence cannot be assumed present in our signed app.
- `src/main/codex/unified-exec.ts:acknowledgeCompletedOutput/offerCompletedOutput`
  retain output until publication and a later owner receipt, reoffering a failed
  publication without rerunning the command. Socket finish is weaker than remote
  receipt, and neither proves user-visible rendering. Use exact Core results for
  retained MCP responses; do not copy its process manager or treat its later-call
  receipt heuristic as exact per-call acknowledgement.
- `src/main/mcp/call-context.ts` separates machine-changing work from recorder
  settlement. Our compaction/switch barrier must settle real Core operations and
  canonical history before rebinding; an HTTP response ending or a bookkeeping
  counter reaching zero cannot establish that the machine stopped changing.
- `src/main/session/continuation.ts` separates preparation from ambiguous dispatch,
  reserves a single replacement owner, and commits durable state before publishing
  the new mapping. A dispatched-but-unconfirmed prompt is not automatically
  replayed. Adapt this invariant to Core-owned checkpoints, not a second session
  WAL. An adapter restart does not make old in-memory MCP promises recoverable.
- `src/main/mcp/code-mode-tool.ts` batches nested tool calls and emits selected
  outputs; this is bounded composition, not another model reasoning loop. Keep
  the native Core tool surface, including existing code mode; add no QuickJS
  runtime, reduced schemas or duplicate filesystem/network executor. Batching may
  reduce calls/context but does not measure Chat-limit deduction.
- `src/main/bridge.ts:stopSessionTurn` exposes exact-turn stop pending and explicitly
  leaves native cancellation unconfirmed. This is a useful display distinction,
  not a proof of killed local work. Our lifecycle implementation follows the
  requested AgentDock distinction: observation detach, explicit stop, confirmed
  Core/tool outcome and unknown/timeout must remain different states.

Avoid adopting automatic recovery/Goal as an inference-free fix.
`docs/recovery-inventory.md` describes bounded reload/reopen repairs that may queue
automatic Continue; these can add Chat submissions. `src/main/goal.ts` selects
another model/helper to author follow-up user messages, so it adds both decisions
and submissions. `src/main/session/finish.ts` holds a tool for up to25 seconds and
returns HELD/RELEASED; its own comment says HELD is an instruction, not a server
lock on finalization. It can return eligible queued input through a tool result,
but cannot establish user/developer role equivalence, perpetual turn retention,
reliable provider stop, or quota savings. Do not add this as a mandatory finish
tool or Pro test.

Implementation consequence: retain the previously selected split (miuuyy flow
reference, AgentDock lifecycle reference, independently written Core binding).
CoS adds exact ownership, result-delivery and ambiguous-dispatch regression cases;
it does not replace our signed-app transport or resolve its MCP/Temporary Chat
availability gate. Next offline proof must use two sequential and parallel native
calls, duplicate/reordered result delivery, observer detach/reconnect, exact-turn
stop and sibling survival, interrupted compaction, and restart with ambiguous
dispatch. Assert zero result-phase user submits and no repeated native effects.
Then the minimal separately explained real trial is one accepted non-Pro Chat
task with two harmless Core calls and a final answer, with context/retry/review
submissions counted separately. User-visible progress, stop outcome and latest
Core resume need actual client observations; server send/output retention alone
are insufficient. No implementation or operational change followed this review.

## Output silence and user-visible progress: additional review

The user reports that most apparent ChatGPT output freezes still allow ongoing
reasoning/tool work. Treat that as an important user observation, not a measured
90-percent incident classification or proof of one root cause. Silence must not
trigger stop, Continue, a new conversation or replay by itself. Execution,
provider observation and user-visible rendering are distinct states.

Current-source findings (no inference or fault injection):

- `desktop_app_transport.mjs` submits through the signed app's internal function
  and reads its Scope/JRn conversation store, rather than scraping rendered reply
  text. A stalled visible transcript with a healthy store need not block this
  path. However initial Scope discovery uses React hooks/DOM and each later read
  uses Runtime.evaluate in the app renderer. Renderer stalls, app/store/stream
  failures and CDP loss remain dependencies. This is not an independent backend
  service or proof that the user's freezes are harmless here.
- Only finished_successfully assistant messages with the exact correlation are
  returned; partial progress is not projected. Instant has a130s deadline, other
  non-Pro modes600s. The catch path attempts controller abort/eHt/lane deletion
  after any attempted submission, including observer failures. That cleanup is
  incompatible with blindly retaining a future MCP turn across observer loss;
  classify failure before cleanup. A timeout or successful cancellation request
  does not prove stopped generation/tool work.
- `desktop_responses_server.mjs` injects heartbeat every2s independently of provider
  progress. The reused bridge emits a parser-ignored response.heartbeat; this can
  keep transport alive but cannot prove provider liveness, useful progress or
  user delivery. Native tool items/deltas are actual Core evidence, while the
  current proof counters are global aggregates, not a per-turn progress UI.
- Upstream0.161 TUI already renders turn lifecycle, item/command output and
  completion/interruption from app-server notifications. Reuse this path, rather
  than another tool runtime or fabricated assistant commentary. Official app-server
  documentation confirms these distinct events; installed-version support and
  reconnect/subscription semantics still need their own affected-path proof.

Recommended independent design, pending implementation:

1. Keep internal inference/continuation as the default. Add a minimal factual
   status in the existing Codex UI by default: awaiting provider, executing tools,
   awaiting approval/input, observation unavailable, stopping, confirmed terminal.
   Keep transport reachability, last observed activity age and actual work progress
   separate. A spin animation or heartbeat must never refresh the work timestamp.
   No invented percentage, generated reassurance or claim of ongoing reasoning.
2. Offer optional expanded native tool/output details and an explicit owned Chat
   viewer. The viewer must attach to the same actual conversation/turn without a
   new user submit, recreation, stop, reload or mode change; omit the option if
   exact attachment cannot be qualified. Merely reopening today's UI transport
   does not prove this: its current fallback can construct another temporary lane.
   Display failure is an observation fallback, never an inference fallback after
   submission. Changing display mode must preserve all tools, permissions and
   model/effort, and must not increase Chat submissions.
3. Recover observation of the same owner/generation first. Keep the MCP call and
   Core result IDs; replay only retained delivery, never actions. If the provider
   or exact owner cannot be verified, report unknown and fence new admission,
   rather than guessing stopped/running or automatically resending. Separate a
   deliberate execution deadline from display/observer inactivity. Explicit stop
   still closes admission, interrupts the exact Core turn, cancels its remote
   generation and reports confirmed/partial/unknown outcomes independently.
4. Keep canonical Core history and native item delivery; any small adapter status
   projection is disposable and contains no alternate history. Prefer existing
   frontend events/status surface. If extra transport telemetry is needed, use
   the adapter's existing authenticated local boundary with exact home/backend/
   thread/turn binding and content-free fields, no new daemon or fake model text.
   Do not repeatedly thread/resume merely to observe; it can alter subscriptions.

Qualification must separate these fault cases: freeze only visible rendering
while Scope/tool activity continues; disconnect only the observer and reconnect
to the same turn; hold provider output with only adapter heartbeats; drop a tool
result response after the native effect; stop during a live native operation;
restart adapter with ambiguous dispatch. Verify actual client labels/output and
last-activity age, no duplicate mutations, zero extra user submits for observation
and display toggles, same call/result ownership and latest Core state on resume.
No heartbeat-only trace may show verified progress, and stop requested may not
show stopped until evidence arrives. Existing read-only Android turn observation
proves a narrower non-ephemeral mechanism; it does not qualify full notification
reattachment or ephemeral monitoring. Start with offline fixtures; explain the
scope and expected submissions before a bounded real non-Pro trial. This review
changed documentation only; no runtime, provider or Chat conversation was changed.
The official installed Android backend also accepts a synthetic merged startup
catalog with the same12 parsed native tools as its Chat-only startup catalog;
per-thread override still fails. This qualifies startup metadata placement only,
not a production merged catalog or reload-free route. An actual turn/interrupt in
that disposable backend closes the model SSE connection and emits no additional
provider HTTP request. Therefore the current adapter cannot identify explicit
user-stop from socket closure alone. Keep its existing cancellation behavior
until an exact-thread/turn control path is qualified; simply detaching req.signal
would break stop while enabling retained MCP continuation. Code paths and fixtures
remain opt-in source work; production stable51 and installed provider46db05e remain.

Additional Android no-inference proof (`chat-core-resolved-catalog.json`) reaches
the real native exec_command, interrupts while that owned Python process is
running, and observes the same turn through a second connection using only
thread/turns/list. Its state changes inProgress→interrupted, its request metadata
contains the exact native thread/turn pair, and that regular test process exits.
No new provider HTTP request accompanies stop. Thus bounded authoritative native
state observation is a candidate control path without another service or executor;
it is not yet connected to the retained broker. The same runtime rejects both
turns/list and full-history read for ephemeral threads. Native server/turn binding,
ephemeral/reviewer cancellation, reconnect and resistant process-tree handling
remain gates; this ordinary process is not evidence of general forced termination.
Do not silently downgrade ephemeral tools or resume a stopped turn to observe it.

GitHub source comparison pins (read-only; no candidate code executed or installed):

| Repository | Inspected HEAD | Relevant result |
|---|---|---|
| miuuyy/codex-chatgpt-web | 889f291 /6.1.9 | Full MCP requests suspend until Core returns exact call IDs; HTTP sampling rounds reuse that Chat response. Merged native/Web catalog and model dispatch preserve the built-in provider. Installed bridge remains92a356f /6.1.5; only parser/SSE helpers are bundled into our active adapter, not its full-mode transport. |
| a252937166/codex-chatgpt-web-bridge | 85cc3d0 | Each sample and protocol-repair attempt submits another browser prompt. Native proof documented against0.144.6 is a mock sequence. Tool-result truncation and opaque-checkpoint placeholders are unsuitable for lossless adoption. |
| guberm/chatgpt-web-provider | e18fc0d | Responses-shaped browser facade; stream:true returns501 and no native tool loop. Unverified model selection can still report the requested model. |
| connectedGraph/chatgpt-desktop-bridge | 4d159df | CDP DOM submit/read for Chat Completions; latest user text only, no Responses endpoint/native tool loop. |
| adamallcock/codex-chatgpt-control | 1e50539 | Browser SDK, not a drop-in provider; useful send-once and ambiguous-submission ownership rules. |
| thezillo/codex-proxy | 293ac38 | Uses /backend-api/codex/responses with Codex auth, not the ordinary Chat conversation transport. Useful stream/error pass-through; not evidence of ordinary Chat message savings. |
| l0z4n0-a1/chatgpt-bridge | 381b0e7 | src/config.ts and upstream.ts use /backend-api/codex and Codex auth.json. Browserless transport, but not an ordinary Chat quota replacement. |
| XiaoPuOuO/openchatx-mcp | c6ae729 | Direct official MCP local tools with Chat as planner; no Responses provider. Adds its own tool runtime, jobs, goals and summary store. Summary retrieval consumes/deletes the handoff; unsuitable as a replacement for durable Core history. |
| int04/ChatCmd | e604ac8 | Independent MCP runtime plus extension/DOM automation. Useful operation fencing and persisted compact/resume ordering; immediate user updates can ride an existing tool result. No native Codex Responses provider; its runtime/database would duplicate ours. |
| cromat/chatgpt-web-session-proxy | 753d7d8 | sessions.ts calls browser.sendPrompt for each delta, including tool outputs. Persistent conversation/shadow transcript saves repeated context, not user submissions. |
| Octo-Lex/ChatGPT-Web2API | 497527d | CDP Chat query tool/REST adapter, not inverse native MCP continuation. Useful anchored observation-only recovery; backend_client still needs a browser and token/session maintenance. Shared MCP tab isolation is not enabled by default. |
| tang-vu/tab2api | b624600 | Responses mapper has empty tools; Anthropic client tools use text envelopes and another UI request. Stream output is buffered. No improvement over our native tool transport. |
| yo20ywork-max/chatgpt-codex-bridge | 55a98c0 | codexRunner.ts spawns codex exec/resume as an intelligent worker; moves execution to another Codex inference loop rather than making Chat the current Core's model. |
| DeepCogNeural/codex-gpt-bridge | fe923d5 | Chat-to-Codex invokes official codex mcp-server; inverse ask_chatgpt uses paid Responses API. Different inference/accounting route. |
| lanqian528/chat2api | a63a90c | Direct private conversation protocol, last inspected commit2025-03-28. action=next per request, token/challenge maintenance, text projection of server tools; no demonstrated Core tool-result continuation. |
| littleCat212/bridgeGPT | e85e369 /5.0.6 | GitHub API confirms a miuuyy fork; same MCP broker design. Not independent evidence of a better architecture. |
| uvwt/agentdock | 8c23892 | Independent Go MCP executor, no model inference/Responses provider. Commands live under runtime context rather than request context; request cancellation yields a retained running session. Explicit kill waits for process-tree exit and reports timeout/failure. Optional ACP hosts another agent rather than replacing its model. |
| ChesterRa/cccc | 953ad1f /0.4.43 preparation | Native Rust coordination daemon with browser-delivered Chat actors and direct MCP workspace tools, not a Responses provider. Separates delivery/read/reply/completion; code mode batches nested tools. Also implements a browserless Dots/MCP Events route, subject to different host/usage/identity constraints. |

Primary reusable flow: one accepted Chat task calls the tunnel MCP; the broker
emits that pending call through native Responses/SSE, Core executes it, and the
next HTTP request's function_call_output resolves the original MCP promise. Chat
continues the same response rather than receiving another user prompt. The current
adapter instead calls app yRt once per JSON judgment/result packet. MCP full mode
therefore offers a concrete submission-reduction candidate, not measured quota
savings. Initial multipart setup, reviews, compaction, retries and new user turns
remain separate costs. Its90s MCP deadline,30s agent-wait polling, exact call/result
ledger and observer-versus-turn cancellation deserve targeted reuse, not the
whole desktop launcher/installer or broad native V1 downgrade.

Do not copy the source blindly: its opaque native compaction replay degrades to
a note, while transparent ocx1 summaries are readable. Its browser integration and
account-bound proofs do not establish our signed Linux-app/CDP or Android path.
Reusable references at the pinned commit: [tool/result loop](https://github.com/miuuyy/codex-chatgpt-web/blob/889f291d06339400a8b9c2941473f65cd47d4830/src/adapters/chatgpt-web/index.ts#L1302),
[pending MCP resolution](https://github.com/miuuyy/codex-chatgpt-web/blob/889f291d06339400a8b9c2941473f65cd47d4830/src/adapters/chatgpt-web/turn-broker.ts#L496),
[model catalog](https://github.com/miuuyy/codex-chatgpt-web/blob/889f291d06339400a8b9c2941473f65cd47d4830/src/model-catalog.ts),
[native dispatch](https://github.com/miuuyy/codex-chatgpt-web/blob/889f291d06339400a8b9c2941473f65cd47d4830/src/server.ts#L644),
and [opaque checkpoint limitation](https://github.com/miuuyy/codex-chatgpt-web/blob/889f291d06339400a8b9c2941473f65cd47d4830/src/responses/compaction.ts#L75).

Expanded comparison does not establish a superior ordinary-Chat provider protocol.
It favors a smaller adaptation: keep the already qualified app-owned sign-in and
internal submission transport, connect the existing native MCP continuation to
Core's executor, and scope merged-catalog/model routing to opted-in execution.
Changing the submission transport alone cannot turn a fresh Chat generation into
the result of an outstanding tool call. Removing the Linux/browser dependency is
still unqualified, not a benefit proved by a browserless Codex proxy. App-native
MCP/Temporary Chat compatibility must pass before selecting this as production.
Normal model switching and full-mode tool continuation are independent changes;
neither solves the opaque native checkpoint handoff by itself.

Reuse narrow lifecycle ideas, not another management plane: observation failure
must reobserve the same turn without resending; disconnecting one HTTP sampler
must not cancel its retained Chat turn; actual user interruption must revoke new
tool admission, interrupt owned active Core operations and confirm the terminal
state. Core events/history remain the user-visible and resume authority. A compact
handoff must durably preserve state before rebinding; estimated context budgets
or a one-shot summary UUID do not prove lossless resumption. Injecting new user
instructions through a tool result may avoid a submit but changes their role;
keep this separate from result continuation and do not claim native steering parity.

Official OpenAI documentation was rechecked: [SIWC plan usage](https://learn.chatgpt.com/docs/sign-in-with-chatgpt)
explicitly uses Codex/ChatGPT Work allowance; [public Responses authentication](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
is a supported browserless route, but does not meet the separate ordinary-Chat
allowance objective. [Workspace Agent triggers](https://developers.openai.com/workspace-agents/trigger-runs)
can return a conversation URL/run status but still cannot retrieve the final
agent response; an MCP callback provider remains speculative. No new inference,
real quota experiment, credential access or runtime change occurred. Ten further
repositories were source-inspected in TMPDIR, including one confirmed fork; no
candidate code or installer was executed. Acceptance still requires one accepted
Chat task, two harmless Core calls/results and final output, with each setup,
review, retry and compact submission counted separately; long calls, parallel
calls, cancellation and observation reconnect must not duplicate native actions.

AgentDock/CCCC add useful execution and lifecycle evidence, not a reason to add
another tool executor or collaboration daemon. AgentDock command.go explicitly
keeps an operation alive after its MCP request ends and command.go:killSession
waits for process-tree exit before reporting killed. Copy that distinction into
the existing Core-to-MCP binding: sampler/observer cancellation is not user-stop;
explicit stop must halt new admission and confirm the owned execution outcome.
Its task checkpoint store does not retain a model's hidden work state. Linux
arm64 release targets do not establish Android/Termux compatibility.

CCCC code_mode.rs runs a Node cell with yielding/nested tools and revalidates
Actor generation/binding. That can reduce tool round trips and intermediate
model-facing output for known batches, but is not an autonomous model loop or
measured Chat-limit reduction. Its local_tools.rs executes CCCC's own repo/shell/
Git tools. Wrapping that runtime would duplicate Core's executor and approvals.
Reuse delivery-versus-completion semantics and exact host-session pairing instead;
do not infer work completion, user-visible delivery or a stopped cloud generation
from a webhook receipt or an Actor access revocation.

CCCC's Dots path is a materially different browserless initiation candidate:
MCP2.0 event callback wakes a saved host task; runtime_wait_next_turn claims exact
queued work; temporary execution authority permits tools and exact completion.
The implementation serializes accepted work and binds one Dot to one Actor/Group
per connector; shared Dot memory is not independent per-Core-thread context.
No Codex Responses mapping or per-request model/effort control is established.
Its in-process HTTP fixtures explicitly do not contact ChatGPT. Source inspection
is not real Dots subscription/callback/account qualification.

[Official MCP Events](https://developers.openai.com/plugins/build/mcp-events)
supports Work Cloud and Dots, requires protocol2026-07-28, persistent webhook
subscriptions and verified outbound callbacks; it is not an ordinary Chat
message-submission API. [Dots access and usage](https://learn.chatgpt.com/docs/dots)
lists Pro100/200/500, Business Premium and Enterprise rollout. Dot conversations
do not count toward Chat limits; delegated Work/Codex still uses those products'
allowance and deeper work has its own allowance. Do not promise free unlimited
inference or availability to this user's Plus/unspecified Business account.
Dots is excluded from current work at the user's request because it is not
available now. These findings are historical comparison only: no event transport,
subscription, entitlement investigation or Dots implementation is an active task.
Neither candidate code nor tests/installer was executed; no Chat task, subscription,
callback, user state or operational runtime was changed during this source review.

Concrete review scope without Dots; these are acceptance targets, not implemented
repairs or approval for live provider activation:

| Boundary / actual code | Investigation or candidate change | Required proof |
|---|---|---|
| Native catalog and /model: frontend/manager-tui/native.patch, native_catalog_request_probe.py | Verify the actual backend startup catalog and capabilities; evaluate an opt-in merged catalog/model route before removing ChatModelReentry. Preserve independent native execution and app-owned Chat login, including an empty Codex home. | Actual backend requests use selected metadata; native-to-Chat-to-native retains thread/PTY/configuration/permissions. Adapter absence cannot break ordinary native launch. /switch account re-entry is separate. |
| HTTP admission: desktop_responses_server.mjs fetch/req.json, desktop_protocol.mjs admitDesktopRequest | Measure wire bytes and Content-Encoding; qualify bounded decoding/size errors before context staging. Reject unreadable native checkpoints before the parser replaces them with a note. | Auth/shape/size/checkpoint failures are distinguishable with zero Chat submissions; complete supported schemas/history are retained. A higher byte ceiling alone is not a repair. |
| Tool/result continuation: desktop_responses_server.mjs transport.exchange; bundled turn-broker/turn-execution | Map real Chat MCP calls to native calls; the following function_call_output resolves that exact pending invocation instead of submitting another yRt prompt. Reuse the existing broker; Core alone executes and approves. | Offline two-call/parallel/duplicate/missing-result fixtures first; then one accepted Chat task, two real harmless Core calls and final reply without result-phase user submits. App MCP/Temporary Chat availability is a gate; Work-only local function exports are not proof. |
| Lifetime and explicit stop: server req.signal/SSE close callback, desktop_app_transport.mjs catch/retire | Trace how upstream turn/interrupt reaches the adapter. Separate an expected tool-round boundary and observer loss from actual user interruption using proven signals. Do not merely detach AbortController. | Disconnect does not resubmit or duplicate an action; explicit stop prevents later call admission, interrupts owned Core work and reports confirmed termination or unresolved state. If the wire cannot distinguish stop, resolve that boundary before retaining live Chat turns. |
| Visible progress and completion: adapter queue/SSE, app owned-conversation state, native TUI events | Relay host-visible text and actual Core tool events; distinguish accepted, generating, tool-running, waiting, interrupted, failed and final. Heartbeats/readiness are not work progress. | Fault-injected stream loss while tools run leaves observable progress; restored view shows the latest result once. Do not mark task success from HTTP200, a finished tool or a server counter. Physical TUI observation is required. |
| Restart, ownership and compaction: DesktopTranscript, lane keys, guardian children, DesktopCompaction | Check exact thread/turn/call ownership and completed-result reuse; preserve readable canonical context before route/chat rebinding. Qualify which pending host calls survive each restart; never assume an in-memory promise is durable. | A/B concurrent lanes and guardian remain separate; stale replies fail; completed mutations are not rerun. Long native/Chat transitions and cold resume retain requirements/results. Unrecoverable pending execution is explicit rather than silently replayed. |
| Setup/retry cost: desktopStages, DesktopPromptContext, recordDesktopSubmission; app instruction/history parameters | Evaluate one initial task/context transfer and warm deltas, preserving roles and whole tool schemas. Count setup/work/result/review/retry/compact separately; do not relabel user messages as tool output to claim savings. | Same offline fixture and actual accepted-submit evidence agree; no hidden ACK/retry submits. Reviewer tools remain active. Counters describe submissions, not Chat-limit deductions. |
| Batching and dependency reduction: existing Core tool surface, install_desktop_provider.py and transport selection | Consider CCCC-style batching only through the native advertised tools/approval path and only when intermediate model judgment is unnecessary. After acceptance, package only reachable dependencies and retire superseded transport logic. | Tool capability/approval/cancellation/media semantics remain; compare round trips, latency and output volume without claimed quota savings. No second executor, Node service, workflow daemon or runtime under prj. Keep necessary app/display dependencies until their removal is proved. |

Execution order: offline admission/catalog/opaque-checkpoint repair and continuation/
cancellation fixtures; then the smallest account-bound MCP feasibility proof;
then affected real model switching, long-history/resume, sibling/reviewer and TUI
visibility acceptance; finally remove replaced code/dependencies and publish through
normal release gates. Stop expanding the MCP candidate if app/plugin compatibility
or reliable cancellation is unavailable. Pro inference remains prohibited;
GPT-6 Instant is the first live model. The initial live proof expects one work
submission, with any first-time setup counted and budgeted separately in advance.
Do not immediately duplicate it through the old path: its known two-call flow uses
three work submissions. State the exact trial, submission bound and stop condition
before any real Chat use; later fault/switch/review trials need separate budgets.

## Independent window return and optional tmux

- `codex termux launch` selects an existing profile with Up/Down and Enter;
  Esc/Ctrl-C cancels, the saved default is unchanged, --profile is explicit and
  upstream bytes after -- are preserved. Default is tmux off. Ordinary interactive
  `codex` uses the same optional window routing without a profile picker.
- Native-capable Termux runs in the invoking terminal. Confirmed unsupported native
  API uses one random named dedicated terminal per launch without requiring tmux.
  Inconclusive AM failure never becomes a constructor fallback. Stock manual rename
  or close races may create an inert empty terminal, never restart/resume work.
- Bare foreground FD bindings and tmux pane bindings share notification focus.
  UUID changes use the live FD rather than labels/titles or stored conversations.
  `/switch`, /new, /resume and /title do not change the Android origin.
- Hidden/status requests inside tmux refuse before picker, launch descriptors,
  Android commands or work. Both Manager and AI removed their inside-session
  new-window branch; no nested session, extra pane or replacement workload is
  created. Independent Android terminals can each own a separate container.
  `codex termux attach SESSION` rebinds only detached Manager work without replay.
- tmux and Android starters use private single-consumption bounded descriptors for
  raw argv/caller environment; terminal identity stays new-terminal owned. The
  descriptors are removed before exec. Lazy cleanup requires verified departure.
  Temporary descriptors can contain caller auth environment; permanent bindings,
  Android intents, notifications and logs contain none of it.
- Native Termux extension on the existing authenticated AM socket captures exact
  handle/PID/start from peer ancestry and rechecks the app's live table on focus.
  It stops at the app boundary and introduces no new daemon/socket/registry.
  Installed package com.termux/code118, 0.118.0+8629e63.origin2, base8629e632.
  Existing APK acceptance/build38099833533 remains applicable; app inputs unchanged.
  APK SHA256d32973c555ff9d8d726306948a0e0af1c2b41ccd2938a22edcd848bf7a278453.
  Original and accepted APK remain in Download/termux-origin-candidate.
- User tapped all four fresh Manager return notifications. A/B bare and hidden tmux C with synthetic native workloads recorded exit0, selected=true and count5; D's seeded stock named
  route returned exit0/count5 on the custom APK. D proves named-service reuse,
  not an installation of an official APK. All four owned workloads exited normally;
  user-requested cleanup restored baseline count1 and removed the fake install trees.
  Private proof: ~/.cache/codex/acceptance/window-device-88b0a09.json. This return
  proof remains applicable to8ba5997: the follow-up only removes nested construction;
  fresh launch, binding and focus code is unchanged. Actual Android signed installed
  Manager also refused all three nested entrypoints before any state creation.
- A fresh notification is required for each trial: Android consumes it on one tap.
  Never ask for a second tap on an already consumed notification.
- The owned direct signed-stable46 title trial has no PTY intermediary or copied
  authentication. The user confirmed normal rendering and Termux title propagation;
  upstream persisted the selected activity/app-name/model/run-state/current-dir
  order. `/title` configures title fields/order, separately from route identity.
  A discarded intermediary trial rendered badly; it is not product acceptance.
- Bare same-PID frontend re-entry needs replacement of the old qualified FD
  binding. The affected real-exec path now passes native and named-route return,
  UUID changes and preservation of unknown bindings. Local composed full acceptance
  passed Core193/one pre-existing ignored, Manager42+55+three path gates, builder25,
  Python18+59, fmt/Clippy/diff. Two startup fixtures now clear inherited window-ready
  state, so a connected caller cannot silently bypass the first-launch advisory
  test. Hosted acceptance and signed stable47 installation passed. A fresh real frontend
  trial exposed a separate missing bare-registration call, which blocks its
  `/switch` notification-return acceptance until the frontend is replaced.

## Source and installed AI

- AI main e8d49c3 formally includes the installed profile/auth/Manager adapter and
  migration support; the approved normal push completed, with a clean source tree.
  Enhanced full verification passes50/warn0/fail0, including TUI166 and isolated
  installed Manager profile lifecycle. Migration stages complete accounts before
  publication, restores the source on failure and refuses unreadable live owners;
  unique AGY profile state is preserved. No real account migration was performed.
  All thirteen installed ai_* modules match the published source; only three
  modules needed atomic replacement, and seventeen other runtime/config hashes
  remained unchanged. Private proof: ~/.cache/codex/acceptance/ai-formal-source.json.
- App source remains the separate formal ~/prj/termux repository, with official Git
  history and read-only upstream. AI has no competing Codex profile/window authority.
- AI follow-up1f958e5 removes its AGY shim/installation path; enhanced full verification
  passes48/warn0/fail0. AGY main ac228cf publishes the formerly installed native Android
  support. Its full suite passed and the formal setup installed wrapper261011.01,
  upstream1.2.17, with one $PREFIX/bin/agy entrypoint. The raw input and all9302
  protected profile/authentication files retain their hashes; installed library and
  builder match published source. Native allocator/syscall patterns stay unchanged,
  while explicit Linux compatibility remains available. Default installation does
  not require glibc/proot. Private proof: agy-formal-source.json in the same cache.

## Reusable acceptance

- Exact source6c5819c acceptance38087812283 passed every job, including actual
  Android/AArch64 Core execution and locked workspace tests/Clippy/fmt.
  Local Core192 passed/one pre-existing ignored; Manager40 unit +55 integration
  +two public tmux/native-FD paths passed; release-builder25 and Python18+59 pass.
  Two superseded AI-action assertions were updated before the final Manager gate.
- Scoped d18f796 acceptance38100063268 covers documentation; runtime jobs skipped.
  Its unchanged6c runtime proof is reused rather than described as a fresh build.
- Repository-local development rules are accepted; main is a thin source caller.
  Standard checks are scripts/check.sh docs, python, rust or full, chosen by scope.

## Remaining limits

- The installed native frontend still gates FD registration on TMUX/TMUX_PANE,
  despite Manager supporting bare bindings. Existing synthetic native workload
  path proofs do not cover this real-frontend call. The maintained patch now removes
  that condition, and its native child gate exercises both bare and tmux processes.
  Native compilation/tests/lint, qualification binding, signing/publication and
  fresh physical return across `/switch` are pending; stable47 is not evidence that
  bare real-frontend notification return is complete. `/title` rendering itself
  remains accepted by the user.

- Broader authorized normalization continues separately: genuine official-APK
  fallback execution, independent AGY/OpenCode
  notification hooks, isolated real ChatGPT Web acceptance and the final home/usr
  authority audit. Existing synthetic browser proofs do not establish account,
  model, tool-loop, cancel, compact or resume acceptance. Photos v3 cleanup90b8546
  is published: retired snapshots remain recoverable in Git, active data/source
  bytes are preserved and clean-tree packaging/service-smoke passed.

- Public update currently fetches complete payloads with a300-second per-payload
  budget. Slow large downloads can hit that limit and trigger local-derived
  fallback; the existing same-version frontend rule can then refuse fallback.
  A verified inventory assembled from identical installed bytes and immutable
  Release assets is accepted by the ordinary `codex update --local` path. Efficient
  public acquisition reuse and clearer transport/fallback reporting remain separate
  Core work; do not change signing/admission to work around network speed.
- Existing old tmux work does not infer a detached original Android ancestor.
  New registered launch or explicit detached Manager attach supplies the origin;
  no notification creates replacement work to retrofit old jobs.
- Switching a live bare PTY into/out of tmux is not supported; choose a new safe
  launch. No user job was stopped to change execution mode.
