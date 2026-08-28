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

### Bundle M1-R1 — fresh B1..B9 re-audit and hardening

- Trigger: explicit user direction to distrust the prior implementation worker
  and re-review the Rust Core from the beginning. Historical M1-B1..M1-B9
  acceptance records are review inputs only, not current proof.
- Fresh baseline: commit `92422311301500fef0a6a5859607917f59ec6fc9`
  builds cleanly offline; all 50 current tests passed once serially and eight
  additional times with the default parallel test runner. Passing those tests
  does not close the issues found by direct source review.
- Exact outcome: independently validate the current B1..B9 code against
  `SPEC.md`, correct confirmed safety/correctness gaps without expanding product
  scope, and establish a new direct-Lead baseline before any B10 work.
- Sandbox hardening: treat recognized `sandbox_mode` configuration as a policy
  boundary rather than relying on the prior narrow textual forms. At minimum,
  trim surrounding configuration whitespace, recognize separate and attached
  short `-c` config arguments plus long `--config` forms, allow only a clearly
  normalized `danger-full-access` sandbox value, and fail closed on other
  non-empty `sandbox_mode` values before runtime I/O. Preserve exact `--`
  scanning semantics and raw argv for accepted requests.
- FD restoration hardening: explicit failure paths must attempt complete cleanup
  but must no longer silently discard restoration syscall failures. Best-effort
  Drop cleanup may remain as a last-resort fallback; returned setup/exec paths
  must surface a restoration failure if exact caller FD state cannot be restored.
- Test isolation hardening: tests that close, replace, or otherwise mutate
  process-global FD 33/34 must execute in dedicated subprocesses so the default
  parallel Rust test runner never races on those descriptors. Test artifacts
  remain under temporary roots only.
- Re-audit scope includes exact first-argument dispatch; raw argv/stdout/stderr/
  exit behavior; the five-variable child-only contamination fence; TTY/signal
  process fidelity; FD 33/34 mapping/non-mutation/restoration; sandbox policy
  ordering; base environment planning; and B9 final-exec composition.
- No B10 process-environment capture, runtime/generation selection, normal
  `main` wiring, doctor, manifest/updater implementation, Manager work, network,
  package installation, live product mutation, or activation belongs in M1-R1.
- Writable product path: `crates/core/src/main.rs` only. Lead-owned authority
  updates to `GOAL.md`/`WORKBOARD.md` are allowed separately. No dependency or
  manifest change is authorized by this bundle.
- Worker mode remains OFF. The primary Lead implements directly. tmcp
  `harness.run` may be used only for bounded tests/validation, never for code
  mutation or development.
- Validation: `CARGO_NET_OFFLINE=true`, repository-external `CARGO_TARGET_DIR`,
  `cargo fmt --check`, focused hardening tests, all locked workspace tests both
  serial and default-parallel, repeated parallel stress runs, and locked build.
- Completion gate: confirmed gaps are fixed; new tests demonstrate the hardened
  behavior; no process-global FD mutation remains in ordinary parallel tests;
  protected live state remains unchanged; the Lead records a fresh behavior-by-
  behavior proof disposition for B1..B9. B10 resumes only after this gate.

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
