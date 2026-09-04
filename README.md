# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The rewrite has one public command, `codex`, and two internal layers:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- a separate Manager layer reached through `codex termux` for profiles,
  sessions, notifications, and other Termux conveniences.

## Current status

The Rust Core implementation through M2-B11 is accepted in the `GOAL.md`
acceptance ledger. `WORKBOARD.md` currently tracks the bounded M2-R1 durability
closure for generation publication, fresh/legacy entrypoint retries, and final
release-builder modes before independent product review.

This branch is not installed over the working Codex runtime. Release delivery
and promotion to `main` remain separate acceptance gates.

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
