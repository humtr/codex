# Rust Core Workboard

This file owns only the active implementation bundle. Accepted evidence and
historical disposition belong in `GOAL.md`; normative behavior belongs in
`SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Bound HEAD at B11 selection: `40d04dcb5a02687fc48a1897e36309c387edc91f`.
- Remote `origin/rewrite/rust-core` remains at
  `253156c37a2bd22af8faae0bce03587999ffd136`; the local branch is ahead and
  no push is authorized.
- Current milestone: M2 delivery/recovery is active; M2-B10 is accepted and
  M2-B11 is the selected bundle.
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

- B11 is qualification-first. Do not add production behavior unless a
  concrete fresh-environment or legacy-upgrade failure proves the accepted
  contract incomplete.
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
5. Close B11 only after grouped acceptance, release and formatting checks,
   protected-surface verification, authority update, and commit.
6. Reserve independent product review for the completed Milestone 2 acceptance
   candidate.

## Selected next action

### M2-B11 — isolated fresh-Termux and upgrade-from-legacy qualification

#### Outcome

Qualify the accepted prebuilt Rust Core and signed local artifacts on both a
fresh supported Termux environment and a separately provisioned disposable
legacy-upgrade environment. Demonstrate that installation, normal launch,
version/doctor checks, update activation, failure recovery, and rollback
remain usable without touching the current live installation.

#### Accepted input

- Accepted B10 tip: `40d04dcb5a02687fc48a1897e36309c387edc91f`.
- Accepted SPEC SHA-256:
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- Current Core, release-builder, and bootstrap source identities are recorded
  in the B10 ledger in `GOAL.md`.
- Use the B10 release Core artifact
  `12bd6c525026d74df8f9784444cebfee45d33f3f00af5512b59168f38a9c8d01` and
  release-builder artifact
  `d1db9e39f5b90dbf8f71e33b7f1a2fb80f6bd7399123278301328c77abeeb5ec` as the
  release-qualified inputs, or regenerate them in a private temporary root
  and record the new same-revision digests.
- The B11 slice 0 product-boundary proof passed in a private temporary target:
  the locked `-D warnings` release build succeeded, and the release Core
  rejected both `--sandbox=read-only` and `sandbox linux` with status 2 and the
  explicit bwrap non-enforcement error before any generation or activation
  state was created. This validates the product fail-closed boundary; the
  separate Codex-runner bwrap sandbox error is not a product-path result.
- The current device is observation-only for B11. Its live launcher and
  `resolv.conf` identities must remain unchanged.

#### Vertical proof map

| Slice | Observable outcome | Focused proof | State |
| --- | --- | --- | --- |
| 0 | Assemble the release-qualified Core, bootstrap, signed local manifest, and disposable environment inputs with network disabled where required. | Release-builder plus existing B10 artifact/signature checks; release-built Core sandbox fail-closed proof; record nonzero tests and exact digests. | complete |
| 1 | A fresh supported Termux root installs the prebuilt Core and can launch, report version, run doctor, and use the accepted local update/recovery path offline. | Fresh-root end-to-end qualification using only release inputs; inspect installed paths and state. | complete (isolated root) |
| 2 | A separately provisioned legacy root upgrades through the supported boundary while preserving required user state and exposing the accepted Core path. | Legacy-upgrade end-to-end qualification; compare only observable behavior and protected-state identities. | in progress (safe refusal complete; upgrade pending) |
| 3 | Failure injection, rollback, cleanup, and repeatability remain valid in both disposable roots, with no residue or live-state mutation. | B10 recovery regressions plus environment-specific checks and repeated bounded runs. | pending |
| 4 | The bundle is ready for the Milestone 2 independent product review. | Grouped locked suite, release build, formatting, shell syntax, diff review, protected-surface verification, and authority closure. | pending |

#### Slice 1 closure evidence

- The focused proof
  `tests::test_m2_b11_slice1_fresh_root_public_path_includes_doctor` ran
  against the current locked release-built Core with `-D warnings` and
  completed `1 passed, 0 failed`. It created a fresh disposable
  Termux-shaped HOME/PREFIX/TMPDIR root only.
- The release bootstrap exited 0; installed Core `--version` exited 0;
  `doctor` produced the expected healthy upstream / unavailable Manager /
  degraded summary (exit 1); signed offline local update exited 0; doctor
  remained usable after update; explicit rollback exited 0; and doctor
  remained usable after rollback.
- The network-denial sentinel was never touched. Final v3 state was
  `current=b11-fresh-g0`, `previous=b11-fresh-g1`; no activation
  transaction or bootstrap temporary residue remained.
- This isolated fresh-root slice does not claim a second physical Termux
  installation; that device-level qualification remains distinct under the
  SPEC acceptance principles.
- Current Slice 1 source SHA-256 is
  `8c897a4b93eae89bf69d4afb1625f28d083901889edc5f24c6734d29565f0a74`.
  The release Core artifact is
  `12bd6c525026d74df8f9784444cebfee45d33f3f00af5512b59168f38a9c8d01`;
  the release-builder artifact is
  `d1db9e39f5b90dbf8f71e33b7f1a2fb80f6bd7399123278301328c77abeeb5ec`;
  bootstrap is
  `c1b107699a64c08cc49a99ceb433c3dd1b6c7ca53637fb0b53cc73b6ce35e9fa`;
  and SPEC is
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.

#### Slice 2 preflight

- Read-only lookup found no second Termux app/prefix or disposable legacy root;
  only the protected live prefix is present.
- SPEC defines fresh bootstrap and public update/rollback, but no
  legacy-migration command or procedure. The bootstrap explicitly refuses
  existing authoritative v3 state and an existing differing
  `PREFIX/bin/codex`, so it cannot be treated as an upgrade protocol.
- The sealed legacy branch is historical evidence only; no legacy source or
  internal model was copied. The preflight exposed a concrete
  failure-atomicity gap in fresh bootstrap; only the shared entrypoint
  precheck and its focused regression were added, and no legacy upgrade
  protocol was invented.
- Slice 2 remains pending an explicitly provisioned disposable legacy
  environment and a specified supported upgrade boundary.

#### Slice 2 safe-boundary evidence

- The focused proof
  `tests::test_m2_b11_slice2_bootstrap_rejects_legacy_entrypoint_without_persistent_state`
  ran against the release-qualified Core with `-D warnings` and completed
  `1 passed, 0 failed`.
- The affected `bootstrap` focused group completed `8 passed, 0 failed`; it
  covered the B7/B8 bootstrap paths, the fresh Slice 1 path, and this
  legacy-entrypoint refusal path.
- Bootstrap now validates an existing entrypoint before publishing the
  bootstrap trust seed, then rechecks the same invariant before publication
  to cover the no-coordination race without adding an upgrade protocol.
- The legacy entrypoint bytes and mode remained unchanged; no network sentinel,
  trust seed, v3 activation state, or temporary bootstrap residue remained.
- Current Slice 2 source SHA-256 is
  `e0578a76aa41d8d22c205f76adbbbec0eebb5018dbe9c0387fcc5d55f68d985e`;
  bootstrap is
  `9924e12cd97dcabc4f9fe59b24a48ae4b2efb399d71a825c54161bf474470586`;
  the release Core remains
  `12bd6c525026d74df8f9784444cebfee45d33f3f00af5512b59168f38a9c8d01`;
  and the release-builder artifact remains
  `d1db9e39f5b90dbf8f71e33b7f1a2fb80f6bd7399123278301328c77abeeb5ec`.

#### Environment and safety boundary

- The qualification roots must be disposable and explicitly distinct from the
  current Termux prefix. Do not mutate `/data/data/com.termux/files/usr/bin/codex`,
  `/data/data/com.termux/files/usr/etc/resolv.conf`, installed generations,
  trust/key state, Manager state, auth, profiles, sessions, or package state.
- Do not install Rust, Cargo, Clang, or other build dependencies into a
  qualification root. The device-side input is the prebuilt release output.
- Network denial is part of the offline proof. Any network-dependent step must
  be explicitly isolated and recorded; normal launch must not depend on it.
- If an environment cannot be provisioned without touching protected live state,
  stop the slice and report the external-environment requirement. Do not
  weaken the contract or silently substitute a zero-test invocation.

#### B11 completion record

Pending. No production mutation is selected before the fresh and legacy
qualification results identify a concrete contract gap. On completion, move
accepted evidence and KEEP/COLLAPSE/DELETE disposition to `GOAL.md`, replace
this item with the next milestone action, and commit the closed bundle.
