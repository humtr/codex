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
- During each Core milestone, use one reusable read-only Sol Technical Lead for
  bounded checkpoint planning and material implementation-problem
  consultation. Additional planners and checkpoint reviewers are disabled.
- Perform an independent product review only after the Milestone 2 acceptance
  candidate is complete.

## Sol Technical Lead planning and consultation

- The primary implementing agent owns execution, routine local microplanning,
  evidence retrieval, mutation, validation, and all final decisions. It must
  validate Technical Lead proposals against `SPEC.md` and `GOAL.md` before
  updating `WORKBOARD.md` or implementing them.
- At the start of each milestone, create one read-only Technical Lead with
  model `gpt-5.6-sol`, reasoning effort `max`, and `fork_context: false`.
  Reuse that identity with `send_input` for the milestone's bounded work-bundle
  plans and for material problems that focused local diagnosis has not resolved
  confidently. Resume it when possible and do not spawn parallel or duplicate
  planning/advisory agents.
- Invoke the Technical Lead for the initial milestone bundle, after completion
  of the current bundle, after a material deviation, blocker, or authority
  change, and at the milestone transition. Do not invoke it for each source
  edit, test, commit, or other routine action inside an accepted bundle.
- Technical Lead access is packet-only. Instruct it not to call shell,
  filesystem, repository, web, MCP, delegation, or other tools and never to
  enumerate or scan the repository. The implementing agent owns all evidence
  retrieval.
- The initial packet must contain the bound branch and commit, one exact
  checkpoint or problem, governing contract excerpts, at most eight relevant
  source/test snippets or diffs, concise evidence and attempted actions when
  applicable, protected surfaces, and the decision needed next. The
  packet must not exceed 12,000 estimated input tokens, or 48,000 characters
  when no tokenizer is available.
- Every later `send_input` is delta-only: include the prior and current commit,
  changed paths, only the relevant diff or new evidence, and any changed
  authority. Do not resend unchanged material. A delta packet must not exceed
  5,000 estimated input tokens, or 20,000 characters without a tokenizer.
- If the packet is insufficient, the Technical Lead may request one exact
  additional snippet or evidence item. The implementing agent retrieves and
  returns only that item; the Technical Lead must not read it independently.
  Request an answer of no more than 900 words.
- The Technical Lead may analyze, recommend, and propose the next bounded
  checkpoint plan. It must not mutate files or runtime state, delegate, invent
  product semantics, authorize work, implement the plan, or review or certify
  milestone acceptance.
- Before entering a materially different bundle, the implementing agent records
  its validated disposition and executable bundle in `WORKBOARD.md`. Record a
  concise synthesis in `GOAL.md` when the plan or consultation changes the
  current plan, acceptance claim, blocker, or proof requirement.
- When an accepted bundle is exhausted but the milestone gate is incomplete,
  reuse the same Technical Lead to propose the next bounded bundle. When the
  gate is proven, the implementing agent records the evidence, rotates to a new
  Technical Lead for the next milestone, replaces the current `WORKBOARD.md`
  target, and continues. Exhausting a work bundle is not itself a blocker or a
  reason to stop for user input.
- Replace the Technical Lead only at a milestone boundary, when its identity is
  confirmed unavailable, or when it cannot accurately restate the bound commit
  and governing authority. Do not persist its live identity in tracked files.
