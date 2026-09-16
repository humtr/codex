# Release Automation and Local-Derived Update Plan

Status: selected implementation plan; **RALD-1 accepted on 2026-09-15; RALD-2 accepted on 2026-09-16; RALD-3 accepted on 2026-09-16 after default-branch install and GitHub-hosted manual dry-run; RALD-4..RALD-7 remain pending**.

Baseline: `rewrite/rust-core` at
`21bb1cd78d4a6e6ef8e124b7f07230206c5aa5ea` (`termux: guard rollback holds and automate stable intake`).

Authority remains, in order, `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`. This file
is a drift-control plan owned by the selected `WORKBOARD.md` bundle. It does not
override `SPEC.md`, does not retroactively alter accepted evidence in `GOAL.md`,
and must be updated together with `WORKBOARD.md` if the selected implementation
changes.

## 1. Objective

Complete the release/update architecture by separating three concerns that are
currently partially coupled:

1. **official release production**: periodically discover a genuinely newer
   official OpenAI Codex stable release, construct the Termux-adapted generation,
   qualify it, sign it with the repository release authority, stage it, prove it
   through public readback and disposable update/runtime smoke, and only then
   promote the signed stable index;
2. **official release consumption**: preserve the accepted `codex update`,
   `codex update --rollback`, rollback hold/guard, and `codex update --force`
   behavior as the device-side signed-stable consumer/recovery plane; and
3. **local-derived execution**: let any supported Termux user explicitly build
   an official-upstream-derived temporary generation without possessing the
   project release private key, without changing public release authority, and
   without making that generation publishable as official stable.

The result must not require an always-on maintainer Termux device. Official
release production runs on GitHub-hosted Actions. The release private key is
provided only to the signing job through the repository Actions secret named
`CODEX_RELEASE_SIGNING_KEY`.

## 2. Frozen decisions

The following are selected decisions, not implementation suggestions. Changing
one requires an explicit user decision and a corresponding plan/authority-doc
change before code is changed.

### 2.1 `codex update` remains the consumer command

- `codex update` continues to prefer the authenticated signed stable channel.
- Existing signature, digest, compatibility, anti-rollback, probe, atomic
  activation, last-known-good generation, rollback hold, legacy rollback-Core
  guard, and failure semantics remain in force.
- `codex update --rollback` remains the sole public rollback selector.
- `codex update --force` keeps its accepted narrow meaning: retry exactly the
  authenticated held public sequence for one invocation and bypass only the
  hold comparison. It is not a local-build or release-production flag.
- Official release creation/publishing is removed from the semantic meaning of
  `codex update`. Release production and release consumption are separate
  planes.

### 2.2 `codex update --build-local` is always local-derived

- Add one explicit Core selector, `codex update --build-local`.
- The result is local-derived **regardless of whether an official private key is
  present on the device**.
- The command never reads `CODEX_RELEASE_SIGNING_KEY`,
  `CODEX_TERMUX_UPDATE_PRIVATE_KEY`, or the maintainer release-key default path.
- It never creates a GitHub Release, dispatches a workflow, writes `main`,
  advances the public stable index, or calls `gh`.
- A transport-unavailable fallback from plain `codex update`, if retained by
  the final `SPEC.md`, uses the same local-derived construction path. It does
  not silently become an official signing/publishing path on a maintainer
  device.

### 2.3 Local-derived generations do not acquire public authority

- A local-derived generation is created only from the exact official OpenAI
  release metadata/archive path already qualified by the product. No arbitrary
  executable or alternate mirror becomes a local-derived source authority.
- The build occurs in a private staging area and is qualified by the same
  Termux adaptation and candidate probes as an official candidate.
- The implementation uses a fresh device-local ephemeral Ed25519 signing key to
  bind the locally built immutable generation. The ephemeral private key is
  created in private temporary storage, never enters persistent Core state, is
  never uploaded, and is deleted after activation/failure cleanup. Only the
  public verifier needed to authenticate the installed local-derived generation
  may persist in the existing generation verifier slot.
- `activation-state-v3.update_key` remains the official project update authority.
  Local-derived activation must never rotate or replace it.
