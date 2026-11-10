# Integration review action triage — active and non-normative

**As of:** 2026-11-10 KST. **Review source SHA:** `49cfab806cb99d47a6de0d918ed13777db8e11a5`.
**Inspected integration HEAD:** `0aebdd7f2614a38c08f96e1c266f0a7c2d88609a`.
This is a **temporary work selector**, not a product specification, a permanent issue ledger,
an approval to execute risky operations, or evidence of completed device acceptance.
For the current running generation and independently observed gates see
[CURRENT](../../CURRENT.md); for the next authorized steps see [HANDOFF](../../HANDOFF.md).
The [SPEC](../../SPEC.md) and its normative annexes remain authoritative.

**Deletion condition:** Once all relevant packets below have been accepted, consciously
deferred or rejected with exact commit/test references in Git and no active HANDOFF
dependency, delete this file. Do not turn it into a chronological log.

## Evidence, scope and confidence

- User-supplied `codex-integ-review-20261110.tar.gz`
  (SHA-256 `e324620856522ba915089ed5f5a3d19f99b068e72d825df5f2ab3aa970ca8c1d`),
  `codex-integ-transcripts-20261110.tar.gz`
  (`17bd3c7439fcb7ba1ba44ae4d45ce16d38b0c12ccd640224a21f53cfc092bdea`)
  and `codex_wrapper_review_assessment_20261110.md`
  (`2fa04883a2d7d44438dc7a942a615e3c0065dabd310f312379a3ba7509c4aa29`).
  Input archives and full source snapshot are **not** added to the repository.
- The original 109 observations were re-evaluated as **80 true / 28 partial /
  1 false positive**. The independent adoption recommendation assigns
  **P0 10, P1 38, P2 33, conditional 18, no targeted fix 8, documentation
  already addressed 1, rejected 1**. These are source-review judgments,
  **not** 109 independent failing-device reproductions or proof of exploitation.
  The 16 model transcripts are prompts/replies, not lower-level tool logs.
- The older *separate* 74-item internal review, preserved by
  `git show 49cfab80:docs/history/CODE_REVIEW_20261110.md`, had
  **26 reflected / 3 partial / 7 already satisfied / 14 insufficient evidence /
  24 unresolved** at its own baseline. Its IDs have different unit meanings:
  never add 74 and 109 or equate identifiers without their review generation.
- At HEAD `0aebdd7f` we independently read source mechanisms for
  all ten recommended P0 IDs, but did **not** run new fault-injection,
  live-model, installed-device or security penetration tests. Core/Provider
  runtime files underlying them have no intervening source change from
  `49cfab80`. The resulting judgment is **source-supported, not
  device-accepted**. Current Source Acceptance success does not waive this.
- Specific live facts from CURRENT remain separate: prior synthetic
  roundtrips passed; candidate `/provider` picked GPT-6 but the first
  actual turn failed `native_thread_owner_unavailable` with mixed
  Core/Provider/Native ownership. Do not silently count earlier direct-profile
  success as installed-TUI acceptance.

## Differences from the older 74-item review

- Earlier redaction and root CLI parser corrections were real, but new
  **U1-4** (prefixed secret keys / Bearer whitespace) and **U1-6**
  (`--worktree`, `-i` subcommand swallowing and profile
  `sandbox_mode`) identify **additional uncovered cases**, not reasons
  to revert the earlier fixes.
- Earlier Core rollback/SIGKILL recovery was verified; **U1-2** targets a
  different snapshot-error and pointer/hold-finalization window. Never
  "fix" it by deleting an unverified live rollback guard.
- Earlier zombie-thread/process-owner protections must be retained when
  addressing **U2-2**. A zombie leader does not prove that its worker threads
  have exited; preserve FD36/inode lease semantics from the earlier review.
- Older TUI review found no proven unbounded *individual* child command.
  **U5-4** now shows an unbounded **outer** `waiting` loop and
  unvalidated `retry_after_seconds`. Make polling bounded/cancelable
  while preserving a potentially admitted Native input and its draft.
- Earlier publication pagination and Manager ESC-input observations overlap
  **U7-5** and **U2-14**; earlier release-CAS/already-pinned-dispatch concerns
  are **additional** focused failure cases, not automatically covered by U7-2.
- **U6a-1 is rejected as a CI-bypass claim:** Provider workflow triggers do
  not depend on `ci_scope.py`. **U7-4 is a separate real branch and
  notifier ownership issue.** **U7-1's outdated README pin was removed** by
  the documentation refactor; independently test Base online installation
  rather than reintroducing that pin.
- Do not repair **U7-8** as active publication (no caller), **U7-10** without
  a Cargo-produced counterexample, or **U7-3** unless legacy IDs are still
  required; remove dead paths only after consumer inventory.

## Recommended bounded work packets

Each packet is a separate owned source change with testable positive and
negative outcomes, a coherent commit per distinct fix, and exact-HEAD
source/provider/TUI gate checks. **No signing, activation, service restart,
account migration, live-home cleanup, force push, or Android sdcard access.**
Preserve active Full159 and Full153 rollback. Investigate the source and owner
again at actual task HEAD: a finding ID is a hypothesis, not an instruction to
patch mechanically.

### R01 — Release/CI evidence admission (U7-4 P0; U6b-4 P1)

- **Owners/files:** `.github/scripts/qualification_signal.py`,
  `.github/workflows/{source-acceptance,manager-tui-native,provider-qualification}.yml`,
  `scripts/check.sh` and applicable tests.
- **Task:** Make branch and exact-SHA artifact ownership explicit. Remove
  unconditional publication-notifier dispatch on unrelated refs; do not
  admit an integration artifact as a production release merely because CI
  passed. Wire omitted Chat-model transition tests into the appropriate
  required provider/TUI check without broadening unrelated gates.
- **Acceptance:** allowed source branch/ref/SHA passes; wrong branch, wrong
  head SHA, stale artifact, and failed/skipped required gate are **rejected**;
  a docs-only integration run cannot spuriously dispatch
  `accepted-release-inputs.yml` on `main`. Verify GitHub
  event/test matrix before changing the real production workflow.
  **Do not manually dispatch current integration source-acceptance or
  manager-tui-native before R01:** their unconditional notifier can
  trigger `accepted-release-inputs` on `main`, even from docs-only.

### R02 — Authenticated local MCP controls (U3-1 P0)

