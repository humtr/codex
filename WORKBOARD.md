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

### Bundle M1-B2 — upstream final-exec primitive

- Prior accepted evidence: M1-B1 commit
  `36c98dd8882ddba18657ab3f289eace1121ff39b`.
- Exact outcome: add the smallest Unix/Android upstream execution primitive that
  performs a final process replacement with an explicitly supplied upstream
  program and raw `OsString` arguments. Prove with subprocess tests that the
  upstream process receives arguments unchanged and that stdout, stderr, and
  exit status cross the exec boundary unchanged. This bundle does not yet pick
  the product upstream path or wire normal `main` dispatch to a live runtime.
- Writable worker path: `crates/core/src/main.rs` only.
- Governing contracts: `SPEC.md` sections 3 and 5 require every non-Core command,
  including `--version` and `-V`, to reach upstream without wrapper version
  output and require preservation of argv, standard streams, and exit status at
  the final execution boundary. Use raw `OsStr`/`OsString`; do not introduce
  lossy UTF-8 conversion.
- Worker configuration: one new `agy` CLI implementation worker through a
  bounded Task-owned shell execution, `accept-edits` mode with terminal sandbox
  restrictions and one non-interactive prompt. The known-incompatible registered
  `agy` harness stdin adapter remains unused. No delegation, authority-document
  edits, commits, pushes, package installation, or live product mutation.
- Named validation: with `CARGO_NET_OFFLINE=true` and `CARGO_TARGET_DIR` outside
  the repository, run `cargo fmt --check`, `cargo test --locked --workspace`,
  and `cargo build --locked --workspace`.
- Required focused evidence: a child process that calls the production exec
  primitive must demonstrate exact stdout bytes, exact stderr bytes, and a
  chosen nonzero exit status; upstream-visible argv must include unchanged
  `--version`, unchanged `-V`, ordinary arguments, and a non-UTF-8 argument on
  Unix. Tests must not depend on or execute the installed Codex product.
- Explicitly deferred: product upstream-path discovery, environment
  sanitization/planning, TTY and signal-specific probes, FD 33/34 setup,
  sandbox-policy enforcement, doctor, Manager, updater, network behavior, and
  live installation/activation.
- Protected surfaces: every path except `crates/core/src/main.rs`, plus all live
  launcher/runtime/Manager paths, `$PREFIX/etc/resolv.conf`, profiles, sessions,
  auth data, Git refs, legacy history, and unrelated worktrees.
- Completion gate: no new dependency or file; existing classifier tests remain
  green; exec primitive is used by focused subprocess tests without temporary
  wrapper output or public test-only command semantics; all named validation
  passes; repository status contains only the authorized source path.
- Integration disposition: the primary Lead inspects the actual source diff,
  reruns load-bearing validation in an external target directory, and commits
  only if the bundle gate is satisfied.

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
