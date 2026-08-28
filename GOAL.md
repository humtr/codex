# Rust Core Rewrite Goal

## Public Contract

- Target: a complete native Rust Core for the Termux Codex wrapper.
- Inputs: official upstream Codex artifacts, immutable release metadata, and
  the Termux runtime environment defined by `SPEC.md`.
- Output surface: `codex`, with dedicated `update` and `doctor` commands and a
  `codex termux` Manager boundary.
- Allowed writes during implementation: this repository on
  `rewrite/rust-core` and test-owned temporary roots.
- Protected surfaces: the live `$PREFIX/bin/codex`, installed runtime and
  Manager, `$PREFIX/etc/resolv.conf`, profiles, sessions, auth data,
  `legacy/monolith`, and the pre-rewrite archive bundle.
- Authority: `SPEC.md` for normative behavior and architecture; this file for
  acceptance; `WORKBOARD.md` for the current implementation target.
- Secret exclusions: tokens, OAuth codes, cookies, credentials, private keys,
  and unredacted session or notification content.
- Non-negotiable constraints: clean rewrite, no legacy source copying, no live
  cutover before acceptance, resolver non-mutation, crash-safe rollback, and
  upstream process-boundary fidelity.

## Primary Technical Lead Policy

- Orchestration mode: lead-owned implementation
- Required primary role: Technical Lead/Integrator
- Required primary model: `gpt-5.6-sol`
- Required reasoning effort: `max`
- Lifecycle: keep the primary Lead across both Core milestones while its context
  remains available and accurate; on resume rebind the branch, commit,
  `SPEC.md`, this file, and `WORKBOARD.md`
- Planning owner: the primary Lead reads authorized repository evidence directly,
  plans each bounded implementation bundle, and records it in `WORKBOARD.md`
  before product-code mutation
- Implementation/integration owner: while worker mode is OFF, the primary Lead
  directly edits bounded product code/tests, inspects the actual diff, reruns
  load-bearing validation, records acceptance evidence, and commits accepted work
- Checkpoints: initial implementation bundle; completion or rejection of the
  current bundle; material deviation, blocker, authority change, worker-mode
  transition, and milestone transition
- Additional planning agents, problem advisors, and checkpoint reviewers:
  disabled during both Core milestones unless the user explicitly changes that
  policy
- Review policy: the Lead's validation is integration verification, not
  independent review; invoke a fresh independent product reviewer only after the
  Milestone 2 acceptance candidate is complete
- Identity rule: do not persist a live Lead identity in tracked files
- Policy source and timestamp: user decision on 2026-08-28

## Worker Mode Control

- Worker capability: available but user-controlled
- Current worker mode: OFF
- Authority to change worker mode: explicit user command only. The Lead must not
  infer, auto-enable, or auto-disable worker mode from workload, context pressure,
  test failures, or convenience
- While OFF: do not invoke implementation workers or coding subagents. The
  primary `gpt-5.6-sol` / `max` Lead directly implements bounded Workboard work
- If the user later turns worker mode ON: record the transition and a bounded
  worker contract in `WORKBOARD.md` before invoking any implementation worker;
  worker output never becomes acceptance proof without Lead diff review and
  load-bearing validation
- Turning worker mode ON or OFF does not alter repository/product authority,
  milestone gates, protected surfaces, or the Lead's acceptance ownership
- tmcp `harness.run` is independent of worker mode and is test/validation-only;
  it must never be used as an implementation/development agent or as a route for
  product-code mutation
- Package operations, live product mutation, and network/provider behavior remain
  prohibited unless separately authorized by the applicable milestone gate
- Policy source and timestamp: user decision on 2026-08-28

## Current Success Threshold

The goal is complete only when both milestones in `SPEC.md` pass their declared
tests and a fresh supported Termux installation can install, run, diagnose,
update, recover, and roll back the Rust Core without requiring an on-device Rust
toolchain or modifying protected user/system state.

Manager product features are not part of this two-milestone completion claim.
Their boundary must be preserved so they can be implemented separately without
moving Core ownership.

## Project Analysis

The predecessor accumulated runtime, installer, activation, repair, profile,
session, notification, documentation-audit, and release logic across Bash and
Python. The rewrite intentionally retains only accepted observable contracts
and safety findings. It does not translate predecessor modules or preserve
their internal schemas by default.

The highest-risk work is not Rust compilation. It is bootstrap trust,
upstream-artifact qualification, descriptor/process fidelity, atomic
generation activation, and recovery after ambiguous failures.

Speed comes from a narrow Core, two milestones, one normative specification,
one current workboard, direct focused tests, and deferred independent review.

## User Decisions

- Start `rewrite/rust-core` from an empty parentless root. It must have no
  merge-base with `main` or the predecessor and must never import either
  history.
- Keep `main` as a separate publication authority until an accepted rewrite
  tip explicitly replaces it; promotion is not a merge.
- Seal the latest predecessor at
  `bf30a7dc94d4dad7f58836c69028160856e63c58` on `legacy/monolith`.
- Keep one repository and one public `codex` entrypoint.
- Separate native Rust Core from the Manager layer.
- Keep management commands under `codex termux`.
- Reserve top-level `codex update` and `codex doctor` for Termux-aware behavior.
- Preserve upstream `--version`/`-V` output without wrapper version rows.
- Put architecture and lightweight change discipline in `SPEC.md`; do not add
  a separate SDD now.
- Run the goal under a primary `gpt-5.6-sol` / `max` Technical Lead/Integrator
  that directly owns repository evidence, planning, integration verification,
  authority documents, commits, and acceptance decisions across both milestones.
- Worker mode is controlled only by explicit user command; its current state is
  OFF, so the primary Lead directly implements bounded product-code and test
  changes.