- Local-derived provenance is integrity-bound and unambiguous. At minimum it
  binds an exact local-derived format marker, the official upstream version and
  archive digest, the authenticated public baseline generation and release
  sequence, and the builder/Core provenance required to reproduce admission.
- Local-derived generation identity/sequence data must not allocate or consume a
  new **public** release sequence. Public anti-rollback comparisons after a
  local-derived activation use the integrity-bound authenticated public baseline,
  not an invented local public sequence.
- No generic `--local-derived <directory>` import surface is added. The special
  admission is reachable only from the in-process official-source build path, so
  an arbitrary self-signed directory cannot claim local-derived authority.

### 2.4 Local-derived rollback/update interaction is explicit

- A local-derived forward activation still retains one complete previous
  generation through the existing atomic generation transaction.
- Rolling back **from** a local-derived generation to the retained official
  generation must not create an official-channel `update-hold` for the
  local-derived generation; the v1 hold remains a public signed-sequence
  suppression mechanism.
- Rolling back from an official signed generation remains unchanged, including
  the accepted hold/guard behavior, even if the retained previous generation is
  local-derived.
- `codex update --force` never targets a local-derived generation.
- A later authenticated official stable remains governed by the official
  `update_key`; successful official activation replaces local-derived current
  state through the normal atomic transaction without trusting the local
  ephemeral signer as forward-update authority.

### 2.5 Official release production moves to GitHub-hosted Actions

- Add a scheduled workflow with an initial cadence of every six hours plus
  `workflow_dispatch` for controlled retries/dry runs.
- GitHub schedule execution occurs from the default branch, so the final
  activation step must install the fixed scheduler/publication workflow on
  `main`. Development and acceptance remain on `rewrite/rust-core` first.
- The scheduled workflow must use an **accepted, pinned** `rewrite/rust-core`
  source commit for release tooling. It must not treat the moving implementation
  branch head as implicit release authority. Updating that source pin is a
  separate accepted repository change.
- GitHub-hosted compute performs upstream discovery, deterministic build/adaption,
  qualification, smoke tests, signing, staging, readback, and promotion. No
  self-hosted Termux runner is required.
- Cross-build details are not assumed. The implementation must first prove the
  repository-native Android/AArch64 build on the selected GitHub-hosted image;
  Android NDK/cross-toolchain use is acceptable only after the resulting Core,
  Manager, helpers, and adapted upstream runtime pass the same release probes and
  a Termux-compatible runtime smoke.

### 2.6 Repository Actions secret is the official signing input

- The workflow references only `secrets.CODEX_RELEASE_SIGNING_KEY`.
- Documentation and tests never contain the secret value.
- The job writes the secret, if present, only to a private file under
  `$RUNNER_TEMP` with `0600` permissions and with shell tracing disabled.
- Before signing, OpenSSL derives the Ed25519 public key and compares it exactly
  with the pinned/authoritative public update key expected by the accepted stable
  channel. A missing, malformed, non-Ed25519, or mismatched secret fails before
  any release/promotion mutation.
- The private key file is deleted by an unconditional cleanup trap. It is never
  uploaded as an artifact, included in cache state, printed, hashed into logs as
  a secret identifier, or copied into a generation/repository/release.
- `GITHUB_TOKEN`/GitHub permissions authorize repository operations only; they do
  not substitute for the Ed25519 release authority.

### 2.7 Publish success is not runtime success

A candidate is not stable merely because build, GitHub Release upload, or Pages
deployment succeeded. Promotion requires all of the following, in order:

1. exact official OpenAI stable discovery and digest binding;
2. reproducible/repository-native Termux generation construction;
3. static release qualification and focused tests;
4. Termux-compatible runtime smoke of the built candidate;
5. official Ed25519 signing after secret/public-key match;
6. immutable GitHub Release staging;
7. Pages deployment that preserves the **currently authoritative successful
   stable generation** while exposing the candidate;
8. complete public HTTPS byte/digest/signature readback;
9. a disposable no-argument `codex update` from the currently authoritative
   public stable to the candidate, followed by launch/version, Core doctor, and
   second-update/no-op smoke sufficient to prove the delivered runtime works;
