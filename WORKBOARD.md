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
- Public stable is signed sequence 11 generation
  `local-hosted-0-154-0-37fbbd8033b8-rald45-transition` at the verified GitHub
  Pages release base after accepted RALD-5 promotion commit
  `f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1`.
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
- RALD-4 Actions-secret signing is **accepted 2026-09-17**. The signing helper
  source is pinned at `dfcdbead5fdcde454bedebf1a269816977f4f544`; the repaired
  hosted producer/signing source is pinned at
  `1cdcb44d035ec5b1ce6339f2aa7b0e95831a6f0a`, and the successful acceptance
  workflow head is `d0af224b6738e80feb458e6030b6da517d94a1f5`. Hosted run
  `35176798621` passed producer qualification, native ARM64 Manager/Core/runtime
  smoke, production-authority signing, independent signature verification, and
  cleanup. Repository-side negative/positive signing, workflow-contract, locked
  workspace, clippy, and full workspace gates remain green. No production
  private-key value was requested, read, copied, or logged.
- RALD-4.5 activation-safe candidate-probe correction is **proved 2026-09-18**.
  Product source `37fbbd8033b8cc2d508689ab1d6637b4c4f5d516` preserves the exact candidate
  version/integrity probe but removes full upstream-doctor health from activation.
  Backward-compatible transition proof run `35284270406` at workflow head
  `f74045156bff00cae22f3c8d67823096ab1fe13f` passed current-stable-to-candidate
  update, public HTTPS byte/signature readback, exact launch/version, semantic
  credential-free doctor validation, and second-update no-op. Upstream doctor
  actually ran and was `unhealthy`; Termux Core was `healthy`. Transition mode
  skipped CAS promotion at that proof point, so `main` remained
  `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` and public stable remained signed
  sequence 10 until the separately authorized RALD-5 promotion below.
- RALD-5 is **accepted 2026-09-18**. Bounded authorization commit
  `d1b53576f9b173dcf786b21271b556712d694bbf` adds a separate false-by-default
  transition-promotion input without weakening the transition-stage fence.
  Repaired negative run `35285462288` reached signed immutable staging and Pages,
  deliberately failed public verification after local runtime tampering, skipped
  CAS promotion, and left `main` at
  `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90`. Positive run `35285792273`
  then passed exact source/upstream qualification, native ARM64 smoke,
  production-authority signing, immutable Release/Pages staging, every-byte HTTPS
  readback, actual old-stable-Core update, exact version, semantic credential-free
  doctor validation, and second-update no-op for signed sequence 11 generation
  `local-hosted-0-154-0-37fbbd8033b8-rald45-transition`. The final CAS used
  `force:false` and committed exactly one child
  `f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1` of the old `main`; public stable
  now targets that sequence-11 transition generation. Orchestration run
  `35285446624` completed successfully. RALD-6 was separately authorized on
  2026-09-18 and is active below; RALD-7 remains not started and unauthorized.
- First bridge negative-proof run `35192131899` at workflow head
  `16335d9fb7a5b170363d26a4ecde5add47f45bbc` proved the manual equality gate,
  exact official archive binding, Android/AArch64 build and native smoke,
  production-authority signing, and independent signature verification. It then
  failed closed in Release staging after exact 12-asset preparation but before a
  candidate tag or Release existed. Pages, public-readback, and CAS jobs were
  skipped; `main` remained `b17cc05ec8ec18bdbfd960e413dc4c47ffeb9f90` and the
  signed stable blobs were unchanged. This is not the intended RALD-5 negative
  gate and is not acceptance evidence.
