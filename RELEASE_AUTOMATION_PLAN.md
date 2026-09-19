# Release Automation and Local-Derived Update Plan

Status: RALD-1 through RALD-7 and the post-RALD legacy-lag production remediation are complete. UX-1 update human output, including authenticated progress ordering, is source-accepted at exact product source `07f77b89a177682954d80ae3f797377c4731de64`. **UX1-PROD-15 is fully complete and accepted (2026-09-19)**: production run `35425409155` promoted signed sequence 15 generation `local-hosted-0-155-1-07f77b89a177-ux1-human-output` by the existing non-forced exact-parent CAS to `main=ba36c44f871ef266c4887986535ed87a4d2becc9` after all build/signing/Release/Pages/readback/disposable-runtime gates passed, and the authorized live Termux consumer subsequently activated that exact generation through ordinary signed public `codex update`. Live verification ended at exact `codex-cli 0.155.1`, exact-current no-op output `Codex 0.155.1 is already up to date. ✅`, healthy Termux Core/Manager/runtime, clean non-TTY output, and actual PTY transient cleanup proof. **UPDATE-PROGRESS-RESPONSIVENESS-PROD-16 is fully complete and accepted (2026-09-19)**: accepted source `81131655d98f114b5324bd8ee5866cff0a171941` provides an 80 ms animated TTY spinner and a 30-second signed-control transfer ceiling while retaining the 300-second payload ceiling; production run `35434790060` promoted signed sequence 16 generation `local-hosted-0-155-1-81131655d98f-update-progress-responsiveness` by non-forced exact-parent CAS to `main=52c69f21472fb2d082ef87f0e6583c88b7a8e3db`; the live Termux consumer then activated that exact generation through ordinary signed public `codex update`. New-Core PTY proof observed 374 in-place checking redraws across all ten spinner frames, cleanup before permanent output, exact-current `Codex 0.155.1 is already up to date. ✅`, and clean non-TTY output. **UPDATE-EXACT-CURRENT-FASTPATH-PROD-17 is fully complete and accepted (2026-09-19)**: accepted source `7817b939c81ce15c76d3d0d57157ca5e378a8491` was published as signed sequence 17 generation `local-hosted-0-155-1-7817b939c81c-exact-current-fastpath` by production run `35437042333`; final public-control `main=56ba28e1baee87721767ba34b505cc2bc1303c44` retains that stable pair with the temporary trigger bridge removed; the live Termux consumer then activated sequence 17 through ordinary signed public `codex update`, and the new Core's second exact-current update completed in 1268 ms with exact no-op output and no state delta.

Baseline: `rewrite/rust-core` at
`21bb1cd78d4a6e6ef8e124b7f07230206c5aa5ea` (`termux: guard rollback holds and automate stable intake`).

Authority remains, in order, `SPEC.md` -> `GOAL.md` -> `WORKBOARD.md`. This file
is a drift-control plan owned by the selected `WORKBOARD.md` bundle. It does not
override `SPEC.md`, does not retroactively alter accepted evidence in `GOAL.md`,
and must be updated together with `WORKBOARD.md` if the selected implementation
changes.

UX-1 source acceptance is complete: signed release control plus digest-bound
`generation.meta` authenticate the target version before the permanent update
header and later download/verification progress become visible; PTY tests bind
the exact phase order; non-TTY tests prove stable plain output with no terminal
control bytes.

UX1-PROD-15 is the separately authorized production operation. It must add a
false-by-default manual-only gate that is reachable only from `refs/heads/main`
with explicit publication authorization and exact authenticated baseline
`0.155.1` / sequence 14 /
`local-hosted-0-155-1-566034e1aff4`. The official stable must still be exact
`0.155.1`, ordinary comparison must first yield no candidate, the accepted
product source must be exact `07f77b89a177682954d80ae3f797377c4731de64`,
and the computed next sequence must be 15. Before signing, all non-Core
load-bearing component digests/modes must equal sequence 14, the new Core must
differ and be built from the accepted source, and `generation.meta` may differ only by generation identity plus
`core_artifact_digest`, with both old/new descriptor digests required to
equal the actual old/new Core SHA-256 and every other descriptor field unchanged.
Existing signing, immutable Release, Pages/LKG,
public HTTPS readback, disposable update/version/doctor/no-op, and non-forced
exact-parent CAS gates remain unchanged. Schedules must explicitly reject the
gate. Once sequence 15 is promoted, the exact baseline fence makes the gate
unusable.