10. atomic signed-index promotion through one non-forced compare-and-swap from
    the exact verified `main` parent.

Any pre-promotion failure leaves the old signed stable index authoritative and
its generation publicly reachable. An ambiguous final ref result is never
assumed committed or uncommitted; it is re-read and classified exactly.

### 2.8 Last-known-good connectivity is a hard publication invariant

- During candidate deployment the currently authoritative stable generation is
  kept byte-for-byte reachable at its already signed `release_base`. This is the
  public last-known-good (LKG) slot.
- The candidate is a second slot and cannot displace LKG merely by uploading or
  deploying successfully.
- Only after the disposable public update/runtime smoke passes and the signed
  index promotion is known committed does the candidate become the new LKG.
- A later candidate deployment may retire the generation older than the current
  LKG if the Pages whole-site size bound requires it. Correctness depends on
  **current LKG + candidate**, not on an unbounded public history.
- Device-side `current`/`previous` remains the runtime rollback mechanism. The
  public LKG rule protects network/release continuity and does not create a
  public fallback ladder or change `codex update --rollback` semantics.
- Immutable GitHub Release staging artifacts may be retained longer for forensic
  recovery, but they are not an alternate activation authority when their URL
  differs from the signed `release_base`.

## 3. Baseline: what is already complete in `humtr/codex`

At the selected baseline the project already has a substantially stronger
consumer/update trust model than the two external reference projects:

- signed stable `update-index-v1` plus signed immutable generation admission;
- complete-generation staging, probes, atomic activation, and one retained
  previous generation;
- Core entrypoint coordination across updates;
- `codex update --rollback`, rollback hold, bounded `--force`, and the legacy
  rollback-Core guard;
- official OpenAI stable metadata/archive qualification and a prebuilt local
  adaptation path;
- GitHub Release staging plus a Pages reconstruction/deployment workflow that
  preserves current stable while deploying a candidate;
- complete public readback and disposable public-update smoke before stable
  promotion; and
- fail-closed non-forced atomic replacement of signed index + signature.

The remaining architectural defects selected here are:

1. official release production is currently triggered as a maintainer-only side
   effect of `codex update` on a Termux device;
2. ordinary-user local production still depends on the official signing key, so
   it is not actually a general local-build capability;
3. the repository has no scheduled upstream watcher/producer on GitHub-hosted
   Actions; and
4. release signing is deliberately excluded from GitHub-hosted Actions in the
   accepted baseline, whereas the newly selected policy permits it through the
   repository secret with explicit key matching and fail-closed handling.

## 4. External reference snapshot

These references are implementation evidence only. They do not override this
repository's authority documents and no source is copied from them.

### 4.1 `DioNanos/codex-termux` / `@mmmbuto/codex-cli-termux` distribution pattern

Evidence snapshot: `DioNanos/codex-termux` `main` at
`235ec42906fbeb1a5c17f0a9e596e1f86fd5a8df` (observed 2026-09-15).

Relevant workflow: `.github/workflows/termux-npm-build-publish.yml`.

Observed pattern:

- trigger: pushes to `main`/`lts` plus manual dispatch, not a periodic upstream
  release watcher;
- builder: GitHub-hosted Ubuntu (`ubuntu-24.04`);
- target: Android ARM64 cross-build with Android NDK/Rust target;
- output: npm tarball + SHA-256 uploaded as Actions artifacts;
- publisher: intentionally **not** GitHub Actions; npm publication is performed
  locally by the maintainer, and the workflow intentionally holds no npm write
  token.

Useful lesson for this project: a real Termux phone is not required to construct
Android/AArch64 Codex artifacts; a GitHub-hosted cross-build can be a valid
release producer once repository-specific runtime qualification is proven.

What is not adopted: manual/local package-registry publication as the normal
release path, npm registry authority, or a push-trigger-only upstream freshness
model.

### 4.2 `wallentx/antigravity-cli-termux`

Evidence snapshot: `wallentx/antigravity-cli-termux` `dev` at
`3b9bc32ad244d1020c950f2d2a82a33e3a91b3d2` (observed 2026-09-15).

Relevant workflow: `.github/workflows/auto-sync-release.yml` and its
`check-version.sh`/Termux smoke support.

