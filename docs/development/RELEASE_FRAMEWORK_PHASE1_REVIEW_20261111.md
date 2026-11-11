# Release framework — phase-1 independent re-review brief (2026-11-11)

**Status:** Non-normative finite review input. This is the updated,
self-contained execution orientation for the user's full 45-section
"Codex 릴리스 프레임워크 전면 통합" prompt. It is NOT a phase-1
approval, source implementation, release admission or replacement for
[product contracts](../../SPEC.md). Current observed receipts:
[CURRENT](../../CURRENT.md). Next steps and authorization:
[HANDOFF](../../HANDOFF.md). Retire this review brief when its approved
finite implementation plan and results have been integrated.

## Goal and strict two-phase boundary

Re-evaluate and improve the approved prior **W0–W14** release framework
based on actual current code, signed Core/Full, tools, ChatGPT Provider,
Native execution and provenance, account/model/auth boundaries, Manager
notifications and Android terminal focus, then implement the **approved**
integration with one developer authority. Prior design choices, source
review severity counts, historic model replies and this brief's
hypotheses are **not guaranteed facts**.

**Phase 1:** Read-only independent code, Git, installed state,
CI/signing, existing source/model history and package/owner inspection.
Deliver preferred architecture, exact W0–W18 DAG, module/source/test
LoC budgets, regression and Android proof matrix, alternative rejection
reasons, normative edit proposals, risk and rollback boundaries.
**Explicitly request user approval before phase 2.** No implementation,
normative document edit, PR merge, source-branch update, CI rerun,
signing, public promotion, service restart, arbitrary Stop/Stop Tree,
unverified ledger reset, authentication or SQLite/JSONL mutation
under the phase-1 prompt. The present HANDOFF/CURRENT review edits are
a **separately requested documentation-only preparation**, not W
implementation authority.

**Phase 2 after approval:** Implement bounded source/spec/test packets,
make coherent commits, integrate valid owned changes into one developer
branch, preserve historical consumers and signed assets, and report
receipt-backed local/synthetic/Native outcomes. Separate grants remain
mandatory for workflow dispatch, CI-triggering pushes, signing,
publication/index promotion, live device change, account/profile
migration, service restarts and historical DB repair.

## Verified baseline replacing obsolete assumptions

| Old plan statement | Current correction |
| --- | --- |
| Integration begins at `refactor/docs-contract-20261110` / `0aebdd7f` | Remote developer source is `rewrite/rust-core` `7b98296a`; local root was at older dirty `27103ec`. Do not overwrite either owner. |
| PR #29 pending integration (W7) | PR #29 is already merged as `afdfb8bd7e57`. W7 is source-integration complete; verify affected 0.167.1+ proofs and never rewrite signed Full163. |
| Signed LKG Full163 only | Signed 0.167.0 **Full163** and newer separately published **Core 0.167.1 S164** both exist. Neither proves the currently selected handset generation or a new Full release. |
| Positive 0.167.1 TUI pin test failure | Older `f9dfc3f` run failed; new `7b98296a` TUI run was in progress when read. Do not predeclare pass or fail. |
| W0–W14 documents already integrated | Local `3ef675ad` 13-document (+499/-6) plan remains in a separate worktree and is not integrated; review its actual `RELEASE_FRAMEWORK_IMPLEMENTATION`, `QUALIFICATION` and `UPDATE_DISPLAY` contents. |
| Native Busy root cause unknown | The user provided a concrete `write_stdin` continuation/`Opaque` settlement cause and successful ledger cleanup/Native execution; local fix and regression are verified as **uncommitted**, absent from remote integration. |
| "No Termux shell capability" | `local_context`, `local_execute` and `local_process` exist in Provider source, but are absent from the connected Bridge catalog; `local_execute` reserves the same shared claim and cannot serve as the emergency bypass. |
| `main` may be ordinary developer mainline | `main` currently has separate Git ancestry and owns accepted release/publication refs, last `dc693046`. Preserve its role pending the evidence-based A/B branch decision. |

## Preserve W0–W14: exact questions and corrected DAG

- **W0:** Inventory actual remote/Termux HEAD, installed signed
  generation, Base/Full public heads, source/candidate/approved CI runs,
  provider/auth/model/home, native effects, terminal hooks, local 3ef
  plan, dirty worktrees and pending authorizations.
