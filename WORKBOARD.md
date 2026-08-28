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

### Bundle M1-B15 — typed read-only doctor composition surface

- Prior evidence: M1-B14 commit `be6492f895185caf7d9b922b16330a1cd8f00033`;
  validation `job_ijk_b7acb35e72` passed B14 3/3, full serial 93/93, eight
  default-parallel full repetitions, formatting, diff check, and warning-free
  locked build.
- Add a dependency-free, module-private doctor report model with separate typed
  state domains for upstream, Termux Core, and Manager diagnostics. Upstream may
  be healthy, unhealthy, or unsupported; Core may be healthy, unhealthy, or API
  incompatible; Manager may be healthy, unhealthy, unavailable, or API
  incompatible.
- Compose those states into one deterministic summary with explicit precedence:
  API incompatibility, then unhealthy, then degraded for unsupported/unavailable,
  otherwise healthy. Keep the semantic exit class typed; do not freeze numeric
  process exit codes or public CLI option parsing in B15.
- Add deterministic human rendering with clearly separated Upstream, Termux Core,
  Manager, and Summary sections, plus exactly one dependency-free JSON envelope
  with `schema_version: 1` and the SPEC keys `upstream`, `termux_core`, `manager`,
  and `summary`.
- The B15 output model accepts no arbitrary diagnostic strings, raw upstream
  output, auth/session/notification content, paths, environment values, or other
  caller-controlled payloads. All emitted text is selected from bounded static
  status vocabulary, providing a fail-closed redaction baseline before process
  capture/parsing is introduced.
- No upstream process execution, filesystem or environment access, Manager call,
  generation/runtime selection, resolver/config I/O, network behavior, update,
  activation, rollback, dependency addition, or normal `main` wiring belongs in
  B15. Actual raw upstream doctor execution/composition is a later bounded bundle.
- Focused `m1_b15_` tests must cover all section-state combinations and summary
  precedence, explicit unsupported upstream/unavailable Manager representation,
  exact human section separation, exact valid JSON envelope rendering, and prove
  the renderer output vocabulary is bounded and deterministic with no process
  environment side effects.
- Keep all 93 post-B14 tests green. Validate offline with an external Cargo target,
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