Observed pattern:

- trigger: `schedule` every six hours plus manual dispatch;
- builder: GitHub-hosted `ubuntu-latest`;
- discovery: fetches an official upstream manifest, compares the version against
  the repository release, and exits without building when already current;
- build: creates the Termux standalone artifact on GitHub-hosted compute;
- qualification: uploads intermediate artifacts and runs installer/artifact
  smoke through a Termux-oriented reusable workflow;
- provenance: GitHub build-provenance attestation;
- publisher: automatically creates the GitHub Release after smoke success.

Useful lesson for this project: periodic upstream detection, GitHub-hosted build,
Termux-oriented smoke, and automatic Release publication can be one unattended
pipeline.

What is not adopted: GitHub attestation as a replacement for this project's
Ed25519 `update_key`/`release.sig` trust model, mutable upstream branch merge as
release authority, or publication without this project's signed-index/readback/
atomic-promotion gates.

## 5. Comparison matrix

| Dimension | Current accepted `humtr/codex` | DioNanos/mmmbuto pattern | wallentx pattern | Selected final `humtr/codex` |
| --- | --- | --- | --- | --- |
| Upstream check trigger | maintainer `codex update` | push/manual | 6-hour schedule/manual | 6-hour schedule/manual official producer |
| Release/update separation | partially coupled on maintainer device | build vs local npm publish separated | producer pipeline separate from user runtime | strictly separated producer and consumer planes |
| Build compute | maintainer Termux prebuilt adaptation | GitHub Ubuntu + Android NDK | GitHub Ubuntu (+ Termux-oriented smoke) | GitHub-hosted, repo-qualified Android/AArch64 build/adaptation |
| Official signing | device-local Ed25519 key | package publishing credential kept local | GitHub provenance attestation, no equivalent project release key | `CODEX_RELEASE_SIGNING_KEY`, exact derived-public-key match |
| User `codex update` | signed stable + rollback/hold/force; maintainer may also publish | npm/package-manager model | release download/install model | signed stable consumer only; accepted rollback/hold/force preserved |
| User local build | effectively requires official key in fallback | not equivalent | build script can run in supported local/cross environment | `--build-local` always local-derived; official key ignored |
| Local build can publish official | yes on maintainer gate | maintainer publication is separate | automated GitHub Release | no; official publication exists only in producer workflow/manual release tooling |
| Candidate runtime proof | public update smoke exists | cross-build artifact checks; publication local | installer/artifact Termux smoke | static + Termux smoke + public readback + disposable `codex update`/launch/doctor/no-op |
| Stable promotion | signed index atomic CAS after gates | npm/latest release process | GitHub Release creation | existing signed index atomic CAS retained |
| Public last-known-good continuity | current stable preserved with candidate during Pages replacement | registry/release history | GitHub Releases | hard LKG + candidate invariant until candidate is runtime-proven |
| Device rollback | signed current/previous + hold/guard | package-manager dependent | release reinstall model | unchanged accepted current/previous + hold/guard |

## 6. Target architecture

```text
OFFICIAL PRODUCER (GitHub-hosted)
  schedule/manual
      -> official OpenAI stable metadata + exact archive digest
      -> compare with currently signed public stable
      -> pinned accepted wrapper source/tooling
      -> Android/AArch64 build + Termux adaptation
      -> tests + Termux-compatible runtime smoke
      -> CODEX_RELEASE_SIGNING_KEY preflight/public-key match
      -> official manifest/index signing
      -> immutable GitHub Release staging
      -> Pages deploy: current LKG + candidate
      -> complete HTTPS readback
      -> disposable public codex update + launch/doctor/no-op smoke
      -> atomic signed stable-index promotion

OFFICIAL CONSUMER (every supported device)
  codex update
      -> signed stable index
      -> signed immutable generation
      -> probe
      -> atomic activation
      -> one retained previous generation
      -> rollback / hold / bounded force unchanged

LOCAL-DERIVED (every supported device)
  codex update --build-local
      -> official OpenAI stable metadata/archive only
      -> local adaptation
      -> ephemeral local signing key
      -> local-derived provenance + public baseline binding
      -> probe
      -> atomic local activation while preserving official update_key
      -> never publish
```

