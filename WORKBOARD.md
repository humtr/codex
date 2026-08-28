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

### Bundle M1-B16 — qualified upstream doctor subprocess probe

- Prior evidence: M1-B15 commit `6bea7a53004f43178599e65a6f630c7bb06355b9`;
  validation `job_ikt_87c72178ad` passed B15 5/5, full serial 98/98, eight
  default-parallel full repetitions, formatting, diff check, and warning-free
  locked build.
- Add one module-private, supported-upstream doctor probe that accepts only a B13
  `QualifiedRuntimeAssets` selection plus explicit B10 process snapshot,
  certificate fallback, resolver path, and managed-config directory. Do not
  accept a separate raw runtime program or compatibility directory.
- Extract/reuse one common temporary runtime-FD mapping primitive so both final
  exec and the doctor child use the same read-only resolver/config validation,
  FD33/34 collision handling, CLOEXEC behavior, restoration, and restoration
  error precedence. The doctor child must not permanently mutate caller FD state.
- The doctor probe invokes the selected upstream runtime directly, never the
  public Core launcher. Its argv is the existing Termux-safe sandbox prelude plus
  the internal upstream `doctor` command; no user-controlled doctor argv is
  introduced in B16.
- Apply the B10 base-environment plan and exact B3 five-variable contamination
  fence to the doctor child. Child stdout/stderr are sent to null and are never
  inserted into the B15 report model. Exit success maps to
  `UpstreamDoctorStatus::Healthy`; a completed nonzero/signal result maps to
  `Unhealthy`; setup/spawn failures remain typed I/O/environment/policy errors.
  `Unsupported` remains an explicit higher-level state and must not be inferred
  by parsing raw stderr text.
- No public `doctor --json` parsing, numeric public exit-code mapping, `main`
  dispatch wiring, Manager call, runtime/generation discovery, network, update,
  activation, rollback, package operation, or product-state mutation belongs in
  B16.
- Focused `m1_b16_` tests use subprocess isolation for FD-mutating paths and must
  prove: healthy and nonzero classification; exact safe doctor argv; B10
  temp/cert/PATH application; contamination fencing and unrelated-env retention;
  child FD33/34 visibility; secret-like stdout/stderr suppression; typed missing
  runtime/spawn failure; parent FD33/34 restoration; and resolver/config content
  non-mutation in test-owned roots.
- Keep all 98 post-B15 tests green. Validate offline with an external Cargo target,
  focused tests, full serial suite, eight default-parallel repetitions,
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
