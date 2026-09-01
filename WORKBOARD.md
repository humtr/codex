# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M2-B7 signed trust-key rotation
  is accepted by the current authority state; M2-B8 is selected
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- Execution discipline: follow `AGENTS.md` outcome-first closure rules; close one
  vertical proof slice before beginning another independent contract, stop on
  every red/nonzero-proof failure, and reserve grouped acceptance for the stable
  bundle

## Product-speed policy

- Build only the delivery pieces already required by `SPEC.md`: one prebuilt
  Termux Core artifact and the smallest fresh-install bootstrap needed to verify
  one immutable `codex-release-v3`, stage/probe it, initialize v3 state, and
  activate the first complete generation.
- Reuse B6 artifact adaptation plus the accepted B7 release/trust/state formats.
  Do not add a second updater, alternate bootstrap trust path, package-manager
  installer, discovery service, archive fallback, or another persistent state
  owner.
- Fresh bootstrap may use the bootstrap-provisioned Ed25519 key only before v3
  state exists. After initial activation, Core owns the accepted v3 state and the
  bootstrap key is not a Core recovery fallback.
- Keep release production separate from device installation. Building and
  qualifying prebuilt artifacts may compile in the release workspace; the
  supported install/update path must not require compilation on the target
  device.
- B8 is not live cutover or device qualification. Offline-device recovery,
  fresh-Termux qualification, legacy upgrade, launch/update overlap proof, and
  final independent review remain separate gates unless fresh authority later
  selects them.

## Mandatory bundle execution method

- Fresh-bind branch, HEAD, dirty state, current authority, and protected live
  identities before each implementation resume.
- The registered validation metadata is currently stale and npm-bound for this
  Rust orphan lineage. Repair that project validation routing before the first
  B8 product mutation, prove a green direct Rust baseline, and do not count the
  historical npm ENOENT as product evidence.
- Update `SPEC.md` first if B8 discovers that the existing bootstrap/artifact
  contract is insufficient. A new security-property change still requires
  explicit user approval before mutation; implementation choices within the
  accepted contract do not.
- Each slice closes vertically with production behavior, named nonzero focused
  proof, warning-free compile/build as relevant, and Lead diff inspection. A red
  compile/test/warning, leaked temporary root, stale assertion, or mismatched
  revision freezes new behavior until repaired.
- Run the full serial suite, three complete default-parallel suites, explicit
  live read-only smoke, zero-residue check, protected identity comparison, and
  warning-free locked release build only after all B8 behavior slices are green.
- On acceptance, reduce B8 evidence into `GOAL.md`, replace this Workboard item,
  and commit. Do not publish or cut over without separate authority.

## Selected next action

### M2-B8 — prebuilt Core and minimal fresh bootstrap

#### outcome

Produce and qualify the smallest release-production Core artifact and
fresh-install bootstrap that can start from no installed Core/v3 state, verify
one immutable v3 release through the bootstrap-provisioned key, construct one
complete initial generation, run the required self/probe checks, and initialize
one authoritative v3 state. The result must feed the same generation layout and
future updater path already accepted in B4-B7 without on-device compilation or a
second installer/update protocol.

#### accepted input

- B7 Core source SHA-256:
  `c4e501ece0ac6ccf75a01409f7e8e804297b326a5ead6317516f9f9755f1a3e0`.
- B7 approved SPEC SHA-256:
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- B7 final grouped evidence: Core 75/0/1-ignored plus builder 5/0 in serial and
  each of three complete parallel runs; explicit live read-only smoke 1/1;
  warning-free locked release; zero test-root residue; exact protected live
  identities unchanged.
- B6 release builder already accepts an explicit Core artifact and produces one
  complete unsigned generation source without signing, installation, activation,
  or live-state mutation.
- B7 defines the only initial trust transition: the bootstrap key may authenticate
  the first v3 release and initialize `update_key/current/current_key`; Core does
  not fall back to that key after state exists.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — fresh authority and validation routing | Rebind the committed B7 state, repair Rust project-validation metadata, inventory current release/build/bootstrap boundaries, and select the smallest artifact/bootstrap implementation shape without changing product behavior | canonical Rust validation route works nonzero; direct baseline agrees; exact B8 file/artifact boundary recorded; no product mutation | selected |
| 1 — prebuilt Core artifact | Produce one immutable release-production Core artifact with explicit platform/architecture identity and digest, without installing it or claiming target-device compilation | named artifact build/identity/digest tests pass nonzero; warning-free locked release build; no live mutation | blocked by slice 0 |
| 2 — fresh bootstrap initial trust | Implement only the bootstrap operations already authorized by SPEC/B7: environment check, immutable v3 admission with bootstrap key, complete staging/probe, and initial v3 state activation in isolated roots | valid local initial install plus malformed/key-mismatch/probe/fault matrix pass nonzero; no fallback or partial state | blocked by slice 1 |
| 3 — release-production integration | Feed the prebuilt Core artifact through B6 generation production and the same v3 signed-release fixture/bootstrap path; prove no duplicate updater or archive/install path appears | one complete release-production-to-bootstrap flow and affected B6/B7 groups pass nonzero | blocked by slice 2 |
| 4 — grouped acceptance | Add no new behavior; run final bundle proof and synchronize authority | full serial + three complete parallel suites, explicit live read-only smoke, format/diff, warning-free locked release, zero residue, protected identities unchanged, GOAL update, commit | blocked by slice 3 |

#### protected surfaces

- `$PREFIX/bin/codex`, `$PREFIX/etc/resolv.conf`, current live generations/trust
  state, Manager state, auth/profile/session data, package state, and publication
  refs remain read-only throughout B8 development and acceptance.
- No private signing key is stored in Core, bootstrap, repository product data,
  or persistent device state. Test signing keys remain ephemeral and external to
  product trust.

#### stop lines

- no live installation, activation, device cutover, or publication;
- no package-manager install or supported on-device compilation path;
- no second updater/bootstrap recovery authority, adjacent-key trust, discovery,
  mirror/fallback service, general PKI, or unbounded keyring;
- no security-contract expansion without fresh SPEC-first user approval;
- no legacy source copying or translation;
- no worker, planner, or reviewer unless the user explicitly turns worker mode
  on.
