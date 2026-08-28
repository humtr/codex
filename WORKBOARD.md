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
  session; owns evidence retrieval, planning, worker packets, actual diff review,
  integration validation, commits, and acceptance decisions across both Core
  milestones
- Implementation worker: disabled; the primary Lead directly implements the
  bounded code/test changes in this Workboard
- Planning agents, problem advisors, and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

### Bundle M1-B10 — capture actual Termux process environment into the base-env planner

- Prior accepted evidence: M1-B9 commit
  `692cd8b0c9cc4babe273ab9bdfa9d14eabc9db0c`.
- Exact outcome: add the smallest Unix/Android-only process-environment boundary
  that can feed B8's pure `TermuxBaseEnvInputs` and therefore B9's final-exec
  composition without embedding a single Termux app-data path or selecting a
  runtime/generation. This bundle still does not wire normal `main`.
- Keep `compat_dir`, fallback `cert_file`, and optional fallback `cert_dir` as
  explicit caller inputs. Read only `PREFIX`, `TMPDIR`, inherited `PATH`,
  `SSL_CERT_FILE`, and `SSL_CERT_DIR` from the current process. Derive only the
  prefix bin directory from `PREFIX` using native path joining; do not derive a
  generation root, runtime executable, managed config directory, resolver path,
  HOME-owned Core state, or compatibility-tool location.
- Separate process I/O from planning: introduce a small owned snapshot type and
  one thin process reader using raw `std::env::var_os`; introduce a pure
  snapshot-to-plan composition that validates required process inputs and calls
  the accepted B8 planner. Do not make B8 itself read global process state.
- `PREFIX` and `TMPDIR` are required and must fail clearly when absent or empty.
  Do not add speculative canonicalization, existence checks, symlink resolution,
  app-package-name checks, or hard-coded `/data/data/com.termux` policy. Preserve
  inherited non-UTF-8 `PATH` and certificate values byte-for-byte on Unix.
- The derived prefix bin path must be exactly native `PREFIX` joined with `bin`.
  The existing B8 PATH order remains selected `compat_dir`, derived prefix/bin,
  then inherited non-empty PATH. The B8 certificate precedence remains unchanged:
  inherited non-empty values win over the explicit fallback values.
- Production B10 code must not read the filesystem, mutate the process
  environment, construct or execute a `Command`, open FD 33/34, select a
  generation, parse a manifest, inspect live runtime state, perform networking,
  or change public command semantics.
- Required focused tests named `m1_b10_`: pure synthetic snapshots proving exact
  prefix/bin derivation and B8 assignment order; absent/empty `PREFIX` and
  `TMPDIR` errors; raw non-UTF-8 inherited PATH/certificate preservation; and a
  synthetic unusual prefix proving no fixed Termux app-data root is embedded.
  Add one subprocess-only process-reader proof if needed so tests never mutate
  the parent test process environment.
- Keep all 50 accepted B1-B9 tests green. Validation uses
  `CARGO_NET_OFFLINE=true` and a repository-external `CARGO_TARGET_DIR` for
  formatting, focused B10 tests, all locked workspace tests, and locked workspace
  build. tmcp `harness.run` may be used only to execute bounded tests/validation;
  it must not be used for implementation or product-code mutation.
- Implementation owner: the primary `gpt-5.6-sol` / `max` Lead directly edits
  `crates/core/src/main.rs` for this bundle. No implementation worker or coding
  subagent is authorized. No dependency, manifest, or extra tracked-file change
  is authorized.
- Protected surfaces: every other repository path, all live resolver/runtime/
  launcher/Manager state, profiles, sessions, auth, Git refs, sealed legacy
  history, and unrelated worktrees.
- Explicitly deferred: generation-manifest schema and selection, runtime and
  compatibility-tool selection, resolver/config path selection, normal `main`
  wiring, doctor, updater interfaces, installation, activation, and Manager
  behavior.
- Completion gate: actual diff contains one thin raw process reader plus pure
  snapshot-to-B8 composition and focused tests; no filesystem I/O, global env
  mutation, lossy conversion, fixed app-data path, runtime/generation decision,
  dependency, public-surface expansion, or protected-state change; Lead reruns
  focused and full validation before acceptance.

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
- Do not spawn a planning agent, problem advisor, checkpoint reviewer, or
  implementation worker.
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
