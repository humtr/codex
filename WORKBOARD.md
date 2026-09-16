# Rust Core Workboard

This file owns only the active implementation or repository-maintenance bundle.
Accepted evidence and historical disposition belong in `GOAL.md`; normative
behavior belongs in `SPEC.md`.

## Current routing

- Repository: `humtr/codex`.
- Active branch: `rewrite/rust-core`.
- Original Rust Core goal status: **complete**.
- TERMUX-COMPAT TC-1, TC-2, and TC-3 are accepted and recorded in `GOAL.md`.
- TC-LIVE-BRIDGE source repair commit is
  `1b51902775c89c79522ef868921e4b95229a35fb` and has been pushed to
  `origin/rewrite/rust-core`.
- Selected upstream remains `rust-v0.154.0`, upstream commit
  `6b9826e3aa83b1a5947db50f4332cb9c65f1b340`.
- TC-LIVE-BRIDGE repository-native two-step live cutover is accepted and
  complete.
- Final live runtime is `codex-cli 0.154.0` with current generation
  `local-20260914-tc3-canonical-1` and retained previous generation
  `local-20260914-r10-browser-bridge-1`.
- Final live launcher SHA-256 is
  `0055ec0ecc762e4a4c878be62fd26f93118785218f004b26fe12b178cd3380eb`.
- UPDATE-CHANNEL-LATEST is accepted and closed. Final source acceptance before
  the closure ledger is `057f078a091441c1f624c7b229649bd1463ef774`.
- Public stable is signed sequence 10 generation
  `local-20260914-update-channel-bridge-1` at the verified GitHub Pages release
  base; retained R10 no-argument update and exact-current no-op are accepted.
- AUTO-UPSTREAM-ROLLBACK is accepted and source-closed at
  `21bb1cd78d4a6e6ef8e124b7f07230206c5aa5ea`.
- Active implementation bundle: **RELEASE-AUTOMATION-LOCAL-DERIVED (RALD)**.
- RALD-1 local-derived Core contract, RALD-2 Core/official-producer separation,
  and RALD-3 GitHub-hosted unsigned producer preflight are accepted. RALD-3 is
  pinned to producer source commit `28e65b32c8719cf913e62080d4674b54dbcc1a01`;
  the workflow was installed on default branch `main` by
  `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90`, and hosted manual run
  `35090080089` completed successfully with `candidate=false`.
- The drift-control execution contract is `RELEASE_AUTOMATION_PLAN.md`.
- RALD-4 Actions-secret signing is **source-complete / secret-backed hosted gate
  pending**. The signing helper source is pinned at
  `dfcdbead5fdcde454bedebf1a269816977f4f544`; workflow wiring is at
  `c178715f551362ac649e6c1ebc3422ca5cad1677`. After the first bounded
  `candidate=true` hosted attempt exposed an Android API-24 linker defect, the
  repaired hosted producer source is pinned at
  `1cdcb44d035ec5b1ce6339f2aa7b0e95831a6f0a`. Repository-side negative/positive
  signing, workflow-contract, locked workspace, clippy, and full workspace gates
  are green. No repository signing secret or production private-key value was
  created, changed, read, copied, or logged. RALD-5 has not started.
- The first separately authorized RALD-4 positive attempt was hosted run
  `35125040646`. It proved authenticated public stable `0.154.0`, the bounded
  `0.153.4 -> 0.154.0` acceptance comparison, and `candidate=true`, then failed
  closed during Android/AArch64 cross-link before candidate packaging, smoke, or
  secret-backed signing because API-24 bionic does not export `renameat2`.
  Signing was skipped and the production secret was not exposed. The source
  repair is `1cdcb44d035ec5b1ce6339f2aa7b0e95831a6f0a`.
