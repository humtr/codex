# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound M2-R2 implementation base:
  `rewrite/rust-core@7356f4883af2697dc67e6c6fab04ca9da3fb5086`.
- M2-R1 review follow-up and its three-finding remediation are accepted at
  `rewrite/rust-core@33b4bf3f6a4fcff7d2f7bbf67bb1d24b76b73d48`; detailed
  evidence and disposition are recorded in `GOAL.md`.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is active; M2-R1 is accepted and
  M2-R2 closes the three recorded independent-review findings. Promotion
  remains after repaired review and final acceptance.
- The inherited pre-commit hook references absent
  `tools/update-wrapper-version.sh`; do not modify it. If it alone rejects an
  exact, revalidated staged tree, use the established `--no-verify` closure.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- `main` remains publication authority. No merge, promotion, push, or live
  cutover is authorized by this bundle.
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
5. Each M2-R2 slice must close with its mapped nonzero focused regression,
   relevant compile/test proof, and actual diff inspection before the next
   independent contract starts.
6. The final M2-R2 batch must repeat grouped acceptance, protected-surface
   verification, authority updates, and the implementation commit.
7. Reserve a fresh independent product review for the completed Milestone 2
   acceptance candidate; this bundle directly closes the recorded findings.

## Next action

### M2-R2 independent-review remediation

- This bundle is bound to
  `rewrite/rust-core@7356f4883af2697dc67e6c6fab04ca9da3fb5086` and addresses
  the recorded review findings: active-generation nested symlink escapes,
  stale multi-writer activation journals, and missing `doctor --json` failure
  envelopes.
- Baseline before product mutation: Core `110 passed, 0 failed, 1 ignored`
  and release-builder `7 passed, 0 failed` under a fresh external target.
- Slice 1 — active generation confinement: validate the selected generation,
  asset parents, and selected leaves; add a real public-main regression for
  current-generation, runtime, compat, Manager, and helper symlink escapes.
  State: completed — exact focused Core test `1 passed, 0 failed`; the public
  probe exercised all six symlink cases and returned the bounded Core error.
- Slice 2 — activation writer coordination: serialize activation and explicit
  recovery with the state-root kernel lock; add contention/no-mutation and
  lock-release regression coverage. State: completed — exact focused Core test
  `1 passed, 0 failed`; activation and recovery returned `WriterBusy` without
  transaction files, and activation succeeded after lock release.
- Slice 3 — doctor machine failure contract: render a redacted unhealthy
  upstream status after valid JSON-mode probe/setup failure; add a public-main
  `doctor --json` regression. State: completed — exact focused Core test
  `1 passed, 0 failed`; missing resolver returned an unhealthy JSON envelope
  with nonzero health-failure status and no path/error leakage.
- Grouped slice acceptance: M2-R2 `3 passed, 0 failed`; adjacent M2-B9
  overlap/recovery `4 passed, 0 failed`; adjacent M2-B2 loader/public-route
  `6 passed, 0 failed`. Final full suite, protected-surface verification,
  authority update, and implementation commit: in progress. No push,
  promotion, live cutover, or bounded device qualification is in scope.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