## 7. Implementation phases and gates

### Phase RALD-0 — authority and baseline lock

Deliverables:

- this plan and `WORKBOARD.md` selection;
- exact baseline/remote fast-forward verification;
- no runtime/workflow behavior change.

Gate: documentation-only diff, `git diff --check`, clean commit and fast-forward
push to `rewrite/rust-core`.

### Phase RALD-1 — local-derived Core contract

Status: **accepted 2026-09-15**. The required selector, ephemeral local authority,
public-baseline binding, `update_key` preservation, internal-only special
admission, local-derived rollback semantics, transport fallback wiring, and
anti-publish boundary are implemented without changing the frozen RALD design.
Focused RALD-1 E2E passed 4/4; accepted ARH/signed-channel regressions passed
4/4; locked workspace check, clippy `-D warnings`, formatting, `git diff --check`,
and the full locked workspace suite all passed. The full suite result was Core
153 passed / one explicit live smoke ignored, Manager 20/20 plus 11/11
integration, and release-builder 15/15. Public stable and the live installation
were not mutated. The then-live official producer was intentionally left for
RALD-2 and is now detached by the accepted RALD-2 phase.

Primary files expected to change:

- `SPEC.md` (normative local-derived/update interaction);
- `crates/core/src/main.rs` (CLI routing, local-derived construction/admission,
  default transport fallback wiring if retained);
- focused Core tests;
- possibly a small dedicated local-derived module if separation reduces risk.

Required tests:

- `--build-local` is accepted; malformed/combined selectors are usage errors;
- presence of the official private key does not alter local-derived behavior;
- official private-key paths and `gh` are not opened/invoked;
- exact official metadata/archive digest qualification is retained;
- ephemeral private material is bounded/private/cleaned and never persistent;
- activation leaves `update_key` byte-identical;
- installed local-derived generation re-verifies under its stored local public
  verifier after process restart;
- arbitrary self-signed local directory cannot enter the special path;
- public sequence baseline is not incremented/consumed;
- plain signed `codex update` can later supersede local-derived state according
  to the recorded public baseline;
- rollback from local-derived to official does not create a public hold;
- rollback from official and `--force` retain all accepted ARH behavior.

Gate: focused process E2E plus all existing rollback/hold/force regressions.

### Phase RALD-2 — detach official production from Core `codex update`

Status: **accepted 2026-09-16**. Core no longer contains the device-side
`automatic_update` official producer/publisher or ambient official-key/GitHub
wiring. `codex update` remains signed-channel consumption plus the accepted
RALD-1 local-derived transport fallback; maintainer credentials and authenticated
`gh` do not change that behavior. The explicit release-builder
`fetch/build/publish` producer boundary remains intact for later orchestration.
Focused consumer/RALD-1/ARH/channel E2E passed 4/4; explicit builder-to-admission
passed 1/1; formatting, `git diff --check`, locked workspace check, workspace
clippy `-D warnings`, and producer-symbol absence audit passed; full locked
workspace tests passed with Core 145 passed / one explicit live smoke ignored,
Manager 20/20 plus 11/11 integration, and release-builder 15/15. Public stable,
the live installation, GitHub Release/Pages stable state, and Actions secrets
were not mutated. RALD-3 is next.

Primary files expected to change:

- `crates/core/src/automatic_update.rs` and related `main.rs` publisher wiring;
- `crates/release-builder` APIs/CLI as needed for a non-installed producer;
- tests proving `codex update` never signs/publishes merely because maintainer
  credentials happen to exist.

The accepted consumer-side official discovery/admission helpers may be reused,
but release production must become explicit release tooling rather than an
ambient runtime side effect.

Gate: ordinary, held, force, rollback, and transport-fallback update tests show
no GitHub mutation path from Core.

### Phase RALD-3 — GitHub-hosted scheduled producer