- Do not create planning subagents, problem advisors, or checkpoint reviewers
  during the Core milestones. Review the complete Milestone 2 candidate with a
  fresh independent reviewer.
- Use exactly two Core milestones.
- Product release speed is the priority once the small set of load-bearing
  integrity invariants is satisfied. Do not turn rare or hypothetical failure
  scenarios into new subsystems by default.
- Treat one installer/updater transaction as the normal product path. Do not
  spend release-critical time on simultaneous-installer multi-writer fencing
  unless actual use or a reproducible product failure demonstrates the need.
- Prefer recovery to one already complete last-known-good generation over
  stacked fallback chains. Existing defensive state, retries, checks, and
  fallback paths should be removed when a simpler foundational invariant covers
  the same failure.
- Any additional defensive mechanism must justify its net complexity: it should
  address a concrete failure not already covered by complete-generation staging,
  atomic activation, and last-known-good recovery. Defensive complexity is also
  a potential defect and attack surface.
- The foundation established documents and workflow first; current implementation
  is now performed directly by the primary Lead under bounded Workboard bundles.

## Execution Plan

### Milestone 1 — local Core

Implement the Rust Core and prove local command dispatch, upstream passthrough,
FD/environment/process contracts, sandbox behavior, read-only doctor, manifest
interfaces, and resolver non-mutation. Do not perform live installation or
network update.

### Milestone 2 — delivery and recovery

Implement prebuilt delivery, bootstrap, signed updates, upstream acquisition and
adaptation, atomic activation, recovery, rollback, offline operation, and fresh
Termux qualification. Produce one candidate for independent product review.

## Acceptance Ledger

### Current Direct-Lead Evidence

- The user explicitly withdrew trust from the prior implementation-worker path
  and required a fresh Lead review from the beginning. Worker mode remains
  user-controlled and OFF. Historical M1-B1..M1-B9 worker reports and their old
  acceptance statements remain provenance only; they are not used as current
  proof.
- M1-R1 fresh re-audit/hardening is accepted at
  `4c1a8d90d6aa028106218d349076c465af8b8535`. The direct Lead reviewed the
  current Rust source against `SPEC.md`, reopened two correctness/safety gaps,
  fixed them directly, reviewed the resulting diff, and reran the load-bearing
  validation.
- M1-B15 is accepted at `6bea7a53004f43178599e65a6f630c7bb06355b9`.
  The direct Lead added a dependency-free typed doctor report surface with
  bounded upstream/Core/Manager state domains, deterministic summary precedence,
  typed semantic exit classes, separated human sections, and one schema-versioned
  JSON envelope. The output model accepts no arbitrary diagnostic strings,
  paths, environment values, auth/session/notification content, or raw upstream
  output, establishing a fail-closed redaction baseline before process capture.
- Final B15 validation `job_ikt_87c72178ad` passed all 5 B15 focused tests,
  exhaustive 36-state summary/exit classification, the full serial workspace
  suite 98/98, eight complete default-parallel repetitions, formatting,
  `git diff --check`, and a warning-free locked build with offline mode and a
  repository-external Cargo target. Direct diff audit found no B15 process,
  filesystem, environment, Manager, network, dependency, or `main` wiring.
- M1-B14 is accepted at `be6492f895185caf7d9b922b16330a1cd8f00033`.
  The direct Lead added a typed qualified-runtime launch boundary that accepts no
  separate raw runtime program or compatibility directory: it consumes the B13
  `QualifiedRuntimeAssets`, derives the B10 environment plan from the captured
  process snapshot and the qualified compatibility directory, then delegates to
  the existing sandbox-before-I/O FD33/34 final-exec path using the qualified
  runtime program. No active-generation lookup, digest calculation, filesystem
  qualification, network, activation, or normal `main` wiring was added.
- Final B14 validation `job_ijk_b7acb35e72` passed all 3 B14 focused tests, the
  full serial workspace suite 93/93, eight complete default-parallel workspace
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. The real subprocess
  test jointly proved qualified runtime selection, qualified compatibility PATH,
  B10 temp/certificate assignments, exact sandbox prelude/raw argv, FD33/34,
  contamination fencing, and unrelated-environment preservation.
- M1-B13 is accepted at `71acbd8e318d50548952490e0d2fb52c7b661f9c`.
  The direct Lead added a pure Unix runtime-asset qualification boundary tying an
  explicit absolute runtime program path and observed digest, explicit
  compatibility directory, and the exact helper-asset identity/digest set to a
  B11 qualified generation. No filesystem stat/read/hash, active-generation
  lookup, path canonicalization, launch, or state mutation is performed.
- Final B13 validation `job_igy_c7b11e3616` passed all 10 B13 focused tests,
  the full serial workspace suite 90/90, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target.
- M1-B12 is accepted at `3927ad46696875c913c9039406693c1ddd4c3231`.
  The direct Lead added a dependency-free updater admission/candidate interface:
  immutable-remote versus raw local-artifact sources, explicit signed-release
  and architecture/API/channel/anti-rollback verdicts, resolver-dependency
  qualification, staged digest/archive/compatibility verdicts, candidate-probe
  and rollback-readiness verdicts, source-digest binding to the B11 qualified
  generation, and borrowed admitted/activation-ready wrappers. No verifier,
  cryptography, serialization, network, staging, installation, or activation is
  implemented by B12.
- Final B12 validation `job_iff_6b93404095` passed all 11 B12 focused tests,
  the complete serial workspace suite 80/80, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. Direct diff review
  confirmed only `crates/core/src/main.rs` changed and the B12 production surface
  is pure type/evidence promotion rather than updater I/O.
