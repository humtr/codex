# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Branch ancestry: independent empty-root lineage; do not merge or rebase
  `main` or `legacy/monolith`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone: Milestone 2 — delivery and recovery
- Primary Technical Lead/Integrator: the main `gpt-5.6-sol` / `max` goal
  session; owns evidence retrieval, contract compilation, direct implementation
  while worker mode is OFF, actual diff review, integration validation, commits,
  and acceptance decisions
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it. Do not invoke implementation workers or coding subagents while
  OFF
- Planning agents, problem advisors, and checkpoint reviewers: disabled until
  the complete Milestone 2 candidate reaches the independent-review gate
- Live product cutover: not authorized by this bundle; all mutable M2-B1 evidence
  stays under test-owned temporary roots
- Click-inspired discipline: no Click plugin or Hook is installed. Reuse
  successful current evidence, do not reopen or replace this contract without
  new material evidence, use only narrow implementation feedback, and run the
  repository-required acceptance suite as one final grouped batch once stable.
  Repository authority always overrides any generic verification-budget concept.

## Current objective

Deliver a recoverable, prebuilt Core release system: immutable signed releases,
safe acquisition/adaptation, atomic activation and rollback, offline recovery,
concurrency/failure qualification, fresh-install/legacy-upgrade evidence, and a
complete candidate suitable for independent review.

## Selected next action

### Bundle M2-B1 — crash-safe generation state and activation recovery

#### outcome

Fix the first Milestone 2 physical Core state representation and implement the
smallest durable activation transaction in test-owned roots. A complete candidate
generation may be promoted into one authoritative pointer state only through a
journaled atomic replacement; recovery after any represented interruption must
resolve to exactly the complete old state or complete new state and never a mixed
current/verified/previous generation set.

#### boundary

- in_scope: `crates/core/src/main.rs` only; explicit root-derived Core state paths,
  strict std-only pointer-state/journal encoding and parsing, durable file-write /
  fsync / atomic-rename primitives, activation and recovery over test-owned
  filesystem roots, and deterministic injected-failure tests around each durable
  boundary.
- out_of_scope: live `$HOME`/`$PREFIX` state mutation, installed launcher/context
  wiring, network/download, signature cryptography or key rotation, archive
  extraction/adaptation, fresh-install bootstrap, Manager features, release
  publication, real product activation, package operations, or dependency adds.

#### must_hold

- Milestone 2 now fixes the Core artifact/state layout under explicit roots:
  immutable generation directories live under `generations/`; one authoritative
  `activation-state` file stores the logical `current`, `verified`, and
  `previous` generation identities as one bounded pointer set; one
  `activation-journal` records exactly the before/after states while a transition
  is pending. Temporary replacement files remain private implementation detail.
- Generation identities are opaque nonempty single-line values; parsing rejects
  empty values, embedded newline/NUL, unknown fields, duplicate fields, missing
  fields, extra records, and unsupported format versions. No lossy conversion or
  path-derived identity is permitted.
- State replacement is durable in this order: validate inputs; create+fsync a
  private journal temporary; atomically rename it to `activation-journal`; fsync
  the directory; create+fsync the new state temporary; atomically rename it over
  `activation-state`; fsync the directory; remove the journal; fsync the
  directory. No active state is removed before a replacement is ready, and a
  short/partial journal write can never become the canonical journal.
- Recovery with a journal accepts only two coherent cases: authoritative state
  equals the recorded `before` state (transition not committed) or equals the
  recorded `after` state (transition committed). It removes the stale journal and
  retains that complete state. Any third state, malformed journal/state, missing
  authoritative state where one is required, or before/after ambiguity fails
  closed without synthesizing pointers.
- Initial activation is represented explicitly with no prior state rather than a
  fabricated previous generation; rollback transitions must retain the former
  verified/current identity according to the same state semantics.
- M2-B1 never mutates a generation directory after it is presented as complete,
  never follows a path supplied by a generation identity, and never writes
  outside the explicit test-owned Core state root.
- Failures before the state rename leave the old state authoritative; failures
  after the state rename leave the new state authoritative. Recovery must make
  both cases deterministic and idempotent, including a stale journal after a
  successful commit.

#### build