- **W1:** Identify independent Core, Runtime Bundle (runtime, Code
  Mode host and inseparable helpers), Manager, TUI, Provider input
  fingerprints. Include upstream commit/archive, target/ABI,
  adaptation/patches, Cargo/native dependencies, toolchain, generated
  protocol, interface version and actual prior successful proof.
- **W2:** Remove duplicate Android Core production only after a
  canonical producer/consumer DAG is proved cycle-free; preserve
  source, Core build, Bionic smoke and negative gates. No assumption
  that identical input digest proves equal execution or valid CI.
- **W3:** Manager independent artifact, qualified Core Manager
  interface and failure/missing-Manager negative proofs. Base must not
  depend on Manager/TUI/Provider.
- **W4:** Full recomposition proves identity and compatibility of
  selected Core/Runtime/Manager/TUI, Retain and digest-bound exact
  successful receipts. Full composition remains a real qualification
  even if component bytes are reused.
- **W5:** Define per-upstream independent `rN` module revisions,
  separate Full combination identity, signed generation and global
  Base/Full issuance sequence. Reserve signed sequences atomically
  including unpublished issued candidates. Sequence, SHA, digest,
  signing identity and UI version have different roles.
- **W6:** Map Retain stable functional IDs to actual executed
  upstream 0.167.1 PASS results, preserve mutation-negative boundary.
- **W7:** Source-merged #29: read/compatibility/rollback negative
  verification only. The compatibility `current` alias is derived,
  never a trust/activation authority.
- **W8:** Isolate real `pin_action_updates_shared_section_state`
  positive failure and exact TUI current-source qualification.
  Do not skip/rename the assertion merely to pass.
- **W9a:** Study the actual local 3ef draft normative contracts,
  including QUALIFICATION and UPDATE_DISPLAY, then propose minimal
  SPEC/annex/INDEX/TEST_DEPENDENCIES adjustments. Do NOT merge
  provisional normative documents before phase-1 approval.
- **W9b:** After implementation, reconcile real code/proofs to
  one factual CURRENT and finite HANDOFF; remove stale instructions,
  preserve only necessary compatibility and Git history.
- **W10:** Explicitly independent signed CI, candidate admission,
  public readback, Base/Full index promotion and controlled handset
  activation/rollback. Historical full 0.167.0 remains immutable.
- **W11:** Compact truthful `codex update` default progress, no-op,
  changed-component and failure output. Preserve actual `--details`,
  signing, rollback and no-publish-before-gate behavior.
- **W12:** Core-owned offline `codex termux version` for Base without
  Manager; same typed authenticated local component view as W11,
  default/`--details`/`--json`, no duplicated registry.
  `codex --version` and `-V` remain upstream-byte-identical.
- **W13:** Expected default Termux no-bwrap is information, whereas
  explicit unsupported sandbox requests and real SQLite/JSONL
  discrepancies remain distinct errors.
- **W14:** Exact CLI/version/update/Doctor positive and negative
  regressions including terminal cleanup, rollback, disconnected
  input, non-TTY, malformed metadata and old signed readers.

**Cycle-breaking DAG:** W1 defines component inputs. In parallel,
W6 Retain mapping, isolated W2 Core and W3 Manager, W13 Doctor,
and read-only W7 verification can proceed under separate file/writer
owners. W5 first defines metadata schema and per-module revision
input contract; W12 consumes that schema and W11 consumes W12's
single parsed view. W4 then consumes W2/W3/W6/W8 and W5 schema
to produce composed proof. Only **after** W4 may W5 commit a final
signing sequence/admission against its exact immutable inputs.
W14 verifies affected command contracts. W9b closes verified docs.
W10 is the separate hard-stop release/device gate. No circular
"sign first to qualify composition" dependency.

## Additional independently owned W15–W18

