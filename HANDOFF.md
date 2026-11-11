# Handoff — unfinished integration acceptance

Updated: 2026-11-12 KST. One current unfinished-work record.
Read [CURRENT](CURRENT.md), [AGENTS](AGENTS.md) and the
[finite integration plan](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).
The user's continuous authorization covers development, safe branch integration
and unsigned CI; there is no intermediate plan-approval gate.

## Remaining work

1. Run the exact adapted-upstream hook and TUI regressions in hosted CI; local
   patch assembly is accepted, but local TUI Rust compilation is still blocked
   by the unavailable pinned rusty_v8 dependency.
2. Push the integrated source by fast-forward to the actual current
   rewrite/rust-core after rechecking main intent and concurrent upstream
   discovery. Run and observe Source, Core/Runtime, Manager, Provider,
   full TUI Native, unsigned Full composition and stock Android acceptance
   against the exact source. Fix real failures and rerun.
3. With the candidate frontend/runtime, finish the same-thread /switch
   destination checkpoint and original-result continuity fixture. Historical
   quota replay was repaired; candidate Core/Manager with isolated copies of
   the installed S165 frontend/runtime prove Owner lookup and reentry only,
   not the destination inference. Operating S165 Core was not replaced.
4. Use owned isolated candidate artifacts for actual Native compiler
   concurrency/settlement and terminal continuity where possible. Distinguish
   real Native, live app, synthetic account and Android emulator/physical
   Activity proof. Reuse the independently accepted existing-app C development
   loop only within its operating-binary boundary.
5. Record actual commits, file/LoC ownership, final exact-SHA CI receipts,
   protected-asset readback and branch dispositions. Remove completed items
   from this handoff; do not retain this list as future instructions once
   acceptance is recorded.

## Protected operating boundaries

Existing S164/S165 and previous signed generations, stable indexes, formal
signing/publication, operating installation replacement, service restart,
account/token changes, real SQLite/JSONL repairs and active Writer transfer
remain separate operations. None is needed merely to complete source/unsigned
qualification. Do not perform them as a development side effect.

The primary checkout's local e4f9ab5d lineage and other workers' untracked files
remain untouched. Do not delete retained worktrees or branches from an ancestry
classification alone. Unknown receipts require original-call reconciliation;
uncertain Owner/effects remain preserved and fail closed.