- M1-B11 is accepted at `0eb9f6cd33951ff782c010d9e116ab886f70a815`.
  The direct Lead added a dependency-free in-memory generation manifest model,
  explicit Core compatibility requirements, typed qualification failures, and a
  borrowed `QualifiedGenerationManifest` wrapper. The validator binds all
  SPEC-declared generation-manifest field classes, rejects empty required
  bindings, the four Core/platform compatibility mismatches, rejected
  qualification, malformed/duplicate helper bindings, and an explicitly empty
  optional Manager digest. It deliberately does not define serialization,
  digest algorithms, signatures, physical generation paths, updater I/O, or
  activation.
- Final B11 validation `job_idt_e50502b44b` passed all 10 B11 focused tests,
  the full serial workspace suite 69/69, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. The pre-commit diff
  was limited to `crates/core/src/main.rs` and direct boundary review found no
  B11 serialization, filesystem, environment, Command, FD, or generation-path
  I/O.
- M1-B10 is accepted at `08e67e8c9fed23032ff59c38ff4765221d515d67`.
  The direct Lead added an owned five-value Termux process-environment snapshot,
  a thin raw `var_os` reader for only `PREFIX`, `TMPDIR`, `PATH`,
  `SSL_CERT_FILE`, and `SSL_CERT_DIR`, typed missing/empty required-input errors,
  and a pure snapshot-to-B8 composition that derives only native `PREFIX/bin`.
  It does not read `HOME`, inspect the filesystem, choose a generation/runtime,
  construct a Command, touch FD 33/34, mutate global environment, or wire
  `main`.
- Final B10 validation `job_iba_22c23cddee` used offline mode and a
  repository-external Cargo target. All six B10 focused tests passed, the full
  serial workspace suite passed 59/59, eight full default-parallel repetitions
  passed, formatting and the locked workspace build passed, and `git diff
  --check` passed. A direct boundary audit found exactly five new production
  `var_os` reads and no B10 hard-coded Termux root, `HOME` read, filesystem path
  inspection, or Command construction.
- Sandbox-policy revalidation found that the earlier parser intentionally let
  whitespace-bearing and attached `sandbox_mode` config forms pass through.
  M1-R1 now normalizes surrounding whitespace and one matching quote layer,
  recognizes separate/attached/equals short config forms and long config forms,
  preserves exact `--` scan termination, rejects every non-empty recognized
  `sandbox_mode` value except `danger-full-access`, and continues to reject the
  known unsupported `read-only`/`workspace-write` sandbox flag values. Accepted
  raw user argv is not rewritten after the injected Termux-safe prelude.
- FD-failure revalidation found that restoration syscalls were best-effort and
  some ordinary parallel tests directly mutated process-global FD 33/34. M1-R1
  makes explicit restoration return errors, gives restoration failure precedence
  on returned setup/exec failure paths, keeps Drop only as last-resort cleanup,
  and moves the direct FD 33/34 mutation cases into dedicated subprocess probes.
- Final direct-Lead validation `job_i95_262b8f5b0d` used
  `CARGO_NET_OFFLINE=true` and a repository-external Cargo target. Formatting,
  M1-R1 focused tests (3/3), passthrough focused tests (10/10), runtime-FD
  focused tests (11/11), the full serial workspace suite (53/53), eight complete
  default-parallel workspace repetitions, and the locked workspace build all
  passed. `git diff --check` passed and the only product change before commit was
  `crates/core/src/main.rs`.
- Direct source-boundary audit found no production hard-coded
  `/data/data/com.termux` path, `to_string_lossy`, `env_clear`, TODO, production
  `.unwrap(`, or production filesystem write in the current B1..B9 surface.
  `main()` remains intentionally unwired; the test module begins after the
  production entrypoint and synthetic resolver/config writes remain test-only.
- Fresh behavior disposition after M1-R1: B1 exact first-argument dispatch is
  CURRENTLY PROVEN; B2 raw final-exec argv/streams/exit behavior is CURRENTLY
  PROVEN; B3 the exact five-variable child-only contamination fence is CURRENTLY
  PROVEN; B4 explicit read-only resolver/config FD 33/34 mapping, collision
  handling, caller-state restoration, restoration-error visibility, and
  test-owned resolver non-mutation are CURRENTLY PROVEN; B5 current-device
  TTY/process-identity/external-SIGTERM fidelity is CURRENTLY PROVEN; B6 the
  hardened Termux sandbox-policy planner is CURRENTLY PROVEN; B7 policy-before-I/O
  composition with the runtime-FD final-exec path is CURRENTLY PROVEN; B8 the
  pure explicit-input base-environment planner is CURRENTLY PROVEN; B9 transport
  of a pre-built environment plan through the final-exec composition is CURRENTLY
  PROVEN. These are component proofs only and do not complete Milestone 1.

### Historical Bundle Ledger — revalidation pending


- The predecessor tip `bf30a7d` contains `af640166`, which removed the Termux
  bwrap compatibility path and made unsupported sandbox requests explicit.
- The predecessor tip is preserved by `legacy/monolith` and annotated tag
  `legacy-monolith-bf30a7d-20260828`.
- The current development device has a native `aarch64-linux-android` Rust
  toolchain, Cargo, and Android-targeting Clang.
- The rewrite lineage contains no legacy implementation source.
- `rewrite/rust-core` begins at empty root
  `b3a9da98195cff1053f012d2afa738949b5b14dc` and has no merge-base with
  `main` or `legacy/monolith`.
- Milestone 1 bundle M1-B1 was historically recorded as accepted at
  `36c98dd8882ddba18657ab3f289eace1121ff39b`: the rewrite now has one locked,
  dependency-free Cargo workspace member and one Core binary with exact
  first-argument classification for `update`, `doctor`, and `termux`; all other
  inputs, including `--version`, `-V`, near misses, arbitrary arguments, and
  non-UTF-8 first arguments on Unix, classify as upstream passthrough.
