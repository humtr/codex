# Handoff — one development route and active acceptance plan

Updated: **2026-11-13 KST**. This is the **single current execution
backlog and dependency order**, not a second SPEC, source of observed facts
or authorization to modify an operating release.

Read [AGENTS](AGENTS.md), [documentation policy](docs/DOCUMENTATION.md),
[CURRENT](CURRENT.md), [SPEC](SPEC.md), then only affected scoped annexes.
Historic W0-W20 provenance and plan-vs-outcome accounting:
[finite integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).

## Decision: one continuing development line, staged acceptance

- **Development authority:** `rewrite/rust-core`. `main` is the distinct
  formal signing/publication/approval line. Temporary owned worktrees and
  verification branches are permitted; no feature/document may remain only
  on a long-lived competing development branch.
- **Current installed release:** Full S167, Codex 0.167.2; the selected,
  supervised and running Provider identities must be compared separately.
  Do not modify signed generations, current/rollback, the runit environment,
  accounts, service processes, original Native Writers, user records or sdcard
  in these documentation/acceptance phases.
- **Scope evolution:** The original Pro assignment called for immediate
  W0-W20 implementation and exact-source CI. Most source work was integrated
  and a 0.167.2 Full installed by subsequent authorized operations. The
  **latest user instruction supersedes the *order***: (1) prove the real
  A/O/W return chain, (2) test the installed 0.167.2 product and differentiate
  proof levels, (3) document findings and integrate user-contributed task
  designs on this branch, (4) only then implement discovered defects,
  runit automation and major CI redesign in a distinct authorized stage.
  It does **not** waive the original product contracts or conceal incomplete
  live acceptance.
- **Do not wait** for the retired Pro chat or requeue it; independent session
  creation is allowed within the stated test authority. Old Pro/G1/G2/G3
  original receipts, worktrees and active/uncertain effects remain protected.
  Do not infer a durable job from a single model turn.
- **Status vocabulary:** PASS (actual evidence), FAIL (actual failed
  execution), BLOCKED (no safe authority/environment), UNKNOWN (ambiguous
  dispatch/result), NOT_TESTED, NOT_IMPLEMENTED. Keep per-SHA and per-device
  evidence distinct. No blanket percentage is a release acceptance.

The original technical scope is W0-W20; new requirements receive named backlog
IDs linked to their owning W task, and become new W IDs only when a truly
independent owner and acceptance are required. The user has not authorized a
second executor, owner DB, model transcript store or privileged bypass.

## Stage V0 — reconfirm immutable baseline and original owners

Before every effect, reconcile latest exact remote HEAD, installed Core and
Provider generation/digests, operating runit process executable, tunnel version,
active worktrees and Native receipts. Saved Chats may be read without submitting
inference; match conversation IDs to account and project. Compare any new
facts to [CURRENT](CURRENT.md), never trust an inherited estimate.

Starting evidence for this cycle: development `0ba05892`, `main`
`fa6a185`, S167 Full, Provider runit PID 21766, tunnel service running.
No new work is authorized to stop these services or claim another Writer.
The original integration worktree retains dirty documentation; selectively
compare it before discarding any unique facts.

## Stage V1 — real assistant -> orchestrator -> worker -> orchestrator -> assistant

This is the first **new runtime acceptance** gate, not a roleplay and not
fulfilled by historical W1/W2 answer texts.

Run a bounded new logical job with explicit actor identities and source-bound
report addresses. A creates an independent O saved-Chat submitted-inference
session in the correct project; O creates independent W1 and W2 only as actual
budget/capacity allows. Use W1 `17*19=323` and W2 `23*29=667`.
W1 and W2 must be independent actual model conversations; choose concurrent
or sequential execution based on verified constraints. Before each ASSIGN
fix the native thread/Core home, ChatGPT conversation, logical parent,
report destination, idempotency key, result-send command, ACK route, and
finite budget. Do not hand a worker a bare 'report back' instruction.

| Gate | Required original proof | Current new-job state |
| --- | --- | --- |
| C1 | A creates and starts an actual independent O model | NOT_TESTED |
| C2 | O (not A) creates and assigns actual independent W1/W2 | NOT_TESTED |
| C3 | W1/W2 produce verified saved-model results | NOT_TESTED |
| C4 | each W result is sent to the *bound O* return address | NOT_TESTED |
| C5 | O's real next model input consumes each distinct report | NOT_TESTED |
| C6 | O independently verifies and ACKs or revises each report | NOT_TESTED |
| C7 | O report delivered to the *bound A* destination | NOT_TESTED |
| C8 | A actually consumes/reports to user with receipt | NOT_TESTED |

