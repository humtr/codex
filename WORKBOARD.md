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

### Bundle M1-B4 — FD 33/34 runtime inheritance

- Prior accepted evidence: M1-B3 commit
  `815c9104c726f212ee4a51b518af14e8c133b20c`.
- Exact outcome: add a production final-exec path that accepts an explicit
  resolver file path and an existing managed-config directory path, opens both
  read-only, maps the resolver to FD 33 and the directory to FD 34 without
  lossy path conversion, and preserves those descriptors across exec. If any
  setup step or the final exec fails, restore the caller's prior FD 33/34 state
  before returning the error.
- Writable worker path: `crates/core/src/main.rs` only. No manifest or dependency
  changes are authorized.
- Governing contracts: `SPEC.md` section 5 requires FD 33 to expose the selected
  resolver source read-only, FD 34 to expose the process-local managed config
  directory, both to survive final exec, and the resolver source never to be
  created, rewritten, chmodded, repaired, or deleted by Core. M1 remains
  temporary-root only and must not touch live resolver/runtime state.
- Implementation constraint: standard library plus the smallest target-local
  Unix/Android FFI required for descriptor duplication/flags is allowed inside
  this source file; do not add a crate. The implementation must handle source
  descriptors that collide with 33/34, must ensure mapped descriptors are not
  close-on-exec, and must not leak backup/source duplicate descriptors into the
  exec target. A failed exec must not leave caller FD 33/34 altered.
- Worker configuration: one new bounded `agy` CLI implementation worker in
  `accept-edits` mode through the Task-owned shell route. No delegation,
  authority-doc edits, commits, pushes, package operations, network update, or
  live product mutation.
- Named validation: `CARGO_NET_OFFLINE=true` with repository-external
  `CARGO_TARGET_DIR`; run `cargo fmt --check`,
  `cargo test --locked --workspace`, and `cargo build --locked --workspace`.
- Required focused evidence: use only test-owned temporary resolver/config
  paths. A private exec probe must observe FD 33 as the exact resolver file with
  its original bytes, FD 34 as the exact config directory, and FD 33 must reject
  a write attempt. Capture resolver path, bytes, Unix mode, inode/device, size,
  and modification timestamp before and after the probe and prove they remain
  unchanged. A separate failed-exec probe must prove both the originally-absent
  FD case and an existing-sentinel FD case are restored after failure.
- Keep all M1-B1/B2/B3 argv, stream, exit, and environment-fence tests green.
- Explicitly deferred: creating or mutating managed config contents, exact
  product runtime/config path selection, normal `main` wiring, TTY/signals,
  sandbox-policy parsing, doctor, generation/updater interfaces, network,
  installation, and activation.
- Protected surfaces: every repository path except `crates/core/src/main.rs`,
  plus the live resolver, launcher/runtime/Manager paths, profiles, sessions,
  auth data, Git refs, sealed legacy history, and unrelated worktrees.
- Completion gate: no dependency/file expansion; read-only FD setup survives
  exec; resolver metadata/content/path evidence is unchanged; failed setup/exec
  restores prior FD 33/34 state; all named validation passes; status contains
  only the authorized source file.
- Integration disposition: the primary Lead reviews unsafe/FFI boundaries and
  actual diff, reruns load-bearing validation in an external target directory,
  and commits only accepted work.

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