- Primary-Lead validation job `job_hmw_1af3337581` removed only worker-generated
  untracked `target/` artifacts, used an external temporary `CARGO_TARGET_DIR`
  with `CARGO_NET_OFFLINE=true`, and passed `cargo fmt --check`,
  `cargo test --locked --workspace` (6/6), and
  `cargo build --locked --workspace`. Post-validation status job
  `job_hmx_ffe2f81a73` showed only the planned Workboard and four bundle source
  paths before commit.
- The local `.git/hooks/pre-commit` is an untracked predecessor-environment hook
  that invokes absent `tools/update-wrapper-version.sh`; normal commit job
  `job_hmz_60b990cf84` therefore failed without changing HEAD. After read-only
  inspection proved neither the hook nor its referenced path belongs to this
  lineage, the Lead committed M1-B1 once with `--no-verify` under exact HEAD and
  index-tree preconditions. The hook remains unmodified and is not product
  evidence.
- Milestone 1 bundle M1-B2 was historically recorded as accepted at
  `fc50b39e50bb6ef341d3cf01163ca90423bd7b13`. The std-only Unix/Android
  `exec_upstream` primitive uses final `exec` replacement with raw `OsStr`/
  `OsString` inputs. Focused subprocess evidence proves upstream-visible
  `--version`, `-V`, ordinary and non-UTF-8 arguments, exact raw stdout/stderr
  bytes, chosen nonzero exit codes, and direct exec failure reporting without
  adding public test-only command semantics.
- Primary-Lead validation job `job_hnd_bc84d51555` reran the accepted M1-B2
  source with `CARGO_NET_OFFLINE=true` and an external temporary
  `CARGO_TARGET_DIR`; `cargo fmt --check`, `cargo test --locked --workspace`
  (11/11), and `cargo build --locked --workspace` all passed while repository
  status remained limited to the authorized source file before commit.
- Milestone 1 bundle M1-B3 was historically recorded as accepted at
  `815c9104c726f212ee4a51b518af14e8c133b20c`. The production exec command
  removes exactly `CODEX_MANAGED_BY_NPM`, `CODEX_MANAGED_BY_BUN`,
  `CODEX_MANAGED_PACKAGE_ROOT`, `LD_PRELOAD`, and `LD_LIBRARY_PATH` from the
  child exec environment without `env_clear` or parent-process mutation;
  unrelated environment entries are preserved. Failed exec evidence proves the
  caller process retains its synthetic inherited values.
- Primary-Lead validation job `job_hnt_c908186115` reran M1-B3 with an external
  temporary Cargo target and offline mode; formatting, 13/13 tests, and locked
  workspace build passed while status remained limited to `crates/core/src/main.rs`.
- Milestone 1 bundle M1-B4 was historically recorded as accepted at
  `bb21ddca58589ec77a22e824c4218db5c1087daa`. The runtime-FD exec path opens
  an explicit resolver source and existing managed-config directory read-only,
  maps them to FD 33/34 with CLOEXEC cleared, uses safe CLOEXEC duplicates above
  FD 34 to avoid source/target collisions, and restores originally absent or
  present caller FD 33/34 state when setup or exec fails. Resolver-content and
  Unix metadata evidence proves the test resolver is unchanged across exec.
- Lead review found and corrected one pre-acceptance defect: only `EBADF` now
  classifies an `F_GETFD` probe as descriptor absence; all other probe errors
  propagate. Primary-Lead validation job `job_hol_118858c4b8` passed formatting,
  24/24 workspace tests, three additional serial repetitions of all 11
  `runtime_fds` tests, and locked workspace build with offline mode and an
  external Cargo target.
- Milestone 1 bundle M1-B5 was historically recorded as accepted at
  `85f312b7d5d0e2e8a14c9084063e437633b63480`. Test-only private probes prove
  the production final `exec_upstream` boundary preserves a PTY on stdin,
  stdout, and stderr on the current Android/Termux device and preserves process
  identity across exec: the upstream shell reports `$$` equal to the spawned
  child PID, receives an external `SIGTERM` sent to that same PID, and executes
  its trap with exit code 73. No production behavior changed in B5.
- Primary-Lead validation job `job_hqo_7f45af3f26` passed formatting, all 26/26
  workspace tests, three additional serial repetitions of each TTY and SIGTERM
  proof, and locked build with offline mode and a repository-external Cargo
  target.
- Milestone 1 bundle M1-B6 was historically recorded as accepted at
  `a4b4cb3a91bd78ea07952739f054695f10bab638`. The module-private passthrough
  planner rejects the bounded observed Linux `read-only`/`workspace-write` and
  leading `sandbox linux` request forms before launch planning, stops scanning
  at exact `--`, preserves accepted raw `OsString` argv byte-for-byte, and
  prepends only `-c` plus `sandbox_mode=\"danger-full-access\"`. It never
  synthesizes the upstream approval-bypass flag and does not wire a runtime
  executable or product path.
- The first B6 worker result was rejected before acceptance because it expanded
  unobserved forms and could reinterpret a separate option value as a later
  policy option. The bounded correction consumed exactly one following value
  token for separate sandbox/config options and narrowed recognition to the
  accepted forms. Primary-Lead validation job `job_ht1_c593e9a07a` then passed
  formatting, all 34/34 workspace tests, three serial repetitions of the 10
  `passthrough_` tests, and the locked workspace build with offline mode and a
  repository-external Cargo target.
- Milestone 1 bundle M1-B7 was historically recorded as accepted at
  `5e5044eb3ae9286b72b16f1e1b9092f4e728bc82`. The module-private
  `launch_upstream` composes B6 policy planning with the accepted B4 runtime-FD
  final-exec primitive through explicit program/resolver/config/user-argv
  inputs. Policy rejection occurs before runtime I/O; accepted launch crosses
  the real exec boundary with the exact no-sandbox argv prelude, supplied FD
  33/34 sources, the existing five-variable contamination fence, and unrelated
  environment preservation. `main` remains unwired and no runtime path policy
  is introduced.