- Staging repair commit `4f289759db48c77c0df9a012e2b5a8d91a391b17`
  removes direct candidate-tag ref creation and makes Release creation bind its
  locator tag to the exact verified pre-promotion `main` parent; an existing tag
  must match that same parent. Signed manifest/index bytes remain candidate
  authority. Workflow-contract repair commit
  `449f1ed89433ea86c9678a4d247d048d73f9138e` fixes this invariant in the RALD-5
  contract tests. The required repaired negative proof is satisfied by run
  `35285462288` before the accepted positive promotion run `35285792273`.
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
- Hosted run `35163080182` at workflow head
  `16c7ee27030b4bd5c46ac4da5ef2b7891f7b7a02` proved deterministic AVD creation,
  KVM-backed x86_64 boot, and `arm64-v8a` in the guest 64-bit ABI list. The next
  exact-smoke step failed at its first `adb shell` with `device offline` because
  the boot step's `EXIT` trap also ran on success, killing the adb server and
  emulator wrapper at the step boundary. Signing was skipped and the production
  signing secret was not injected. The selected lifecycle repair keeps failure
  cleanup armed during boot, records the emulator PID and disarms that trap only
  after successful ABI qualification, rechecks `adb get-state=device` with a
  bounded poll before exact Manager/Core/runtime execution, and moves emulator/
  adb cleanup to an `always()` post-smoke step. The same repair changes the
  qualified-tar size check from macOS `stat -f` to GNU `stat -c` for Ubuntu.
- Hosted run `35168633626` at workflow head
  `7c491db6900d860afcb12e9a01eec2c227c3e521` proved that lifecycle repair:
  producer and boot/ABI qualification succeeded, the emulator remained online
  across the step boundary, and all three unchanged ARM64 candidate binaries were
  pushed into the guest. The exact runtime step then exited 139 before its first
  expected output assertion, so qualification/upload and signing were skipped and
  the production signing secret was not injected. Because the old fail-fast shell
  did not identify which executable produced SIGSEGV or preserve its crash
  evidence, commit `7ab3a2a0a5dde84893e0e2e1510b56da6b600665` kept the same
  commands and fail-closed acceptance gate while adding bounded diagnostic
  capture.
- Hosted diagnostic run `35169112943` at workflow head
  `7ab3a2a0a5dde84893e0e2e1510b56da6b600665` isolated the blocker exactly.
  Producer, candidate revalidation, KVM-backed API-35 x86_64 boot, secondary
  `arm64-v8a` qualification, and cross-step emulator lifecycle all succeeded.
  Manager returned 0, Core returned 0, and only the unchanged upstream runtime
  returned 139. The guest reported `libndk_translation.so`, native-bridge exec
  enabled, and ndk-translation version `0.2.3`; Android's crash buffer recorded
  SIGSEGV/SEGV_MAPERR inside
  `ndk_translation_program_runner_binfmt_misc_arm64` while executing
  `runtime --version`. Qualification/upload and signing were skipped, and the
  production signing secret was not injected.
- A separate read-only forensic check downloaded that run's exact unsigned
  candidate and authenticated the current public `0.154.0` release manifest with
  the accepted update public key. Candidate runtime SHA-256
  `123c96efbd8b16e1ccd5c34a6212b0d8f1c895e92917829cb6371ddfd39aa8c0`
  is byte-for-byte identical to the signed public runtime digest. The builder
  selects the official static AArch64 ET_EXEC and the accepted adaptation only
  performs the fixed-length 54-byte path-string policy; this is therefore not a
  newly introduced candidate-runtime byte regression.
- The user explicitly selected replacement of the unusable x86_64 translation
  substrate because release automation is not acceptable unless GitHub Actions
  can execute build -> real runtime smoke -> sign -> independent verify end to
  end. `RELEASE_AUTOMATION_PLAN.md` now selects one common native hosted pre-sign
  path: GitHub-hosted `ubuntu-24.04-arm`, exact host `aarch64`, and a pinned AOSP
  Android 15 ARM64 runtime APEX at commit
  `8aeb37cca394ce39c1311744c60960cbd466aa77` with SHA-256
  `83bf0dce249728dae48149b80d28b48115c54adad95a352120d58a6ac669d1fc`.
  Manager/Core retain their Android PIE bytes and execute through the extracted
  official bionic `linker64`/libraries; the exact static ARM64 runtime executes
  natively on the same runner. Emulator/KVM/foreign-architecture translation and
  package-manager bootstrap are no longer accepted. Candidate bytes, upstream
  digest binding, deferred Manager gate, accepted signing authority, independent
  signature verification, and no-publication RALD-4 boundary remain unchanged.
  Hosted positive proof is now complete in run `35176798621`; RALD-4 is
  accepted.
