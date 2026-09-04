# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The rewrite has one public command, `codex`, and two internal layers:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- a separate Manager layer reached through `codex termux` for profiles,
  sessions, notifications, and other Termux conveniences.

## Current status

The Rust Core implementation through M2 and the authorized local install/cutover
are accepted in the `GOAL.md` acceptance ledger. The R4 wrapper-owned update
boundary, signed-channel admission, doctor presentation, and code-mode
companion placement are also accepted; `WORKBOARD.md` has no active
implementation bundle.

The R4 Core launcher is installed in the working Termux runtime through a
bounded, digest-checked device cutover. Ordinary `codex update` is now owned by
Core and can activate only a signed generation prepared by the wrapper release
pipeline; it never delegates to the upstream self-updater. The existing signed
v1 generation remains active, so `codex doctor` reports its legacy `compat/`
layout as `migration_required` until a newly signed root-level companion
generation is delivered. Publishing that signed channel or promoting R4 to
`main` remains a separate acceptance gate.

Implementation is intentionally split into two milestones:

1. local Rust Core execution and compatibility contracts;
2. secure fresh installation, self-update, generation activation, and rollback.

Independent product review follows the completed Milestone 2 candidate rather
than interrupting ordinary implementation checkpoints.

## Documents

- `SPEC.md` — normative product and architecture contract
- `GOAL.md` — success threshold and acceptance ledger
- `WORKBOARD.md` — current milestone and next work only
- `AGENTS.md` — repository-local execution and safety rules

## Branches

- `main` — publication authority; not the implementation base
- `rewrite/rust-core` — independent orphan Rust Core implementation lineage
- `legacy/monolith` — sealed predecessor at `bf30a7d`

The implementation branch starts at an empty root and has no Git ancestry in
common with `main` or the legacy implementation. Promotion to `main` is an
explicit ref replacement after acceptance, not a merge.
