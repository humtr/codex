# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 is paused after accepted M2-B1 while the
  user-directed M1-R2 exhaustive simplification/closure audit runs
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- Click-inspired discipline: no Click plugin or Hook is installed; reuse fresh
  evidence, avoid re-reading unchanged evidence, and batch load-bearing validation
  once the refactor stabilizes

## Product-speed policy

- Ship the smallest correct state machine. Release velocity takes priority over
  speculative resilience after load-bearing integrity invariants are met.
- Remove a check, retry, wrapper, fallback, state field, or test harness when a
  simpler foundational invariant or direct product path subsumes it.
- Do not add locks, leases, fencing, fallback ladders, repeated retries, or new
  defensive branches without a concrete reproducible product failure not already
  handled by the existing core invariants.
- Tests are evidence, not architecture. A production layer is not retained merely
  because historical tests were written around it.

## Selected next action

### M2-B2 — minimal activated-generation loader and real public `main`

#### outcome

Consume the smaller M1-R2 foundation and close the real product entrypoint by
loading exactly one already-complete activated local generation, constructing
the existing runtime/Manager/doctor dispatch context from that generation, and
having production `main()` execute `plan_public_dispatch -> execute_public_dispatch`.
Do not expand B2 into download, network update, multi-writer coordination, or a
fallback ladder.

#### R2 evidence carried forward

M1-R2 exhaustive simplification is implemented at
`2b73f4ba23726ddab0792bbba721a2835dcb86d9`:

- production/test source was reduced from the accumulated proof-layer form to
  2,330 production lines and 1,624 test lines;
- historical M1 bundle-specific test names were removed and replaced by one
  contract-oriented suite plus the retained M2-B1 fault/recovery matrix;
- duplicate command classification, parent FD backup/restore state machines,
  environment wrapper layers, updater evidence-promotion wrappers, Manager
  generation-pointer comparison, doctor planner/coordinator wrappers, nested
  launch errors, the B24 proof-only entrypoint wrapper, and the speculative
  shared-resolver fallback model were deleted or collapsed;
- retained behavior covers exact routing, sandbox policy, raw argv including
  non-UTF8, upstream streams/exit/TTY/signal, environment fencing, FD33/34,
  manifest/runtime/Manager qualification, updater qualification inputs, bounded
  read-only doctor, and M2-B1 activation recovery;
- serial retained suite passed 33/33 with one explicit live smoke ignored by
  default; the explicit live resolver/launcher smoke passed 1/1; three complete
  default-parallel suite runs passed.

R2 also proved that the remaining `main()` gap is not another M1 wrapper: the
missing input is physical active-generation context acquisition, which SPEC
assigns to Milestone 2. Milestone 1 product closure therefore remains open only
until B2 connects that M2-owned input to the already-proven M1 execution path.

#### boundary

- Use the SPEC ownership split: immutable generation artifacts under the local
  Core generation root and activation/journal authority under the Core state
  root. Exact physical names may be chosen here once and kept minimal.
- Read/recover the current activation state through the existing M2-B1 state
  implementation; do not create a second pointer or recovery mechanism.
- Resolve exactly one current generation. A generation identifier that is used
  as a filesystem component must satisfy one simple path-component invariant;
  do not add canonicalization/fallback search chains.
- Load one small versioned local generation descriptor sufficient to bind the
  runtime path, compatibility directory, optional Manager artifact, manifest
  compatibility fields, and doctor capability required by the existing dispatch
  context. Launch must not re-download or package-manage.
- A missing/malformed/incomplete current generation fails clearly. Do not search
  `previous`, scan other generations, or silently fall back during ordinary
  launch. Recovery/rollback remains an explicit activation-state operation.
- `update` remains a Core-owned handoff until the later M2 updater bundle; B2
  must not invent network behavior merely to make the route non-dead.
- Tests use explicit temporary HOME/state/artifact roots. The installed Codex,
  live resolver, live Manager state, auth/session/profile state, and package
  state remain read-only.

#### must hold

- `codex --version`, `-V`, empty argv, and every non-Core command still reach the
  selected upstream runtime with exact argv/stream/exit/TTY/signal behavior.
- exact first-token `doctor` and `termux` still use the existing bounded Core
  paths; `update` still remains Core-owned without live network mutation.
- one activated generation supplies runtime and optional Manager authority; no
  mixed-generation context or fallback ladder exists.
- sandbox planning still occurs before runtime descriptor I/O that is avoidable
  for an invalid request, and resolver/config FD sources remain read-only.
- production `main()` actually reaches the public execution path; it must no
  longer stop after planning argv.
- release build warnings caused by unreachable M1 product paths are removed by
  real reachability, not hidden with new `allow(dead_code)` annotations.

#### verification

- focused temporary-root loader/main tests for missing, malformed, valid
  upstream, doctor, Manager-unavailable/available, and update-handoff cases;
- retained M1 contract suite and M2-B1 fault/recovery suite serial;
- explicit real-Termux resolver/installed-launcher read-only smoke;
- complete default-parallel repetitions after stabilization;
- `cargo fmt --check`, `git diff --check`, and warning-free locked release build;
- live resolver and installed launcher identity unchanged before/after the final
  grouped gate.

#### stop lines

- no live installation or activation in B2;
- no network, package-manager, signed-release acquisition, or automatic update;
- no new lock/lease/fencing/multi-writer protocol;
- no implicit previous-generation fallback on launch;
- no revival of proof-only layers deleted by M1-R2.

## Next action after M2-B2

Continue the shortest remaining release path: local immutable release staging and
qualification feeding the same generation descriptor/activation contract. Add
network acquisition or signing only in the bundle that actually needs it.