- Hosted run `35172048744` at workflow head
  `3ccbad1dbc5d0eb4f4fb94e8f23e74deb27bbe63` proved the replacement substrate
  itself. Producer and candidate revalidation succeeded; GitHub assigned the
  smoke job to `ubuntu-24.04-arm`, `uname -m` was `aarch64`, and the exact pinned
  AOSP runtime APEX passed its byte/hash checks and yielded the expected ARM64
  bionic linker/libraries. Exact candidate execution then returned
  Manager/Core/runtime rc `0/0/0`, eliminating the prior translation/runtime
  blocker. The step failed only in a subsequent exact stdout/stderr assertion,
  before deferred-marker removal, qualified upload, or signing, so the production
  signing secret was not injected. The next bounded retry preserves every output
  assertion and only emits at most 4096 bytes of each nonsecret smoke stdout/stderr
  on assertion failure so the remaining formatting mismatch can be identified;
  acceptance is not weakened.
- Hosted diagnostic run `35172361087` at workflow head
  `9ba1deaa379b3952df7c4408ca7a36ddd45285e1` again returned
  Manager/Core/runtime rc `0/0/0`. Manager stdout exactly matched the artifact
  probe, Core stdout contained the exact update usage line, runtime stdout was
  `codex-cli 0.154.0`, and runtime stderr was empty. The only mismatch was the
  pinned bionic linker emitting the same deterministic two-line warning for both
  Android PIE executables because the minimal hosted userspace does not recreate
  `/linkerconfig/ld.config.txt`. The accepted workflow repair requires those two
  pinned-substrate warning lines byte-for-byte for Manager and Core; any missing,
  changed, or additional stderr still fails closed.
- Hosted acceptance run `35176798621` at workflow head
  `d0af224b6738e80feb458e6030b6da517d94a1f5` completed end to end. Producer
  authentication/build/adaptation succeeded, `ubuntu-24.04-arm` native smoke
  revalidated the candidate and pinned AOSP bionic substrate, exact
  Manager/Core/runtime execution passed, the deferred Manager marker was removed,
  and the qualified unsigned candidate crossed into the signing job. Before
  secret exposure the job revalidated the candidate and accepted public
  authority; the bounded signing step then used the existing repository signing
  authority to sign release sequence `11`. Independent OpenSSL verification
  passed for both `release.manifest`/`release.sig` and
  `update-index-v1`/`update-index-v1.sig`. The acceptance index used only the
  non-routable `.invalid` release base, temporary signing material and signed
  output were removed in-job, and signed-artifact upload was skipped. No
  Release/Pages/main/public-stable/live-runtime mutation occurred.
- RALD-5 Release/Pages LKG-preserving publication and runtime-proven promotion is
  accepted.
- RALD-6 fresh-install/update delivery E2E is **accepted 2026-09-18**. Product
  source `9972a3288c0531ba744e9bd1356273d9a080aa79` adds only the bounded
  `install-online.sh` acquisition frontend and leaves the audited local
  installer/bootstrap unchanged. Focused run `35289795309` accepted slice A.
  Load-bearing native ARM64 run `35290588365` at workflow head
  `8d3862342d773ac8a2dfb44c975f55771424ffb7` accepted slices B/C end to end:
  immutable public installer readback, empty-root fresh install to signed public
  sequence 11, exact `codex-cli 0.154.0`, default public update no-op with state
  snapshot equality, bounded production-authority signing of runner-local
  sequence-12 fixture `local-rald6-seq12-9972a3288c05`, atomic local fixture
  locator promotion, ordinary same-client no-argument update to sequence 12 with
  public sequence 11 retained as previous, exact version, and second no-op with
  state snapshot equality. Run artifacts were empty; the fixture was not
  uploaded/released/deployed/promoted. `main` and public stable remained
  `f221de1225471fb5eda5bbdfcbd0d9db0c2f43b1` / sequence 11, and the live
  installation was untouched.
- RALD-6 failed proof runs `35290065429` and `35290289525` are diagnostic
  history only. Both completed the accepted fresh-install/public-no-op half and
  stopped before sequence-12 signing because the fixture step switched to the
  disposable Termux HOME/PATH before invoking runner Rust tooling, hiding
  `rustup` and then `cargo`. Signing/delivery were skipped and the production
  secret was not exposed. Commit
  `8d3862342d773ac8a2dfb44c975f55771424ffb7` fixes only that proof-step
  ordering; it adds no toolchain/package install or product trust path.
