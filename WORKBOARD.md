# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Original Rust Core goal status: **complete**.
- TERMUX-COMPAT Goal Lift status: **accepted**.
- TC-1, TC-2, and TC-3 are accepted and recorded in `GOAL.md`.
- Accepted TC-3 source commit:
  `370caceb63bd7fddb7b6075da49d0404755a8e43`.
- Selected upstream qualified by the accepted lift: `rust-v0.154.0`, upstream
  commit `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`. A future upstream target must
  revalidate release-sensitive compatibility findings before reuse.
- Active implementation bundle: **none**.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## Closure routing

The preplanned TERMUX-COMPAT sequence is complete. This workboard does not
implicitly authorize another source slice. New product-code work requires a new
bounded goal/routing decision after rereading `SPEC.md` and `GOAL.md` and
revalidating any upstream-sensitive assumptions.

The accepted TC-3 source commit is local only. A source push, remote `main`
promotion, release/index publication, live install or cutover, package-manager
mutation, provider/account change, live credential mutation, resolver mutation,
or installed runtime/helper replacement remains a separate explicitly authorized
operation.

## Resume rule

A fresh session resumes by reading `SPEC.md`, then `GOAL.md`, then this file and
checking branch/HEAD plus the selected upstream target. If this workboard still
shows no active implementation bundle, do not mutate product code merely to
continue the completed lift. Inspect or plan only until repository authority
selects a new bounded bundle.

Historical accepted evidence stays in `GOAL.md`; do not repopulate this file
with closed-bundle implementation history.