Historical saved W1/W2 finals (323/667) exist, but no proven historical
C4-C8 chain; see CURRENT. Do not replay those workers to retrofit success.
If queue -> idle Chat wakeup or addressed delivery is unavailable, record
the exact furthest stage, original receipt and needed manual input. Any
manual successor turn is manual recovery, never unattended orchestration.
If V1 blocks, continue independent safe V2 testing and register the missing
bridge behavior as P0; do not declare the entire product broken.

Test no duplicate model/tool effects across one uncertain queue delivery;
same logical job O1->O2 and worker replacement when safely possible.
A saved conversation ID, Native work ID and queue ACK are different receipts.
Stop requests and unresolved Native claims from other tasks are not takeover
permission.

Observe **actual** `max_parallel`, `max_app_starts`,
`max_native_calls`, `max_elapsed_seconds`, parent-work linkage,
starting-native admission, and `extend_budget`. The desired future
defaults are ceilings (5 parallel, 256 app starts, 1024 native calls; elapsed
configurable), **not** proof of current defaults. Prior source analysis
indicated budget extension does not cover all four dimensions or submitted
inference; recheck actual current API and preserve pending effects.

Deferred design, not an implementation claim:
[durable logical jobs](docs/development/orchestration/SESSION_INDEPENDENT_DURABLE_ORCHESTRATION.md)
and [ChatGPT/Native return bridge](docs/development/orchestration/CODEX_CHATGPT_ORCHESTRATION_BRIDGE_DESIGN.md).

## Stage V2 — installed 0.167.2 and exact-source product acceptance

Use V1's real proven workers if available; otherwise conduct independent
nonconflicting tests through authorized shell/Native paths. Each row must
record *actual installed generation, development source SHA, test path,
receipt/exit, proof level, conclusion and gap*.

| Track | Minimum actual acceptance and important boundary |
| --- | --- |
| R1 Core, release, CLI | `codex --version`, `termux version` JSON/details, Doctor no-bwrap, Core-only vs Full, update no-op, rollback metadata, old generation coexistence |
| R2 Provider/AppHost | running S27 executable vs selected generations, protocol-4 capability negotiation, valid `/v1/native/responses` model call/SSE, legacy path, error 400/404/413, disconnect/receipt recovery |
| R3 Identity and TUI | `/provider`, `/model`, `/switch`, actual account/model authority, effort, old model/thread retention and cross-generation safe refusal |
| R4 Native effects | file create -> compile -> execute -> modify -> compile -> execute in same thread, exact stdout/exit, tool/stream continuity, process-tree wait, scoped Stop, reconciliation, same/independent writer concurrency |
| R5 Reviewer/Termux | real AutoReview allow/deny invocation when called, denied execution absent, missing bwrap accurately classified, sandbox request failure not hidden |
| R6 MCP and orchestration | independent `termux_shell` under Native Busy, exact-call replay, collision reject, same-project chat identities, C1-C8 delivery |
| R7 Services and terminal | Provider/tunnel/keeper runit owner/image, restart/rollback simulation without mutating live service, notification tap -> correct terminal/thread, double-click/Android Activity restrictions |
| R8 GitHub CI | 0ba exact Source/Core/Manager/Provider/Android/TUI success; Full early-failure admission; artifact fingerprints and evidence reuse across source changes |

Do not confuse a model-only result with a Native command, HTTP 400 from
malformed JSON with normal inference, a Termux API notification receipt with
visible Android Activity, or an operating S167 run with an uninstalled
development candidate.

For the existing **0ba05892** exact-source Full run `43353147279`,
failure was *no successful exact-source Core qualification at admission
time*. Core `43353147456` and TUI `43353819723` later passed. The
first minimal CI action, when this stage is authorized to launch unsigned
CI, is to verify all prerequisites and attempt **one** exact-source Full
qualification; do not rewrite Full assembly or rerun a 2-4 hour TUI test
without changed inputs. If a new documentation-only SHA is pushed, use the
CI's actual immutable producer fingerprints and exact-source admission
requirements instead of labeling old CI as proof for the new SHA.

## Stage V3 — document findings and unify the development line

One canonical obligation, one proof record, one open backlog:

- Stable requirements -> SPEC and owning `docs/spec/` annex only.
- Present evidence -> CURRENT, including exact generation/SHA/run IDs,
  source vs installed/live vs physical Android limits.
- Current unfinished steps, ordering, priorities and owners -> **this
  HANDOFF**; do not publish another competing roadmap.
- The original W0-W20 historical scope, decision changes and provenance ->
  `docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md`.
- Proposed unimplemented durable orchestration and bridge:
  `docs/development/orchestration/` design pair, referenced from here.
- Proposed unimplemented runit update/rollback reconciliation ->
  [runit task](docs/development/RUNIT_GENERATION_ACTIVATION.md).
- Raw old failed receipts, snapshots and superseded ledgers remain
  recoverable from precise Git/CI/chat identifiers, not duplicated in
  evergreen norms.

