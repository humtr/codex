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

### M1-R2 — exhaustive Milestone 1 simplification and real-entrypoint closure

#### outcome

Re-audit the complete surviving Milestone 1 implementation from M1-B1 through
M1-B24 plus M1-R1, classify every production definition and M1-specific test/
probe harness as KEEP, COLLAPSE, or DELETE, apply the resulting simplification,
and close the actual public `main` wiring gap. M2-B2 does not resume until the
resulting smaller M1 product path passes the load-bearing regression gate.

#### exhaustive scope

- Review every surviving production definition attributed to M1-B1..B24/R1,
  including dispatch, sandbox planning, process exec/environment fencing,
  FD33/34 handling, Termux environment composition, manifest/runtime/Manager
  qualification, updater interfaces, doctor/report path, public dispatch context,
  and public entrypoint composition.
- Review the complete M1 test/probe harness. Remove historical bundle-specific
  probes when their behavior is already covered by a retained end-to-end path or
  when the production mechanism they prove is removed.
- Trace actual product reachability from `main()`. `#[allow(dead_code)]` is not an
  acceptable substitute for product integration.
- Compare surviving adjacent bundle layers for equivalent validation or wrapper
  semantics. Prefer one direct composition path over planner→wrapper→wrapper
  chains that add no distinct product invariant.
- Preserve only user-visible or load-bearing invariants required by SPEC: exact
  command routing, raw upstream argv/stream/exit/TTY/signal fidelity, required
  Termux sandbox behavior, child environment contract, FD33/34 resolver/config
  contract, qualified complete generation/runtime identity, bounded read-only
  doctor, Manager boundary, updater/release qualification inputs needed by M2,
  resolver non-mutation, and protected-state boundaries.

#### mandatory known findings to resolve

- Actual production `main()` at `789b84e1` still stops after
  `classify_first_arg`; it must be wired to the real public execution path or the
  M1 completion claim must remain open.
- B1 classification and B20 public-route planning overlap and must be collapsed
  unless a distinct public contract requires both.
- B2/B7/B9/B14 launch/exec wrappers must be reduced to the minimum final-exec and
  doctor-child mechanisms actually needed by the product.
- B11/B12/B13/B21 qualification wrappers and repeated validation must be traced
  end-to-end; retain integrity checks but remove evidence/wrapper ceremony that
  real M2 release staging can represent directly.
- B15..B19 doctor planner/coordinator/render/error layers must be reduced where
  equivalent behavior can be expressed in one bounded command path.
- B21..B23 Manager/generation/context wrappers, including runtime/Manager
  generation-coherence machinery, must prefer construction from one generation
  over defensive mismatch checks when possible.
- M1-specific test scaffolding must be consolidated after product simplification;
  bundle provenance is kept in Git history, not as duplicate permanent test code.

#### stop lines

- Do not invent new hardening during this audit. New safety logic requires a
  demonstrated failure outside existing invariants.
- Do not weaken required public behavior merely to reduce line count.
- Do not touch the live installed Codex, resolver, Manager, auth/session/profile
  state, package state, legacy history, or publication refs.
- Do not proceed to M2-B2 until the exhaustive disposition and simplification are
  complete and accepted.

#### verification

- Maintain a complete disposition of all surviving M1 production definitions and
  M1-specific test/probe groups; no bundle may be skipped because it was already
  reviewed historically.
- Use narrow compile/focused feedback while editing, then one grouped final gate:
  formatting/diff checks, all retained tests serial, complete default-parallel
  repetitions required by repository acceptance, warning-free locked release
  build, and protected live resolver/launcher identity unchanged.
- Final acceptance must report what was kept, collapsed, and deleted, plus the
  production/test line-count change and confirmation that real `main` reaches the
  intended public path.

## Next action after M1-R2

Resume M2-B2 minimal local release/bootstrap path from the smaller accepted M1
foundation. Do not restore deleted proof-only layers when M2 can consume the
same foundational invariant directly.
