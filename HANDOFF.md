# Handoff — current integration work

Updated 2026-11-10 KST. **Only unfinished work and acceptance boundaries
belong here.** Product contracts: [SPEC](SPEC.md), verified status:
[CURRENT](CURRENT.md), operational preflight:
[Termux constraints](docs/TERMUX_CONSTRAINTS.md), historical instruction
archive: [legacy handoff](docs/legacy/2026-11-10/PRO_HANDOFF.md).

## Selected branch and permitted effects

- Work exclusively from `integration/codex-native-20261110` at the actual
  verified HEAD. Last code-qualified revision: `5384d938`. Inspect the
  current branch, other worktree ownership and actual diff first.
- Preserve operating Full159, Full153 rollback, signed generations, installed
  Core, active Provider/service/host, account homes and unrelated Native work.
  Never touch Android sdcard. Source edits, tests, commits and non-force push
  do not authorize operational replacement.

## Finish in this order

1. Align the real TUI candidate Core, configured Provider and Native writer
   ownership **before** submitting any model request. Verify `$PREFIX/bin/codex`
   dispatcher, HOME/CODEX_HOME, workspace, socket root and active service
   bindings. A bare helper or mixed isolated/operating owner is not a valid
   live-TUI acceptance.
2. Through `/provider`, select real GPT-6 and obtain **two completed model
   responses on one Native thread**. Re-read its original turn IDs, settings
   and full history; do not count prior `--profile chatgpt` diagnostic turns.
3. Inject SSE disconnect and HTTP 413 at their real boundaries. Re-enter the
   same Native work/thread, reconcile the original result and prove no second
   dispatch, no lost tool effect and no permanent denial of follow-up input.
4. Prove another independent Native work can execute while a bounded opaque
   effect is held, and that real overlapping unsafe writes remain protected.
5. Re-run necessary **exact-current-SHA** source, Provider, TUI and runtime
   checks, then separately decide whether signing and device activation are
   authorized. Do not promote on CI-only evidence.

Record each original receipt, run URL, tested SHA, expected and observed
outcomes, failed case and remaining gap. Preserve earlier failures in a dated
legacy evidence record; keep [CURRENT.md](CURRENT.md) bounded to the most
recently verified state. No unsupported success claim or retry-as-workaround.

## Known latest blocker

The candidate TUI selected GPT-6 but its first real input returned
`native_thread_owner_unavailable` because the experiment used isolated
Core/home/workspace against an operating Provider bound elsewhere. The
shortened Unix socket root fixed startup, **not** inference ownership.
See [CURRENT.md](CURRENT.md) for the exact CI scope; the historic 2026-11-08
Pro plans and work IDs are preserved in [legacy](docs/legacy/README.md), not
current instructions.
