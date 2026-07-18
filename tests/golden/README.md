# Golden behavior fixtures

These fixtures freeze observable Codex Termux wrapper behavior before implementation ownership and layout changes.

The initial fixture set was reviewed and selectively rebuilt from archived PR #5 at `archive/pr-5-golden-runtime-contracts-20260717` (`4157d3baff8548e2586d95efd367f02589339802`). The archive is evidence, not a merge dependency.

Fixture classes:

- `public`: CLI routing, profile persistence, and public error surfaces.
- `internal-contract`: state/registry serialization and runtime process contracts that downstream refactors must preserve.
- `white-box-transition`: transaction rollback behavior. These fixtures deliberately call private transition helpers and must be updated when internal transaction boundaries are intentionally redesigned.

The capture harness uses an explicit environment allowlist and does not inherit credentials, tokens, SSH agent variables, or unrelated host configuration into fixture subprocesses. Expected outputs normalize repository, sandbox, home, prefix, and temporary paths. Golden updates require an intentional `--update` run and code review.
