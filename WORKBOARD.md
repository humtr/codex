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
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is active; M2-R1 is accepted and
  M2-R2 is accepted after closing the three recorded independent-review
  findings. A fresh independent product review and final acceptance remain
  before promotion.
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
5. Each selected bundle must close with its mapped nonzero focused regression,
   relevant compile/test proof, and actual diff inspection before the next
   independent contract starts.
6. The final bundle batch must repeat grouped acceptance, protected-surface
   verification, authority updates, and the implementation commit.
7. Reserve a fresh independent product review for the completed Milestone 2
   acceptance candidate; this bundle directly closes the recorded findings.

## Next action

### M2 independent product review candidate

- M2-R2 independent-review remediation is accepted at
  `rewrite/rust-core@a56a85cb88d3866ddc52134aae6dedaf88ac6c1f`.
- Closed findings: active generation loading now rejects generation-root and
  selected asset symlink escapes; activation and explicit recovery writers
  share one kernel-held state-root lock without a persistent lock record;
  valid `doctor --json` probe/setup failure produces a redacted unhealthy
  machine report with nonzero health-failure status.
- Focused proof: M2-R2 `3 passed, 0 failed`; adjacent M2-B9 `4 passed, 0
  failed`; adjacent M2-B2 `6 passed, 0 failed`; and the corrected B4
  source-admission regression `1 passed, 0 failed`.
- Final grouped proof: locked workspace serial Core `113 passed, 0 failed,
  1 ignored` and release-builder `7 passed, 0 failed`; three independent
  default-parallel repetitions passed the same counts. Warnings-denied locked
  release build, formatting, bootstrap shell syntax, and `git diff --check`
  passed. Explicit real-Termux read-only smoke passed `1 passed, 0 failed`.
- Protected surfaces remain unchanged: live launcher and resolver identities
  remain the recorded hashes, and no installed generation/trust state,
  Manager, auth/profile/session data, package state, push, promotion, or live
  cutover was touched. All disposable roots were external and the final
  canonical residue scan is empty.
- Review is read-only until a fresh independent product review records its
  result. Do not promote to `main`, push, replace the live launcher, mutate
  the live resolver, or invoke bounded device qualification without explicit
  authorization.

- Review red gate resolved: `cargo clippy --locked --workspace --all-targets
  -- -D warnings` exposed five existing warning-denied findings in the Core
  production/test source (two `too_many_arguments`, one `needless_as_bytes`,
  and two `io_other`). The existing dispatch context now owns the doctor
  inputs, the one test helper is inline, and the remaining expressions use
  the direct forms; the corrected gate passes without expanding product
  behavior.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
