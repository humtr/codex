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
