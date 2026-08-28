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
- Checkpoint planning and review agents are disabled during the two Core
  milestones. Perform an independent product review only after the Milestone 2
  acceptance candidate is complete.

## Implementation problem consultation

- The planning/review prohibition does not disable bounded consultation when
  implementation reaches a material problem that focused local diagnosis has
  not resolved confidently.
- On the first such problem in a milestone, use one read-only advisor with
  model `gpt-5.6-sol`, reasoning effort `max`, and `fork_context: false`.
  Reuse that same advisor for later problems in the milestone with
  `send_input`; resume it if it was closed. Do not spawn parallel advisors.
- Advisor access is packet-only. Instruct it not to call shell, filesystem,
  repository, web, MCP, delegation, or other tools and never to enumerate or
  scan the repository. The implementing agent owns all evidence retrieval.
- The initial packet must contain the bound branch and commit, one exact
  problem, governing contract excerpts, at most eight relevant source/test
  snippets or diffs, concise failure evidence and attempted actions, protected
  surfaces, and the decision needed next. The
  packet must not exceed 12,000 estimated input tokens, or 48,000 characters
  when no tokenizer is available.
- Every later `send_input` is delta-only: include the prior and current commit,
  changed paths, only the relevant diff or new evidence, and any changed
  authority. Do not resend unchanged material. A delta packet must not exceed
  5,000 estimated input tokens, or 20,000 characters without a tokenizer.
- If the packet is insufficient, the advisor may request one exact additional
  snippet or evidence item. The implementing agent retrieves and returns only
  that item; the advisor must not read it independently. Request an answer of
  no more than 900 words.
- The advisor may analyze and recommend. It must not mutate files or runtime
  state, delegate, invent product semantics, authorize work, or act as a
  checkpoint planner or product reviewer.
- The implementing agent remains responsible for verifying the advice and
  choosing the action. Record a concise consultation and disposition in
  `GOAL.md` only when it changes a contract, current plan, acceptance claim,
  blocker, or proof requirement.
- Replace the advisor only at a milestone boundary, when its identity is
  confirmed unavailable, or when it cannot accurately restate the bound commit
  and governing authority. Do not persist its live identity in tracked files.
