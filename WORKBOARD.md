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
  session; owns evidence retrieval, contract compilation, direct implementation
  while worker mode is OFF, actual diff review, integration validation, commits,
  and acceptance decisions across both Core milestones
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it. Do not invoke implementation workers or coding subagents while
  OFF
- Planning agents, problem advisors, and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone
- Click-inspired discipline: no Click plugin or Hook is installed. Within one
  selected bundle, reuse successful evidence while its revision remains current,
  do not reopen repository-wide discovery or replace the contract without new
  material evidence, use narrow implementation feedback only as needed, and run
  the repository-required acceptance suite as one final validation batch once
  the implementation is stable. Repository authority and required gates always
  override any verification-budget concept.

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

### Bundle M1-B23 — injected qualified public-dispatch execution

#### outcome

Compose the exact B20 public route into one Core execution boundary over an
explicitly supplied, already-qualified local context. Upstream invocations cross
B14 final exec, doctor invocations cross B19 ordered local doctor composition,
`termux` invocations cross B22 Manager handoff, and `update` is returned as a
bounded raw-argv handoff to the already-defined M1 updater interface without
performing live update behavior.

#### boundary

- in_scope: `crates/core/src/main.rs` only; one borrowed local dispatch context,
  one typed completion/error surface, and one dispatcher that consumes
  `PublicDispatchRoute` exactly once.
- out_of_scope: physical active-generation/current-pointer discovery, `main`
  wiring, live update/network/download/staging/activation/rollback, Manager
  discovery or features, new output/exit-code policy, dependencies, install,
  package operations, or live product/runtime/resolver/Manager mutation.

#### must_hold

- `PublicDispatchRoute::Upstream` passes its complete raw argv exactly once to
  B14 `launch_qualified_runtime`; no Core command token is removed or added at
  this layer.
- `Doctor` passes only the B20 trailing raw argv to B19; invalid usage still
  fails before runtime/resolver/config I/O and a successful result remains the
  bounded B15/B18 `DoctorCommandOutcome`.
- `Termux` passes only the B20 trailing raw argv to B22. `Unavailable` remains
  zero-exec and `Available` uses only the B21-qualified Manager executable.
- `Update` performs no I/O and does not inspect runtime/Manager/doctor inputs; it
  preserves every trailing raw `OsString` byte and order in a typed M1 handoff.
- Branch-specific errors remain distinct: upstream launch, doctor command, and
  Manager launch failures cannot be collapsed into fabricated success.
- The dispatcher does not discover, qualify, stat, hash, or rewrite any asset;
  all authority is injected through already-qualified wrappers and explicit
  read-only inputs.

#### build

- Add a borrowed `LocalPublicDispatchContext` containing B13 qualified runtime,
  B21 Manager qualification, B10 environment snapshot/certificate inputs,
  resolver/config paths, and the existing typed doctor capability/status inputs.
- Add a small `PublicDispatchCompletion` for `Update`, `Doctor`, and
  `TermuxUnavailable`, plus a `PublicDispatchExecutionError` that wraps existing
  branch-specific errors.
- Implement `execute_public_dispatch(route, context)` as one match with no new
  spelling table, discovery layer, filesystem policy, or handler abstraction.

#### verification

- focused: update zero-I/O/raw-byte preservation; upstream real exec through B14
  with complete raw argv; doctor invalid-usage-before-I/O plus one successful
  bounded result; termux unavailable zero-exec plus available real exec; typed
  branch-error preservation and no cross-route handler activity.
- done_when: focused B23 evidence passes; all 132 pre-B23 tests remain green;
  formatting and diff checks pass; locked offline build is warning-free; one
  grouped final acceptance batch runs the full serial suite and eight complete
  default-parallel repetitions. B14/B19/B22 focused suites are not rerun
  separately unless their underlying implementations are changed.

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
- Do not repeat a successful same-revision read/search or repository-wide
  inventory without material evidence that the prior observation is stale or
  insufficient; narrow the next observation instead.
- Do not replace the selected bundle with a new plan for an in-scope technical
  choice. Stop and revise authority only if the outcome, boundary, must-hold
  conditions, or required verification truly changes.
- The primary Lead must keep direct edits inside the selected bundle, inspect the
  actual diff, and run the grouped load-bearing validation before acceptance.
- Do not modify `legacy/monolith` or rewrite sealed tags.
- Do not expand the document hierarchy during ordinary implementation.

## Next milestone

Milestone 2 — delivery and recovery — remains defined in `SPEC.md` and is not
current work until the Milestone 1 ledger is complete. Completion of that ledger
causes the same primary Lead to replace this file's current target with the
Milestone 2 plan; it does not require a routine user stop. Exhaustion of an
accepted bundle before then causes the Lead to compile the next bounded contract,
not to end the task.
