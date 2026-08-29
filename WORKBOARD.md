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

#### current slice checkpoint

- Slice 0 began from the recorded red source snapshot at
  `8df74abfca793fe9e8008553b5d9742ba6d2b4d4`. The documentation-policy commit
  moved the bound working-tree base to
  `f4fd8138e4e8683ff92e7aff5d7148b7f4428876` without changing that source or
  its diff.
- Slice 0 is green on source SHA-256
  `a320a6bf50a27e01e16456e271bf32940b4d6648037c2520f2b8c5a8f0e7fb61`
  and source-diff SHA-256
  `93a998dd829f0b991eb373061382cbd6daf5e6810a506f57c5d355698f11a1a6`
  (`+830/-145`). This is working-tree evidence, not bundle acceptance.
- KEEP: the shared read-only upstream command probe and doctor adapter; the v2
  `current`/`previous` state codec; pinned trust/OpenSSL roots and typed errors;
  and the single strict manifest, policy, signature, inventory, staging,
  candidate-probe, and activation chain.
- COLLAPSE: activated launch and candidate probe now share one loaded-generation
  runtime qualification boundary; activation uses one source admission,
  recovery/sequence, staging, staged verification, probe, then pointer-update
  path.
- DELETE: the dead HOME generation-root helper, optional unsigned staging,
  one-use current-state wrapper, redundant state-parent creation, the
  `verified` field, and the superseded B3 public no-PREFIX/no-activation claim.
- Affected test disposition: B1 v2 state/fault cases are retained for slice 4;
  B2 root fixtures and both real qualification paths compile and pass; B3 keeps
  its six staging contracts with required signed-metadata retention; the public
  assertion is replaced by one signed temporary-HOME/PREFIX initial activation.
- Green evidence: warning-free locked test build; exact public test 1/1; B3
  staging 6/6; B2 loaded-route 1/1 and real-main 1/1; format and diff checks.
  Full-suite, release, live-smoke, protected-identity, ledger, and commit gates
  remain reserved for slice 5.
- Trust/manifest admission is green on source SHA-256
  `248e6847af679e06d042f00a58ffbb3ddfb24ae5548b29a4fa2780112b36b2ea`
  and source-diff SHA-256
  `baea48f020296b426b0c207e3d58e83a64881a3a4f4a1056b85a90c9a3522924`
  (`+1133/-156`). Admission now checks only the pinned HOME key and PREFIX
  OpenSSL before parsing, and accepts only canonical LF manifests with positive
  decimal sequence/count fields.
- Trust/manifest evidence: warning-free locked test build; named focused tests
  4/4; retained public signed activation 1/1; format and diff checks. Missing
  OpenSSL/key, a wrong pinned key despite correct source-adjacent trust material,
  exact-byte signature mutation, and signed policy mismatch all failed without
  creating a target generation, config directory, or activation state.
- Exact inventory/staging is green on source SHA-256
  `38b8337f811728849eb4d6a70e97c349575dbc11898841081b5c9fc1db6aaf01`
  and source-diff SHA-256
  `78ee1599ce7e4df8f2f32f31a83f3db115b45474210d68f58b37c02b8d31bd10`
  (`+1655/-192`). Actual inventory derivation now rejects non-canonical helper
  indices, control-bearing paths, unbounded descriptor helper counts, symlinked
  source/content roots, symlinked fixed files, special compatibility content,
  invalid UTF-8 names, and a symlinked publication root.
- Inventory/staging evidence: warning-free locked test build; named focused
  tests 4/4; affected trust tests 4/4; retained B3 staging 6/6; retained public
  signed activation 1/1; format and diff checks. Signed digest mismatch,
  omitted/missing/unlisted files, and unsafe source shapes all failed before a
  target generation or activation state was created. A valid staged copy kept
  the exact manifest/signature bytes, passed staged verification, then detected
  a post-stage runtime mutation without activation and with no candidate
  residue. Shared no-follow type checks replace duplicated admission/staging
  checks; silent actual-path de-duplication is removed rather than normalizing a
  filesystem contradiction.