Production run `35425409155` satisfied this contract. Its authenticated
baseline was exact sequence 14 generation
`local-hosted-0-155-1-566034e1aff4`; ordinary comparison first yielded no
candidate, then only the manual UX-1 gate admitted sequence 15. The run passed
the non-Core digest/mode identity gate and ordered raw-record descriptor
comparison, Android/AArch64 smoke, production signing and independent
verification, immutable Release staging, LKG-preserving Pages, public HTTPS
byte readback, disposable update/version/doctor/no-op proof, and the existing
`force:false` exact-parent CAS. Post-promotion public stable readback matched
the promoted index/signature bytes and reverified the signature. The exact
sequence-14 fence is therefore now inert by construction.

The live consumer leg is also complete. tmcp job `job_w4h_aabff4f63b`
measured the pre-update installation at exact `codex-cli 0.155.0` with healthy
Termux Core/Manager/runtime. Job `job_w4i_4679e91eab` used only ordinary
public `codex update` and activated exact sequence-15 generation
`local-hosted-0-155-1-07f77b89a177-ux1-human-output`, after which
`codex --version` reported exact 0.155.1. Job `job_w4j_b7c769f26d`
proved the new Core's exact-current non-TTY output, absence of CR/ANSI controls,
and healthy post-update doctor state. Job `job_w4k_377b6ef362` ran the same
no-op with both child stdout and stderr attached to a PTY and observed
`Checking for updates...` as the transient line, carriage-return/erase cleanup,
then the exact permanent current-version line. UX1-PROD-15 requires no further
publication or live-consumer action.

UPDATE-PROGRESS-RESPONSIVENESS is a bounded post-production follow-up and does
not reopen the accepted UX1-PROD-15 operation. Its accepted source
`81131655d98f114b5324bd8ee5866cff0a171941` changes only presentation timing
and transport responsiveness: both stdout and stderr must be TTYs for the
single transient progress line; the spinner is redrawn every 80 ms while a
blocking phase is active and is stopped/joined before permanent output;
non-TTY output remains free of progress control bytes. The exact
pre-authentication text remains `Checking for updates...`. Signed index,
signature, manifest, authority-signature, and authenticated release-metadata
control fetches use the existing 15-second connect timeout with a 30-second
transfer ceiling; large signed payloads retain the existing 300-second
transfer ceiling. Signature/digest/mode/version verification, anti-rollback,
candidate qualification, atomic activation, LKG/rollback, and stable CAS are
unchanged.

Exact-source Termux validation job `job_w9s_d47fda726c` passed spinner-frame,
control/payload timeout, and PTY redraw/cleanup tests (including a requirement
that `Checking for updates...` redraw more than once), the full locked
workspace test suite, workspace check, clippy `-D warnings`, rustfmt, and
`git diff --check`, with a clean worktree before and after. The user then
granted explicit production authorization on 2026-09-19. The selected
publication gate was exact workflow source
`39983f628fe568ec98b8c69bb87b02f9449c0104`: manual-only on `main`, exact
authenticated 0.155.1 / sequence 15 /
`local-hosted-0-155-1-07f77b89a177-ux1-human-output`, ordinary comparison
first `candidate=false`, exact next sequence 16, and exact accepted product
source `81131655d98f114b5324bd8ee5866cff0a171941`. Schedules and all historical
one-shot gates reject it. Before signing, every non-Core load-bearing digest/mode
must equal sequence 15, Core must differ, and the ordered descriptor may change
only unique generation identity plus Core digest. tmcp job
`job_wai_242ba900ee` executed the positive gate and negative wrong-ref,
authorization, generation, sequence, candidate, and source cases, plus the
duplicate-helper descriptor comparator/forbidden-delta proof, all successfully.
That order is now complete. The exact producer workflow was installed as the
sole-file non-forced child
`main=40501cf88d86c1c9281d918141e90389ad2006f6` of the sequence-15
stable parent. Production run `35434790060` then passed the exact baseline and
ordinary-`candidate=false` gate, sequence-15 non-Core byte/mode preservation,
Core-only descriptor delta, Android/AArch64 smoke, accepted-authority signing
and independent verification, immutable Release, LKG-preserving Pages, public
HTTPS every-byte readback, disposable ordinary update/version/doctor/no-op, and
the existing `force:false` CAS. The CAS reported
`promotion_result=committed`, producing
`main=52c69f21472fb2d082ef87f0e6583c88b7a8e3db`, whose only changes from the
producer-install parent are `update-index-v1` and its signature.

