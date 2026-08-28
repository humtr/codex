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
- Implementation worker: exactly one mutating worker at a time, bounded to the
  accepted code/test paths and validation; reused only within that bundle
- Planning agents, problem advisors, and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

### Bundle M1-B8 — explicit-input Termux base environment plan

- Prior accepted evidence: M1-B7 commit
  `5e5044eb3ae9286b72b16f1e1b9092f4e728bc82`.
- Exact outcome: add a module-private, std-only, pure environment-planning
  function/type that receives all path/environment inputs explicitly and returns
  deterministic child-environment assignments. It must not read `std::env`,
  inspect the filesystem, mutate the parent environment, construct a `Command`,
  call an exec primitive, select a product runtime/generation, or wire `main`.
- This is a bounded Termux base-compatibility plan, not a permanent public env
  API. `SPEC.md` requires a qualified runtime environment and selected official
  runtime/compatibility paths but does not fix these positive variable details.
  Sealed-predecessor read-only evidence from `job_htl_ad77938ce5`,
  `job_htr_d0247b30c9`, and `job_hu0_0299f115d7` observed the temp/certificate/
  PATH behavior below; B8 may implement that hypothesis for M1 validation, but
  it is not promoted to `SPEC.md` proof until later real-Termux qualification.
- Explicit planner inputs: selected compatibility-tool directory, actual Termux
  prefix `bin` directory, selected temp directory, selected certificate file,
  optional selected certificate directory, and inherited raw `PATH`,
  `SSL_CERT_FILE`, and `SSL_CERT_DIR` values supplied by the caller. Do not
  derive these from hard-coded app-data paths.
- Planned assignments:
  - `TMPDIR`, `TMP`, `TEMP`, and `SQLITE_TMPDIR` = the supplied temp directory.
  - `SSL_CERT_FILE` = inherited non-empty raw value when supplied, otherwise the
    supplied certificate file.
  - `SSL_CERT_DIR` = inherited non-empty raw value when supplied; otherwise the
    supplied optional certificate directory; if neither exists, plan no
    `SSL_CERT_DIR` assignment.
  - `PATH` = supplied compatibility-tool directory, then supplied prefix `bin`,
    then inherited non-empty raw PATH. If inherited PATH is absent/empty, do not
    create an empty trailing component.
- Raw-value rule: preserve non-UTF-8 inherited environment bytes on Unix/Android;
  do not use lossy conversion. PATH construction must reject or clearly report
  an input path component that cannot be represented safely in a Unix PATH
  rather than silently changing it.
- Explicit exclusions: B8 must not plan or mutate `HOME`, `CODEX_HOME`, XDG
  variables, `GODEBUG`, `BROWSER`, `CODEX_SELF_EXE`, Manager/profile/session
  variables, approval policy, or the five contamination removals already owned
  by B3. Do not infer a code-mode-host environment variable from predecessor
  internals before official runtime qualification.
- Required focused proof: deterministic exact assignments for all four temp
  variables; inherited-vs-fallback certificate precedence including empty-value
  cases; optional certificate-directory absence; exact PATH ordering with
  absent/empty inherited PATH; byte-for-byte preservation of a non-UTF-8
  inherited PATH; unusual synthetic explicit paths proving no hard-coded Termux
  app-data root; and negative assertions that every excluded variable is absent
  from the plan. Planner inputs/output must be independent of the current live
  process environment.
- Keep all 38 accepted B1-B7 tests green. Use `CARGO_NET_OFFLINE=true` and a
  repository-external `CARGO_TARGET_DIR` for `cargo fmt --check`, focused B8
  tests, `cargo test --locked --workspace`, and
  `cargo build --locked --workspace`.
- Worker configuration: one bounded `agy` CLI implementation worker in
  `accept-edits` mode through the Task-owned shell route, `fork_context: false`.
  No delegation, commits, pushes, package operations, network/update behavior,
  or live product state.
- Writable worker path: `crates/core/src/main.rs` only. No dependency, manifest,
  authority-document, or extra-file changes are authorized.
- Protected surfaces: every other repository path, all live resolver/runtime/
  launcher/Manager state, profiles, sessions, auth, Git refs, sealed legacy
  history, and unrelated worktrees.
- Explicitly deferred: applying the plan to the final `Command`, selected
  runtime/generation resolution, code-mode-host qualification, normal `main`
  wiring, doctor, generation/updater interfaces, installation, activation, and
  Manager behavior.
- Completion gate: actual diff contains only the pure planner and focused tests;
  no live env/filesystem read or mutation, path-policy decision, lossy raw-value
  conversion, dependency, public-surface expansion, or protected-state change;
  all named validation passes and the Lead reviews the plan as a bounded M1
  compatibility hypothesis rather than permanent public contract.

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
- Do not spawn a planning agent, problem advisor, checkpoint reviewer, or more
  than one implementation worker.
- Do not let a worker edit authority documents, commit, push, broaden its own
  read/write scope, or touch protected live state.
- Do not treat a worker report or worker-run test as acceptance proof until the
  primary Lead has inspected the actual diff and rerun load-bearing validation.
- Do not resend unchanged context or exceed the worker packet budgets in
  `GOAL.md`.
- Do not modify `legacy/monolith` or rewrite sealed tags.
- Do not expand the document hierarchy during ordinary implementation.

## Next milestone

Milestone 2 — delivery and recovery — remains defined in `SPEC.md` and is not
current work until the Milestone 1 ledger is complete. Completion of that ledger
causes the same primary Lead to replace this file's current target with the
Milestone 2 plan; it does not require a routine user stop. Exhaustion of an
accepted bundle before then causes the Lead to plan the next bundle, not to end
the task.
