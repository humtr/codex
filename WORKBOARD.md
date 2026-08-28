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

### Bundle M1-B11 — pure generation-manifest qualification interface

- Prior direct-Lead evidence: M1-B10 commit
  `08e67e8c9fed23032ff59c38ff4765221d515d67`; final validation
  `job_iba_22c23cddee` passed B10 6/6, the complete serial suite 59/59,
  eight full default-parallel repetitions, formatting, `git diff --check`, and
  the locked workspace build.
- Exact outcome: define the smallest dependency-free, in-memory generation
  manifest contract that binds every field required by SPEC section 6 and
  produces a distinct qualified-manifest wrapper only after compatibility and
  qualification validation. This bundle performs no filesystem or launch I/O.
- Manifest data model must bind: upstream package identity and version; immutable
  source artifact digest; expected platform and architecture; exact patch-policy
  identifier and patch report; resulting runtime digest; zero or more named
  helper digests; Core artifact digest; optional Manager artifact digest; Core
  API compatibility identity; persistent schema compatibility identity;
  qualification result; and creation metadata.
- Keep digest values opaque at this stage. Require them to be present/non-empty,
  but do not invent or freeze a digest algorithm, encoding, signature scheme, or
  serialized manifest representation. Likewise, patch reports and creation
  metadata remain opaque non-empty manifest-bound values.
- Introduce explicit validation requirements containing the platform,
  architecture, Core API identity, and persistent schema identity supported by
  the current Core. Validation must reject empty required fields, mismatches in
  those four compatibility bindings, rejected qualification status, empty helper
  identities/digests, duplicate helper identities, and an explicitly present but
  empty optional Manager digest.
- Successful validation returns a module-private `Qualified...` wrapper that
  borrows or owns the validated manifest without copying/normalizing its opaque
  values. Later runtime/path selection must be able to require this wrapper
  rather than an unvalidated manifest.
- Do not define generation physical paths, `current`/`verified`/`previous`
  pointer mechanics, runtime/helper executable paths, resolver/config paths,
  manifest serialization/parsing, signed release manifests, updater transport,
  anti-rollback, activation, rollback, doctor, or normal `main` wiring in B11.
- Production B11 code must perform no filesystem access, process environment
  read/write, Command construction, FD work, network/provider access, package
  operation, install, activation, or live-state mutation.
- Focused tests named `m1_b11_` must cover: one fully valid manifest; each of the
  four compatibility mismatches; rejected qualification; missing/empty required
  bindings across representative field classes; helper empty identity/digest and
  duplicate identity rejection; optional Manager absent/valid/empty behavior;
  exact qualified-wrapper retention of opaque/non-ASCII metadata; and planner
  purity/no side effects.
- Keep all 59 accepted post-B10 tests green. Validate with
  `CARGO_NET_OFFLINE=true`, a repository-external `CARGO_TARGET_DIR`, formatting,
  focused B11 tests, all locked workspace tests serially, default-parallel stress
  repetitions, and locked workspace build.
- Worker mode is user-controlled and remains OFF. The primary `gpt-5.6-sol` /
  `max` Lead directly implements this bundle. tmcp `harness.run` may be used only
  for bounded tests/validation, never for code mutation or development.
- Writable product path: `crates/core/src/main.rs` only. No dependency, manifest
  file, extra tracked file, live resolver/runtime/launcher/Manager, profile,
  session, auth, Git-ref, legacy-history, network, package, install, or activation
  change is authorized.
- Completion gate: the diff contains only the in-memory manifest/requirements/
  qualification types, validator, typed errors, and focused tests; no serialization,
  physical path, runtime selection, filesystem I/O, environment I/O, dependency,
  public-surface expansion, or protected-state change; Lead reruns focused/full/
  stress validation before acceptance.

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
