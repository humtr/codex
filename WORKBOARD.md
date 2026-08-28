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

### Bundle M1-B13 — pure qualified runtime/compatibility asset selection

- Prior direct-Lead evidence: M1-B12 commit
  `3927ad46696875c913c9039406693c1ddd4c3231`; final validation
  `job_iff_6b93404095` passed B12 11/11, the complete serial suite 80/80,
  eight full default-parallel repetitions, formatting, `git diff --check`, and a
  warning-free locked build.
- Exact outcome: define an in-memory qualification boundary that binds the
  runtime program, compatibility directory, and every selected helper asset to
  an already-qualified B11 generation manifest. B13 selects no active generation
  and performs no launch or filesystem I/O.
- Define one runtime asset binding with a raw native program path and an opaque
  observed digest. Define zero or more helper asset bindings with an opaque
  helper identity, raw native asset path, and opaque observed digest. Keep the
  compatibility directory as a separate raw native directory path because B8
  consumes it as a PATH component; do not infer it from helper paths.
- Runtime program, compatibility directory, and helper asset paths must be
  non-empty absolute paths. NUL-containing paths fail. The compatibility
  directory must also satisfy the existing B8 explicit PATH-component rule, so
  it cannot contain `:` on Unix. Do not canonicalize, stat, resolve symlinks, or
  require any physical filename/layout beyond these pure path-shape invariants.
- Runtime observed digest must be non-empty and byte-for-byte equal to the B11
  qualified manifest's `runtime_digest`.
- Every helper binding must have a non-empty identity and observed digest. The
  selected helper set must exactly match the qualified manifest helper set by
  identity and digest: reject missing helpers, extra helpers, duplicate selected
  identities, and digest mismatches. B11 already proves manifest-side helper
  identity uniqueness. Zero selected helpers is valid only for a manifest with
  zero helpers.
- Successful validation returns a borrowed `QualifiedRuntimeAssets` wrapper that
  retains the `QualifiedGenerationManifest`, runtime binding, compatibility
  directory, and helper slice without copying, UTF-8 conversion, path
  normalization, or digest transformation. Later runtime launch composition must
  require this wrapper rather than raw asset inputs.
- Do not implement generation `current`/`verified`/`previous` lookup, manifest
  serialization/parsing, digest computation, helper execution, runtime launch,
  resolver/config selection, environment capture, doctor, updater I/O,
  activation, rollback, network, package operations, or normal `main` wiring.
- Focused tests named `m1_b13_` must cover: valid runtime plus multi-helper set;
  zero-helper manifest; empty/relative/NUL runtime path; empty/relative/NUL/colon
  compatibility directory; empty runtime digest and digest mismatch; helper
  empty identity/path/digest; relative/NUL helper path; missing/extra/duplicate
  helper identities; helper digest mismatch; raw non-UTF8 Unix path retention;
  exact borrowed-wrapper pointer retention; deterministic/no-side-effect behavior.
- Keep all 80 accepted post-B12 tests green. Validate with
  `CARGO_NET_OFFLINE=true`, a repository-external `CARGO_TARGET_DIR`, formatting,
  focused B13 tests, all locked workspace tests serially, default-parallel stress
  repetitions, warning-free locked build, and `git diff --check`.
- Worker mode is user-controlled and remains OFF. The primary `gpt-5.6-sol` /
  `max` Lead directly implements this bundle. tmcp `harness.run` may be used only
  for bounded tests/validation, never for code mutation or development.
- Writable product path: `crates/core/src/main.rs` only. No dependency, manifest
  file, extra tracked file, live resolver/runtime/launcher/Manager, profile,
  session, auth, Git-ref, legacy-history, network, package, install, or activation
  change is authorized.
- Completion gate: the diff contains only raw asset-binding types, pure
  validation/promotion logic, typed errors, and focused tests; no physical state
  selection, filesystem/environment/Command/FD/network I/O, serialization,
  dependency, public-surface expansion, or protected-state change; Lead reruns
  focused/full/stress validation before acceptance.

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
