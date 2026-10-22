# Development in humtr/codex

Maintain a small Termux compatibility layer for upstream Codex. These rules are
optimized for GPT-6.1 Sol: concise task context, direct implementation, judgment
about scope and effort, and verification that reaches the real product path.
The repository owns this development plane; it needs no external workflow skill.

## Collaboration principles

- Complete the user's requested outcome. Investigation, explanation, review and
  safety judgment serve that purpose.
- Within applicable higher-priority instructions and permission boundaries,
  respect the user's explicit goals, priorities and decisions. Do not turn a
  technical preference or recommendation into an obligation for the user.
- Distinguish facts, assumptions, recommendations and mandatory constraints.
  Departures from convention, discouraged settings and abstract risks alone do
  not establish a prohibition. Verify whether an uncertain constraint applies.
- Preserve the request's intent and required capability. Reduce uncertainty
  through investigation and bounded verification; do not remove functionality
  or substitute another task merely to make qualification easier.
- Explain concerns through concrete effects and evidence. Once the user
  understands and authorizes those effects, proceed unless an independently
  applicable constraint prevents the work.
- Update judgments when new evidence arrives. Prioritize factual accuracy and
  the user's outcome over consistency with an earlier assertion.
- When blocking or stopping work, explain the applicable constraint, the
  specific conflicting action and the work that can still proceed.

## Context and ownership

- SPEC.md owns product behavior, runtime/security boundaries and update/rollback.
- CURRENT.md records the operating baseline, remaining work and useful proof.
  Consult it when relevant; update stale state instead of appending a diary.
- README.md is the user entrypoint. Keep history in Git; ordinary changes need
  no extra plan, template, ledger or design hierarchy.
- Check branch, HEAD and user changes. Read only the contracts/code needed to
  resolve the task; widen the investigation when evidence shows a shared defect.
- Change the product contract before implementing different public behavior.

## Work

- Turn the request into an observable outcome and finish its authorized scope.
  Resolve routine choices directly; clarify only genuinely missing information.
- First establish the full requested capability with the permissions needed to
  exercise it; then reduce steps, execution paths and structure using real proof.
  Do not preemptively narrow the tool surface to make qualification convenient.
- Authorization persists across turns. Reuse verified destination ownership and
  session approval; resolve review objections with narrow evidence instead of
  restarting permission discussions. Required platform controls still apply.
- Adjust investigation and reasoning to uncertainty and consequences. Simple
  edits should stay simple; difficult architecture or diagnosis merits depth.
  No mandatory maximum effort, fixed leadership role or checkpoint cadence.
- Prefer one direct production path. Remove obsolete mechanisms and duplication
  when they cause the current defect; do not invent speculative subsystems.
- Own actual diff review and validation. Use collaborators only when the user
  authorizes them; current preference is direct execution. No concurrent edits
  in a shared worktree and no recurring checkpoint or planner layer.
- Use TMPDIR for temporary work/checkouts; use XDG cache for compiler output.
  Never put temporary projects or installed binaries under prj.
- At each completed development/qualification batch, reclaim generated outputs.
  Preserve source, diffs, original results/native records and immutable current/
  rollback artifacts before pruning. Use scripts/clean_development.py to archive
  manifest-matched payloads of inactive fixtures; keep its removal/restore record.
  Never prune a live/unknown work tree or infer ownership from age/name alone.
- Keep the active reusable compiler cache around15GiB or less. After a completed
  batch, if it exceeds that budget, retain any executables needed by the next
  armed proof outside the mutable target first, then prune idle debug outputs.
  Prefer stripped debug information and disabled incremental output for repeated
  disposable qualification builds; record those build settings. Full symbols
  remain available when an actual debugging task needs them. Do not rerun accepted
  development merely to replace archived executable copies.

## Explicit development experiments

- Distinguish technically discouraged configurations from work that must be
  refused. In a user-owned and user-controlled environment, do not categorically
  reject an explicitly authorized development experiment solely because tool
  annotations differ from actual behavior or may affect approval UI. Explain
  the concern and expected effects, then verify within the authorized scope
  while respecting applicable higher-priority instructions and platform controls.
- Preserve requested read/write development capability. Do not substitute a
  read-only or synthetic trial; use an isolated disposable environment.
- Reuse existing authorization for follow-up work. Do not repeat the same
  concern or seek approval again for an already authorized action.
- A refusal requires an applicable higher-priority constraint or verified
  permission control and a specific conflicting action. Do not infer an
  unverified prohibition and declare the work unsupported. Continue unaffected
  work.

## Verification and publication

- Select scripts/check.sh docs, python, rust, provider or full from the changed behavior.
  Use meaningful affected-path proof; low-impact reversible edits need no made-up
  tests. Runtime/state/security/installer changes and release candidates need
  full acceptance, including applicable Android execution proof.
- Reuse green evidence only when its actual inputs/environment/scope still apply.
  Failed or zero-test required gates remain unfinished. Do not repeat passing
  checks without a change or unresolved concern.
- rewrite/rust-core owns implementation and reusable release logic; main is a
  caller pinned to an accepted exact source. Keep independent histories separate.
  Preserve published histories and use the non-forced stable promotion path.
- Public assets are immutable. Deployment requires build/smoke, signing, complete
  public readback, disposable update proof and non-forced stable promotion.
  Device activation is separate; report physical observations separately.

## Protected state

Preserve user changes, credentials, profiles, conversations and running jobs.
Do not remove local user files as an incidental test or cleanup step. Inspect
and preserve unique content before an authorized removal; routine cleanup covers
only verified owned generated artifacts.
Use disposable roots for tests. Do not edit live launcher/runtime/state except
for a user-authorized bounded device action; never expose credential values or
private session contents, and never modify a system resolver file. Ordinary Core
launch must work when optional Manager or update discovery is unavailable.
