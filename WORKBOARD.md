# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound M2-R2 implementation base:
  `rewrite/rust-core@a56a85cb88d3866ddc52134aae6dedaf88ac6c1f`.
- M2-R1 review follow-up and its three-finding remediation are accepted at
  `rewrite/rust-core@33b4bf3f6a4fcff7d2f7bbf67bb1d24b76b73d48`; detailed
  evidence and disposition are recorded in `GOAL.md`.
- M2-R2 independent-review remediation is accepted at the implementation
  commit above; detailed evidence and disposition are recorded in `GOAL.md`.
- The independent product review is complete at
  `rewrite/rust-core@5a7a5292f38876087a5c9b5a41b1dd7e8dbf082b`; the review-only
  clippy cleanup is included there and did not expand product behavior.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is accepted and locally promoted;
  the active bundle is the release install/update qualification and explicitly
  authorized local Termux cutover. Remote publication remains separate.
- The inherited pre-commit hook references absent
  `tools/update-wrapper-version.sh`; do not modify it. If it alone rejects an
  exact, revalidated staged tree, use the established `--no-verify` closure.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- The pre-promotion local `main` at
  `37f0a775ddc64d1641655a0cc83c0c2e681df704` was not contained by the sealed
  `legacy/monolith` history. It is preserved by the exact local backup branch
  `legacy/main-pre-m2-20260904`.
- `main` remains publication authority and is locally promoted by direct ref
  replacement to this accepted `rewrite/rust-core` tip; this is not a merge or
  rebase. Remote refs remain unchanged and no push or live cutover is included.
- Worker mode is OFF. The primary Lead owns implementation, validation,
  authority updates, commit, and acceptance.

## Product-speed policy

- The accepted B11 path is qualification-first. Its concrete legacy gap was the
  absence of an explicit safe handoff from a non-v3 entrypoint to the accepted
  Core; the implementation is limited to the exact bootstrap boundary in
  `SPEC.md`.
- Use a release-built, locked Core and signed local artifacts as the only
  product input. Build outputs stay in private temporary roots; no generated
  artifact is committed.
- The bwrap resolution is an explicit product boundary: Core does not invoke
  or repair bwrap on Termux. Ordinary launch uses the accepted no-sandbox
  policy; explicit unsupported Linux sandbox requests fail closed with a clear
  error; the official input archive may contain bwrap, but final generation
  selection consumes and discards that resource.
- Fresh-Termux and legacy-upgrade checks must target separate disposable
  environments. The current installed launcher, resolver, Manager state,
  auth/profile/session data, and package state are protected.
- Do not copy legacy implementation or internal data models into the rewrite.
  Inspect legacy behavior only through observable qualification outcomes.
- Do not introduce another fallback, promotion wrapper, trust source, or
  compatibility layer without first updating `SPEC.md` and mapping a focused
  regression.

## Mandatory execution gates

1. Rebind branch, HEAD, dirty state, authority revisions, and protected live
   identities before every resume.
2. Use the canonical project registry and revision-3 private project root for
   all disposable qualification environments.
3. Keep each slice vertical: production change if any, focused regression,
   nonzero focused invocation, relevant compile/test success, and diff review.
4. Stop on compilation failure, zero tests, stale expectations, unexpected
   warnings/dead paths, or missing proof mapping.
5. Each selected bundle must close with its mapped nonzero focused regression,
   relevant compile/test proof, and actual diff inspection before the next
   independent contract starts.
6. The final bundle batch must repeat grouped acceptance, protected-surface
   verification, authority updates, and the implementation commit.
7. Reserve a fresh independent product review for the completed Milestone 2
   acceptance candidate; this bundle directly closes the recorded findings.

## Next action

### M2 release install/update qualification and local cutover

- Bound source is `rewrite/rust-core@57034e4cd2d4f259c9046ac11073dc0b7f7dbb47`,
  also the local `main` promotion tip. The code acceptance evidence recorded
  above is reused because this bundle changes only the delivery frontend and
  qualification path until a product defect is found.
- Slice 1 — installation contract: update `SPEC.md` first; focused proof is
  `sh -n install.sh` plus a nonzero invalid-argument invocation. Protected
  surface: no target or live-state writes.
- Slice 2 — delivery frontend: add `install.sh` as an exact-argv `exec` into
  `bootstrap/codex-bootstrap`; focused proof must cover fresh and
  `upgrade-legacy` grammar forwarding and missing/symlinked bootstrap refusal.
- Slice 3 — disposable qualification: use a release-built Core, the official
  pinned upstream archive, and a job-private signing key to exercise fresh
  install, legacy handoff, `codex update --local`, explicit rollback, and
  failure/recovery. Reuse the existing Core update path; do not add a second
  updater or compile on the target.
- Slice 4 — authorized local cutover: verify the current legacy entrypoint
  digest, run `install.sh upgrade-legacy` against the authenticated release,
  then verify installed `codex --version`, `codex doctor --json`, local update,
  rollback readiness, state/generation integrity, and bwrap fail-closed
  behavior. Preserve resolver, auth/profile/session, Manager, package, and
  unrelated user state.
- Slice gate: stop on a zero-test invocation, compile/warning failure, any
  trust or generation mismatch, failed cleanup, unexpected bwrap invocation,
  or protected-surface identity change. Keep all build/release roots external
  and remove job-private signing material after qualification.
- No remote push or remote publication is part of this bundle. Worker mode
  remains OFF; the primary Lead owns implementation, validation, cutover, and
  acceptance.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
