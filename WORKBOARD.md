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

### Bundle M1-B3 — upstream environment contamination fence

- Prior accepted evidence: M1-B2 commit
  `fc50b39e50bb6ef341d3cf01163ca90423bd7b13`.
- Exact outcome: ensure the final upstream exec command removes only the five
  inherited contamination variables currently supported by both the normative
  "no package-manager or preload variables" requirement and sealed predecessor
  observations: `CODEX_MANAGED_BY_NPM`, `CODEX_MANAGED_BY_BUN`,
  `CODEX_MANAGED_PACKAGE_ROOT`, `LD_PRELOAD`, and `LD_LIBRARY_PATH`. Preserve
  unrelated environment entries unchanged. Do not mutate the parent process
  environment as part of planning or on an exec failure.
- Writable worker path: `crates/core/src/main.rs` only.
- Governing contracts: `SPEC.md` section 5 requires construction of a qualified
  runtime environment without leaking package-manager or preload variables.
  Sealed legacy `shell/exec.sh`, `src/wrapper/runtime_env.py`, and the
  `runtime-exec` golden probe identify the five names above as predecessor
  observable behavior; that evidence informs this bounded test but is not
  rewrite proof and does not authorize copying legacy implementation.
- Non-goals: do not adopt legacy `CODEX_SELF_EXE`, `CODEX_HOME`, TMP/TMPDIR,
  certificate, `BROWSER`, XDG, GODEBUG, or PATH rules in this bundle; product
  runtime-path and compatibility-tool selection remain a later Core plan, while
  profiles remain Manager-owned.
- Worker configuration: one new `agy` CLI implementation worker through a
  bounded Task-owned shell execution, `accept-edits` mode with terminal sandbox
  restrictions and one non-interactive prompt. No delegation, authority-doc
  edits, commits, pushes, package installation, network update, or live product
  mutation.
- Named validation: with `CARGO_NET_OFFLINE=true` and repository-external
  `CARGO_TARGET_DIR`, run `cargo fmt --check`,
  `cargo test --locked --workspace`, and `cargo build --locked --workspace`.
- Required focused evidence: in a private probe process set synthetic values for
  all five contamination variables immediately before calling the production
  exec primitive and prove the upstream process sees each as absent. Set at
  least one unrelated synthetic environment entry and prove its exact value is
  preserved. Also prove a failed exec does not clear the parent process's
  synthetic contamination value.
- Explicitly deferred: normal `main` upstream-path wiring, additional runtime
  environment overrides, TTY/signals, FD 33/34, resolver checks, sandbox-policy
  enforcement, doctor, Manager, updater, network behavior, installation, and
  activation.
- Protected surfaces: every repository path except `crates/core/src/main.rs`,
  plus live launcher/runtime/Manager state, `$PREFIX/etc/resolv.conf`, profiles,
  sessions, auth data, Git refs, sealed legacy history, and unrelated worktrees.
- Completion gate: std-only implementation; exactly the five declared names are
  removed from the child exec environment; unrelated environment survives;
  parent environment is unchanged on exec failure; all M1-B1/B2 tests remain
  green; named validation passes; status contains only the authorized source
  file.
- Integration disposition: the primary Lead inspects the actual diff, reruns
  load-bearing validation outside the repository, and commits only accepted
  work.

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