| ID | Owning change/relationship | Precedence and required test |
| --- | --- | --- |
| **W15** Native shared-effect safe settlement | Provider `native_claims.rs` and original broker/host/tool receipts. Incorporates known local uncommitted 4-line behavior correction plus regression test after rebase, then review Controller/Owner loss, real process exit, ACK unknown and unrelated work. **Before** W15 end-to-end Native effects, W17 live model and W16 headless execution. Tests: 15 claim cases, actual B/C/D receipts, Stop/Stop Tree, uncertain-result no replay, safe conflicts and independent jobs. |
| **W16** Notification-to-terminal continuity | Manager notification/focus, terminal/RunCommandService, TUI resume/observe, Provider work identity. Existing exact terminal selection first; create once for headless; tap and auto-open race/idempotency; Android foreground/background restrictions; correct original thread without new inference or authority transfer. Can develop UI/Manager synthetic path in parallel with W15; real headless Native acceptance follows restored availability. |
| **W17** Provider account/model/auth and compatibility | Actual `registration`, `accounts`, TUI routing and saved Native model/provider metadata. Contrast `chatgpt_desktop` vs `chatgpt_linux` plus retaining ID/display-only. No unsupported model-prefix rewrite; validate Linux AppHost auth independent from ordinary Codex `auth.json`, original Chat/account/workspace rights, attached vs submitted inference. Static tests can precede W15; real model roundtrip follows it. |
| **W18** Termux host control/recovery entrance | Bridge + Provider `local.rs`, host execution/permission owner, shared-claim admission and separate service health. Existing standalone `local_*` is not sufficient while sharing the claim ledger. Design read-only diagnostic outside Native execution claims and a narrowly authorized evidence-backed recovery operation that functions during stuck Native admission. Requires W0 security/architecture proof; must not create silent unrestricted second writer. |

**New planning LoC must be broken out** by actual source owner,
test/fixture, changes and shared overlap. The original **W0–W14
1,370–3,020 implementation LoC** and **500–800 documentation LoC**
are reference estimates, not a new total. The earlier Android
terminal proposal was **260–520 product + 230–420 test LoC**,
not an automatically additive unique budget. For W15, the
locally verified first fix is a small focused diff plus one
24-line regression but wider guarantee is not yet scoped;
W17 may be **0** implementation lines if preserving the
existing Provider ID is correct. Re-estimate W18 only after
the authenticated host-boundary design and avoid double counting
W15 with R05 or W16 with W3/TUI.

## Termux host shell: primary discovery is justified; bypass is conditional

**Current fact:** Source `providers/chatgpt/src/local.rs` implements
`local_context/local_execute/local_process` only with a configured
`execution.standalone_permission_profile`. It launches process I/O
via prepared Native/Core services and **reserves the identical
`SharedNativeGuard::offer_external` ledger** before execution.
The connected `@TermuxBridge` catalog did not expose those tools.
This is an alternate local development workflow, **not** a
guaranteed break-glass shell when the Provider/Native ledger wedges.

**Preferred phase-1 recommendation:**

1. Put a **host environment and resource diagnostic entrypoint**
   first in Bridge discoverability for Termux users: identity,
   connection, process/owner/lease observation, Core/Provider
   status, claims/receipts and terminal availability, with strict
   redaction and no inference or Native work allocation.
2. Reuse `local_*` for normal Core-profile process execution when
   its permission and shared-effect model applies. Do not silently
   bypass arbitrary Native conflict protection for normal writes.
3. Add the smallest **independently authenticated host recovery
   path** that does not require successfully acquiring a Native
   turn/Controller/effect. A repair can only target a proven
   stale claim after process-PID/start-time and original
   result/ACK/owner checks. Gate any mutation through an explicit
   user-allowed capability; hold the same file lock, preserve a
   private backup and use atomic revision. Preserve uncertain
   effects; no global `rm`/kill/force-stop.
4. If truly general-purpose direct Termux shell is necessary for
   authorized host maintenance, use a separately permissioned
   trusted local service / user-selected execution grant; never
   make that authority available merely from a ChatGPT model
   connection. Bound scope by Termux UID and actual caller,
   command/argv authorization, execution time, output size and
   audit receipts; block credential exfiltration/cross-account
   access. The shell must remain operable when Native claims
   are stuck but may not become another Native writer.
5. Prove with a **deliberately orphaned claim**: diagnostic still
   works; ordinary competing Native tool still safely denies;
   unauthenticated caller and unapproved repair deny; valid
   proof-backed recovery settles only the stale identity;
   subsequent original Native tool executes; concurrent real
   effect never loses its ownership. Also test Provider/AppHost
   offline and Android/Termux restart behavior.

