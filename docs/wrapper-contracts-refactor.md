# Codex Termux wrapper contracts refactor

This branch converts the Termux wrapper from one large shell implementation into a thin compatibility loader plus explicit shell domains. The public `codex` execution path remains the compatibility boundary: the loader sources domain files, then the managed launcher calls `codex_main "$@"` exactly as before.

## Goals

- Keep public `codex` behavior stable unless compatibility tests cover the change.
- Preserve upstream raw binaries, patched runtime trees, managed launchers, support files, and rollback state.
- Make wrapper ownership machine-readable through `codex-wrapper.manifest.json`.
- Make strict audit enforce manifest consistency, domain ownership, protected path contracts, private cross-domain call boundaries, and entrypoint compatibility.
- Make CI publish a complete handoff artifact so a later agent can resume from the exact repository snapshot.

## Shell domains

| Domain | File | Responsibility |
| --- | --- | --- |
| state | `lib/codex-termux/state.sh` | status UI, managed path safety, temp paths, locks, Python helper bridge |
| runtime | `lib/codex-termux/runtime.sh` | runtime fetch/build/activate/repair/update/rollback readiness |
| notify | `lib/codex-termux/notify.sh` | hook configuration and Termux notification surface |
| doctor | `lib/codex-termux/doctor.sh` | human/JSON diagnostics and public doctor dispatch |
| profile | `lib/codex-termux/profile.sh` | profile selection, recent-profile state, and runtime context execution |
| session | `lib/codex-termux/session.sh` | session sharing and `codex termux session` |
| dispatch | `lib/codex-termux/dispatch.sh` | public `codex termux` namespace and `codex_main` |

## Recursive merge-readiness artifact

The merge-readiness workflow publishes `merge_readiness_report.json`, `canon-audit.json`, per-test logs, `branch.patch`, and a full repository snapshot ZIP. A later session can download that artifact and continue the same measure/change/test/audit loop from the exact tree state.
