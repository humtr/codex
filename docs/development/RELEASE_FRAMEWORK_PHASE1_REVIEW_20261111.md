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
| Integration begins at `refactor/docs-contract-20261110` / `0aebdd7f` | Remote developer source is `rewrite/rust-core` `7b98296a`; local root is now `e4f9ab5d` with a committed Native patch plus protected unrelated untracked files. Do not overwrite either owner. |
| PR #29 pending integration (W7) | PR #29 is already merged as `afdfb8bd7e57`. W7 is source-integration complete; verify affected 0.167.1+ proofs and never rewrite signed Full163. |
| Signed LKG Full163 only | Signed 0.167.0 **Full163** and newer separately published **Core 0.167.1 S164** both exist. Neither proves the currently selected handset generation or a new Full release. |
| Positive 0.167.1 TUI pin test failure | Older `f9dfc3f` run failed; [the newer `7b98296a` TUI Native qualification](https://github.com/humtr/codex/actions/runs/43013363622) completed **successfully**. This is CI, not installed device/model proof. |
| W0–W14 documents already integrated | Local `3ef675ad` 13-document (+499/-6) plan remains in a separate worktree and is not integrated; review its actual `RELEASE_FRAMEWORK_IMPLEMENTATION`, `QUALIFICATION` and `UPDATE_DISPLAY` contents. |
| Native Busy root cause unknown | The `write_stdin` continuation/`Opaque` settlement defect is fixed in **committed local `e4f9ab5d`**. A pre-fix operating Provider caused a repeat, then was replaced/restarted; Native B/C/D and the 15-case claim regression passed. Remote integration and other effect lifecycles remain open. |
| "No Termux shell capability" | The independently implemented **one-tool `termux_shell`** is committed locally as `c0a81898`, installed in a separate Termux MCP worker process and exposed as the **first of 18 tools** in this very Chat. A real direct shell call succeeded with exit 0; its source is not yet integrated remotely. |
| `main` may be ordinary developer mainline | `main` has separate Git ancestry/publication authority, last `b2567063`. Preserve this real owner pending the evidence-based A/B branch decision. |

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
- **W8:** The older `pin_action_updates_shared_section_state` positive
  failure must be distinguished from the **success** of the exact
  `7b98296a` TUI Native CI run. Do not skip/rename the assertion;
  installed same-thread two-model-turn acceptance remains separate.
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
  changed-component and failure output. Remove duplicated keep-current
  choices (`1`/`q`) and avoid applying the Full enhanced-TUI version
  requirement to Core-only build/selection. Preserve actual `--details`,
  signing, rollback and no-publish-before-gate behavior.
- **W12:** Core-owned offline `codex termux version` and explicit
  **Core / Full** user-facing edition distinction; Core execution
  requires a qualified upstream Runtime Bundle but not Manager or the
  enhanced TUI. Full's optional incompatible TUI must not block a
  qualified Core-only 0.167.1 path. Reuse W11's authenticated local
  component view, no duplicate registry; retain upstream-byte-identical
  `codex --version` / `-V`.
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
| **W15** Native shared-effect safe settlement | Local `native_claims.rs` `e4f9ab5d` is **committed**, the specific leak was operationally settled and the Provider restarted on repaired code; B/C/D Native receipts and 15/15 claim tests are accepted. **Remaining**: integrate exact source into remote development and verify deployed artifact provenance, Controller/Owner loss, ACK unknown, unrelated/parallel work, Stop Tree and safe conflict isolation. Native availability is restored for ordinary acceptance, not globally proved for all fault states. |
| **W16** Notification-to-terminal continuity | Manager notification/focus, terminal/RunCommandService, TUI resume/observe, Provider work identity. Existing exact terminal selection first; create once for headless; tap and auto-open race/idempotency; Android foreground/background restrictions; correct original thread without new inference or authority transfer. Can develop UI/Manager synthetic path in parallel with W15; real headless Native acceptance follows restored availability. |
| **W17** Provider account/model/auth and compatibility | Actual `registration`, `accounts`, TUI routing and saved Native model/provider metadata. Contrast `chatgpt_desktop` vs `chatgpt_linux` plus retaining ID/display-only. No unsupported model-prefix rewrite. New MCP connector refresh and shell execution **do not prove** the Linux app is authenticated as `janmori101`; establish real app account/workspace ownership separately from `wrlab`, ordinary Codex `auth.json`, attached/submitted inference and Native execution-home. |
| **W18** Independent Termux MCP shell | **Operating implementation already complete** in local `feat/termux-mcp-shell-20261111` / `c0a81898`: a single first-listed `termux_shell` is served by its own Termux app-UID MCP worker, independent of Native/Core/effect claims. Broad shell capability follows the user's actual task instructions; recovery is its initial focus but not the exclusive use. Reused `call_id`/same arguments resolve original shell results; there is no second receipt tool or separate model/history ledger. Existing shared annotations remain unchanged. The Linux app refreshed 17→18 actions and this Chat executed the actual shell. **Remaining**: approved source integration, actual `janmori101` principal validation and fault/owner tests without recreating the connector. |

**New planning LoC must be broken out** by actual source owner,
test/fixture, changes and shared overlap. The original **W0–W14
1,370–3,020 implementation LoC** and **500–800 documentation LoC**
are reference estimates, not a new total. The earlier Android
terminal proposal was **260–520 product + 230–420 test LoC**,
not an automatically additive unique budget. For W15, the
locally committed first fix is a focused behavior change plus its
regression; it is operating-recovered but remotely unmerged and
wider effect guarantees are not yet scoped;
W17 may be **0** implementation lines if preserving the
existing Provider ID is correct. Re-estimate W18 only after
the authenticated host-boundary design and avoid double counting
W15 with R05 or W16 with W3/TUI.

## Termux host MCP shell — proven operating entrance and remaining source boundary

The user selected **one first-class, broadly capable Termux app-UID
shell tool**, `termux_shell`, with **recovery as the initial objective,
not a permanent purpose limitation**. It must be usable while Native
claims or Core/Provider model execution are blocked. It does not take
ownership of the Codex reasoning, approval, transcript or Native effect
ledger, nor does it override an actual user instruction. The tool is
listed first, retains existing common MCP tool annotations exactly and
needs no separate `termux_shell_receipt` action. Matching stable
`call_id` and argument identity returns its stored original shell
receipt without executing the command again.

This is **implemented and operating**, not a hypothetical "read-only
`termux_status` first, `termux_recover` second" design. Source:
local `c0a81898`, `providers/chatgpt/src/termux_shell.rs`,
`termux_mcp_worker.rs`, and the stdio routing in `runtime.rs`.
Running binary lives in the Termux-only
`~/.local/lib/codex/termux-mcp-worker/generations/` tree and the
active `mcp-stdio` entrypoint is separately backed up. The
Provider's older `local_execute` still uses
`SharedNativeGuard::offer_external` and is **not** this independent
MCP shell.

Operating evidence: the local Termux AI reported a controlled
`codex-chatgpt-tunnel` restart, verified Linux ChatGPT app
`refresh_actions` `ok: true`, query cache refresh and **17 → 18
actions**. Independently, the active stdio wrapper returned one
`termux_shell` first and all original Codex tools, and **this Chat**
directly called `@TermuxBridge.termux_shell`:
`DIRECT_TERMUX_MCP_OK`, exit **0**. All these are stronger
proof than a local build alone; they are not proof of a live
`janmori101` account without checking the current authenticated
Linux app. The 6ad4… connector line and historical
`janmori101` 6ae9… line must be distinguished by actual identity.

**Still required:** integrate the existing local implementation into
the single formally approved development baseline; preserve original
Provider/Native ownership and existing MCP metadata; validate
fail-closed caller/account boundaries and independent command access
during an intentionally stuck Native admission, including original
receipt and no duplicate effects. Do not repeat finished tool
installation as an unresolved prerequisite. No new permanent shell
manager, duplicate Native execution journal or duplicate connector
is called for by this accepted design.

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
