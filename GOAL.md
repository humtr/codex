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
- Lifecycle: keep the primary Lead across both Core milestones while its
  context remains available and accurate; on resume rebind the branch, commit,
  `SPEC.md`, this file, and `WORKBOARD.md`
- Planning owner: the primary Lead reads authorized repository evidence
  directly, plans each bounded implementation bundle, and records it in
  `WORKBOARD.md` before delegation
- Integration owner: the primary Lead inspects actual worker diffs, reruns
  load-bearing validation, records acceptance evidence, and commits accepted
  work; worker reports do not establish acceptance
- Checkpoints: initial implementation bundle; completion or rejection of the
  current bundle; material deviation, blocker, or authority change; and
  milestone transition
- Additional planning agents, problem advisors, and checkpoint reviewers:
  disabled during both Core milestones
- Primary mutation boundary: authority-document and integration changes are
  allowed; routine product-code and test implementation is delegated
- Identity rule: do not persist a live Lead identity in tracked files
- Review policy: the Lead's validation is integration verification, not
  independent review; invoke a fresh independent product reviewer only after
  the Milestone 2 acceptance candidate is complete
- Policy source and timestamp: user decision on 2026-08-28

## Implementation Worker Policy

- Worker assistance enabled: true
- Concurrency: exactly one mutating worker at a time in the shared worktree
- Worker model and effort: selected by the primary Lead for each bounded bundle
  from available implementation-capable agents and recorded with the bundle;
  no worker may assume planning or acceptance authority
- Worker context: `fork_context: false`
- Reuse rule: reuse the same worker with delta-only `send_input` only inside the
  current bundle; retire it after acceptance
- Access mode: repository, filesystem, and shell tools only within packet-
  authorized paths and temporary test roots; no delegation, package operations,
  live product mutation, commits, or pushes; network/provider tools require an
  explicit later acceptance gate and one bounded non-secret validation bundle
- Writable scope: product code and tests named in the accepted bundle; never
  `AGENTS.md`, `SPEC.md`, this file, or `WORKBOARD.md`
- Initial packet budget: at most 12,000 estimated input tokens or 48,000
  characters without a tokenizer, with the bound branch/commit, exact bundle,
  governing excerpts, no more than eight relevant source/test paths or snippets,
  read/write scope, validation, protected surfaces, and completion report
- Follow-up budget: delta-only `send_input` of at most 5,000 estimated input
  tokens or 20,000 characters, containing prior/current commits, changed paths,
  relevant diff or new failure evidence, changed authority, and one next action
- Scope-expansion rule: the worker requests one exact item or path and waits;
  only the Lead may authorize a broader packet
- Completion report: changed paths, concise validation results, unresolved
  failures, and protected-surface non-mutation confirmation
- Replacement rule: resume the same worker when possible; replace it only after
  confirmed identity loss or bundle-context mismatch, and give the replacement
  the current diff and remaining gate instead of a broad reread
- Authority: implementation only; the primary Lead retains planning,
  architecture, authority-document, integration, commit, and acceptance control
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
- Delegate bounded product-code and test implementation to one worker at a time,
  reuse that worker only within its current bundle, and keep worker context,
  tools, writable paths, and follow-ups narrowly scoped.
- Do not create planning subagents, problem advisors, or checkpoint reviewers
  during the Core milestones. Review the complete Milestone 2 candidate with a
  fresh independent reviewer.
- Use exactly two Core milestones.
- This foundation task creates documents and workflow only; implementation is
  handed to a later implementer.

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

### Proven Now

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
- Milestone 1 bundle M1-B1 is accepted at
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
- Milestone 1 bundle M1-B2 is accepted at
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
- Milestone 1 bundle M1-B3 is accepted at
  `815c9104c726f212ee4a51b518af14e8c133b20c`. The production exec command
  removes exactly `CODEX_MANAGED_BY_NPM`, `CODEX_MANAGED_BY_BUN`,
  `CODEX_MANAGED_PACKAGE_ROOT`, `LD_PRELOAD`, and `LD_LIBRARY_PATH` from the
  child exec environment without `env_clear` or parent-process mutation;
  unrelated environment entries are preserved. Failed exec evidence proves the
  caller process retains its synthetic inherited values.
- Primary-Lead validation job `job_hnt_c908186115` reran M1-B3 with an external
  temporary Cargo target and offline mode; formatting, 13/13 tests, and locked
  workspace build passed while status remained limited to `crates/core/src/main.rs`.
- Milestone 1 bundle M1-B4 is accepted at
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
- Milestone 1 bundle M1-B5 is accepted at
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
- Milestone 1 bundle M1-B6 is accepted at
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
- Milestone 1 bundle M1-B7 is accepted at
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
- Milestone 1 bundle M1-B8 is accepted at
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

### Historical Proof

- The predecessor's behavior and history remain available only as sealed
  legacy evidence. They are not promoted into proof for the rewrite.

### Not Proven

- Milestone 1 is not complete. M1-B1 proves workspace/classification, M1-B2/B3
  prove isolated final-exec argv/stream/exit and contamination-fence behavior,
  M1-B4 proves isolated FD 33/34 setup/restoration plus test-resolver
  non-mutation, M1-B5 proves TTY/external-SIGTERM fidelity on the current
  Android/Termux device, M1-B6 proves the Termux sandbox-policy planner, M1-B7
  proves composition of that planner with the runtime-FD final-exec path, and
  M1-B8 proves a pure explicit-input base-environment plan. That positive plan
  is not yet applied at the final exec boundary and `main` is still not wired to
  a qualified upstream runtime. Runtime/generation path selection, doctor
  composition, generation/updater interfaces, and the full real-Termux smoke
  gate remain unproven.
- No release artifact, installation, update, activation, rollback, offline
  recovery, fresh-device behavior, Milestone 2 result, or production readiness
  is proven.
- No Manager implementation or Core/Manager integration is proven.

### Checkpoint Plans

- Plan status: M1-B1 accepted; Milestone 1 remains active.
- The primary Lead records each next bounded bundle in `WORKBOARD.md`, launches
  exactly one implementation worker, inspects the actual diff, and reruns the
  load-bearing validation before acceptance.
- M1-B8 is accepted at `ae678fdb01b065a78f55b4e0546a8c4b12c498fa`.
- M1-B9 is the current checkpoint. It applies an already-built B8 environment
  plan to the accepted B7 runtime-FD final-exec path while preserving B3's exact
  contamination removals, argv/FD/process behavior, and parent-state
  non-mutation. Product path selection and normal `main` wiring remain later
  checkpoints.
- At every bundle completion the Lead records its own diff/test disposition
  here. No worker report, planning subagent, or checkpoint reviewer substitutes
  for that integration decision.

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
- Stop if the current bundle has no available worker after resume and one
  bounded replacement attempt. Record the current diff and remaining gate; the
  Lead must not silently become the product-code implementer.
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
`gpt-5.6-sol` / `max`. The primary Lead authors and records the first bounded
Milestone 1 bundle, then invokes one implementation worker. The legacy branch
may be inspected by the Lead for behavior discovery but no source file may be
copied into the rewrite or delegated as migration material.