- **Owner:** Provider MCP HTTP boundary, `providers/chatgpt/src/mcp.rs`,
  service router and tests.
- **Task:** Authenticate control tools at their actual service boundary;
  `Origin` absence and loopback binding are not authentication.
  Preserve explicit per-turn capability checks for Native calls and arrange
  authorized client bearer use without logging credentials.
- **Acceptance:** same-device unauthenticated `/mcp` control
  `tools/list`/`tools/call` do **not** disclose/control;
  invalid token rejected; valid authorized control and Native capability
  routes still work. Inspect all listeners/forwarders and client creation.

### R03 — Core crash-safe and pre-exec safety (U1-2, U1-4, U1-6 P0; U1-1 P1)

- **Owner:** `crates/core`, focused regression fixtures.
- **Task:** Reconcile rollback guard/update hold from the authoritative
  pointer and snapshot across returned error and SIGKILL windows; cover
  prefixed secret keys and multi-space Bearer redaction; validate all
  supported root options and profile sandbox override *before* exec.
  Track force-update hold as a separate logical fix/commit.
- **Acceptance:** failed snapshot cannot brick subsequent correct
  update/rollback; no unverified guard purge; real secret-free synthetic
  samples never leak; unsupported Termux sandbox requests fail pre-exec
  across exact entrypoints; valid upstream options remain forwarded.

### R04 — Remote dispatch and durable Native continuity
(U4-3 P0; U3-2, U3-7, U3-9 P1)

- **Owners:** `providers/chatgpt/src/{mcp_bridge,mcp,operator}.rs`;
  operation journal and tests.
- **Task:** Separate persisted dispatch intent, remote acceptance and
  completed result. On ambiguous dispatch, consult original remote/Native
  receipt before returning `started/reused` or admitting a new
  round. Preserve delayed `source_join`, compaction token and
  cache-evicted/restarted unsettled work semantics.
- **Acceptance:** inject failures before and after dispatch, after ACK,
  and across restart/compaction. Original call ID has exactly one effect,
  true result recovery and no false reuse or `mcp_pending_new_input`
  wedge. Never create a new thread/model call to disguise a missing result.

### R05 — Opaque effect ownership and safe parallel jobs
(U4-6 P0; U3-4, U3-5, U3-6 P1; linked U2-1, U2-2 P1)

- **Owners:** `providers/chatgpt/src/{local,operator,operator_native}.rs`,
  SharedNativeGuard/runtime binding, Manager process probes as applicable.
- **Task:** Reconcile ambiguous local-process effects with authoritative
  process/writer evidence. Count `starting_native` as reserved
  assignment capacity, settle orphan app-marker claims and remove output
  head-of-line blocking without allowing a second writer.
- **Acceptance:** after unknown delivery, preserve uncertain running effect,
  recover exact exit when proven, release the owned claim only then, allow
  unrelated independent work, cap same-parent `max_parallel`, keep
  unsafe conflicting writes blocked. Non-dumpable and zombie-process probes
  may not infer "exited" from mere inspection failure. **No blind lock purge.**

### R06 — TUI one-shot re-entry and input synchronization
(U5-1, U5-4 P0; U5-3, U5-5, U5-6, U5-7, U5-8, U5-9 P1)

- **Owner:** `frontend/tui/src/app`, `termux_chat_models.rs`,
  applicable app-server/Provider contracts.
- **Task:** Scope/clear one-shot `CODEX_CHAT_MODEL_REENTRY` and
  return-profile markers; avoid leaking settings on unrelated
  `/resume`/`/switch`. Make `waiting` response
  timing validated and cancellation/retry state bounded; preserve drafts
  and original Native work, correct missed model and pagination paths.
- **Acceptance:** switch → resume → normal resume/switch has correct
  model/home/profile; zero/huge/missing retry delay fails safely with draft;
  repeated waiting is interruptible without replay; queued drafts are
  resynchronized when stale, and two real GPT-6 turns use the same writer.

### R07 — Non-destructive migration first (U6b-2 P0; U6b-1 P1)

- **Owner:** `scripts/shared_visibility_transition.py` and
  `scripts/shared_state_migrate.py`.
- **Task:** Make actual app-server acceptance, durable intent, complete
  pagination and recovery prerequisites of destructive rename/unlink/
  SQLite operations. Preserve original state for restart/retry and prevent
  migration on active stores.
- **Acceptance:** isolated copied fixture with >1 result page and
  fault-injected RPC failure, interruption and rerun; no loss, duplication,
  wrong-writer adoption or unrecoverable `activating` journal.
  **Never run destructive acceptance on installed user homes.**

### R08 — Qualified release and update transaction hardening (P1)

- **IDs:** U1-3, U1-5, U1-12, U7-2, U7-5, U7-7; keep U1-1 with R03.
- **Older internal residual:** immutable-release retry/CAS after main moves,
  `already-pinned` dispatch recovery, and bounded publication
  pagination. Recheck source and the signed-generation contract first.
- **Acceptance:** error and process-loss retry return to one verified
  generation; immutable published data are never rewritten; stale/partial
  draft and >100 GitHub runs do not permanently wedge permitted work;
  candidate=false cannot silently qualify Full; existing Base/Full
  fingerprints and signing gates remain fail-closed. Test current
  `install-online.sh --variant base` positive/negative input on
  isolated fixtures; U7-1's deleted README pin must not return.

### R09 — Qualification, credentials and owned test artifacts (P1)

- **IDs:** U6a-2, U6a-3, U6a-4, U6a-9, U6a-10, U6a-11, U6a-15, U6b-8, U4-1.
- **Acceptance:** no credentials in process argv or persisted diagnostics;
  sealed inputs include effective configs; notification receipts are checked;
  deploy readback matches exact bytes, not substring; relay shutdown is
  orderly; only demonstrably generated, inactive, owned fixtures are
  removed. Retain original evidence and preserve live caches.
- **Special case:** `d3check.*` is a legacy test-root naming/
  accumulation issue, **not** permission to restore old development
  phase names or delete an active Cargo target.

### R10 — Remaining P1 focused patches after the blocking packets

- Core/Manager: **U1-9, U1-11, U2-5, U2-9** (terminal sanitization, child
  environment/lifecycle, handoff cooldown, no-follow filesystem checks).
- Provider: **U3-3, U3-8**, plus remaining R04/R05 IDs (turn-token redaction
  and human-question ID normalization).