The live consumer remained on healthy sequence 15 until the public sequence-16
promotion was independently re-read. `job_wam_258c839b9b` then ran only
ordinary PTY `codex update` and activated sequence 16. Its post-command
harness returned nonzero solely because it expected the initiating sequence-15
Core to already animate the pre-activation checking phase; the same capture had
already matched the expected same-version header and signed-activation success
output. Read-only `job_wan_fe35cc553b` immediately proved the exact sequence-16
generation with healthy Core/runtime/code-mode host/Manager. Finally
`job_wao_d345a6690f` used the new Core for an exact-current PTY proof and
observed 374 in-place checking redraws across all ten spinner frames, verified
erase-before-permanent-output cleanup, and then proved the non-TTY exact-current
line contains no CR/ANSI controls. PROD-16 requires no further publication or
live-consumer action.

UPDATE-EXACT-CURRENT-FASTPATH is the selected post-production performance
correction and is source-accepted at exact implementation source
`7817b939c81ce15c76d3d0d57157ca5e378a8491`. This is not a timeout-policy
change. The previous exact-current path authenticated the signed stable index
and then unnecessarily reacquired the entire already-installed signed
generation, including the roughly 233 MB runtime payload, before returning
`AlreadyCurrent`.

The accepted correction keeps the signed-index authentication boundary intact.
Ordinary no-argument `UpdateHoldPolicy::Enforce` may return exact-current
without remote release acquisition only when the authenticated index generation
exactly equals the active generation, `current_key == update_key`, the
release-base generation identity is exact, rollback hold/guard state validates,
and the installed current generation fully re-verifies under the official
authority. It then creates no generation acquisition tree, fetches no release
control/payload bytes, performs no candidate probe/staging, and mutates no
state. `--force`, local-derived current state, a differing authenticated
generation, malformed hold/guard state, bad index/signature, invalid
release-base binding, or invalid local installed state remain on the existing
full/fail-closed paths. Candidate, rollback, local-derived, explicit
remote/local, signature/digest/mode, anti-rollback, activation, LKG, and CAS
semantics are unchanged.

Focused validation job `job_wcd_60498c3b61` passed the exact-current
signed-channel and rollback-hold/`--force` regressions; its final nonzero shell
status was caused only by Cargo's untracked worktree-local `target/`, which
inspection job `job_wci_78c1c8931f` proved was the sole dirty entry.
Authoritative full job `job_wcj_53ab115dbc` moved Cargo output to job-private
storage and passed cleanly: Core 149 passed / 0 failed / 1 explicit real-Termux
ignore, Manager 20/20, Manager integration 11/11, release-builder 19/19,
workspace check, clippy `-D warnings`, rustfmt, and `git diff --check`.
The curl-log proof shows an ordinary exact-current update makes exactly the
signed index and index-signature requests and no release-control or payload
request; the force regression shows `--force` still fetches and validates
release manifest, descriptor, and runtime data.

Production/live publication was explicitly authorized by the user on
2026-09-19 as UPDATE-EXACT-CURRENT-FASTPATH-PROD-17. The bounded deployment
must add a false-by-default manual-only `main` gate for exact authenticated
baseline upstream `0.155.1`, sequence 16 generation
`local-hosted-0-155-1-81131655d98f-update-progress-responsiveness`, ordinary
comparison `candidate=false`, exact accepted product source
`7817b939c81ce15c76d3d0d57157ca5e378a8491`, and exact next sequence 17.
Schedules and every historical acceptance/remediation one-shot must reject the
new gate. Before signing, every non-Core load-bearing byte/mode must equal
sequence 16 while Core differs; ordered descriptor comparison permits only
generation identity plus Core digest with both digests bound to the corresponding
Core bytes. The existing production signing, native Android/AArch64 smoke,
immutable Release, LKG-preserving Pages, every-byte public HTTPS readback,
disposable ordinary update/version/semantic-doctor/no-op, and non-forced
exact-parent CAS remain mandatory.