- Probes/sequence/activation is green on source SHA-256
  `d6ee359f975b4495827533b90523c3eb553f45aa9d066023141ed35c3388e419`
  and source-diff SHA-256
  `32b608855c7ccc4696d0c04cf08258b0dfc95d4b1c445b46932ee59555fcdd6e`
  (`+1828/-197`). Review found and corrected one Workboard/code ordering defect:
  SPEC-owned anti-rollback now recovers and verifies current state before any
  candidate staging or execution, rather than probing an older signed runtime
  before rejecting its sequence.
- Activation evidence: warning-free locked test build; named focused tests 3/3;
  format and diff checks. The real public path proved initial activation and a
  second update with exactly one previous pointer, plus successful version and
  supported-doctor probes. Equal sequence was rejected before the deliberately
  unhealthy candidate could be staged or run. Version and doctor failures left
  the old activation-state bytes unchanged, left only complete signed inactive
  generations, and created no journal or state temporary.
- Simplified recovery/rollback is green on source SHA-256
  `bc9757f55cef6a4cba9dd478b1c75bae5440c815b889f4461135428b7b134744`
  and source-diff SHA-256
  `adffee15f56d764110f8baf455846f0e56fa7454935cc248e85bc0dbe19d6b10`
  (`+2159/-280`). The previously test-only rollback planner now reaches the real
  exact `codex update --rollback` Core path defined first in `SPEC.md`; it
  recovers state, requires one previous, verifies that signed target, binds its
  descriptor identity to the pointer, and uses the same atomic transaction.
- Recovery/rollback evidence: warning-free locked test and non-test dev builds;
  public rollback tests 2/2; retained pointer/journal/fault matrix 12/12;
  affected activation tests 3/3; update usage 1/1; format and diff checks. A
  valid rollback swapped current/previous and retained two complete signed
  generations. No-previous, mismatched target identity, and unverifiable target
  failures preserved the authoritative state bytes and left no transaction
  files. v2 now rejects control-bearing identities and equal current/previous.
- Final disposition: KEEP one v2 codec, one journal transaction/recovery path,
  one production filesystem I/O implementation, the test-only durable-call
  injector, and one public rollback bridge. COLLAPSE current/previous equality
  into pointer validation and local-update/rollback into the existing Core
  update dispatcher. DELETE the stale module-wide dead-code allowance, the
  redundant rollback equality branch, every `verified` representation, and the
  proof-only rollback closure.
- The first grouped acceptance was green on source SHA-256
  `bc9757f55cef6a4cba9dd478b1c75bae5440c815b889f4461135428b7b134744`:
  full serial 58 passed / 0 failed / 1 explicit smoke ignored by default; three
  complete default-parallel runs each 58/0/1; explicit real-Termux read-only
  smoke 1/1; warning-free locked release build; format and diff checks. The
  later identity-binding repair supersedes this as final acceptance evidence.