- Review-driven UI/product tests: focus on R06 and existing
  `cdp_operation_timeout`, SSE, HTTP 413 and duplicate prevention
  failures rather than disconnected cosmetic rewrites.
- Separate historical internal Manager residuals: long ESC/CSI/PTY input,
  quota refresh acknowledgment, ambiguous window launch ACK and bounded
  filtered result lists. Do not remove FD36 or treat zombie leader as dead.
- **Gate:** accept only when independently reproduced or when a source
  contract/failure-case test proves the affected caller; unrelated cleanup
  must not displace P0 or end-to-end acceptance.

## Disposition of the non-blocking observations

- **P2 (33):** U1-7, U1-8, U1-13, U1-14; U2-3, U2-6, U2-11, U2-12, U2-14;
  U3-10, U3-12; U4-9, U4-10, U4-11, U4-12, U4-13; U5-12, U5-13, U5-16;
  U6a-5, U6a-6, U6a-8, U6a-12, U6a-13, U6a-14; U6b-3, U6b-5, U6b-7, U6b-9, U6b-11; U7-6, U7-11, U7-12.
  Schedule only when their actual owner path is modified, a regression is
  demonstrated, or the related gate is in scope.
- **Conditional (18):** U1-10, U1-15; U2-7, U2-8, U2-10, U2-13; U3-11;
  U4-4, U4-5, U4-8; U5-2, U5-10, U5-11, U5-15; U6a-7; U6b-6; U7-3, U7-9.
  Verify supported scenario/remaining consumer before either fix or removal.
- **No targeted fix (8):** U2-4, U4-2, U4-7, U4-14, U5-14, U6b-10,
  U7-8, U7-10. **Already addressed documentation (1):** U7-1
  (keep positive/negative Base installer acceptance).
  **Reject (1):** U6a-1 CI-bypass interpretation, not the separate U7-4.
- This counts all **109** new review IDs once; it does not resolve the
  old 74 review by arithmetic or create a new product requirement.

## Execution boundary, dependency order and LOC budget

**Scope:** Planning only at integration HEAD `e16bb12d`. Do not infer a working multiple-Native-writer runtime from having multiple worktrees. Source owners may review separate components in parallel; **a real Native tool/process writer may be shared only if the product contract explicitly coordinates that single thread/work**. Until R05 is accepted, schedule one physical Termux Native execution lane for this product unless two complete independent Core+Provider+Native environments and every shared guard/journal/endpoint are actually demonstrated isolated.

**Sizing method:** Engineering **ranges of touched implementation lines + test/fixture lines**, counted as approximate changed source lines (inserted/modified/deleted), not development hours, not current file LOC and not measured delivered diff. Based on existing Core `main.rs` ~29,250 LOC, Provider `mcp.rs` ~4,634 / `operator.rs` ~8,626 / `mcp_bridge.rs` ~1,187 / `local.rs` ~876, TUI synchronization/reentry ~2,171, and shared migration ~1,527. Estimates include negative and failure/recovery fixtures but **exclude** documentation-only edits, production activation, external CI wait, version-wide baseline rebuild, investigation of unrelated findings, and cleanup of other jobs. Re-estimate after each proof-backed source diagnosis. Budget is nonbinding; preserve safety even if a fix exceeds it.

| Packet / isolated source owner | Implementation changed LOC | Test/fixture changed LOC | Packet total | Release relevance |
| --- | ---: | ---: | ---: | --- |
| **G0 R01** — GitHub gate and notifier | 80–180 | 120–250 | **200–430** | Mandatory before promoting or dispatching GitHub acceptance |
| **G1a R02** — MCP bearer/control authorization | 70–150 | 110–230 | **180–380** | Mandatory authorization boundary |
| **G1b R04** — original dispatch intent/result recovery | 140–300 | 160–360 | **300–660** | Mandatory Native continuity |
| **G1c R05** — effect claims, assignment capacity and parallelism | 150–380 | 180–450 | **330–830** | Mandatory isolation and availability |
| **G2 R03** — Core rollback, redaction and root parsing | 140–320 | 180–410 | **320–730** | Mandatory Core/rollback correctness |
| **G3 R06** — TUI one-shot settings and synchronization | 130–310 | 160–350 | **290–660** | Mandatory installed model path |
| **G4 R07** — migrated-state destructive handoff | 90–200 | 130–300 | **220–500** | Mandatory **if** used by installer, upgrade or migration; otherwise defer with explicit disablement proof |
| **G5a R08** — qualified release/update retry and Base | 160–340 | 180–400 | **340–740** | Mandatory before any signed release involving these paths |
| **G5b R09** — qualification/credentials/test outputs | 110–260 | 120–290 | **230–550** | Mandatory for active credential and evidence gates; defer unrelated fixture hygiene |
| **G6 R10** — remaining scoped P1 fixes | 100–250 | 120–260 | **220–510** | Conditional: move reachable credential/FS/launch risks into release blockers |
| **R01–R09 total** | **1,070–2,440** | **1,340–3,040** | **2,410–5,480** | Before release as applicable, with G4 conditional |
| **R01–R10 maximum planned scope** | **1,170–2,690** | **1,460–3,300** | **2,630–5,990** | Not an authorization to expand the release |

No fixed per-task code quota: these ranges bound *planning*, not what tests may be removed to meet a target. A finding touching two packets is implemented by its single actual code owner and counted **once**. Additional P2/conditional cases are not included.

### Resource partition: source editor is not Native writer

