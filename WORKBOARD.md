# Rust Core Workboard

This file owns only the current implementation, repository-maintenance, release,
or live-consumer bundle. Normative product behavior belongs in `SPEC.md`.
Accepted and historical evidence belongs in `GOAL.md`. Release mechanics belong
in `RELEASE_AUTOMATION_PLAN.md`.

## Current routing

- Repository: `humtr/codex`.
- Source authority branch: `rewrite/rust-core`; always re-read the remote head
  before work.
- Selected maintenance bundle: **none**.
- Last accepted product source:
  `75aed30c7ad6c427d30115b0667957130da198f5`
  (**STARTUP-ADVISORY-SINGLE-KEY-TRANSIENT-V1**).
- Current public stable and live Termux consumer are both signed sequence
  **27**, generation
  `local-hosted-0-160-0-75aed30c7ad6-startup-advisory`, upstream
  `codex-cli 0.160.0`.
- Public stable promotion commit:
  `2dc79bd11842c9e5970b8c73c470886b780f2ff8`.
- No source/product/public/live mutation is authorized until a new bounded
  Workboard bundle is selected.

## Preserved architecture

- SCS-1 through SCS-6 are accepted and closed. `$HOME/.codex` is the canonical
  user-global conversation state; Manager profiles are execution/auth/config
  identities, not conversation owners.
- Steady-state Manager remains a thin wrapper. It must not reintroduce a second
  thread store, routine SQLite/transcript parsing, conversation ownership, or a
  parallel session index.
- Core owns install/update/rollback/doctor and signed generation activation.
- Historical accepted bundles remain evidence in `GOAL.md`; they are not active
  routing merely because their original text used “current” or “active.”

## Resume rules

1. Read `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`; read
   `RELEASE_AUTOMATION_PLAN.md` before release or live-runtime work.
2. Rebind remote `rewrite/rust-core`, public `main`, signed stable index,
   official upstream, and live activation state before mutation.
3. Never replay a completed effect. Continue from the freshest verified frontier.
4. A new feature, cleanup, public release, or live-state mutation outside the
   selected bundle requires a new bounded Workboard selection.
5. If no bundle is selected, source/product/public/live mutation is not
   authorized.
