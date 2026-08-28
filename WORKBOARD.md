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

### Bundle M1-B10 — capture actual Termux process environment into the base-env planner

- Prior direct-Lead evidence: M1-R1 commit
  `4c1a8d90d6aa028106218d349076c465af8b8535`; final validation
  `job_i95_262b8f5b0d` passed 53/53 serial tests, eight full default-parallel
  repetitions, focused sandbox/FD suites, formatting, and locked build.
- Exact outcome: add the smallest Unix/Android process-environment boundary that
  feeds the proven B8 `TermuxBaseEnvInputs` without selecting a runtime or
  generation and without wiring normal `main`.
- Introduce a small module-private owned snapshot containing exactly `PREFIX`,
  `TMPDIR`, `PATH`, `SSL_CERT_FILE`, and `SSL_CERT_DIR` as raw `Option<OsString>`
  values, plus one thin reader using only `std::env::var_os` for those five keys.
- Keep selected `compat_dir`, fallback `cert_file`, and optional fallback
  `cert_dir` as explicit caller inputs. The snapshot-to-plan function must be
  pure: validate required process inputs, derive only
  `PathBuf::from(prefix).join("bin")`, build `TermuxBaseEnvInputs`, and call the
  existing B8 planner.
- `PREFIX` and `TMPDIR` are required and must fail with clear module-private
  typed errors when absent or empty. Preserve B8 planner errors as typed causes
  or variants; do not parse error strings.
- Do not canonicalize, stat, open, resolve symlinks, check existence, enforce a
  Termux package/application name, read `HOME`, or hard-code any app-data root.
  Do not derive a generation root, runtime executable, resolver path, config
  path, compatibility-tool path, or other product path.
- Preserve raw non-UTF-8 inherited PATH and certificate values byte-for-byte on
  Unix. The derived prefix bin path must use native path joining. Existing B8
  semantics remain: PATH order is explicit compatibility directory, derived
  prefix/bin, then inherited non-empty PATH; inherited non-empty certificate
  values win over explicit fallbacks.
- Production B10 code must not mutate the global process environment, construct
  or execute a `Command`, touch FD 33/34, read/write the filesystem, select a
  generation/runtime, parse a manifest, perform networking, or change public
  command semantics.
- Focused tests named `m1_b10_` must cover exact prefix/bin derivation and B8
  assignment order; absent/empty `PREFIX`; absent/empty `TMPDIR`; raw non-UTF-8
  inherited PATH/certificate preservation; an unusual synthetic prefix proving
  no fixed Termux app-data root; B8 error propagation; and a read-only capture
  check showing the snapshot matches those five current process variables
  without mutating them.
- Keep all 53 accepted post-M1-R1 tests green. Validate with
  `CARGO_NET_OFFLINE=true`, a repository-external `CARGO_TARGET_DIR`, formatting,
  focused B10 tests, all locked workspace tests serially, default-parallel stress
  repetitions, and locked workspace build.
- Worker mode is user-controlled and remains OFF. The primary `gpt-5.6-sol` /
  `max` Lead directly implements this bundle. tmcp `harness.run` may be used only
  for bounded tests/validation, never for code mutation or development.
- Writable product path: `crates/core/src/main.rs` only. No dependency, manifest,
  extra tracked-file, live resolver/runtime/launcher/Manager, profile, session,
  auth, Git-ref, legacy-history, network, package, install, or activation change
  is authorized.
- Completion gate: the diff contains only the bounded raw process snapshot,
  capture reader, pure snapshot-to-B8 composition, typed errors, and focused
  tests; no fixed app-data root, filesystem I/O, global env mutation, lossy
  conversion, product-path decision, dependency, public-surface expansion, or
  protected-state change; Lead reruns focused/full/stress validation before
  acceptance.

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
