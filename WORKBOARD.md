# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound M2-R1 implementation base: f4fa0b53518f49cd7077d2765bf68f713dc38fe0.
- M2-R1 review follow-up is accepted at
  `rewrite/rust-core@084531b42bbcd6235c32393a576a09265269974e` and its evidence
  is recorded in `GOAL.md`.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is active; M2-R1 is accepted and
  independent product review is the next gate. Promotion remains after review.
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

### M2-R1 independent-review remediation — three findings

- Bound base: `rewrite/rust-core@c9de9bf36ea36b96ffc75cc998a8d80cf71b9cf7`.
  The current source tree is intentionally dirty only with this remediation;
  no unrelated changes are in scope.
- Slice 1 — fresh bootstrap authority and residue: the authenticated Core
  activation path must receive the supplied key/Core snapshots, bind the signed
  generation's `core_artifact_digest` to that Core, and resolve all v3
  transaction residue before trust-seed or entrypoint publication. Production
  paths are `bootstrap_self_test`, `run_internal_bootstrap_mode`, and
  `bootstrap_initial_signed_local_release`. Focused regressions are
  `test_m2_r1_fresh_bootstrap_preflights_transaction_residue_before_publication`
  and `test_m2_r1_fresh_bootstrap_rechecks_core_binding_after_source_swap`.
  State: implementation and focused proof green; actual diff review pending.
- Slice 2 — generation-root confinement: local update, fresh bootstrap,
  remote acquisition, installed verification, and ordinary loading must reject
  a symlink/non-directory generation root before any root-dependent mutation or
  existing-destination reuse. Production paths are the generation-root
  callers of `ensure_real_directory_tree`/`ensure_real_directory`.
  Focused regression is
  `test_m2_r1_fresh_bootstrap_rejects_symlink_generation_root_with_existing_destination`;
  the existing empty-root update regression remains required. State: pending
  final slice review.
- Slice 3 — bounded control/artifact reads: descriptor and manifest loads,
  bootstrap snapshots, bootstrap Core/key publication copies, and release-builder
  Core snapshots must enforce their normative byte limits before and during
  reads/copies. Production paths are `load_local_generation`, local manifest
  loading, `bootstrap/codex-bootstrap`, and `snapshot_core_artifact`.
  Focused regressions are
  `test_m2_r1_generation_descriptor_read_is_bounded_before_loading`,
  `test_m2_r1_bootstrap_snapshots_enforce_input_bounds_before_publication`, and
  the B8 Core-artifact rejection matrix. State: pending final slice review.
- Final batch: rerun the grouped locked workspace suite, repeated parallel
  suites, warnings-denied release build, formatting/diff checks, explicit
  real-Termux read-only smoke, and protected-surface identity checks. Then
  update `GOAL.md`, replace this bundle with the post-review gate, inspect the
  staged tree, and commit. Do not promote to `main`, push, replace the live
  launcher, mutate the live resolver, or invoke bounded device qualification.
- Protected surfaces: live launcher, resolver, installed generation/trust
  state, Manager, auth/profile/session data, package state, and publication
  refs remain untouched.

Worker mode remains OFF; the primary Lead owns every slice, validation step,
authority update, and acceptance decision.
