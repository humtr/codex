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

### M2-B3 — explicit local generation staging

#### outcome

Take one explicit caller-supplied local generation source, copy only the fixed
B2 generation layout into a private candidate directory under the immutable
generation root, validate that candidate with the same descriptor/loader rules,
and atomically publish the complete candidate as an **inactive** generation.
This is the shortest offline/bootstrap path that can be built before signed
release admission exists.

#### accepted input

M2-B2 is accepted at `bee38e9eb481973c00205fb8a7191cdb22392f7c` with:

- real production `main -> plan_public_dispatch -> current-generation loader ->
  qualification -> dispatch`;
- focused B2 6/6, full serial 38/0/1-ignored, explicit live smoke 1/1,
  default-parallel 3/3, warning-free locked release build;
- live resolver and installed launcher identity unchanged;
- current-only ordinary loading with no previous-generation fallback, network,
  package-manager, generation scan, or duplicate state-root generation tree.

#### boundary

- B3 accepts one explicit local **directory source** only. Archive extraction is
  a later bundle because generic archive handling adds a separate parser/safety
  surface; do not implement it merely to call this step an installer.
- The source must contain exactly the B2 load-bearing generation artifacts:
  `generation.meta`, regular `runtime`, directory `compat/`, optional regular
  `manager`, and any declared regular `helpers/<index>` files. Copy only these
  fixed paths; do not mirror arbitrary source-tree entries.
- Add one descriptor `generation_id` field and require it to satisfy the same
  single-component invariant used by activation state. The published path is
  `generations/<generation_id>`.
- Construct in a private candidate path under the generation root. Candidate
  copy rejects symlinks/special files and path escape. Compatibility-directory
  recursion copies regular files/directories only and never follows symlinks.
- Validate the copied candidate by loading its own descriptor/layout before
  publication. Publication is one atomic rename from candidate to the final
  inactive generation path; final-path collision fails rather than overwriting.
- Do **not** change `activation-state` in B3. A successfully staged generation is
  inactive until the later digest/signature admission gate explicitly activates
  it through the existing M2-B1 transaction.
- Do not add lock/lease/fencing, retry ladders, candidate registries, cleanup
  databases, alternate staging roots, network access, package-manager calls, or
  automatic previous-generation fallback.
- On an ordinary handled error, remove the private candidate directory created
  by that attempt when possible; cleanup failure must not mutate active state.

#### must hold

- currently active generation and activation-state bytes remain unchanged by B3;
- a published generation is complete or absent;
- source symlinks/special files never become generation content;
- candidate validation uses the same B2 loader format rather than a second
  validator stack;
- existing M1/B2 launch, sandbox, FD33/34, doctor, Manager, and current-only
  behavior remains unchanged;
- live resolver, installed launcher, Manager state, auth/session/profile state,
  and package state remain read-only.

#### verification

- focused temp-root staging tests: valid runtime-only source, optional Manager,
  compat nested regular files, malformed descriptor, unsafe source entry,
  final-path collision, copy failure/cleanup, and active-state non-mutation;
- load the published inactive generation directly and prove it matches B2
  descriptor/layout semantics while ordinary `current` still points to the old
  generation;
- retained full serial suite and M2-B1 fault/recovery suite;
- explicit real-Termux resolver/installed-launcher smoke;
- complete default-parallel repetitions after stabilization;
- `cargo fmt --check`, `git diff --check`, warning-free locked release build,
  and protected live identities unchanged.

#### stop lines

- no activation of the newly staged local generation in B3;
- no archive parser/extractor;
- no signature or cryptographic digest implementation yet;
- no network, package manager, automatic update, multi-writer protocol, or
  fallback ladder;
- no revival of proof-only M1 layers.

## Next action after M2-B3

Add the minimum signed/digest release-admission boundary needed to trust a
staged local generation, then activate that admitted generation using the
existing M2-B1 transaction. Keep offline/local flow working before adding remote
acquisition.