After promotion is independently confirmed, the live Termux consumer is
authorized to activate sequence 17 only through ordinary signed public
`codex update`, then prove exact `codex-cli 0.155.1`, healthy Termux
components, and a second exact-current no-op under the accepted fast path.
No manual generation copy, direct launcher/Core overwrite, alternate trust key,
fake credentials, provider-success fixture, or force push is authorized.

The preferred production trigger remains interactive `workflow_dispatch`.
During this authorized operation the tmcp operation endpoint began returning
HTTP 404 before dispatch submission, while GitHub repository writes/readback
remain available. To complete the user's explicit deployment request without
weakening candidate/publication gates, one temporary push-trigger bridge is
authorized. It must require one exact pre-trigger parent, exact
`refs/heads/main`, an exact dedicated trigger commit message, and the same
seq16 generation/version, accepted source, ordinary-`candidate=false`,
target-sequence-17, publication, signing, readback, disposable-runtime, and
`force:false` CAS requirements. Initial push run `35436887423` at
`main=5f7a316cd433d659979c1ec1bf648294002b4307` produced no jobs because a
colon-space inside the unquoted GitHub expression made the workflow YAML
invalid; stable index/signature bytes stayed identical to sequence 16, so no
release mutation occurred. The corrected retry is rebound to exact
`github.event.before = 5f7a316cd433d659979c1ec1bf648294002b4307` and a
colon-free exact trigger message. The bridge is not schedule authority, cannot
admit any historical one-shot, and must be removed from `main` after
successful promotion.

Production retry run `35437042333` completed successfully. It admitted only
the exact seq16/0.155.1 baseline after ordinary comparison returned
`candidate=false`, built accepted source
`7817b939c81ce15c76d3d0d57157ca5e378a8491`, preserved every non-Core
load-bearing byte/mode, changed only Core-bound descriptor fields, passed native
Android/AArch64 smoke, signing/independent verification, immutable Release,
LKG-preserving Pages, public every-byte readback, disposable update/version/
semantic-doctor/no-op, and committed the existing `force:false` CAS.
Promotion commit `f361b4a241b34cee1a7ca5bf4c98914a6b9dd600` published signed
sequence 17 generation
`local-hosted-0-155-1-7817b939c81c-exact-current-fastpath`. The temporary
push bridge was then removed; final
`main=56ba28e1baee87721767ba34b505cc2bc1303c44` differs from the promotion
commit only by restoring the normal non-push workflow, and stable index/signature
bytes remain the promoted seq17 pair.

The production publication and live consumer legs are both complete. After tmcp
transport recovered, read-only preflight `job_wdn_35abc56ffe` confirmed the
live client at exact `codex-cli 0.155.1` on healthy signed sequence 16 while
the device read public stable as sequence 17. Ordinary no-argument signed
public update `job_wdp_a5ba929cd5` then activated sequence 17 in 62014 ms;
that duration is the expected one-time old-Core full acquisition cost, not the
new exact-current path. It printed the accepted same-version correction header
and signed activation success lines and left exact version at 0.155.1.

New-Core proof `job_wdu_343ee51ae9` bound the active generation exactly to
`local-hosted-0-155-1-7817b939c81c-exact-current-fastpath`, with healthy
Core, runtime, code-mode host, Manager, upstream, and summary and doctor exit 0.
Its timed ordinary exact-current update completed in **1268 ms**, emitted exactly
`Codex 0.155.1 is already up to date. ✅`, emitted empty stderr with no
CR/ANSI controls, and left the Core generation/activation/launcher snapshot
byte-identical. Final version remained exact `codex-cli 0.155.1`. No manual
generation install, direct launcher/Core overwrite, alternate trust key, fake
credential/provider fixture, force push, or protected user-state mutation was
used. UPDATE-EXACT-CURRENT-FASTPATH-PROD-17 is closed.


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

