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
- Delegate product-code and test implementation to one bounded implementation
  worker at a time.
- Perform an independent product review only after the Milestone 2 acceptance
  candidate is complete.

## Primary Sol Technical Lead and implementation workers

- Start or resume the goal with the primary agent configured as
  `gpt-5.6-sol` at `max` reasoning. That primary agent is the Technical
  Lead/Integrator; do not create a planning subagent to replace it.
- Keep the primary Lead across both Core milestones while its context remains
  available and accurate. On resume, it must rebind the exact branch, commit,
  `SPEC.md`, `GOAL.md`, and `WORKBOARD.md`. Do not persist a live agent identity
  in tracked files.
- The Lead reads the authorized repository evidence directly and owns
  architecture interpretation, bounded work-bundle planning, authority-document
  updates, worker instructions, actual diff review, integration validation,
  commits, and acceptance-ledger decisions. It must not outsource evidence
  selection or acceptance to the worker.
- The Lead does not routinely implement product code or tests. Before each
  implementation bundle it records the exact outcome, writable paths, governing
  contracts, tests, protected surfaces, and completion gate in `WORKBOARD.md`.
- Spawn only one implementation worker at a time with `fork_context: false`.
  Reuse that worker with `send_input` only within its current bundle; close or
  replace it after the bundle is accepted. Do not run concurrent workers in the
  shared worktree.
- A worker may use repository, filesystem, and shell tools only for the paths
  and temporary test roots authorized in its packet. It may modify product code
  and tests in that scope and run the named non-destructive validation. It must
  not edit `SPEC.md`, `GOAL.md`, `WORKBOARD.md`, or `AGENTS.md`; mutate live
  resolver/runtime/profile/session/auth/Manager state; install or update
  packages; use network/provider tools unless the current bundle and a later
  acceptance gate explicitly authorize one bounded non-secret validation;
  commit or push; or delegate work.
- The initial worker packet must contain the bound branch and commit, one exact
  accepted bundle, governing contract excerpts, no more than eight relevant
  source/test paths or snippets, authorized read/write scope, named validation,
  protected surfaces, and the required completion report. It must not exceed
  12,000 estimated input tokens or 48,000 characters without a tokenizer.
- Every worker follow-up is delta-only: include the prior and current commit,
  changed paths, only the relevant diff or new failure evidence, changed
  authority if any, and one exact next action. Do not resend unchanged material.
  A follow-up must not exceed 5,000 estimated input tokens or 20,000 characters
  without a tokenizer.
- The worker must return changed paths, concise validation results, unresolved
  failures, and confirmation that protected surfaces were not touched. If it
  needs evidence or scope outside the packet, it requests the exact item or path
  and waits for the Lead; it must not broaden its own scope.
- While a worker is mutating the shared worktree, the Lead waits and does not
  make concurrent edits. Afterward the Lead inspects the actual diff, reruns the
  load-bearing validation, and accepts or rejects the bundle. Worker summaries
  and tests are evidence inputs, not acceptance proof by themselves.
- Send rejected or incomplete work back to the same worker as an exact delta.
  Replace it only when its identity is confirmed unavailable or it cannot
  accurately restate the bundle and bound revision; a replacement receives the
  current diff and remaining gate, not a broad repository reread.
- The Lead handles planning changes and material diagnosis directly. Do not
  spawn another planner, problem advisor, or checkpoint reviewer during the two
  Core milestones. The Lead's own validation is integration verification, not
  independent review.
- When a bundle is accepted but the milestone gate is incomplete, the Lead
  plans and records the next bundle itself. When Milestone 1 is proven, the same
  Lead updates the ledger and workboard and continues into Milestone 2 without a
  routine user pause. A fresh independent reviewer is invoked only after the
  Milestone 2 acceptance candidate is complete.
