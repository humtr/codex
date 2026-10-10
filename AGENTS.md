# Development in humtr/codex

Maintain a small Termux compatibility layer for upstream Codex. These rules are
optimized for GPT-6.1 Sol: concise task context, direct implementation, judgment
about scope and effort, and verification that reaches the real product path.
The repository owns this development plane; it needs no external workflow skill.

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
- Adjust investigation and reasoning to uncertainty and consequences. Simple
  edits should stay simple; difficult architecture or diagnosis merits depth.
  No mandatory maximum effort, fixed leadership role or checkpoint cadence.
- Prefer one direct production path. Remove obsolete mechanisms and duplication
  when they cause the current defect; do not invent speculative subsystems.
- Own actual diff review and validation. Use collaborators only when the user
  authorizes them; current preference is direct execution. No concurrent edits
  in a shared worktree and no automatic review or planner layer.
- Use TMPDIR for temporary work/checkouts; use XDG cache for compiler output.
  Never put temporary projects or installed binaries under prj.

## Verification and publication

- Select scripts/check.sh docs, python, rust or full from the changed behavior.
  Use meaningful affected-path proof; low-impact reversible edits need no made-up
  tests. Runtime/state/security/installer changes and release candidates need
  full acceptance, including applicable Android execution proof.
- Reuse green evidence only when its actual inputs/environment/scope still apply.
  Failed or zero-test required gates remain unfinished. Do not repeat passing
  checks without a change or unresolved concern.
- rewrite/rust-core owns implementation and reusable release logic; main is a
  caller pinned to an accepted exact source. Keep independent histories separate.
  Never force-push main or delete published branches without explicit approval.
- Public assets are immutable. Deployment requires build/smoke, signing, complete
  public readback, disposable update proof and non-forced stable promotion.
  Device activation is separate; report physical observations separately.

## Protected state

Preserve user changes, credentials, profiles, conversations and running jobs.
Use disposable roots for tests. Do not edit live launcher/runtime/state except
for a user-authorized bounded device action; never expose credential values or
private session contents, and never modify a system resolver file. Ordinary Core
launch must work when optional Manager or update discovery is unavailable.