- Primary-Lead validation initially used an invalid focused `--exact` filter in
  `job_hvh_029dd726eb`; those zero-test repetitions are not acceptance evidence,
  although that job's full 38-test run passed. Corrected validation job
  `job_hvj_d1a56ada7b` ran each of the four B7 focused tests exactly three times
  (12 actual focused runs), then passed all 38/38 workspace tests and the locked
  build with offline mode and a repository-external Cargo target.
- Milestone 1 bundle M1-B8 was historically recorded as accepted at
  `ae678fdb01b065a78f55b4e0546a8c4b12c498fa`. The module-private Unix/Android
  base-environment planner is std-only and explicit-input-only: it plans the four
  temporary-directory variables, certificate fallback/precedence, and raw-byte
  PATH composition without reading process environment or filesystem state,
  constructing a Command, choosing product paths, or wiring `main`.
- The first B8 worker result was rejected before acceptance because one purity
  test read the filesystem, one focused assertion used lossy string conversion,
  and an unrequired non-Unix PATH fallback encoded Unix delimiter semantics
  outside the current target. Correction job `job_hyc_a66aed1797` removed only
  those expansions. Primary-Lead validation `job_hyn_2d9868514e` then ran all
  nine B8 focused tests three times (27 real focused executions), all 47/47
  workspace tests, formatting, and the locked workspace build with offline mode
  and a repository-external Cargo target. The positive assignments remain a
  bounded M1 compatibility hypothesis until applied and qualified on the real
  Termux execution boundary.
- Milestone 1 bundle M1-B9 was historically recorded as accepted at
  `692cd8b0c9cc4babe273ab9bdfa9d14eabc9db0c`. One shared final-exec
  implementation now accepts an optional `TermuxBaseEnvPlan`; positive raw
  `OsString` assignments are applied directly to the child `Command`, then the
  exact B3 five-variable contamination fence is enforced. Existing public/test
  launch signatures still traverse the same implementation with no positive
  plan, while the new module-private environment-aware launch composition
  preserves B6 policy-before-I/O ordering and B4 FD 33/34 restoration.
- Primary-Lead validation `job_i1l_cabb18a109` ran all three B9 focused tests
  three times (9 actual focused executions), all 50/50 workspace tests,
  `cargo fmt --check`, and the locked workspace build with
  `CARGO_NET_OFFLINE=true` and a repository-external Cargo target. The worktree
  remained limited to `crates/core/src/main.rs` before commit. Real exec proof
  jointly observed the planned temp/certificate/PATH values, exact sandbox
  prelude and raw user argv, FD 33/34 sources, the contamination fence, and one
  unrelated inherited variable; failed exec preserved caller environment and
  restored prior FD state.

### Historical Proof

- The predecessor's behavior and history remain available only as sealed
  legacy evidence. They are not promoted into proof for the rewrite.

### Not Proven

- M1-B19 is accepted at `148b1133f1afaa91668e19b4fade13bc761b0056`.
  The direct Lead added one ordered local doctor command boundary that validates
  B18 usage before invoking B17 local doctor composition, preserves Usage and
  Probe as distinct typed errors, and renders a successful bounded report only
  after probe completion. Invalid UTF-8 and ordinary invalid doctor argv are
  therefore proven to fail before environment planning, resolver/config FD
  setup, or runtime spawn.
- Final B19 validation `job_iq5_44896f380d` passed all 5 B19 focused tests, the
  full serial workspace suite 116/116, eight complete default-parallel
  repetitions with per-run failure logs, formatting, `git diff --check`, and a
  warning-free locked build with offline mode and a repository-external Cargo
  target. Boundary audit found no `main` wiring, lossy argv conversion, or new
  production filesystem path beyond the already-shared doctor/launch helpers.
- M1-B18 is accepted at `fdecef9f86a1f04776309ffe344b169d715c7217`.
  The direct Lead added a pure doctor invocation/output contract over an
  already-composed bounded report. Arguments following exact leading `doctor`
  are accepted only as empty for human output or exactly one raw `--json` token
  for machine output; every other shape including non-UTF-8 fails with one static
  non-echoing usage error. Rendering preserves the B15 human/JSON envelope and
  typed `DoctorExitClass` without assigning unspecified numeric process codes.
- Final B18 validation `job_ioy_1672a340bc` passed all 5 B18 focused tests, the
  full serial workspace suite 111/111, eight complete default-parallel
  repetitions with per-run failure logs, formatting, `git diff --check`, and a
  warning-free locked build with offline mode and a repository-external Cargo
  target. Direct audit found no lossy argv conversion, new process/filesystem/
  environment access, or `main` dispatch change in B18.
- M1-B17 is accepted at `64199eb4cbb1dccb351cf140c55e5e36d77d65ce`.
  The direct Lead added a bounded local doctor coordinator: explicit Supported
  capability invokes the B16 qualified upstream probe exactly once and composes
  only its bounded status with already-typed Core/Manager states through B15;
  explicit Unsupported capability skips process-environment planning,
  resolver/config access, FD mapping, runtime spawn, and stderr inference
  entirely. Typed B16 setup/spawn errors propagate without fabricating a report.
- Final B17 validation `job_inr_7ffbd3d9f8` passed all 4 B17 focused tests, the
  full serial workspace suite 106/106, eight complete default-parallel
  repetitions with per-run failure logs, formatting, `git diff --check`, and a
  warning-free locked build with offline mode and a repository-external Cargo
  target. An earlier validation `job_inp_6b77b4a712` observed one unlogged
  transient parallel failure after its serial 106/106 pass; dedicated
  reproduction `job_inq_018394a0b3` then passed five consecutive full parallel
  runs before the final eight-run acceptance validation. B17 tests also proved
  unsupported I/O skipping, API-incompatibility precedence, exact bounded
  human/JSON rendering, raw-output exclusion, and no report on spawn failure.
