# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M2-B9 launch/update overlap and
  injected-failure proof is accepted by the paired `GOAL.md` authority update at
  product tip `377fed80710e131ef6558118afcb45031818b302`; M2-B10 is selected
- Remote `origin/rewrite/rust-core` remains
  `253156c37a2bd22af8faae0bce03587999ffd136`; no B7-B9 commit is published
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- The inherited pre-commit hook still references absent
  `tools/update-wrapper-version.sh`. Do not modify or bypass it speculatively;
  when it alone rejects an already validated exact staged tree, record the
  failure, revalidate that tree, and only then use the established `--no-verify`
  closure
- The normal B9 acceptance authority commit was rejected before commit by that
  exact inherited hook failure. HEAD remained `377fed80710e131ef6558118afcb45031818b302`
  and the staged GOAL/WORKBOARD tree remained
  `fad5d112114d7fe74d909690d8f5a656f0ce092a`. The authority closure may use the
  established `--no-verify` precedent only after this record is staged and the
  exact two-path index, absence of unstaged/untracked paths, and absence of
  `target/` are revalidated; the hook remains untouched
- Execution discipline: follow `AGENTS.md` outcome-first closure rules; close one
  vertical proof slice before beginning another independent contract, stop on
  every red/nonzero-proof failure, and reserve grouped acceptance for the stable
  bundle

## Product-speed policy

- B10 is qualification-first. The accepted implementation already has exactly
  the offline-capable product paths required by SPEC: fresh bootstrap for the
  first v3 state, `codex update --local` for later signed local artifacts, the
  existing v3 transaction recovery, and explicit `--rollback`.
- Prove those paths with release-produced artifacts before adding production
  behavior. A failing proof may justify a minimal repair inside the existing
  contract; it does not justify a second installer/updater or a new recovery
  state owner.
- Offline means the selected flow must need no remote acquisition. Tests must
  make network acquisition observably unavailable and fail if the product tries
  to invoke curl or another network path. The already-present local OpenSSL
  dependency remains permitted exactly as SPEC states.
- Offline recovery does not mean reconstructing authoritative v3 state from the
  bootstrap key after initialization. Corrupt or absent authoritative trust state
  still fails closed. Recovery here is the existing activation-journal recovery
  plus explicit last-known-good rollback using already installed/signed local
  generations.
- Do not add package-manager use, network fallback, release discovery, generation
  scan, bootstrap-key fallback, alternate trust source, second state machine, or
  fallback chain merely to satisfy qualification.

## Mandatory bundle execution method

- Fresh-bind branch, HEAD, dirty state, authority files, remote branch, and
  protected live identities before product mutation and after every local commit.
- Project registry revision 3 Rust profiles are canonical: `portable` is locked
  workspace check, `portable-install` is locked workspace build, and
  `portable-test` is the locked serial workspace suite. The separate tmcp
  `project.validation.describe` package.json limitation is not product evidence.
- All B10 install/update/recovery proof uses isolated HOME/PREFIX/TMPDIR roots.
  Never bootstrap, activate, update, roll back, or recover the installed live
  product during development or acceptance.
- Prefer test-only fixtures and fault injection around the existing B8 bootstrap,
  B4/B7 local update/rollback, and B1 state transaction boundaries. Test
  instrumentation must not create a public command, persistent format, trust
  source, or release path.
- Use release-produced local artifacts, not ad-hoc incomplete generation
  directories, for end-to-end qualification claims. Bind source/artifact hashes
  in the acceptance evidence.
- Update `SPEC.md` first only if a concrete failing proof requires a normative
  behavior/security change. Any new security-property change, trust object, or
  persistent state owner requires explicit user approval before that mutation.
- Each slice closes with a named nonzero focused proof, relevant canonical
  check/build, zero residue, and Lead diff inspection. A red gate freezes new
  behavior until its local cause is repaired and rerun.
- After all B10 proof slices are green, run full serial, three complete parallel
  suites, explicit live read-only smoke, warning-free locked release, format /
  bootstrap syntax / diff checks, zero residue, and protected identity comparison.
  Then reduce evidence into `GOAL.md`, select the isolated fresh-Termux /
  upgrade-from-legacy qualification bundle, and commit. Do not push or cut over
  without separate authority.

## Selected next action

### M2-B10 — offline local-artifact install/recovery qualification

#### outcome

Prove one complete no-network delivery and recovery story using the already
accepted product paths and actual release-produced local artifacts: fresh offline
bootstrap into an isolated root, later signed `codex update --local`, ordinary
launch from the activated generation, recoverable v3 transaction cleanup, and
explicit last-known-good rollback. No step may require remote acquisition or
silently replace a failed local path with another authority source.

#### accepted input

- Accepted B9 product tip:
  `377fed80710e131ef6558118afcb45031818b302`.
- Accepted SPEC SHA-256:
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- B9 Core source SHA-256:
  `130f1097f9d31bdedf7a5212daf9dc3be6b5027e2481863708135411d00cb317`.
- Existing B8 release production qualifies the prebuilt Core and produces the
  signed-generation inputs used by bootstrap; B8 fresh bootstrap owns only first
  v3-state establishment; B4/B7 own signed local update/rollback; B1 owns the
  sole durable activation/recovery transaction.
- SPEC requires offline local-artifact installation and recovery before release,
  while explicitly requiring post-initialization authoritative state loss or
  corruption to fail closed rather than re-authorize the bootstrap key.
- B9 grouped acceptance passed canonical serial Core 85/0/1-ignored plus builder
  7/0, three complete parallel suites at the same counts, explicit live read-only
  smoke 1/1, warning-free locked release, format/syntax/diff checks, zero B9
  residue, and exact protected launcher/resolver identity preservation.
