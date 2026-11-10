# Handoff — current integration work

Updated 2026-11-10 KST. **Only unfinished work and acceptance boundaries
belong here.** Product contracts: [SPEC](SPEC.md), verified status:
[CURRENT](CURRENT.md), operational preflight:
[Termux constraints](docs/TERMUX_CONSTRAINTS.md). Completed prior
work is available in Git history, not as new instructions.

## Selected branch and permitted effects

- Work exclusively from `integration/codex-native-20261110` at the actual
  verified HEAD. Last product-runtime change baseline: `5384d938`.
  Last exact-source CI baseline before review-doc updates: `0aebdd7f`,
  with [Source Acceptance #42787269906](https://github.com/humtr/codex/actions/runs/42787269906)
  successful on that SHA; **not** device or live-model acceptance.
  Never infer CI success on a later review-doc commit from this receipt.
  Inspect HEAD, remote, other worktree ownership and diff first.
- Preserve operating Full159, Full153 rollback, signed generations, installed
  Core, active Provider/service/host, account homes and unrelated Native work.
  Never touch Android sdcard. Source edits, tests, commits and non-force push
  do not authorize operational replacement.

## Review-driven implementation packets

Use the [temporary review work queue](docs/development/REVIEW_ACTIONS_20261110.md)
to correlate the new 109 findings with the prior 74-item internal review.
P0/P1 are **priority hypotheses**, not observed operating failures.
Re-read actual code/owner and add reproducible negative/recovery tests before
each cohesive commit. Keep separate packets on isolated worktrees; serialize
overlapping Native writers, merges and real device changes.

1. **R01 CI ownership (U7-4, U6b-4):** exact branch/ref/SHA admission,
   correct gate matrix and no unsolicited acceptance dispatch on `main`.
   **Do not manually dispatch source-acceptance or manager-tui-native on
   the integration branch before this is fixed:** their existing
   `if: always()` notifier can launch `accepted-release-inputs` on main.
2. **R02 MCP control authentication (U3-1):** unauthenticated loopback
   control calls rejected; authorized Native and service callers still work.
3. **R03 Core safety (U1-2/4/6):** crash-safe guard, secret redaction,
   upstream option and unsupported Termux sandbox parsing. No blind purge.
4. **R04/R05 Native continuity/concurrency (U4-3/6, U3-4 et al.):**
   reconcile original dispatch/result/effect claims, keep independent work
   available without a second writer or unsafe claim release.
5. **R06 TUI input/reentry (U5-1/4 et al.):** one-shot markers, validated
   bounded/cancelable waiting and preserved draft/native input identity.
6. **R07 destructive migration (U6b-2):** isolated copies, durable intent
   and confirmed app-server processing before irreversible effects. **Do
   not** run migration against live user homes as a documentation task.
7. **R08-R10** then isolate release retries, Base online installer,
   Provider fixture cleanup/credential gates, remaining P1 cases and
   qualifying P2/conditional findings. **Reject** U6a-1's CI-bypass
   interpretation; do not restore old README pins.

**Nonconflicting execution and effort:** The review action queue now contains
a self-contained U1–U7 finding dictionary, per-packet implementation/test
LOC ranges (R01–R09 2,410–5,480; with R10 2,630–5,990), source-owner and
physical Native writer/claim/socket isolation matrix, stages S0–S6, and
release/branch gates. Until R05 establishes safe independent capacity, use
one active Termux Native-effect lane or positively demonstrate completely
isolated Core+Provider+Native environments; worktrees alone are not isolation.
Keep main as publication authority. After R01's notifier/branch fix,
propose a separately authorized fast-forward to the existing sole developer
branch `rewrite/rust-core` (its remote HEAD is an ancestor of integration);
**do not touch** its three untracked files or advance that checkout during
this documentation work. See the queue for the read-only baseline and
required non-force/owner checks.

Product tests and source commits do not permit signed rollout, active
services/account changes, Android sdcard access or unapproved fixture cleanup.

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
Prior Pro plans and work IDs are historical evidence in Git, not
current instructions.