RALD-4 acceptance may use one explicitly requested `workflow_dispatch`
**acceptance-only** positive proof instead of waiting indefinitely for the next
upstream stable. This is a test-stimulus exception, not a production candidate
selection rule. It is permitted only on `refs/heads/rewrite/rust-core`, with the
comparison baseline fixed to `0.153.4`, and only while both the independently
authenticated current public stable and the real official OpenAI stable resolve
to exact `0.154.0`. The official metadata/archive digest, Android/AArch64 build,
pre-sign qualification, runtime smoke, current public release-sequence binding,
accepted public-key match, production-authority signing, independent signature
verification, and private-key cleanup remain identical to the ordinary path.
The hosted pre-sign runtime smoke, including this acceptance proof and the
ordinary future producer path, executes the exact built ARM64 candidate natively
on GitHub-hosted `ubuntu-24.04-arm`. It must first prove `uname -m` is `aarch64`.
Manager and Core retain their Android/AArch64 PIE form and are executed through
an exact pinned Android bionic userspace: the workflow fetches
`com.android.runtime-arm64.apex` from AOSP `platform/prebuilts/runtime` commit
`8aeb37cca394ce39c1311744c60960cbd466aa77`, requires SHA-256
`83bf0dce249728dae48149b80d28b48115c54adad95a352120d58a6ac669d1fc`,
and extracts only its ARM64 `linker64` plus bionic `libc`, `libdl`, and `libm`
from the APEX payload with runner-provided read-only filesystem tooling. Manager
and Core must pass their existing exact smoke commands through that pinned
Android linker/userspace; the unchanged official static AArch64 runtime must
pass `--version` directly on the same native ARM64 host. No emulator, KVM,
foreign-architecture translation, package-manager install, x86_64 rebuild, or
candidate repack is accepted. Missing ARM64 runner identity, missing extraction
tooling, APEX byte/hash mismatch, malformed extracted bionic files, or any exact
candidate execution failure fails before signing. Because this deliberately
minimal hosted harness does not recreate Android's generated
`/linkerconfig/ld.config.txt`, the pinned bionic linker emits two deterministic
configuration-warning lines before Manager/Core application stderr. Those exact
pinned-substrate bytes must match byte-for-byte for each dynamic executable; any
additional, missing, or changed stderr fails. Runtime stderr remains empty. This
replaces the earlier API-35 x86_64 `ndk_translation` acceptance substrate after hosted evidence proved
that translation layer SIGSEGVs on the byte-identical already accepted runtime;
it strengthens rather than relaxes the Android/AArch64 execution proof and does
not create publication authority.
The acceptance-only signed index must use a non-routable `.invalid` release base;
its signed output must be verified inside the signing job and deleted rather than
uploaded as an artifact. It must not create or mutate a GitHub Release, Pages,
`main`, the public stable index/signature, or live runtime state. Scheduled runs
and ordinary manual runs continue to compare the actual authenticated public
stable directly with the official upstream stable. This acceptance-only switch is
not an RALD-5 publication input and must not be carried into a public promotion
path as a version-comparison bypass.

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

Gate: **passed 2026-09-17** by acceptance-only `workflow_dispatch` run
`35176798621` at workflow head `d0af224b6738e80feb458e6030b6da517d94a1f5`.
The run authenticated public stable `0.154.0`, built and qualified the real
official `0.154.0` candidate, passed exact native ARM64 Manager/Core/runtime
smoke on `ubuntu-24.04-arm`, removed the deferred Manager marker, revalidated the
qualified candidate and accepted public authority before secret exposure, signed
release sequence `11` with the existing repository authority, and independently
verified both release-manifest and update-index signatures with the accepted
public key. The acceptance index used only the `.invalid` release base; temporary
signing material and the signed acceptance output were removed in-job, and signed
artifact upload was skipped. No Release/Pages/main/public-stable/live-runtime
mutation occurred. RALD-5 was not started.

### Phase RALD-4.5 — activation-safe candidate probe and stable-Core transition

Activation-time candidate integrity must remain independent from user/provider
health. The candidate gate keeps signed descriptor/manifest/index admission,
exact signature/digest/mode checks, qualified Termux execution, the exact
upstream version probe, release-sequence anti-rollback, atomic Core+generation
activation, LKG, rollback, and CAS safety. It does not require full upstream
`codex doctor` exit zero. Public `codex doctor` still runs the real upstream
and Termux diagnostics and preserves health-failure semantics.

For backward compatibility with current public stable sequence 10, the transition
candidate may use signed `upstream_doctor=unsupported` only with the exact R10
bridge creation marker. That signal is consumed solely by the old stable Core's
activation gate; the corrected Core restores real upstream-doctor execution on
the public doctor route. The credential-free disposable proof accepts doctor
exit 0 or the documented health-failure exit only when its bounded JSON report is
valid, the exact activated Termux Core/runtime/code-mode health is healthy, the
upstream section proves the diagnostic actually ran, and the exit code agrees
with the composed report. Missing credentials/provider reachability are not
candidate-integrity failures.

