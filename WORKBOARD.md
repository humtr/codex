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

### Bundle M1-B18 — exact doctor invocation and output plan

- Prior evidence: M1-B17 commit `64199eb4cbb1dccb351cf140c55e5e36d77d65ce`;
  final validation `job_inr_7ffbd3d9f8` passed B17 4/4, full serial 106/106,
  eight default-parallel full repetitions, formatting, diff check, and
  warning-free locked build. Earlier transient parallel evidence is retained in
  `GOAL.md` and is not used as acceptance proof.
- Add a pure, module-private `DoctorOutputMode` and doctor invocation planner for
  arguments after the exact leading `doctor` token. No trailing args selects
  human output; exactly one `--json` selects the single B15 JSON envelope.
- Reject every other shape, including duplicate `--json`, unknown options,
  positional arguments, `--`, and non-UTF-8 bytes, with one bounded typed usage
  error. Its Display text must be static and must never echo or lossily decode
  the rejected argv.
- Add one pure result surface that renders an already-composed `DoctorReport`
  through the selected B15 renderer and returns the existing typed
  `DoctorExitClass`. JSON mode must still return a valid complete machine
  envelope for degraded, unhealthy, and API-incompatible reports.
- Usage errors remain represented by the planner's distinct error type and must
  not be collapsed into `HealthFailure` or `ApiIncompatibility`. Do not assign
  numeric process exit codes in B18 because SPEC requires semantic distinction
  but does not define numbers.
- No `main` dispatch wiring, runtime/generation discovery, B17 execution,
  Manager/Core health discovery, filesystem/environment I/O, network, update,
  activation, rollback, package operation, dependency addition, or product-state
  mutation belongs in B18.
- Focused `m1_b18_` tests must cover exact human/JSON planning, exhaustive
  representative invalid forms including raw non-UTF-8, static non-leaking usage
  error text, human/JSON rendering for every doctor semantic exit class, valid
  JSON preservation on diagnostic failure, and deterministic/environment-pure
  planning/rendering.
- Keep all 106 post-B17 tests green. Validate offline with an external Cargo
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
