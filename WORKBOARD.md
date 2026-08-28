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

### Bundle M1-B7 — explicit-input passthrough launch composition

- Prior accepted evidence: M1-B6 commit
  `a4b4cb3a91bd78ea07952739f054695f10bab638`.
- Exact outcome: compose the accepted B6 passthrough-policy planner with the
  accepted B4 runtime-FD/final-exec path through one module-private production
  launch function. The function receives the selected upstream program,
  resolver path, managed-config directory, and original raw user argv as
  explicit inputs; it performs sandbox-policy planning first and only then calls
  the existing `exec_upstream_with_runtime_fds` primitive with the planned argv.
- This is composition, not path policy. Do not derive, hard-code, canonicalize,
  create, repair, or select product runtime/generation/config paths in this
  bundle. Do not wire normal `main` or implement update/doctor/Manager behavior.
- Writable worker path: `crates/core/src/main.rs` only. No dependency, manifest,
  authority-document, or extra-file changes are authorized.
- Governing contracts: `SPEC.md` sections 3 and 5 require upstream passthrough,
  explicit unsupported-sandbox failure, selected no-sandbox policy, read-only
  FD 33/34 sources, final-process fidelity, and a qualified runtime environment.
  B4/B6 acceptance in `GOAL.md` is evidence for the two primitives being
  composed; do not duplicate or reinterpret their internals.
- Error boundary: keep any new launch error type module-private. A policy
  failure must be distinguishable from runtime I/O/exec failure without lossy
  argv conversion. Unsupported sandbox input must return before the resolver,
  config directory, or program is opened/executed.
- Required focused proof: (1) pass an unsupported sandbox request together with
  deliberately nonexistent program/resolver/config paths and prove the result
  is the policy error rather than I/O; (2) for accepted ordinary input, use a
  test-owned executable fake upstream plus temporary resolver/config roots and
  prove after the real final exec that argv begins with exactly `-c` and
  `sandbox_mode="danger-full-access"`, followed by every original user argument
  in order, FD 33/34 refer to the supplied read-only sources, and the existing
  five contamination variables are absent while an unrelated environment value
  survives; (3) preserve existing failed-exec FD restoration behavior by
  reusing the B4 primitive rather than copying its implementation.
- Test-only helper artifacts must live under temporary roots and be removed by
  the test. They must not become tracked files or public command semantics.
- Keep all 34 accepted B1-B6 tests green. Use `CARGO_NET_OFFLINE=true` and a
  repository-external `CARGO_TARGET_DIR` for `cargo fmt --check`, focused B7
  tests, `cargo test --locked --workspace`, and
  `cargo build --locked --workspace`.
- Worker configuration: one bounded `agy` CLI implementation worker in
  `accept-edits` mode through the Task-owned shell route, `fork_context: false`.
  No delegation, commits, pushes, package operations, network/update behavior,
  or live product state.
- Protected surfaces: every repository path except `crates/core/src/main.rs`,
  all live resolver/runtime/launcher/Manager state, profiles, sessions, auth,
  Git refs, sealed legacy history, and unrelated worktrees.
- Explicitly deferred: actual runtime/generation path selection, positive
  runtime-environment additions not already normatively required, normal `main`
  wiring, doctor, generation/updater interfaces, installation, activation, and
  Manager behavior.
- Completion gate: actual diff contains only the bounded composition and tests;
  policy failure demonstrably precedes runtime I/O; accepted launch crosses the
  real exec boundary with planned argv plus FD/env contracts intact; no copied
  FD/env implementation, dependency, public-surface expansion, path-policy
  decision, tracked test artifact, or protected-state mutation; Lead reruns the
  load-bearing validation before acceptance.

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
