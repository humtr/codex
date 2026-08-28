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

### Bundle M1-B9 — apply the base-environment plan at final exec

- Prior accepted evidence: M1-B8 commit
  `ae678fdb01b065a78f55b4e0546a8c4b12c498fa`.
- Exact outcome: apply an already-built `TermuxBaseEnvPlan` to the child
  `Command` used by the accepted runtime-FD final-exec path, while preserving
  B3's exact five-variable contamination fence, B4 FD 33/34 behavior, B6
  sandbox-policy ordering, and B7 argv/process composition. This bundle does not
  derive the plan inputs or choose any product path.
- Preserve the existing public/test-facing signatures and behavior of
  `exec_upstream`, `exec_upstream_with_runtime_fds`, and `launch_upstream`.
  Introduce only the smallest module-private implementation/composition needed
  to let a new environment-aware launch path pass `Some(&TermuxBaseEnvPlan)`;
  the existing paths must continue through the same implementation with no
  positive plan. Do not copy the FD setup/restoration body or the five-variable
  fence into a second implementation.
- At child-Command construction, apply every planned `(OsString, OsString)`
  assignment without UTF-8/lossy conversion, then enforce the exact B3 removals
  afterward so `CODEX_MANAGED_BY_NPM`, `CODEX_MANAGED_BY_BUN`,
  `CODEX_MANAGED_PACKAGE_ROOT`, `LD_PRELOAD`, and `LD_LIBRARY_PATH` cannot be
  reintroduced at the final boundary. Do not call `env_clear` and do not mutate
  the parent environment.
- The new module-private launch composition receives the selected program,
  resolver path, managed-config directory, original raw user argv, and a
  pre-built environment plan explicitly. Sandbox-policy rejection still occurs
  before resolver/config I/O or exec. No `std::env` read, runtime/generation
  selection, hard-coded Termux path, or normal `main` wiring is authorized in
  production B9 code.
- Required focused real-exec proof: use only test-owned temporary
  resolver/config/fake-upstream artifacts and an explicit B8 plan. Across the
  actual final exec boundary prove the exact planned `TMPDIR`, `TMP`, `TEMP`,
  `SQLITE_TMPDIR`, `SSL_CERT_FILE`, optional `SSL_CERT_DIR`, and `PATH` values
  are visible; the exact sandbox prelude and original user argv remain ordered;
  FD 33/34 still expose the supplied read-only resolver/config sources; all five
  B3 variables are absent even when inherited in the probe process; and one
  unrelated synthetic inherited variable survives. The test fake upstream must
  not depend on the planned PATH for helper lookup.
- Required failure proof: with a valid explicit plan but a deliberately missing
  upstream program, the environment-aware path returns an exec error while the
  caller's corresponding parent environment values remain byte-for-byte
  unchanged and B4's prior FD 33/34 restoration still holds. Do not mutate the
  parent merely to simplify this proof.
- Raw-value rule: production application must pass `OsStr`/`OsString` values
  directly to `Command`; no `to_str`, `to_string_lossy`, split/rejoin, or other
  normalization is permitted. B8 already proves raw planner construction; B9
  proves transport to the execution boundary without changing that rule.
- Keep all 47 accepted B1-B8 tests green. Use `CARGO_NET_OFFLINE=true` and a
  repository-external `CARGO_TARGET_DIR` for formatting, focused `m1_b9_`
  tests, all locked workspace tests, and locked workspace build.
- Worker configuration: one bounded `agy` CLI implementation worker in
  `accept-edits` mode through the Task-owned shell route, `fork_context: false`.
  No delegation, commits, pushes, package operations, network/update behavior,
  or live product state.
- Writable worker path: `crates/core/src/main.rs` only. No dependency, manifest,
  authority-document, or extra tracked-file changes are authorized.
- Protected surfaces: every other repository path, all live resolver/runtime/
  launcher/Manager state, profiles, sessions, auth, Git refs, sealed legacy
  history, and unrelated worktrees.
- Explicitly deferred: deriving planner inputs from the live environment,
  selected runtime/generation resolution, code-mode-host qualification, normal
  `main` wiring, doctor, generation/updater interfaces, installation,
  activation, and Manager behavior.
- Completion gate: actual diff contains one shared execution implementation and
  bounded environment-aware composition/tests; no duplicated FD/fence logic,
  parent-env mutation, lossy conversion, path-policy decision, dependency,
  public-surface expansion, or protected-state change; real exec proves the
  positive assignments and pre-existing argv/FD/fence contracts together; Lead
  reruns focused and full validation before acceptance.

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