Whether this is exposed as a first-listed `termux_status` plus
`termux_recover` (recommended) or a broader audited `termux_exec`
is a phase-1 architecture decision, not an automatically approved
permission escalation.

## Provider, Native, Manager and Android boundaries

**Authentication:** Android/ChatGPT web, authenticated Linux app,
and Termux Codex execute in separate environments. Distinguish
original Chat account/workspace/conversation binding, Linux app
inference, Codex CLI `auth.json` for ordinary OpenAI Provider,
Provider local service token, Native execution-home ACL, Core
approval and original tool receipt. `401 token_revoked` previously
occurred on an **ordinary** OpenAI route, not proof the independent
ChatGPT Provider failed. Never force all accounts to equal.
`attached` inference need not make a second Linux AppHost request;
`submitted` inference does require actual host/model eligibility.

**Model naming:** Provider ID and display name are not stored model
ID. Study actual `model_provider`, Native catalog namespace,
`chatgpt-desktop/app/gpt-6` and prior aliases as consumed by
upstream TUI/session formats; never manufacture `chatgpt-linux/gpt-6`
or replace retained thread settings without demonstrated compatibility.

**Native effects:** distinguish accepted input, original Native
thread/turn/tool IDs and controller, shared `Opaque` reservation,
verified process completion, ACK observation, stream lifetime,
stop receipt and owner-generation fencing. Per-work `active=[]`
is not global lease proof. Recover only specifically settled
effects; never replay unknown calls or delete old histories.

**Notification and Activity:** use Manager's existing click
`__terminal-focus-v1` and RunCommandService-related terminal
creation ownership; recheck modern `frontend/tui` code instead
of relying on retired `frontend/manager-tui` paths. Selecting a
terminal, observing an active Native thread and **resuming a
finished thread** are different permissions. Require actual
visible Activity/session proof, correct home/profile and
idempotent click/auto-display races; abide by Android background
launch restrictions.

## Source and release branch decision

Read actual `main` accepted-release inputs, public signed Base/Full
indexes, source/frontend pins, trigger permissions, branch
protection, tag/asset consumption, signed/unsigned gates and
upstream automation before choosing:

- **A:** `main` release/publication authority plus one
  `rewrite/rust-core` sustained development source (strong
  present provisional recommendation because current main
  has separate ancestry and publication orchestration).
- **B:** `main` is sustained development with independent
  signing/admission; requires a tested atomic migration of
  every workflow, CAS/pin and existing consumer.

Recommend exactly one with evidence and user approval. Integrate
actual unique valid source changes from other branches only after
owner verification; record merged, obsolete, live-owned or
retain-for-release classifications and do not mass-delete branches.
Never replace the local dirty checkout or bring
`providers/chatgpt/src/accounts.rs` into source without its owner.

## Proof and report gates

**Phase-1 report:** source/installed/CI baseline; W0–W14 plan
assessment; Provider ID/auth/model and stored-thread boundary;
actual Native Busy producer and operating rescue vs source fix;
Termux host diagnostic/recovery need and threat model; Manager
notification/TUI display plan; one main/development choice;
W0–W18 revised DAG; per-work source/test LoC and isolation;
normative/document file owners and actual proposed edits;
recallable CI/test receipts, negative cases, risks and rollback.
Present one preferred design and **ask for phase-2 approval**.

**Phase-2 tests (after approval):** source and exact output +
negative/mutation, originally admitted tool receipt, real GPT-6
same-thread two-turn, SSE/413 and no duplicate result, Owner/Stop/
Stop Tree/opaque conflict/independent work, portable provider auth
and original Chat account mismatch denial, Android notification
tap and terminal focus/create, Bionic/installed CLI and
update/rollback, pre-existing signed-asset immutability. Previous
success may be reused only for identical source/input/meaningful
proof boundary; running/failed/unexecuted tests remain so.

Preserve Android sdcard, untracked configs, accounts, installed
generations, auth/profiles, transcripts, journal and actual active
writers throughout. Data-doctor SQLite/JSONL repair is independent
and **read-only** until separately authorized. No stage may
reinterpret a request ACK as a command/result/Android focus success.
