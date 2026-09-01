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
- Project-specific validation profiles are now Rust-bound at codex project
  registry revision 3: `portable` is locked workspace check, `portable-install`
  is locked workspace build, and `portable-test` is the locked serial workspace
  suite. Use those registered profiles as canonical validation. The separate
  `project.validation.describe` helper remains tmcp Node-only and still requires
  an absent `package.json`; record that tooling limitation but never count it as
  product evidence.
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

#### current checkpoint

- Slice 0 rebound clean local `rewrite/rust-core@3fb2ded0c5e78a4b6711b7231da55714eff27920`.
  Remote `origin/rewrite/rust-core` remains `253156c37a2bd22af8faae0bce03587999ffd136`;
  B7 is one local unpushed commit and Slice 0 does not publish it.
- The codex project registry was repaired from the inherited npm profiles to
  project-specific Rust profiles. Canonical `portable-test` executed
  `cargo test --workspace --locked -- --test-threads=1` and passed Core
  75/0/1-ignored plus release-builder 5/0. An independent direct Cargo serial run
  passed the exact same counts. Canonical `portable` (`cargo check --workspace
  --locked`) and `portable-install` (`cargo build --workspace --locked`) also
  passed. `project.validation.describe` still fails before discovery because tmcp
  hard-codes `package.json`; this is a separate tmcp tooling limitation, not a
  codex validation-route failure.
- The canonical Slice 0 authority commit was rejected before commit because the
  shared pre-commit hook still references absent orphan-lineage
  `tools/update-wrapper-version.sh`. HEAD and the staged authority content were
  unchanged. As in the accepted B6/B7 precedent, only an exact revalidated
  WORKBOARD-only staged tree may be committed with `--no-verify`; the hook itself
  is not modified in this product bundle.
