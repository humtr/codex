# Handoff — release framework independent review and source integration

Updated: 2026-11-11 KST. This is the **single unfinished-work handoff**.
Product obligations are in [SPEC](SPEC.md) and its [functional annexes](docs/spec/INDEX.md);
agent behavior is in [AGENTS](AGENTS.md), document authority in
[DOCUMENTATION](docs/DOCUMENTATION.md), verified facts and limits in
[CURRENT](CURRENT.md). Earlier `PRO_HANDOFF.md` content is historical and not
an alternative instruction source.

Read the bounded [revised phase-1 implementation-review brief](docs/development/RELEASE_FRAMEWORK_PHASE1_REVIEW_20261111.md)
with the user's full 2026-11-11 W0–W14 release-framework prompt. The brief
updates that prompt against the verified integration baseline and names W15–W18.
The user's original substantive requirements remain in scope; a short
handoff does **not** silently drop them.

## Source and ownership gate before effects

- Official product development reference: remote `rewrite/rust-core` at last
  checked `7b98296a411a2c435bb9f34ee51f286f33f3491b`. Query live HEAD,
  source changes, actual local checkout, competing worktrees and release
  authority before implementing. `main` is a separate publication/approval
  line, last read at `dc6930465ce309ea13b69ab4057a07a446cc84e1`.