Gate: **proved 2026-09-18** by hosted run `35284270406` at workflow head
`f74045156bff00cae22f3c8d67823096ab1fe13f`, using product source
`37fbbd8033b8cc2d508689ab1d6637b4c4f5d516`. The run authenticated public
stable sequence 10, staged the signed sequence-11 transition candidate, completed
public HTTPS readback, updated from the actual old public stable Core, verified
exact candidate launch/version and second-update no-op, and observed real
`upstream=unhealthy` with `termux_core=healthy` in credential-free doctor JSON.
The transition fence skipped the CAS promotion job; `main` and public stable
remained unchanged at that proof point. The separately required promotion
authorization was subsequently granted and consumed only by the accepted RALD-5
path below. RALD-6/7 are not started by either proof.

### Phase RALD-5 — publication, LKG continuity, and promotion

Extend the existing Release/Pages path rather than introducing another release
transport.

For RALD-5 acceptance only, the user explicitly authorizes one bounded
same-version publication bridge so acceptance does not depend on waiting for a
future upstream release. The bridge is reachable only through manual
`workflow_dispatch` on `refs/heads/rewrite/rust-core`, requires explicit RALD-5
publication authorization, and requires both the independently authenticated
public stable and the exact official OpenAI stable to be `0.154.0`. The ordinary
version comparison must first classify that equality as `candidate=false`; only
then may the separate bridge input admit the same exact official metadata/archive
as a distinct candidate generation using the next authenticated public release
sequence. The exact archive digest binding, Android/AArch64 construction and
smoke, production-authority signing, immutable Release assets, LKG-preserving
Pages deployment, HTTPS readback, disposable update/runtime/no-op proof, and
non-forced exact-parent CAS remain unchanged. Scheduled runs and ordinary manual
runs continue to require a genuinely newer official upstream version. The RALD-4
acceptance-only comparison switch remains non-publishable and cannot substitute
for this bridge.

A candidate GitHub Release tag is a locator, not candidate byte authority. When
staging creates that locator, it must bind the exact verified pre-promotion
`main` parent; an existing locator must match that same parent or staging fails.
The signed release manifest/index, accepted Ed25519 authority, exact inventory,
digests, and public byte readback remain the candidate authority. Tag creation or
Release upload alone never authorizes promotion.

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

Gate: **accepted 2026-09-18**. Bounded workflow commit
`d1b53576f9b173dcf786b21271b556712d694bbf` preserves the transition-stage
no-promotion default and requires a separate false-by-default
`rald45_transition_promote` authorization before a transition generation can
reach the existing CAS job. Repaired negative run `35285462288` built, natively
smoked, production-signed, staged, and Pages-deployed a same-version candidate,
then deliberately tampered the fetched runtime; public verification rejected the
candidate, promotion was skipped, and the old stable remained authoritative.
Separately authorized positive run `35285792273` reproduced signed sequence 11
generation `local-hosted-0-154-0-37fbbd8033b8-rald45-transition`, passed
immutable Release and public HTTPS byte readback, disposable update from the
actual sequence-10 stable Core, exact launch/version, credential-free semantic
doctor validation with real `upstream=unhealthy` and
`termux_core=healthy`, and second-update no-op. The promotion job then
reverified the signed public index and performed only `force:false` exact-parent
CAS. Result `committed` advanced `main` from
`b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` to its single child
`f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1`, making the transition generation
the public signed stable. Orchestration run `35285446624` passed all bounded
checks. RALD-6/7 are not started by this acceptance.

### Phase RALD-6 — fresh-install and update delivery E2E

Status: **accepted 2026-09-18**. Public stable remained fixed at signed sequence
11 generation `local-hosted-0-154-0-37fbbd8033b8-rald45-transition` throughout
the phase. RALD-6 added and proved acquisition/delivery only; it did not mutate
the live installation, public stable pointer, Release/Pages authority, or begin
RALD-7.