- Slice 0 fresh-bound clean authority HEAD `ca2331d...` and unchanged remote
  `253156c...`; canonical project-registry `portable` locked workspace check was
  green before and after the test-only fixture change. The exact product
  entrypoints remain B8 fresh bootstrap, B4/B7 public local update/rollback, and
  the sole B1 v3 recovery state machine. A test-only B10 fixture now reuses the
  real B8 release-production pipeline: release-mode Core -> static probe runtime
  -> official-shape local archive -> `codex-release-builder` -> signed v3 local
  generation. A test-only isolated `PREFIX/bin/curl` sentinel logs any attempted
  acquisition and exits 97; no production path or persistent/public contract was
  changed. The first Slice 0 gate stopped only on rustfmt layout drift, then the
  formatted exact change passed `cargo fmt --check` and `git diff --check`.
  With an actual locked release Core, the named Slice 0 fixture proof passed 1/1
  and left zero matching temp-root residue; the existing B8 release-production
  to real-bootstrap integration also passed 1/1 with the same release Core.
  Lead diff inspection found only test-module helpers/harness additions and no
  production behavior mutation.
- The normal Slice 0 commit was rejected before commit solely by the inherited
  orphan-lineage hook because `tools/update-wrapper-version.sh` is absent. HEAD
  remained `ca2331d...` and the exact staged Core/WORKBOARD tree remained
  `f79b227cb3a78d06ef6e5e6b6f0cd3c1fc74f06a`. Slice 0 may use the established
  `--no-verify` closure only after restaging this record, revalidating exactly
  those two paths, confirming no unstaged/untracked path and no `target/`, and
  leaving the shared hook untouched.
- Slice 1 uses the same actual release-produced g0 fixture and runs the real
  `bootstrap/codex-bootstrap` into isolated HOME/PREFIX/TMPDIR after replacing
  isolated curl with the Slice 0 denial sentinel. The focused proof passed 1/1:
  bootstrap exited zero, installed stable Core SHA matched the authenticated
  release Core, authoritative v3 state contained current g0 with no previous and
  one current/update key, the installed signed generation reverified, ordinary
  `--version` launch succeeded, no bootstrap transaction residue remained, and
  the network-attempt log stayed absent before and after launch. B10 Slice 0-1
  then passed together 2/2 with zero B10 temp-root residue; format/diff checks
  and canonical project-registry `portable` locked workspace check passed.
  Lead diff inspection found exactly one additional test-only end-to-end test;
  no production/bootstrap script behavior, persistent state, public surface,
  trust contract, or SPEC changed.
- The normal Slice 1 commit was rejected before commit solely by the same
  inherited hook because `tools/update-wrapper-version.sh` is absent. HEAD stayed
  `19402be...` and exact staged Core/WORKBOARD tree
  `4fdfd45a426aae486a4a2c1a29546fd66a0a2028` was unchanged. Slice 1 may use
  the established `--no-verify` closure only after restaging this record,
  revalidating those two paths, confirming no unstaged/untracked path and no
  `target/`, and leaving the hook untouched.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — offline authority and fixture boundary | Fresh-bind B9, trace the exact bootstrap/local-update/recovery/rollback entrypoints and establish a network-denial sentinel plus release-produced local fixture without product changes | canonical baseline green; exact no-network observables and one-path invariants recorded; no production mutation | complete: actual release Core + release-builder signed fixture, network-denial sentinel, focused 1/1, B8 integration 1/1, canonical check green, zero focused residue; test-only change only |
| 1 — fresh offline install | Build/qualify an actual release Core and signed initial generation, then run real bootstrap in an isolated root while remote acquisition is unavailable | named end-to-end proof establishes stable Core + v3 state + complete current generation and launches it; network sentinel untouched | complete: focused 1/1, B10 2/2, installed Core/state/signed generation/launch exact, sentinel untouched, canonical check green, zero residue; test-only change only |
| 2 — offline local update and rollback | From the offline-installed generation, activate a newer signed local artifact through public `update --local`, launch it, then use public explicit rollback to the retained signed previous generation | named proof passes current/previous/trust/sequence checks and both launches; no remote acquisition or bootstrap fallback | selected |
| 3 — offline transaction recovery | Starting from recoverable activation-journal states, prove the existing updater/recovery path resolves one complete old/new state offline and explicit rollback remains usable when a valid previous generation exists | named recovery matrix passes without a second recovery mechanism, state reconstruction, network path, or mixed generation | blocked by slice 2 |
| 4 — grouped acceptance | Add no new behavior; run final bundle proof and synchronize authority | full serial + three complete parallel suites, explicit live read-only smoke, warning-free locked release, format/syntax/diff, zero residue, protected identities unchanged, GOAL update, commit | blocked by slice 3 |

#### protected surfaces

- `$PREFIX/bin/codex`, `$PREFIX/etc/resolv.conf`, installed live generations and
  trust state, Manager state, auth/profile/session data, package state, and
  publication refs remain read-only throughout B10 development and acceptance.
- No private signing key may enter Core, repository product data, or persistent
  device state. Test signing material remains ephemeral in isolated roots.

#### stop lines

- no live installation, activation, recovery, rollback, device cutover, or
  publication;
- no remote acquisition in an offline proof and no package-manager invocation;
- no bootstrap-key fallback after v3 initialization, state reconstruction from
  bootstrap material, second installer/updater, second state owner, alternate
  recovery protocol, release discovery, generation scan, or fallback ladder;
- no weakening signed inventory, key rotation, anti-rollback, previous-generation
  verification, or complete-old/new transaction invariants;
- no normative security-contract expansion without fresh SPEC-first explicit
  user approval;
- no legacy source copying or translation;
- no worker, planner, or reviewer unless the user explicitly turns worker mode
  on.
