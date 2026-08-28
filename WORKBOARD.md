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
  session; owns evidence retrieval, planning, direct implementation while worker
  mode is OFF, actual diff review, integration validation, commits, and acceptance
  decisions across both Core milestones
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it. Do not invoke implementation workers or coding subagents while
  OFF
- Planning agents, problem advisors, and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

### Bundle M1-B14 — qualified-runtime final launch composition

- Prior evidence: M1-B13 commit `71acbd8e318d50548952490e0d2fb52c7b661f9c`;
  validation `job_igy_c7b11e3616` passed B13 10/10, full serial 90/90, eight
  default-parallel full repetitions, formatting, diff check, and warning-free
  locked build.
- Add one module-private launch boundary that accepts `QualifiedRuntimeAssets`, a
  `TermuxProcessEnvSnapshot`, explicit fallback certificate file/directory,
  explicit resolver path, explicit managed-config directory, and raw user argv.
- It must derive the B10 environment plan using exactly the qualified assets'
  compatibility directory and then call the existing `launch_upstream_with_env`
  using exactly the qualified runtime program path. Do not accept a separate raw
  runtime program or compatibility directory at this boundary.
- Preserve existing ordering: environment planning is pure; the existing launch
  function still performs sandbox-policy validation before resolver/config FD I/O
  or exec. Preserve B3 contamination fencing, B4 FD33/34 behavior, raw argv,
  process/TTY/signal/exit semantics, and B9 env application.
- Use a typed module-private error that distinguishes B10 environment-plan failure
  from existing launch policy/exec failure without string parsing.
- No active generation lookup, manifest parse/serialization, digest calculation,
  filesystem qualification, resolver/config selection, HOME-derived path, updater
  I/O, doctor, activation, rollback, network, package operation, or normal `main`
  wiring belongs in B14.
- Focused `m1_b14_` tests: invalid process snapshot fails before runtime I/O;
  unsupported sandbox fails before invalid resolver/config I/O; real subprocess
  exec using test-owned resolver/config/fake runtime proves the B13 runtime path
  is used, B13 compatibility directory drives PATH, planned temp/cert values
  arrive, sandbox prelude/raw argv remain exact, and FD33/34 expose the supplied
  test artifacts.
- Keep all 90 post-B13 tests green. Validate offline with an external Cargo target,
  focused tests, full serial suite, eight default-parallel repetitions,
  warning-free locked build, formatting, and diff check.
- Worker mode is user-controlled and remains OFF. The primary `gpt-5.6-sol` /
  `max` Lead directly implements this bundle. `harness.run` is test-only.
- Writable product path: `crates/core/src/main.rs` only; no dependency, extra
  tracked file, live product/runtime/resolver/Manager, Git-ref, install, activation,
  package, or network change is authorized.

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
- Do not spawn a planning agent, problem advisor, or checkpoint reviewer. Do
  not invoke an implementation worker or coding subagent while user-controlled
  worker mode is OFF.
- The primary Lead must keep direct edits inside the selected bundle and must
  inspect the actual diff and rerun load-bearing validation before acceptance.
- Do not modify `legacy/monolith` or rewrite sealed tags.
- Do not expand the document hierarchy during ordinary implementation.

## Next milestone

Milestone 2 — delivery and recovery — remains defined in `SPEC.md` and is not
current work until the Milestone 1 ledger is complete. Completion of that ledger
causes the same primary Lead to replace this file's current target with the
Milestone 2 plan; it does not require a routine user stop. Exhaustion of an
accepted bundle before then causes the Lead to plan the next bundle, not to end
the task.
