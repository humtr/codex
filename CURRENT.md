# Chat provider operating state and technical evidence

Reviewed2026-10-18. The current production [SPEC](https://github.com/humtr/codex/blob/rewrite/rust-core/SPEC.md)
owns behavior and approved targets; the production [completion plan](https://github.com/humtr/codex/blob/rewrite/rust-core/CURRENT.md#product-completion-scope-and-work-order)
is the single ordered product backlog. Maintained provider source now lives in
rewrite/rust-core/providers/chatgpt; this branch retains historical implementation,
unique work and technical proof until final integration. Historical production snapshots
here predate current Core/Manager/TUI and release automation; never copy them over
current rewrite/rust-core. AGENTS.md remains unchanged.

## Current integration work

The active JS/Python runtime, focused tests and licenses have been selectively
integrated into rewrite/rust-core under providers/chatgpt. The approved next target
is an independent Rust Provider with a minimal app-side JS bridge; conversion is
not implemented yet. That branch remains the product/contract authority; do
not merge this experiment's historical Core, Manager, TUI or release snapshots.
Preserve the existing uncommitted1m catalog/profile test changes.

A no-Chat regression now fixes successful-but-uncommitted compaction: transient
summary instructions no longer enter the resident canonical prefix. The previous
source fails the same trial with desktop_context_replaced; the candidate reuses
one conversation and sends only the next real user delta, without tool replay.
This is one confirmed defect, not proof of the cause of every observed compaction.
Content-free context samples distinguish append/replay, changed instructions/tools,
compaction and host-acknowledged new-conversation counts. These are separate from
Chat allowance deductions. Candidate tests:105 JS and45 Python pass; no real Chat,
operating runtime change or device activation. Private proof: context-prefix-negative.log,
context-local-regression.log and context-provider-all-{js,python}.log.

## Operating baseline

- Public/user Full60/source54f9264 pins Base58 on qualified upstream0.163.0.
  Previous user Full57 remains rollback. Native TUI1c413cf and Manager quota work
  are production-owned, not implementations to replace from this branch.
- Provider immutable code payload45072df8448589d3523e/source7bb202e uses
  returnTransport=local. Full updates do not update or qualify this experiment.
 527 package files,25 live-qualified execution inputs, private runtime binding and
  prior payloads remain separate from mutable generated models/profile settings.
- The user changed the generated5-model catalog to context_window and
  max_context_window=1,000,000, with auto_compact_token_limit=null. The two existing
  uncommitted profile generator/test edits make the same32,768→1m change. Preserve
  both source edits, user config/auth and running jobs. Their intended1m values
  are also preserved in the integrated source; neither source port nor this
  documentation revision activates a provider payload.
-1m is a local scheduling/context override, not demonstrated host capacity. No
  current1m/project/Full60 long-development qualification is established by prior
 32k/stable53/54 trials. Current source tests hardcode this value; the product
  target replaces source editing with one provider-owned validated config.
- Actual signed-app primary-entry discovery works across Home/Plugins/Chat without
  UI navigation; app identity, host auth and local-function ownership are retained.
  Latest baseline source checks103 Bun/45 provider Python tests, package imports/
  entry builds/HTTP admission. Retained actual stable54 proof has2 user-message
  objects (one hidden context),2 tool returns,3 model starts,0 Pro/retry and native
  reports persisted/restored/rendered. No new live Chat was sent for this review.

## Active remaining work

The production completion plan owns the complete execution order and exit
criteria, including Core isolation before Rust Provider migration, the subsequent
features, integration closure and release. Do not maintain a competing ordered
roadmap here. TUI is an independent upstream-based frontend, not Manager's
private UI. The provider remains usable without Manager or enhanced TUI.

This branch contributes its active modules, dependencies/licenses and meaningful
regression drivers, not old Core/Manager/shared-profile/UI/workflow snapshots.
Original source history and all unique work are preserved. Branch closure follows
migration of requirements and accepted exact-input proof, not merely a merge.
No second roadmap or extra plan/document hierarchy is introduced here.

### Current operating blocker: native-to-Chat long conversation

On2026-10-17 two long-thread transitions to GPT-6 (Extra High, then High) onFull60
fail400 unsupported_desktop_route before Chat submission. Native logs show
run_pre_sampling_compact/ModelDownshift/PreTurn with previous model gpt-6.1-sol
through the newly selected Chat endpoint. Exact upstream0.163/source8308618 retains
that previous model in maybe_run_previous_model_inline_compact. The local adapter
rejects it:2 arrivals,2 admission failures,0 admitted requests/user submissions.

The persisted transition snapshot also contains an opaque native checkpoint, not
an ocx1-readable summary; its metadata is a turn reference, not readable context.
That is stored-state evidence, not a capture of the failed request. Actual HTTP
regression separately proves previous-model rejection and opaque-context rejection
after model-name substitution. Do not remove the guard or invent/drop history.
A fresh Chat avoids this specific previous-model condition; its Full60 inference
was not requalified by diagnosis. Raising the context declaration is not proof of
readable cross-provider handoff or a permanent repair.

Private proofs: chat-full60-operating-diagnosis.json verifies signed Full60/all8
payloads and527 provider files; chat-full60-cross-provider-admission.json passes
17 actual HTTP admissions/rejections with0 sampling; chat-full60-diagnosis-python.log
passes45 Python tests. Runtime, profile and authentication remained unchanged.

### Configuration, context and project prerequisites

Approved single-file target: $XDG_CONFIG_HOME/codex/providers/chatgpt.toml (default
~/.config/codex/providers/chatgpt.toml), with effective/show/validate/edit/set and
optional Manager/TUI views using the same provider validator. Commands/schema are
not implemented. Preserve1m explicit override; distinguish context tokens,
automatic-compaction thresholds, encoded/decoded HTTP limits and host capacity.
Generated profiles/catalogs are outputs. Catalog changes must use a matching
qualified backend rather than mutate a live sibling or immutable code.

Project mode creates newly owned chats inside the configured remote ChatGPT
project, creating the project if absent. Current local transport uses retained
conversations with no project binding; older text transport requests temporary
chats. Neither proves project creation, selection or temporary+project behavior.
First establish app-owned operation/identity shape read-only, then no-inference
create/reuse/membership readback and uncertainty/concurrency handling, then one
scoped model/tool proof. Bind account/workspace/exact project, resolve duplicate
names explicitly and refuse silent root-level fallback. Existing chats are not
moved/deleted. Project instructions/memory affect behavior and reviewer isolation;
no global personalization/sharing changes are implied. Official user-facing
capability does not establish an API for this adapter:
[projects](https://learn.chatgpt.com/docs/projects?surface=app),
[selection](https://learn.chatgpt.com/docs/reference/slash-commands).

### Required provider surfaces and gaps

| Surface | Retained evidence / remaining requirement |
| --- | --- |
| Tool identity/search | Canonical wire names, digests-only diagnostics and old-round filtering are installed. Actual historical unadvertised_tool cause was not recorded; do not assert a Tdev authentication cause. Native search/discovered MCP paths pass deterministic Core tests. |
| Invalid arguments/recovery | Nonempty malformed JSON/array/whitespace reaches Core as exact raw arguments; its real error returns on original call. Local correction uses no new user prompt. Unknown delivery, partial publication, changed IDs/results and failed cleanup remain blocked. |
| Complete native tools | Advertised JSON/freeform/search, batch workers/polls, driver-answered question and reviewer pass bounded historical live/offline surface trials. Physical question UI and broader failure combinations remain. |
| Context/compaction | Historical manual/automatic/interrupted repair→summary→continuation proofs exist. Cross-provider opaque state,1m behavior, mid-turn combinations and hours-long exact-input work remain open. |
| Owners/lifetime | Multiple native threads/guardian use separate lanes; short app/adapter replacement and observation-only reconnect have proof. Real-host observer loss, full restart/reboot and long work require current-input acceptance. |
| Native transcript | Public initial/intermediate/final reports and native results persist/restore/render. Heartbeat is not model progress. MemoryCite/unsupported boxes need public text handling without HTML execution or fabricated private reasoning. |
| Latency/cost/cache | Warm resident deltas and bounded phase timings are implemented. Model starts, user objects, tools, setup, retries, summaries and actual limit deductions remain distinct; no Codex-quota comparison proves Chat message debits. |
| Independent MCP | Own broker/stdio/catalog/original-call projection passes offline; actual host reaches catalog then confirm_action before Core execution. Exact connector/call-bound acknowledgment, deadlines, long tools/questions/cancellation/late results remain unqualified. |
| Model configuration | /model primary, --profile chatgpt fallback. Same-Chat model/effort keeps an owned lane and native process. Native↔Chat re-entry and hardcoded production TUI family/effort checks need generic configuration and readable handoff. GPT-6 first;5.6 deferred; no automatic Pro. |
| Remote history | Retained Chat is the installed local transport behavior. Project placement/settings, account-scoped mapping and explicit archive/delete/temporary choices remain implementation/acceptance work. No user chat deletion. |
| Operating packaging | Immutable527-file package/reinstall/rollback/six bootstrap clients pass historical0-Chat tests. Current stable optional payload inventory/compatibility/authenticated delivery/rollback still needs integration; importing source is not admission. |

## Retained exact evidence and limitations

- continuity-qualification.json, development-qualification.json,
  batch-qualification.json and surface-qualification.json record bounded trials.
  Private cache ~/.cache/codex/acceptance retains raw traces/negative results.
- GPT-6 Instant native surface/batch trial: background workers overlap, native
  commands/questions/reviewer/reports/cold restore pass;8 model starts,6 tool
  returns,2 user objects,0 Pro/retry. This is not physical question acceptance.
- Development121.865s: actual baseline failures, repair/tests/holdout, manual
  checkpoint with tool-only token, unchanged completed source on continuation,
  cold Core inference;4 users,8 returns,12 starts,1 summary,0 Pro/retry.
- Automatic86.641s:5 user objects,6 returns,11 starts,1 summary/checkpoint,
  no Pro/retry. It predates later compaction-cancel/warm-resend changes.
- Autonomous copied-ai272.170s: independent issue selection, code fixes and full
  verification;1 user,19 returns,20 starts. Original tracked45 files unchanged.
  Raw overall failure was an exact-report-count observer; real multiple reports
  are retained. The driver hashed inputs after the run, so this is useful behavior
  evidence, not exact-byte release admission.
- Earlier source recovery fixtures: chat-invalid-input-{core,array-core,
  whitespace-core}-next and chat-tool-next-{baseline,rejected,discovery}.json;
  deterministic Core+real HTTP/app-port, actual tools and cold rendered reports,
 0 real sampling. The native question driver does not represent a user tap.
- Current installed source package/activation evidence: chat-runtime-package-*,
  chat-context-runtime-install-cycle, chat-primary-entry-*,
  chat-mcp-lazy-core-live-1 and continuity-qualification's input hashes. Installed
  local-route proof is separate from failed live independent-MCP confirmation.
- Earlier Instant intervals30/19/5/15s measured user-to-final, not network TTFB.
  Request timing now distinguishes admission/readiness/dispatch/public output/
  settlement; do not substitute heartbeat for useful progress.
- Settings/project live tests still need necessity and expected sampling stated
  before running. No automatic Pro, quota-drain, speculative replay or host-policy
  bypass. GUI/browser/CAPTCHA, SIWC/public Responses and media remain separate
  optional research; no new inference is justified by stale trial instructions.

## Qualification and source integration

Use README's scoped Bun/Python tests and exact source/package HTTP/build admission
first, then actual current signed Core/Manager/TUI with deterministic host for
native paths. Any fixture copying installed generations uses matching production
validators via CODEX_CHAT_CORE_PROOF_SOURCE; this is a test input, not a runtime
dependency on a development checkout. Preserve auth, user data and unrelated jobs.

Port/rewrite only the reachable adapter slice after the approved boundary work;
maintain dependency source/licenses and accurate attribution. Ship no historical
trial drivers/reports/GUI setup just because they sit beside runtime code. Keep
Base independent, Full paired and provider opt-in. Every final release requires
exact-input acceptance, immutable signed readback, disposable update/rollback and
non-forced promotion before a separate bounded user-device activation.
