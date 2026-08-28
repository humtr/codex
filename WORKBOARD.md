# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M1-R2 is closed, M2-B3 is
  accepted at `b692853a436e7df2540ccb1c52e967af4e921375`, and M2-B4 is the selected
  implementation target
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- Execution discipline: follow `AGENTS.md` outcome-first closure rules; reuse
  fresh same-revision evidence, expand to exhaustive review only on a systemic
  breadth trigger, close one vertical proof slice before starting another, and
  batch only the stabilized bundle's full acceptance validation

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

## Mandatory bundle execution method

- The current item advances through the ordered proof map below. A slice is
  closed only when its production behavior, regression, nonzero focused command,
  relevant compile/test result, and Lead diff inspection agree.
- Do not add a second independent behavior while the current slice is red or
  unmapped. A compile failure, zero-test invocation, stale superseded test,
  warning/dead path, or missing proof mapping freezes new product behavior.
- A red slice is repaired by inspecting the entire affected class and recording
  KEEP/COLLAPSE/DELETE dispositions. Do not retain optional or compatibility
  behavior merely to keep an older test shape alive.
- Cheap compile and focused commands run at every slice boundary. The full
  serial suite, repeated parallel runs, explicit live smoke, protected-surface
  check, and release build run once after all slices are green.
- This proof map belongs only to the current bundle. On acceptance, summarize
  it in `GOAL.md`, replace this Workboard item, and do not preserve another
  roadmap or evidence hierarchy.

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

#### recovery checkpoint

- Bound HEAD: `8df74abfca793fe9e8008553b5d9742ba6d2b4d4`.
- Bound `crates/core/src/main.rs` SHA-256:
  `de0943aa415bb6a7416a7e83a59755f53f90a53df1d10a437694e96a66433575`.
- Bound source-diff SHA-256:
  `7007cad7881378a0a66f75fa207f968c2b5d01e2efebe22b67e1588121d81621`.
- Lead disposition: the `+684/-88` source diff is retained as repairable WIP,
  not accepted evidence. New B4 behavior is frozen until slice 0 closes.
- Red gates: the test target does not compile because three `LocalCoreRoots`
  fixtures lack the new trust/OpenSSL fields; no B4 regression exists; the old
  B3 public test asserts no `PREFIX` and no activation; the release build has
  one dead-code warning; state recovery/sequence checking precedes candidate
  staging/probes instead of the ordered public path.
- Mandatory breadth audit covers every production definition changed by this
  source diff and every affected B1-B3 test/probe. At minimum it must decide the
  dead generation-root helper, optional unsigned staging branch, duplicate
  loaded-generation/runtime qualification, stale B3 public test, and activation
  ordering as KEEP, COLLAPSE, or DELETE.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — recover the public happy path | Exhaustively disposition the bound WIP, remove proof-only/dead duplication, restore test compilation, and replace the stale B3 public assertion with one signed temporary-HOME/PREFIX `update --local` initial-activation subprocess proof | warning-free locked test build; the exact public happy-path test runs nonzero and passes; actual diff inspected | selected |
| 1 — trust and manifest admission | Prove pinned-key selection, strict manifest parsing/order/bounds, signature verification, platform/API/schema/channel policy, and missing/wrong key/OpenSSL failures without staging or activation | named focused trust/manifest tests pass; bad inputs leave activation state absent/unchanged | blocked by slice 0 |
| 2 — exact inventory and staging | Prove safe inventory paths, exact file-set equality, source and staged SHA-256 verification, signed metadata retention, and complete-or-absent publication | named focused inventory/staging tests pass; mismatch and unsafe content never activate | blocked by slice 1 |
| 3 — probes, sequence, and activation | Enforce source admission → staging → staged verification → version/doctor probes → state recovery/sequence check → atomic activation; prove initial activation, update with one previous, probe failure, non-monotonic sequence, and the actual public result | named focused activation tests and public subprocess cases pass; old current remains unchanged on every pre-activation failure | blocked by slice 2 |
| 4 — simplified recovery and rollback | Prove v2 `current` + optional `previous`, explicit rollback swap, journal recovery, and the retained crash/short-write/storage/permission fault matrix with no `verified` field or fallback ladder | complete pointer/journal focused matrix passes and changed state/test definitions receive final KEEP/COLLAPSE/DELETE audit | blocked by slice 3 |
| 5 — grouped acceptance | Add no new product behavior; run the complete bundle acceptance and synchronize authority | full serial and repeated parallel suites, explicit live read-only smoke, format/diff checks, warning-free locked release, protected identities unchanged, ledger update, commit | blocked by slice 4 |

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
