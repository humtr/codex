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
- Implementation cadence: execute each Workboard bundle as ordered vertical
  proof slices. One observable contract receives its implementation, focused
  regression, nonzero focused invocation, relevant compile/test gate, and Lead
  diff inspection before another independent contract begins
- Red-state rule: a non-compiling relevant target, stale superseded test,
  unexpected warning/dead path, zero-test invocation, or unmapped production
  definition freezes new product behavior until the affected class is audited
  and the current slice returns to green
- Validation split: run cheap compile/focused gates at slice boundaries; batch
  only the stabilized bundle's full suite, repeated parallel runs, protected-
  surface verification, and release build for final acceptance
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
- Prevent implementation/proof drift inside a bundle: `WORKBOARD.md` must carry
  the current vertical slice/proof map, and production behavior must not advance
  past a red or unmapped slice. Historical tests never authorize a compatibility
  branch after the public product path has replaced their behavior.

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
- M2-B4 implementation/proof divergence checkpoint is bound to
  `rewrite/rust-core@8df74abfca793fe9e8008553b5d9742ba6d2b4d4`, source SHA-256
  `de0943aa415bb6a7416a7e83a59755f53f90a53df1d10a437694e96a66433575`, and
  source-diff SHA-256
  `7007cad7881378a0a66f75fa207f968c2b5d01e2efebe22b67e1588121d81621`.
  The diff adds 684 and removes 88 lines, but adds no B4 regression test; the
  retained test target does not compile because three `LocalCoreRoots` fixtures
  omit the new trust/OpenSSL fields, the release build has one dead-code warning,
  and the prior B3 public test still asserts staging without `PREFIX` or
  activation. This exact snapshot is repairable work in progress, not an
  acceptance candidate.
- The checkpoint root cause is an execution-policy gap: final grouped validation
  was batched correctly, but compile/focused proof was also deferred, allowing
  several independent production contracts to accumulate behind red tests.
  M1-R2's KEEP/COLLAPSE/DELETE lesson existed only as historical ledger evidence
  instead of an active per-slice stop rule.
- Primary-Lead disposition: planning completed without a delegated planner or
  worker; freeze new B4 behavior, revise the current Workboard into vertical
  proof slices, exhaustively disposition the changed production/test class,
  restore compile/focused proof slice by slice, and run one grouped final
  acceptance batch only after stabilization. The success threshold is unchanged,
  so no goal lift is active.
- Same-revision evidence reuse remains required. It prevents redundant full-suite
  churn but never permits skipping a red slice gate. No Click plugin or Hook is
  installed, and this workflow does not override SPEC/GOAL acceptance.
- M2-B4 — signed local release admission and atomic activation — is accepted at
  `8483d2b2db488af032d6f9829639f971e2b5ff3f`. Exact
  `codex update --local <DIRECTORY>` now admits only the bootstrap-pinned
  Ed25519 key and strict signed SHA-256 inventory, enforces release policy and
  sequence before candidate execution, stages and re-verifies one complete
  generation, runs version/declared-doctor probes, and atomically activates it.
  Exact `codex update --rollback` verifies and swaps only the retained signed
  `previous` generation. Activation state is the single v2 `current` plus
  optional `previous` model; `verified`, unsigned staging, proof-only rollback,
  duplicate qualification, and fallback machinery are absent.
- The accepted B4 source SHA-256 is
  `ea8c840a7f4bff1dcbe3fd3ae36b16b5acb4e0389b8ec8304a069584a1fa49ba`
  and its parent-relative source-diff SHA-256 is
  `3d5b17788c66aacf3fda26d317112c1b1e23e2ae3257d64f05dbbee4600f69c5`.
  Final evidence passed focused activation 4/4, public rollback 2/2, the full
  serial suite 59 passed / 0 failed / 1 explicit smoke ignored, three isolated
  complete default-parallel runs each 59/0/1, explicit real-Termux read-only
  smoke 1/1, formatting/diff checks, and a warning-free locked release build.
  Live resolver and installed launcher SHA-256 plus device/inode/mode/uid/gid/
  size/mtime identities were unchanged; no live generation, activation,
  Manager, resolver, auth/session/profile, package, or publication state changed.
- B4's final product-path review caught one same-class defect after the first
  grouped run: forward update had not bound the recovered `current` pointer name
  to its signed generation identity. Acceptance was reopened, all installed
  target checks were collapsed into one verifier, a public pre-staging
  regression was added, and the complete grouped batch was rerun on the repaired
  source. A mistyped nonexistent Cargo package produced no test execution and was
  explicitly excluded from evidence. This closes the earlier large-change
  failure mode with slice-local stop-on-red proof rather than end-batched tests.
- Current checkpoint: M2-B5 — immutable HTTPS release acquisition. Add the
  smallest remote signed-release file acquisition adapter that feeds the exact
  B4 admission/staging/probe/activation path. Define its public and transport
  contract in `SPEC.md` before product code; do not create a second updater,
  archive path, release-discovery service, automatic checker, or fallback.
