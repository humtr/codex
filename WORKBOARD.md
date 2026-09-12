# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Core goal status: **complete**. The final completion-gate audit at
  `rewrite/rust-core@2340bd345ff8f867c16ed78a28f1e5ca1fbb2f49` found no
  missing Milestone 1, Milestone 2, independent-review, or success-threshold
  gate.
- `origin/rewrite/rust-core` was verified at
  `2340bd345ff8f867c16ed78a28f1e5ca1fbb2f49` before this documentation-only
  closure. Actual remote `main` remains
  `004702fd8081df2a2b07efd1ed394b510bf4953b`; remote publication or promotion
  is not part of this closure.
- R10 live qualification remains accepted at generation
  `local-1789181261-r10-live-1`, with
  `local-1788570645-23374-1` retained as the previous generation and its exact
  prior launcher bound in rollback metadata.
- There is no active implementation bundle, no selected next source slice, and
  no active Goal Lift.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- The inherited pre-commit hook references absent
  `tools/update-wrapper-version.sh`; do not modify it. If it alone rejects an
  exact, revalidated staged tree, use the established `--no-verify` closure.
- Worker mode remains OFF.

## Resume rule

Future product work starts only after a concrete new product risk or
user-visible capability updates `GOAL.md` (or activates a Goal Lift) and a new
bounded implementation bundle replaces this closed state before product-code
mutation.

Remote `main` promotion, release/index publication, and further live mutation
remain separate explicitly authorized operations. Historical accepted evidence
stays in `GOAL.md`; do not repopulate this file with closed bundle history.