- The separately re-authorized retry was hosted run `35148549722` at workflow
  head `c29cca7a3c5abb9fe269d2adf67f97645962a5ae`. Producer authentication,
  official `0.154.0` resolution, repaired Android/API-24 cross-build, candidate
  adaptation/qualification, and unsigned upload all succeeded. The smoke job
  reverified the candidate, then failed closed before emulator boot because the
  `macos-15-arm64` runner did not expose `sdkmanager` on `PATH`. Exact Manager,
  Core, and runtime execution, qualified upload, and signing were skipped; the
  production signing secret was not injected. The workflow repair now binds
  Android SDK tools under `$ANDROID_SDK_ROOT` with executable checks.
- The standing user authorization now permits bounded RALD-4 acceptance-only
  retries until this blocker is resolved. Hosted run `35155202983` exposed the
  unbounded ADB wait; `0de76cc04787f3eb6c28d42deeec3533b092b00f` bounded boot
  discovery and added emulator liveness/logging. Run `35161605865` at
  `d2becd586e6a20360110f38a28d595b6c850a7e7` then proved the macOS ARM64 hosted
  path is structurally unavailable: emulator 37.1.11 exits with
  `HVF error: HV_UNSUPPORTED` even with `-accel off`. Run `35162096906` at
  `bc8c8fd0224a2d38530b011093765f28afeeb364` moved smoke to Linux x64 and reached
  candidate revalidation, but failed before emulator installation. Commit
  `aca5ad8f9a13532e2977aa0a3a3e33a7d032507b` fixed that bootstrap order; hosted
  run `35162436291` then installed the emulator and system image successfully but
  failed closed before emulator launch because `avdmanager` entered the custom
  hardware-profile prompt, never created `rald3.ini`, and the subsequent emulator
  reported `Unknown AVD name [rald3]`. All runs stopped before exact
  Manager/Core/runtime smoke and signing, so the production signing secret was not
  injected. The selected repair keeps the KVM-backed API-35 Google APIs x86_64
  substrate and exact ARM64 candidate, but makes AVD creation deterministic:
  `ANDROID_AVD_HOME` is runner-temp-owned, `pixel_6` and `google_apis/x86_64` are
  explicit, and `emulator -list-avds` must prove `rald3` exists before launch.
  The guest must still advertise `arm64-v8a` and the unchanged Manager/Core/runtime
  binaries must execute through Android's ARM translation layer before signing.
- RALD-5 Release/Pages LKG-preserving publication and runtime-proven promotion,
  RALD-6 fresh-install/update delivery E2E, and RALD-7 full acceptance remain
  later phases.
- Public stable remains fixed during source development and may advance only after
  the selected plan's focused/full gates, public readback, disposable public-update
  runtime smoke, and exact non-forced promotion checks pass.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## RALD-4 source-complete disposition

RALD-4 source implementation is complete on 2026-09-16, but final acceptance is
intentionally withheld until the production-authority secret-backed hosted gate is
explicitly authorized and passes. Commit
`dfcdbead5fdcde454bedebf1a269816977f4f544` adds the fail-closed signing helper,
qualified-candidate boundary, sequence derivation, and disposable fixture tests.
Commit `c178715f551362ac649e6c1ebc3422ca5cad1677` wires that source into the hosted
workflow while retaining top-level `contents: read` permission and the RALD-3
unsigned producer/pre-sign semantics.

The signing job is reachable only for `candidate=true` after the Android/AArch64
smoke job succeeds and the deferred Manager marker is removed. The qualified
unsigned candidate is revalidated before secret exposure.
`CODEX_RELEASE_SIGNING_KEY` appears only in the exact signing step environment,
not global workflow state; the helper writes it only to an owner-only temporary
directory/key file under runner temporary storage, invokes the existing release
builder without inheriting the secret environment, and removes the key directory
on success or failure. Before signing it derives the Ed25519 raw public key and
requires byte-for-byte equality with the pinned accepted update authority. The
resulting manifest and update-index signatures are then independently verified
with that accepted public key before only the signed output archive can cross the
signing-job artifact boundary. No cache or publication/write authority is added.

