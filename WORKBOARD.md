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

### M2-B4 — signed local release admission and atomic activation

#### outcome

Turn the accepted B3 local staging path into one complete offline update path:
verify one pinned-key Ed25519 release manifest and exact SHA-256 generation file
inventory, stage the admitted source, run the minimum candidate probes, and
activate it with the existing crash-safe transaction. At the same time remove
the redundant `verified` pointer so activation state is only `current` plus one
explicit `previous` rollback target.

#### accepted input

- B3 code: `b692853a436e7df2540ccb1c52e967af4e921375`.
- B3 focused 7/7; full serial 46/0/1-ignored; explicit live smoke 1/1;
  default-parallel 3/3; warning-free locked release; protected live identities
  unchanged.
- Current Termux already has OpenSSL 3.6.3 at `$PREFIX/bin/openssl`; SHA-256 and
  Ed25519 `pkeyutl -verify -rawin -pubin` were proven in a job-private roundtrip.
  B4 must use this existing executable only and must not install crypto tooling.

#### release trust and manifest

- The trusted Ed25519 public key is exactly
  `~/.local/lib/codex/core/release-public-key.pem`, provisioned by bootstrap.
  No key search, alternate key, release-supplied key, TOFU, or fallback exists.
- A local source accepted for activation contains regular `release.manifest` and
  `release.sig` files in addition to the B3 generation files.
- `release.manifest` is a strict bounded UTF-8 format with exact field order:
  format/version, `generation_id`, positive monotonic `release_sequence`,
  `channel`, platform, architecture, Core API, persistent schema, `file_count`,
  then an exact SHA-256 inventory.
- B4 supports the single current release channel `stable`. Do not add channel
  negotiation or fallback.
- Inventory paths are safe relative UTF-8 paths under the fixed generation
  layout. They may name only `generation.meta`, `runtime`, optional `manager`,
  declared `helpers/<index>`, and regular files recursively beneath `compat/`.
  Every load-bearing file appears exactly once and no listed file may escape the
  generation root.
- Signature verification occurs over the exact manifest bytes before staging.
  SHA-256 verification occurs against the explicit source and again against the
  staged immutable generation before activation.
- The admitted `release.manifest` and `release.sig` are copied into the private
  candidate before B3's atomic publication so an activated generation retains
  the signed sequence/inventory that admitted it.

#### activation and simplification

- Remove `verified` from `GenerationPointerState`, state encoding, journal
  encoding, parsers, fault tests, and docs. It is redundant because B4 permits
  only an admitted/probed generation to become `current`.
- Activation state becomes `current` + optional `previous`. Initial activation
  has no previous; update activation sets previous to the old current; rollback
  swaps current/previous. Ordinary launch still reads current only.
- Anti-rollback compares the signed new `release_sequence` with the signed
  manifest retained by the current generation. New sequence must be strictly
  greater. Initial activation has no prior sequence.
- Before activation, qualify the staged generation with the existing B2 loader,
  run upstream `--version` as a read-only process probe, and run the existing
  bounded upstream doctor probe when the descriptor declares doctor support.
- Probe failure leaves the old active generation untouched. A complete inactive
  staged generation may remain; do not add cleanup registries/retry ladders for
  that harmless state.
- Activation uses the existing journaled M2-B1 transaction after state recovery.
  Do not add lock/lease/fencing or a second transaction mechanism.

#### public path

`codex update --local <directory>` becomes the complete B4 offline path:

1. load the pinned public key and `$PREFIX/bin/openssl`;
2. verify signed manifest/policy/source SHA-256 inventory;
3. stage the complete inactive generation;
4. re-verify staged SHA-256 inventory;
5. run candidate version/doctor probes;
6. recover current activation state and enforce sequence monotonicity;
7. atomically activate the candidate;
8. report success or fail without changing the old current generation.

#### must hold

- no release-supplied or dynamically discovered trust key;
- no custom/home-grown signature or hash implementation;
- no network, package manager, archive parser, automatic update, lock/fencing,
  multi-writer protocol, or ordinary-launch fallback;
- signed policy mismatch, bad signature, digest mismatch, sequence rollback, or
  probe failure occurs before activation;
- current/previous always name complete immutable generations;
- resolver, installed launcher, Manager state, auth/session/profile state, and
  package state remain read-only in tests.

#### verification

- focused: valid initial activation, valid update with one previous, explicit
  rollback, bad signature, wrong trusted key, digest mismatch, missing/unlisted
  inventory file, policy mismatch, non-monotonic sequence, version-probe failure,
  doctor failure, OpenSSL/key missing, and crash/fault matrix after the pointer
  simplification;
- actual public `update --local` subprocess in temp HOME/PREFIX with a job/test
  generated Ed25519 keypair where only the public key enters the product trust
  path;
- retained full serial suite, explicit real-Termux read-only smoke, complete
  default-parallel repetitions, `cargo fmt --check`, `git diff --check`, and
  warning-free locked release build;
- protected live resolver/installed-launcher identities unchanged before/after.

#### stop lines

- no remote release lookup/download;
- no archive extraction;
- no live product activation in tests;
- no release signing private key stored in the repository or product state;
- no extra fallback, key rotation, revocation service, transparency log, or
  multi-writer coordination in B4.

## Next action after M2-B4

With a complete signed offline update path working, add the smallest immutable
remote release-manifest/artifact acquisition path feeding the exact same B4
admission/staging/activation flow. Do not create a second updater.
