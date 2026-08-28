# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Branch ancestry: independent empty-root lineage; do not merge or rebase
  `main` or `legacy/monolith`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone: Milestone 1 — local Core
- Primary Technical Lead/Integrator: the main `gpt-5.6-sol` / `max` goal
  session; owns evidence retrieval, planning, worker packets, actual diff review,
  integration validation, commits, and acceptance decisions across both Core
  milestones
- Implementation worker: exactly one mutating worker at a time, bounded to the
  accepted code/test paths and validation; reused only within that bundle
- Planning agents, problem advisors, and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

### Bundle M1-B1 — minimal workspace and command classifier

- Bound base: `rewrite/rust-core@5a660669055029be2b3ce53a6aa9bb7261b7290a`.
- Exact outcome: create a locked dependency-free Cargo workspace containing one
  Core binary and implement only the exact first-argument classifier needed to
  distinguish Core-owned `update`, `doctor`, and `termux` commands from upstream
  passthrough. Runtime execution remains for a later bundle.
- Writable worker paths: `Cargo.toml`, `Cargo.lock`, `crates/core/Cargo.toml`,
  and `crates/core/src/main.rs` only.
- Governing contracts: `SPEC.md` section 3 requires interception only when the
  exact first argument is `update`, `doctor`, or `termux`; every other argv
  shape, including `--version` and `-V`, remains upstream passthrough. The
  rewrite discipline forbids legacy source copying and unnecessary dependencies.
- Worker configuration: one replacement `agy` CLI worker launched through a
  bounded Task-owned shell execution, `accept-edits` mode with terminal sandbox
  restrictions and a single non-interactive prompt. The registered `agy`
  harness adapter is not used because its stdin prompt transport is incompatible
  with the installed CLI; a corrected non-mutating CLI probe returned `READY`.
  The worker has implementation authority only and may not edit authority
  documents, commit, push, install packages, use provider tools, delegate, or
  touch live product state.
- Named validation: `cargo fmt --check`, `cargo test --locked --workspace`, and
  `cargo build --locked --workspace`, all from the repository root without
  network or package installation.
- Protected surfaces: `AGENTS.md`, `SPEC.md`, `GOAL.md`, `WORKBOARD.md`,
  `README.md`, `legacy/monolith`, all live launcher/runtime/Manager paths,
  `$PREFIX/etc/resolv.conf`, profiles, sessions, auth data, and unrelated
  worktrees/refs.
- Completion gate: exactly one workspace member and one Core binary; committed
  lockfile; zero external dependencies; focused tests cover empty argv, exact
  `update`/`doctor`/`termux`, near-miss spellings, `--version`, `-V`, arbitrary
  passthrough arguments, and a non-UTF-8 first argument on Unix/Android without
  lossy parsing; named validation passes; no path outside the worker scope is
  changed.
- Integration disposition: after the worker returns, the primary Lead inspects
  the actual diff, reruns the load-bearing validation, accepts or rejects the
  bundle, and commits only accepted work before planning M1-B2.

## Milestone 1 required outcomes

1. Create a minimal locked Cargo workspace with one Core binary and no unused
   dependency.
2. Implement exact first-argument routing for `update`, `doctor`, and `termux`;
   classify every other argv shape as upstream passthrough.
3. Prove `--version` and `-V` preserve exact upstream stdout, stderr, and exit
   status without Core version output.
4. Implement environment planning and final upstream execution with preserved
   argv, standard streams, TTY, signals, and exit status.
5. Open resolver/config sources read-only, map FD 33/34, preserve them across
   final exec, and prove the live resolver is unchanged.
6. Implement explicit unsupported-sandbox behavior without bwrap.
7. Implement read-only local doctor composition with redacted human and JSON
   output; unavailable Manager is represented explicitly.
8. Define and validate the generation-manifest and updater interfaces without
   performing network or live activation.
9. Add unit, integration, fault, and real-Termux smoke tests in temporary roots.
10. The primary Lead updates `GOAL.md` with exact Milestone 1 evidence. If every
    gate passes, it replaces this workboard's current target with its bounded
    Milestone 2 plan and continues.

## Milestone 1 completion gate

- clean locked release build on the current Termux device;
- all focused and integration tests pass;
- argv/TTY/signal/exit and FD33/34 contracts pass;
- doctor is read-only and secret-redacted;
- resolver stat, mode, content digest, and path are unchanged;
- no file under the live launcher/runtime/Manager paths changed;
- no legacy implementation source was copied;
- no network update or product activation occurred.

## Stop lines

- Do not begin Milestone 2 work while a Milestone 1 gate is unresolved.
- Do not implement Manager product features.
- Do not run package installation or update commands.
- Do not spawn a planning agent, problem advisor, checkpoint reviewer, or more
  than one implementation worker.
- Do not let a worker edit authority documents, commit, push, broaden its own
  read/write scope, or touch protected live state.
- Do not treat a worker report or worker-run test as acceptance proof until the
  primary Lead has inspected the actual diff and rerun load-bearing validation.
- Do not resend unchanged context or exceed the worker packet budgets in
  `GOAL.md`.
- Do not modify `legacy/monolith` or rewrite sealed tags.
- Do not expand the document hierarchy during ordinary implementation.

## Next milestone

Milestone 2 — delivery and recovery — remains defined in `SPEC.md` and is not
current work until the Milestone 1 ledger is complete. Completion of that ledger
causes the same primary Lead to replace this file's current target with the
Milestone 2 plan; it does not require a routine user stop. Exhaustion of an
accepted bundle before then causes the Lead to plan the next bundle, not to end
the task.
