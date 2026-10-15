# Operating state and next work

SPEC.md owns behavior; this file owns current status and remaining work.
Production authority is [rewrite/rust-core CURRENT.md](https://github.com/humtr/codex/blob/rewrite/rust-core/CURRENT.md).
Historical investigations and superseded plans remain in Git (before3dc817f);
they are not additional active work lists. Preserve negative proof in the private
acceptance cache. No external workflow skill or separate task ledger is required.

## Operating baseline

- Public/device Core and frontend: signed stable52, CLI0.161.0, source e8dc2d7.
  Publication38733863985 and ordinary device activation pass. Fourteen protected
  files and seven process identities survive activation. Stable51 is rollback.
  Native41-test/source/device catalog and permission gates are qualified.
- Installed opt-in Chat provider: d7513cc7785f3d5dbcef, source717bd94,
  `returnTransport=local`,590 verified payload files. Prior561-file payload retained.
  This provider is experimental; Core stable publication does not make it a fully
  qualified public Chat provider. No later source edit is implicitly installed.
- Actual installed signed-app + stable51 Core proof: one Chat user message,
  two real commands/tool-role returns, three model starts; initial/intermediate/
  final reports, canonical persistence and cold Core/TUI restore pass. Fresh
  cold bootstrap also passes. Chat allowance deduction remains unknown.
  First cold-start failure preceded dispatch; its original cause is unconfirmed.
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
HTTP success, a server counter or a cached tool listing. Work in priority order;
reuse unchanged green evidence rather than repeatedly rerunning it.

| Order | Work | Acceptance / current gap |
| --- | --- | --- |
| 1 | Tool identity and dynamic discovery | Diagnose the current unadvertised_tool. Preserve namespace, schemas, freeform and searched tools; exact advertised names must survive search, compaction and continuation. No guessed aliases or weakened admission. |
| 2 | Failure recovery | Distinguish a known completed invalid sample from unknown delivery. No permanent poison from a recoverable validation error, automatic resubmission, repeated mutation or cross-transport fallback. |
| 3 | Complete native development surface | Qualify MCP search/call/result, custom/freeform tools, parallel batches, background terminals, approvals/reviewer and user questions on the installed path. Two shell commands are insufficient. |
| 4 | Long context and switching | Preserve canonical requirements/results through automatic/fault compaction, native-to-Chat/Chat-to-native switching, restart and cold resume. Reject unreadable opaque checkpoints before placeholder conversion; never silently truncate history. |
| 5 | Lifetime and owners | Qualify generation stop, native command stop, observation-only loss/reconnect, app/adapter restarts, concurrent tasks/accounts and guardian isolation. A turn interrupt is distinct from termination of an already-running unified-exec process. |
| 6 | Transcript and visible progress | Relay actual model initial/intermediate/final reports and native tool results once, in order. Verify real TUI visibility/persistence and resumed latest state during injected output/observer loss. Heartbeat is not model progress. |
| 7 | Latency and model-change reload | Time startup/context/dispatch/first-public-output/completion separately; reduce buffering and unnecessary resend. Use upstream session model/effort actions for same-Chat changes; cross-provider handoff remains separately qualified. |
| 8 | Independent MCP return transport | Establish actual new host schema and original-call Core result delivery. Qualify hosted deadlines, long execution/approvals, late results and observation recovery. Preserve local and MCP choices through the same Core/transcript projection. |
| 9 | Models and settings | /model is primary; upstream --profile chatgpt remains shared fallback across account homes. Actual family/mode/effort must match selection; preserve native defaults, account home, permissions, PTY and thread. |
| 10 | Cost and cache | Separate user submissions, tool returns, model starts, HTTP rounds and observed Chat limit changes. Count setup/review/retry/compact too. Reuse warm deltas/known batching without losing judgments or quality; correctness cannot require provider cache retention. |
| 11 | Operating lifecycle | Qualify long-running use, physical reboot, cold bootstrap, install/update/reinstall/rollback, app-version compatibility and explicit remote history retention. Preserve auth, settings, conversations and running user jobs. |
| 12 | Simplify and release | Package only reachable dependencies after proof, retire replaced text/GUI submission paths where safe, keep accurate attribution, concise current docs and one installation authority. Qualify/sign/publish/activate the Chat provider through an explicit final release scope. |

First finish1/2, then3, then4/5. Performance and alternative transport work must
not obscure a broken basic native tool loop. Source/offline implementation and
pushes are authorized. Explain necessity, scope and expected Chat submissions
before live trials or operating provider changes; do not stop existing tasks.

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

Provider suite passes77 Bun tests and41 Python tests; the15 affected local
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
Operating provider activation and a bounded actual Chat trial remain outstanding;
source/offline success is not the user's physical MCP retry acceptance.

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
