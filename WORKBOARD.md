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

### Bundle M1-B20 — exact raw-argv public dispatch route

- Prior evidence: M1-B19 commit `148b1133f1afaa91668e19b4fade13bc761b0056`;
  validation `job_iq5_44896f380d` passed B19 5/5, full serial 116/116, eight
  default-parallel full repetitions, formatting, diff check, and warning-free
  locked build.
- Add a pure module-private public dispatch plan that consumes all raw argv after
  argv[0]. Exact first-token `update`, `doctor`, and `termux` become distinct
  Core routes and consume only that first token; each route retains every
  trailing raw `OsString` byte-for-byte for its later handler.
- Every other argv shape, including empty argv, `--version`, `-V`, `--`, exact
  upstream command names, near misses/affixes/case changes, and non-UTF-8 first
  tokens, becomes one Upstream route retaining the complete original argv in the
  original order. B20 must not synthesize sandbox args; B7/B14 remain the later
  launch boundary for that behavior.
- Reuse `classify_first_arg` as the exact classifier rather than introducing a
  second spelling table. The public route intentionally collapses all
  `CommandClass::Passthrough` shapes into Upstream; only the three SPEC-owned
  commands are intercepted.
- No route execution, `main` dispatch wiring, B19 invocation, final exec,
  generation/runtime discovery, Core/Manager health discovery, updater action,
  network, activation, rollback, numeric exit mapping, dependency addition, or
  product-state mutation belongs in B20.
- Focused `m1_b20_` tests must prove all three exact Core routes and exact tail
  preservation; complete upstream argv preservation for empty/version/near-miss/
  delimiter forms; raw non-UTF-8 first and trailing bytes; no accidental
  interception of `--version`/`-V`; deterministic planning with no environment
  side effects; and no mutation/reordering of caller-owned argv.
- Keep all 116 post-B19 tests green. Validate offline with an external Cargo
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