Repository-side gate evidence is green: RALD-3 preflight 6/6, RALD-4 signing
fixture 6/6, RALD-3 workflow contract 5/5, and RALD-4 workflow contract 5/5.
Fixture negatives cover absent secret, malformed PEM, non-Ed25519 private key,
and non-matching Ed25519 derived authority; each fails before the publisher is
called. The matching disposable Ed25519 key signs successfully, both generated
signatures independently verify, fixture private-key bytes do not appear in logs
or signed outputs, and temporary key material is removed. PyYAML parsing,
formatting, and `git diff --check` pass. The actual release-builder suite is 17/17;
locked workspace check and workspace clippy `-D warnings` pass; full locked
workspace tests are Core 145 passed / one explicit real-Termux smoke ignored,
Manager 20/20 plus 11/11 integration, and release-builder 17/17. Tests use only
disposable fixture keys/roots; the live installation is not accessed.

The remaining RALD-4 acceptance action is one user-authorized
production-authority positive GitHub-hosted signing execution. To avoid making
acceptance depend on the calendar date of the next upstream release, the selected
plan now permits one bounded `workflow_dispatch` acceptance-only stimulus on
`rewrite/rust-core`: it may compare fixed disposable baseline `0.153.4` against
real official stable `0.154.0` only while the independently authenticated public
stable is also exact `0.154.0`. This does not lower or rewrite public stable. The
real official archive/digest, Android/AArch64 build and smoke, sequence derived
from authenticated public state, accepted authority match, signing, independent
verification, and key cleanup remain required. The acceptance signed index uses
a non-routable `.invalid` release base and its signed output is deleted in-job
rather than uploaded. Scheduled and ordinary manual runs retain the real-public-
stable comparison. The gate must still prove the secret derives the accepted
update public authority, candidate signing succeeds, independent verification
succeeds, and no private material leaks. The secret value itself must not be
requested or exposed. RALD-5 publication authority is absent, public Release/
Pages/index state remains protected, and RALD-5 has not started. The bounded
acceptance-stimulus amendment is repository-green: RALD-3 preflight 6/6,
RALD-4 signing fixture 6/6, RALD-3 workflow contract 5/5, amended RALD-4
workflow contract 6/6, YAML parse and `git diff --check`, formatting,
release-builder 17/17, locked workspace check, workspace clippy `-D warnings`,
and full locked workspace tests with Core 145 passed / one explicit real-Termux
smoke ignored, Manager 20/20 plus 11/11 integration, and release-builder 17/17.

The first authorized acceptance-only hosted execution is run `35125040646` at
workflow head `99235e0541fcd6e0cd541c2157093fce13c90ac2`. Public-stable authentication
and official-stable resolution succeeded, and the log records the exact bounded
comparison baseline `0.153.4` while authenticated public stable remained
`0.154.0`; the producer therefore entered the real `candidate=true` cross-build.
The build then failed closed under NDK API 24 because Core and Manager linked a
direct `renameat2` libc symbol that API-24 bionic does not export. Candidate
adaptation/upload, Android smoke, and signing were skipped, so the production
signing secret was not injected and no signed acceptance output existed.