- Protected identities remained exact before/after that superseded run: live resolver SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  and installed launcher SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`;
  device, inode, mode, uid, gid, size, and mtime were also unchanged. No live
  generation, activation, Manager, resolver, auth/session/profile, package, or
  publication state was mutated.
- Final product-path diff review reopened slice 3 before commit: rollback and
  ordinary launch bind a pointer name to the signed generation identity, but
  forward update did not bind its recovered `current` target before carrying
  that name into `previous`. This is a same-class state-integrity defect, not a
  new feature. Freeze bundle acceptance, collapse installed-target verification
  into one helper, add a public pre-staging regression, then rerun the affected
  slice and the exact grouped acceptance on the new revision.
- Reopened slice 3 is green on source SHA-256
  `ea8c840a7f4bff1dcbe3fd3ae36b16b5acb4e0389b8ec8304a069584a1fa49ba`
  and source-diff SHA-256
  `3d5b17788c66aacf3fda26d317112c1b1e23e2ae3257d64f05dbbee4600f69c5`.
  One installed-target verifier now binds the expected path identity for
  current-sequence admission, staged publication, and rollback. The named
  public mismatch regression proved rejection before staging with exact state
  bytes and transaction-file absence; all activation tests passed 4/4, public
  rollback tests passed 2/2, and the locked non-test build, format, and diff
  checks are green. The affected identity-binding class is fully dispositioned;
  final grouped acceptance remains pending on this exact source.
- The first focused retry named a nonexistent Cargo package and was rejected
  before any test ran; it is not evidence. The corrected `codex` package filter
  is the nonzero 4/4 activation result recorded above.
- Final grouped acceptance is green on the repaired source and diff hashes
  above: full serial 59 passed / 0 failed / 1 explicit smoke ignored by default;
  three isolated complete default-parallel runs each 59/0/1; explicit
  real-Termux read-only smoke 1/1; warning-free locked release build; format and
  diff checks. Final direct review covered every changed production definition,
  state transaction, installed-generation lookup, public dispatcher branch,
  and mapped regression; no unresolved duplicate proof path or red gate remains.
- Protected identities remained exact before/after the repaired acceptance:
  live resolver SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  and installed launcher SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`;
  their recorded device, inode, mode, uid, gid, size, and mtime tuples are also
  unchanged. No live generation, activation, Manager, resolver,
  auth/session/profile, package, or publication state was mutated. Acceptance
  build directories created by this run were removed after validation.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — recover the public happy path | Exhaustively disposition the bound WIP, remove proof-only/dead duplication, restore test compilation, and replace the stale B3 public assertion with one signed temporary-HOME/PREFIX `update --local` initial-activation subprocess proof | warning-free locked test build; the exact public happy-path test runs nonzero and passes; actual diff inspected | green — 2026-08-29 |
| 1 — trust and manifest admission | Prove pinned-key selection, strict manifest parsing/order/bounds, signature verification, platform/API/schema/channel policy, and missing/wrong key/OpenSSL failures without staging or activation | named focused trust/manifest tests pass; bad inputs leave activation state absent/unchanged | green — 2026-08-29 |
| 2 — exact inventory and staging | Prove safe inventory paths, exact file-set equality, source and staged SHA-256 verification, signed metadata retention, and complete-or-absent publication | named focused inventory/staging tests pass; mismatch and unsafe content never activate | green — 2026-08-29 |
| 3 — probes, sequence, and activation | Enforce source admission → state recovery/sequence check → staging → staged verification → version/doctor probes → atomic activation; prove initial activation, update with one previous, probe failure, non-monotonic sequence, and the actual public result | named focused activation tests and public subprocess cases pass; old current remains unchanged on every pre-activation failure | green — 2026-08-29 |
| 4 — simplified recovery and rollback | Prove v2 `current` + optional `previous`, explicit rollback swap, journal recovery, and the retained crash/short-write/storage/permission fault matrix with no `verified` field or fallback ladder | complete pointer/journal focused matrix passes and changed state/test definitions receive final KEEP/COLLAPSE/DELETE audit | green — 2026-08-29 |
| 5 — grouped acceptance | Add no new product behavior; run the complete bundle acceptance and synchronize authority | full serial and repeated parallel suites, explicit live read-only smoke, format/diff checks, warning-free locked release, protected identities unchanged, ledger update, commit | green — 2026-08-29 |

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
  swaps current/previous through exact `codex update --rollback`. Rollback first
  recovers transaction state and reuses pinned-key admission to verify the one
  retained target; it does not apply forward anti-rollback policy, re-probe an
  already admitted generation, scan generations, or add a fallback. Ordinary
  launch still reads current only.
- Anti-rollback compares the signed new `release_sequence` with the signed
  manifest retained by the current generation. New sequence must be strictly
  greater. Initial activation has no prior sequence. Recovery and this check
  occur before staging or executing the candidate.
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
3. recover current activation state and enforce sequence monotonicity;
4. stage the complete inactive generation;
5. re-verify staged SHA-256 inventory;
6. run candidate version/doctor probes;
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
