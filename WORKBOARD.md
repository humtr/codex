# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound M2-R1 implementation tip: bb083b41c61cb3112b70b68822940efe1fbcc89b.
- The bound tip is clean and is the exact starting point for the durability
  closure below. Its baseline is release-buildable, B11-focused green, and
  full-workspace green; no uncommitted product change is being carried in.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is active; M2-R1 closes the
  independent-review findings on the already accepted B10/B11 path. Promotion
  and independent review remain after this bounded closure.
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

### M2-R1 durability closure

Bound baseline: `bb083b41c61cb3112b70b68822940efe1fbcc89b`, clean on
`rewrite/rust-core`. The baseline gates are recorded in the implementation
turn: locked release build with warnings denied, B11-focused proof, serial
workspace acceptance, formatting/shell/diff checks, and protected-surface
verification all pass. The following slices are the live proof map for this
bundle:

1. **Generation publication durability** — define and implement final-file
   sync, bottom-up generation-tree sync, atomic publication, generation-root
   sync, and exact reuse after a post-rename sync failure. Focused proof:
   injected publication-boundary failures plus a public update retry that
   reuses the exact signed generation. Protected surfaces: state roots and
   live launcher remain untouched. Focused M2-R1 generation-fault and public
   reuse tests pass; M2-B3 6/6 and M2-B4 14/14 remain green. `completed`.
2. **Fresh bootstrap publication durability** — move trust-seed and stable
   entrypoint publication behind authenticated Core, preserving differing-target
   rejection and same-Core retry. Focused proof: trust-seed and stable-entrypoint
   parent-sync fault coverage through the public bootstrap script. Protected
   surfaces: only disposable prefix/home roots. Focused trust-seed and
   entrypoint retry tests pass; M2-B8 5/5 remains green. `completed`.
3. **Legacy handoff retry durability** — every successful completed retry must
   synchronize the entrypoint parent, including an already-Core target.
   Focused proof: post-rename parent-sync failure, repeated retry while the
   parent remains unsynchronized, then successful recovery. Focused retry
   test passes; M2-B11 9/9 remains green. `completed`.
4. **Builder final-mode durability and authority close** — sync runtime and
   descriptor files after their final modes are set; update README status and
   close accepted evidence in `GOAL.md`. Focused proof: release-builder mode
   and complete-output tests plus grouped acceptance. Builder focused proof
   passes 1/1 after final-mode sync changes. `completed`.
5. **M2-R1 acceptance batch** — run the release build, focused slices, serial
   full workspace suite, formatting/shell/diff checks, protected-surface
   verification, inspect the exact diff, then commit. No push, promotion,
   live launcher replacement, resolver mutation, or device qualification is in
   scope. Locked warnings-denied release build passed; serial workspace passed
   Core 103/0/1-ignored and builder 7/0; three default-parallel repetitions
   passed; protected launcher/resolver identities were unchanged. `completed`.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