| Resource/authority | Allowed independent ownership | Conflicting/forbidden |
| --- | --- | --- |
| Source, Git index, branch | One feature branch **and** worktree per packet; dedicated integration owner merges/updates one shared development ref **sequentially** | Multiple editors of one checkout/index; force-pushing, resetting or cleaning another worker |
| Native thread/work and writer lease | Original owning Native work and matching Core; parallel Native execution only on proven separate physical writer/lease/journal and supported coordination | Different packet using same Native work/thread, adopting an installed writer, changing an active thread's home |
| Core + Provider execution identity | A **fully isolated pair** of Core/Native and Provider service with the same matched isolated HOME, CODEX_HOME, workspace and qualified bindings; or pure static/mock test | Isolated Core pointed to operating Provider; two Providers accepting the same writer without authority |
| Journals, operation IDs and external `claims.json` | Check **canonical** path, UID, inode/owner, claim namespace and original call receipts before admitting concurrent effects | Treating a different `work_id` or cloned HOME as proof that shared global opaque claims are independent; clearing unknown claims |
| Unix sockets / TMPDIR | Canonical owner-private **short** Termux socket root, verify both full Core/shared and Native FD35 paths at <= **107 encoded bytes** including actual UID/hash; separate long evidence/scratch dirs | Deep TMPDIR, fabricated symlink alias, same socket/port/UID identity used as different owner |
| Build/test artifacts | Per-packet `CARGO_TARGET_DIR`, isolated fixtures, owner-only temp output and unique test sockets; shared downloaded crates only when safely immutable/read-only | Deleting live build caches, reusing writable targets/fixtures, mutating system or Android sdcard |
| Operating generations, account, services | **Observation only** for all source packets; Full159/Full153 are last observed, refresh before operational work | Restart, activation, sign/publish, profile/auth edits, migration against real home |
| GitHub workflows and release authority | G0 exclusively owns workflow changes; separate **read-only** evidence review; CI after verified notifier fix and exact ref/SHA | Concurrent workflow writes, unguarded manual source-acceptance / manager-tui-native dispatch, integration SHA treated as production main authorization |

**Pre-effect admission check for each packet:** `pwd`, `git status --short --branch`, HEAD + remote, other worktrees, original Native work/owner/status and pending effects, canonical HOME/CODEX_HOME/provider endpoint, actual lease/claim paths, socket byte budgets, isolated target/tmp paths. If any may overlap, do **not** start a second Native execution or automatically stop the owner; continue nonconflicting source inspection, or wait for original receipt settlement. Do not use unsupported Termux `workspace-write` Linux namespace sandbox as an isolation substitute. Different model-agent sessions are **not** different Native writers by themselves.

### Dependency schedule / exclusive acceptance gates

1. **S0 — Freeze evidence and boundaries:** read/write ownership audit; record all worktrees and the three untracked `rewrite/rust-core` files **by path only** without reading/editing/staging them; record last verified source/CI SHA. Keep existing installed signed generations and services unchanged. No runtime effects during planning.
2. **S1 — G0 R01 first (CI-only):** source/test corrections for branch/ref/SHA admission, notifier conditions and model-transition test gate. Verify simulated push, workflow_dispatch, failed/skipped/foreign-run and exact-branch cases **without** the presently unguarded GitHub notifier. Do not start a production approval workflow to test its safety.
3. **S2 — Independent source-owner packets, only after isolation inventory:** G1a **R02 → G1b R04 → G1c R05 are serial within one Provider owner** (MCP and claims share files/contracts). G2 R03, G3 R06 *synthetic-only*, and G4 R07 *copy/fixture-only* may be developed in separate owned worktrees. If real Native tools compete for one opaque lease, **serialize their actual executions**; parallel reading/code review does not require concurrent Native effects. G4 must never use installed user homes.
4. **S3 — Integrate one packet at a time:** one integration owner verifies tested SHA, code diff, test failures/negative proof and rollback behavior, merges in order G0, G1a, G1b, G1c, G2, G3, G4 as applicable; no silent blanket skip of source/provider/TUI gates. Re-run exact-merged-HEAD checks when shared interfaces change.
5. **S4 — Exclusive isolated-candidate Native acceptance:** stop admitting other packet experiments (do not forcibly stop active work); reconcile earlier effects, prepare one qualified **isolated candidate** Core+Provider+Native writer with identical binding and its own HOME, CODEX_HOME, namespace and socket; enter its real TUI `/provider` GPT-6 route. Prove two actual replies on one thread; SSE cut, HTTP 413, original-call result recovery, stop/handoff and independent work without duplicate effects. Never use the operating Provider for the isolated candidate or modify installed Core as a shortcut. If a candidate cannot be tested without touching operating state, record the gap and defer **installed** verification to the separately authorized device gate. G3 live validation follows G1/G2 merge; synthetic fixtures are insufficient.
6. **S5 — G5 release-source hardening:** R08/R09, real Base/Full integrity/rollback and exact immutable publication inputs tested in isolation. Resolve release-affecting R10 security/FS inheritance or explicitly prove unreachable; defer cosmetic/optional cases. No signed publish here.
7. **S6 — Explicit release/installed acceptance decision:** exact SHA source, Provider/TUI and S4 real isolated Native proof, independent signed generation and rollback checks, plus main-lineage reconciliation first. Seek **separate explicit authorization** for any signing, publication, `main` integration and device activation; after an authorized installed update, recheck selected installed Core/Provider/Native ownership and repeat two-turn/fault recovery and rollback acceptance on the real installed route. A green CI gate or isolated candidate alone is not installed-device acceptance.

**Gate rule:** only S1 and source/mock parts of S2 may overlap in review; all mutations of the same code owner, Native effects, integration ref, public workflows, signed generations and device acceptance are **serialized**. Actual hardware parallelism is an R05 acceptance result, **not** an assumed prerequisite that can bypass the current global opaque-claim limitation.

### Branch strategy: one development authority, main remains release authority

- Observed before this document change: `origin/rewrite/rust-core` at `27103ec0` is an **ancestor** of integration `e16bb12d`, which is **117 commits ahead**. `origin/main` `53176ed7` **diverged** from integration: 413 commits unique to main and 634 unique to integration. A direct fast-forward to main is impossible and a blind merge risks production release state.
- Workflow ownership is **currently tied to `rewrite/rust-core`** in `source-acceptance.yml`, `provider-qualification.yml`, `accepted-release-inputs.yml`, `provider-release.yml`, and `qualification_signal.py`. The integration branch isn't interchangeable with that trusted release source. The existing `notify_acceptance if: always()` paths can wrongly dispatch main follow-up; this is G0's first remediation.
- **Recommendation:** keep `main` as release/publication branch; use **one long-lived `rewrite/rust-core` development branch** after G0 fixes and actual branch-owner approval. During G0 and packet preparation, retain `integration/codex-native-20261110` as the **temporary sole integration landing branch**, not a second automatic release authority. Once G0's exact-ref checks pass, independently verify a fast-forward-only move of `rewrite/rust-core` to the accepted integration HEAD; preserve all untracked/dirty files in its checked-out worktree, ensure no tracked-path collision, coordinate its active owner, no force/reset/clean. If unsafe, **do not advance** it; stay on integration and stop release dispatch.
- Do **not** add a third `develop` branch unless branch naming is deliberately changed **in every workflow, caller, source-acceptance and release-admission consumer in a single tested migration**. Changing a branch name is not a security fix; keeping two independently advancing sources will create inconsistent trusted SHA.
- Keep `main` separate until all S6 gates and a **specific main-versus-development reconciliation** of the diverged commit histories succeed. Use a reviewed merge/PR and official signed publication chain only with separate operational approval. Never force-update main or treat an integration CI run as an accepted production promotion.
- In the **existing `/data/data/com.termux/files/home/prj/codex` worktree**, untracked `ais.config.toml`, `chatgpt.config.toml`, `providers/chatgpt/src/accounts.rs` must **all remain untracked and untouched**. Do not inspect secret-bearing contents, stage, rename, move, overwrite, reset or clean them. Their presence does not authorize branch movement.

