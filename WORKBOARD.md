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

### Bundle M1-B5 — TTY and external-signal fidelity evidence

- Prior accepted evidence: M1-B4 commit
  `bb21ddca58589ec77a22e824c4218db5c1087daa`.
- Exact outcome: add focused private subprocess tests proving the existing
  production `exec_upstream` boundary preserves an attached terminal on stdin,
  stdout, and stderr and preserves the process identity needed for an external
  `SIGTERM` sent after exec to reach the upstream process. No product behavior
  change is expected or authorized unless a test exposes a real defect.
- Writable worker path: `crates/core/src/main.rs` only; tests remain private to
  the Rust test binary and may not add public flags, subcommands, or output.
- Governing contract: `SPEC.md` section 5 requires preservation of stdin,
  stdout, stderr, TTY behavior, signals, and upstream exit status. B2 already
  proves raw streams/exit; B5 supplies the missing TTY/signal evidence.
- Current real-Termux test capability: read-only Lead probe job
  `job_hpc_c69baf49b3` observed installed `script`, `setsid`, and `stty` tools
  on the current aarch64 Android device. No package installation is authorized.
- TTY evidence: on Android, launch the existing private Rust exec probe beneath
  `script` solely as a test PTY provider; inside the probe call the production
  `exec_upstream` primitive and have the upstream shell prove `-t 0`, `-t 1`,
  and `-t 2`. The assertion must distinguish the upstream payload from
  `script`'s own wrapper output and must not invoke the installed Codex product.
  If a portable source-level test can avoid `script` with a smaller safe FFI
  surface, that is also acceptable, but no new dependency may be added.
- Signal evidence: spawn an isolated private probe that final-execs an upstream
  shell which prints a readiness marker, installs a `SIGTERM` trap, and waits.
  The parent records the child PID, waits for readiness, sends `SIGTERM` to that
  same PID using the smallest Unix FFI needed, and proves the upstream trap ran
  and exited with the chosen code. Apply a bounded timeout/kill cleanup so a
  failed test cannot hang the suite.
- Keep all 24 accepted B1-B4 tests green. Use repository-external
  `CARGO_TARGET_DIR` and `CARGO_NET_OFFLINE=true` for `cargo fmt --check`,
  `cargo test --locked --workspace`, and `cargo build --locked --workspace`.
- Explicitly deferred: runtime/generation path selection, additional environment
  planning, normal `main` wiring, sandbox policy, doctor, manifest/updater
  interfaces, network, installation, activation, and Manager behavior.
- Protected surfaces: every repository path except `crates/core/src/main.rs`,
  all live resolver/runtime/launcher/Manager state, profiles, sessions, auth,
  Git refs, legacy history, and unrelated worktrees.
- Completion gate: TTY proof covers all three standard descriptors; signal proof
  delivers `SIGTERM` externally after the upstream readiness marker and observes
  the upstream-selected exit/result; no hang, public test surface, dependency,
  or extra file; all prior and named validation passes.
- Integration disposition: the primary Lead reviews the actual test mechanics
  for false positives, reruns the full suite plus the new focused tests, and
  commits only if they prove the production exec boundary rather than a helper
  path.
- Lead acceptance: the reviewed diff changes only `#[cfg(test)]` probe/test
  code. The unique TTY markers are emitted only by the upstream shell after the
  private Rust probe calls production `exec_upstream`; the Android PTY provider
  is `script` only. The signal probe likewise final-execs the upstream shell,
  waits for its `READY:PID:<pid>` marker, proves shell `$$` equals the originally
  spawned child PID, then sends external `SIGTERM` to that exact PID and observes
  the upstream trap's exit code 73. Cleanup guards bound failed cases. Primary-
  Lead validation job `job_hqo_7f45af3f26` passed `cargo fmt --check`, all
  26/26 workspace tests, three additional serial repetitions of each TTY and
  SIGTERM test, and `cargo build --locked --workspace` with offline mode and a
  repository-external Cargo target. M1-B5 is accepted for commit.

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
