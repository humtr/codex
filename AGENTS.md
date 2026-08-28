# Agent Rules for humtr/codex

This repository contains a clean-room rewrite of the Termux compatibility
layer for upstream Codex. Keep the repository small and product-local.

## Authority

- `SPEC.md` owns normative product, architecture, command, state, security,
  update, and rollback contracts.
- `GOAL.md` owns the current success threshold and acceptance ledger.
- `WORKBOARD.md` owns only the current milestone and next implementation work.
- `README.md` is an entrypoint and must not introduce independent semantics.

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
- Checkpoint planning and review agents are disabled during the two Core
  milestones. Perform an independent product review only after the Milestone 2
  acceptance candidate is complete.