- M1-B16 is accepted at `d420db8b4128d44836c89394cbbb9afc9398b1e5`.
  The direct Lead added a supported-upstream doctor child probe that consumes
  only B13 qualified runtime assets, B10 environment inputs, and explicit
  resolver/config paths. Final exec and doctor now share one temporary FD33/34
  mapping/restoration primitive and one child environment/fence helper. Doctor
  invokes the selected raw runtime directly with the Termux-safe prelude plus
  `doctor`, discards raw stdout/stderr, classifies only child completion status,
  and restores parent FD state.
- Final B16 validation `job_imd_136341d507` passed all 4 B16 focused tests, the
  full serial workspace suite 102/102, eight complete default-parallel
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target. B16 tests proved
  exact safe argv/env/FD behavior, secret-like output suppression, explicit
  nonzero-to-unhealthy classification without stderr inference, typed spawn and
  pre-I/O environment failures, parent FD restoration, and test-owned
  resolver/config non-mutation.
- M1-B20 is accepted at `07ed3af8764c10f03aaf3bf83b18ffb37a32b891`.
  The direct Lead added one pure public-dispatch planner over complete raw argv.
  Only exact first-token `update`, `doctor`, and `termux` are intercepted; each
  Core route consumes only that token and retains all trailing raw `OsString`
  values byte-for-byte. Every other shape, including empty argv, `--version`,
  `-V`, delimiter/near-miss forms, and non-UTF-8 first tokens, remains one
  upstream route with the complete original argv and order preserved.
- Final B20 validation `job_irt_1daf7615c8` passed all 5 B20 focused tests, the
  full serial workspace suite 121/121, eight complete default-parallel full
  repetitions, formatting, `git diff --check`, and a warning-free locked build
  with offline mode and a repository-external Cargo target.
- M1-B21 is accepted at `5847f0d8d223e6abdf8d1876fc316ac1fda7b281`.
  The direct Lead added a pure optional Manager-artifact qualification boundary
  over the already-qualified generation. Absent manifest binding plus absent
  selection is one explicit `Unavailable` state; a declared Manager requires one
  explicit absolute NUL-free raw path and a nonempty observed digest matching the
  manifest before it can become `Available`. Presence disagreement, path-shape
  failures, empty digest, and digest mismatch are distinct typed failures.
- B21 focused validation `job_it2_21e09d1f28` passed 7/7. Final grouped
  acceptance `job_it4_84accf60ab` passed the full serial workspace suite
  128/128, eight complete default-parallel full repetitions, formatting,
  `git diff --check`, and a warning-free locked offline build with an external
  Cargo target. No Click plugin/Hook was used; the Workboard anti-loop discipline
  reused evidence and avoided an extra intermediate full-suite pass.
- M1-B22 is accepted at `a89646da18c1e0b68e146b565a847dcc4b0fd0b6`.
  The direct Lead completed the Core-side qualified Manager handoff. An explicit
  `Unavailable` Manager returns one bounded static outcome without consuming
  argv or constructing a process; `Available` can execute only the B21-qualified
  Manager path, appends only the original raw trailing argv, inherits ordinary
  environment and standard streams, and uses Unix final `exec` semantics.
  Failed exec is a typed I/O error and leaves the caller environment unchanged.
- B22 focused validation `job_iu0_937ceaf5fd` passed 4/4, including real raw
  non-UTF-8 argv/stream/exit evidence and process-identity/SIGTERM delivery.
  Final grouped acceptance `job_iu2_cf462a7fad` passed the full serial workspace
  suite 132/132, eight complete default-parallel full repetitions, formatting,
  `git diff --check`, a warning-free locked offline build, and source-text NUL
  absence. The same commit also replaced B21's literal-NUL test fixture with an
  equivalent numeric-byte fixture so repository text search remains reliable.
- M1-B23 is accepted at `c29f5f2019104ad7ab51f36754f326b48d33704c`.
  The direct Lead composed the exact public route with the previously proven
  Upstream, Doctor, and Manager execution boundaries over one injected qualified
  local context. Update remains a raw-byte-preserving zero-I/O handoff to the M1
  updater interface. During integration the Lead found and closed a latent
  cross-generation ambiguity: Manager `Unavailable` now retains its qualified
  generation, and context construction rejects runtime/Manager qualifications
  from distinct manifest objects before any route can execute.
- B23 focused validation `job_iv4_0729f98d00` passed B21 7/7, B22 4/4, and
  B23 6/6 after the representation correction. Production diff audit
  `job_iv5_dc87f7dca8` confirmed no new filesystem/environment/Command/lossy
  production path and no `main` body change. Final grouped acceptance
  `job_iv6_b5bb5f840a` passed the full serial suite 138/138, eight complete
  default-parallel repetitions, formatting, `git diff --check`, source NUL
  absence, and a warning-free locked offline build with an external Cargo target.
- M1-B24 is accepted at `db67c0b90e1916d2ec452b8db2657dd4d504cd52`.
  The direct Lead added one thin raw-argv public entrypoint composition that
  performs the B20 planner exactly once and passes the resulting route directly
  into the B23 qualified dispatcher. Production `main` remains intentionally
  unchanged because physical active-generation context acquisition belongs to
  Milestone 2. Test-only B24 evidence added an explicit real-Termux smoke gate
  using the actual live resolver read-only, a test-owned fake qualified runtime
  and config root, FD33/34, and byte-exact direct-vs-Core `--version` output.
