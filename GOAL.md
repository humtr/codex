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

- Milestone 1 is not complete. The normal `main` entrypoint is still not wired to
  a qualified upstream runtime. Runtime/generation selection, doctor composition,
  generation/updater interfaces, and the complete real-Termux smoke gate remain
  unproven. Process-environment capture is now proven by M1-B10.
- B4's current non-mutation proof is against test-owned resolver fixtures; the
  Milestone 1 completion gate still requires pre/post evidence that the actual
  live resolver path, content, mode, and stat identity remain unchanged during
  the bounded real-Termux smoke qualification.
- No release artifact, installation, update, activation, rollback, offline
  recovery, fresh-device behavior, Milestone 2 result, or production readiness
  is proven.
- No Manager implementation or Core/Manager integration is proven.

### Checkpoint Plans

- M1-R1 is closed at `4c1a8d90d6aa028106218d349076c465af8b8535`.
- M1-B10 is closed at `08e67e8c9fed23032ff59c38ff4765221d515d67`.
- Current checkpoint: M1-B11 — define and validate a pure in-memory generation
  manifest qualification interface before any generation path selection,
  serialization, updater I/O, or `main` wiring.
- Worker mode remains OFF by explicit user policy; the primary Lead implements
  M1-B11 directly. `harness.run` remains test/validation-only.
- B11 binds the SPEC-declared manifest inputs/outputs as opaque in-memory values:
  upstream package identity/version, immutable source artifact digest, expected
  platform/architecture, patch-policy identifier/report, runtime digest, helper
  digests, Core digest, optional Manager digest, Core API compatibility,
  persistent schema compatibility, qualification result, and creation metadata.
- B11 validates required non-empty bindings, expected platform/architecture,
  supported Core API/schema identity, helper binding uniqueness/completeness, and
  a successful qualification result, returning a distinct qualified wrapper that
  later runtime selection can require.
- B11 intentionally does not define a serialized manifest format, digest
  algorithm, signature format, generation directory name, runtime/helper path,
  `current` pointer behavior, updater network/local-artifact behavior, activation,
  or rollback. Those require later bounded interfaces and evidence.

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
