# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M2-B8 prebuilt Core and minimal
  fresh bootstrap is accepted by the paired `GOAL.md` authority update at
  product tip `192fcece0b416004bddd9181e24b1245290ebe81`; M2-B9 is selected
- Remote `origin/rewrite/rust-core` remains
  `253156c37a2bd22af8faae0bce03587999ffd136`; no B8 commit is published
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- The normal B8 acceptance authority commit was rejected before commit by the
  known orphan-lineage pre-commit hook because
  `tools/update-wrapper-version.sh` is absent. HEAD and the exact staged
  GOAL/WORKBOARD tree remained unchanged. This authority closure may use the
  established `--no-verify` precedent only after re-staging this record and
  revalidating the exact two-path index; the shared hook remains untouched
- Execution discipline: follow `AGENTS.md` outcome-first closure rules; close one
  vertical proof slice before beginning another independent contract, stop on
  every red/nonzero-proof failure, and reserve grouped acceptance for the stable
  bundle

## Product-speed policy

- B9 is proof-first. The accepted SPEC already requires basic launch/update
  overlap and injected-failure coverage while explicitly rejecting speculative
  multi-writer coordination without demonstrated product need.
- Start from the existing invariants only: generations are complete or absent,
  candidate construction is outside the active path, ordinary launch reads only
  the one authoritative `current`, and activation/recovery publish one complete
  `codex-activation-state-v3` old or new state.
- Do not add a lock, lease, fencing token, retry/fallback ladder, second state
  owner, alternate recovery path, or multi-writer protocol unless an isolated
  reproducible overlap test demonstrates a concrete product failure that the
  accepted invariants do not cover.
- Simultaneous update attempts are not a first-class product feature. B9 must
  prove launch safety across one normal updater transaction overlapping launch;
  it does not need to make multiple updater writers succeed concurrently.
- Ordinary launch remains free of signature verification, OpenSSL, network I/O,
  generation scans, `previous` fallback, or updater coordination.

## Mandatory bundle execution method

- Fresh-bind branch, HEAD, dirty state, authority files, remote branch, and
  protected live identities before product mutation and after every local commit.
- Project registry revision 3 Rust profiles are canonical: `portable` is locked
  workspace check, `portable-install` is locked workspace build, and
  `portable-test` is the locked serial workspace suite. The separate tmcp
  `project.validation.describe` package.json limitation is not product evidence.
- Use isolated HOME/PREFIX/TMPDIR roots for all overlap/fault proof. Never point a
  test updater or candidate generation at the installed live product.
- Prefer test-only synchronization/fault instrumentation around existing state
  boundaries over production coordination code. Such instrumentation must not
  create a new public command, persistent format, trust source, or release path.
- Update `SPEC.md` first only if a concrete failing proof requires a normative
  behavior/security change. Any new security-property change, trust object, or
  persistent state owner requires explicit user approval before that mutation.
- Each behavior slice closes with named nonzero focused proof, relevant canonical
  check/build, residue check, and Lead diff inspection. A red gate freezes new
  behavior until its local cause is repaired and rerun.
- After all B9 proof slices are green, run the full serial suite, three complete
  default-parallel suites, explicit live read-only smoke, warning-free locked
  release build, format/diff checks, zero residue, and protected identity
  comparison. Then reduce evidence into `GOAL.md`, replace this Workboard item,
  and commit. Do not push or cut over without separate authority.

## Selected next action

### M2-B9 — launch/update overlap and injected-failure proof

#### outcome

Prove in isolated roots that ordinary launches which overlap a successful
activation, a failed pre-activation update, and activation recovery can observe
only one complete old generation or one complete new generation. The proof must
exercise the real ordinary-launch loader and the accepted v3 activation/recovery
state machine. It must not make launch depend on updater coordination and must
not invent a production lock merely to serialize the test.

#### accepted input

- Accepted product tip:
  `192fcece0b416004bddd9181e24b1245290ebe81`.
- Accepted SPEC SHA-256:
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- B8 Core source SHA-256:
  `6b401eb2890e6bad3117685cb7ce156690a5dfe53b0bab83fba7c5d335ca22bf`.
- B8 release-builder source SHA-256:
  `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`.