Status: **accepted 2026-09-16**. The producer source is pinned to exact commit
`28e65b32c8719cf913e62080d4674b54dbcc1a01`. Release-builder supports the
explicit pre-sign `--defer-manager-probe` hosted cross-build boundary, with
Android/AArch64 ELF proof, a bounded deferred marker, and an unconditional
`publish` rejection before signing while that marker exists. The repository-owned
workflow defines the selected six-hour/manual schedule, read-only permissions,
serialized concurrency, authenticated public-stable comparison, exact official
upstream discovery, Android/AArch64 Core/Manager build and unsigned adaptation,
strict inventory/digest checks, short-lived unsigned artifact transfer, and an
ARM64 Android executable smoke that must discharge the Manager probe. No RALD-4
signing-secret access or RALD-5 release/publication/promotion authority is present.

Repository evidence remains green: dependency-free preflight tests 5/5;
workflow contract tests 5/5; deferred Manager focused regressions 3/3; YAML
parse, format, `git diff --check`, locked workspace check, clippy `-D warnings`,
and final full locked workspace tests in `job_ulk_1460dff8e1` with Core 145
passed / one explicit live smoke ignored, Manager 20/20 plus 11/11 integration,
and release-builder 17/17. The fixed workflow/source pin was installed on
default branch `main` in
`b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` without changing
`update-index-v1`. GitHub-hosted manual `workflow_dispatch` run `35090080089`
then completed successfully on `ubuntu-24.04` with `contents: read`, fetched
exact source `28e65b32c8719cf913e62080d4674b54dbcc1a01`, authenticated the
current public stable and its release manifest, and resolved wrapper/upstream
stable as `0.154.0` / `0.154.0`. The resulting `candidate=false` decision
correctly skipped cross-build, candidate adaptation/upload, and Android smoke;
the run produced no artifacts. Public stable/index signature, GitHub Release ID
`388405334`, and Pages publication state remained unchanged; no Actions signing
secret or live installation state was accessed or mutated. RALD-4 has not
started.

Add a repository-owned workflow, provisionally
`.github/workflows/auto-release-termux.yml`, with:

- `schedule: 0 */6 * * *` and `workflow_dispatch`;
- minimal permissions per job;
- concurrency serialization so two release producers cannot race stable
  promotion;
- exact accepted wrapper-source SHA pin;
- official OpenAI stable discovery and stable-version comparison;
- deterministic Android/AArch64 Core/Manager/adaptation build;
- artifact inventory/digest qualification;
- Termux-compatible runtime smoke before signing;
- no release mutation in dry-run mode.

Avoid mutable third-party release-authority dependencies. A reusable external
workflow, if ever used for non-authority smoke, must be pinned to an immutable
commit; repository-owned smoke is preferred.

Gate: **passed** by GitHub-hosted `workflow_dispatch` run `35090080089`; the
installed fixed source pin was reproduced and the run produced no public
mutation.

### Phase RALD-4 — Actions-secret signing

Integrate `CODEX_RELEASE_SIGNING_KEY` only after the unsigned candidate passes
pre-sign qualification.

Gate cases:

- secret absent -> fail closed;
- malformed PEM -> fail closed;
- wrong Ed25519 key -> fail closed;
- correct key -> derived raw public key equals official authority and signing
  succeeds;
- log/artifact/cache inspection proves no private-key bytes/path copies beyond
  the bounded runner temporary;
- candidate signatures verify independently with the public key.

No secret value is needed by repository developers/tests; secret-dependent
positive execution occurs only in GitHub Actions after the repository-side
preflight is accepted.

### Phase RALD-5 — publication, LKG continuity, and promotion

Extend the existing Release/Pages path rather than introducing another release
transport.

Required behavior:

- stage candidate as immutable GitHub Release assets;
- keep the currently authoritative stable generation in the Pages artifact while
  candidate is tested;
- verify exact signed inventory and combined site-size bound;
- deploy candidate without moving stable index;
- read back every signed byte from public HTTPS;
- run disposable public update from current stable to candidate;
- prove candidate launch/version and doctor, and prove a second no-argument
  update is the expected no-op;
- promote `update-index-v1` + `.sig` atomically only after all gates;
- keep old stable authoritative on every pre-promotion failure;
- classify final compare-and-swap outcome by re-reading the ref when needed.

Gate: a deliberately broken candidate reaches staging at most; it cannot advance
stable and current LKG remains publicly usable.

