# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Branch ancestry: independent empty-root lineage; do not merge or rebase
  `main` or `legacy/monolith`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone: Milestone 2 — delivery and recovery
- Primary Technical Lead/Integrator: the main `gpt-5.6-sol` / `max` goal
  session; owns evidence, direct implementation while worker mode is OFF, diff
  review, validation, commits, and acceptance
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it; do not infer a transition from workload or failures
- Planning agents, problem advisors, checkpoint reviewers, implementation
  workers, and coding subagents: disabled unless the user explicitly changes
  that policy
- Live product cutover: not authorized by the current bundle; mutable evidence
  remains under test-owned roots
- Click-inspired discipline: no Click plugin or Hook is installed. Reuse fresh
  evidence, avoid reopening settled decisions without new material evidence,
  and run load-bearing acceptance as one grouped batch once stable. Repository
  authority overrides generic workflow/verification-budget rules

## Product-speed policy

- Ship the smallest correct state machine. Release velocity is more important
  than speculative resilience once the core integrity invariants are met.
- The load-bearing invariants are: construct a generation completely before it
  can become active; activate atomically; never expose a mixed generation; keep
  one complete last-known-good recovery target; preserve protected user/system
  state.
- One installer/updater transaction is the normal model. Do not build locks,
  leases, fencing tokens, distributed coordination, or a multi-writer protocol
  merely because two installs could theoretically overlap.
- If overlapping install/update attempts occur, it is acceptable for one to
  fail or retry. Recovery may return to the already complete last-known-good
  generation. Simultaneous writers are not a separate availability product.
- Do not build fallback ladders. Prefer one explicit recovery path. If a basic
  invariant makes an existing check, retry, state field, or fallback redundant,
  remove the redundant mechanism rather than preserving both.
- New defensive logic requires a concrete reproducible failure not already
  covered by complete-generation staging, atomic activation, or last-known-good
  rollback. Extra defense is also extra defect/security surface.

## Current objective

Move directly toward an installable prebuilt Core: accept a qualified immutable
local release, stage one complete generation outside the active path, self-test
it, and activate it through the accepted M2-B1 transaction. Keep the path small
enough that the same machinery can become the fresh-install bootstrap and later
network update path without parallel fallback implementations.

## Selected next action

### Bundle M2-B2 — minimal local release/bootstrap path

#### outcome

Turn the M1 artifact/update evidence and the M2-B1 activation transaction into
the shortest end-to-end local install path under test-owned roots. A qualified
local immutable artifact becomes one complete staged generation, passes the
minimum required probe, and is atomically activated. At the same time, remove
any B1 pointer/recovery role that proves redundant with a simpler single
last-known-good recovery contract.

#### boundary

- in_scope: the minimum Rust Core code/tests needed to consume an explicit local
  qualified release input, materialize a test-owned candidate generation, bind
  its manifest/runtime assets, perform the required local probe, and call the
  accepted M2-B1 activation/recovery primitive; simplify redundant B1 defensive
  state while preserving its complete-generation and atomicity guarantees.
- out_of_scope: remote download, periodic update checks, package-manager use,
  multi-writer locking/fencing, distributed coordination, fallback ladders,
  Manager product features, publication refs, live installed-product mutation,
  or a second installer implementation.

#### must_hold

- Candidate generation is complete before activation and the active generation
  is never modified in place.
- A failed stage/probe/activation leaves a complete already-known generation as
  the only recovery target; no cascading fallback sequence is introduced.
- The local-artifact path is the bootstrap/update foundation, not a special test
  implementation that will later be replaced by a second path.
- The normal design assumes one updater/installer transaction. Do not add a
  kernel lock, lease, fencing token, or stale-writer protocol in this bundle.
- Basic launch/update overlap testing only needs to prove a launcher never sees
  a partially constructed generation. Simultaneous installer ordering is not a
  release gate.
- Existing B1 `verified`/`previous` roles must be traced once. If both are not
  required for distinct user-visible semantics, collapse the redundant role now
  instead of carrying defensive state forward.
- No live `$PREFIX/bin/codex`, live resolver, Manager state, auth/session/profile
  state, package state, or publication ref may be changed.

#### build

- Reuse M1 B11/B12/B13 qualification types and M2-B1 activation primitives;
  do not create parallel validators or a second transaction model.
- Prefer direct composition over adapters with retries/fallbacks. One failure
  should return one typed error and leave the accepted complete generation
  authoritative.
- Keep the bootstrap surface small: environment detection, qualified immutable
  local artifact acceptance, staging, probe, activation. Network retrieval and
  signature/key plumbing can feed the same path in later bundles.
- Delete or fold redundant defensive helpers/state encountered in this path when
  the load-bearing invariant already covers their purpose.

#### verification

- Focus on the direct happy path and load-bearing failures: valid local artifact
  install, malformed/incompatible input rejection before activation, stage/probe
  failure preserving the old complete generation, activation interruption using
  the already accepted B1 recovery proof, and one launch during candidate staging
  proving it still resolves only the active complete generation.
- Do not multiply edge-case matrices for hypothetical simultaneous installers.
- Acceptance: focused M2-B2 tests, full existing suite, formatting/diff checks,
  warning-free locked release build, and protected live resolver/launcher
  identity unchanged.

## Milestone 2 required outcomes

1. Produce prebuilt Android/Termux Core release artifacts.
2. Implement the minimal fresh-install bootstrap.
3. Implement signed immutable release manifests and key-rotation policy.
4. Acquire and safely adapt official upstream artifacts.
5. Implement atomic update, activation, recovery, and rollback.
6. Prove offline install and recovery.
7. Prove launches never observe a partial generation during update; do not make
   speculative simultaneous-installer coordination a release gate.
8. Qualify isolated fresh-Termux and upgrade-from-legacy paths.
9. Produce a complete candidate and run the fresh independent product review.

## Milestone 2 completion gate

- exact source, artifact digests, generation, test set, and device/runtime
  boundary are recorded for every release claim;
- signed release and key-rotation policy are enforced before candidate use;
- candidate generation is complete before activation and active state never
  resolves to a mixed generation after injected interruption;
- update/rollback/offline recovery are crash-safe; launch/update overlap proves
  launches resolve only complete generations, without requiring speculative
  simultaneous-installer coordination;
- fresh installation and legacy upgrade are demonstrated in isolated roots or
  devices without damaging the current working installation;
- prebuilt aarch64 Android/Termux artifacts require no on-device Rust toolchain;
- no resolver/auth/profile/session/Manager-owned state is damaged;
- the complete candidate passes the independent review gate before any
  publication authority is changed.

## Stop lines

- Do not add multi-writer fencing, lock hierarchies, lease protocols, fallback
  ladders, repeated retries, or duplicated validators without concrete evidence
  that the simpler invariant cannot handle the failure.
- Do not preserve defensive code solely because it already exists; remove it
  when a simpler foundational rule subsumes it.
- Do not let defensive hardening become the critical path to product release.
- Do not mutate the current installed Codex product or protected user/system
  state while implementing this bundle.
- Do not spawn additional agents/workers while worker mode is OFF.
- Do not repeat a successful current-evidence read/search or repository-wide
  inventory without material staleness; narrow the next observation instead.
- Do not modify `legacy/monolith`, sealed tags, publication refs, or the live
  installed launcher/runtime.

## Next milestone

There is no routine third Core milestone. After M2-B2, continue with the shortest
remaining path to prebuilt delivery, signed release input, offline recovery, and
fresh-install qualification. The complete Milestone 2 candidate, not defensive
feature count, controls the independent-review gate.
