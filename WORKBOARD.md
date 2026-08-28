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

### Bundle M1-B6 — explicit Termux sandbox policy planner

- Prior accepted evidence: M1-B5 commit
  `85f312b7d5d0e2e8a14c9084063e437633b63480`.
- Exact outcome: add a small std-only production argument-planning function for
  upstream passthrough. It must reject explicit Linux sandbox requests that the
  Termux product cannot enforce and, for accepted passthrough, prepend exactly
  `-c` and `sandbox_mode="danger-full-access"` before the original raw argv.
  This bundle does not wire `main` or choose a runtime executable/path.
- Writable worker path: `crates/core/src/main.rs` only. No dependency, manifest,
  file, or authority-document changes are authorized to the worker.
- Governing contract: `SPEC.md` section 5 says Linux namespace/bwrap sandboxing
  is not a product capability, `read-only`/`workspace-write` requests must fail
  clearly rather than be silently weakened, and ordinary supported launch uses
  the explicitly selected upstream no-sandbox policy. Approval policy remains
  upstream-controlled; Core must not synthesize an approval-bypass flag.
- Sealed predecessor evidence is discovery only, not rewrite proof. Lead jobs
  `job_hr7_58317ca55f` and `job_hr9_78251f660c` observed these prior public
  request forms: leading `sandbox linux`; `-s VALUE`; `--sandbox VALUE`;
  `--sandbox=VALUE`; attached `-sread-only`/`-sworkspace-write`; `-c VALUE` or
  `--config VALUE` where the value requests `sandbox_mode` read-only or
  workspace-write; and `--config=sandbox_mode=...`. Scanning stops at the exact
  `--` separator. `danger-full-access` itself is allowed.
- Parsing boundary: inspect only UTF-8 option tokens needed to recognize the
  ASCII policy forms; preserve every accepted original `OsString` byte-for-byte
  and treat non-UTF-8/unrecognized argv as ordinary upstream input. Do not use
  lossy conversion. Missing values remain upstream usage concerns rather than
  becoming Core guesses.
- Required behavior/tests: reject both `read-only` and `workspace-write` in each
  observed form with a clear Termux/Linux-sandbox error; reject leading
  `sandbox linux` before any launch planning; allow `--sandbox
  danger-full-access`; stop policy scanning after `--`; preserve arbitrary and
  non-UTF-8 original args exactly; prepend only the two declared config args;
  and prove the planner never synthesizes
  `--dangerously-bypass-approvals-and-sandbox`.
- Keep all 26 accepted B1-B5 tests green. Use `CARGO_NET_OFFLINE=true` and a
  repository-external `CARGO_TARGET_DIR` for `cargo fmt --check`,
  `cargo test --locked --workspace`, and `cargo build --locked --workspace`.
- Worker configuration: one bounded `agy` CLI implementation worker in
  `accept-edits` mode through the Task-owned shell route. No delegation,
  commits, pushes, package operations, network/update behavior, or live state.
- Explicitly deferred: normal `main` dispatch/wiring, runtime/generation/config
  path selection, actual official upstream artifact qualification, doctor,
  manifest/updater interfaces, installation, activation, and Manager behavior.
- Protected surfaces: every repository path except `crates/core/src/main.rs`,
  all live resolver/runtime/launcher/Manager state, profiles, sessions, auth,
  Git refs, sealed legacy history, and unrelated worktrees.
- Completion gate: planner behavior is deterministic and raw-argv preserving;
  unsupported requests fail before an exec primitive is called; accepted argv
  receives only the selected no-sandbox config prelude; no dependency/public
  surface expansion; all named validation passes; status contains only the
  authorized source file.
- Integration disposition: the primary Lead reviews every recognized/rejected
  argv shape against SPEC plus the bounded discovery evidence, reruns full and
  focused validation, and commits only accepted work.
- Lead correction and acceptance: the first B6 result was rejected because it
  exposed the planner error publicly, recognized unobserved attached `-c...` and
  literal-quote flag forms, and could reinterpret a separate option's value as a
  later policy option. The bounded correction keeps planner/error types private,
  recognizes only the observed sandbox/config forms, requires an exact
  `sandbox_mode` config key, consumes exactly one following value token for
  separate `-s`/`--sandbox` and `-c`/`--config` forms, stops at exact `--`, and
  preserves accepted raw `OsString` argv byte-for-byte after the exact two-arg
  no-sandbox prelude. Primary-Lead validation job `job_ht1_c593e9a07a` passed
  `cargo fmt --check`, all 34/34 workspace tests, three additional serial
  repetitions of the 10 `passthrough_` focused tests, and
  `cargo build --locked --workspace` with offline mode and a repository-external
  Cargo target. No public launch wiring, dependency, extra file, approval-bypass
  synthesis, or live state change was introduced. M1-B6 is accepted for commit.

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
