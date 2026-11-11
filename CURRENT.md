# Current product state

Updated: 2026-11-11 KST. **Read-only evidence, status and limits only.**
Product contracts: [SPEC](SPEC.md) and owning annexes; pending
work: [HANDOFF](HANDOFF.md). Statements from another AI's device
report are explicitly distinguished from direct source/CI checks.
This document authorizes no CI, deploy, cleanup or user-data mutation.

## Exact source, branches and documentation

- Remote product development branch `rewrite/rust-core`:
  `7b98296a411a2c435bb9f34ee51f286f33f3491b` at the last source
  readback. `integration/codex-native-20261110` at `a55d5bb8` is already
  in its ancestry; [PR #29](https://github.com/humtr/codex/pull/29)
  merged as `afdfb8bd7e574c749132254c55cd3f66204d3289`. This
  is a source merge, not a newly installed Full image.
- `main`, the separate release/publication authority, last read at
  `dc6930465ce309ea13b69ab4057a07a446cc84e1`.
  Do not merge the separate Git histories by assumption.
- The actual primary **Termux local checkout**, read-only inspected
  through a real Native tool response on 2026-11-11, was at local
  `rewrite/rust-core` `27103ec06b1c16c07b6a245d4b4542f8248cc5b4`,
  not current remote HEAD. It held a modified
  `providers/chatgpt/src/native_claims.rs` and untracked
  `ais.config.toml`, `chatgpt.config.toml`,
  `providers/chatgpt/src/accounts.rs`. These assets were not touched
  by the read-only inspection; their contents/owners are not thereby
  authorized for integration.
- The independent clean local plan worktree
  `/data/data/com.termux/files/home/prj/codex-release-framework-plan-20261111`
  is at `plan/release-framework-20261111` /
  `3ef675ad721c0a08a50891394cf03df3abb66b2d`.
  Its actual last commit modifies 13 documents (+499/-6):
  `docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md`,
  `docs/spec/delivery/QUALIFICATION.md`,
  `docs/spec/interfaces/UPDATE_DISPLAY.md` and ten related contracts.
  The code/CI was not changed by that commit. These three main plan
  documents are **not yet in current remote development HEAD**.
  Preserve and independently review their detailed content before
  phase-2 normative integration.
- Current documentation system still separates top-level `SPEC`,
  29 scoped functional annexes, `AGENTS` behavior, `DOCUMENTATION`
  policy, `INDEX`, `TEST_DEPENDENCIES.toml`, `CURRENT` facts and
  `HANDOFF` unfinished acceptance. This review does not replace
  that architecture with an old `PRO_HANDOFF.md` document.

## Signed and published states — not device activation

- `release/upstream.json` in development source selects upstream
  **Codex 0.167.1**, source `0a94398fa60c4fac0e7659e4213a0a36e613dd46`.
- Signed [Full163 0.167.0](https://github.com/humtr/codex/releases/tag/full-v0.167.0-c19b61b-s163)
  was published with immutable `full-v0.167.0-c19b61b-s163`.
  PR #29 reports the device's derived `core/current` compatibility
  alias was corrected to signed Full163 and Full159/Full153 were
  retained. **No independent current handset activation readback**
  is established by this document review.
- Signed [Core S164 0.167.1](https://github.com/humtr/codex/releases/tag/core-v0.167.1-7b98296-s164)
  was published at 2026-11-11 11:40 UTC from `7b98296a`.
  Core S164 is **not** a new Full163 or proof of a physical upgrade.
  It postdates the 2026-11-10 provisional W0–W14 planning baseline.
- The derived `core/current` link is not an activation, signing or
  rollback authority. The PR #29 source fix is **not in Full163**;
  older signed bytes and rolling rollback entries remain immutable.

## Exact-source qualification observations

| Source / boundary | Observed result | Not demonstrated |
| --- | --- | --- |
| `7b98296a` [Source Acceptance](https://github.com/humtr/codex/actions/runs/43012469220) | **success** | Device/model |
| `7b98296a` [Core qualification](https://github.com/humtr/codex/actions/runs/43013017085) | **success** | Full/TUI/installed |
| `7b98296a` [TUI Native qualification](https://github.com/humtr/codex/actions/runs/43013363622) | **in_progress** at last check | Completed outcome |
| `f9dfc3f` [TUI Native qualification](https://github.com/humtr/codex/actions/runs/42945712023) | **failure**, positive `pin_action_updates_shared_section_state` case | A successful replacement result |

Remote docs review may update CI status after completed readback.
A new documentation-review commit or PR does not inherit a parent's
CI result. Exact-source publication is not installed-flow acceptance.

## Native Busy: proved local incident, resolution and integration gap

**User-provided independent Termux AI operating report (2026-11-11):**

- Direct Termux shell `printf 'DIRECT_TERMUX_EXEC_OK\n'` returned
  `DIRECT_TERMUX_EXEC_OK`, exit **0** as Termux uid 10593.
- The **actual shared-effect source** is
  `providers/chatgpt/src/native_claims.rs`, backed by
  `$PREFIX/var/tmp/codex-native-effects/shared-native-v1/claims.json`.
  An orphaned `Opaque` entry arose from
  `functions.write_stdin` continuation with session 59940 after its
  process had exited. The new claim inserted `session: None` despite
  a known continuation; after `Process exited with code 0`,
  `observed()` filtered `held.session == Some(session)` and returned
  early, skipping the record's own-identity removal. It then rejected
  unrelated `functions.exec_command` offers as `native_resource_busy`.
- The Termux repair agent reports a private ledger backup at
  `/data/data/com.termux/files/usr/var/tmp/codex-native-effects/shared-native-v1/claims.json.bak-20261111-2005`.
  With an exclusive file lock and atomic replacement, the agent
  reconciled the **settled specific** claim; the post-repair ledger
  was `{"claims":{}}`. The original OS process had already exited
  normally; **no SIGKILL was necessary**.
- The repair agent's **local uncommitted** code patch retains
  `existing_session` when offering and storing a new claim,
  always retains it as the guard's continuation, and removes the
  original identity upon verified session exit. Added test:
  `continuation_without_prior_claim_cleans_up_on_exit`.
  Reported tests: `cargo test native_claims` **15/15**,
  `cargo test --lib` **205/205**, provider binary build success.
- Reported actual post-repair acceptance:
  A direct shell output/exit 0;
  B fresh Native exec `NATIVE_RESOURCE_RECOVERED` and `pwd` exit 0;
  C same work second exec `SEQUENTIAL_EXEC_OK_2` exit 0;
  D finished work followed by new work exec `TEST_D_REUSE_OK`
  exit 0 and finish; E preserved conflict denials under 15
  Native-claims tests. These are user-supplied operating receipts,
  **not** independent inspection of each raw work/test receipt.
- This documentation review **independently read the actual local Git
  diff** and confirmed the reported guard/settlement changes plus
  the regression test exist **uncommitted**. A subsequent read-only
  Native `functions.exec_command` containing `git` commands actually
  exited **0**; this independently demonstrates that at least that
  invocation was admitted after recovery.
- Remote `rewrite/rust-core` at `7b98296a` still contains
  `session: None` and `continuation: if resumed {...} else {None}`,
  and `observed()` still returns before removing the own identity
  in the continuation-exit case. **The source repair is NOT yet
  integrated, committed, signed or proven active in the installed
  Provider binary.** The concrete recurring defect must not be marked
  product-fixed until that acceptance is complete.

Earlier work-specific `native_effects.active=[]` and a successful
`codex_stop` did **not** prove the global ledger was empty. The source
and operating report together identify the specific mistake; do not
resurrect the old "unknown root cause" diagnosis. Do not generalize
this one repaired claim to all possible Busy/Owner-loss cases.

## TermuxBridge standalone shell / emergency access findings

- The **currently connected** `@TermuxBridge` tool schema exposes
  `codex_*` plus `native_catalog`/`native_tool`; no independent
  `termux_exec` or `local_execute` was advertised.
- The source *does* contain `providers/chatgpt/src/local.rs` tools
  `local_context`, `local_execute`, `local_process`, conditional on
  `execution.standalone_permission_profile`. They allow standalone
  process I/O without a Native conversation, but ordinary
  `local_execute` explicitly calls
  `SharedNativeGuard::offer_external` on **the same process-shared
  native claims ledger** before Core command execution. It can
  therefore also be blocked by a stale `Opaque` claim.
- Source includes configured Core permissions and claim-scoped
  idempotent process/output state, **not** an out-of-band rescue
  shell independent of a broken Provider ledger/Core. Merely ordering
  standalone tools first would not have resolved this incident.
  Any new host recovery power must be independently authenticated,
  explicitly authorized for destructive actions, and safety-scoped
  to verified owner/lease/receipt state; no broad admin bypass
  or second Native executor is approved yet.

## Separate remaining real-flow acceptance

Installed TUI `/provider` real GPT-6 two model turns on one Native
thread, exact Provider/Core writer/home alignment, original SSE/HTTP
413 recovery without replay, independent safe parallelism, live
notification click → correct Activity/terminal/session and opt-in
headless auto-display remain **unaccepted** in the evidence reviewed.
Earlier local auth observations (e.g. general CLI `token_revoked`)
must not be reclassified as Provider inference failure without
demonstrated routing. Native-model IDs and stored Provider settings
are distinct, and no account/profile or historical JSONL/SQLite
repair was authorized or performed here.

The dated [U1–U7 review queue](docs/development/REVIEW_ACTIONS_20261110.md)
remains a hypothesis/triage snapshot with R01–R10 work and prior
2,410–5,990 changed-LoC planning range. Verify its remaining
producers against current source rather than replaying all findings.
