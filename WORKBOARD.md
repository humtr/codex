# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound M2-R1 implementation base: f4fa0b53518f49cd7077d2765bf68f713dc38fe0.
- Review follow-up is being validated on
  `rewrite/rust-core@d90e3f080c88f51b29bc2c1fc1949d48cdeb9afd` with one
  uncommitted Core publication change; the base M2-R1 closure remains accepted
  in `GOAL.md` until this follow-up is closed.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is active; the M2-R1 review follow-up
  is the current implementation slice, after which independent product review
  remains the next gate. Promotion remains after review.
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
5. Each M2-R1 slice must close with its mapped nonzero focused regression,
   relevant compile/test proof, and actual diff inspection before the next
   independent contract starts.
6. The final M2-R1 batch must repeat grouped acceptance, protected-surface
   verification, authority updates, and the implementation commit.
7. Reserve independent product review for the completed Milestone 2
   acceptance candidate.

## Next action

### M2-R1 review follow-up — generation collision race

- Observable contract: immutable generation publication must never replace a
  complete generation that appears after the pre-publish existence check.
- Production path: `FsGenerationPublishIo::rename` uses the existing Unix
  no-replace primitive; the publish error maps `AlreadyExists` to the existing
  `GenerationCollision` outcome.
- Focused regression: `test_m2_r1_generation_collision_race_never_replaces_existing_directory`
  injects an existing destination between the check and publish, verifies the
  sentinel remains intact, and verifies candidate cleanup.
- Slice state: focused compile/test is green (`2/2` M2-R1 generation tests);
  grouped acceptance, protected-surface verification, actual diff review, and
  commit remain pending.
- Protected surfaces: live launcher, resolver, installed generation/trust
  state, Manager, auth/profile/session data, and publication refs remain
  untouched.

### M2 independent product review candidate

After the follow-up is accepted, review `SPEC.md`, `GOAL.md`, and the
committed M2-R1 product path as one candidate. Keep review read-only until
findings are recorded. Do not promote to `main`, push, replace the live
launcher, mutate the live resolver, or invoke bounded device qualification
without explicit authorization.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