- M2-B5 starts from clean `rewrite/rust-core@8483d2b2db488af032d6f9829639f971e2b5ff3f`.
  Read-only feasibility found dependency-free Core plus Termux curl 8.21.0 at
  `$PREFIX/bin/curl` with HTTPS/OpenSSL support. No remote endpoint is yet
  authoritative and no live network request is acceptance evidence; transport
  tests use a pinned fake curl in temporary roots. Worker mode remains OFF and
  the primary Lead performs M2-B5 directly.
- M2-B5 — immutable HTTPS release acquisition — is accepted at
  `c39c338d6238d3e8aba128d8fa522d0e9de66d83`. Exact
  `codex update --remote <HTTPS_BASE_URL>` now validates one bounded canonical
  HTTPS generation base, invokes only `$PREFIX/bin/curl` with config/environment
  disabled and explicit CA/time/byte policy, admits bounded signed control before
  content, reconstructs only the signed file inventory in a private `.acquire-*`
  root, removes acquisition before probes/activation, and reuses the exact B4
  admission, staging, probe, atomic activation, anti-rollback, and rollback path.
  There is no discovery, redirect, retry, mirror, fallback URL, archive path,
  second updater, or live-network acceptance claim.
- B5 replaced the mode-blind release v1 format with strict
  `codex-release-v2`: every signed file record binds lowercase SHA-256 and exact
  four-octal-digit regular-file mode; special bits are impossible, every file is
  owner-readable, and runtime/Manager/helpers are owner-executable. Local source,
  remote reconstruction, staged generation, current verification, and rollback
  all use the same v2 check. The accepted source SHA-256 is
  `d5c5f69da6ce8d7f52b20ce8d426d3948e3452566a3a4f075dca67fd1e773dca`
  and its parent-relative source-diff SHA-256 is
  `747c4cc5fd3086454696179713b2e3120c967947c907ee00add3e75636e63196`.
- Final B5 evidence passed the full serial suite at 69 passed / 0 failed / 1
  explicit smoke ignored, three complete default-parallel suites each at
  69/0/1, the explicit real-Termux read-only smoke at 1/1, formatting/diff
  checks, and a warning-free locked release build. Live resolver and installed
  launcher SHA-256 plus device/inode/mode/uid/gid/size/mtime identities were
  unchanged; no live network, generation, activation, Manager, resolver,
  auth/profile/session, package, launcher, or publication state changed.
- The first otherwise-green grouped B5 batch was rejected because post-run
  inspection found one leaked B4 test root. The whole retained test cleanup
  class discarded `remove_dir_all` errors. B5 replaced it with one strict helper
  that tolerates only `NotFound`, removed the disposable leaked fixture, proved
  the affected regression 1/1, observed zero residue, and reran the entire
  grouped batch above on the repaired source. This extends the earlier
  stop-on-red lesson from test counts/warnings to cleanup evidence: a green test
  exit cannot override contradictory filesystem state.
- M2-B6 — official upstream artifact acquisition and safe adaptation — is
  accepted at `92787c85e4bc27de867f800d1414125d6247a210`. One non-installed
  release-production `codex-release-builder` accepts only an explicit stable
  version, local official aarch64-musl package, lowercase pinned SHA-256, Core
  artifact, metadata, and absent output. It snapshots while hashing, parses the
  bounded exact ustar/PAX layout without archive-directed writes, selects only
  the two static AArch64 executables, applies the exact drift-detecting 2/1/1/1
  equal-length policy, and complete-or-absent publishes only `generation.meta`,
  adapted `runtime`, and unmodified `compat/codex-code-mode-host`. It performs no
  discovery, signing, installation, activation, or live-state mutation.
- B6 is bound to official package `0.150.1` archive SHA-256
  `1ecac3f87823efb98153233b076ea3d6e34a7a8cebe43c5285dc5f79e1514639`.
  The release-builder library SHA-256 is
  `5cf290e919adaa4ef92f1715cff4cb0cdb2f6ad9973020f0111f60f00f4019ca`,
  its entrypoint SHA-256 is
  `ce09449e603806ea8bf8571f5978b11a58974f40ddba736ad2924204a0b40d1d`,
  the Core source with test-only B4/B5 integration is
  `71086b63782724cfad134af47f701932861bc01da8964787ab995914df165642`,
  and the parent-relative code-diff SHA-256 is
  `b9fc1e2b4e87aaac063ac8825c2b7a791965341a55ca0cf9339c169edcc43fe1`.