- B8 bootstrap SHA-256:
  `c1b107699a64c08cc49a99ceb433c3dd1b6c7ca53637fb0b53cc73b6ce35e9fa`.
- B8 final acceptance: canonical serial Core 81/0/1-ignored plus builder 7/0;
  three complete default-parallel workspace suites at the same counts; explicit
  real-Termux read-only smoke 1/1; warning-free locked release build; format,
  bootstrap shell syntax, and diff checks green; zero test-root/build residue;
  exact protected launcher and resolver identities unchanged.
- SPEC states that ordinary launch reads only `current`; candidate construction
  occurs outside the active path; activation/recovery transact the complete
  bounded trust-and-generation state; overlap must preserve a complete state
  boundary; and launch must never observe a mixed or partially constructed
  generation.
- Slice 0 fresh-bound clean authority commit `0bcac01...` and remote
  `253156c...`; canonical project-registry `portable` passed with locked workspace
  check. The real ordinary path is `run_public_main` -> `execute_activated_route`
  -> `load_activated_generation`; the loader currently invokes
  `recover_activation_state` and then opens exactly the recovered `current`
  generation. The accepted activation transaction has exactly eight durable I/O
  calls: write/sync journal temporary, publish journal, sync root, write/sync
  state temporary, atomically replace activation state, sync root, remove
  journal, sync root.
- Slice 0 added test-only `M2B9PauseIo`, which wraps the existing `ActivationIo`
  boundary without changing production behavior or persistent/public contracts.
  The named harness pauses a real old->new activation after durable call 3,
  proves the canonical journal is durable while authoritative state remains the
  complete old state and no temporary state file exists, resumes the remaining
  five calls, then proves the same real loader resolves the complete new
  generation with no transaction residue. `cargo fmt --check` and `git diff
  --check` passed. The first bare-name `--exact` invocation selected zero tests
  and was rejected as non-evidence; the corrected fully-qualified exact focused
  test passed 1/1 with zero matching temp-root residue.
- The normal Slice 0 commit was rejected before commit by the known
  orphan-lineage pre-commit hook because `tools/update-wrapper-version.sh` is
  absent. HEAD and the staged Core/WORKBOARD tree were unchanged. Slice 0 uses
  the established exact-tree `--no-verify` precedent only after this record is
  re-staged and the exact two-path index, absence of `target/`, and clean
  unstaged/untracked state are revalidated; the shared hook remains untouched.
- Slice 1 first stopped on formatting-only drift before its behavior test; the
  exact regression was formatted without adding behavior. The corrected named
  overlap proof then failed nonzero at durable call 1: while updater paused after
  writing/syncing `activation-journal.tmp`, ordinary
  `load_activated_generation` called `recover_activation_state`, removed that
  updater-owned temporary, and the updater's next journal publication failed
  with ENOENT. Authoritative state remained the complete old state and no
  transaction residue remained. This is a concrete launch/update overlap defect,
  not a hypothetical coordination case.
- Fresh SPEC recheck after the red shows no contract delta is required: SPEC
  already says ordinary launch reads only `current`, while `codex update` must
  recover v3 state before update. The selected minimal repair therefore removes
  recovery ownership from ordinary launch only: `load_activated_generation`
  reads the authoritative pointer with `read_pointer_state` and keeps all
  updater/rollback recovery paths unchanged. This adds no lock, retry, fallback,
  state owner, persistent object, trust behavior, or public surface.
- After that one-line repair, the exact Slice 1 overlap regression passed 1/1
  across durable calls 1, 3, 4, and 6 with zero focused residue. The existing B2
  loader regressions passed 3/3, the Slice 0 harness remained green 1/1, `cargo
  fmt --check` and `git diff --check` passed, and canonical project-registry
  `portable` locked workspace check passed. Lead diff inspection found exactly
  one production behavior line (`recover_activation_state` ->
  `read_pointer_state`) plus the mapped Slice 1 regression and this authority
  record; no updater/recovery implementation, public surface, or persistent
  format changed.
- The normal Slice 1 commit was rejected before commit by the same known
  orphan-lineage pre-commit hook because `tools/update-wrapper-version.sh` is
  absent. HEAD and the exact staged Core/WORKBOARD tree remained unchanged.
  Slice 1 may therefore use the established `--no-verify` precedent only after
  this record is re-staged and the exact two-path index, clean unstaged/untracked
  state, and absence of `target/` are revalidated; the hook remains untouched.