Commit `1cdcb44d035ec5b1ce6339f2aa7b0e95831a6f0a` repairs only that platform link
boundary: Android/AArch64 keeps exact `RENAME_NOREPLACE` semantics by invoking
the arm64 Linux `renameat2` syscall through bionic's stable `syscall` wrapper;
non-Android paths retain the existing direct libc call. Post-repair local gates
pass for formatting, locked workspace check, release-builder 17/17, Manager
20/20 plus 11/11 integration, Core 145 passed / one explicit real-Termux smoke
ignored, and `git diff --check`. A real Android/AArch64 Termux link of Core,
Manager, and release-builder also has no undefined `renameat2` symbol. The hosted
workflow now pins this repair commit. The separately re-authorized retry,
`35148549722`, then proved the repaired cross-build in GitHub hosting: the entire
producer job succeeded through real official archive adaptation, qualification,
and unsigned candidate upload. Its macOS ARM64 smoke job also downloaded and
reverified that exact candidate, but failed before emulator creation with
`sdkmanager: command not found`; Manager/Core/runtime execution and signing were
therefore skipped. The follow-up workflow repair removes that PATH dependency by
binding `sdkmanager`, `avdmanager`, `adb`, and `emulator` to executable paths
under `$ANDROID_SDK_ROOT`. Run `35155202983` exposed the unbounded ADB wait and
`0de76cc04787f3eb6c28d42deeec3533b092b00f` made boot discovery bounded and
log-producing. Run `35161605865` then made the macOS ARM64 platform blocker
explicit: emulator 37.1.11 still attempts HVF with `-accel off` and exits with
`HVF error: HV_UNSUPPORTED`. Run `35162096906` moved smoke to Ubuntu x64 and
proved candidate download/revalidation there, while
`aca5ad8f9a13532e2977aa0a3a3e33a7d032507b` repaired emulator installation.
Run `35162436291` proved that installation but exposed a separate AVD creation
contract defect: `avdmanager` prompted for a custom hardware profile and returned
without a usable `rald3.ini`, so the emulator failed with `Unknown AVD name
[rald3]`. The signing job was skipped and the production secret was not injected.
The current repair keeps the hosted-only `ubuntu-24.04` KVM + API-35 Google APIs
x86_64 substrate and byte-identical ARM64 candidate, but binds `ANDROID_AVD_HOME`
to private runner temp storage, selects `pixel_6` plus `google_apis/x86_64`
explicitly, and requires `emulator -list-avds` to contain `rald3` before launch.
The guest must still advertise `arm64-v8a`, and exact Manager, Core, and runtime
execution through Android's ARM translation layer remains the pre-sign gate.
RALD-4 remains pending until that smoke, secret-backed signing, and independent
verification pass. RALD-5 remains not started.

## Accepted RALD-3 disposition

RALD-3 is accepted on 2026-09-16. Commit
`28e65b32c8719cf913e62080d4674b54dbcc1a01` is the workflow's immutable
producer-source pin. Release-builder has one explicit hosted cross-build
exception, `--defer-manager-probe`: it requires a Manager artifact, first proves
that private snapshot is an Android/AArch64 PIE ELF, emits the unsigned
`.manager-probe-deferred` marker, and makes `publish` fail before signing until
the probe is discharged. Native/default qualification remains unchanged.

The repository-owned `auto-release-termux.yml` source defines six-hour and manual
triggers, read-only permissions, concurrency serialization, authenticated current
stable readback, official OpenAI stable comparison, pinned-source Android/AArch64
Core/Manager cross-build, unsigned candidate inventory/digest checks, one-day
unsigned artifact transfer, and ARM64 Android Manager/Core/runtime smoke. The
workflow source contains no signing-secret access, release signing, GitHub
Release/Pages publication, stable-index write, or git push. Repository preflight
and workflow-contract tests are 5/5 each; deferred Manager focused regressions
are 3/3; full locked workspace tests are Core 145 passed / one explicit live
smoke ignored, Manager 20/20 plus 11/11 integration, and release-builder 17/17.
The final repository gate is `job_ulk_1460dff8e1`.