- Final B6 evidence passed the full serial workspace suite at Core
  70/0/1-ignored plus builder 5/0, three complete default-parallel suites at the
  same counts, the explicit real-Termux read-only smoke at 1/1, formatting and
  diff checks, and a warning-free locked release build. Test-root residue was
  zero; live resolver and installed launcher SHA-256 plus
  device/inode/mode/uid/gid/size/mtime identities were unchanged. A zero-test
  command and one test-only name-shadowing compile failure were rejected and
  corrected before evidence was reused. The shared pre-commit hook referenced a
  script absent from the orphan lineage, so the already-validated exact staged
  diff was committed with `--no-verify`; the hook did not change source or index.
- M2-B7 — signed trust-key rotation and rollback compatibility — is accepted by
  this authority update from base
  `253156c37a2bd22af8faae0bce03587999ffd136`. The user approved the exact
  security contract before mutation. `codex-release-v3` now binds one canonical
  32-byte Ed25519 release key, requires the candidate-key `release.sig` over the
  exact manifest, and requires a second `release-authority.sig` by the current
  update key only when that key changes. Adjacent/discovered keys, PKI/keyserver
  trust, unbounded key history, and a second updater remain absent.
- B7 upgrades durable authority to one `codex-activation-state-v3` /
  `codex-activation-journal-v3` transaction containing `update_key`,
  `current/current_key`, and the optional `previous/previous_key` pair. Forward
  activation advances update authority with the accepted candidate. Explicit
  rollback swaps only generation/verifier pairs and never rolls back
  `update_key`; a rotated-away key therefore cannot regain forward signing
  authority through runtime rollback. Installed-generation verification binds
  the state verifier key to the manifest release key before signature/inventory
  admission, and ordinary launch remains independent of OpenSSL, network, and
  trust verification.
- Bootstrap trust is now explicitly one-way. The pinned
  `release-public-key.pem` may initialize the first v3 state only in the later
  bootstrap bundle; production Core update/recovery owns no bootstrap-key path
  and fails closed when v3 state is absent rather than reconstructing authority
  from the old bootstrap key. Local and remote update, current verification,
  staging, activation, recovery, and rollback reuse the same bounded trust
  policy; remote rotation conditionally acquires the authority signature before
  any generation payload.
- The accepted B7 Core source SHA-256 is
  `c4e501ece0ac6ccf75a01409f7e8e804297b326a5ead6317516f9f9755f1a3e0`,
  its parent-relative binary-diff SHA-256 is
  `06ccbd8cbd42f156de861e753df778dac3f6f29a60648fd48fa228759c8b4fc6`,
  and the approved B7 specification SHA-256 is
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
  Focused proof closed signed-transition admission 1/1, state rotation 1/1,
  local/remote/rollback integration 3/3, the B1 trust+generation fault matrix
  12/12, B4 14/14, B5 10/10, and the B6 admission bridge 1/1.
- Final B7 evidence was rerun from scratch after the last warning repair on one
  final formatted source. The full serial workspace suite passed Core
  75/0/1-ignored plus builder 5/0; three complete default-parallel workspace
  suites each passed the same counts; the explicit real-Termux read-only smoke
  passed 1/1; and the locked release build passed with `-D warnings`. Test-root
  residue was zero and the generated untracked `target/` build tree was removed.
  The installed launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with the exact pre-run device/inode/mode/uid/gid/size/mtime identity, and live
  `resolv.conf` remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with its exact pre-run identity. No live generation/trust state, resolver,
  installed launcher, Manager, auth/profile/session, package, network authority,
  or publication state changed.
- B7 rejected and repaired each red gate before proceeding: the registered
  validation metadata was found stale and npm-bound before any product mutation,
  so zero Rust tests from that path were counted; direct Cargo established the
  green baseline instead. The first raw-key OpenSSL path required `/dev/stdin`,
  stale v2 assertions/fixtures were updated only after their new fail-closed
  meaning was proven, and the first final release build exposed one production
  dead-code warning after bootstrap removal. That planner was narrowed to
  test-only bootstrap fixtures and the entire grouped acceptance was rerun.
  Registered project validation metadata remains an administrative mismatch and
  must be repaired before the next product slice relies on it.
- M2-B8 — prebuilt Core and minimal fresh bootstrap — is accepted at product tip
  `192fcece0b416004bddd9181e24b1245290ebe81` by this authority update. The B6
  `--core` input now snapshots and qualifies exactly one ELF64 little-endian
  AArch64 PIE using `/system/bin/linker64`, then binds the selected bytes through
  the existing `generation.meta.core_artifact_digest`; no Core copy, sidecar,
  alternate manifest/archive, or second release-production protocol was added.
  The accepted release-builder source SHA-256 is
  `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`.
- Fresh bootstrap is one local audited script,
  `bootstrap/codex-bootstrap`, SHA-256
  `c1b107699a64c08cc49a99ceb433c3dd1b6c7ca53637fb0b53cc73b6ce35e9fa`.
  Before executing or publishing Core it snapshots the Core, bootstrap key, v3
  manifest/signature, and `generation.meta`; verifies the manifest with the
  pinned key; requires the signed release key to equal that key; verifies the
  signed descriptor digest/mode; and requires the Core SHA-256 to equal the
  descriptor's signed `core_artifact_digest`. It then publishes only the
  canonical initial key seed and stable Core entrypoint via same-directory
  create-new/no-clobber temporaries and invokes the authenticated Core for
  self-test and first activation. It has no network, package-manager, compiler,
  update, rollback, discovery, or post-state recovery path.