**Exit/document retirement:** each packet leaves its accepted commit and affected test/receipt, then is removed from HANDOFF when done. Retire this whole temporary review queue when no active packet depends on it and tracked Git evidence preserves dispositions. Neither SPEC nor AGENTS requires revision for this scheduling choice.

## Self-contained review finding dictionary

These `U` identifiers are **external review chapter/row labels**, not product/API names, repository paths, stable ticket IDs, or proof of a reproduced defect. The descriptions below are a **distillation of the supplied per-finding assessment** (review baseline `49cfab80`), not a substitute for current-code and device tests. `P0`/`P1` are proposed development priorities; `C` means confirm usage, `N` means no targeted fix, `R` means already addressed in documentation, and `X` means reject. The earlier internal 74-review uses different IDs; do **not** conflate. References to "Argon" inside a finding are disagreements between the external reviewers, not a reproduced failure.

### U1 — Core: update, rollback, command dispatch and terminal diagnosis

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U1-1` | P1 | `update-hold`가 force 활성화 후에도 남아 이후 로컬 빌드·원격 빌드·시작 업데이트 검사 전부 차단. |
| `U1-2` | P0 | 스냅샷 실패 시 rollback guard 잔존, 포인터 커밋~hold 정리 사이 크래시 윈도우 실재. |
| `U1-3` | P1 | 아카이브 절반만 참(`create_new(true)` 재시도 실패). 미봉인 디렉토리 절반은 일반 에러 시 거짓 — prepare 함수가 에러 시 output 삭제. |
| `U1-4` | P0 | `_`를 식별자 문자로 취급해 prefix 키(`OPENAI_API_KEY` 등) 미redact; `Bearer` 뒤 공백 여러 개도 누락. |
| `U1-5` | P1 | flock 전 Binding 읽기, pid 파일 누락 시 전체 중단, PID 재사용 누수/중단, SIGHUP 후 미대기 전부 확인. (Argon은 Medium이라 했으나 동일 유저 non-dumpable 프로세스에서도 PermissionDenied 발생해 High 유지.) |
| `U1-6` | P0 | `--worktree` 누락, `-i` 루프가 `exec`/`review` 삼킴, `profiles.*.sandbox_mode` 미검사. |
| `U1-7` | P2 | 핸들러 부재·Ctrl-Z 6시간 스누즈는 참. 단, termios 손상은 kill 신호 때만 발생 (키보드 Ctrl-C/Z는 바이트로 읽혀 복원됨). |
| `U1-8` | P2 | `WaitInput`을 긴 블로킹 호출에 걸쳐 유지해 Ctrl-C/Z 무응답은 참. "오안내 에러" 절반은 과장 — `gh()`가 WAIT_SIGNAL 폴링 후 단일 eprintln. |
| `U1-9` | P1 | CSI `K`가 파라미터 무시하고 행 전체 삭제, OSC가 `\n`에서 안 멈춤. |
| `U1-10` | C | 비UTF-8 `CODEX_HOME` 폴백·`private_dir` 거부는 참. 단, 기본 TUI 경로는 `native_socket`을 써서 `private_dir`을 안 탐. |
| `U1-11` | P1 | `LD_PRELOAD`/`LD_LIBRARY_PATH` 미제거 exec/spawn, `probe`가 조기 I/O 에러 시 자식 누수. |
| `U1-12` | P1 | prune 범위 누락·시퀀스 카운터 부재는 참. 단, live 삭제 위험은 activation writer lock 구조상 조건부라 부분참 유지. (Argon은 참이라 했으나 직접 근거 우선.) |
| `U1-13` | P2 | `timezone::configure`가 `private_dir` 전에 캐시 디렉토리 생성+canonicalize해 심볼릭링크 검사 우회. |
| `U1-14` | P2 | `prepare`가 `XDG_CACHE_HOME` 비어있음/상대경로를 안 걸러 `/proc` live-compiler 검사 우회. |
| `U1-15` | C | `TMPDIR` 변경 시 stale 심볼릭링크 소켓 정리 영구 실패, 죽은 소켓 파일 잔존. |

### U2 — Manager and Core runtime binding, process and filesystem ownership

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U2-1` | P1 | non-dumpable 프로세스(`ssh-agent` 등)가 있으면 `runtime_in_use()`가 `ERR_BUSY`로 중단돼 profile delete/rename 전부 차단. |
| `U2-2` | P1 | 좀비 리더 + 워커 생존 시 `process_start`/`process_exited` 둘 다 실패해 `native::servers()` 전체 중단 (`profile.rs`의 `process_alive`와 모순). |
| `U2-3` | P2 | 프로브가 프로세스 그룹 없이 spawn, 타임아웃 시 자식 파이프를 문 자손이 있으면 reader join 영구 hang. 단, 트리거가 좁고 빌드타임 도구라 Medium. |
| `U2-4` | N | 게시 디렉토리 0700 vs 파일 0755/0644 불일치는 사실. 단, 동일 UID 플로우 무영향이라 Low. |
| `U2-5` | P1 | `handoff()`가 가드 검사 전에 소스 쿨다운을 `cooldowns.json`에 저장. |
| `U2-6` | P2 | 논블로킹 `connect()` 재시도가 `EAGAIN\|EINTR`만 허용, `EISCONN`/`EALREADY`/`EINPROGRESS`에서 fail-closed. |
| `U2-7` | C | `getuid`/`geteuid` 불일치는 실재하나 setuid 실행 맥락이 레포에 없음. |
| `U2-8` | C | 하위 디렉토리 소유권/모드 검사 누락은 실재. 단, 부모 디렉토리가 0700+소유자 검증됨이라 크로스 UID TOCTOU 차단됨. (Argon은 Medium 유지했으나 직접 근거로 Low.) |
| `U2-9` | P1 | `read_bounded()`·`read_launch()`·`set_mode()`이 `O_NOFOLLOW` 없이 경로 기반 동작, `ensure_directory_chain()` 비원자적 생성+chmod. |
| `U2-10` | C | `adapt_tui()`의 symlink 검사 누락·입력 `O_NOFOLLOW\|O_NONBLOCK` 미사용. 단, 입력 사전 검증됨 + 스왑 윈도우 극히 좁음. |
| `U2-11` | P2 | `owns_writer()`가 스캔 중 FD 닫힘 시 `NotFound`를 `Err`로 전환, `(dev, ino)` 재검증 없음. |
| `U2-12` | P2 | `normalize-lock`이 `--audit` 파일을 정규화 전에 만들고 에러 시 안 지움, `Cargo.lock`을 SHA-256 검사 전에 제자리 기록. |
| `U2-13` | C | `window::service()`가 `,` 포함 경로 거부, `start()`가 `cwd` 절대경로 미검증. |
| `U2-14` | P2 | `launch::choose()`가 `ESC O A`/`B` 애플리케이션 커서 화살표 거부, 미인식 이스케이프 시 피커 중단. |