- RALD-7 full acceptance/activation is **active and explicitly authorized
  2026-09-18**. Source acceptance first runs the complete repository gate on
  `rewrite/rust-core` with producer source pinned to accepted RALD-6 product
  source `9972a3288c0531ba744e9bd1356273d9a080aa79`. Until that gate is green,
  `main`, the signed public stable index, GitHub Release/Pages authority, and
  any live installation are protected and unchanged. After source acceptance,
  activation may fast-forward `main` only to install the accepted scheduled
  producer plus its Pages reusable workflow without changing the stable index,
  then must pass a hosted manual dry-run before any real candidate cycle.
  Force push is prohibited. First full-gate run `35296308869` stopped before
  activation: workflow/action syntax, staged credential/private-key scan,
  `git diff --check`, and signed public-stable byte audit all passed, while
  package/workspace tests failed because the generic Ubuntu jobs had not
  provisioned the deterministic Termux proof substrate required by existing Core
  tests (`PREFIX/bin/openssl`, `script`, `clang`, `gzip`, `cat`,
  `chmod`, plus the fixed Termux shell path). This is a test-runner fixture
  defect, not a product failure. `main` and public stable remained unchanged;
  repair only the hosted test substrate and rerun all source gates. Second run
  `35296431449` proved that repair reduced Core failures to 2 while the
  workflow/credential gate remained green: one builder proof rejected the
  symlinked test `gzip` because release tooling correctly requires a real
  executable file, and one local-derived proof demonstrated that its static
  AArch64 probe must compile and execute on an AArch64 host rather than x86_64.
  Therefore the package/workspace full-test jobs move to native
  `ubuntu-24.04-arm` and install regular copies of strict tool fixtures; this
  remains test substrate only. Third run `35296572414` reduced the remaining
  Core failures to the same 2 and proved both were a different boundary:
  release-builder and local-derived production correctly reject the Linux test
  executable because it lacks the Android AArch64 `/system/bin/linker64`
  contract. The repair must not weaken that check. Instead, cross-build the exact
  accepted product source `9972a3288c0531ba744e9bd1356273d9a080aa79` into a
  real Android/AArch64 Core with the accepted NDK path, transfer that unsigned
  test artifact only to native ARM64 full-test jobs, provision the already pinned
  AOSP bionic substrate, and reuse the existing test-only
  `CODEX_B10_RELEASE_CORE` hook for the two proofs. Production builds still use
  `current_exe()` unconditionally. Fourth run `35296820584` proved the exact
  Android Core cross-build and public audit, then exposed the final synthetic
  control mismatch: test helper `b4_write_signed_release` signed host Linux
  platform fields even when the generation itself was Android/AArch64, so the
  real Android Core correctly rejected that test release. The repair remains
  test-only: synthetic release control now uses the generation's own platform
  binding, and while the explicit `CODEX_B10_RELEASE_CORE` test hook is present
  the Linux harness may inspect either native fixtures or the exact
  Android/AArch64 proof platform. Production release policy remains compile-time
  exact. Fifth run `35297146973` reduced Core to 142 passed / 4 failed / 1
  ignored. Three failures came from injecting the production Android Core into
  the entire cfg(test) suite, which enabled optional release-Core process tests
  that depend on test-only doctor/fault hooks; the fourth local-derived failure
  mixed a native Linux baseline with an Android candidate inside one test
  process. The repair does not drop coverage: the two tests that require the
  Android artifact are excluded only from the generic full-suite invocation and
  immediately rerun as mandatory exact focused tests with
  `CODEX_B10_RELEASE_CORE`; all other tests retain the normal cfg(test)
  environment. A test-only generation-requirements selector accepts exact
  Android/AArch64 only while that explicit hook is present. Sixth run
  `35297488569` then passed the complete package gate, including both mandatory
  Android focused proofs and Manager integration, and passed workspace check and
  full workspace tests. The sole remaining failure was clippy
  `-D unused-variables`: the manifest parameter of the test-only selector is
  intentionally unused in production cfg. Rename it to `_manifest` without any
  behavioral change and rerun every gate. `main` and public stable are still
  unchanged.
