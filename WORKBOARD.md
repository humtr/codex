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

### Bundle M1-B21 — optional Manager artifact qualification boundary

#### outcome

Provide the Core-side authority boundary required for the exact `termux` public
route to distinguish an explicitly unavailable Manager from a Manager artifact
that is bound to the already-qualified generation. This bundle does not invoke
Manager and does not implement Manager UX or product behavior.

#### boundary

- in_scope: `crates/core/src/main.rs` only; pure types and validation that consume
  `QualifiedGenerationManifest` plus an optional explicit Manager artifact
  selection containing a raw path and observed digest.
- out_of_scope: Manager implementation, Manager process spawn, `main` wiring,
  generation discovery/current pointers, filesystem reads/stat/digest
  computation, network, update/activation/rollback, dependency additions, live
  product/runtime/resolver/Manager mutation, or numeric process-exit mapping.

#### must_hold

- A generation with no `manager_artifact_digest` plus no selected artifact maps
  to one explicit `Unavailable` state rather than fabricated success.
- A generation that declares a Manager digest requires exactly one explicit
  selected artifact with a nonempty absolute NUL-free raw path and a nonempty
  observed digest equal to the manifest digest before it can become `Available`.
- A selected artifact when the manifest declares no Manager, or a missing
  selection when the manifest declares one, fails closed with distinct typed
  errors.
- Digest mismatch and invalid path shape fail before any future execution could
  receive a Manager artifact.
- Raw non-UTF-8 absolute paths remain byte-exact; no lossy conversion, process
  environment access, filesystem I/O, or mutation is introduced.

#### build

- Reuse `QualifiedGenerationManifest` as the only generation authority and the
  existing B13 absolute-path validation semantics instead of adding a second
  path policy.
- Add a small optional selection type, a qualified `Available` wrapper plus
  explicit `Unavailable` result, and typed qualification errors.
- Keep the result borrowed/opaque so B22 can compose `PublicDispatchRoute::Termux`
  without rediscovering or revalidating artifact identity.

#### verification

- focused: tests for absent/absent -> Unavailable, present/matching -> Available,
  both manifest/selection disagreement directions, empty/mismatched digest,
  relative/empty/NUL paths, raw non-UTF-8 retention, determinism, and no process
  environment side effect.
- done_when: focused B21 tests pass; every pre-B21 workspace test remains green;
  formatting and diff checks pass; locked offline build is warning-free; final
  acceptance uses one grouped repository-required batch including the existing
  full serial suite and default-parallel stress repetitions.

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