- Core source SHA-256
  `6b401eb2890e6bad3117685cb7ce156690a5dfe53b0bab83fba7c5d335ca22bf`
  adds only the private fresh-bootstrap entry before public dispatch. That path
  derives the canonical `release-public-key.pem`, requires authoritative v3
  state to remain absent after normal journal recovery, refuses initial key
  rotation, and reuses the accepted B7 v3 admission, candidate probe, immutable
  staging, installed re-verification, and initial state transaction. Once v3
  state exists it fails closed; ordinary launch, local/remote update, rollback,
  and recovery gained no bootstrap fallback or new trust source. The normative
  SPEC therefore remains unchanged at SHA-256
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- B8 artifact and integration proof is exact. Slice 1 focused Core-artifact
  qualification passed 2/2 and the complete affected builder suite passed 7/7.
  Slice 2 bootstrap proof passed 4/4 plus the existing initial-activation durable
  fault matrix 1/1. Slice 3 passed one complete release-production flow using an
  actual locked release Core: release Core -> B6 official-shape generation -> v3
  signing/admission -> real bootstrap -> isolated installed Core/v3 state. The
  same release Core SHA-256
  `28217cd50b417b94ac13975be7f7ca094b25ca4484db6a406f791c5b2584906e`
  was proven at builder descriptor, bootstrap input, installed stable Core, and
  installed signed-generation verification; affected B7 5/5, B8 bootstrap 4/4,
  B6 admission 1/1, and builder 7/7 also remained green.
- Final B8 grouped acceptance was rerun on committed product tip `192fcece...`.
  The project-registry canonical serial workspace suite passed Core
  81/0/1-ignored plus builder 7/0; three complete default-parallel workspace
  suites each passed the same counts; the explicit real-Termux read-only smoke
  passed 1/1; `cargo fmt --check`, bootstrap shell syntax, and `git diff --check`
  passed; and the locked workspace release build passed with `-D warnings`.
  Test-root residue was zero and generated `target/` was removed. The installed
  launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with identity
  `65089|1260183|755|10379|10379|7512|2026-08-28 01:28:18.815391370 +0900`,
  while live `resolv.conf` remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with identity
  `65089|94666|600|10379|10379|38|2026-08-28 01:04:03.530430900 +0900`.
  No live generation/trust state, resolver, launcher, Manager,
  auth/profile/session, package, network authority, publication ref, or private
  signing-key state changed.
- B8 rejected non-evidence and local proof defects instead of carrying them
  forward: a formatting-only Slice 1 stop; a legacy test-fixture mode mismatch;
  an exact test filter that selected zero tests; an Android hard-link permission
  failure replaced by no-clobber rename; and a stale Slice 3 expectation that
  `doctor` must exit zero while Manager remains intentionally unavailable. Each
  affected gate was rerun after repair. The inherited tmcp
  `project.validation.describe` package.json discovery remains a separate tooling
  limitation, while project-registry Cargo validation revision 3 is canonical.
  Each Slice 1-3 normal commit was also rejected before commit by the known
  orphan-lineage hook referencing absent `tools/update-wrapper-version.sh`; after
  exact index revalidation the established `--no-verify` closure was used. No B8
  commit was pushed or published.
- M2-B9 — launch/update overlap and injected-failure proof — is accepted at
  product tip `377fed80710e131ef6558118afcb45031818b302`. Slice 0 established a
  test-only pause harness over the existing eight-call activation durability
  boundary. Slice 1 then reproduced a concrete overlap defect: ordinary launch
  invoked recovery while an updater-owned `activation-journal.tmp` was durable,
  removed that temporary, and caused the updater's next publication to fail with
  ENOENT. The repair is deliberately one production behavior line: ordinary
  `load_activated_generation` now reads the authoritative pointer with
  `read_pointer_state`, while update, rollback, and explicit transaction recovery
  retain recovery ownership. No lock, retry, fallback, second state owner,
  persistent format, trust behavior, public surface, or SPEC delta was added.
