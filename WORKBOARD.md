# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound R3 implementation base:
  `rewrite/rust-core@c9873ae6e678ad0af1d2ff4299a396536b6d51dd`.
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
- Current milestone: R6 signed wrapper publication is accepted on top of the
  accepted R5 official upstream build-input acquisition commit on
  `rewrite/rust-core`.
  R4's Core ownership, signed-generation admission, and doctor behavior remain
  the implementation baseline. Local `main` remains the publication authority;
  no second main promotion or remote push is authorized.
- The inherited pre-commit hook references absent
  `tools/update-wrapper-version.sh`; do not modify it. If it alone rejects an
  exact, revalidated staged tree, use the established `--no-verify` closure.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- The pre-promotion local `main` at
  `37f0a775ddc64d1641655a0cc83c0c2e681df704` was not contained by the sealed
  `legacy/monolith` history. It is preserved by the exact local backup branch
  `legacy/main-pre-m2-20260904`.
- `main` remains publication authority and was locally promoted by direct ref
  replacement to the previously accepted `rewrite/rust-core` tip; this is not a
  merge or rebase. Remote refs remain unchanged and no push was performed; the
  authorized local live cutovers are recorded in `GOAL.md`.
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

R6 is accepted: the non-installed Rust release builder now validates the R5
first-target generation, emits a Core-compatible v3 manifest and stable signed
index, signs both with an explicitly supplied Ed25519 private key, and
atomically produces `releases/<generation-id>/` plus the index files. Focused
builder and Core admission regressions prove the real format, signatures,
inventory, modes, URL identity, source preservation, collision handling,
symlink rejection, and staging cleanup. Detailed evidence is recorded in
`GOAL.md`.

The next separately authorized operation is external publication of a real
release output using the active Core `update_key` private key. No upload to
OpenAI or the wrapper distribution surface was performed by R6; live
`codex update` must remain fail-closed until an operator publishes the output
and the installed runtime can fetch it.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