- Slice 2 proves failed pre-activation updates cannot perturb ordinary launch.
  One test-only regression injects both before/after failures at durable calls
  1 through 4, requires authoritative state to remain the complete old state,
  snapshots any journal/temporary bytes before launch, proves the ordinary
  loader returns only the old generation without mutating those transaction
  files, and then proves updater-owned recovery restores the same old state with
  zero transaction residue. The exact focused proof passed 1/1 across all eight
  combinations; all B9 tests passed 3/3; the existing B1 every-durable-boundary
  recovery matrix passed 1/1; `cargo fmt --check` and `git diff --check` passed;
  canonical project-registry `portable` locked workspace check passed; and the
  focused residue scan returned zero. No production line, updater/recovery
  behavior, persistent format, public surface, or SPEC contract changed.
- The normal Slice 2 commit was rejected before commit by the same inherited
  orphan-lineage pre-commit hook because `tools/update-wrapper-version.sh` is
  absent. HEAD remained `10d368a...` and the exact staged two-path tree remained
  unchanged. Slice 2 may use the established `--no-verify` closure only after
  restaging this record, revalidating the exact Core/WORKBOARD index, confirming
  no unstaged/untracked path and no `target/`, and leaving the shared hook
  untouched.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — overlap authority and harness | Rebind accepted B8, trace the real launch loader plus activation/recovery durable boundaries, and build the smallest isolated synchronization/fault harness without changing production behavior | exact observation points and old/new invariants recorded; baseline canonical check green; no product mutation | complete: canonical check green; exact 8-call boundary traced; test-only pause harness focused 1/1; format/diff clean; zero focused residue |
| 1 — successful activation overlap | Hold a launch-capable old generation, overlap repeated ordinary launches with one successful activation to a complete new generation, and classify every launch observation | named overlap regression proves every completed launch is wholly old or wholly new; no mixed generation, fallback, or launch-time coordination | complete: concrete call-1 defect reproduced; one-line ordinary-loader repair; Slice 1 1/1, B2 loader 3/3, Slice 0 1/1, canonical check green, zero focused residue |
| 2 — failed update overlap | Overlap ordinary launches with a candidate/update failure before authoritative activation and prove the old current remains the only launchable generation | named injected-failure regression passes across the selected pre-activation failure boundaries; state/current and launch observations remain old and complete | complete: before/after faults at calls 1..4; focused 1/1, B9 3/3, B1 durable matrix 1/1, canonical check green, zero residue; test-only change only |
| 3 — recovery overlap | Exercise recoverable activation journal states while launches and recovery interleave, proving recovery resolves to one complete old/new state and launch never consumes temporary/journal state as a generation | named recovery-overlap matrix passes at each existing durable boundary; no new recovery mechanism | selected |
| 4 — grouped acceptance | Add no new behavior; run final bundle proof and synchronize authority | full serial + three complete parallel suites, explicit live read-only smoke, warning-free locked release, format/diff, zero residue, protected identities unchanged, GOAL update, commit | blocked by slice 3 |

#### protected surfaces

- `$PREFIX/bin/codex`, `$PREFIX/etc/resolv.conf`, installed live generations and
  trust state, Manager state, auth/profile/session data, package state, and
  publication refs remain read-only throughout B9 development and acceptance.
- No private signing key may enter Core, repository product data, or persistent
  device state. Test signing material remains ephemeral in isolated roots.

#### stop lines

- no live installation, activation, device cutover, or publication;
- no speculative lock, lease, fencing token, multi-writer protocol, fallback
  ladder, generation scan, or launch-time trust verification;
- no second updater, second state owner, alternate recovery path, adjacent-key
  trust, discovery service, general PKI, or unbounded keyring;
- no production behavior change unless a concrete B9 failing proof demonstrates
  a gap in the accepted complete-generation/atomic-state invariants;
- no normative security-contract expansion without fresh SPEC-first explicit
  user approval;
- no legacy source copying or translation;
- no worker, planner, or reviewer unless the user explicitly turns worker mode
  on.
