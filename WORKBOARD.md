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
- Primary implementer: owns execution, routine microplanning, evidence,
  validation, and final decisions
- Technical Lead: one reusable packet-only `gpt-5.6-sol` / `max` session per
  milestone for bounded work-bundle planning and material problem consultation;
  follow-up input is delta-only under the budgets in `GOAL.md`
- Additional planning agents and checkpoint reviewers: disabled
- Live installation or activation: prohibited in this milestone

## Current objective

Produce the smallest buildable Rust Core that proves local Termux execution and
compatibility contracts without networking, self-update, Manager implementation,
or mutation of the installed Codex product.

## Selected next action

Construct the bounded initial Milestone 1 packet, create the Milestone 1 Sol
Technical Lead, obtain one executable work-bundle proposal, validate it against
`SPEC.md` and `GOAL.md`, and record the accepted bundle here before the first
implementation mutation.

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
10. Update `GOAL.md` with exact Milestone 1 evidence. If every gate passes,
    rotate to a Milestone 2 Technical Lead, replace this workboard's current
    target with the accepted Milestone 2 bundle, and continue.

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
- Do not spawn a planner or advisor other than the configured milestone
  Technical Lead, and do not invoke a checkpoint reviewer.
- Do not treat Technical Lead output as authority, implementation, review, or
  acceptance evidence.
- Do not give the Technical Lead repository/tool access, resend unchanged
  context, or exceed the packet and output budgets in `GOAL.md`.
- Do not modify `legacy/monolith` or rewrite sealed tags.
- Do not expand the document hierarchy during ordinary implementation.

## Next milestone

Milestone 2 — delivery and recovery — remains defined in `SPEC.md` and is not
current work until the Milestone 1 ledger is complete. Completion of that ledger
triggers a milestone-boundary Technical Lead rotation and replacement of this
file's current target; it does not require a routine user stop. Exhaustion of an
accepted bundle before then triggers the next planning checkpoint, not task
completion.