- Add small `CoreStatePaths`, `GenerationPointerState`, `ActivationJournal`, and
  typed parse/I/O/recovery error surfaces. Use a canonical versioned text format
  with fixed field order and length-bounded values; do not introduce serde or a
  hashing dependency in this bundle.
- Use `OpenOptions::create_new` for transaction temporaries, `File::sync_all`,
  same-directory `rename`, and directory `sync_all` on the current Unix/Android
  target. Publish both journal and state only from fully written+synced private
  temporaries. Recovery may clean only the fixed transaction-owned temporary
  names; unrelated files are never removed.
- Isolate durability calls behind a tiny internal operation/fault-point boundary
  so tests can inject one failure at each ordered step without changing the
  public state semantics. Fault injection exists only for tests and must not
  weaken production ordering.
- Build activation from an explicit old pointer state plus a complete candidate
  generation identity; B12 candidate/readiness and later signed-release evidence
  will feed this transaction in later bundles rather than being reimplemented
  here.

#### verification

- focused: canonical encode/parse and malformed/collision cases; initial and
  ordinary activation; rollback-state transition semantics; failure injected
  before/after every durable boundary; partial journal/state temporary recovery;
  recovery from old+journal and new+journal; stale-journal idempotence;
  malformed/ambiguous/missing-state fail-closed cases; permission/create/rename/
  remove failures; no writes outside the test root; and generation directories
  unchanged byte-for-byte.
- done_when: focused M2-B1 tests pass; all 139 nonignored M1/B24 default tests
  remain green with the explicit B24 smoke still ignored by default; formatting
  and diff checks pass; locked release build is warning-free; one grouped final
  acceptance batch runs the full serial suite and eight complete default-parallel
  repetitions in repository-external Cargo targets. No live product path is
  touched.

## Milestone 2 required outcomes

1. Produce prebuilt Android/Termux Core release artifacts.
2. Implement the minimal fresh-install bootstrap.
3. Implement signed immutable release manifests and key-rotation policy.
4. Acquire and safely adapt official upstream artifacts.
5. Implement atomic update, activation, recovery, and rollback.
6. Prove offline install and recovery.
7. Prove concurrent launch/update behavior and injected-failure recovery.
8. Qualify isolated fresh-Termux and upgrade-from-legacy paths.
9. Produce a complete candidate and run the fresh independent product review.

## Milestone 2 completion gate

- exact source, artifact digests, generation, test set, and device/runtime
  boundary are recorded for every release claim;
- signed release and key-rotation policy are enforced before candidate use;
- candidate generation is complete before activation and active state never
  resolves to a mixed generation after injected interruption;
- update/rollback/offline recovery are crash-safe and concurrent launch/update
  behavior is bounded;
- fresh installation and legacy upgrade are demonstrated in isolated roots or
  devices without damaging the current working installation;
- prebuilt aarch64 Android/Termux artifacts require no on-device Rust toolchain;
- no resolver/auth/profile/session/Manager-owned state is damaged;
- the complete candidate passes the independent review gate before any publication
  authority is changed.

## Stop lines

- Do not mutate the current installed Codex product or live resolver/Manager/user
  state while implementing M2-B1; all mutable evidence remains test-owned.
- Do not add network, signature, bootstrap, archive, or Manager implementation to
  this state-transaction bundle.
- Do not spawn a planning agent, problem advisor, checkpoint reviewer,
  implementation worker, or coding subagent while user-controlled worker mode is
  OFF.
- Do not repeat a successful current-evidence read/search or repository-wide
  inventory without material staleness; narrow the next observation instead.
- Do not replace this contract for an in-scope technical choice. Revise authority
  only when new material evidence changes the outcome, boundary, must-hold
  conditions, or required verification.
- The primary Lead must inspect the actual diff and run the grouped load-bearing
  validation before acceptance.
- Do not modify `legacy/monolith`, sealed tags, publication refs, or the live
  installed launcher/runtime.

## Next milestone

There is no routine third Core milestone. Exhausting M2-B1 while Milestone 2 is
open causes the same Lead to compile the next bounded M2 contract. Completion of
all Milestone 2 gates produces the candidate for fresh independent review; review
acceptance, not bundle count, controls any later publication decision.
