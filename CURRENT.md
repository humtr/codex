# Operating state and next work

SPEC.md owns behavior; this file owns current status and remaining work.
Production authority is [rewrite/rust-core CURRENT.md](https://github.com/humtr/codex/blob/rewrite/rust-core/CURRENT.md).
Historical investigations and superseded plans remain in Git (before3dc817f);
they are not additional active work lists. Preserve negative proof in the private
acceptance cache. No external workflow skill or separate task ledger is required.

## Operating baseline

- Public/device Core and frontend: signed stable53, CLI0.163.0, source36bfeed.
  Release38904810656 and source/native/device qualification pass on the production
  branch. Actual version and Core/Manager doctor confirm device selection; stable52
  is retained for rollback. Production upgrade proof and Chat inference acceptance
  remain separate; the new narrow stable53 live proof is described below.
- Installed opt-in Chat provider:9dd2881029f69124a63a, source0ee0f59,
  `returnTransport=local`,595 installed files verified including its manifest.
  Prior9014deaf810ff40a583d and older recovery payloads are retained.
  This provider is experimental; Core stable publication does not make it a fully
  qualified public Chat provider. No later source edit is implicitly installed.
- Earlier installed signed-app + stable51 Core proof: one Chat user message,
  two real commands/tool-role returns, three model starts; initial/intermediate/
  final reports, canonical persistence and cold Core/TUI restore pass. Fresh
  cold bootstrap also passes. Chat allowance deduction remains unknown.
  First cold-start failure preceded dispatch; its original cause is unconfirmed.
- Retained9014deaf signed-app + stable52 Core proof passes actual tool search, exact
  underscore-prefixed discovered MCP tool admission, automatic approval review,
  one real isolated stdio MCP read, original-call result return, final marker,
  all three report phases and cold Core/TUI restore. GPT-6 Instant,26.071s total,
  observed2 user submissions (primary + guardian),2 tool returns,4 model starts,
  no Pro/retry. This is the local return transport calling a native MCP tool,
  not qualification of the separate remote MCP return transport. Runtime endpoint/
  capability and13 base auth/config/state files are preserved, as are seven
  existing process identities. All590 payload hashes verify. Trial start budget
  was removed; the ordinary installed provider is ready. Proofs:
  chat-owned-mcp-installed-live.json and chat-tool-recovery-operating-final.json.
- Installed9dd288 + actual signed-app + stable53 Core passes the live malformed-
  argument recovery trial in33.035s on GPT-6 Instant (gpt-6/thinkingEffort=null).
  One raw malformed function call and its correlated actual Core error persist;
  the model corrects it, executes exactly one write/one read and completes. Initial,
  intermediate and final reports render in actual native TUI and survive cold Core
  restart without another model request. Observed1 user message,3 tool-role returns,
  4 model starts,0 reviewer/Pro/retry. This is not a Chat-limit-debit measurement.
  Endpoint/capability and12 existing auth/config files plus13 existing process
  identities are unchanged. The bounded isolated adapter used installed product
  bytes and is stopped/removed; ordinary operating bootstrap is ready without
  a trial budget. No existing task was stopped. Private evidence directory:
  chat-tool-recovery-operating-next/{live-result,live-adapter-proof,operating-final}.json.
- Ordinary GPT-6 Instant/Medium/High/Extra High tool loops passed on the earlier
  internal transport. This is useful historical proof, not blanket qualification
  of the newly installed local-return lifecycle, MCP or long-context combinations.
- Current native execution owner is Core; Manager owns account/window convenience.
  Chat authentication remains in the signed app, independent of Manager profiles.
  App/display/renderer/Scope/CDP remain real dependencies. No extracted account
  token, second executor, transcript database or extra persistent service is added.
- Some Responses parser/SSE/protocol helpers reuse pinned miuuyy6.1.5/source92a356f.
  Repository-owned transport/lifecycle integration is independent code. Preserve
  dependency licenses and accurate attribution; do not claim a wholly original
  Responses implementation. Historical connector display names are not hardcoded
  requirements. New independent MCP tools are native_catalog/native_tool.

## Active remaining work

This is the complete active backlog for ChatGPT as a development model in local
Codex. Completion means actual native execution and user-visible behavior, not
HTTP success, a server counter or a cached tool listing. Responsibility realignment
and the0.163 production upgrade are complete on rewrite/rust-core. The user's next-work instruction now authorizes offline source
work on tool input/admission and recovery, followed by native development coverage.

| Order | Work | Acceptance / current gap |
| --- | --- | --- |
| 1 | Tool identity and dynamic discovery | Exact names/search/freeform admission retained; actual0.163 offline search/discovered MCP path passes. New ordinary-function raw-input recovery is source/offline-qualified below; installed/live coverage of the complete tool contract remains. |
| 2 | Failure recovery | Installed local transport passes actual signed-app/stable53 malformed JSON→Core error→model correction with one user message. Arrays/whitespace and other recovery cases pass offline; further live error surfaces remain. Completed pre-dispatch rejections require genuine new input; unknown delivery stays blocked without work replay. |
| 3 | Complete native development surface | Actual installed trial passes MCP/search, real guardian, freeform patch, background polling, native question RPC and reports/restore. Live calls serialized and workers did not overlap; overall FAIL. Offline same-sampling batches execute concurrently. Parallel host/request controls and an explicit batching fallback need investigation; physical question UI remains unqualified. |
| 4 | Long context and switching | Preserve canonical requirements/results through automatic/fault compaction, native-to-Chat/Chat-to-native switching, restart and cold resume. Reject unreadable opaque checkpoints before placeholder conversion; never silently truncate history. |
| 5 | Lifetime and owners | Qualify generation stop, native command stop, observation-only loss/reconnect, app/adapter restarts, concurrent tasks/accounts and guardian isolation. A turn interrupt is distinct from termination of an already-running unified-exec process. |
| 6 | Transcript and visible progress | Relay actual model initial/intermediate/final reports and native tool results once, in order. Verify real TUI visibility/persistence and resumed latest state during injected output/observer loss. Heartbeat is not model progress. |
| 7 | Latency and model-change reload | Time startup/context/dispatch/first-public-output/completion separately; reduce buffering and unnecessary resend. Use upstream session model/effort actions for same-Chat changes; cross-provider handoff remains separately qualified. |
| 8 | Independent MCP return transport | Establish actual new host schema and original-call Core result delivery. Qualify hosted deadlines, long execution/approvals, late results and observation recovery. Preserve local and MCP choices through the same Core/transcript projection. |
| 9 | Models and settings | /model is primary; upstream --profile chatgpt remains shared fallback across account homes. Actual family/mode/effort must match selection; preserve native defaults, account home, permissions, PTY and thread. |
| 10 | Cost and cache | Separate user submissions, tool returns, model starts, HTTP rounds and observed Chat limit changes. Count setup/review/retry/compact too. Reuse warm deltas/known batching without losing judgments or quality; correctness cannot require provider cache retention. |
| 11 | Operating lifecycle | Qualify long-running use, physical reboot, cold bootstrap, install/update/reinstall/rollback, app-version compatibility and explicit remote history retention. Preserve auth, settings, conversations and running user jobs. |
| 12 | Simplify and release | Package only reachable dependencies after proof, retire replaced text/GUI submission paths where safe, keep accurate attribution, concise current docs and one installation authority. Qualify/sign/publish/activate the Chat provider through an explicit final release scope. |

After responsibility realignment, first finish1/2, then3, then4/5. Performance
and alternative transport work must not obscure a broken basic native tool loop.
Source/offline implementation and pushes are authorized by the current next-work
instruction. Operating provider activation and live trials remain separate.
Explain necessity, scope and expected Chat submissions before live trials or
operating provider changes; do not stop existing tasks.

### Deferred functional improvements and current evaluation

- Same-Chat /model changes: exact no-op does nothing; model/effort-only selection
  uses native session updates, retaining the UI and compatible owned Chat context.
  Separate real provider/account changes from these updates. Current frontend
  predicate and provider route-change reset both require correction.
- Context projection: remove redundant prior-assistant replay and the user-visible
  JSON context envelope. Preserve roles and original instruction authority. Consider
  minimal bootstrap plus instruction/tool discovery over existing result channels,
  without optionalizing mandatory guidance, narrowing native capabilities or adding
  a second executor. Measure model starts and input size, not just visual hiding.
- Remote history: design explicit retained/grouped, temporary and cleanup choices.
  Preserve the user's ability to inspect Codex work in the ChatGPT app; grouping
  alone is not a storage bound. Verify actual temporary-chat personalization and
  tool behavior before offering it. Existing close uses cancellation eHt, not
  deletion; isTemporaryChat is currently false. No existing remote chat deletion
  is authorized by this plan. Provider owns transport/retention; Manager owns UX.
- Approval defaults: native ~/.codex has auto_review, but an observed managed home has
  no reviewer default; an observed frontend explicitly carries reviewer=user.
  Preserve no-sandbox capability independently of the reviewer. Align selected
  common/account defaults and fresh/resume/switch/model transitions in the proper
  policy owner rather than forcing auto_review from Core.
- Known and additional tool errors: audit encoding, schema/search/freeform/batch
  admission, delayed callbacks and failure recovery. Use reproducible negative
  cases and actual native-path regression tests. Unexecuted input errors should
  be correctable by the model where safe; uncertain dispatch must never replay a
  mutation. Do not silently repair arbitrary JSON or pretend an invalid call ran.
- Long development is a required product gate. Evaluate an isolated real repository
  with autonomous issue selection, justified changes, failure/repair/retest loops,
  complete regression checks, context pressure, interruption/resume and accurate
  progress. Diagnose transport failures separately from model judgment. No source
  writes, live account changes or publication in the original target repository.

Current installed9014deaf/sourcee27ab09 evaluation evidence:

- GPT-6 Extra High simple coding passes in61.815s: four real commands plus one
  patch, initial2/3 test failures corrected to3/3 passes, syntax check succeeds,
  test source unchanged. Observed signed-app control is gpt-6-thinking/max;
  one user submission, five tool returns, six model starts. This narrow result
  does not qualify autonomous repository work or actual guardian invocation.
  Proof: chat-xhigh-code-vmz8xwtg/report.json in the private acceptance cache.
- Autonomous improvement of an exact tracked copy of ai1f958e5 fails after39.548s.
  Environment-only Unix path overflow was corrected before the trial; baseline
  full verification then passes48/warn0/fail0. The model completes two exploratory
  commands and reports intent, but no edit or regression loop. The next call
  passes exact tool-name admission and fails parsing a non-freeform input string
  as JSON at desktop_tool_session.mjs:22. The adapter aborts the turn before
  publishing that call; the exact malformed argument is not retained. This is
  the confirmed boundary, not proof of a particular quoting mistake or poor model
  development judgment. One user submission, two returns, three model starts;
  no guardian/compaction/retry/Pro. All50 protected source/config hashes remain
  unchanged, no code diff or remaining owned process, temporary roots removed.
  Proof: chat-ai-long-8k18zos4/report.json in the private acceptance cache.

These earlier observations remain the operating-provider baseline. The source
repair below follows completed responsibility work; it has not replaced the
installed provider or repeated real Chat inference.

### Current source repair and offline qualification

Ordinary advertised JSON tools retain malformed/non-object nonempty input as raw
Responses function arguments. Core parses them, creates the actual correlated
error, and remains the sole executor. The next tool-role app message returns that
error on the original call ID; a new corrected call follows in the same native
turn. No guessed JSON, adapter-created tool result, new Chat user prompt or automatic
command replay is added. The owned history shim gives the unchanged dependency
parser an object-only validation view, then restores exact rejected arguments in
ordered context. Native wire, results, names/namespaces and pairing stay intact;
the reused parser/SSE dependency and production Core require no modifications.

A completed call ID reused with different content now refuses instead of silently
ignoring it. Normally completed pre-dispatch input/search/gateway rejections allow
new-input recovery only after successful cleanup; missing/duplicate/changed results,
partial publication and ambiguous delivery still cannot trigger automatic replay.
Empty ordinary JSON strings and tool-search validation remain explicit rejections;
custom/freeform source remains byte-exact. MCP-return malformed input is not newly
qualified by this local-transport change.

Actual Android stable53 Core + real HTTP/Responses adapter + deterministic app
port passes three separate malformed JSON/array/whitespace cases. Each returns the
actual Core error on the original ID, preserves raw history, runs exactly two
corrected shell commands, renders initial/intermediate/final reports and restores
those reports after cold Core restart without another model request. Each case has
one synthetic user submission, three tool returns and four synthetic dispatches;
actual Chat submissions=0. Baseline, rejected-sample/new-input recovery and native
search/discovered MCP execution also pass on0.163 with persisted/restored/rendered
reports. They use temporary roots removed by the probes, not real user repositories.
Test-only CODEX_CHAT_CORE_PROOF_SOURCE selects the matching production validators
for the installed runtime; it is not a provider launch/runtime dependency.

Checks:46 Node tests,83 Bun tests,42 provider Python tests and source Python18+59
pass. Native evidence in the private acceptance cache: chat-invalid-input-core-next,
chat-invalid-input-array-core-next, chat-invalid-input-whitespace-core-next and
chat-tool-next-{baseline,rejected,discovery}.json. The first malformed-input probe
correctly failed report-count acceptance because its fixture emitted two identical
intermediate markers; the corrected fixture emits one and all gates pass. This was
an observer/fixture defect, not accepted product proof.

The repair is now installed as9dd288/source0ee0f59 and its actual signed-app/
stable53 error/correction path passes as described above. The proof explicitly
counts persisted malformed calls and matching Core errors before accepting the
successful corrected commands. The new test driver does not change installed
product bytes. Provider Python42 tests pass after that driver addition. No Pro,
automatic generation retry, native config/auth rewrite or remote chat deletion was
performed. Next is full native development-surface qualification (MCP, custom/
freeform, parallel/background tools, reviewer and user input), then long-development
and context/lifecycle evaluation. Same-Chat reload/performance follows correctness.

### Native development-surface trial: partial live pass, overall FAIL

The user requested tests and retained traces. Reusable desktop_surface_probe.py
uses actual signed stable53 Core and either a deterministic app port or installed
9dd288's signed-app transport. Only disposable directories are modified. Native
execution, approvals and canonical storage remain Core-owned; the probe adds no
executor or operating service. It supplies only one predeclared KEEP answer to the
owned native question RPC, records resolved-request identity and verifies its exact
result in Core history. This proves the RPC path, not a physical user tapping UI.

Actual GPT-6 Instant trial:68.173s,1 primary user message +1 real guardian message,
8 tool-role returns,10 model starts,0 retry/Pro. MCP discovery/read, automatic
review, exact freeform patch, two background starts/polls/completions, native
question/answer, initial/intermediate/final reports, persistence, cold report
restoration and native TUI rendering pass. Neither restore nor rendering requests
new inference. The test stops/removes only its owned adapter/Core/worker roots.
Installed payload595 files and12 existing auth/config files remain unchanged.
Observed submissions are not measured Chat-limit deductions.

Parallel acceptance fails: every actual sampling response offers only one call;
worker starts are6.964s apart, exceeding their five-second lifetimes, with no
overlap. The deterministic trial emits two calls per sampling response and both
workers overlap (start gap0.004s). Thus adapter/Core batch execution is established,
while actual Chat generation of a batch is not. One serial live sample cannot
establish an immutable host limitation or distinguish request defaults from model
behavior. No automatic retry or second live sample was performed to force a green.

Evidence: committed surface-qualification.json contains the feature matrix,
observed counters, test-source hashes and private trace checksums. Private evidence
is ~/.cache/codex/acceptance/chat-surface-live-1/{result,adapter-proof,native-tools,
native-items,surface-batches,public-trace,progress,preservation}.json (progress is
JSONL; actual rendered transcript is tui.txt). Successful deterministic proof is
chat-surface-offline-4. First two offline failures remain: login-shell profile
initialization attempted service reconciliation in an isolated home and delayed
workers. Explicit login=false fixes this fixture; no production shell/default is
changed. A generated live-wrapper argv/output-root problem was caught and fixed
before inference; chat-surface-entry-proof.json executes that exact setup with a
fake port, never a model. Full wrapper/fixture source is committed for reproduction.

Next: inspect actual app request/response parallel controls without inference.
If multicall generation is not reliable, qualify an explicitly declared batch in
the local gateway: one original host call maps to stable per-tool Core call IDs,
individual native schema/approval/execution and one aggregate original-call return.
Never invent multiple completed actions from a single call or add an executor.
Requalify ordering, mixed errors, cancellation, exact-once returns and persisted
transcript before a new bounded live batch proof. Keep long-development/context/
lifetime gates unfinished; do not claim the full surface is qualified.

### Deferred and conditional scope

- GPT-6 first. GPT-5.6 remains deferred after earlier continuation failures.
- Observed Pro selections may be listed for the user's work; never spend Pro
  inference on development/qualification. Preserve host entitlement gates.
- Images and other non-text input need separate capability/admission proof.
  Current text-only and opaque-context limits must be explicit, not hidden loss.
- Phone remote-view width/input/scroll and a same-Chat diagnostic viewer are lower
  priority. Detailed provider-wait/observation-age UI is conditional on a proved
  native display gap; no second progress registry or fabricated commentary.
- Shared cache across native Codex and Chat is not established. Byte equality,
  estimated token counts and reported zero cache usage do not prove cache status
  or Chat allowance savings. Supported caching must remain optional.
- Media, hours-long load and physical reboot remain unqualified. A successful
  manual compact or old GUI trial does not qualify those combinations.

### Separate production maintenance

The production branch owns upstream0.162+ qualification, efficient signed update
acquisition/reuse and clearer fallback reporting, plus existing title/history
integrity doctor warnings. Keep these independent from Chat model acceptance.
Broader Termux normalization, AGY/OpenCode hooks and other projects retain their
own owners; do not reintroduce Tdev changes as the solution to this provider task.

## Current user-trial diagnosis

- MCP-list task: native shell succeeds; dynamic tool discovery succeeds; then
  desktop_tool_session.mjs:17 rejects a returned name as unadvertised_tool before
  Core/Tdev admission. Latest search loaded eight tools. Offline parsing of the
  same namespace/name shapes succeeds with exact wire names announced. Actual
  rejected name/live catalog were not recorded, so do not assert a confirmed
  Tdev auth/network failure or one confirmed name-mismatch cause.
- The installed source exposes both bare name and wireName in model catalog data.
  The candidate now exposes one exact canonical name while preserving full Core
  schemas. It filters old message IDs and old-round decoded/completion/error/ACK
  callbacks. A negative control against the unchanged installed source reproduces
  stale message/completion admission and a poisoned retry; the candidate rejects
  those stale observations and recovers after new input. These confirmed source
  defects still do not establish which caused the user's particular rejected name.
- The installed source marks every post-submit error delivery-unknown. The
  candidate permits recovery only after normal host terminal, zero published
  native calls and successful owned cleanup; classification waits for the actual
  terminal even if an invalid message arrives first. Unchanged requests/replaced history
  refuse until new user input preserves the canonical prefix. Unknown delivery
  and failed cleanup stay blocked. Calls wait for terminal and whole-batch
  validation, so a later invalid call cannot publish earlier partial work.
  Rejected-name/catalog diagnostics contain only digests, counts and known shape
  classes; no raw proposed name, arguments or report is stored.
- Observed Instant reply intervals were about30/19/5/15 seconds. They are persisted
  user-to-final intervals, not network first-byte times. Context was about68k
  characters; token usage is estimated. Startup, context ingestion, provider queue,
  generation and final-phase buffering have not been individually measured.
- Private diagnosis: chat-local-unadvertised-tool-diagnosis.json and
  chat-instant-latency-observation.json. Investigation submitted no extra Chat.

## Current source qualification

Provider suite passes77 Bun tests and42 Python tests; the15 affected local
transport tests cover pre-submit recovery failure and a message arriving before
its terminal, in addition to the initial correction.
An initial unscoped Bun collection also picked the dependency-only TypeScript
fixture and failed its relative imports. The documented provider scope is now
explicit *.test.mjs; the proper provider gate passes. Do not call the first gate green.

Actual signed stable52 Core with the deterministic app port passes the ordinary
two-command/report/cold-TUI path. The added offline-only --reject-tool gate passes
one real write, completed invalid sample refusal, no automatic resubmit, explicit
new user recovery, one real read, unchanged inode/mtime/size, exactly two persisted
commands and initial/intermediate/final cold-TUI restore. No Chat inference is
submitted by these fixtures. Counters distinguish four total fixture dispatches
from two on the replacement lane. The fixture flag refuses live-app options.
Proofs: chat-tool-recovery-{core,core-rejection,core-terminal,negative-control}.json
and chat-tool-recovery-{bun-terminal,python-final,local-final}.log in the private cache.
Unchanged Rust/frontend production gates remain reusable; no native inputs changed.
Immutable9014deaf810ff40a583d/sourcee27ab09 is now activated and live-qualified
as recorded in Operating baseline. Aggregate source/offline proof remains
chat-tool-recovery-qualified.json; activation/actual model proof is separate.
The user's original rejected name was never recorded, so the exact original cause
and physical retry of that private conversation are not retroactively proved.
Broader tool/long-context/lifecycle work remains open.

The new --mcp-discovery fixture uses actual signed Core and a private read-only
stdio server. Search loads one exact namespace tool, auto_review stays active,
and a deterministic guardian reply tests plumbing rather than model judgment.
The first two offline attempts failed because this new fixture lacked guardian
support; negative results are retained and no Chat was used. After adding the
structured guardian port, the complete offline path passes. Its42 Python tests
include the marker server's discovery/read/no-modification test. This fixture flag
refuses live-app options; actual installed HTTP validation uses the same native
Core scenario separately, with the real model and a bounded startup budget.
No user Tdev service, account home or conversation is used as fixture state.

## Evidence and limits to retain

- stable52-native-device-qualified.json binds41 actual native TUI tests, Android
  legacy/named/--profile menu round trips and permission lifecycle. These submit
  zero inference. stable52-device-activation.json verifies signed installed files,
  protected state/processes, healthy Core/Manager doctor and repeat update no-op.
- chat-local-installed-runtime.json and chat-local-installed-cold-{bootstrap,runtime}.json
  qualify the installed local two-command/report/persistence path. The first
  failed cold start is retained as chat-local-installed-runtime-first-failure.json.
- chat-local-stop-resume-live.json qualifies one local execution interruption,
  exact-thread background-terminal termination, mixed result/new-user continuation
  and no mutation replay. Ordinary Ctrl-C has not been changed to kill every job.
- chat-independent-mcp-core-qualified.json uses actual signed Core and independent
  stdio/broker with a deterministic host, not live Chat. Actual host still returns
  six old connector definitions from cache and sends no new tools/list wire request.
  chat-independent-mcp-discovery.json is negative proof. Refresh/binding needs
  positive app listing plus the actual new endpoint response before live settlement.
- Hosted MCP source preserves original invocation/result identities independent
  of reused JSON-RPC IDs. A host timeout does not prove native work stopped or a
  late result was received. Pending promises are not durable across adapter death.
- Core local commands/background terminals remain the only executor. Observer
  loss must not submit Continue, create a conversation or replay work by itself.
- Existing genuine initial/intermediate/final report proof is narrow. Do not
  project private reasoning, invent assistant prose from heartbeat, or call
  partial source/offline fixtures complete phone/display acceptance.
