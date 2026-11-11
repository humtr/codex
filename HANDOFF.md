# Handoff — current integration and acceptance

Updated: 2026-11-11 KST. This is the **single current unfinished-work
handoff**, not a specification, product-status history or authorization to
deploy. Follow [AGENTS](AGENTS.md) and the [documentation policy](docs/DOCUMENTATION.md).
Read [CURRENT](CURRENT.md) for verified source, CI and device claims;
read [SPEC](SPEC.md) and its annexes for required behavior.
`PRO_HANDOFF.md` and previous handoff snapshots are **not current authorities**.

## Source of truth and existing integration

- The sole ongoing product-development source branch is
  [`rewrite/rust-core`](https://github.com/humtr/codex/tree/rewrite/rust-core).
  Resolve its **actual** current HEAD, worktrees, dirty files and owners
  before editing; a SHA in this handoff is evidence, not a writable lease.
- `integration/codex-native-20261110` at `a55d5bb8` is already an ancestor
  of `rewrite/rust-core`. Do not merge it again. [PR #29](https://github.com/humtr/codex/pull/29)
  for the Core `current` compatibility alias was merged in `afdfb8bd7e57`.
  Preserve the exact code and safety checks; the fix is not part of the
  previously signed Full163 image.
- `main` is the **separate publication/approval authority**, not another
  developer branch to merge by Git ancestry. Frozen release snapshots,
  experiments, retired fixes and historic WIP are not automatic merge inputs.
  Before retaining any divergent branch changes, compare actual behavior,
  patch provenance, tests and consumers. Local-only worktrees/plans remain
  unverified until local Git access works again.

## Next unfinished boundaries, in order

1. **P0 Native availability and safe recovery.** After the user completely
   restarted Termux, a new attached Native work accepted creation and
   attachment but rejected a read-only `functions.exec_command` with
   `native_resource_busy` and `executed=false`. Its per-work
   `native_effects.active=[]`, `unconfirmed_effects=0` and `pending_calls=[]`.
   Some old works still reported `reconciliation_required` or
   `stop_requested`. These facts do **not** establish the global owner.
   Inspect the wrapper's actual shared-effect owner, leases, reconciliation,
   stop and re-entry logic through an **independent local Termux shell**;
   do not route emergency repair through the blocked Bridge command lane.
   Repair only proven stale ownership, preserve unresolved effects, then
   demonstrate fresh/same-work/repeated work execution and safe conflict
   rejection. No blind lock deletion, all-process kill or fake success.
2. **Release framework consolidation (W0–W14):** Independently revalidate
   the user's existing plan against current code before implementation.
   Preserve the whole release framework, ChatGPT Provider naming and
   independent authentication boundary, upstream/Native model identity,
   existing session continuity, SPEC contract and full implementation/tests.
   Explicitly include Native Busy/Stop self-recovery and TermuxBridge/
   AppHost-started notification tap → correct session/terminal focus
   continuity. Distinguish proposal, verified behavior, signed release
   and device activation. The requested **plan → user approval → implement**
   boundary still applies to the full W0–W14 rewrite; this documentation
   integration is not approval for that implementation.
3. **Review work selection:** Use the dated
   [2026-11-10 review matrix](docs/development/REVIEW_ACTIONS_20261110.md)
   as hypothesis/evidence only. Reassess each R01–R10 item against the
   actual current source; `c19b61bb` already changed release notification
   admission, so do not reimplement a prior fix because its original review
   entry remains. Confirm any remaining source/CI ownership, MCP auth,
   Native result/effect admission, TUI re-entry or migration concerns with
   exact producer/consumer tests. Keep unrelated local work protected.
4. **Whole-route acceptance:** At `f9dfc3f` Source Acceptance
   [#42942784769](https://github.com/humtr/codex/actions/runs/42942784769)
   and Core Qualification
   [#42942787296](https://github.com/humtr/codex/actions/runs/42942787296)
   passed, but TUI Native
   [#42945712023](https://github.com/humtr/codex/actions/runs/42945712023)
   **failed**. PR #29 head `a8cb66c` separately passed
   [Source Acceptance](https://github.com/humtr/codex/actions/runs/42962985877)
   and [Core Qualification](https://github.com/humtr/codex/actions/runs/42962989662);
   neither proves exact post-merge/documentation HEAD acceptance.
   Requalify each affected boundary at its real commit. Installed TUI
   `/provider` → GPT-6 two responses on the same Native thread, SSE/HTTP
   413 recovery, no duplicate effects and independent safe concurrency
   remain separate live checks.

## Operational and approval fence

- The signed [Full163 publication](https://github.com/humtr/codex/releases/tag/full-v0.167.0-c19b61b-s163)
  is Codex **0.167.0**; current source's
  [upstream recipe](release/upstream.json) is **0.167.1**. Do not collapse
  these generations or claim the merged alias fix was shipped in Full163.
  PR #29 reports Full163 device activation and retained Full159/Full153;
  obtain a fresh readback before another device-side operation.
- Do not change installed/rollback generations, keys, accounts, profiles,
  services, approval policy, active workers or Android sdcard as a
  documentation/source-integration side effect. Signing, rollout, device
  replacement, live-service restart and broad migrations require distinct
  explicit authorization.
- Before committing: inspect exact HEAD/owner and relevant contracts, isolate
  writers, check focused positive/negative tests and diffs. Before accepting:
  verify exact-current-SHA source/Provider/Core/TUI gates **and** original
  Native/model/tool receipts when relevant. Stop only the task proven owned;
  do not reset other sessions or hide unresolved delivery.

Remove a completed item from this handoff once its commit and scoped proof
are recorded in CURRENT or Git; do not append a chronology.
