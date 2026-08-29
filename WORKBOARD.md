# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M2-B6 is accepted at
  `92787c85e4bc27de867f800d1414125d6247a210`; M2-B7 is the selected
  implementation target
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- Execution discipline: follow `AGENTS.md` outcome-first closure rules; close one
  vertical proof slice before beginning another independent contract, stop on
  every red/nonzero-proof failure, and reserve grouped acceptance for the stable
  bundle

## Product-speed policy

- Add only the trust transition required to replace a compromised or expiring
  bootstrap key while retaining the one explicit signed rollback target. Do not
  build general PKI, key discovery, a key server, or an unbounded keyring.
- Preserve one local and one remote updater. Rotation must reuse their exact
  signed admission, staging, activation, recovery, and rollback path rather than
  add a second trust updater.
- Resolve transition authorization, durable key ownership, activation ordering,
  crash recovery, and previous-generation verification together before changing
  the manifest or trust-anchor format.
- Keep ordinary launch independent of OpenSSL, network, Manager, and key-rotation
  availability. Bootstrap packaging and device qualification remain later
  bundles; B7 must not implement them opportunistically.
- Keep Core dependency-free unless a vertical slice proves the existing Termux
  OpenSSL plus bounded Rust code cannot meet the approved contract. Do not
  install packages during development.

## Mandatory bundle execution method

- Bind branch, HEAD, dirty state, source identity, and authorities at every
  resume. A dirty resume records every red gate here before product mutation.
- Update `SPEC.md` before changing the signed manifest, trust-anchor ownership,
  persistent trust state, activation/rollback behavior, or security policy.
  Because B7 changes a security property, report the exact proposed SPEC delta
  and obtain user approval before that edit or any product mutation.
- Each slice closes vertically with its production behavior, named regression,
  nonzero focused invocation, relevant warning-free build/test result, and Lead
  diff inspection.
- A failed compile, zero-test or rejected test command, stale assertion, warning,
  leaked temporary root, unmapped production branch, or mismatched revision
  freezes new behavior until the whole affected class is dispositioned
  KEEP/COLLAPSE/DELETE.
- Cheap compile and focused gates run at slice boundaries. The full serial suite,
  three complete default-parallel runs, explicit live read-only smoke, protected
  identity check, and locked release build run only after all behavior slices are
  green.
- On acceptance, reduce this proof map into `GOAL.md`, replace this Workboard
  item, and commit. Do not preserve a parallel roadmap or evidence hierarchy.

## Selected next action

### M2-B7 — signed trust-key rotation and rollback compatibility

#### outcome

Define and implement the smallest signed trust transition that lets a release
replace the bootstrap-pinned Ed25519 verification key without accepting an
untrusted adjacent key, weakening anti-rollback, creating a second updater, or
making the one retained previous generation unverifiable. Key transition and
generation activation must recover together to one complete old or new trust and
generation state.

Slice 0 is authority-only. It must trace the accepted B4/B5 trust, inventory,
activation, recovery, and rollback path; compare the minimum viable transition
shapes; and report one exact recommended contract. No `SPEC.md`, product code,
manifest format, or persistent layout changes until the user approves that
security choice.

#### accepted input

- B6 implementation:
  `92787c85e4bc27de867f800d1414125d6247a210`.
- B6 release-builder library SHA-256:
  `5cf290e919adaa4ef92f1715cff4cb0cdb2f6ad9973020f0111f60f00f4019ca`.
- B6 final evidence: full serial Core 70/0/1-ignored plus builder 5/0; three
  complete default-parallel runs at the same counts; explicit live read-only
  smoke 1/1; warning-free locked release build; zero test-root residue; exact
  protected live identities.
- B4/B5 admit `codex-release-v2` only through one bootstrap-provisioned Ed25519
  public key, exact-manifest signature verification, monotonic release sequence,
  complete generation staging, and one explicit signed previous target.
- Current production has no key-rotation statement, next-key field, durable trust
  transaction, previous trust key, alternate-key search, or recovery rule tying
  trust state to generation state. Historical or hypothetical formats are not
  implementation authority.

#### current checkpoint

- Selection bound clean `rewrite/rust-core@92787c85e4bc27de867f800d1414125d6247a210`,
  ahead of its remote by seven commits. B6 changed no installed or active product
  state and no branch was published.
- Remaining Milestone 2 gates are prebuilt Core/bootstrap, signed key rotation,
  launch/update overlap proof, offline device recovery, isolated fresh-Termux
  and legacy-upgrade qualification, and the final independent review. Rotation
  precedes bootstrap so bootstrap does not freeze an unreviewed trust format.
- No rotation design has user approval. `SPEC.md` and product code remain frozen
  while Slice 0 is selected. Worker mode remains OFF and the primary Lead owns
  the authority analysis directly.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — trust-transition authority | Exhaustively trace current key consumers and transaction boundaries, compare minimum transition shapes, and bind authorization, state, activation, rollback, and recovery semantics | exact recommended SPEC delta reported; user approval; SPEC-only diff reviewed; no product mutation | selected |
| 1 — signed transition admission | Admit only the approved current-key-authorized transition through the existing v2 successor format and local/remote verifier | named valid-transition and malformed/unauthorized-key matrices pass nonzero; ordinary non-rotating releases remain one direct path | blocked by slice 0 |
| 2 — atomic trust activation and recovery | Couple the bounded trust-state change to generation activation/recovery so every outcome is complete old or new and the retained previous release remains verifiable | focused activation, rollback, stale-journal, kill-point, and no-partial-state regressions pass nonzero | blocked by slice 1 |
| 3 — updater integration | Prove local, remote, current verification, and rollback reuse the same trust policy with no launch dependency or second updater | one end-to-end local transition, one remote reuse, one rollback regression, and affected B4/B5 groups pass nonzero | blocked by slice 2 |
| 4 — grouped acceptance | Add no new product behavior; run final bundle proof and synchronize authority | full serial and three complete parallel suites, explicit live read-only smoke, format/diff, warning-free locked release, zero residue, protected identities unchanged, GOAL update, commit | blocked by slice 3 |

#### protected surfaces

- `$PREFIX/etc/resolv.conf`, installed `$PREFIX/bin/codex`, live generations and
  activation/trust state, Manager state, auth/profile/session data, package
  state, private signing keys, and publication branches remain read-only.
- All signing keys used in tests are ephemeral and repository-external. No test
  key becomes product trust authority or persists after its test root is removed.

#### stop lines

- no SPEC or product mutation before explicit user approval of the security
  contract;
- no untrusted key beside a release, alternate-key search, network key lookup,
  CA/PKI subsystem, unbounded key history, or private key in Core;
- no second updater, ordinary-launch verification dependency, fallback ladder,
  multi-writer protocol, bootstrap implementation, device cutover, or release
  publication;
- no legacy source copying or translation;
- no worker, planner, or reviewer unless the user explicitly turns worker mode
  on.

## Next action after M2-B7

After rotation is accepted, select the prebuilt Core and minimal fresh-bootstrap
bundle from fresh authority. B7 does not pre-order or claim device installation,
offline qualification, legacy upgrade, or independent-review evidence.