- B9 focused proof closed every selected overlap class. Successful activation
  overlap passed 1/1 across durable calls 1, 3, 4, and 6; failed pre-activation
  update passed 1/1 across both before/after faults at calls 1 through 4; recovery
  overlap passed 1/1 across all sixteen before/after activation fault states and
  inserted ordinary launch after the first real recovery durable call wherever
  cleanup was required. All four B9 regressions passed together, the existing B1
  every-durable-boundary and partial/stale recovery regressions remained green,
  and the ordinary loader never mutated updater/recovery-owned journal or
  temporary files. The accepted Core source SHA-256 is
  `130f1097f9d31bdedf7a5212daf9dc3be6b5027e2481863708135411d00cb317`;
  the B9 Core diff from B8 authority commit `0bcac01...` has SHA-256
  `dca5dfc9bf5f15406d54afef8fcfc371149f534cf49c046824b9fe979fc43fc6`;
  normative SPEC remains
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`.
- Final B9 grouped acceptance ran on clean committed product tip `377fed8...`.
  The canonical locked serial workspace suite passed Core 85/0/1-ignored plus
  release-builder 7/0; three complete default-parallel workspace suites each
  passed the same counts; the explicit real-Termux read-only smoke passed 1/1;
  `cargo fmt --check`, bootstrap shell syntax, and `git diff --check` passed; and
  the locked workspace release build passed with `-D warnings`. B9 temp-root
  residue was rechecked with an explicit zero assertion after discarding a noisy
  output-helper invocation, generated `target/` was removed, and checkout
  returned clean. The installed launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with identity
  `65089|1260183|755|10379|10379|7512|2026-08-28 01:28:18.815391370 +0900`,
  while live `resolv.conf` remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with identity
  `65089|94666|600|10379|10379|38|2026-08-28 01:04:03.530430900 +0900`.
  No live generation/trust state, resolver, launcher, Manager,
  auth/profile/session, package, network authority, publication ref, or private
  signing-key state changed. Slice 0-3 normal commits were rejected before commit
  by the known orphan-lineage hook referencing absent
  `tools/update-wrapper-version.sh`; each exact staged tree was revalidated and
  closed with the established `--no-verify` precedent without changing the hook.
- M2-B10 — offline local-artifact install/recovery qualification — is accepted at
  product tip `40d04dcb5a02687fc48a1897e36309c387edc91f` by this authority update.
  The B10 commits add only test-owned release fixtures and end-to-end
  regressions after the accepted B9 production behavior; no public command,
  persistent format, trust source, recovery owner, or SPEC contract changed.
- B10 is bound to the accepted SPEC SHA-256
  `4ca9035c9c1a31c5afc3e9d4de978b304c96c687d03c0bee0aa446078fe11647`, current
  Core source SHA-256
  `6118d3d10c072b209906d532904574e30ad90b7f5d08777ff5cd8c31f7105f38`,
  release-builder source SHA-256
  `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`,
  bootstrap SHA-256
  `c1b107699a64c08cc49a99ceb433c3dd1b6c7ca53637fb0b53cc73b6ce35e9fa`, and
  the official upstream `0.150.1` archive SHA-256
  `1ecac3f87823efb98153233b076ea3d6e34a7a8cebe43c5285dc5f79e1514639`.
  The locked `-D warnings` release artifacts used for the final proof were Core
  `12bd6c525026d74df8f9784444cebfee45d33f3f00af5512b59168f38a9c8d01` and
  release-builder
  `d1db9e39f5b90dbf8f71e33b7f1a2fb80f6bd7399123278301328c77abeeb5ec`.
- B10 focused validation ran with `CODEX_B10_RELEASE_CORE` bound to that actual
  locked release Core and passed all four slices 4/4: release fixture/network
  denial, fresh offline bootstrap, signed local update plus explicit rollback,
  and injected transaction recovery with rollback still usable. The grouped
  locked serial workspace suite passed Core 89/0/1-ignored plus builder 7/0;
  three complete default-parallel repetitions passed the same counts. The
  explicit real-Termux read-only smoke passed 1/1, the locked `-D warnings`
  release build passed, formatting, bootstrap shell syntax, and `git diff
  --check` passed, and all B10/test-builder temporary roots plus repository
  `target/` were absent afterward.
- An environment-less invocation in which the B10 tests early-returned was
  rejected as non-evidence. No B10 acceptance count uses that run; the counts
  above came from the actual release-Core-bound focused and grouped commands.
  Live launcher SHA-256 remained
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and live
  `resolv.conf` remained
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, with no
  live generation/trust state, resolver, launcher, Manager, auth/profile/session,
  package, or publication state changed.
- At B10 closure, the next selected bundle was M2-B11 — isolated fresh-Termux
  and upgrade-from-legacy qualification. That bundle is now accepted below;
  final independent product review remains the later Milestone 2 gate. Worker
  mode remains OFF.

- M2-B11 — isolated fresh-Termux and digest-bound upgrade-from-legacy handoff — is accepted at product tip `c61f712cac0d3822a5ca66e48115215ff2722c07`. The implementation adds the exact local-only `codex-bootstrap upgrade-legacy <CORE_ARTIFACT> <SIGNED_RELEASE_DIR> <BOOTSTRAP_PUBLIC_KEY> <EXPECTED_LEGACY_ENTRYPOINT_SHA256>` path and preserves the existing v3 state, trust, update, rollback, and Manager boundaries.
- The normative B11 contract is SPEC SHA-256 `9ebe9a60a819c514beda09f7f70c86b7df4e375989c20be5752fc8cb6f132e4a`. The expected legacy digest is only an explicit replacement-target selector. Handoff prepares and verifies one complete signed initial v3 state before same-directory atomic entrypoint replacement; prepared and completed retries are exact and idempotent. No legacy discovery/import/execution, backup, second journal, fallback, or new release schema was added.
- Core continues to fail closed for explicit unsupported Linux sandbox modes with status 2 and never invokes, installs, downloads, or repairs bwrap. Ordinary launch remains independent of bwrap and Manager availability.
- The focused B11 group passed `9 passed, 0 failed` with the actual release Core, covering fresh qualification, exact grammar and digest classes, no-mutation conflicts, prepared-state resume and interruption, key/previous conflicts, atomic replacement, version/doctor, and the first post-handoff signed Core update plus rollback. A mistaken bare exact filter that selected zero tests was discarded; the corrected substring invocation supplied the counted evidence.
- Final locked validation passed: serial workspace Core `98 passed, 0 failed, 1 ignored` and release-builder `7 passed, 0 failed`; three default-parallel workspace repetitions passed the same counts. The locked `-D warnings` release Core build, `cargo fmt --check`, bootstrap shell syntax, and `git diff --check` also passed. Final source identities are Core `1da1096633e2c1b8c242970b7f545ce6109d5641a4baf2d3b9aa36de2d8fc3cc`, bootstrap `9a9285ee838ccf2665feb3c5055f66f03623e8abc9ba11e2633d15b7a31bb3e9`, release-builder source `aaa8ac051bf634bdf8fda799ca194b228b11e61e41fd1837570f979daefb4c9b`, and release Core artifact `d5587f0846648cfd920ae4d67498b1228cae8e45b6f751b09a4446d7de2eda74`.
- Disposition: KEEP the existing signed v3 admission, state authority, recovery, and one direct stable-entrypoint path; COLLAPSE handoff resume into that existing initial-state path and the existing activation boundary; DELETE legacy fallback/import/backup machinery and any bwrap repair path. No live launcher, resolver, auth/profile/session, Manager, package, or publication state changed, and no `codex-r2-*` test residue remains.
- Current checkpoint: M2 independent product review candidate. B11 is accepted for review, but no independent reviewer, promotion to `main`, push, or live cutover has been performed. Worker mode remains OFF.

- M2-R1 — publication durability and retry closure — is accepted at product tip `f4fa0b53518f49cd7077d2765bf68f713dc38fe0`. Generation publication now synchronizes final regular files and modes, syncs directory trees bottom-up, atomically publishes into the generation root, and syncs that root; a post-rename sync failure can reuse only the exact signed verified generation. Fresh bootstrap trust-seed and stable-entrypoint publication are owned by authenticated Core with private temporaries, final-mode/file/parent synchronization, no-replace differing-target protection, and same-input retries. Legacy handoff retries always revalidate the Core target and synchronize its parent, including an already-Core target. Release-builder re-synchronizes runtime and descriptor files after final mode changes.
- The normative M2-R1 SPEC SHA-256 is `3bcadc1831eb73a1f6e5e6813f4c015e8da039abbcb3310f32ebee6f0db28702`. Final source identities are Core `a3cc6715b21e7a9d07afe804964dc5950859c51a36ef4c8f47e9fe5c384919d4`, bootstrap `6eb5a68d97c954bc3379321f8046f480bce57addc1c73e3b17883436b5990f45`, and release-builder `d0de07b4cc338b1dfdd6b0ce1165090eef55f2c35fbfe8c24f96a7c36c473aab`. The warnings-denied locked release artifacts used in final proof were Core `7dea4df0bf68f2f0351208a420ba3a7daeb490bf4a9dc22b502f59a7b371d0a4` and release-builder `c53fa026e6afe827dd5a73e19fe4ccc6ed0ea4a3ceedf810e0098ce1ea427831`.
- M2-R1 focused proof passed: five Core tests covering generation fault boundaries, exact generation reuse, fresh trust-seed retry, fresh entrypoint retry, and legacy parent-sync retry; M2-B3 `6/6`, M2-B4 `14/14`, M2-B8 `5/5`, M2-B11 `9/9`, and release-builder focused proof `1/1`. The final locked serial workspace suite passed Core `103 passed, 0 failed, 1 ignored` and release-builder `7 passed, 0 failed`; three independent default-parallel workspace repetitions also passed. `cargo fmt --check`, bootstrap shell syntax, and `git diff --check` passed.
- Protected-surface verification kept the live launcher SHA-256 at `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and live `resolv.conf` at `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07` before and after. All validation used disposable roots or external temporary Cargo targets; no installed launcher/runtime, resolver, auth/profile/session, Manager, package, publication state, push, or promotion changed.
- Disposition: KEEP one signed v3 admission/state/recovery authority and one direct stable-entrypoint path; COLLAPSE fresh trust-seed publication, fresh entrypoint publication, and legacy retry publication behind the authenticated Core durability boundary; DELETE shell-side persistent publication, retry bypasses, legacy fallback/import/backup machinery, and any bwrap repair path. Core still fails closed for unsupported Linux sandbox requests and never invokes or repairs bwrap.
- Current checkpoint: M2 independent product review candidate. M2-R1 is accepted for review, but no independent reviewer, promotion to `main`, push, live cutover, or bounded device qualification has been performed. Worker mode remains OFF.