The selected public acquisition surface is one no-argument
`install-online.sh` fetched from the immutable accepted RALD-6 source commit.
It is a transport frontend to the already audited local
`install.sh -> bootstrap/codex-bootstrap -> Core` path, not another installer
or updater. It requires the existing Termux shell/curl/OpenSSL and never invokes
a package manager or compiler. The frontend fetches the bootstrap public key
from the fixed project release locator, requires the exact accepted public-key
SHA-256, verifies the canonical public stable index and its signature before
using its generation/release base, verifies the candidate release manifest
before using its inventory, downloads only the bounded signed inventory into a
private temporary root, fetches the local installer/bootstrap from one immutable
accepted repository commit, and delegates the actual fresh installation to that
unchanged local boundary. Persistent trust, entrypoint, generation, activation,
and recovery writes remain exclusively bootstrap/Core-owned.

The load-bearing hosted proof runs on native `ubuntu-24.04-arm` with the same
pinned AOSP Android bionic interpreter substrate already accepted by RALD-5.
It starts from disposable Termux-shaped HOME/PREFIX/TMPDIR roots, invokes the
online frontend through public HTTPS, and must land on exact signed public
sequence 11 and exact `codex-cli 0.154.0`. A subsequent ordinary no-argument
update against the default public index must be an exact-current no-op with a
byte-for-byte state snapshot match.

The same installed client must then prove forward delivery without mutating any
public publication surface. In the same hosted run, the exact public
sequence-11 generation may be copied to a new fixture generation identity,
with only that descriptor identity changed, and signed as release sequence 12
by the existing production authority through the already accepted bounded
Actions signing helper. The secret is exposed only to that exact signing step;
private material is deleted there, and the signed fixture is never uploaded as
an Actions artifact, GitHub Release, Pages content, or `main` commit. A
runner-local HTTPS server with a disposable locally trusted transport
certificate first serves the authentic public sequence-11 index, while the
sequence-12 generation is staged but unreachable from the stable locator. The
fixture promotion atomically switches only the runner-local stable locator to
the signed sequence-12 index. The same client must then use ordinary
no-argument `codex update` to activate sequence 12, retain sequence 11 as
`previous`, preserve exact runtime version behavior, and make the next update
an exact-current no-op.

Acceptance requires the focused installer contract test plus that complete
hosted fresh-install -> public no-op -> fixture promotion -> normal update ->
second no-op proof. No test-only credential/provider success, alternate device
trust key, hidden public URL override, live installation mutation, or RALD-7
work is permitted.

Gate: **passed**. Focused hosted run `35289795309` accepted the online
installer contract. Final native ARM64 run `35290588365` at workflow head
`8d3862342d773ac8a2dfb44c975f55771424ffb7` fetched immutable accepted installer
source `9972a3288c0531ba744e9bd1356273d9a080aa79` over public HTTPS and
fresh-installed exact signed public sequence 11 into empty disposable
Termux-shaped roots through the unchanged bootstrap/Core boundary. Exact
`codex-cli 0.154.0` and the default public exact-current no-op both passed,
including state snapshot equality. The same installed client then saw authentic
sequence 11 through runner-local HTTPS, while an exact-copy sequence-12 fixture
was signed only inside the accepted bounded production-authority step and staged
locally. Atomic fixture-locator replacement exposed the signed sequence-12 index;
ordinary no-argument `codex update` activated
`local-rald6-seq12-9972a3288c05`, retained sequence 11 as `previous`, and the
next update was an exact-current no-op with another state snapshot equality.
The run uploaded no artifacts and performed no GitHub Release, Pages, or
`main` write. Final public stable remained sequence 11 at
`f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1`. Earlier fixture-preparation runs
`35290065429` and `35290289525` failed before secret exposure because the
proof narrowed HOME/PATH before runner Rust tooling; the accepted repair only
moved release-builder preparation before that disposable environment switch.
RALD-7 remains separately gated and unstarted.

### Phase RALD-7 — full acceptance and activation