- RALD-7 source acceptance run `35297704150` is green, and activation commit
  `17ccd88f02b00d040e2aea7bd417e98bb53ff630` installed the accepted producer
  and Pages reusable workflow on `main` without changing the signed stable
  index. During activated dry-run, official upstream advanced to `0.155.0`.
  Producer run `35298325500` authenticated public sequence 11, resolved exact
  official 0.155.0 metadata/digest, cross-built Core/Manager, then failed closed
  before artifact upload/signing/publication because the release-builder still
  required the historical exhaustive 0.154 archive resource list. Probe run
  `35298499738` proved the official 0.155.0 package remains
  `layoutVersion=1` with the same requested version/target/variant/entrypoint
  contract and the same selected `bin/codex` plus
  `bin/codex-code-mode-host`, while adding a bounded voice-resource subtree.
  The exact-resource whitelist is therefore replaced normatively by semantic
  layoutVersion-1 validation plus strict tar safety/size/type bounds. Product
  source `566034e1aff42bde2f3221ebc2da2b16def77d44` implements that contract:
  required semantic JSON fields are order/whitespace independent, unknown valid
  extension values are bounded and ignored, only the two selected binaries are
  materialized, arbitrary additional regular/directory archive resources remain
  streamed/discarded, and symlink/special/traversal/duplicate/size/type/ELF
  checks remain fail-closed. Probe run `35308307480` independently confirmed
  official 0.155.0 keeps both selected files as static AArch64 ELF and preserves
  exact patch source counts `2,1,1,1`. Full repository acceptance is being
  rerun against this product source before the activated `main` producer pin
  may change. The failed candidate run reached no signing secret, Release,
  Pages, CAS, or stable mutation.
- Public stable remains signed sequence 11 generation
  `local-hosted-0-154-0-37fbbd8033b8-rald45-transition`; no RALD-7 candidate
  has advanced it yet.
- `legacy/monolith` remains sealed at
  `bf30a7dc94d4dad7f58836c69028160856e63c58`.
- Worker mode remains OFF.

## Accepted RALD-4 disposition

RALD-4 is accepted on 2026-09-17 after successful production-authority hosted
proof in run `35176798621`. The accepted path reached native ARM64 runtime smoke,
the bounded signing boundary, independent signature verification, and cleanup
without publication authority. Commit
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

The one bounded `workflow_dispatch` acceptance-only stimulus was consumed by
successful run `35176798621`. It compared fixed disposable baseline `0.153.4`
against real official stable `0.154.0` only after independently authenticating
public stable as exact `0.154.0`; public stable was not lowered or rewritten.
The run used the real official archive/digest, Android/AArch64 build and native
smoke, sequence derived from authenticated public state, accepted-authority
match, production signing, independent verification, and cleanup. Its signed
index used the non-routable `.invalid` release base and signed output was deleted
in-job rather than uploaded. Scheduled and ordinary manual runs retain the real-
public-stable comparison. The secret value was not requested or exposed. RALD-5
publication authority remains absent, public Release/Pages/index state remains
protected, and RALD-5 has not started. The bounded acceptance-stimulus amendment
is repository-green: RALD-3 preflight 6/6,
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
Run `35163080182` then passed that entire boot/ABI gate and reached the exact
Manager/Core/runtime step, but its first adb command saw `device offline`: the
boot step had left `trap cleanup_emulator EXIT` armed, so successful step exit
killed the adb server and emulator wrapper before the next step. The selected
repair preserves that trap for boot failures only, writes the qualified emulator
PID before `trap - EXIT`, performs a bounded device-state recheck at the smoke
boundary, and adds an `always()` cleanup step after qualification/upload. Because
this smoke job now runs on Ubuntu, its qualified-archive bound also uses GNU
`stat -c` instead of the stale macOS `stat -f`. The signing job was skipped and
the production secret was not injected. That emulator path is superseded by the
native ARM64 substrate; exact ARM64 execution, secret-backed signing, and
independent verification are accepted above in run `35176798621`. RALD-5 remains
not started.

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
RALD-4 was not started by that RALD-3 run and is accepted separately above.

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