### U3 — Provider MCP broker, Operator, control plane and assignment accounting

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U3-1` | P0 | `Broker::router`가 POST `/mcp`를 인증 레이어 없이 마운트, `Controls::call`도 토큰 검사 없음. (리스너는 127.0.0.1만.) |
| `U3-2` | P1 | `source_join`이 durable receipt 목록에서 빠진 건 맞음. 단, 즉시 재시도는 멱등성 덕에 성공하고 네이티브 라운드가 전진해야 실패. (Argon은 Low라 했으나 지연 재시도 실패 확인돼 Medium 유지.) |
| `U3-3` | P1 | `enqueue`가 `[token, journal.owner]`만 검사하는 코드 주장은 정확. 단, 악용에 다른 턴의 48자리 hex 토큰 선행 탈취가 필요. |
| `U3-4` | P1 | `"starting_native"` 단계 작업이 `max_parallel` 카운트에서 누락돼 동시 형제 작업이 예산 초과 가능. |
| `U3-5` | P1 | `reconcile_stopped_work_effects`가 `app_marker` 아래 고아 물리 클레임을 `record.id`로만 조회해 누락. |
| `U3-6` | P1 | `connect_output`의 bounded 채널 send 대기가 단일 `select!` 루프에서 `terminate` RPC·인입 응답을 head-of-line 블로킹. |
| `U3-7` | P1 | compaction 롤백이 토큰 복원 없이 폐기하는 메커니즘은 참. 단, 영향 과소평가 — `"running"`으로 롤백되면 활성 턴 도구 호출이 즉시 깨짐 (재시작 후만이 아님). |
| `U3-8` | P1 | `human_requests` 삽입이 `Value::to_string()`으로 JSON 문자열 id에 따옴표 추가, `answer_question`의 따옴표 없는 조회와 불일치. |
| `U3-9` | P1 | `Broker::unsettled`이 인메모리 캐시만 보고 저널 폴백 없음 — 재시작/캐시 축출 후 `mcp_turn_unavailable`. |
| `U3-10` | P2 | `validate_arguments`가 `minimum`/`maximum`/`minLength`/`maxLength`/중첩 스키마 미검사. |
| `U3-11` | C | `try_send` 실패 시 `"abandon"` 드롭은 참. 단, "리로드까지 슬롯 누수"는 틀림 — 순수 읽기 receipt는 만료·스윕됨. |
| `U3-12` | P2 | 동기 `Journal::write`(`sync_all()` 포함)·`canonicalize`이 async Tokio 뮤텍스 보유 중에 실행. |

### U4 — Provider local execution, MCP bridge, unknown delivery and desktop controls

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U4-1` | P1 | `disableOutputCap: true` + 무제한 append로 디스크 고갈 가능. 단, 권한 경계는 안 넘음 (`local_execute` 자체가 명령 실행 권한). |
| `U4-2` | N | `serve`가 `execution.mode`를 무시하는 건 맞음. 단, Operator가 내부적으로 McpBridge/AppHost를 만들고 README도 mode만으로 경로가 안 정해진다고 명시 — "비활성화" 영향 주장은 틀림. |
| `U4-3` | P0 | McpBridge `submit`이 dispatch 전 round/token 저널 커밋, 롤백 없음 — 정확 재시도가 거짓 `{"started":true,"reused":true}`, 다른 라운드는 `mcp_pending_new_input` wedge. |
| `U4-4` | C | stop/release가 원격 실패에도 broker 턴 폐기, stop 상태 미영속. 단, stop 측 펜싱은 의도된 설계일 수 있음. |
| `U4-5` | C | `host_stop`이 검사+`shutdown()` 전 구간에 `host` 뮤텍스를 보유해 "락 없음" 전제는 틀림. 좁은 cloned-Arc 경합만 남음. (Argon은 Medium 유지했으나 직접 근거 우선.) |
| `U4-6` | P0 | 전역 `claims.json` 공유 + `delivery_unknown`인 `local_execute`가 계정 전체 `Effect::Opaque` 클레임을 절대 해제 안 함. (처음 Low로 내렸으나 Argon의 `settle_effect` 근거를 직접 확인해 Medium 복원.) |
| `U4-7` | N | `policy.inFlight`가 동시 채팅 읽기 실패시키는 건 맞음. 단, 의도된 pacing 정책이고 2초 재시도로 retryable. |
| `U4-8` | C | `"accept"` 라벨 룰·JSON minify는 맞음. 단, 평문 매칭이 대소문자 구분이라 대문자 `Accept:` 헤더는 redact 안 됨. |
| `U4-9` | P2 | 백틱 상태가 줄/문단 경계를 넘어 `<MemoryCite>`·`<box>` stripping을 끄는 건 맞음. 단, 영향은 외관상 마크업 누수뿐. |
| `U4-10` | P2 | `--thread` 검증 에러 메시지가 허용되는 `approve-ui` 누락. |
| `U4-11` | P2 | `control_command` missing-action 에러가 `projects` 누락. |
| `U4-12` | P2 | `browser_defaults`가 `mimeapps.list` 매칭 시 공백 trim 안 함. |
| `U4-13` | P2 | X11 디스플레이 번호를 파일시스템 검사로 선택, 경합 시 재시도 없이 실패. |
| `U4-14` | N | `connectDesktop` 기본값이 `app: false`라 합성 `Ctrl+Shift+N`·버튼 클릭이 라이브 앱 창에 들어감 (인-레포 호출자는 전부 `{app:true}` 전달). |