Status: **accepted 2026-09-18**. Final accepted product source is
`566034e1aff42bde2f3221ebc2da2b16def77d44`. Full source acceptance
`35308972611`, schedule-authorization acceptance `35310001193`, activated
dry-run `35310213404` / nested producer `35310222884`, real production cycle
`35321953530`, and post-promotion full acceptance `35322553140` establish
the complete chain. The real cycle promoted signed sequence 12 generation
`local-hosted-0-155-0-566034e1aff4` by non-forced exact-parent CAS to
`main=9aa8be4c63fe8d60dea130b979c0c12d1a5164bc` only after native ARM64
smoke, production signing, immutable Release staging, LKG-preserving Pages,
public HTTPS readback, disposable update/version/doctor/no-op proof, and every
automatic gate succeeded. The six-hour schedule is now authorized only for
ordinary strictly-newer official stable; manual publication retains explicit
authorization and acceptance-only controls remain schedule-ineligible. No force
push or live installation mutation occurred.

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
5. activate the six-hour `schedule` as publication authority only for an
   ordinary strictly-newer official candidate; scheduled acceptance-only,
   same-version, transition, and negative-test controls remain false, while
   manual dispatch continues to require explicit publication authorization;
6. run one real newer-version candidate cycle through the same
   build/smoke/sign/Release/Pages/readback/disposable-runtime/non-forced-CAS
   gates that future scheduled runs use;
7. record accepted evidence in `GOAL.md` and close the bundle in `WORKBOARD.md`.

### Post-RALD-7 legacy-lag compatibility correction

Status: **proof/source accepted; production remediation complete**. A retained sequence-7 R10 client was reconstructed from exact
historical source and official 0.153.4. Direct public update to current signed
sequence 12 failed closed in run `35343356792` solely at the historical
credential-dependent candidate doctor gate. Private sequence-13 repair proof
`35343870274` reused exact current 0.155.0 component bytes and the previously
accepted R10 activation bridge signal, then proved direct ordinary update,
previous retention, exact version, real doctor execution, and second-update
no-op. SPEC now requires the signed bridge doctor signal for every public stable
while R10 remains in the compatibility floor. Source correction
`3c0d2742bf99aa931b840b454b314e3fac428c9c` implements that producer rule and
full acceptance `35344310885` is green. Bounded remediation workflow source
`536d06a6088ccf3c850a6031c895a0ae6c2fe709` is additionally accepted by full
source run `35355371929`. Its false-by-default
`legacy_lag_jump_remediation` input is manual-only and `main`-ref-only; it
requires the authenticated public baseline to be exact version 0.155.0,
generation `local-hosted-0-155-0-566034e1aff4`, sequence 12, and next sequence
13 before overriding the ordinary equality no-op. Scheduled runs explicitly
reject this gate, and after sequence 13 promotion its sequence/generation
preconditions make it unusable. The remediation path additionally binds every
load-bearing component to the signed sequence-12 digest/mode inventory, permits
only the generation-id plus exact R10 doctor-signal descriptor delta, rebuilds
the exact historical R10 Core with the historical API-30 requirement, signs a
private sequence-7 proof fixture only through the existing production signing
helper, and requires the direct public sequence-7 update/previous-retention/
doctor/no-op proof before the unchanged non-forced CAS.

The source proof did **not** itself authorize public correction. Separate user
authorization subsequently executed the required ordering. First, only the
accepted corrected producer workflow was mirrored to exact
`main=9aa8be4c63fe8d60dea130b979c0c12d1a5164bc`; non-forced child
`0455ba6a86ec2a7392416f3ecba68925c9a8adfd` changed no stable index bytes.
Then manual production run `35364873732` selected the exact bounded
`legacy_lag_jump_remediation` path. It proved the sequence-13 component
digests/modes equal sequence 12, with only the signed generation identity and
R10 doctor bridge descriptor delta; production-signed and independently
verified both candidate and historical sequence-7 fixture; staged the immutable
candidate Release/tag on exact parent `0455ba6a...`; deployed candidate Pages
while retaining LKG; read back every signed byte; and directly updated the
historical sequence-7 R10 client to sequence 13. The corrected Core retained
sequence 7 as previous, reported 0.155.0, executed real doctor semantics
(`upstream=unhealthy`, `termux_core=healthy`), and proved second-update
no-op. Only after those gates did `force:false` exact-parent CAS report
`promotion_result=committed`, creating
`main=5bef52d07a07bd8612b4538dfb29a2396937a3fb` with only
`update-index-v1` and `update-index-v1.sig` changed from its parent. Public
stable is signed sequence 13
`local-hosted-0-155-0-566034e1aff4-legacy-lag-remediation`. The corrective
same-version gate remains forbidden to schedules and is now fail-closed/inert
because its exact sequence-12 baseline no longer matches. No fake credentials,
force push, or live installation mutation was used.

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