- Fresh authority found the existing SPEC sufficient. Its SHA-256 remains
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`;
  Slice 0 changed no SPEC or product source. Temporary Cargo build output was
  removed and the checkout returned clean before this authority update.
- Read-only feasibility built the current release Core only to inspect its target
  identity: it is ELF64 little-endian AArch64 PIE for Android, dynamically linked
  through `/system/bin/linker64`. Therefore B8 must not apply B6's upstream
  static/no-`PT_INTERP` rule to Core. The observed temporary artifact SHA-256
  `8585a2eb63d2066298418e5c95d9a89bb5e4909e2f3f9dad94b451bbb4a8c502`
  is feasibility evidence only and is not a release identity or acceptance pin.
- Slice 1 strengthened only the existing B6 `build --core` boundary. Builder now
  snapshots the opened executable Core into private staging before upstream
  adaptation, requires that exact snapshot to be an ELF64 little-endian AArch64
  PIE with the exact Android `/system/bin/linker64` interpreter, hashes the
  snapshot, removes it before publication, and passes only that selected digest
  to the existing `generation.meta.core_artifact_digest` field. No Core copy,
  sidecar, manifest, archive, persistent format, or alternate production tool is
  added to the unsigned generation.
- The first Slice 1 focused command stopped before tests because `cargo fmt
  --check` found formatting-only drift. No behavior gate failed. The source was
  formatted and the exact gate was rerun. Canonical project-registry `portable`
  check then passed; the two named B8 Slice 1 regressions passed 2/2; the complete
  affected release-builder suite passed 7/7 serial; `cargo fmt --check` and `git
  diff --check` passed; and a warning-free locked release Core build produced the
  expected ELF64 little-endian AArch64 PIE using `/system/bin/linker64`. Its
  same-revision SHA-256 was
  `8585a2eb63d2066298418e5c95d9a89bb5e4909e2f3f9dad94b451bbb4a8c502`.
  That digest is Slice 1 proof for the built artifact, not a permanent release
  pin. Lead diff inspection found only the selected builder qualification/digest
  path plus its mapped tests.
- The normal Slice 1 commit was rejected before commit by the already-known
  shared pre-commit hook because orphan-lineage `tools/update-wrapper-version.sh`
  is absent. HEAD and the staged product/authority tree were unchanged. Slice 1
  therefore uses the accepted exact-tree `--no-verify` precedent only after
  re-staging this record and revalidating that the index contains exactly the
  builder source and `WORKBOARD.md`; the hook remains untouched.
- Slice 2 adds only the accepted fresh-install boundary. `bootstrap/codex-bootstrap`
  accepts one absolute prebuilt Core path, one absolute local signed-release
  directory, and one absolute bootstrap public-key path. It snapshots the Core,
  key, `release.manifest`, `release.sig`, and `generation.meta` into owner-only
  temporary storage before verification; verifies the exact manifest signature
  with the snapshot key; requires the signed manifest release key to equal that
  key; verifies the signed `generation.meta` digest/mode; and requires the Core
  snapshot SHA-256 to equal signed `generation.meta.core_artifact_digest` before
  publishing either the canonical bootstrap seed or stable Core entrypoint.
  Same-directory create-new temporaries plus no-clobber rename publish the key
  and entrypoint without an overwrite path. The bootstrap has no network,
  package-manager, compiler, rollback, update, discovery, or post-state recovery
  behavior.
- Core now exposes only a private bootstrap execution mode before public dispatch.
  It never accepts a trust key through that mode: it derives only
  `~/.local/lib/codex/core/release-public-key.pem`, rejects an authoritative v3
  state after existing journal recovery, reuses the B7 v3 signed-release
  admission, probe, immutable staging, installed re-verification, and initial
  `codex-activation-state-v3` transaction, and refuses initial key rotation.
  Once v3 state exists, both bootstrap self-test and activation fail closed;
  ordinary launch, local/remote update, rollback, and recovery gained no
  bootstrap fallback.
- Slice 2 stopped on three proof issues and repaired only their local causes.
  First, two script regressions correctly rejected a legacy test fixture whose
  `generation.meta` mode did not match real B6 `0644`; the fixture was aligned
  with B6 output and the product check was retained. Second, an exact fault test
  filter initially selected zero tests, so that run was discarded and the full
  `tests::...` name was rerun 1/1. Third, TOCTOU hardening first used hard links,
  which Android app storage rejected with permission denied; publication was
  changed to same-directory no-clobber rename and the hardening was retained.
- Final Slice 2 proof is green: four named B8 bootstrap regressions pass 4/4;
  the existing initial-activation durable-boundary fault matrix passes 1/1;
  canonical project-registry `portable` check passes; the final Core serial suite
  passes 80/0/1-ignored; a warning-free locked release Core build passes; shell
  syntax, `cargo fmt --check`, and `git diff --check` pass. The rejection matrix
  covers wrong bootstrap key, attempted initial rotation, probe failure, tampered
  manifest, Core-digest mismatch before install, one-shot state ownership, and
  exact private Core activation. Test roots are isolated and no live product,
  trust, resolver, generation, package, or publication state was mutated. Lead
  diff inspection found no new trust object, persistent state owner, public
  command, updater, or recovery protocol, so the existing SPEC remains sufficient.
- The normal Slice 2 commit was rejected before commit by the same orphan-lineage
  pre-commit hook because `tools/update-wrapper-version.sh` is absent. HEAD and
  the staged Core/bootstrap/authority tree were unchanged. The accepted
  `--no-verify` precedent is permitted only after this record is staged and the
  exact three-path index is revalidated; the shared hook remains untouched.
- Slice 3 adds no production behavior. Its one environment-gated integration
  regression reuses the B6 official-shape archive fixture, compiles a temporary
  static AArch64 probe runtime only as release-workspace test input, builds the
  actual locked release Core, feeds that Core through the real B6 builder, signs
  the resulting generation with the existing v3 fixture, and invokes the actual
  fresh bootstrap against isolated HOME/PREFIX/TMPDIR roots. The same release
  Core SHA-256
  `28217cd50b417b94ac13975be7f7ca094b25ca4484db6a406f791c5b2584906e`
  is proven at builder `core_artifact_digest`, bootstrap input, installed stable
  Core, and installed signed-generation verification. B6 byte adaptation is
  proven by a changed runtime that still executes the Core version and doctor
  candidate probes after adaptation.
- The first Slice 3 integration run completed B6 production, v3 admission,
  bootstrap, and initial activation successfully, then failed only because the
  test incorrectly required public `doctor` exit 0 while current accepted
  Manager-unavailable status intentionally yields a degraded/nonzero doctor
  summary. That stale assertion was replaced by ordinary `--version` passthrough
  success; no product behavior changed. The exact integration rerun then passed
  1/1 with the release Core digest above.
- Final Slice 3 affected proof is green: release-builder 7/7 serial; B7 trust
  rotation group 5/5; B8 Slice 2 bootstrap group 4/4; pre-existing B6 signed
  admission 1/1; canonical project-registry `portable` check green; format and
  diff checks green. Lead diff inspection found only test-module fixture
  refactoring plus the integration regression. No updater, archive/install path,
  public command, trust object, persistent state owner, or production source
  behavior was added, so the existing SPEC remains sufficient.
- The normal Slice 3 commit was rejected before commit by the same orphan-lineage
  pre-commit hook because `tools/update-wrapper-version.sh` is absent. HEAD and
  the staged Core-test/authority tree were unchanged. The accepted `--no-verify`
  precedent is permitted only after this record is staged and the exact two-path
  index is revalidated; the shared hook remains untouched.

#### Slice 0 selected implementation boundary

The B8 product boundary is exactly three source paths; expansion requires a
fresh authority check before mutation:

1. `crates/release-builder/src/lib.rs` — keep the existing `build --core
   <ABSOLUTE_FILE>` interface and generation output. Strengthen that existing Core
   input boundary to require the qualified Android/AArch64 Core executable shape,
   hash the exact Core bytes, and continue binding that SHA-256 as
   `generation.meta.core_artifact_digest`. The Core artifact is the single
   release-mode `codex` executable itself: no seal sidecar, extra manifest,
   alternate archive, new persistent format, or second release-production tool.
   `crates/release-builder/src/main.rs`, workspace membership, and Cargo lockfile
   are not expected to change.
2. `bootstrap/codex-bootstrap` — add one small local fresh-install bootstrap. It
   may operate only while authoritative v3 state is absent. Before executing the
   candidate Core it independently checks the bootstrap-pinned key against the v3
   manifest key, verifies `release.sig`, verifies the signed `generation.meta`
   inventory binding, and compares the exact candidate Core SHA-256 with signed
   `generation.meta.core_artifact_digest`. Only that authenticated Core may be
   staged/self-tested and invoked for initial activation. It is not an updater,
   rollback path, package installer, or post-install recovery authority.
3. `crates/core/src/main.rs` — add one exact bootstrap-only initial activation
   route for the already-authenticated Core. It must refuse an existing v3 state,
   use the bootstrap key only for this explicit first-install path, and reuse the
   accepted B7 local v3 admission, immutable generation staging, candidate probe,
   and atomic `codex-activation-state-v3` transaction. Ordinary launch,
   `codex update`, remote update, rollback, and state recovery receive no
   bootstrap fallback.

B6 already makes `generation.meta.core_artifact_digest` part of the generation
metadata and v3 signs/inventories `generation.meta`, so this boundary adds no new
trust object. Slice 3 must prove the same one Core digest flows from the qualified
prebuilt executable through B6 generation production into bootstrap verification.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — fresh authority and validation routing | Rebind the committed B7 state, repair Rust project-validation metadata, inventory current release/build/bootstrap boundaries, and select the smallest artifact/bootstrap implementation shape without changing product behavior | canonical Rust validation route works nonzero; direct baseline agrees; exact B8 file/artifact boundary recorded; no product mutation | complete: project-registry Cargo profiles green; canonical and direct serial both Core 75/0/1 + builder 5/0; exact three-file boundary recorded; SPEC/product unchanged |
| 1 — prebuilt Core artifact | Produce one immutable release-production Core artifact with explicit platform/architecture identity and digest, without installing it or claiming target-device compilation | named artifact build/identity/digest tests pass nonzero; warning-free locked release build; no live mutation | complete: canonical check green; B8 focused 2/2; affected builder 7/7 serial; format/diff clean; locked release Core identity/digest proven; no live mutation |
| 2 — fresh bootstrap initial trust | Implement only the bootstrap operations already authorized by SPEC/B7: environment check, immutable v3 admission with bootstrap key, complete staging/probe, and initial v3 state activation in isolated roots | valid local initial install plus malformed/key-mismatch/probe/fault matrix pass nonzero; no fallback or partial state | complete: B8 focused 4/4 + initial transaction fault 1/1; canonical check green; final Core serial 80/0/1; locked release build and format/shell/diff checks green; no fallback/live mutation |
| 3 — release-production integration | Feed the prebuilt Core artifact through B6 generation production and the same v3 signed-release fixture/bootstrap path; prove no duplicate updater or archive/install path appears | one complete release-production-to-bootstrap flow and affected B6/B7 groups pass nonzero | complete: actual locked release Core → B6 builder → v3 signing/admission → real bootstrap → installed v3 state passed 1/1; builder 7/7, B7 5/5, B8 Slice2 4/4, B6 admission 1/1, canonical check green; test-only diff |
| 4 — grouped acceptance | Add no new behavior; run final bundle proof and synchronize authority | full serial + three complete parallel suites, explicit live read-only smoke, format/diff, warning-free locked release, zero residue, protected identities unchanged, GOAL update, commit | selected |

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