For every new defect record an ID, severity, date, source/installed generation,
actual symptom, repro and original receipt, proven cause vs hypothesis,
affected producer/consumer and owner, safe temporary boundary,
implementation/test requirements (positive and negative), dependency,
operating authorization and closure evidence. Do not convert conjecture to
a confirmed defect.

Review specifically the source-document branches
`docs/durable-orchestration-contract-20261112` and
`work/tunnel-termux-v0.0.20-20261113`, plus any other unique active
documentation changes; transfer by exact blob/verified hunk, **not whole
unrelated histories**, and track source SHA. After one safe non-force
fast-forward to `rewrite/rust-core`, verify remote readback and classify
old branches: merged, equivalent, safe to delete, blocked by active Writer,
or unique content requiring additional review. Do not delete or rewrite
branches merely to make the count small.

Run real document lint, declaration tests and `git diff --check`.
Docs-only push triggers some development CI; confirm no signing/publication
side effects, keep exact input-bound evidence, and avoid unnecessary
heavy builds when existing proof is legitimately reusable.

## Stage D — subsequent scoped development, after V1-V3 evidence

This stage is **backlog/design only for the present verification-and-doc
assignment**. User's newest instruction places code repairs, wide CI
refactoring and runit automation *after* completed chain testing and
document consolidation; do not smuggle them into V1-V3.

| Priority | ID and owning W | What remains / completion oracle |
| --- | --- | --- |
| P0 | D-ORCH-01 (new A/O/W bridge; W15/W17/W18 consumers) | Explicit W->O and O->A addressing, durable receipts and receiver ACK; prove C4-C8, no duplicate effect, safe account/project and generation binding |
| P0 | D-ORCH-02 | Idle saved-Chat wakeup, independent O/W replacement and durable job checkpoint/claim; test disconnected receiver, concurrency and restart, not just a queue ACK |
| P0 | D-BUDGET-01 | Configurable safe limits for parallel/app starts/native calls/elapsed; extend_budget all four dimensions on real admissible sessions; verify parent/sibling admission, negative cases, original work unaffected |
| P1 | D-NATIVE-01 (W15/W20) | Real tree/Stop/ACK/unknown-effect reconciliation, independent concurrent Native commands and conflicting Writer refusal, same-thread switch/provider continuity |
| P1 | D-ROUTE-01 (W17/W19) | Real protocol-4 valid JSON/SSE, 400/404/413 recovery, stable URI + legacy compatibility, correct model/account/Provider |
| P1 | D-RUNIT-01 (W10/W18 lifecycle) | Future opt-in runit Provider/tunnel/keeper generation sync on install/update/rollback/reboot; selected vs active binary/pending status; no duplicate supervisor, account changes or unsafe restarts |
| P1 | D-CI-01 (W1-W4/W10) | Full gate waits for exact-source prerequisite success, avoids immediate predictable failures, reuses producer digests, preserves required tests and publication gate |
| P2 | D-DEVICE-01 (W16/W20) | Real Android notification tap and optional auto terminal display, no duplicate terminal/thread, foreground/background/security test |
| P2 | D-CLI-01 (W11-W14) | Verify any remaining installed Core-only/update/version/Doctor and old-format migration gaps; patch only reproduced defects |
| P2 | D-REFS-01 (W9/W10) | Finish safe legacy-branch/worktree retirement only with exact owner/PR/tag/CI evidence and no unique changes |

This is a dependency-ordered queue, not approval to implement future runtime
changes today. Once each scope is separately authorized, implement in the
**same** development line with bounded Writer branches, focused tests,
exact-source CI and clean integration. A blocking task cannot prevent safe
independent tests or documentation.

## Stage Q — final exact-source qualification and separate operating release

After permitted development changes: settle W0-W20 source-vs-proof
matrix, run the *affected* Source/Core/Runtime/Manager/Provider/TUI/Android
and Full gates on exact required inputs, record run IDs and conclusions,
then update CURRENT and reduce HANDOFF to genuine remaining tasks. Do not
mark scheduled/running CI success. Do not skip a needed test merely because
it is expensive. Fast-forward/readback the single dev branch only.

Signed publication, release sequence reservation/stable index promotion,
new Provider activation, reboot, service restart, account/login changes,
real user SQLite/JSONL repair and on-device replacement require their
**separate explicit operating authorization**. Source acceptance can be
completed without these actions, but physical/account acceptance cannot be
declared from synthetic receipts.

## Completion reporting required

Report the original W0-W20 scope-versus-implemented status, changed
requirements/order, actual test matrix, C1-C8 deepest verified state,
bounded budget results, exact model/Native/shell receipts, detected issues
with status and severity, three imported documentation origins,
local/remote commits and CI implications, branch-retirement dispositions,
operating asset invariants, deferred implementation backlog, and exact
next authorized acceptance gate. Never declare the overall 0.167.2
orchestration/product fully accepted before its required live criteria pass.
