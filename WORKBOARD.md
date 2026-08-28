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

### Bundle M1-B24 — raw public entrypoint and real-Termux smoke gate

#### outcome

Close the Milestone 1 local-Core execution proof without inventing Milestone 2
state layout. Add one raw-argv entrypoint composition that performs B20 planning
then B23 execution over an already-qualified local context, and add one explicit
ignored real-Termux smoke test that runs only when deliberately selected against
the current device's live resolver in read-only mode with all writable artifacts
under a test-owned temporary root.

#### boundary

- in_scope: `crates/core/src/main.rs` only plus validation commands; a thin
  `execute_public_entrypoint(raw_argv, context)` composition and test-only
  real-Termux smoke/snapshot helpers.
- out_of_scope: physical active-generation/current/verified/previous pointer
  implementation, installed `main` context discovery, live launcher/runtime/
  Manager replacement, updater network/download/activation/rollback, Manager
  product features, dependencies, package operations, or any write under
  `$PREFIX` except normal operating-system access metadata outside product
  control.

#### must_hold

- The entrypoint calls the existing B20 planner exactly once and passes the
  resulting route directly to B23; it has no second spelling table or branch
  semantics of its own.
- Exact `--version`/`-V` and arbitrary upstream raw argv remain upstream routes;
  exact `update`/`doctor`/`termux` retain the B20 first-token consumption rules.
- The explicit real-Termux smoke derives the resolver from the actual captured
  `PREFIX`, opens it only through the existing read-only FD33 path, uses only a
  temporary config directory/fake qualified runtime, and never executes or
  rewrites the installed public `codex` launcher.
- Smoke evidence snapshots the live resolver path target/content and stable stat
  identity before/after and fails on any change other than access-time metadata;
  validation also records an external SHA-256/stat snapshot of the resolver and
  installed launcher before/after the grouped acceptance batch.
- The smoke compares direct fake-upstream `--version` stdout/stderr/exit with the
  Core entrypoint path byte-for-byte while also proving FD33/34 availability in
  the Core path. No Core/Manager version text may be appended.
- No source test containing the live resolver is part of the default suite; the
  real smoke is ignored by default and must be invoked explicitly on the current
  Termux device.

#### build

- Add `execute_public_entrypoint` as a generic raw-`OsString` composition over
  `plan_public_dispatch` and `execute_public_dispatch`; do not modify `main` or
  add a production context provider.
- Add test-only protected-file snapshot logic using Unix metadata fields that are
  stable across reads (device/inode/mode/uid/gid/size/mtime plus symlink target
  when applicable) and exact content bytes; exclude atime from equality.
- Reuse the existing exec-probe process. The B24 scenario constructs one coherent
  qualified context using actual process env + live resolver and test-owned
  compatibility/config/runtime paths, then calls the raw public entrypoint with
  `--version`.

#### verification

- focused: B24 raw entrypoint route composition on zero-I/O Core routes and one
  explicit ignored real-Termux smoke proving direct-vs-Core version parity,
  FD33/34, resolver/launcher non-mutation, and test-owned writes only.
- done_when: the explicit real-Termux smoke passes on the current Termux device;
  all 138 pre-B24 default tests remain green; formatting/diff checks pass; a
  locked release build is warning-free; one grouped final acceptance batch runs
  the full serial suite and eight complete default-parallel repetitions while
  external resolver/launcher SHA-256 and stable-stat snapshots remain identical.
  If every Milestone 1 gate is then proven, the Lead records closure and replaces
  this Workboard target with the bounded Milestone 2 delivery/recovery contract.

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
