# Agent Rules for humtr/codex

This repository contains a clean-room rewrite of the Termux compatibility
layer for upstream Codex. Keep the repository small and product-local.

## Authority

- `SPEC.md` owns normative product, architecture, command, state, security,
  update, and rollback contracts.
- `GOAL.md` owns the current success threshold and acceptance ledger.
- `WORKBOARD.md` owns only the current milestone and next implementation work.
- `README.md` is an entrypoint and must not introduce independent semantics.
- At implementation start or resume, use the installed `$goal-md` skill to bind
  this repository's `GOAL.md`, then read `SPEC.md`, `GOAL.md`, and
  `WORKBOARD.md` in that order. The skill does not override these authorities.

When a proposed change alters a public command, ownership boundary, persistent
state, update/rollback behavior, security property, or Termux runtime contract,
update `SPEC.md` before implementation. Update `GOAL.md` when the success
threshold changes. Ordinary implementation detail belongs only in code, tests,
and the current `WORKBOARD.md` item.

Do not add a separate SDD, roadmap, lineage system, Design tree, or evidence
hierarchy unless the current documents can no longer express a concrete,
irreversible design decision without ambiguity.

## Branches

- `legacy/monolith` is sealed history at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`. Never commit to or rewrite it.
- `main` is the publication and release authority, not an implementation base.
- `rewrite/rust-core` is an independent orphan implementation lineage that
  begins with an empty root. Never merge, rebase, or otherwise import `main`
  or legacy history into it.
- Promotion replaces `main` with the accepted `rewrite/rust-core` lineage; it
  is not a merge between unrelated histories.
- Do not force-push `main` or delete published branches unless the user
  explicitly authorizes that exact operation.

## Rewrite discipline

- This is a rewrite, not a refactor or source migration.
- Do not copy legacy Bash, Python, C, tests, generated files, or internal data
  models into the new implementation.
- Legacy code may be inspected only to discover required observable behavior,
  failure cases, or safety constraints. Re-express accepted behavior in
  `SPEC.md` and new tests before implementing it.
- Prefer the smallest coherent Rust Core. Manager functionality remains behind
  the boundary defined in `SPEC.md`.
- Do not add abstraction layers, compatibility shims, or dependencies without
  a current milestone requirement.

## Outcome-first execution discipline

- At every start or resume, bind the exact branch, HEAD, dirty state, and current
  authority before interpreting prior reports or continuing work.
- Convert the user's directive into both the explicit requested outcome and the
  implied quality bar. Completion means satisfying the request **and** removing
  same-root-cause defects that would predictably make that result incomplete.
- A newly discovered class-level defect, duplicated proof layer, stale invariant,
  or simplification opportunity is a breadth trigger: inspect every relevant
  surviving instance across the current product path and affected prior bundles.
  Do not sample when the evidence says the problem is systemic.
- Trace findings backward to their root cause and forward to the real public
  product path. Do not stop at the first failing test, wrapper, or local symptom.
- Classify surviving machinery as KEEP, COLLAPSE, or DELETE. Prefer one direct
  product path and one foundational invariant over wrappers, promotion types,
  retries, fallback ladders, or defensive state that merely restates the same
  fact.
- A proof-only helper, test injection path, or internal wrapper cannot close a
  product requirement when the real entrypoint does not reach that behavior.
- Treat evidence rigorously: a zero-test invocation, false-positive audit, stale
  result, or mismatched revision is not acceptance evidence. Correct the check
  and rerun the exact load-bearing gate; reuse successful same-revision evidence
  instead of creating validation churn.
- Pull adjacent work into the current closure only when it follows from the same
  root cause, removes known product debt, or is necessary for the claimed public
  path. Do not turn proactive investigation into speculative resilience work or
  a new subsystem.
- Close each accepted bundle with implementation, focused regression, grouped
  full acceptance, protected-surface verification, authority update, and commit.
  Keep any later-bundle work separate so it cannot contaminate prior acceptance.
- Optimize for product progress: the preferred way to exceed the requested
  outcome is to discover and eliminate hidden blockers, stale assumptions, and
  unnecessary machinery on the same path, not to add more features or defenses.

## Vertical proof-slice discipline

- Divide each current `WORKBOARD.md` bundle into ordered contract slices. Each
  slice names one observable product behavior, the production and test paths it
  may change, its focused proof, protected surfaces, and its current state. This
  is the execution map for the current bundle, not a second roadmap or ledger.
- Before the first product mutation, establish a runnable baseline. On a dirty
  resume, bind the exact HEAD and source-diff identity and record every red gate
  in `WORKBOARD.md`. If the relevant test target does not compile, freeze new
  product behavior and restore runnable proof before continuing implementation.
- Close one slice vertically before starting another independent contract:
  implementation, focused regression, a nonzero focused invocation, relevant
  compile/test success, and actual diff inspection all belong to the same slice.
- Map every new production definition or branch to a named regression in the
  current slice. Production code without mapped proof is unfinished work and
  must not accumulate behind a later all-at-once test phase.
- Compilation failure, a zero-test command, a stale test asserting superseded
  behavior, an unexpected warning/dead path, or a missing proof mapping is a
  stop-on-red event. Stop adding behavior, diagnose the whole affected class,
  apply KEEP/COLLAPSE/DELETE, and restore the slice gate first.
- A broad structural change triggers an exhaustive disposition of every changed
  production definition and affected test/probe on that product path. Historical
  tests do not justify retaining compatibility shims or optional branches after
  the public contract has replaced their behavior.
- Run cheap compile and focused gates at slice boundaries. Reserve the grouped
  full suite, repeated parallel runs, protected-surface verification, and release
  build for the stabilized bundle's final acceptance batch.
- Keep the live slice/proof map only in `WORKBOARD.md`. At bundle close, reduce
  it to accepted evidence and disposition in `GOAL.md`, replace the Workboard
  item, and do not preserve a parallel evidence hierarchy.

## Safety

- Never modify `$PREFIX/etc/resolv.conf` or another system resolver file.
- Never mutate the installed Codex launcher/runtime, profiles, sessions, auth
  data, or Manager state while developing or testing unless a later acceptance
  gate explicitly authorizes a bounded device test.
- Never print or persist auth tokens, OAuth codes, cookies, credentials, or
  unredacted session content.
- Use temporary roots for all filesystem tests.
- Normal launch must remain usable when update checks or Manager components
  are unavailable.

## Validation and review

- Every implemented contract needs a focused regression test.
- Preserve upstream argv, TTY, signals, standard streams, and exit status at
  the final execution boundary.
- Fault-test generation activation and rollback before any live cutover.
- Run the two Core milestones under one primary Sol Technical Lead/Integrator
  that owns planning, implementation delegation, integration verification, and
  acceptance decisions. Additional planners, advisors, and checkpoint
  reviewers are disabled.
- Worker use is user-controlled. When worker mode is OFF, the primary Lead
  implements and validates directly and must not invoke implementation workers,
  planning agents, or checkpoint reviewers.
- Perform an independent product review only after the Milestone 2 acceptance
  candidate is complete.

## Primary Sol Technical Lead and user-controlled workers

- Start or resume the goal with the primary agent configured as `gpt-5.6-sol` at
  `max` reasoning. The primary Lead owns authority binding, architecture
  interpretation, implementation direction, actual diff review, validation,
  commits, and acceptance decisions.
- Worker mode is controlled only by an explicit user command. Repository history,
  previous worker usage, or an implementation bundle must never turn workers on
  implicitly.
- When worker mode is OFF, the primary Lead performs product-code, test,
  integration, diagnosis, and authority-document work directly. Do not invoke
  implementation workers, coding subagents, planners, problem advisors, or
  checkpoint reviewers.
- When worker mode is explicitly ON, use at most one bounded implementation
  worker in the shared worktree at a time. The primary Lead still owns the
  contract, writable scope, protected surfaces, actual diff review, load-bearing
  validation, commit, and acceptance decision. Worker summaries are evidence,
  not acceptance proof.
- Never run concurrent mutations in the shared worktree. Do not let a worker edit
  `SPEC.md`, `GOAL.md`, `WORKBOARD.md`, or `AGENTS.md`, mutate protected live
  state, install packages, push, or broaden its own scope unless the current
  user-authorized bundle explicitly permits that operation.
- A fresh independent reviewer is reserved for the Milestone 2 acceptance
  candidate or an explicit user request; routine bundle completion does not add
  another review layer.
