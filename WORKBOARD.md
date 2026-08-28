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
- Current milestone: Milestone 1 — local Core
- Primary Technical Lead/Integrator: the main `gpt-5.6-sol` / `max` goal
  session; owns evidence retrieval, planning, direct implementation while worker
  mode is OFF, actual diff review, integration validation, commits, and acceptance
  decisions across both Core milestones
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it. Do not invoke implementation workers or coding subagents while
  OFF
- Planning agents, problem advisors, and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

### Bundle M1-B12 — pure updater admission and candidate-qualification interface

- Prior direct-Lead evidence: M1-B11 commit
  `0eb9f6cd33951ff782c010d9e116ab886f70a815`; final validation
  `job_idt_e50502b44b` passed B11 10/10, the complete serial suite 69/69,
  eight full default-parallel repetitions, formatting, `git diff --check`, and a
  warning-free locked build.
- Exact outcome: define an in-memory, dependency-free updater interface that
  models the mandatory evidence gates in SPEC section 8 without resolving a
  release, downloading/staging an artifact, verifying cryptography, touching the
  filesystem, constructing a generation, or activating anything.
- Define an update source as either an immutable remote locator or an explicit
  local artifact path. Keep the remote locator opaque text and the local path as
  raw `OsStr`; require only non-empty identity/path here and do not parse URLs,
  canonicalize paths, stat files, or perform network/filesystem I/O.
- Define admission evidence that binds a non-empty signed-release manifest
  identity and its expected immutable source-artifact digest plus explicit
  satisfied/rejected verdicts for: release signature, architecture policy,
  Core-API policy, channel policy, and anti-rollback policy.
- Do not invent the signature algorithm, keyring, release-channel ordering,
  version ordering, anti-rollback counter/epoch, or comparison algorithm. Those
  verdicts are inputs from future bounded verifier/policy providers. B12's job is
  to fail closed unless every mandatory verdict is satisfied.
- Model updater resolver dependence explicitly: an independent resolver is
  accepted; sharing the patched runtime resolver is accepted only when a
  non-empty qualification identity is supplied. Shared-without-qualification is
  rejected before any later stage.
- Successful admission returns a distinct borrowed `AdmittedUpdateRequest`
  wrapper. No later B12 candidate qualification function accepts the raw request.
- Define staged-artifact evidence with explicit satisfied/rejected verdicts for
  artifact digest verification, archive safety, and compatibility metadata.
  Define candidate evidence using the existing B11 `QualifiedGenerationManifest`,
  a candidate-probe verdict, and verified-rollback-readiness verdict.
- Candidate qualification must reject every failed staged/probe/rollback verdict
  and reject when the B11 generation manifest's `source_artifact_digest` differs
  byte-for-byte from the admitted signed-release expected digest. Success returns
  a distinct borrowed activation-ready wrapper retaining the admitted request and
  qualified generation without copying or normalizing opaque bindings.
- Do not implement signed-release manifest serialization/parsing, signature/
  digest algorithms, archive extraction, private staging paths, generation
  construction, pointer mutation (`current`/`verified`/`previous`), activation,
  rollback, network access, automatic update scheduling, resolver I/O, package
  manager invocation, or normal `main` wiring in B12.
- Focused tests named `m1_b12_` must cover: valid remote and local admission;
  empty release identity/digest/source; every admission verdict rejection; shared
  resolver without/with qualification; every staged-artifact verdict rejection;
  source-digest mismatch; failed candidate probe; missing rollback readiness;
  successful activation-ready promotion preserving exact opaque/non-ASCII values
  and raw local path bytes; deterministic/no-side-effect behavior.
- Keep all 69 accepted post-B11 tests green. Validate with
  `CARGO_NET_OFFLINE=true`, a repository-external `CARGO_TARGET_DIR`, formatting,
  focused B12 tests, all locked workspace tests serially, default-parallel stress
  repetitions, warning-free locked build, and `git diff --check`.
- Worker mode is user-controlled and remains OFF. The primary `gpt-5.6-sol` /
  `max` Lead directly implements this bundle. tmcp `harness.run` may be used only
  for bounded tests/validation, never for code mutation or development.
- Writable product path: `crates/core/src/main.rs` only. No dependency, manifest
  file, extra tracked file, live resolver/runtime/launcher/Manager, profile,
  session, auth, Git-ref, legacy-history, network, package, install, or activation
  change is authorized.
- Completion gate: the diff contains only updater interface/evidence/wrapper
  types, pure validators/promotions, typed errors, and focused tests; no verifier
  implementation, serialization, physical state path, filesystem/environment/
  Command/FD/network I/O, dependency, public-surface expansion, or protected-
  state change; Lead reruns focused/full/stress validation before acceptance.

## Milestone 1 required outcomes

1. Create a minimal locked Cargo workspace with one Core binary and no unused
   dependency.
2. Implement exact first-argument routing for `update`, `doctor`, and `termux`;
   classify every other argv shape as upstream passthrough.
3. Prove `--version` and `-V` preserve exact upstream stdout, stderr, and exit
   status without Core version output.
4. Implement environment planning and final upstream execution with preserved
   argv, standard streams, TTY, signals, and exit status.
5. Open resolver/config sources read-only, map FD 33/34, preserve them across
   final exec, and prove the live resolver is unchanged.
6. Implement explicit unsupported-sandbox behavior without bwrap.
7. Implement read-only local doctor composition with redacted human and JSON
   output; unavailable Manager is represented explicitly.
8. Define and validate the generation-manifest and updater interfaces without
   performing network or live activation.
9. Add unit, integration, fault, and real-Termux smoke tests in temporary roots.
10. The primary Lead updates `GOAL.md` with exact Milestone 1 evidence. If every
    gate passes, it replaces this workboard's current target with its bounded
    Milestone 2 plan and continues.

## Milestone 1 completion gate

- clean locked release build on the current Termux device;
- all focused and integration tests pass;
- argv/TTY/signal/exit and FD33/34 contracts pass;
- doctor is read-only and secret-redacted;
- resolver stat, mode, content digest, and path are unchanged;
- no file under the live launcher/runtime/Manager paths changed;
- no legacy implementation source was copied;
- no network update or product activation occurred.

## Stop lines

- Do not begin Milestone 2 work while a Milestone 1 gate is unresolved.
- Do not implement Manager product features.
- Do not run package installation or update commands.
- Do not spawn a planning agent, problem advisor, or checkpoint reviewer. Do
  not invoke an implementation worker or coding subagent while user-controlled
  worker mode is OFF.
- The primary Lead must keep direct edits inside the selected bundle and must
  inspect the actual diff and rerun load-bearing validation before acceptance.
- Do not modify `legacy/monolith` or rewrite sealed tags.
- Do not expand the document hierarchy during ordinary implementation.

## Next milestone

Milestone 2 — delivery and recovery — remains defined in `SPEC.md` and is not
current work until the Milestone 1 ledger is complete. Completion of that ledger
causes the same primary Lead to replace this file's current target with the
Milestone 2 plan; it does not require a routine user stop. Exhaustion of an
accepted bundle before then causes the Lead to plan the next bundle, not to end
the task.