### Phase RALD-6 — fresh-install and update delivery E2E

Before calling the architecture complete, prove the actual user acquisition
surface, not only an already-installed updater:

- identify the intended public one-line/bash installation entrypoint;
- if the repository still lacks a network delivery frontend, add the minimal
  bootstrap downloader needed to retrieve the authenticated Core, signed stable
  generation, and bootstrap public key without creating a second trust/update
  implementation;
- disposable fresh install must land on the current official signed stable;
- subsequent `codex update` must be an exact-current no-op;
- after a scheduled newer stable is staged/promoted in fixtures, the same
  installed client must update through the normal consumer path.

This phase must not weaken the existing audited local `install.sh`/bootstrap
boundary. Network acquisition is only a delivery frontend to that boundary.

### Phase RALD-7 — full acceptance and activation

Repository gate:

- release-builder full tests;
- Core full tests including local-derived/process E2E;
- Manager integration tests;
- `cargo check --workspace --locked`;
- `cargo test --workspace --locked`;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`;
- `cargo fmt --all -- --check`;
- workflow syntax/action validation;
- `git diff --check`;
- staged added-line credential/private-key scan;
- final protected-state and public-stable audit.

Activation order after source acceptance:

1. push clean fast-forward implementation to `rewrite/rust-core`;
2. mirror/install the fixed scheduled workflow and its accepted source pin on
   `main` without changing `update-index-v1`;
3. run a manual dry-run/preflight on GitHub-hosted Actions;
4. run one real candidate cycle only after all preflight evidence is green;
5. allow stable promotion only through the same automatic gates that future
   scheduled runs use;
6. record accepted evidence in `GOAL.md` and close the bundle in `WORKBOARD.md`.

## 8. Failure and recovery matrix

| Failure point | Required result |
| --- | --- |
| upstream metadata unavailable/malformed | no build, stable unchanged |
| archive digest mismatch | fail before adaptation/signing |
| cross-build/qualification failure | no signing, no Release |
| secret absent/wrong | no signing, no Release/public mutation |
| candidate runtime smoke fails | no signing/publication, or no promotion if failure occurs after signing |
| GitHub Release staging fails | stable unchanged |
| Pages deploy fails | stable unchanged; current LKG still reachable |
| public readback differs | stable unchanged |
| disposable `codex update` fails | stable unchanged |
| candidate launches/doctor fails | stable unchanged |
| index promotion CAS loses race | do not force; re-resolve and classify |
| promotion result indeterminate | do not guess; re-read exact ref/index state |
| post-promotion unrelated cleanup/local action fails | public commit stays authoritative; next consumer update re-resolves it |
| local-derived build/probe fails | active device generation/update authority unchanged |
| local-derived rollback | atomic previous swap; no local-derived public hold |

## 9. Explicit non-goals

- no top-level `codex rollback`;
- no repurposing of `--force`;
- no official private key distribution to users;
- no official signing from `--build-local` even on the maintainer's device;
- no untrusted arbitrary local self-signed generation import;
- no package-manager/upstream self-updater authority;
- no mutable mirror or alternate upstream source authority;
- no self-hosted phone requirement for scheduled release production;
- no assumption that upload/deploy success proves runtime correctness;
- no unbounded generation/key history or automatic runtime fallback ladder;
- no live resolver/auth/profile/session mutation for release qualification;
- no force push to implementation or publication refs.

## 10. Drift-control / resume protocol

Every implementation/resume session for this bundle must:

1. verify `origin/rewrite/rust-core` and the selected base/head;
2. read `SPEC.md`, then `GOAL.md`, then `WORKBOARD.md`, then this file;
3. restate which RALD phase is active and which frozen decisions apply;
4. inspect current public stable read-only before any publication work;
5. use disposable/temp roots for tests and never the installed live runtime unless
   a later task has explicit live authorization;
6. update this plan first if a frozen decision changes;
7. update `SPEC.md` before implementing changed normative behavior;
8. record accepted evidence in `GOAL.md` only after the corresponding gate is
   actually proven.

External reference observations must be treated as dated snapshots, not ongoing
authority. Future changes in those repositories do not silently alter this plan.