### U5 — TUI model selection, resume, input synchronization and UX

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U5-1` | P0 | `remove_var` 사용처 0건, 두 exec 경로 모두 미제거, `resume_model_settings()`가 매 `/resume`마다 env 재독해 이전 프로필 모델 강제. |
| `U5-2` | C | 소유 src 미해시·헤더 실패 주장은 실제 워크플로에서 거짓 — `git add -N` 후 포맷된 패치라 src 포함되고 헤더 없음. 잠재적 취약성만 남음. |
| `U5-3` | P1 | `config.model`이 None이면 수동 `/switch`·자동 quota handoff가 조용히 실패. |
| `U5-4` | P0 | 무한 재시도 루프 + `retry_after_seconds` 무검증 sleep (0이면 fork-bomb, 큰 값이면 무기한 대기). (Argon은 Medium이라 했으나 사용자 조치 없이 진행돼 High 유지.) |
| `U5-5` | P1 | drain 시 배리어 재검사 건너뛰는 코드는 참. 단, 준비 단계에서 이미 동기화하므로 완전 우회가 아니라 staleness 문제. (Argon은 오탐이라 했으나 큐잉 분기가 `prepared_input` 보존함을 직접 확인.) |
| `U5-6` | P1 | `resume_saved`가 빈 세션 거부·잘못된 스레드 이력 페이지네이션·대상 모델 미검증. |
| `U5-7` | P1 | `quota_switch::State`가 스레드별 스코프 없음, `turn/started`가 `uncertain`·`pending` 무조건 초기화. |
| `U5-8` | P1 | 두 번째 `ThreadResume`이 `..Default::default()`로 launch 설정·권한 오버라이드 누락, readback 미검증. |
| `U5-9` | P1 | `prepare()`가 멱등성 없이 체크포인트를 표준 이력에 주입 — 실패·반복 시 중복 누적. |
| `U5-10` | C | 108바이트 초과 메커니즘은 맞지만 Termux 실제 TMPDIR 기준 60~70바이트로 여유, 테스트 전용 변경. |
| `U5-11` | C | 핑 수학·문서 보장 위반은 확인. 단, "요청 데드라인 초과"는 이 체크아웃에 없는 업스트림 소스라 검증 불가. (Argon은 오탐이라 했으나 문서 위반 확인돼 부분참 유지.) |
| `U5-12` | P2 | `copy()`가 매번 capability discovery + 최대 8초 블로킹 sleep 루프, 캐시 없음. |
| `U5-13` | P2 | 설정 토글마다 25초 `--refresh` 네트워크 호출. |
| `U5-14` | N | openai 스레드에서도 제공자 바이너리 스폰하는 동작은 맞음. 단, 문서상 의도된 동작이라 부분참. (Argon은 참이라 했으나 직접 근거 우선.) |
| `U5-15` | C | 조기 반환과 무관하게 defer 호출하는 구조는 맞음. 단, `QueueDrain::Stop`이 부분 완화하고 업스트림 `Model` 패턴과 동일. (Argon은 참이라 했으나 직접 근거 우선.) |
| `U5-16` | P2 | 중복 플래그 방출·길이 상한 부재는 맞음. 단, 뒤 `-c`가 이겨 영향은 인수 개수 압박에 한정. |

### U6a — Qualification scripts, artifact checks and fixture lifecycle

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U6a-1` | X | 덮어쓰기 메커니즘은 실재하고 테스트에 명문화됨. 단, provider-qualification.yml은 ci_scope.py를 안 쓰고, 혼합 커밋의 'python' 분류는 Python 테스트를 위해 필요한 의도된 동작 — 게이트 우회 아님. |
| `U6a-2` | P1 | argv 노출은 실재. 단, 디스크 로그 절반은 `os.umask(0o077)`로 완화됨. |
| `U6a-3` | P1 | `nullcontext(mkdtemp())`이 서명 세대 복사본+strace 로그를 매번 누수. 단, 로컬 디스크 누수라 Medium. |
| `U6a-4` | P1 | `notify()`에 타임아웃·receipt regex·acceptance assert 없음. 단, 실패는 `queue_accepted: False`로 기록되고 `TimeoutExpired`는 전파됨. |
| `U6a-5` | P2 | `_run_observer_keeper`가 `FileNotFoundError`만 잡고 고정 `.tmp` 경로 정리 안 함. 단, 원자적 `temp.replace()`라 torn 파일 불가. |
| `U6a-6` | P2 | `/proc/<pid>/stat` 파싱이 `comm`에 `)`·공백 있으면 깨짐. 단, 대상 실행파일이 검증돼 `comm`은 사실상 고정명. (인용 행번호 정정: qualify_permissions는 232행.) |
| `U6a-7` | C | cmdline regex가 공백/따옴표/콜론에서 자름. 단, `busy()`가 prefix 매칭이라 잘린 하위경로도 fixture busy로 잡힘. (인용 행번호 정정: 85–86행.) |
| `U6a-8` | P2 | `$ref` resolver가 raw `KeyError`/`TypeError`, 사이클 검사 없음, 매 재귀마다 `check_schema()` 재실행. 단, 정적 스키마 하나만 검증. |
| `U6a-9` | P1 | `code not in old` 부분문자열 검사라 기존 Worker가 새 스크립트를 부분문자열로 포함하면 배포 건너뛰고 성공 보고. |
| `U6a-10` | P1 | `prepare()`가 `plan.json`·`info.json`·`environment-private.json`·`chatgpt.toml`을 seal에서 누락. |
| `U6a-11` | P1 | `finally` 블록이 검증/대기 없이 `SIGTERM` 전송, `/proc/*/cwd` 스캔이 자기 PID 미제외. |
| `U6a-12` | P2 | `os.environ['TMPDIR']` 폴백 없이 raw `KeyError` (TMPDIR 미설정 시). |
| `U6a-13` | P2 | `tail_rows()`가 동시 기록 trailing 행의 `JSONDecodeError` 미가드. |
| `U6a-14` | P2 | 자식 stdout 루프의 무가드 `json.loads()` — 비JSON 진단 행에 크래시. |
| `U6a-15` | P1 | `d3check.*` 임시 디렉토리·per-run Cargo 타겟 캐시 미정리. |

