<!-- PROFILE-LIFECYCLE delivery binding: source59308430d39dc3c57159e260e5f7a873c1b16b8d,
parent471590750468f6d99b520c3a8d47b03a474aacbb, signed40 from39, Core+Manager,
local-hosted-0-160-0-59308430d39d-profile-lifecycle. Runtime/helpers preserved;
ordinary installed update retains39. Authoritative product documents: rewrite/rust-core.
User requested deployment; no profile/auth/history/job mutation or force push. -->

# Codex for Termux

This repository is the clean rewrite of the Termux compatibility layer for the
upstream Codex CLI.

The rewrite has one public command, `codex`, and two internal layers:

- a minimal native Rust Core that makes the upstream runtime work correctly on
  Termux and owns installation, update, diagnosis, activation, and rollback;
- a separate Manager layer reached through `codex termux` for profiles,
  sessions, notifications, and other Termux conveniences.

## Current status

Only the product and work-system foundation exists on this lineage. There is no
new Rust implementation or installable release yet. Do not replace a working
Codex installation from this branch.

Implementation is intentionally split into two milestones:

1. local Rust Core execution and compatibility contracts;
2. secure fresh installation, self-update, generation activation, and rollback.

Independent product review follows the completed Milestone 2 candidate rather
than interrupting ordinary implementation checkpoints.

## Documents

- `SPEC.md` — normative product and architecture contract
- `GOAL.md` — success threshold and acceptance ledger
- `WORKBOARD.md` — current milestone and next work only
- `AGENTS.md` — repository-local execution and safety rules

## Branches

- `main` — clean rewrite lineage
- `rewrite/rust-core` — active Rust Core implementation
- `legacy/monolith` — sealed predecessor at `bf30a7d`

The legacy implementation is not a source base for the rewrite.

Published browser-bridge releases before the Manager TUI may reject a direct
update with “R10 browser helper bridge contract is invalid.” Run this one-time
migration from the immutable signed Release, then ordinary updates. The existing
Core verifies its trusted signature and complete inventory; profiles, conversations
and running clients are preserved. The temporary download is always cleaned.

```sh
(
  set -eu
  migration="$(mktemp -d "${TMPDIR:?}/codex-migration.XXXXXX")"
  trap 'rm -rf -- "$migration"' EXIT
  gh release download local-hosted-0-161-0-ab644771ac89-manager-tui-bridge \
    --repo humtr/codex --dir "$migration" \
    --pattern core --pattern generation.meta --pattern manager --pattern runtime \
    --pattern codex-code-mode-host --pattern helper-0 --pattern helper-1 \
    --pattern release.manifest --pattern release.sig
  chmod 0644 "$migration/generation.meta" "$migration/release.manifest" "$migration/release.sig"
  mkdir "$migration/helpers"
  mv "$migration/helper-0" "$migration/helpers/0"
  mv "$migration/helper-1" "$migration/helpers/1"
  chmod 0755 "$migration/core" "$migration/manager" "$migration/runtime" \
    "$migration/codex-code-mode-host" "$migration/helpers/0" "$migration/helpers/1"
  codex update --local "$migration"
  codex update
)
```
