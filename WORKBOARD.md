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

### Bundle M1-B19 — ordered local doctor command boundary

- Prior evidence: M1-B18 commit `fdecef9f86a1f04776309ffe344b169d715c7217`;
  validation `job_ioy_1672a340bc` passed B18 5/5, full serial 111/111, eight
  default-parallel full repetitions, formatting, diff check, and warning-free
  locked build.
- Add one module-private doctor command boundary that accepts trailing doctor
  argv plus the explicit B17 capability/qualified-runtime/B10/FD/Core/Manager
  inputs. It must call `plan_doctor_invocation` before `compose_local_doctor`,
  then render the successful report with `render_doctor_command`.
- Use one typed error that keeps B18 `DoctorUsageError` distinct from B17
  `QualifiedUpstreamDoctorProbeError`; do not stringify, collapse, or infer the
  error class from text.
- Usage rejection is the load-bearing ordering contract. Invalid, duplicate,
  positional, or non-UTF-8 doctor trailing argv supplied with Supported
  capability and intentionally invalid process snapshot/resolver/config/runtime
  inputs must return Usage without performing B10 planning, FD mapping, or spawn.
- Valid Supported human/JSON paths must cross the existing B16 subprocess probe,
  preserve raw-output suppression and bounded B15 rendering, and return the
  existing semantic `DoctorExitClass`. Valid Unsupported paths must retain B17's
  zero-probe-I/O behavior even with invalid probe-only inputs.
- Probe/setup failures after a valid invocation must remain typed Probe errors
  with no `DoctorCommandOutcome` fabricated. No numeric process exit codes are
  assigned in B19.
- No `main` dispatch wiring, generation/runtime discovery, Core/Manager health
  discovery, network, update, activation, rollback, package operation,
  dependency addition, or product-state mutation belongs in B19.
- Focused `m1_b19_` tests must prove usage-before-I/O for UTF-8 and non-UTF-8
  invalid forms, supported healthy human output, supported unhealthy JSON output,
  unsupported invalid-probe-input JSON output, and supported spawn-error
  propagation without an outcome or raw diagnostic leakage.
- Keep all 111 post-B18 tests green. Validate offline with an external Cargo
  target, focused tests, full serial suite, eight default-parallel repetitions,
  warning-free locked build, formatting, and diff check.
- Worker mode is user-controlled and remains OFF. The primary `gpt-5.6-sol` /
  `max` Lead directly implements this bundle. `harness.run` is test-only.
- Writable product path: `crates/core/src/main.rs` only; no dependency, extra
  tracked file, live product/runtime/resolver/Manager, Git-ref, install,
  activation, package, or network change is authorized.

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