### U6b — Shared state migration, live-flow probes and auxiliary test clients

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U6b-1` | P1 | `finalize_plan`의 `thread/list` 단일 페이지 호출 + `activating` 상태에선 rollback 거부. |
| `U6b-2` | P0 | 파괴적 handoff가 RPC 확정 전에 일어남 + 재실행 거부는 양쪽 확인. 단, 페이징 절반은 과장 — `main()`이 반환값을 버리는 fire-and-forget 트리거라 잘려도 동작 무영향. (Argon은 참이라 했으나 직접 근거 우선.) |
| `U6b-3` | P2 | `Ws.send`에 64비트 WebSocket 길이 프레이밍 없음 (≥65536바이트 시 `struct.error`). 단, qual 전용 클라이언트 + 시끄러운 실패라 Medium. |
| `U6b-4` | P1 | `test_chat_model_transitions.py`를 어떤 `check.sh` 스코프도 안 돌리고, `full`이 provider 단위 테스트·CLI 검사 건너뜀. |
| `U6b-5` | P2 | `AppendWatch.__init__`이 `add_directories()` 실패 시 inotify fd 누수. 단, 실패 경로가 run을 abort시켜 영향 좁음. |
| `U6b-6` | C | `Connection: close` 미전송·TLS 소켓 `shutdown()`은 맞음. 단, `response.completed` 후 EOF까지 읽기는 의도된 동작 — 테스트가 `upstream_eof`를 단언함. |
| `U6b-7` | P2 | `/proc/<pid>/stat` 파싱이 `') '` 포함 comm에서 어긋나고 raw `os.kill` 사용, 자기 PID 미제외. |
| `U6b-8` | P1 | `upstream.send()`를 `readyState` 검사 없이 호출, SIGTERM/SIGINT가 `close()` 없이 종료. |
| `U6b-9` | P2 | 하드코드된 해시 데스크톱 에셋 번들 URL — 번들 재빌드 시 스크립트 깨짐. |
| `U6b-10` | N | `manage.py` 패치·`[history]` 절단 코드는 맞음. 단, 같은 스크립트가 `[history]`를 마지막 섹션으로 생성해 정상 흐름에선 잘릴 게 없고 `manage.py` 패치는 assert로 시끄럽게 실패. (Argon은 참이라 했으나 직접 근거 우선.) |
| `U6b-11` | P2 | `progress_match`가 extra distinct commentary를 안 세서 통과시킴. |

### U7 — GitHub Actions, publication, release inputs and installer

| Review ID | Decision | Concise mechanism / limitation |
| --- | --- | --- |
| `U7-1` | R | 핀·Base 변형 abort는 실재 (`a5063a4`가 `eba60a7a` 후손, diff는 generation `case`만 변경). 단, 문서화된 명령은 `--variant full`이고 구버전 `case`에 `full:full-v*` 매칭 arm이 없어 Full 경로는 동작 — 깨지는 건 Base 변형뿐. (Argon은 Critical 유지했으나 직접 근거 우선.) |
| `U7-2` | P1 | `candidate=false`인 base run이 `success`로 끝나 Full job 시작 + dead regex는 확인. 단, Full lane의 `matching_base` fingerprint 검사가 불일치 base를 시끄럽게 실패시켜 조용한 오배포 방지. (Argon은 High 유지했으나 직접 근거 우선.) |
| `U7-3` | C | prefix 검사 불일치 실재. 단, 현재 세대는 `core-v*`라 모든 검사를 일관되게 통과 — 레거시 전용 잠재 결함. |
| `U7-4` | P0 | `rewrite/rust-core` 하드코드 + `notify_acceptance`의 `github.ref` 가드 누락으로 main에 오발 디스패치. |
| `U7-5` | P1 | 미페이지네이션 `?per_page=100` 확인. 단, 인용된 `qualification_signal.py:68`은 `total_count` 불일치 시 raise라 silent truncation 사례 아님. |
| `U7-6` | P2 | `source-acceptance.yml`에 `msft-mirror-aosp` 폴백·`/system` cleanup 부재 확인. 단, ephemeral 러너라 `/system` 영향 제한적. |
| `U7-7` | P1 | draft 부분 업로드 wedge 확인 — 부분 에셋이 남은 draft는 이후 재시도에서 `length == 0` 검사 영구 실패. |
| `U7-8` | N | regex·github.io 베이스 확인. 단, `workflow_call` 전용 + 호출자 전무한 데드 코드. |
| `U7-9` | C | SPEC 절반만 성립 (인용 :172가 아니라 **:174** — Argon이 잡은 행번호 오류). README:866 절반은 과장 — `accepted-release-inputs.yml:4`에 schedule 트리거 여전히 있음. |
| `U7-10` | N | `syn` 이중 버전 공존 확인. 단, Cargo v4는 모호할 때 version을 기입하므로 문제 되는 형태의 lock 항목은 Cargo가 스스로 안 만듦. |
| `U7-11` | P2 | `Path(shutil.which(...))`를 argparse 기본값으로 평가해 바이너리 부재 시 `--openssl`/`--curl` 파싱 전에 `TypeError`. |
| `U7-12` | P2 | `workload.c`가 `getenv("TMPDIR")` NULL 검사 없이 `snprintf`에 전달, `curl`에 `--proto '=https'`·`--max-filesize` 누락. |

**Use rule:** Start each tasked finding with its spelled-out mechanism above, independently verify its current producer/consumer and negative case, then implement only the justified correction. Review confidence is not release admission; no attached archive is required to understand the queue.

## Required final product acceptance (separate from source repair)

After each affected source owner is qualified and the workflow's exact current
commit is accepted, use the **installed qualified Core** with a matching
Provider service, HOME, workspace and Native writer. Execute actual TUI
`/provider` → GPT-6 first and follow-up responses on the **same**
Native thread. Re-inject original SSE disconnect, HTTP 413, uncertain
execution, delayed recovery, stop/handoff and independent parallel work, with
identity/receipt/allow-deny/effect evidence. A synthetic result, catalog ACK,
CI pass or direct-profile diagnostic is not a substitute. Only afterward may
a separately authorized signed release/device activation be considered.