- `integration/codex-native-20261110` at `a55d5bb8` was already integrated.
  [PR #29](https://github.com/humtr/codex/pull/29) was merged as
  `afdfb8bd7e57`; W7 is **done as source merge**, not pending. Its alias
  fix was not in signed 0.167.0 Full163; remote signed
  `core-v0.167.1-7b98296-s164` uses later source. Signed publication,
  installed generation and current device choice remain distinct facts.
- Original local `plan/release-framework-20261111` worktree
  `/data/data/com.termux/files/home/prj/codex-release-framework-plan-20261111`
  remains clean at `3ef675ad721c0a08a50891394cf03df3abb66b2d`:
  13 files, +499/-6 documentary lines, including
  `docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md`,
  `docs/spec/delivery/QUALIFICATION.md`, and
  `docs/spec/interfaces/UPDATE_DISPLAY.md`. It was based on
  `refactor/docs-contract-20261110` / `0aebdd7f`, is **not** in the
  remote integration HEAD, and must not be replaced or silently merged
  before phase-1 review/approval. Preserve its detailed W0–W14 DAG,
  source owners and input/proof contracts as review inputs.
- The primary Termux checkout `/data/data/com.termux/files/home/prj/codex`
  was **not at remote HEAD** during the read-only preflight: local
  `rewrite/rust-core` was at `27103ec06b1c` and had modifications to
  `providers/chatgpt/src/native_claims.rs` plus untracked
  `ais.config.toml`, `chatgpt.config.toml` and
  `providers/chatgpt/src/accounts.rs`. Do not reset, clean, checkout,
  overwrite, stage or commit those assets. Local worktree branches are not
  evidence of integrated remote changes.

## Two distinct authorization stages

**Phase 1 — independent read-only revalidation and final plan.**
Read actual code/owners, local plan, signed refs/CI and device facts when
accessible. Decide architecture, exact change owners, preconditions, source
and test changed LoC (not hours), compatibility/rollback strategy, and the
full W0–W18 dependency DAG. Report a single preferred recommendation with
confirmed facts, source-only inferences and untested boundaries. **Obtain
the user's approval before phase-2 implementation.** This handoff/document
review is separately authorized documentation and is not that approval.

Before phase-2 approval: **no implementation or normative edits**, PR merge,
developer-ref advance, CI dispatch/re-run, signing/publication, device
activation, service restart, auth/profile/session rewrite, arbitrary Native
Stop/Stop Tree, or uncontrolled ledger purge. Do not treat a prior CI success
or this documentation commit as a release acceptance.

**Phase 2 — authorized implementation.** On approval, use the elected single
development authority and isolated per-owner worktrees, implement the
approved changes, focused negative/recovery tests, and cohesive commits.
Serialize shared source writers, real Native writers and integration merges.
A phase-2 code approval **does not** authorize CI dispatch, signed release,
public stable promotion, physical device replacement or SQLite/JSONL repair.
Pushes into `rewrite/rust-core` can automatically trigger source acceptance
and later publication; inspect this effect and obtain independent authority
before such a push.

## Work plan — preserve all prior W0–W14

- **W0:** Fresh inventory of public release/installed device, unsigned CI,
  Core/Manager/TUI/Provider, authenticated Provider and Native work, remote
  vs local branch/worktree ownership. Compare Full163, Core S164 and exact
  selected device generation; do not conflate.
- **W1/W2/W3:** Complete component input/proof fingerprints; one canonical
  Android Core production result; independently qualified Manager and
  Base/Full separation. No loss of affected Core/API/ABI/negative checks.
- **W4/W5:** Composed Full proof and per-module `rN` / globally unique
  signing sequence. Break the metadata-shape → composition → final-sign
  admission dependency cycle. Never retrospectively rename signed 0.167.0.
- **W6/W8:** Retain functional-ID mapping and the real 0.167.1 TUI
  `pin_action_updates_shared_section_state` positive assertion. Its past
  failure is not an intentional mutation-negative success; refresh status
  of exact-source TUI qualification and repair without skipping tests.
- **W7:** **Already source-merged** Core derived `current` alias (#29);
  only verify current-source and install/rollback compatibility. Never
  backport into immutable Full163 or assume Core S164 was activated.
- **W9a/W9b:** Review local `3ef675ad` normative annexes before approved
  integration; finish all owning SPEC/INDEX/TEST_DEPENDENCIES links and
  accurate CURRENT/HANDOFF only after implementation. Do not inflate
  top-level SPEC or agent instructions.
- **W10:** Independent final CI, signed/public release and device gate.
  No grant to run those actions during phase-1 or automatically in phase-2.
- **W11/W12/W13/W14:** Compact truthful `codex update` default/details,
  Base-capable offline `codex termux version` while preserving byte-exact
  upstream `--version`, accurate expected no-bwrap Doctor classification,
  and retained CLI/Doctor/update/rollback regression oracles.

**New independent owners proposed for phase-1 evaluation:**

- **W15 — Native effect settlement, recovery and availability.** The
  2026-11-11 local incident is root-caused and operationally reclaimed,
  but its `native_claims.rs` regression fix is **uncommitted / absent from
  remote HEAD**; preserve it, verify against current source, then address
  broader Stop/Owner-loss/ACK unknown, unrelated work availability and
  bounded Busy diagnostics with real receipts.
- **W16 — Termux notification / terminal binding.** Reuse existing
  Manager terminal focus/create paths and TUI's actual resume/observe
  contract. Verify exact existing terminal selection, headless thread
  display, idempotent clicks, opt-in display, Android Activity outcome,
  owner/home/account isolation and no new Native inference.
- **W17 — Provider identity, model and auth separation.** Determine whether
  to preserve `chatgpt_desktop` or add a qualified alias; distinguish
  Linux-app inference auth, upstream CLI auth, original Chat account/branch
  ownership and Native writer authority. Do not rewrite existing model IDs,
  `model_provider`, stored thread choices or credentials without proof.
- **W18 — Independent Termux host diagnostic/recovery entrance.** The
  source already implements standalone `local_context`, `local_execute`,
  `local_process`, but currently exposed `@TermuxBridge` has no such
  tools, and `local_execute` itself calls
  `SharedNativeGuard::offer_external` using the same ledger as Native
  calls. Merely listing it first **cannot** recover a stuck shared claim.
  Investigate a first-class, authenticated **out-of-band host diagnostic**
  plus narrowly authorized recovery path that works when Native admission
  is stuck, without creating an unrestricted second Native executor.
  See the revised brief for threat model, proposal and acceptance.

Cross-cutting branch decision belongs to W0/W9 (one regular development
source, `main` vs development/approval responsibility), not a permanently
competing third source branch. Audit divergent or local-only work by unique
behavior and ownership; archive/retire only when no live consumer or
unmerged required change remains.

## Restore R01–R10 review ownership and dependency boundaries

Read the self-contained
[2026-11-10 triage and LOC work queue](docs/development/REVIEW_ACTIONS_20261110.md).
It contains the full U1–U7 findings, producer/consumer matrix, source and
test estimates, staged S0–S6 ordering, physical writer/socket/claim isolation
and release gates. These are **review hypotheses, not automatically open
bugs**; re-assess on the current source. Prior handoff details are retained:

1. **R01:** CI notifier/branch/ref/SHA authority (U7-4/U6b-4).
   `c19b61bb` changed notification admission; verify the real remaining
   negative matrix instead of reimplementing it or dispatching on guesswork.
2. **R02:** Authenticated MCP control calls (U3-1); reject unknown loopback
   callers while retaining valid Native/service callers.
3. **R03:** Core rollback crash safety, secret redaction and explicit
   Termux sandbox/CLI parsing (U1-2/4/6); never purge valid rollback.
4. **R04:** Original input/result/effect journal continuity across
   disconnect, SSE/413 and uncertain delivery (U4-3 et al).
5. **R05:** Shared opaque claims, continuation, capacity and independent
   safe execution (U4-6/U3-4); consume the proven W15 regression fix.
6. **R06:** TUI one-shot re-entry, bounded/cancelable outer waiting and
   draft/settings identity (U5-1/4); do not lose accepted input.
7. **R07:** Non-destructive, transactionally proved migrated-state handoff
   (U6b-2); never test against live user homes.
8. **R08:** Release retry, Base online installer and signed input safety.
9. **R09:** Provider qualification, credential isolation and fixture
   producer ownership.
10. **R10:** Remaining substantiated P1, with P2/conditional findings
    only when actual consumers make them necessary.

The previous R01–R09 preliminary implementation/test budgets were
2,410–5,480 changed LoC (with conditional R10: 2,630–5,990).
They are a **different work partition** from W0–W14 and must not be
summed mechanically. Re-estimate after deduplicating actual shared files.

Order source editing by real owner, not worktree count: only one
writer/index for a checkout; separate Core/Provider/Native identity,
owner-private short socket roots (complete encoded path <=107 bytes),
separate build targets and no shared unknown opaque claims. Parallel
source reading is not proof of safe parallel Native effects.

## Acceptance work, separately evidenced

1. **Preflight:** Verify the installed `$PREFIX/bin/codex` dispatcher,
   actual active generation, Provider service, HOME/CODEX_HOME, original
   thread/writer, workspace, canonical socket path, user/profile and
   exact source. Never mix isolated Core with operating Provider.
2. **Real model:** Installed/qualified TUI `/provider` GPT-6 **two
   completed turns on the same original Native thread**; do not count
   picker success, synthetic Provider output or old `--profile chatgpt`.
3. **Recovery:** Fault inject SSE loss and HTTP 413; preserve original
   turn/call/result/approval/effect without duplicate execution, even
   if AppHost disconnects. Verify Stop/Stop Tree and Owner loss separately.
4. **Native effects:** Repeat independent real tool execution (write,
   compile, run, edit, recompile, run), original stdout and exit codes;
   settled reuse and safe overlapping/nonoverlapping work. Distinguish
   work-level `active=[]` from global physical reservations.
5. **Terminal:** From AppHost/Bridge headless work, receive a Stop
   notification, tap to focus a live exact terminal or create exactly
   one terminal bound to **the same** thread; prove Android focus,
   duplicate tap/auto-open behavior, background restrictions and
   correct account/home. Display must never imply a new inference.
6. **Release/docs:** Reuse exact-qualified artifacts only with source,
   build-input, proof and composition identity; verify all changed
   modules, contracts and negative cases. `git diff --check`,
   documentation dependency and link checks, then exact authorized
   CI/unsigned vs signed/public vs device gates **separately**.

Use the user's original 45-section instruction and revised review brief
for the full phase-1 report topics and phase-2 success criteria.
Report real commit/receipt/failed boundary and request phase-2 approval.
Retire items from HANDOFF on accepted proof; preserve history in Git,
not another permanent chronicle.
