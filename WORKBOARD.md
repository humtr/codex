# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Core goal status: **complete**.
- MNT-1 wrapper-version residue retirement is accepted; its evidence is in
  `GOAL.md`.
- The obsolete repository-local `.git/hooks/pre-commit` is absent. No
  wrapper-version updater, env file, replacement hook, or alternate version
  authority is part of the current rewrite.
- There is no active implementation bundle, no active maintenance bundle, no
  selected next source slice, and no active Goal Lift.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## Resume rule

Future product work starts only after a concrete new product risk or
user-visible capability updates `GOAL.md` (or activates a Goal Lift) and a new
bounded implementation bundle replaces this closed state before product-code
mutation. Repository-only maintenance must likewise open a bounded maintenance
bundle before mutating repository environment state.

Remote `main` promotion, source push, release/index publication, and further
live mutation remain separate explicitly authorized operations. Historical
accepted evidence stays in `GOAL.md`; do not repopulate this file with closed
bundle history.
