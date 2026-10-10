# Working in humtr/codex

Build and maintain the Termux compatibility layer for upstream Codex. Prefer a
small product and a direct path from the user's request to verified behavior.

## Authority and context

- SPEC.md owns product behavior, runtime/security boundaries and update/rollback.
- GOAL.md contains the current objective, accepted operating baseline and open work.
- Keep current execution with the existing work record; no separate workboard,
  mandatory ledger or template. README.md is the user entrypoint.
- Start by checking branch, HEAD and dirty state. Read these short current-state
  record when relevant, then only the SPEC sections and code relevant to the task. Do not
  reread historical commits or unrelated material without a concrete need.
- Use skills when their capability helps this task; no skill is a compulsory
  planning, approval, delegation or checkpoint layer.
- Update a product contract when changing its behavior; ordinary implementation
  detail needs no separate design record. Keep completed investigation in Git.

## Execution

- Honor the user's outcome and existing authorization. Resolve routine choices
  directly; ask only for information or authorization genuinely missing.
- Own diagnosis, implementation, actual diff review, appropriate validation and
  completion. Continue independent useful work while a required answer is pending.
- Follow a demonstrated defect through the affected product path. Expand scope
  when necessary for correctness; avoid speculative features and defensive layers.
- Choose the available model/effort through the user's session. Do not pin model
  families, max reasoning, named leadership roles or recurring review ceremonies.
- Use collaborators only when authorized by the user/platform. Current user
  preference is direct execution. Never run concurrent edits in a shared worktree.
- Create temporary checkouts/probes in TMPDIR; retain reusable compiler state in
  XDG_CACHE_HOME. Do not add temporary projects or installed binaries under prj.

## Verification

- Choose tests from changed behavior and its real failure modes. Low-impact
  reversible changes need appropriate checks, not artificial regression tests.
- Use scripts/check.sh for docs, Python, Rust, or full verification as appropriate.
  Full acceptance is required for runtime/state/security/installer changes and
  release candidates; small scoped changes may close with relevant passing gates.
- Reuse successful evidence only when its actual inputs, environment and scope
  still apply. Record source/artifact identity; do not call reused evidence a new run.
- A failing or zero-test required gate is unfinished. Diagnose it before adding
  dependent behavior. Do not repeat green checks without a change or open concern.
- Public deployment still requires build/smoke, signing, exact public readback,
  disposable update and non-forced stable promotion. Test fixtures alone do not
  prove a real device action; state physical observations separately.

## Source and publication

- rewrite/rust-core owns implementation. main owns publication controls and uses
  an immutable accepted source; it is not a second implementation tree.
- Keep independent histories independent. Never import main or legacy code/history
  into the rewrite. Never force-push main or delete a published branch without the
  user's explicit authorization for that operation.
- Public release assets are immutable. Release changes do not imply live cutover.

## Protected state

Preserve user changes, credentials, profiles, conversations and running jobs.
Use disposable roots for tests; no live launcher/runtime/state edits except a
user-authorized bounded device action. Never print/store credential values or
unredacted session contents, and never change a system resolver file. Core normal
launch must work when optional Manager or update discovery is unavailable.