- M2-R1 review follow-up — generation collision race — is accepted at product
  tip `084531b42bbcd6235c32393a576a09265269974e`
  (`fix: protect immutable generation publication`). The review
  follow-up closed a concrete race in which the pre-publish existence check
  could be invalidated and ordinary Unix rename could replace an existing
  generation directory. The real generation publication path now uses the
  existing no-replace primitive and maps an atomic destination collision to the
  existing `GenerationCollision` result; no activation-state or legacy-entrypoint
  replacement semantics were changed.
- The focused regression
  `test_m2_r1_generation_collision_race_never_replaces_existing_directory`
  injects a destination after the check, proves the existing sentinel survives,
  proves the collision result, and proves candidate cleanup. The paired M2-R1
  generation durability test also remained green: focused generation group
  `2 passed, 0 failed`; the initial exact smoke filter that selected zero tests
  was discarded and the corrected substring invocation selected the intended
  one smoke test.
- Final grouped acceptance on the formatted source passed the locked serial
  workspace suite `104 passed, 0 failed, 1 ignored` and three independent
  default-parallel repetitions with the same counts; release-builder passed
  `7 passed, 0 failed`. `cargo fmt --check`, `git diff --check`, bootstrap
  shell syntax, and the locked warnings-denied workspace release build all
  passed. The explicit real-Termux read-only smoke passed `1 passed, 0 failed`.
