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

The merge-readiness workflow publishes the `codex-agent-handoff-${sha}` artifact.
It contains `merge_readiness_report.json`, `canon-audit.json`, per-test logs,
`branch.patch`, a repository snapshot ZIP, and a `handoff/` directory for the
next agent.

Run the same loop locally with:

```sh
CODEX_MERGE_READINESS_OUT_DIR="$PWD/out/merge-readiness" \
  bash tools/merge-readiness.sh
```

The script must generate diagnostic files before returning a non-zero status.
Later agents can resume from:

- `handoff/connector-handoff.json`
- `handoff/NEXT_AGENT_PROMPT.md`
- `handoff/resume-from-artifact.sh`
- `handoff/artifact-manifest.json`