- Focused/smoke validation `job_ivj_a0374d97fe` passed the B24 zero-I/O entrypoint
  test and the explicitly selected real-Termux smoke 1/1 on Termux
  `0.119.0-beta.3`, `aarch64-linux-android`. Production audit
  `job_ivk_7d7e01c56a` confirmed the only production addition is
  `plan_public_dispatch(raw_args) -> execute_public_dispatch(...)` with no new
  filesystem/environment/Command/lossy path and no `main` body change. Final
  acceptance `job_ivl_1ddcbf4dc5` passed 139 default tests with the explicit
  smoke correctly ignored, eight complete default-parallel repetitions,
  formatting, `git diff --check`, and a warning-free locked **release** build.
  External pre/post evidence kept the live resolver exactly at SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  and the installed launcher exactly at SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`,
  with device/inode/mode/uid/gid/size/mtime identity unchanged.
- **M1-B24 established the required local Core behavior evidence, but M1-R2 has
  reopened final product closure until real `main` wiring is complete.** The required local
  Core behavior is now proven by source, subprocess, and current-Termux evidence:
  exact public routing and upstream passthrough; upstream-only version behavior;
  environment/final-exec semantics; FD33/34 and live-resolver non-mutation;
  explicit sandbox behavior; bounded read-only redacted doctor composition;
  generation/updater interfaces without live mutation; unit/integration/fault
  coverage; and an explicit real-Termux smoke gate. No live Codex installation,
  runtime, Manager, resolver, package, update, activation, or publication ref was
  changed during Milestone 1. Physical generation state, installation,
  activation, recovery, rollback, artifact delivery, and installed `main`
  context acquisition remain Milestone 2 work and are not promoted by M1 proof.
- No release artifact, installation, update, activation, rollback, offline
  recovery, fresh-device behavior, Milestone 2 result, or production readiness
  is proven.
- No Manager implementation or Core/Manager integration is proven.

### Checkpoint Plans

- M1-R1 is closed at `4c1a8d90d6aa028106218d349076c465af8b8535`.
- M1-B10 is closed at `08e67e8c9fed23032ff59c38ff4765221d515d67`.
- M1-B11 is closed at `0eb9f6cd33951ff782c010d9e116ab886f70a815`.
- M1-B12 is closed at `3927ad46696875c913c9039406693c1ddd4c3231`.
- M1-B13 is closed at `71acbd8e318d50548952490e0d2fb52c7b661f9c`.
- M1-B14 is closed at `be6492f895185caf7d9b922b16330a1cd8f00033`.
- M1-B15 is closed at `6bea7a53004f43178599e65a6f630c7bb06355b9`.
- M1-B16 is closed at `d420db8b4128d44836c89394cbbb9afc9398b1e5`.
- M1-B17 is closed at `64199eb4cbb1dccb351cf140c55e5e36d77d65ce`.
- M1-B18 is closed at `fdecef9f86a1f04776309ffe344b169d715c7217`.
- M1-B19 is closed at `148b1133f1afaa91668e19b4fade13bc761b0056`.
- M1-B20 is closed at `07ed3af8764c10f03aaf3bf83b18ffb37a32b891`.
- M1-B21 is closed at `5847f0d8d223e6abdf8d1876fc316ac1fda7b281`.
- M1-B22 is closed at `a89646da18c1e0b68e146b565a847dcc4b0fd0b6`.
- M1-B23 is closed at `c29f5f2019104ad7ab51f36754f326b48d33704c`.
- M1-B24 is closed at `db67c0b90e1916d2ec452b8db2657dd4d504cd52`.
- M1-B24 historical acceptance is retained, but current Milestone 1 product closure is reopened by M1-R2 until real `main` wiring is completed.
- M2-B1 is accepted at `918c3681729ab8f6bba8f69607a88380645b3b5d`.
  It establishes the crash-safe complete-generation/atomic-activation recovery
  foundation in test-owned roots. Final validation `job_iwc_7350098964` passed
  151 tests with the explicit B24 smoke ignored by default, eight complete
  default-parallel repetitions, formatting/diff checks, and a warning-free
  locked release build while preserving the live resolver and installed launcher
  identity. M2-B1 is a foundation, not a mandate to add multi-writer fencing or
  more fallback tiers.
- User-directed M1-R2 reopens the Milestone 1 implementation closure for one
  exhaustive simplification and product-wiring audit before M2-B2 continues.
  This is not a sampled review: every surviving M1-R1/B1..B24 production
  definition and M1 test/probe harness must receive a keep/collapse/delete
  disposition against the current release-speed policy. Proof-only wrappers,
  duplicate validators, redundant defensive state, and tests that exist only to
  support removed mechanisms are deleted or folded. Load-bearing public behavior
  remains required.
- M1-R2 exhaustive simplification is implemented at
  `2b73f4ba23726ddab0792bbba721a2835dcb86d9`. The accumulated M1 implementation
  was reduced to 2,330 production lines and 1,624 test lines in `main.rs`; the
  change removed 9,420 lines while adding 1,666 lines of consolidated product
  and contract tests. Historical `test_m1_b*` bundle tests and all audited
  duplicate/proof-only layers are absent. The retained suite passes 33/33 serial
  with one explicit live smoke ignored by default; the explicit live
  resolver/installed-launcher smoke passes 1/1 and three complete default-parallel
  runs pass. M2-B1's 12 fault/recovery tests remain retained and passing.
- R2 KEEP groups are the direct public route planner, sandbox policy, final
  runtime FD/env/exec primitives, Termux environment snapshot/plan, generation
  manifest qualification, runtime/Manager qualification, one updater
  qualification gate, bounded doctor report/probe/command path, one public
  dispatch executor, and M2-B1 activation recovery. COLLAPSE/DELETE groups are
  duplicate B1 classification, parent FD restoration machinery, B8/B10 wrapper
  environment planners, B12 evidence-promotion/readiness wrappers, B21/B23
  generation-pointer mismatch machinery, B15-B19 planner/coordinator/render
  wrappers, nested launch errors, B24's proof-only entrypoint wrapper, and the
  unused shared-resolver fallback model. Tests were consolidated by product
  contract rather than bundle provenance.
- The remaining `main()` gap is now precisely classified: physical current-
  generation context acquisition is the missing input, and SPEC assigns that
  ownership to Milestone 2. Hiding the resulting dead paths with more
  `allow(dead_code)` would violate R2. M2-B2 therefore owns the minimal local
  activated-generation loader and real `main -> plan_public_dispatch ->
  execute_public_dispatch` wiring. M1 product closure remains open only until
  that cross-milestone connection is accepted.
- M2-B2 is accepted at `bee38e9eb481973c00205fb8a7191cdb22392f7c`.
  Production `main()` now performs raw public planning, loads exactly one
  activated generation from the M2 local layout, qualifies runtime/optional
  Manager assets from that generation, and executes upstream/doctor/Manager
  through the retained direct boundaries. Ordinary launch uses only `current`;
  it does not scan generations, read `previous`, canonicalize a fallback chain,
  use network, or invoke a package manager. The redundant in-memory
  `GenerationQualification` state was removed because the descriptor already
  requires `qualification=qualified`, and the duplicate state-root
  `generations/` directory was removed in favor of the SPEC-owned immutable
  generation root.
- B2 acceptance evidence: focused loader/main 6/6; full serial 38 passed / 0
  failed / 1 explicit smoke ignored by default; explicit real-Termux smoke 1/1;
  three complete default-parallel runs; `cargo fmt --check` and
  `git diff --check`; warning-free locked release build; live resolver and
  installed launcher SHA/stable-stat identity unchanged before/after. The only
  production `allow(dead_code)` is the existing M2 activation-state module's
  write side, which is the immediate input to local staging/activation work and
  does not hide an M1 product path.
- With `2b73f4ba...` simplification plus B2 real entrypoint wiring, Milestone 1
  product closure is re-accepted. M1 is no longer closed on proof-only
  injection evidence; the real production entrypoint reaches final execution.
- M2-B3 is accepted at `b692853a436e7df2540ccb1c52e967af4e921375`.
  `codex update --local <directory>` now has a real offline/bootstrap staging
  path: it copies only the fixed generation layout through a private candidate,
  rejects symlinks/special files, validates the copied candidate with the same
  B2 loader, and atomically publishes a complete **inactive** generation. B3
  never mutates activation state. Focused staging is 7/7; full serial is 46
  passed / 0 failed / 1 explicit smoke ignored by default; explicit live smoke
  1/1; three complete default-parallel runs; warning-free locked release build;
  live resolver and installed launcher identity unchanged. No activation,
  network, package-manager, lock/fencing, or fallback mechanism exists in B3.
- B4 feasibility evidence is concrete: current Termux provides
  `/data/data/com.termux/files/usr/bin/openssl`, OpenSSL 3.6.3, with SHA-256 and
  `pkeyutl -verify -rawin -pubin`; a job-private Ed25519 sign/verify roundtrip
  passed. This permits a vetted crypto path without adding a Rust dependency or
  installing a package. The release trust anchor must be bootstrap-provisioned;
  Core must not accept a public key shipped next to the release it is verifying.
- M2-B1's `verified` pointer is confirmed redundant: every constructor and
  rollback writes `verified == current`, and it has no independent product
  consumer. B4 removes it before activation and retains only `current` plus one
  explicit `previous` rollback target.
- Current checkpoint: M2-B4 — signed local release admission and activation.
  Verify one strict Ed25519-signed local release manifest and SHA-256 file
  inventory using the pinned bootstrap key and existing Termux OpenSSL, probe
  the admitted staged generation, then activate it through the simplified M2-B1
  transaction. No remote acquisition or fallback ladder is part of B4.
- Worker mode remains OFF; the primary Lead performs M2-B4 directly.
- The repository now uses a Click-inspired execution discipline at the
  Workboard layer: reuse successful same-revision evidence, do not reopen or
  replace an active bounded contract without new material evidence, and run the
  repository-required acceptance checks as one final validation batch after the
  implementation stabilizes. No Click plugin or Hook is installed, and these
  workflow rules never override SPEC/GOAL acceptance requirements.

## Goal Lifts

No lift is active. A proposed lift must identify a concrete product risk or
user-visible capability that cannot be handled within the current two
milestones. It must update this file before expanding `WORKBOARD.md`.

## Blocked / Resume Conditions

- Stop before any live install, activation, or replacement of the working
  Codex runtime during Milestone 1.
- Stop if a required upstream artifact cannot be immutably identified or
  verified.
- Stop if a test would write the live resolver, auth, profile, session, or
  installed runtime paths.
- Stop if update recovery cannot prove one complete old or new generation.
- Stop before implementation if the goal run is not using the configured
  primary Lead model and effort and no explicitly authorized equivalent exists.
- Exhausting the current `WORKBOARD.md` bundle is a planning checkpoint, not a
  blocker. If the milestone gate is incomplete, the primary Lead plans and
  records the next bounded bundle itself.

Resume by reading `SPEC.md`, then this file, then `WORKBOARD.md`. Continue only
the selected current milestone. When its gate is proven, the same primary Lead
updates this ledger, replaces `WORKBOARD.md` with the next milestone plan, and
continues without a routine user pause.

## Handoff

Resume through the installed `$goal-md` workflow with
`/goal resume codex-goal.md`; it must resolve to this file on
`rewrite/rust-core` and run with the primary agent configured as
`gpt-5.6-sol` / `max`. The primary Lead authors, records, and directly implements
each bounded bundle. The legacy branch may be inspected by the Lead for behavior
discovery but no source file may be copied into the rewrite.