Default-branch activation installed that exact workflow blob on `main` in
`b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` without changing the signed stable
index. GitHub-hosted `workflow_dispatch` run `35090080089` then completed with
conclusion `success` on `ubuntu-24.04`, checked out exact source
`28e65b32c8719cf913e62080d4674b54dbcc1a01`, passed the 5/5 preflight helper
tests, authenticated the current signed stable generation
`local-20260914-update-channel-bridge-1`, and resolved wrapper/upstream stable as
`0.154.0` / `0.154.0`. The resulting `candidate=false` decision correctly
skipped cross-build, unsigned candidate adaptation/upload, and Android smoke;
run artifacts are empty. Public stable and its signature remained byte-identical,
GitHub Release ID `388405334` remained the current release, no Pages publication
run was triggered, no Actions secret was created/changed/read for signing, and
the live installation was not accessed or mutated. RALD-3 is therefore closed;
RALD-4 remains not started.

## Accepted RALD-2 disposition

Core `codex update` is consumer/local-derived only. The device-side
`automatic_update` producer/publisher and its official private-key, GitHub CLI,
GitHub Release/Pages, readback, and stable-promotion wiring are removed. A
matching former maintainer key and authenticated fake `gh` are inert to ordinary
signed-channel update, rollback, held retry, and exact forced retry; those routes
do not perform ambient upstream producer discovery and do not create a local
publication. Transport-level signed-channel unavailability retains only the
accepted RALD-1 local-derived fallback.

The explicit `codex-release-builder fetch/build/publish` boundary remains intact
and its signed publication still enters existing Core admission. This is source
producer tooling, not installed runtime authority; GitHub-hosted scheduling,
secret-backed official signing, Release/Pages publication, and stable promotion
remain RALD-3 through RALD-5.

Acceptance gate is green: focused consumer/RALD-1/ARH/channel E2E 4/4; explicit
builder-to-admission 1/1; format and `git diff --check`; locked workspace check;
workspace clippy `-D warnings`; removed-producer symbol audit; full locked
workspace tests with Core 145 passed / one explicit live-Termux smoke ignored,
Manager 20/20 plus 11/11 integration, and release-builder 15/15. Tests ran only
in the isolated worktree/disposable roots. The live installation, public stable,
GitHub Release/Pages stable state, and Actions secrets remain unchanged. RALD-3
is next.

## Accepted RALD-1 disposition

Core now exposes exact `codex update --build-local`, and only automatic signed-
channel transport unavailability may enter that same local-derived construction.
The build uses exact official upstream metadata/archive qualification and normal
Termux adaptation/probes, creates a fresh private 0600 Ed25519 key in disposable
staging, signs the immutable local candidate, deletes private key material before
successful activation, and persists only the public verifier in the current
verifier slot. `activation-state-v3.update_key` remains the official authority;
the local release carries the authenticated public baseline sequence rather than
allocating a public sequence.

Special local-derived admission is internal to that construction path. Ordinary
`--local`/`--remote` signed admission cannot use a self-signed local-derived
marker. Local-derived activation is blocked while an authenticated rollback hold
is active. Rollback from local-derived to retained official state creates no
public hold or rollback-Core guard, while rollback from official state and exact
held `--force` behavior remain unchanged. A later authenticated greater public
sequence supersedes local-derived current through the normal official authority.
No local-derived path invokes `gh` or official publication.

Acceptance gate is green: RALD-1 process E2E 4/4; existing ARH/signed-channel
regressions 4/4; locked workspace check; workspace clippy `-D warnings`; format
and `git diff --check`; full locked workspace tests with Core 153 passed / one
explicit live-Termux smoke ignored, Manager 20/20 plus 11/11 integration, and
release-builder 15/15. Tests ran only in the isolated worktree/disposable roots.
The live installation and public stable remain unchanged. At RALD-1 acceptance the
remaining official producer was intentionally deferred; accepted RALD-2 now
removes it from Core runtime.

## Accepted TC-LIVE-BRIDGE disposition

The bounded repair retains existing v3/v4 trust and activation authority. Only
exact `creation_metadata = "r10-browser-helper-bridge-v1"` with the two accepted
browser helper identities in exact order may use signed `helpers/0` and
`helpers/1`. Normal generations use only `browser/open/curl` and
`browser/manual/curl`. Marker/layout mismatch fails closed. No release format,
key, bootstrap authority, package install, PATH authority, raw-runtime patch, or
direct launcher replacement was added.