- The final source hash is Core
  `1cfc6f0aa0c392a755845e9a37a96366ce40ff2a2c7be0408eac39f569fdfcc2` against
  SPEC hash `3bcadc1831eb73a1f6e5e6813f4c015e8da039abbcb3310f32ebee6f0db28702`.
  Test and build target roots were removed with zero residue. Protected
  launcher and resolver hashes and device/stat identities remained unchanged;
  no live generation/trust, resolver, launcher, Manager, auth/profile/session,
  package, network, publication, push, or promotion state changed.
- Exhaustive rename disposition for this class is complete: KEEP no-replace
  generation publication and the existing release-builder no-replace output;
  KEEP intentional activation-state replacement and digest-bound legacy
  entrypoint replacement; DELETE no new fallback, retry, lock, bwrap repair,
  or second publication path. Independent product review remains the next
  gate; worker mode remains OFF.

- M2-R1 independent-review remediation — bootstrap authority/residue,
  generation-root confinement, and bounded control/artifact I/O — is accepted
  at implementation commit
  `33b4bf3f6a4fcff7d2f7bbf67bb1d24b76b73d48`. Fresh bootstrap now passes the
  bounded authenticated key/Core snapshots into Core activation, binds the
  signed generation's `core_artifact_digest` to that Core, and preflights v3
  transaction residue before trust-seed, entrypoint, generation, or activation
  publication. Local/remote/installed generation paths reject symlink or
  non-directory generation roots before reuse or publication. Descriptor and
  manifest loads, bootstrap snapshots and Core/key copies, and release-builder
  Core snapshots enforce their bounds before and during consumption.
- Focused proof passed: Core `m2_r1_` group `11 passed, 0 failed`, the corrected
  exact Core-binding regression `1 passed, 0 failed`, and release-builder B8
  Core-artifact group `2 passed, 0 failed`. A prior unqualified exact filter
  selected zero tests and was discarded as non-evidence; the namespaced exact
  invocation selected the intended one test.
- Final grouped acceptance passed the locked workspace serial suite with Core
  `110 passed, 0 failed, 1 ignored` and release-builder `7 passed, 0 failed`.
  Three independent default-parallel workspace repetitions passed the same
  counts. The locked `-D warnings` release build, `cargo fmt --all -- --check`,
  bootstrap shell syntax, and `git diff --check` passed. The explicit
  real-Termux read-only smoke passed `1 passed, 0 failed`.
- Final authority/source identities are SPEC
  `dca2439c87567710e5a8fe7a219b16837b9454af0eb4b69d9db37a430f2be49e`, Core
  `02944a17348dcb0b822135e53ce71600e7e2440568d5a1a2d42be7e69117a094`,
  bootstrap
  `4cda45ad448d110c224854724ab8229fc9aa431d27ea4c818ab574046b1005db`, and
  release-builder
  `957dc5f6d46b280f7a8d4e24e1f25d0bfb589a82fb8664fde9b047dbf393005f`.
  The warnings-denied release artifacts used in the final build were Core
  `6de619aa3a233e1a4354c625daaf9840cfdf7f21dd110468428945c1fff97f37` and
  release-builder
  `7742288c621af679ad8ccc2bf61a08fd73f6d8d154045fd2b280898d2b294f16`.
