# ChatGPT provider multi-account draft — inactive development note

This directory preserves the in-progress **`accounts.rs` prototype** for isolated multi-account profile/home registrations under the ChatGPT provider, kept outside `providers/chatgpt/src/` while under development so Cargo neither compiles nor runs it.

- `accounts.rs`: Unwired `Accounts` / `Account` journal state, per-profile home directory mapping (`desktop/home` and `desktop/accounts/<id>`), `Identity` binding confirmation (`expected_identity`, `confirm`), `flock`-guarded `remove(..., purge)`, and embedded unit tests (matching the draft in `wip/chatgpt-accounts-source-20261110` at commit `2dd07d42`).

The current single AppHost and account-bound lane contract referenced in [`CURRENT.md`](../../../CURRENT.md) and [`SPEC.md`](../../../SPEC.md) remains in force. Wiring multi-profile support into production later requires single-journal-writer ownership, immutable profile/home and app-identity binding for admitted work, host-profile leases, and crash-safe credential purge without migrating existing homes implicitly.