Source acceptance and the decisive disposable migration/rollback proof are
recorded in `GOAL.md`. The final repository gate passed Core 141/0/1,
release-builder 15/15, Manager 20/20 plus 11/11, locked workspace check/test,
clippy `-D warnings`, formatting, and `git diff --check`.

Live preflight rebound the exact R10 baseline and final signed candidates. The
installed R10 updater then admitted signed bridge sequence 8; only after bridge
health and protected-state checks passed did the new Core admit canonical
sequence 9. Final doctor is healthy, resolver and protected auth/config/profile/
session identities are unchanged, canonical nested browser helpers are active,
and the bridge is retained as the one rollback generation. No live rollback was
performed solely for evidence.

## Accepted UPDATE-CHANNEL-LATEST disposition

Scope is deliberately bounded to the two defects exposed by the post-cutover
update smoke. Core may treat an authenticated candidate as already current only
when its release sequence equals the installed current sequence and both its
generation identity and signed release manifest exactly match current. Lower
sequences and equal-but-different releases remain fail-closed. No-op success must
not stage, probe, rewrite the launcher, or mutate activation state.

The stable channel repair publishes one new signed sequence-10 generation,
`local-20260914-update-channel-bridge-1`, with current repaired Core and exact
`creation_metadata = "r10-browser-helper-bridge-v1"`. Its signed helper inventory
remains `helpers/0` and `helpers/1` so the retained R10 sequence-7 parser can
consume the final public target directly. The selected upstream/runtime and TC1/
TC2/TC3 behavior remain 0.154.0-equivalent; this is a transport compatibility
layout, not a second updater or a rollback of canonical local policy.

GitHub Release assets remain staging for the large top-level files. A fixed
GitHub Pages workflow reconstructs the exact signed tree under
`https://humtr.github.io/codex/<generation_id>/` after binding the pinned public
key and verifying signature, bridge metadata, exact inventory, helper identities,
and every signed digest. The automated Core Release publisher remains fail-closed
for nested signed inventory. The workflow may be mirrored to `main`, but release
bytes never use the Contents API. Failed deployment/readback cannot advance the
signed index. Raw Git-tag publication, slash-bearing Release assets, and
companion-release routing are rejected qualification paths, not fallbacks.

Acceptance is complete. Focused anti-rollback/no-op regressions and the full
repository gate are green; the final gate is release-builder 15/15, Core
142/0/1, Manager 20/20 plus 11/11, locked workspace check/test, clippy
`-D warnings`, formatting, and `git diff --check`. Sequence 10 was built and
signed from the accepted 0.154.0 source, then staged as GitHub Release ID
388405334. Pages workflow run 34848065440 succeeded, and complete HTTPS readback
byte-matched the 292,960,783-byte signed tree. The final signed stable index at
`main` head `40692fd63b4b5408f4600b338e854225d8b653c5` targets sequence 10. A disposable retained R10
performed no-argument `codex update` directly from 0.153.4/sequence 7 to
0.154.0/sequence 10; a second no-argument update returned exit 0 with the exact
already-current message and no launcher/state/generation-tree mutation. The real
live sequence-9 canonical runtime remained healthy and unchanged at the measured
launcher, activation-state, resolver, and protected metadata boundaries. No live
runtime replacement, rollback, package-manager action, PATH/resolver change,
signing-key rotation, or credential/provider mutation occurred in this bundle.

## Accepted AUTO-UPSTREAM-ROLLBACK bundle