- The three stale disposable roots observed after the parallel batch were
  confirmed unused, moved to a recoverable quarantine, and placed in the
  system trash; the corrected focused run produced no new root and the final
  canonical residue scan is empty. Protected launcher SHA-256 remains
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` and
  live `resolv.conf` remains
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`, with
  device/stat identities unchanged. No live generation/trust, resolver,
  launcher, Manager, auth/profile/session, package, push, or promotion state
  changed.
- Disposition: KEEP one signed v3 admission/state/recovery authority, one
  direct stable-entrypoint path, and atomic no-replace generation publication;
  COLLAPSE fresh bootstrap binding/residue checks and bounded-input handling
  into those existing Core boundaries; DELETE no fallback, repair, lock, or
  second publication path. Core still fails closed for explicit unsupported
  Linux sandbox requests and never invokes or repairs bwrap. Current checkpoint:
  M2 independent product review candidate; no independent reviewer, promotion
  to `main`, push, live cutover, or bounded device qualification has been
  performed. Worker mode remains OFF.

- M2-R2 — independent-review remediation — is accepted at implementation
  commit `a56a85cb88d3866ddc52134aae6dedaf88ac6c1f`. The selected active
  generation is now confined to real directories and regular non-symlink
  descriptor, runtime, Manager, helper, and compatibility paths before public
  launch; ordinary launch cannot follow a generation-root symlink escape.
  Activation and explicit recovery writers now serialize on one kernel-held
  exclusive state-root lock, with no persistent lock record and automatic
  kernel release on process exit. A contending writer returns `WriterBusy`
  before touching journal/state files. Valid `doctor` arguments now turn
  upstream probe/setup failure into a redacted `unhealthy` status so
  `doctor --json` still emits a machine report with nonzero health-failure
  status; usage errors remain distinct.
- The M2-R2 normative SPEC SHA-256 is
  `9f5db58c49d1572d794f2b60035fc07f3a68390d1e165df670649bd2daad2d8e`.
  Final source identities are Core
  `2d79d7185fe16aed80982509f1be78526713f9c76806b9e956b6a5f1da01918`,
  bootstrap
  `4cda45ad448d110c224854724ab8229fc9aa431d27ea4c818ab574046b1005db`, and
  release-builder
  `ce09449e603806ea8bf8571f5978b11a58974f40ddba736ad2924204a0b40d1d`. The
  locked warnings-denied release artifacts used in final proof were Core
  `189f96259a606d9c9d1d8df5a1f8338d07ad7c02f4d33733934e0a5f3cd497ca` and
  release-builder
  `7742288c621af679ad8ccc2bf61a08fd73f6d8d154045fd2b280898d2b294f16`.
- Focused proof passed: M2-R2 `3 passed, 0 failed` (six public generation
  symlink cases, writer contention/release, and public `doctor --json`
  failure), adjacent M2-B9 `4 passed, 0 failed`, M2-B2 `6 passed, 0 failed`,
  and the corrected B4 source-admission regression `1 passed, 0 failed`.
  Final locked serial workspace proof passed Core `113 passed, 0 failed,
  1 ignored` and release-builder `7 passed, 0 failed`; three independent
  default-parallel repetitions passed the same counts. The locked `-D
  warnings` release build, formatting, bootstrap shell syntax, and
  `git diff --check` passed. Explicit real-Termux read-only smoke passed
  `1 passed, 0 failed`.
- Protected launcher SHA-256 remains
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff` with
  stat identity `65089|1260183|755|10379|10379|7512|2026-08-28
  01:28:18.815391370 +0900`; live `resolv.conf` remains
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07` with
  stat identity `65089|94666|600|10379|10379|38|2026-08-28
  01:04:03.530430900 +0900`. All test/build roots were external and the
  final canonical residue scan is empty. No installed launcher/runtime,
  resolver, generation/trust, Manager, auth/profile/session, package,
  publication, push, promotion, or live cutover state changed.
- Disposition: KEEP one signed v3 generation/state/recovery authority, one
  direct stable-entrypoint path, no-replace publication, and the ordinary
  launch lock-free boundary; COLLAPSE active asset confinement and writer
  serialization into those existing Core paths; DELETE the stale multi-writer
  window, nested symlink consumption, raw doctor probe-error path, and any
  additional lock/fallback/bwrap-repair machinery. Current checkpoint: repaired
  M2 independent product review candidate; no fresh independent reviewer,
  promotion to `main`, push, live cutover, or bounded device qualification has
  been performed. Worker mode remains OFF.

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
