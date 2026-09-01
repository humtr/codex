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

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — overlap authority and harness | Rebind accepted B8, trace the real launch loader plus activation/recovery durable boundaries, and build the smallest isolated synchronization/fault harness without changing production behavior | exact observation points and old/new invariants recorded; baseline canonical check green; no product mutation | selected |
| 1 — successful activation overlap | Hold a launch-capable old generation, overlap repeated ordinary launches with one successful activation to a complete new generation, and classify every launch observation | named overlap regression proves every completed launch is wholly old or wholly new; no mixed generation, fallback, or launch-time coordination | blocked by slice 0 |
| 2 — failed update overlap | Overlap ordinary launches with a candidate/update failure before authoritative activation and prove the old current remains the only launchable generation | named injected-failure regression passes across the selected pre-activation failure boundaries; state/current and launch observations remain old and complete | blocked by slice 1 |
| 3 — recovery overlap | Exercise recoverable activation journal states while launches and recovery interleave, proving recovery resolves to one complete old/new state and launch never consumes temporary/journal state as a generation | named recovery-overlap matrix passes at each existing durable boundary; no new recovery mechanism | blocked by slice 2 |
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