ARH-1: `codex update --rollback` remains the sole Core-owned public rollback
selector; no top-level `codex rollback` command is added. Rollback success writes
a separate exact-format hold for the rolled-back-from
generation/sequence without changing activation-state v3. After trust and
anti-rollback validation, plain update refuses candidates that do not exceed the
held sequence. A committed greater sequence removes the obsolete hold/guard;
`codex update --force` requires that valid hold, retries exactly its held sequence
for one invocation, retains the normal hold after success, and bypasses no other
admission/probe/activation check.
The disposable first-migration smoke proved that restoring a legacy pre-ARH Core
would bypass that hold on the next process. ARH-1 therefore also owns the bounded
rollback-Core guard repair: if and only if the rollback target lacks exact
`codex-update-hold-v1` capability, keep the held generation's signed Core launcher
as the control plane while rolling back the previous signed payload/pointer. Bind
that exception to exact target/held identities, held sequence, verifier authority,
and held signed Core digest; use the guard as crash-window hold intent, and remove
it after force/greater activation or a normal hold-aware Core rollback.

ARH-2: with no self-hosted Actions runner and no private-key export, `codex update`
is the maintainer-side automation controller only when the default signed channel,
secure matching local signing key, and authenticated local GitHub CLI are all
present. Ordinary clients only consume stable. Exact-current or hold-only public
outcomes may then trigger official OpenAI stable discovery and local adaptation/
signing only for a genuinely newer version. GitHub Release stages already signed
bytes; Actions reconstructs/deploys current stable plus candidate in one Pages
site. Full candidate readback and disposable public update must pass before one
non-forced Git commit atomically replaces the signed index and signature. Any
pre-commit failure leaves stable untouched; an indeterminate final ref result is
not guessed. A known committed promotion survives a later local activation failure,
which is reported as deferred and recovered by the next `codex update`.

Acceptance evidence is complete: focused ARH routing/hold/force/publisher tests
and the disposable legacy-guard process flow pass; release-builder is 15/15; Core
is 156 passed, 0 failed, 1 explicitly ignored real-Termux smoke; Manager
integration is 11/11; locked workspace check/test, clippy `-D warnings`, formatting,
and `git diff --check` all pass. Controlled fixtures prove the newer-version and
publication paths without artificially advancing public stable, and the acceptance
run did not mutate the installed live Codex or protected user/provider state. The
source closure for this accepted ledger state is the corresponding clean
fast-forward commit on `rewrite/rust-core`.

## Selected RELEASE-AUTOMATION-LOCAL-DERIVED plan

This follow-up separates official release production from device-side release
consumption. `codex update`, `codex update --rollback`, rollback hold/guard, and
bounded `codex update --force` retain their accepted consumer/recovery semantics.
A new explicit `codex update --build-local` is always local-derived regardless of
whether an official private key happens to exist on the device; it preserves the
official `update_key` and can never publish official stable.

Official release production moves to a GitHub-hosted scheduled producer. The
selected signing input is repository Actions secret `CODEX_RELEASE_SIGNING_KEY`,
which must derive the exact accepted public update authority before signing. The
secret value must never enter repository content, logs, artifacts, or device
state. GitHub-hosted publication must preserve the currently authoritative
successful public stable generation while a candidate is staged, and upload or
Pages success alone is never enough for promotion: full public readback plus a
disposable real `codex update`/launch/doctor/no-op smoke must pass before the
signed stable index can advance atomically.

`RELEASE_AUTOMATION_PLAN.md` freezes the comparison evidence, trust split,
last-known-good rule, implementation phases, failure matrix, acceptance gates,
and resume protocol. It is the selected plan for this bundle but does not override
`SPEC.md`; RALD-1 must update the normative contract before behavior changes.
This documentation-selection step does not mutate `main`, the public stable
index, GitHub Releases/Pages, Actions secrets, or the installed live runtime.

## Resume rule

A fresh session resumes by reading `SPEC.md`, then `GOAL.md`, then this file and
verifying branch/HEAD plus the final live state before selecting any new work.
No further TERMUX-COMPAT implementation or live-migration bundle is implicitly
selected. Historical accepted evidence stays in `GOAL.md`.
